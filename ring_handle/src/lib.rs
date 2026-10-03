//! Producer and consumer handles.
//!
//! Part of the ring family's concurrency write path.
//!
//! # What this crate adds
//!
//! The requirement is the ring's two ends as two separate values, with
//! capability following ownership. `ring_core` already partitions the
//! capabilities. Its `Producer` cannot drain and its `Consumer` cannot publish,
//! so this crate is not where that split is invented. This crate adds four
//! things that `ring_core` does not do, each of them a *narrowing*:
//!
//! | Added | Why `ring_core` does not have it |
//! |---|---|
//! | The ring is taken **by value** | `Ring::ends` borrows, so the caller keeps the ring and can split it again later. [`Split::new`] consumes it |
//! | `try_clone` is **withheld** | `ring_core::Producer::try_clone` exists and can succeed on an MPSC backend. A `ring_handle::Producer` cannot be duplicated at all |
//! | [`Consumer::drain`] | A drain whose bound is fixed at call time, so it terminates under a live producer |
//! | Nothing reaches the backend | No `Deref`, no `inner()`, no public field, on any of the three types |
//!
//! The overlap with `ring_core` is real, and
//! `docs/decisions/001_handles_are_a_narrowing_layer_over_ring_core.md`
//! documents it rather than minimising it.
//!
//! # Two things the pre-implementation spec asked for that are not here
//!
//! **`is_closed()` on both handles.** It would need a `ring_shutdown`
//! dependency, and `ring_shutdown` depends on `ring_wait`. That would put a
//! parking operation within reach of the tick path and break the rule that
//! the tick path only tries, asserted in `ring_poll`'s suite. Two requirements
//! in tension, resolved by measurement; see
//! `docs/decisions/002_handles_have_no_is_closed.md`.
//!
//! **`&self` receivers.** `&mut self` is what makes "exactly one producer"
//! hold. A shared `&Producer` could be used from two threads at once, which is
//! the cardinality violation `ring_spsc` cannot otherwise detect.

#![deny(missing_docs)]

use ring_core::Ring;

/// A ring that has been given up, and can now only be split.
///
/// Taking the ring **by value** is the whole mechanism. Afterwards there is no
/// route to the ring except through this value, and this value offers exactly
/// one operation.
///
/// ```
/// use ring_config::RingConfig;
/// use ring_core::Ring;
/// use ring_handle::Split;
///
/// let ring : Ring< u32 > = Ring::new( &RingConfig::new( 8 ).unwrap() ).unwrap();
/// let mut split = Split::new( ring );
/// let mut ends = split.ends();
/// let ( mut producer, mut consumer ) = ends.split();
///
/// producer.try_push( 1 ).unwrap();
/// assert_eq!( consumer.try_recv(), Some( 1 ) );
/// ```
#[derive(Debug)]
pub struct Split<T> {
  ring: Ring<T>,
}

impl<T: Send> Split<T> {
  /// Give up the ring.
  ///
  /// The ring is moved in and is not reachable again: not through a getter,
  /// not through `Deref`, and not by splitting twice, since [`Split::ends`]
  /// borrows exclusively.
  pub const fn new(ring: Ring<T>) -> Self {
    Self { ring }
  }

  /// Borrow the two ends, which [`Ends::split`] then separates.
  ///
  /// The two-step shape is inherited. The pair borrows from the `Ends` value,
  /// so the caller has to hold it. Collapsing the steps would need a
  /// self-referential struct.
  pub fn ends(&mut self) -> Ends<'_, T> {
    Ends { inner: self.ring.ends() }
  }
}

/// The two ends, before they are separated.
#[derive(Debug)]
pub struct Ends<'a, T> {
  inner: ring_core::Ends<'a, T>,
}

impl<'a, T: Send> Ends<'a, T> {
  /// Separate the ends into a publishing handle and a draining one.
  ///
  /// Exactly one of each. There is no operation here that yields a second
  /// producer, which is the difference from `ring_core::Producer::try_clone`.
  pub fn split(&'a mut self) -> (Producer<'a, T>, Consumer<'a, T>) {
    let (producer, consumer) = self.inner.split();
    (Producer { inner: producer }, Consumer { inner: consumer })
  }
}

// ── Producer ──────────────────────────────────────────────────────────────

/// The publishing end. Cannot drain, cannot be cloned, cannot reach the ring.
///
/// The absences are the contract; the present methods are ordinary forwarding.
#[derive(Debug)]
pub struct Producer<'a, T> {
  inner: ring_core::Producer<'a, T>,
}

