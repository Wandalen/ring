//! Composed ring over an SPSC, MPSC, or crossbeam backend behind one API.
//!
//! One of the ring family's 33 crates, which implement the concurrency write-path. This is the
//! **composition point**. The crossbeam backend belongs here, behind a cargo
//! feature, alongside the selection between [`ring_spsc`] and [`ring_mpsc`]
//! (-> `docs/decisions/002_crossbeam_queue_is_an_interim_backend_inside_ring_core.md`).
//!
//! # The API is value-shaped, and that is forced rather than chosen
//!
//! The two in-house backends publish through a *slot*: claim a reservation,
//! write in place, let the guard's `Drop` publish. `crossbeam_queue::ArrayQueue`
//! has no such thing. It offers `push( value )` and `pop() -> Option< value >`
//! and nothing else. A slot-shaped uniform API therefore cannot exist across
//! all three backends.
//!
//! So this crate presents a **value-shaped** API. [`Producer::try_push`] takes
//! a `T` and [`Consumer::try_recv`] returns a `T`, and the in-place reservation
//! API stops here. A caller that needs to build a record in the ring's own
//! memory reaches for [`ring_spsc`] or [`ring_mpsc`] directly and gives up the
//! backend swap. That is the trade, and it is not hidden.
//!
//! # What is uniform, and what only looks uniform
//!
//! | Property | [`ring_spsc`] | [`ring_mpsc`] | crossbeam |
//! |---|---|---|---|
//! | Producers permitted | exactly 1 | N | N |
//! | [`Producer::try_clone`] | `None` | `Some` | `Some` |
//! | [`Producer::free_capacity`] | **binding** | advisory | advisory |
//! | Drain order | publication order | per-producer order, interleaved | unspecified |
//! | `OverflowPolicy::DropOldest` | **rejected at construction** | rejected | supported |
//!
//! **`free_capacity` is one signature over two contracts, and the caller cannot
//! see which one it has.** At SPSC a reported `n` means `n` pushes will
//! succeed, because nothing else can take the room. At MPSC and crossbeam
//! another producer may take it between the read and the push. Code written
//! against the binding reading breaks when handed a multi-producer ring, with
//! no signature change and no compiler error. [`Producer::try_clone`] returning
//! `None` is the one machine-checkable way to tell the backends apart.
//!
//! # This crate adds no atomic of its own
//!
//! [`ring_spsc`] asserts zero read-modify-writes across a run, and every
//! publish reaches it through a method here. A counter added in this crate
//! would break that assertion with nothing in `ring_spsc`'s own dependency tree
//! to blame. There is accordingly no `AtomicUsize` anywhere below, and no
//! statistics. Instrumentation belongs in `ring_stats`, which is deliberately
//! not a dependency.
//!
//! ```
//! use ring_config::RingConfig;
//! use ring_core::Ring;
//!
//! let config = RingConfig::new( 8 ).unwrap();
//! let mut ring : Ring< u32 > = Ring::new( &config ).unwrap();
//! let mut ends = ring.ends();
//! let ( mut producer, mut consumer ) = ends.split();
//!
//! producer.try_push( 7 ).unwrap();
//! assert_eq!( consumer.try_recv(), Some( 7 ) );
//! assert_eq!( consumer.try_recv(), None );
//! ```

#![deny(missing_docs)]

use ring_config::RingConfig;
use ring_overflow::{Resolution, would_resolve};
use ring_slot::TypedSlot;
use ring_types::{Capacity, OverflowPolicy, RingError};

/// Which implementation is beneath the API.
///
/// Reported by [`Ring::backend`] so a caller can recover the contract it was
/// given. The module documentation lists the differences the uniform API cannot
/// express, and this is how a caller learns which column applies.
///
/// ```
/// use ring_config::RingConfig;
/// use ring_core::{ Backend, Ring };
///
/// let one = RingConfig::new( 8 ).unwrap();
/// let many = RingConfig::new( 8 ).unwrap().with_producers( 4 );
///
/// assert_eq!( Ring::< u8 >::new( &one ).unwrap().backend(), Backend::Spsc );
/// assert_eq!( Ring::< u8 >::new( &many ).unwrap().backend(), Backend::Mpsc );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Backend {
  /// [`ring_spsc`]: one producer, binding `free_capacity`, no RMW in the path.
  Spsc,
  /// [`ring_mpsc`]: many producers, advisory `free_capacity`.
  Mpsc,
  /// `crossbeam_queue::ArrayQueue`, the interim backend.
  #[cfg(feature = "crossbeam")]
  Crossbeam,
}

/// The storage, owning whichever backend the configuration selected.
///
/// Split into ends with [`ends`](Self::ends), which mirrors [`ring_mpsc`]'s own
/// two-step shape. The intermediate step exists because `ring_mpsc`'s producer
/// is `Copy`. Its handles borrow from a value that must outlive them both, and
/// a one-step `split` cannot name that lifetime. Using the same shape for every
/// backend keeps the API uniform.
#[derive(Debug)]
pub struct Ring<T> {
  storage: Storage<T>,
  overflow: OverflowPolicy,
}

#[derive(Debug)]
enum Storage<T> {
  Spsc(ring_spsc::Ring<TypedSlot<T>>),
  Mpsc(ring_mpsc::Ring<TypedSlot<T>>),
  #[cfg(feature = "crossbeam")]
  Crossbeam(crossbeam_queue::ArrayQueue<T>, Capacity),
}

impl<T: Send> Ring<T> {
  /// Build a ring, selecting the backend from the configuration.
  ///
  /// [`RingConfig::is_multi_producer`] chooses between [`ring_spsc`] and
  /// [`ring_mpsc`]. `new_crossbeam` selects the crossbeam backend, not a config
  /// field, because crossbeam is a build-time opt-in and not a property of the
  /// workload. For the same reason the method is named here but not linked.
  /// Under default features there is no such method to link to, and rustdoc
  /// reports the dangling reference as an error.
  ///
  /// # Errors
  ///
  /// [`RingError::PolicyUnsupported`] for `OverflowPolicy::DropOldest`, which neither
  /// in-house backend can honour. Evicting an unread record contradicts the
  /// exactly-once delivery both of them guarantee. Rejecting it here, at
  /// construction, is deliberate. The alternative is a `try_push` that silently
  /// behaves as `DropNewest`, and a caller who never learns the policy was not
  /// applied.
  ///
  /// ```
  /// use ring_config::RingConfig;
  /// use ring_core::Ring;
  /// use ring_types::{ OverflowPolicy, RingError };
  ///
  /// let evicting = RingConfig::new( 8 ).unwrap().with_overflow( OverflowPolicy::DropOldest );
  /// assert_eq!( Ring::< u8 >::new( &evicting ).unwrap_err(), RingError::PolicyUnsupported );
  /// ```
  pub fn new(config: &RingConfig) -> Result<Self, RingError> {
    if config.overflow() == OverflowPolicy::DropOldest {
      return Err(RingError::PolicyUnsupported);
    }

    // A `match` rather than `if`/`else`, because under the family's brace style
    // the `else` keyword lands on a line of its own. There `llvm-cov` opens a
    // region that nothing can ever execute, so the crate reads 120/121 with both
    // arms demonstrably covered. `tests/manual/readme.md` C4 records the measurement.
    let storage = match config.is_multi_producer() {
      true => Storage::Mpsc(ring_mpsc::Ring::with_config(config)),
      false => Storage::Spsc(ring_spsc::Ring::with_config(config)),
    };

    Ok(Self {
      storage,
      overflow: config.overflow(),
    })
  }

  /// Build a ring on the crossbeam backend, whatever the configuration's
  /// producer count says.
  ///
  /// This is the crossbeam backend's entry point. It accepts
  /// `OverflowPolicy::DropOldest` where [`new`](Self::new) refuses it, because
  /// `ArrayQueue::force_push` does exactly that, the one capability the
  /// in-house rings deliberately lack.
  ///
  /// # Errors
  ///
  /// None currently. The signature matches [`new`](Self::new) so the two are
  /// interchangeable at a call site, which is the point of a swappable
  /// backend.
  #[cfg(feature = "crossbeam")]
  pub fn new_crossbeam(config: &RingConfig) -> Result<Self, RingError> {
    Ok(Self {
      storage: Storage::Crossbeam(crossbeam_queue::ArrayQueue::new(config.capacity().get()), config.capacity()),
      overflow: config.overflow(),
    })
  }