impl<T: Send> Producer<'_, T> {
  /// Publish one record, or hand it back.
  ///
  /// Never blocks. On a full ring the ring's own `OverflowPolicy` decides:
  /// `Fail`, the default, returns the record. On a ring built with
  /// `DropNewest` it is discarded and reported as success, so there an `Ok` is
  /// not by itself evidence the record was kept.
  ///
  /// # Errors
  ///
  /// Returns the record when the ring is full and the policy refuses.
  pub fn try_push(&mut self, record: T) -> Result<(), T> {
    self.inner.try_push(record)
  }

  /// Publish as many records as the ring will take, and hand back the one it
  /// refused.
  ///
  /// The same refusal rule as [`try_push`](Self::try_push): `Err` carries what
  /// did not go in. `Ok( n )` means nothing was refused and the iterator ran
  /// dry. On a ring built with `DropNewest` the result is always `Ok`, and `n`
  /// counts the records the policy discarded as well as the ones it kept.
  ///
  /// # Errors
  ///
  /// `( n, record )` when `n` records went in and then the ring refused
  /// `record`. The iterator resumes after it, so `record` followed by whatever
  /// the iterator still yields is everything that was not published, in order.
  ///
  /// ```
  /// use ring_config::RingConfig;
  /// use ring_core::Ring;
  /// use ring_handle::Split;
  /// use ring_types::OverflowPolicy;
  ///
  /// let config = RingConfig::new( 4 ).unwrap().with_overflow( OverflowPolicy::Fail );
  /// let ring : Ring< u32 > = Ring::new( &config ).unwrap();
  /// let mut split = Split::new( ring );
  /// let mut ends = split.ends();
  /// let ( mut producer, _consumer ) = ends.split();
  ///
  /// let mut records = 0..10;
  /// assert_eq!( producer.try_push_batch( &mut records ), Err( ( 4, 4 ) ) );
  /// assert_eq!( records.next(), Some( 5 ) );
  /// ```
  pub fn try_push_batch(&mut self, records: &mut impl Iterator<Item = T>) -> Result<usize, (usize, T)> {
    self.inner.try_push_batch(records)
  }

  /// How much room there is, as a lower bound.
  ///
  /// Binding at SPSC cardinality, where this is the only producer; advisory at
  /// MPSC, where another producer may take the room first. One signature, two
  /// contracts, selected by a config field. See
  /// `ring_core/docs/decisions/001_free_capacity_keeps_one_signature_across_backends.md`.
  #[must_use]
  pub fn free_capacity(&self) -> usize {
    self.inner.free_capacity()
  }

  /// Whether there is no room at all.
  #[must_use]
  pub fn is_full(&self) -> bool {
    self.inner.is_full()
  }
}

// ── Consumer ──────────────────────────────────────────────────────────────

/// The draining end. Cannot publish, cannot be cloned, cannot reach the ring.
///
/// Because there is exactly one of these, whoever holds it *is* the consume
/// point. That makes where this value lives a correctness question and
/// not only a design one.
#[derive(Debug)]
pub struct Consumer<'a, T> {
  inner: ring_core::Consumer<'a, T>,
}

impl<'a, T: Send> Consumer<'a, T> {
  /// Take one record, if one is waiting.
  ///
  /// `Option` rather than `Result`, because an empty ring is not a failure, and
  /// modelling it as one makes every caller unwrap a non-failure.
  pub fn try_recv(&mut self) -> Option<T> {
    self.inner.try_recv()
  }

  /// Take whatever is waiting into `out`, and report how many moved.
  pub fn try_recv_batch(&mut self, out: &mut Vec<T>) -> usize {
    self.inner.try_recv_batch(out)
  }

  /// Take the records published *before this call*, as an iterator.
  ///
  /// **The bound is fixed here, not as the iterator runs.** An iterator that
  /// kept yielding whatever arrived would have no termination guarantee under a
  /// live producer. That is a live-lock that satisfies "never blocks" at every
  /// individual step. Reading the length once, up front, is what makes a full
  /// drain a bounded operation and what makes it reproducible given a fixed
  /// publication history.
  ///
  /// ```
  /// use ring_config::RingConfig;
  /// use ring_core::Ring;
  /// use ring_handle::Split;
  ///
  /// let ring : Ring< u32 > = Ring::new( &RingConfig::new( 8 ).unwrap() ).unwrap();
  /// let mut split = Split::new( ring );
  /// let mut ends = split.ends();
  /// let ( mut producer, mut consumer ) = ends.split();
  /// producer.try_push_batch( &mut ( 0..3 ) ).unwrap();
  ///
  /// let taken : Vec< u32 > = consumer.drain().collect();
  /// assert_eq!( taken, vec![ 0, 1, 2 ] );
  /// ```
  pub fn drain(&mut self) -> Drain<'_, 'a, T> {
    let remaining = self.inner.len();
    Drain {
      consumer: self,
      remaining,
    }
  }

  /// How many records are waiting, as a lower bound.
  ///
  /// It can only grow, never shrink, because this is the only consumer. That is the
  /// opposite direction from `Producer::free_capacity`'s bound at MPSC
  /// cardinality. The two read as duals and are not.
  #[must_use]
  pub fn len(&self) -> usize {
    self.inner.len()
  }

  /// Whether nothing is waiting.
  #[must_use]
  pub fn is_empty(&self) -> bool {
    self.inner.is_empty()
  }
}

/// The iterator [`Consumer::drain`] returns, bounded at the call that made it.
#[derive(Debug)]
pub struct Drain<'c, 'a, T> {
  consumer: &'c mut Consumer<'a, T>,
  remaining: usize,
}

impl<T: Send> Iterator for Drain<'_, '_, T> {
  type Item = T;

  fn next(&mut self) -> Option<T> {
    if self.remaining == 0 {
      return None;
    }
    let record = self.consumer.try_recv()?;
    self.remaining -= 1;
    Some(record)
  }

  fn size_hint(&self) -> (usize, Option<usize>) {
    (0, Some(self.remaining))
  }
}