  /// Which implementation is beneath this ring.
  #[must_use]
  pub const fn backend(&self) -> Backend {
    match self.storage {
      Storage::Spsc(_) => Backend::Spsc,
      Storage::Mpsc(_) => Backend::Mpsc,
      #[cfg(feature = "crossbeam")]
      Storage::Crossbeam(..) => Backend::Crossbeam,
    }
  }

  /// The ring's capacity in records.
  #[must_use]
  pub fn capacity(&self) -> Capacity {
    match &self.storage {
      Storage::Spsc(ring) => ring.capacity(),
      Storage::Mpsc(ring) => ring.capacity(),
      // The validated `Capacity` is carried in the variant rather than rebuilt
      // from `ArrayQueue::capacity`. Rebuilding it re-ran this crate's only
      // fallible validation inside an infallible accessor, so a backend that
      // ever rounded its capacity would turn `ring.capacity()` into a panic.
      #[cfg(feature = "crossbeam")]
      Storage::Crossbeam(_, capacity) => *capacity,
    }
  }

  /// The overflow policy a full ring will apply.
  #[must_use]
  pub const fn overflow(&self) -> OverflowPolicy {
    self.overflow
  }

  /// Borrow the ring for splitting.
  pub fn ends(&mut self) -> Ends<'_, T> {
    Ends {
      inner: EndsInner::of(&mut self.storage),
      overflow: self.overflow,
    }
  }
}

/// A borrow of a [`Ring`], from which the two ends are taken.
#[derive(Debug)]
pub struct Ends<'a, T> {
  inner: EndsInner<'a, T>,
  overflow: OverflowPolicy,
}

#[derive(Debug)]
enum EndsInner<'a, T> {
  Spsc(&'a mut ring_spsc::Ring<TypedSlot<T>>),
  Mpsc(ring_mpsc::Ends<'a, TypedSlot<T>>),
  #[cfg(feature = "crossbeam")]
  Crossbeam(&'a crossbeam_queue::ArrayQueue<T>),
}

impl<'a, T: Send> EndsInner<'a, T> {
  fn of(storage: &'a mut Storage<T>) -> Self {
    match storage {
      Storage::Spsc(ring) => Self::Spsc(ring),
      Storage::Mpsc(ring) => Self::Mpsc(ring.ends()),
      #[cfg(feature = "crossbeam")]
      Storage::Crossbeam(queue, _) => Self::Crossbeam(queue),
    }
  }
}

impl<'a, T: Send> Ends<'a, T> {
  /// Take the two ends.
  ///
  /// One producer and one consumer, always. Additional producers come from
  /// [`Producer::try_clone`], which reports the backend's cardinality rather
  /// than assuming it.
  pub fn split(&'a mut self) -> (Producer<'a, T>, Consumer<'a, T>) {
    let overflow = self.overflow;

    match &mut self.inner {
      EndsInner::Spsc(ring) => {
        let (producer, consumer) = ring.split();
        (
          Producer {
            inner: ProducerInner::Spsc(producer),
            overflow,
          },
          Consumer {
            inner: ConsumerInner::Spsc(consumer),
          },
        )
      }
      EndsInner::Mpsc(ends) => {
        let (producer, consumer) = ends.split();
        (
          Producer {
            inner: ProducerInner::Mpsc(producer),
            overflow,
          },
          Consumer {
            inner: ConsumerInner::Mpsc(consumer),
          },
        )
      }
      #[cfg(feature = "crossbeam")]
      EndsInner::Crossbeam(queue) => (
        Producer {
          inner: ProducerInner::Crossbeam(queue),
          overflow,
        },
        Consumer {
          inner: ConsumerInner::Crossbeam(queue),
        },
      ),
    }
  }
}

/// The writing end.
///
/// Not `Clone`. Cardinality is the backend's to decide, so a caller requests an
/// extra producer through [`try_clone`](Self::try_clone), which can refuse.
#[derive(Debug)]
pub struct Producer<'a, T> {
  inner: ProducerInner<'a, T>,
  overflow: OverflowPolicy,
}

#[derive(Debug)]
enum ProducerInner<'a, T> {
  Spsc(ring_spsc::Producer<'a, TypedSlot<T>>),
  Mpsc(ring_mpsc::Producer<'a, TypedSlot<T>>),
  #[cfg(feature = "crossbeam")]
  Crossbeam(&'a crossbeam_queue::ArrayQueue<T>),
}

impl<'a, T: Send> Producer<'a, T> {
  /// Publish one record, or hand it back.
  ///
  /// Never blocks. On a full ring the configured [`OverflowPolicy`] decides:
  /// `Fail` returns the record, `DropNewest` discards it and reports success.
  /// `DropOldest` cannot arrive here. [`Ring::new`] refuses it, and
  /// `Ring::new_crossbeam`, which exists only under the `crossbeam` feature,
  /// is the only path that accepts it. Without that feature the policy has no
  /// path at all, which is why the name is not a link.
  ///
  /// # Errors
  ///
  /// The record itself, under `OverflowPolicy::Fail`, when the ring is full.
  /// Returning the value rather than a unit error makes a refusal recoverable
  /// without a copy.
  ///
  /// ```
  /// use ring_config::RingConfig;
  /// use ring_core::Ring;
  /// use ring_types::OverflowPolicy;
  ///
  /// let config = RingConfig::new( 2 ).unwrap().with_overflow( OverflowPolicy::Fail );
  /// let mut ring : Ring< u8 > = Ring::new( &config ).unwrap();
  /// let mut ends = ring.ends();
  /// let ( mut producer, _consumer ) = ends.split();
  ///
  /// producer.try_push( 1 ).unwrap();
  /// producer.try_push( 2 ).unwrap();
  /// assert_eq!( producer.try_push( 3 ), Err( 3 ), "the record comes back" );
  /// ```
  pub fn try_push(&mut self, record: T) -> Result<(), T> {
    let refused = match &mut self.inner {
      ProducerInner::Spsc(producer) => producer.try_push(record),
      ProducerInner::Mpsc(producer) => {
        // Claim before consuming the record. `ring_mpsc::Producer::push` takes
        // the value and returns `Result< Seq, RingError >`, so on a full ring
        // the record is dropped inside it and cannot be handed back, which
        // this signature promises to do. Claiming first moves the refusal ahead
        // of the move, so the record survives it.
        match producer.claim() {
          Ok(mut reserved) => {
            let displaced = reserved.set(record);
            // A release build drops `displaced` here without a signal. If a freshly
            // claimed slot still held a record, that record is lost. The assert
            // stays until `ring_slot` offers a reservation type known to be empty.
            debug_assert!(displaced.is_none(), "a claimed slot held a record");
            Ok(())
          }
          Err(_) => Err(record),
        }
      }
      #[cfg(feature = "crossbeam")]
      ProducerInner::Crossbeam(queue) => {
        if self.overflow == OverflowPolicy::DropOldest {
          let _evicted = queue.force_push(record);
          return Ok(());
        }
        queue.push(record)
      }
    };

    match refused {
      Ok(()) => Ok(()),
      Err(record) => match would_resolve(self.overflow) {
        Resolution::DroppedIncoming => Ok(()),
        Resolution::EvictedOldest | Resolution::Refused => Err(record),
      },
    }
  }

  /// Publish as many of `records` as the ring will take, and report how many.
  ///
  /// Partial acceptance is the normal case, so the return is a count rather
  /// than a `Result`. An all-or-nothing contract would need a rollback the ring
  /// cannot offer cheaply. This call leaves the iterator positioned after the
  /// last record it consumed.
  ///
  /// ```
  /// use ring_config::RingConfig;
  /// use ring_core::Ring;
  /// use ring_types::OverflowPolicy;
  ///
  /// let config = RingConfig::new( 2 ).unwrap().with_overflow( OverflowPolicy::Fail );
  /// let mut ring : Ring< u8 > = Ring::new( &config ).unwrap();
  /// let mut ends = ring.ends();
  /// let ( mut producer, _consumer ) = ends.split();
  ///
  /// let mut records = [ 1, 2, 3, 4 ].into_iter();
  /// assert_eq!( producer.try_push_batch( &mut records ), 2 );
  /// assert_eq!( records.next(), Some( 4 ), "record 3 was consumed by the refusal" );
  /// ```
  #[must_use]
  pub fn try_push_batch(&mut self, records: &mut impl Iterator<Item = T>) -> usize {
    let mut accepted = 0;

    for record in records.by_ref() {
      if self.try_push(record).is_err() {
        break;
      }
      accepted += 1;
    }

    accepted
  }

  /// Another handle onto the same ring, where the backend permits one.
  ///
  /// `None` at SPSC cardinality. A second producer there is a data race that
  /// no signature would reveal, so refusing is the API's only honest answer.
  /// This is also the one machine-checkable way to tell which
  /// `free_capacity` contract applies.
  ///
  /// ```
  /// use ring_config::RingConfig;
  /// use ring_core::Ring;
  ///
  /// let one = RingConfig::new( 8 ).unwrap();
  /// let mut ring : Ring< u8 > = Ring::new( &one ).unwrap();
  /// let mut ends = ring.ends();
  /// let ( producer, _consumer ) = ends.split();
  /// assert!( producer.try_clone().is_none(), "SPSC permits exactly one producer" );
  /// ```
  #[must_use]
  pub fn try_clone(&self) -> Option<Producer<'a, T>> {
    match &self.inner {
      ProducerInner::Spsc(_) => None,
      ProducerInner::Mpsc(producer) => Some(Producer {
        inner: ProducerInner::Mpsc(*producer),
        overflow: self.overflow,
      }),
      #[cfg(feature = "crossbeam")]
      ProducerInner::Crossbeam(queue) => Some(Producer {
        inner: ProducerInner::Crossbeam(queue),
        overflow: self.overflow,
      }),
    }
  }

  /// Room for at least this many more records.
  ///
  /// **Binding at SPSC, advisory at MPSC and crossbeam.** This is the asymmetry
  /// the uniform API cannot express. Use [`try_clone`](Self::try_clone) to
  /// learn which reading applies, or treat every reading as advisory and let
  /// [`try_push`](Self::try_push) be the authority, which is always correct.
  #[must_use]
  pub fn free_capacity(&self) -> usize {
    match &self.inner {
      ProducerInner::Spsc(producer) => producer.free_capacity(),
      ProducerInner::Mpsc(producer) => producer.free_capacity(),
      #[cfg(feature = "crossbeam")]
      ProducerInner::Crossbeam(queue) => queue.capacity() - queue.len(),
    }
  }

  /// Whether the ring has no room, by the same reading as
  /// [`free_capacity`](Self::free_capacity).
  ///
  /// **Binding at SPSC, advisory at MPSC and crossbeam.** It inherits that
  /// asymmetry rather than resolving it, so at MPSC a reported `true` can be
  /// false by the time the caller branches on it. Let
  /// [`try_push`](Self::try_push) be the authority, which is always correct.
  #[must_use]
  pub fn is_full(&self) -> bool {
    self.free_capacity() == 0
  }
}

/// The reading end.
///
/// Exactly one exists per ring on every backend, so unlike [`Producer`] there
/// is no cardinality question and no `try_clone`.
#[derive(Debug)]
pub struct Consumer<'a, T> {
  inner: ConsumerInner<'a, T>,
}

#[derive(Debug)]
enum ConsumerInner<'a, T> {
  Spsc(ring_spsc::Consumer<'a, TypedSlot<T>>),
  Mpsc(ring_mpsc::Consumer<'a, TypedSlot<T>>),
  #[cfg(feature = "crossbeam")]
  Crossbeam(&'a crossbeam_queue::ArrayQueue<T>),
}

impl<T: Send> Consumer<'_, T> {
  /// Take the next record, if one is published.
  ///
  /// `Option`, not `Result`, because "nothing available" is the normal state of
  /// a ring drained at a barrier. Modelling it as an error makes every caller
  /// unwrap a non-failure.
  pub fn try_recv(&mut self) -> Option<T> {
    match &mut self.inner {
      ConsumerInner::Spsc(consumer) => {
        let mut batch = consumer.drain_up_to(1);
        batch.get_mut(0).and_then(TypedSlot::take)
      }
      ConsumerInner::Mpsc(consumer) => {
        let mut batch = consumer.drain_up_to(1);
        batch.get_mut(0).and_then(TypedSlot::take)
      }
      #[cfg(feature = "crossbeam")]
      ConsumerInner::Crossbeam(queue) => queue.pop(),
    }
  }

  /// Move everything currently published into `out`, and report how many.
  ///
  /// Bounded by what was published when the call began, not by what arrives
  /// during it. Otherwise it would not terminate under a live producer.
  ///
  /// ```
  /// use ring_config::RingConfig;
  /// use ring_core::Ring;
  ///
  /// let config = RingConfig::new( 8 ).unwrap();
  /// let mut ring : Ring< u8 > = Ring::new( &config ).unwrap();
  /// let mut ends = ring.ends();
  /// let ( mut producer, mut consumer ) = ends.split();
  ///
  /// assert_eq!( producer.try_push_batch( &mut [ 1, 2, 3 ].into_iter() ), 3 );
  ///
  /// let mut out = Vec::new();
  /// assert_eq!( consumer.try_recv_batch( &mut out ), 3 );
  /// assert_eq!( out, [ 1, 2, 3 ] );
  /// ```
  #[must_use]
  pub fn try_recv_batch(&mut self, out: &mut Vec<T>) -> usize {
    match &mut self.inner {
      ConsumerInner::Spsc(consumer) => {
        let mut batch = consumer.drain();
        let len = batch.len();
        out.extend((0..len).filter_map(|offset| batch.get_mut(offset).and_then(TypedSlot::take)));
        len
      }
      ConsumerInner::Mpsc(consumer) => {
        let mut batch = consumer.drain();
        let len = batch.len();
        out.extend((0..len).filter_map(|offset| batch.get_mut(offset).and_then(TypedSlot::take)));
        len
      }
      #[cfg(feature = "crossbeam")]
      ConsumerInner::Crossbeam(queue) => {
        // Bounded by the length read once, up front, and not by `pop` returning
        // `None`, which under a live producer may never happen.
        let len = queue.len();
        let taken = (0..len).filter_map(|_| queue.pop()).collect::<Vec<_>>();
        let taken_len = taken.len();
        out.extend(taken);
        taken_len
      }
    }
  }

  /// How many records are waiting, at least.
  ///
  /// A lower bound: it may grow between this read and the next drain. It never
  /// shrinks on its own, since this is the only consumer.
  ///
  /// **Binding at SPSC, advisory at MPSC and crossbeam.** This is the mirror
  /// image of [`Producer::free_capacity`](Producer::free_capacity), asymmetric
  /// for the same reason. Against one producer the reading cannot move under
  /// you, against several it can. Let [`try_recv`](Self::try_recv) be the
  /// authority, which is always correct.
  #[must_use]
  pub fn len(&self) -> usize {
    match &self.inner {
      ConsumerInner::Spsc(consumer) => consumer.available(),
      ConsumerInner::Mpsc(consumer) => consumer.available(),
      #[cfg(feature = "crossbeam")]
      ConsumerInner::Crossbeam(queue) => queue.len(),
    }
  }

  /// Whether nothing is waiting, by the same reading as [`len`](Self::len).
  ///
  /// **Binding at SPSC, advisory at MPSC and crossbeam.** A reported `true`
  /// can be false the instant a producer on another thread publishes.
  #[must_use]
  pub fn is_empty(&self) -> bool {
    self.len() == 0
  }
}
