//! Single-producer single-consumer ring API.
//!
//! Part of the ring family's concurrency write path.
//!
//! # What single-producer buys, stated as a dependency list
//!
//! `ring_gating`, `ring_claim`, `ring_publish` and `ring_consume` are all
//! absent. The absence is the crate's thesis, not an omission. Each of those
//! crates exists to answer a question that only has an answer worth
//! computing when producers can overtake one another:
//!
//! - `ring_gating` takes the minimum over a set of consumer cursors. At one
//!   consumer the minimum over one value is that value.
//! - `ring_claim` moves the producer cursor by compare-exchange, because two
//!   producers may race for the same range. At one producer nothing races, so
//!   a plain store moves the cursor.
//! - `ring_publish` spins until the frontier reaches this range's start,
//!   because producer B may finish before producer A. At one producer, the
//!   frontier is always already there.
//! - `ring_consume` gates a commit on a `Barrier` over the cursors a batch
//!   depended on. At one consumer that barrier has one member, and it is this
//!   end's own cursor.
//!
//! What is left is two cursors and an array. No per-slot stamp, no
//! read-modify-write on any path, no loop that can spin on another thread's
//! progress.
//!
//! # Orderings, and why they are not uniform
//!
//! Each cursor has exactly one writer. That is why an end can read **its
//! own** cursor at [`OWN`] (`Relaxed`). This thread performed the store it is
//! reading back, and program order already sequences the load after it. The
//! end reads the **other** end's cursor at [`ring_cursor::GATING`]
//! (`Acquire`), and that load is what makes the peer's slot writes visible.
//!
//! [`ring_cursor::CursorPair`] offers `free_slots`, `pending` and `may_claim`
//! over the same two cursors, and this crate chooses not to call them in the
//! hot path. They read *both* cursors at `Acquire`, which is correct for them
//! because a `CursorPair` does not know which end is asking. Here every caller
//! does know, so the asymmetric form is available, and the symmetric one
//! would pay for a fence the single-writer property already provides.
//!
//! # The producer's cursor cache
//!
//! `claim`'s fast path loads nothing. The producer keeps the next sequence to
//! claim and its latest observation of the consumer cursor in its own fields —
//! lawful because the end is `&mut`-exclusive — and grants from the two cached
//! values. The observation can only lag the real cursor: the consumer's cursor
//! is monotone and the cache is refreshed to it, never extrapolated past it. A
//! claim is granted only when the lagged value itself proves the slot consumed
//! (`cached_head > seq − capacity`), so staleness can refuse a ring that has
//! room, and never overwrite a record the consumer has not taken.
//!
//! When the cache says full, one [`GATING`] load refreshes it — the single
//! consumer-line read the fast path exists to avoid — and that load is also
//! the edge that orders the consumer's take of the old record before the
//! producer's overwrite of the slot: the take precedes the consumer's commit
//! store, the refresh observes that store, and the write follows the refresh.
//! Every claim of a lapped slot is therefore backed by an observation, never
//! by hope. The load's `Acquire` is soundness-required, and it is the one part
//! of the protocol the loom models cannot check: loom linearises each step, so
//! a relaxed refresh that returns the committed value is indistinguishable
//! from an acquired one there, while on hardware without the edge the
//! overwrite and the take can race.
//!
//! # The unsafe, and where its argument lives
//!
//! The producer writes slots while the consumer reads slots, through shared
//! references to one allocation. That is interior mutability across a thread
//! boundary, and `unsafe` is the only way to express it.
//! `docs/workaround/readme.md` records that it belongs here, not in
//! `ring_store` or `ring_slot`, because the invariant that makes it sound is
//! stated entirely in terms of cursors those crates do not hold. The argument
//! is in `Ring::slot`'s and `Ring::slot_mut`'s safety sections. The shape it
//! rests on is asserted as well as described, partly by `tests/spsc_test.rs`
//! and partly by the four cases below.
//!
//! # What the type system refuses
//!
//! `slot_mut`'s safety section says "the caller must be the producer end", and
//! that is worth nothing if a second producer is constructible. The properties
//! that make one unrepresentable are all *negative*: an absent `Clone`, an
//! absent `Sync`, a borrow that outlives nothing. None of them can be asserted
//! at runtime, so this crate checks them here as `compile_fail` doc tests,
//! the only executable form a negative has.
//!
//! These live in the library, not in `tests/spsc_test.rs`, because rustdoc
//! collects doc tests from the library target only. The same five blocks
//! written in an integration test file are never compiled, and would be five
//! checks that silently check nothing. This is not hypothetical. They were
//! written there first, and the doc test count did not move.
//!
//! A second producer cannot be made by cloning the first:
//!
//! ```compile_fail
//! use ring_slot::TypedSlot;
//! use ring_spsc::Ring;
//! use ring_types::Capacity;
//!
//! let mut ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 4 ).unwrap() );
//! let ( producer, _consumer ) = ring.split();
//! let _second = producer.clone();
//! ```
//!
//! Nor by splitting twice while the first pair is still live, because the pair
//! borrows the ring for as long as either end is used:
//!
//! ```compile_fail
//! use ring_slot::TypedSlot;
//! use ring_spsc::Ring;
//! use ring_types::Capacity;
//!
//! let mut ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 4 ).unwrap() );
//! let ( mut producer, _consumer ) = ring.split();
//! let ( _again, _also ) = ring.split();
//! producer.try_push( 1 ).unwrap();
//! ```
//!
//! Nor by sharing one end with a second thread. `!Sync` is what stops it, and
//! these two blocks state the bound itself instead of a `thread::spawn` that
//! would demonstrate it:
//!
//! ```compile_fail
//! use ring_slot::TypedSlot;
//! use ring_spsc::Producer;
//!
//! fn assert_sync< T : Sync >() {}
//! assert_sync::< Producer< 'static, TypedSlot< u8 > > >();
//! ```
//!
//! ```compile_fail
//! use ring_slot::TypedSlot;
//! use ring_spsc::Consumer;
//!
//! fn assert_sync< T : Sync >() {}
//! assert_sync::< Consumer< 'static, TypedSlot< u8 > > >();
//! ```
//!
//! And a drained record cannot outlive the commit that frees the slot it points
//! into. That property makes the borrowed batch sound, not only convenient:
//!
//! ```compile_fail
//! use ring_slot::TypedSlot;
//! use ring_spsc::Ring;
//! use ring_types::Capacity;
//!
//! let mut ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 4 ).unwrap() );
//! let ( mut producer, mut consumer ) = ring.split();
//! producer.try_push( 1 ).unwrap();
//!
//! let escaped = { let batch = consumer.drain(); batch.get( 0 ).unwrap() };
//! let _ = escaped;
//! ```
//!
//! # Shape
//!
//! ```
//! use ring_slot::TypedSlot;
//! use ring_spsc::Ring;
//! use ring_types::Capacity;
//!
//! let mut ring : Ring< TypedSlot< u32 > > = Ring::new( Capacity::new( 8 ).unwrap() );
//! let ( mut producer, mut consumer ) = ring.split();
//!
//! producer.try_push( 1 ).unwrap();
//! producer.try_push( 2 ).unwrap();
//!
//! let batch = consumer.drain();
//! assert_eq!( batch.len(), 2 );
//! assert_eq!( batch.get( 0 ).and_then( TypedSlot::get ), Some( &1 ) );
//! ```
//!
//! Acceptance is binary and lives in a test, not here. The crate's feature is
//! Reached when `tests/spsc_test.rs` exchanges 100 000 items with byte-parity,
//! in order, with zero loss and no lock in the path, and cites the feature
//! textually. That citation is the only crate→feature edge the family records.
//!
//! # Pitfall: what holds here does not carry over to `ring_mpsc`
//!
//! **Trap.** Carrying a property of this crate into `ring_mpsc`, where more than
//! one producer can be racing the same claim. The producer cursor as the
//! published frontier, a claim that cannot be overtaken, a binding
//! [`Producer::free_capacity`], and a claim that finishes in a bounded number of
//! steps all hold here only because there is one producer. None holds there,
//! and `ring_mpsc`'s module documentation explains why its claim is lock-free
//! and not wait-free.
//!
//! **Failure.** Each one breaks only with more than one producer, so a test run
//! with one passes. The two crates hand out their ends differently (`split`
//! here, `ends` then `split` there), so moved code does not compile unchanged,
//! but moved reasoning raises no error.
//!
//! **Mitigation.** Code meant for both rings treats
//! [`Producer::free_capacity`] as a hint, handles [`RingError::Full`] on every
//! push, learns what is published from what a drain returns, and does not
//! assume a push takes a bounded number of steps. The reverse direction is
//! safe. Reasoning honed on `ring_mpsc` is conservative in this crate.

#![deny(missing_docs)]
#![allow(unsafe_code)]

use core::cell::{Cell, UnsafeCell};
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::Ordering;

use ring_config::RingConfig;
use ring_cursor::{CursorPair, GATING, SeqCell};
use ring_slot::{Slot, TypedSlot};
use ring_store::Buffer;
use ring_types::{Capacity, RingError, Seq};

/// The ordering an end reads **its own** cursor with.
///
/// `Relaxed` is sound because each cursor has exactly one writer. The thread
/// performing this load is the thread that performed the store it is reading
/// back, and program order already sequences the two. No inter-thread edge is
/// being established, so none needs to be paid for.
///
/// An end moved to another thread breaks the second sentence above. The
/// loading thread is then not the one that stored. It stays sound because each
/// end is `Send` and not `Sync`, so the move is the only way to hand it over,
/// and every safe way to move a value between threads (spawning, joining, a
/// channel, a mutex) carries the happens-before edge this crate does not
/// supply.
///
/// Contrast [`ring_cursor::GATING`], the ordering an end reads the *other*
/// end's cursor with. There the load is what makes the peer's slot writes
/// visible, so `Acquire` cannot be weakened.
///
/// ```
/// use core::sync::atomic::Ordering;
///
/// assert_eq!( ring_spsc::OWN, Ordering::Relaxed );
/// assert_eq!( ring_cursor::GATING, Ordering::Acquire );
/// ```
pub const OWN: Ordering = Ordering::Relaxed;

/// The ordering a publication and a commit are made visible with.
///
/// One release store ends every producer operation and every consumer batch.
/// It is the only synchronization either path performs, and it pairs with
/// the peer's [`ring_cursor::GATING`] load. Everything this thread wrote to
/// slots before the store is visible to a peer that observes the new cursor
/// value.
///
/// ```
/// use core::sync::atomic::Ordering;
///
/// assert_eq!( ring_spsc::HANDOFF, Ordering::Release );
/// ```
pub const HANDOFF: Ordering = Ordering::Release;

/// A ring one thread writes and one thread reads.
///
/// Three fields' worth of state in two: the slot array, and a
/// [`CursorPair`] holding the producer and consumer cursors on separate cache
/// lines. The ring derives everything else it needs instead of storing it:
/// occupancy, free capacity, and the slot a sequence maps to.
///
/// The ring owns the storage; the two ends [`split`] hands out borrow it. The
/// ring itself deliberately has no push or drain methods. Those would mean a
/// `&Ring` from which a second producer could be made, and one producer is the
/// whole precondition.
///
/// [`split`]: Self::split
///
/// # Lifecycle: one allocation, derived slot states, and teardown
///
/// The slot array is allocated once, in [`new`](Self::new). Nothing a producer
/// or consumer does allocates, so a long-running ring's memory stays flat and a
/// borrowed [`Batch`] can never be invalidated by the ring growing.
///
/// A slot's state is never stored either. Free, published and drained follow
/// from comparing a sequence with the two cursors, and claimed means a live
/// [`Reservation`] holds it. The sequence, not the slot, says which lap a
/// record belongs to. A slot is writable again the moment the consumer's
/// commit passes it, with no write to the slot. A reset step there would cost a
/// store and open a window in which the slot is in neither state.
///
/// Neither end has a destructor and neither owns the storage. The ring releases
/// it when it goes out of scope, which the borrow checker places after both
/// ends, so the order the ends go out of scope in cannot matter. Dropping one
/// end leaves the other usable. A consumer whose producer is gone can still
/// drain everything that was published.
///
/// Records still in the ring are dropped with it, on whichever thread drops the
/// ring. A record left in a slot otherwise goes when a later lap's write
/// replaces it. [`TypedSlot::set`] hands it back, and [`Producer::try_push`]
/// drops it there. Leaking them instead would skip the destructor of every
/// record still in a slot at each teardown, and refusing to drop a non-empty
/// ring would mean a panic in a destructor.
///
/// `every_record_written_is_dropped_exactly_once`,
/// `the_ends_going_out_of_scope_in_either_order_releases_the_storage_once` and
/// `a_departed_producer_leaves_the_published_tail_drainable` in
/// `tests/spsc_test.rs` check the teardown.
///
/// ```
/// use ring_slot::BytesSlot;
/// use ring_spsc::Ring;
/// use ring_types::Capacity;
///
/// let ring : Ring< BytesSlot< 16 > > = Ring::new( Capacity::new( 4 ).unwrap() );
/// assert_eq!( ring.capacity().get(), 4 );
/// assert!( ring.on_distinct_lines(), "the two cursors do not share a line" );
/// ```
pub struct Ring<S> {
  /// The slot array, behind an `UnsafeCell` because the producer writes into it
  /// through a shared reference while the consumer reads from it.
  ///
  /// The cell is on each slot, not around the whole `Buffer`.
  ///
  /// It wrapped the buffer until `Buffer::new` was bounded on `Default` rather
  /// than `Slot`; the `Slot` bound was the only thing making
  /// `Buffer< UnsafeCell< S > >` unconstructible. The outer form was unsound.
  /// Reaching a slot through `( *cell.get() ).at_mut( seq )` materialises
  /// `&mut Buffer< S >`, an exclusive claim over the *entire* allocation, so a
  /// producer writing one slot and a consumer reading a different one aliased
  /// the whole buffer. Miri's data-race detector reports it as a retag
  /// conflict on `Buffer< S >` itself, not on any slot. The two threads never
  /// touched the same record. A per-slot cell claims exactly the slot being
  /// touched, and the cursor invariant below guarantees the touched slots are
  /// disjoint.
  ///
  /// `ring_store` still knows nothing of this crate. It stores whatever
  /// element type it is given, and every `unsafe` stays here, which is the
  /// whole point of the opt-out in `docs/workaround/readme.md`.
  slots: Buffer<UnsafeCell<S>>,
  cursors: CursorPair,
}

// SAFETY: `Ring` is only ever shared between exactly two threads, and only as
// the `&Ring` held by a `Producer` and a `Consumer`. `split` takes `&mut
// self`, so no second pair can exist while a first is alive, and neither end is
// `Clone` or `Sync`. Those two threads touch disjoint slots at every instant:
// the producer only writes the slot for the sequence it has claimed and not yet
// published, the consumer only reads slots that are published and not yet
// committed, and a published-not-committed sequence is by construction below
// the producer's current position. The claim's capacity decision runs on the
// producer's cached cursors, whose soundness is the monotonicity recorded on
// the `cached_head` field: the cache can only lag the consumer cursor, and a
// lapped claim is granted only when the cached observation itself proves the
// record consumed, at the `GATING` load that orders the consumer's take before
// the producer's overwrite. The cursors are atomics and carry the
// happens-before edge (`HANDOFF` store, `GATING` load) that makes each side's
// slot writes visible to the other before the other may touch that slot.
// `S : Send` is required because a record is written on the producer's thread
// and read on the consumer's.
//
// `ring_mpsc` declares this identical line, but justifies it by many producers
// on disjoint slots, not by exactly two threads. The hazard is reading one
// crate's SAFETY comment while editing the other's
// (→ `docs/workaround/readme.md`).
unsafe impl<S: Send> Sync for Ring<S> {}

impl<S: Slot + Default> Ring<S> {
  /// Allocate a ring of `capacity` slots, with both cursors at zero.
  ///
  /// Sequence zero, which is also `Seq::default()`, is therefore the first real
  /// record, not a sentinel for "no sequence". Code that needs "no sequence"
  /// says `Option<Seq>`.
  ///
  /// ```
  /// use ring_slot::TypedSlot;
  /// use ring_spsc::Ring;
  /// use ring_types::Capacity;
  ///
  /// let ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 16 ).unwrap() );
  /// assert_eq!( ring.capacity().get(), 16 );
  /// ```
  #[must_use]
  pub fn new(capacity: Capacity) -> Self {
    Self {
      slots: Buffer::new(capacity),
      cursors: CursorPair::new(capacity),
    }
  }

  /// Allocate a ring sized by a [`RingConfig`].
  ///
  /// This reads only the capacity. A config's wait strategy and overflow
  /// policy describe what a *caller* does when the ring is full or empty, and
  /// this crate never waits and never drops. It only reports.
  ///
  /// ```
  /// use ring_config::RingConfig;
  /// use ring_slot::TypedSlot;
  /// use ring_spsc::Ring;
  ///
  /// let config = RingConfig::new( 32 ).unwrap();
  /// let ring : Ring< TypedSlot< u8 > > = Ring::with_config( &config );
  /// assert_eq!( ring.capacity(), config.capacity() );
  /// ```
  #[must_use]
  pub fn with_config(config: &RingConfig) -> Self {
    Self::new(config.capacity())
  }
}

impl<S> Ring<S> {
  /// How many slots the ring holds.
  ///
  /// ```
  /// use ring_slot::TypedSlot;
  /// use ring_spsc::Ring;
  /// use ring_types::Capacity;
  ///
  /// let ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 64 ).unwrap() );
  /// assert_eq!( ring.capacity().get(), 64 );
  /// ```
  #[must_use]
  pub const fn capacity(&self) -> Capacity {
    self.cursors.capacity()
  }

  /// Whether the two cursors actually occupy different cache lines.
  ///
  /// Delegated to [`CursorPair::on_distinct_lines`], and re-exposed here so a
  /// test can assert the property about the ring it is measuring rather than
  /// about a type it happens to contain.
  ///
  /// ```
  /// use ring_slot::TypedSlot;
  /// use ring_spsc::Ring;
  /// use ring_types::Capacity;
  ///
  /// let ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 2 ).unwrap() );
  /// assert!( ring.on_distinct_lines() );
  /// ```
  #[must_use]
  pub fn on_distinct_lines(&self) -> bool {
    self.cursors.on_distinct_lines()
  }

  /// The two ends, to be moved onto two threads.
  ///
  /// **`&mut self` is what enforces the cardinality.** The returned ends borrow
  /// the ring for as long as they live, so a second call cannot happen while a
  /// first pair exists. At no moment do two `Producer` values name one ring.
  /// Neither end is `Clone` and neither is `Sync`, so neither can be
  /// shared with a second thread after the split either.
  ///
  /// Once the first pair is gone the ring may be split again, and the cursors
  /// carry over. The second pair continues where the first stopped, not at zero,
  /// as `a_second_pair_may_be_split_once_the_first_is_gone` in
  /// `tests/spsc_test.rs` checks.
  ///
  /// ```
  /// use ring_slot::TypedSlot;
  /// use ring_spsc::Ring;
  /// use ring_types::Capacity;
  ///
  /// let mut ring : Ring< TypedSlot< u32 > > = Ring::new( Capacity::new( 4 ).unwrap() );
  /// let ( mut producer, mut consumer ) = ring.split();
  ///
  /// std::thread::scope( | scope |
  /// {
  ///   scope.spawn( move || producer.try_push( 7 ).unwrap() );
  ///   scope.spawn( move ||
  ///   {
  ///     while consumer.is_empty() { std::thread::yield_now(); }
  ///     assert_eq!( consumer.drain().len(), 1 );
  ///   } );
  /// } );
  /// ```
  pub fn split(&mut self) -> (Producer<'_, S>, Consumer<'_, S>) {
    let shared: &Self = self;

    // The tail cache must start at the shared cursor's actual value: on a ring
    // split a second time it carries over, and a cache that restarted at zero
    // would re-grant sequences the first generation published. The head cache
    // starts at zero on purpose — it can only lag a nonzero consumer cursor,
    // and the first full ring refreshes it.
    (
      Producer {
        ring: shared,
        cached_tail: shared.cursors.producer().load(OWN),
        cached_head: Seq::ZERO,
        _one_thread: PhantomData,
      },
      Consumer {
        ring: shared,
        _one_thread: PhantomData,
      },
    )
  }

  /// The slot `seq` maps to, for reading.
  ///
  /// # Safety
  ///
  /// The caller must be the consumer end, and `seq` must be **published and not
  /// yet committed**, meaning at or after this end's own cursor and strictly
  /// before the producer's, as read at [`GATING`]. Those two bounds place the
  /// slot outside the producer's writable set: the producer writes only at its
  /// own cursor position, which is at or above the exclusive upper bound here.
  ///
  /// The `GATING` load that established the upper bound is also what makes the
  /// producer's write to this slot visible; reading a slot on the strength of a
  /// bound obtained any other way is a data race even if the arithmetic holds.
  unsafe fn slot(&self, seq: Seq) -> &S {
    // SAFETY: the caller guarantees `seq` is published and not committed, so no
    // `&mut` to this slot exists. The producer's only outstanding `&mut` is to
    // its own claimed sequence, which is at or above the published frontier.
    // `at` yields `&UnsafeCell< S >`, a shared borrow of one slot rather than
    // of the buffer, so the producer's concurrent write to a different slot is
    // not an alias of this borrow.
    unsafe { &*self.slots.at(seq).get() }
  }

  /// The slot `seq` maps to, for writing.
  ///
  /// # Safety
  ///
  /// The caller must be the slot's sole owner for the returned reference's
  /// whole life, meaning no other `&S`/`&mut S` to the same slot may be live at
  /// the same time. Two call sites establish that, each on its own terms. An
  /// audit of this function recorded the finding that a reader who stops at
  /// the first is left thinking the second one violates this doc. The two are:
  ///
  /// - **The producer end, on a claimed-not-yet-published `seq`.** The `seq`
  ///   equals this end's own cursor, which the producer has not yet advanced.
  ///   That places the slot outside the consumer's readable set, whose
  ///   exclusive upper bound is that same cursor value. The slot must also be
  ///   free, meaning at least one lap behind the consumer's cursor; `claim`
  ///   is the only such caller, and it checks that through its cached
  ///   observation of the consumer cursor, whose value was read at [`GATING`]
  ///   and can only lag the real one — so a granted claim never overwrites a
  ///   record the consumer has not taken.
  /// - **The consumer end, on a published-not-yet-committed `seq`.** It is
  ///   reached through [`Batch::get_mut`], between `drain`/`drain_up_to` and
  ///   the batch's `Drop`. Publication is the producer cursor advancing past
  ///   `seq`, which is also the producer releasing its own claim on it, and the
  ///   producer cannot lap back onto `seq` until this batch's `Drop` advances
  ///   the consumer cursor past it in turn.
  ///
  /// Returning `&mut S` from `&self` is the one place the two-thread design is
  /// not expressible in safe Rust, and it is why `docs/workaround/readme.md`
  /// places the opt-out in this crate.
  #[allow(clippy::mut_from_ref)]
  unsafe fn slot_mut(&self, seq: Seq) -> &mut S {
    // SAFETY: the caller is the slot's sole owner under one of the two regimes
    // documented above (the producer end on a claimed-not-yet-published `seq`,
    // or the consumer end on a published-not-yet-committed `seq` reached
    // through `Batch::get_mut`), so no other `&S`/`&mut S` to this slot is
    // live. `at` yields `&UnsafeCell< S >`, so the write permission this deref
    // needs comes from that one slot's cell and claims nothing about any other
    // slot.
    unsafe { &mut *self.slots.at(seq).get() }
  }
}

impl<S> core::fmt::Debug for Ring<S> {
  /// Cursor positions and capacity, never slot contents.
  ///
  /// Formatting the slots would read every one of them, including the ones the
  /// producer may be writing this instant, which is the data race the whole
  /// crate is arranged to avoid. A `Debug` that is unsound to call from the
  /// consumer's thread would be a trap, so it does not exist.
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.debug_struct("Ring")
      .field("capacity", &self.cursors.capacity().get())
      .field("produced", &self.cursors.producer().load(GATING))
      .field("consumed", &self.cursors.consumer().load(GATING))
      .finish_non_exhaustive()
  }
}

/// The writing end.
///
/// Not `Clone` and not `Sync`, which is what makes "exactly one producer" a
/// fact about the program rather than a documented precondition. `Send`, so it
/// can be moved onto a thread.
///
/// This declaration's exact text is pinned outside this crate. A `ring_handle`
/// trybuild fixture (`tests/ui/producer_shared_across_threads.stderr`) expects
/// this line verbatim in a compiler error, and no dependency edge records
/// why.
#[derive(Debug)]
pub struct Producer<'a, S> {
  ring: &'a Ring<S>,
  /// The next sequence this end will claim — the producer cursor's private
  /// mirror. Advanced by [`claim`](Self::claim), published to the shared
  /// cursor by `Reservation::drop`, and equal to that cursor's value whenever
  /// no reservation is live. Being a plain field is what makes the fast path
  /// load-free: the end is `&mut`-exclusive, so nothing else reads or writes
  /// it.
  cached_tail: Seq,
  /// This end's latest observation of the consumer cursor, taken at
  /// [`GATING`]. Monotone and conservative: it is only ever set to a value the
  /// consumer's cursor actually held, so it can lag that cursor but never run
  /// ahead of it, and a claim granted on it can never overwrite a record the
  /// consumer has not taken.
  cached_head: Seq,
  /// `Cell` is `Send` and not `Sync`, so this marker makes the end movable to a
  /// thread and unshareable between two. Without it the end would inherit
  /// `Sync` from `&Ring`, and two threads holding `&Producer` could each claim
  /// the same sequence.
  _one_thread: PhantomData<Cell<()>>,
}

impl<S> Producer<'_, S> {
  /// How far this end has published.
  ///
  /// ```
  /// use ring_slot::TypedSlot;
  /// use ring_spsc::Ring;
  /// use ring_types::{ Capacity, Seq };
  ///
  /// let mut ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 4 ).unwrap() );
  /// let ( mut producer, _consumer ) = ring.split();
  ///
  /// assert_eq!( producer.position(), Seq::ZERO );
  /// producer.try_push( 1 ).unwrap();
  /// assert_eq!( producer.position(), Seq( 1 ) );
  /// ```
  #[must_use]
  pub fn position(&self) -> Seq {
    self.ring.cursors.producer().load(OWN)
  }

  /// How many pushes are guaranteed to succeed right now.
  ///
  /// **Actionable rather than advisory, and that is SPSC-specific.** With one
  /// producer nothing can take the reported space between the check and the
  /// push, so a reported `n` guarantees the next `n` claims succeed. The same
  /// call on a multi-producer ring is a hint
  /// (→ `ring_core/docs/decisions/001_free_capacity_keeps_one_signature_across_backends.md`).
  ///
  /// ```
  /// use ring_slot::TypedSlot;
  /// use ring_spsc::Ring;
  /// use ring_types::Capacity;
  ///
  /// let mut ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 4 ).unwrap() );
  /// let ( mut producer, _consumer ) = ring.split();
  ///
  /// assert_eq!( producer.free_capacity(), 4 );
  /// producer.try_push( 1 ).unwrap();
  /// assert_eq!( producer.free_capacity(), 3 );
  /// ```
  ///
  /// Fix(free_capacity_underflow_on_a_precondition_violation): this subtraction
  /// used to be `self.ring.capacity().get() - self.occupancy() as usize`,
  /// unguarded. It was one of eight unguarded subtractions left in
  /// `ring/ring_*/src`. `occupancy()` cannot itself underflow
  /// (`Seq::distance_to` is `saturating_sub`), but occupancy can exceed
  /// capacity once the producer laps the consumer. That lapped state is an
  /// unenforceable-at-runtime consequence of a caller violating the
  /// single-producer precondition, not something this crate can check. In that
  /// state this line panicked in a dev build and wrapped to a near-`usize::MAX`
  /// value in release (the workspace sets no `overflow-checks` anywhere).
  /// Root cause: the guard that would make the subtraction sound
  /// (`occupancy() <= capacity`) is structural, enforced by `ring_handle`'s
  /// non-`Clone` handles above this crate rather than checked at this call
  /// site, so nothing here stood between the precondition's absence and the
  /// arithmetic. Pitfall: a subtraction guarded only by a precondition the
  /// crate cannot itself verify is indistinguishable, at the call site, from
  /// one that is always safe. `saturating_sub` makes the lapped case return `0`
  /// (still an accurate answer, since zero slots are safely claimable) instead
  /// of panicking or lying. That has zero behavioural cost for every
  /// contract-respecting caller, and matches the convention
  /// `ring_seqno::free_slots` already uses for the same computation.
  #[must_use]
  pub fn free_capacity(&self) -> usize {
    self.ring.capacity().get().saturating_sub(self.occupancy() as usize)
  }

  /// Whether the next claim would fail.
  ///
  /// The check reads the real cursors, not this end's cached observation. The
  /// two can disagree while the cache is stale; `claim` resolves that by
  /// refreshing before it refuses, so both answers of this check hold — a
  /// full report means the next claim fails, and a not-full report means it
  /// succeeds, possibly after the refresh.
  ///
  /// ```
  /// use ring_slot::TypedSlot;
  /// use ring_spsc::Ring;
  /// use ring_types::Capacity;
  ///
  /// let mut ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 2 ).unwrap() );
  /// let ( mut producer, _consumer ) = ring.split();
  ///
  /// assert!( !producer.is_full() );
  /// producer.try_push( 1 ).unwrap();
  /// producer.try_push( 2 ).unwrap();
  /// assert!( producer.is_full() );
  /// ```
  #[must_use]
  pub fn is_full(&self) -> bool {
    self.occupancy() as usize == self.ring.capacity().get()
  }

  /// Published minus committed, which is how many slots are spoken for.
  ///
  /// The one place the asymmetric orderings are spelled out: this end's own
  /// cursor at [`OWN`], the consumer's at [`GATING`].
  fn occupancy(&self) -> u64 {
    let produced = self.ring.cursors.producer().load(OWN);
    let consumed = self.ring.cursors.consumer().load(GATING);

    consumed.distance_to(produced)
  }

  /// Take the next slot, to be published when the returned value is dropped.
  ///
  /// This returns a guard instead of a bare `claim`/`publish` pair, because an
  /// early return between the two stalls the ring permanently and no runtime
  /// check can distinguish "claimed and about to publish" from "claimed and
  /// abandoned". Making the publish the drop makes the case unreachable,
  /// including on unwind.
  ///
  /// The capacity decision runs on this end's cached cursors, and the fast
  /// path performs no atomic load: the cached tail is this end's own
  /// bookkeeping, and a cached head that lags the consumer can only make the
  /// cache report less room than there is. When the cache reports full, one
  /// [`GATING`] load of the consumer cursor refreshes it — the single read of
  /// the consumer's line on this path — and the refreshed observation decides
  /// honestly. A refused claim moves nothing, cached or shared.
  ///
  /// # Errors
  ///
  /// [`RingError::Full`] when the consumer has not committed far enough for a
  /// slot to be free. Nothing waits; a full ring is reported.
  ///
  /// ```
  /// use ring_slot::{ BytesSlot, Slot };
  /// use ring_spsc::Ring;
  /// use ring_types::Capacity;
  ///
  /// let mut ring : Ring< BytesSlot< 8 > > = Ring::new( Capacity::new( 2 ).unwrap() );
  /// let ( mut producer, mut consumer ) = ring.split();
  ///
  /// // Written in place — the payload never passes through a temporary.
  /// producer.claim().unwrap().write( b"hi" ).unwrap();
  ///
  /// assert_eq!( consumer.drain().get( 0 ).map( BytesSlot::read ), Some( &b"hi"[ .. ] ) );
  /// ```
  pub fn claim(&mut self) -> Result<Reservation<'_, S>, RingError> {
    let capacity = self.ring.capacity().get() as u64;

    if self.cached_head.distance_to(self.cached_tail) >= capacity {
      // The cache says full. The cache can only lag the consumer, so the ring
      // may still have room: refresh the observation once, at the one
      // `Acquire` load the fast path exists to avoid. This load is also the
      // edge that orders the consumer's take of the lapped record before this
      // end's overwrite of its slot.
      self.cached_head = self.ring.cursors.consumer().load(GATING);

      if self.cached_head.distance_to(self.cached_tail) >= capacity {
        return Err(RingError::Full);
      }
    }

    let seq = self.cached_tail;
    self.cached_tail = seq.next();

    Ok(Reservation { ring: self.ring, seq })
  }

  /// Claim a slot, write it through `write`, and publish.
  ///
  /// The fused shape for an arbitrary [`Slot`], where the payload's own setter
  /// is not nameable generically. `Producer< '_, TypedSlot< T > >` has a
  /// [`try_push`] taking the record by value instead.
  ///
  /// [`try_push`]: Producer::try_push
  ///
  /// # Errors
  ///
  /// [`RingError::Full`], and `write` is not called, so a caller can retry
  /// without having consumed anything.
  ///
  /// ```
  /// use ring_slot::{ BytesSlot, Slot };
  /// use ring_spsc::Ring;
  /// use ring_types::Capacity;
  ///
  /// let mut ring : Ring< BytesSlot< 8 > > = Ring::new( Capacity::new( 2 ).unwrap() );
  /// let ( mut producer, mut consumer ) = ring.split();
  ///
  /// producer.push_with( | slot | slot.write( b"abc" ) ).unwrap().unwrap();
  /// assert_eq!( consumer.drain().get( 0 ).map( BytesSlot::read ), Some( &b"abc"[ .. ] ) );
  /// ```
  pub fn push_with<R>(&mut self, write: impl FnOnce(&mut S) -> R) -> Result<R, RingError> {
    let mut reservation = self.claim()?;

    Ok(write(&mut reservation))
  }
}

impl<T: Send> Producer<'_, TypedSlot<T>> {
  /// Put one record into the ring.
  ///
  /// The fused shape, available where the slot type has a by-value setter. It
  /// is built *over* [`claim`], not under it. The guard is the lower-level
  /// operation, so this convenience cannot introduce an abandonment case the
  /// guard rules out.
  ///
  /// [`claim`]: Producer::claim
  ///
  /// # Errors
  ///
  /// [`RingError::Full`], with `record` returned to the caller. A push that
  /// fails must not swallow the record, or a caller applying backpressure has
  /// nothing left to retry with.
  ///
  /// ```
  /// use ring_slot::TypedSlot;
  /// use ring_spsc::Ring;
  /// use ring_types::Capacity;
  ///
  /// let mut ring : Ring< TypedSlot< &str > > = Ring::new( Capacity::new( 1 ).unwrap() );
  /// let ( mut producer, _consumer ) = ring.split();
  ///
  /// assert_eq!( producer.try_push( "first" ), Ok( () ) );
  /// assert_eq!( producer.try_push( "second" ), Err( "second" ), "full, and the record comes back" );
  /// ```
  pub fn try_push(&mut self, record: T) -> Result<(), T> {
    let Ok(mut reservation) = self.claim() else {
      return Err(record);
    };

    // The slot's previous occupant, if any, was committed by the consumer a lap
    // ago; dropping it here is what makes the ring's storage bounded.
    drop(reservation.set(record));

    Ok(())
  }
}

/// A claimed slot, published when dropped.
///
/// Derefs to the slot, so the caller writes the payload in place instead of
/// moving it through a temporary. The publish is the drop, which makes an
/// abandoned claim unrepresentable. An early return, a `?`, or an unwind all
/// publish instead of stalling the ring.
///
/// The one way to keep a claim unpublished is `mem::forget`. That is a caller
/// precondition violation either way, and the failure shape differs between
/// this crate's cached and uncached bookkeeping: the shared cursor stays
/// behind the producer's cached tail, so the next publish jumps over the
/// leaked sequence instead of re-granting it, and the consumer receives an
/// empty or stale record at that position rather than the ring stalling. No
/// slot ends up with two live owners in either case.
#[must_use = "a reservation publishes on drop; dropping it immediately publishes an unwritten slot"]
#[derive(Debug)]
pub struct Reservation<'a, S> {
  ring: &'a Ring<S>,
  seq: Seq,
}

impl<S> Reservation<'_, S> {
  /// The sequence this slot will be published at.
  ///
  /// ```
  /// use ring_slot::TypedSlot;
  /// use ring_spsc::Ring;
  /// use ring_types::{ Capacity, Seq };
  ///
  /// let mut ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 4 ).unwrap() );
  /// let ( mut producer, _consumer ) = ring.split();
  ///
  /// assert_eq!( producer.claim().unwrap().sequence(), Seq::ZERO );
  /// assert_eq!( producer.claim().unwrap().sequence(), Seq( 1 ), "the first one published on drop" );
  /// ```
  #[must_use]
  pub const fn sequence(&self) -> Seq {
    self.seq
  }
}

impl<S> Deref for Reservation<'_, S> {
  type Target = S;

  fn deref(&self) -> &S {
    // SAFETY: `slot_mut`'s precondition holds, as argued in `deref_mut`. A
    // shared reference is strictly weaker than the `&mut` that call is
    // entitled to, and it is derived from the same claimed-not-published
    // sequence.
    unsafe { self.ring.slot_mut(self.seq) }
  }
}

impl<S> DerefMut for Reservation<'_, S> {
  fn deref_mut(&mut self) -> &mut S {
    // SAFETY: `self.seq` is claimed and not published. It is the producer
    // cursor's current value, which `Drop` has not yet advanced, so it is at or
    // above the consumer's exclusive readable bound. `claim` checked it is also
    // at least one lap behind the consumer, so no unread record is being
    // overwritten. This is the only `Reservation`, because `claim` borrows the
    // `Producer` mutably for this value's whole life.
    unsafe { self.ring.slot_mut(self.seq) }
  }
}

impl<S> Drop for Reservation<'_, S> {
  /// Publish, with the one release store that is the producer path's entire
  /// synchronization.
  fn drop(&mut self) {
    self.ring.cursors.producer().store(self.seq.next(), HANDOFF);
  }
}

/// The reading end.
///
/// Not `Clone` and not `Sync`, for the same reason as [`Producer`]. A second
/// consumer would silently divide the records instead of duplicating them.
#[derive(Debug)]
pub struct Consumer<'a, S> {
  ring: &'a Ring<S>,
  /// See [`Producer`]'s field of the same name.
  _one_thread: PhantomData<Cell<()>>,
}

impl<S> Consumer<'_, S> {
  /// How far this end has committed.
  ///
  /// ```
  /// use ring_slot::TypedSlot;
  /// use ring_spsc::Ring;
  /// use ring_types::{ Capacity, Seq };
  ///
  /// let mut ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 4 ).unwrap() );
  /// let ( mut producer, mut consumer ) = ring.split();
  ///
  /// producer.try_push( 1 ).unwrap();
  /// assert_eq!( consumer.position(), Seq::ZERO, "nothing committed yet" );
  /// drop( consumer.drain() );
  /// assert_eq!( consumer.position(), Seq( 1 ) );
  /// ```
  #[must_use]
  pub fn position(&self) -> Seq {
    self.ring.cursors.consumer().load(OWN)
  }

  /// How many published records have not been drained.
  ///
  /// Costs exactly what [`drain`]'s bound computation costs: one `GATING`
  /// load and a subtraction. A caller polling this before deciding whether to
  /// drain has paid for the drain's expensive step twice; [`is_empty`] is the
  /// cheap poll for a tick loop.
  ///
  /// [`drain`]: Self::drain
  /// [`is_empty`]: Self::is_empty
  ///
  /// # Invariant: the reading is a lower bound until this end drains
  ///
  /// Only this end lowers the count, and only when a drained batch is dropped.
  /// The producer only adds to it. A reported `n` therefore stays true until
  /// this end's next drain, and the real count may already be higher. It
  /// mirrors [`Producer::free_capacity`], which is binding for the same reason.
  ///
  /// **Excluded.** An empty reading, from this or from [`is_empty`], promises
  /// nothing. The producer can publish the moment after.
  ///
  /// **Enforced by.** The structure alone. Each cursor has one writer that
  /// only moves it forward, and no test exercises the concurrent case.
  ///
  /// ```
  /// use ring_slot::TypedSlot;
  /// use ring_spsc::Ring;
  /// use ring_types::Capacity;
  ///
  /// let mut ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 4 ).unwrap() );
  /// let ( mut producer, consumer ) = ring.split();
  ///
  /// producer.try_push( 1 ).unwrap();
  /// producer.try_push( 2 ).unwrap();
  /// assert_eq!( consumer.available(), 2 );
  /// ```
  #[must_use]
  pub fn available(&self) -> usize {
    let consumed = self.ring.cursors.consumer().load(OWN);
    let produced = self.ring.cursors.producer().load(GATING);

    consumed.distance_to(produced) as usize
  }

  /// Whether there is nothing to drain.
  ///
  /// A compare rather than a subtract, which is the difference from
  /// [`available`] and the reason this is the operation a tick loop polls
  /// with.
  ///
  /// [`available`]: Self::available
  ///
  /// ```
  /// use ring_slot::TypedSlot;
  /// use ring_spsc::Ring;
  /// use ring_types::Capacity;
  ///
  /// let mut ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 4 ).unwrap() );
  /// let ( mut producer, consumer ) = ring.split();
  ///
  /// assert!( consumer.is_empty() );
  /// producer.try_push( 1 ).unwrap();
  /// assert!( !consumer.is_empty() );
  /// ```
  #[must_use]
  pub fn is_empty(&self) -> bool {
    self.ring.cursors.consumer().load(OWN) == self.ring.cursors.producer().load(GATING)
  }

  /// Take everything published since the last drain.
  ///
  /// Batch-shaped because the commit is one release store however many records
  /// it covers; an item-at-a-time API would pay per record for
  /// synchronization this pays once.
  ///
  /// The returned batch commits on drop, and that is what makes the borrow
  /// sound. The slots become reusable at the commit, so a borrow that could
  /// outlive it would point at memory the producer may already be overwriting.
  ///
  /// ```
  /// use ring_slot::TypedSlot;
  /// use ring_spsc::Ring;
  /// use ring_types::Capacity;
  ///
  /// let mut ring : Ring< TypedSlot< u32 > > = Ring::new( Capacity::new( 4 ).unwrap() );
  /// let ( mut producer, mut consumer ) = ring.split();
  ///
  /// producer.try_push( 10 ).unwrap();
  /// producer.try_push( 20 ).unwrap();
  ///
  /// let batch = consumer.drain();
  /// assert_eq!( batch.iter().filter_map( TypedSlot::get ).copied().collect::< Vec< _ > >(), [ 10, 20 ] );
  /// ```
  pub fn drain(&mut self) -> Batch<'_, S> {
    let start = self.ring.cursors.consumer().load(OWN);
    let produced = self.ring.cursors.producer().load(GATING);

    Batch {
      ring: self.ring,
      start,
      len: start.distance_to(produced) as usize,
    }
  }

  /// Take at most `max` published records.
  ///
  /// The bounded form, for a caller that must not spend an unbounded amount of
  /// time in one drain, such as one working to a frame budget. What is left
  /// over stays published, and the next drain returns it.
  ///
  /// ```
  /// use ring_slot::TypedSlot;
  /// use ring_spsc::Ring;
  /// use ring_types::Capacity;
  ///
  /// let mut ring : Ring< TypedSlot< u32 > > = Ring::new( Capacity::new( 8 ).unwrap() );
  /// let ( mut producer, mut consumer ) = ring.split();
  ///
  /// for value in 0..5 { producer.try_push( value ).unwrap(); }
  ///
  /// assert_eq!( consumer.drain_up_to( 2 ).len(), 2 );
  /// assert_eq!( consumer.drain_up_to( 2 ).len(), 2 );
  /// assert_eq!( consumer.drain_up_to( 2 ).len(), 1, "and the remainder" );
  /// ```
  pub fn drain_up_to(&mut self, max: usize) -> Batch<'_, S> {
    let start = self.ring.cursors.consumer().load(OWN);
    let produced = self.ring.cursors.producer().load(GATING);

    Batch {
      ring: self.ring,
      start,
      len: max.min(start.distance_to(produced) as usize),
    }
  }
}

/// A run of published records, committed when dropped.
///
/// Borrowed, not copied out. The records stay in the slots, and the commit
/// that frees those slots is this value's drop. That pairing is the only
/// shape that is both zero-copy and sound. A bare `&[ S ]` returned from a
/// drain that had already committed would point at memory the producer is free
/// to overwrite.
#[must_use = "a batch commits on drop; dropping it immediately discards the records it covers"]
#[derive(Debug)]
pub struct Batch<'a, S> {
  ring: &'a Ring<S>,
  start: Seq,
  len: usize,
}

impl<S> Batch<'_, S> {
  /// The sequence of the batch's first record.
  ///
  /// ```
  /// use ring_slot::TypedSlot;
  /// use ring_spsc::Ring;
  /// use ring_types::{ Capacity, Seq };
  ///
  /// let mut ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 4 ).unwrap() );
  /// let ( mut producer, mut consumer ) = ring.split();
  ///
  /// producer.try_push( 1 ).unwrap();
  /// drop( consumer.drain() );
  /// producer.try_push( 2 ).unwrap();
  /// assert_eq!( consumer.drain().start(), Seq( 1 ) );
  /// ```
  #[must_use]
  pub const fn start(&self) -> Seq {
    self.start
  }

  /// How many records the batch covers.
  #[must_use]
  pub const fn len(&self) -> usize {
    self.len
  }

  /// Whether the batch covers nothing, meaning the drain found an empty ring.
  ///
  /// ```
  /// use ring_slot::TypedSlot;
  /// use ring_spsc::Ring;
  /// use ring_types::Capacity;
  ///
  /// let mut ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 4 ).unwrap() );
  /// let ( _producer, mut consumer ) = ring.split();
  ///
  /// assert!( consumer.drain().is_empty() );
  /// ```
  #[must_use]
  pub const fn is_empty(&self) -> bool {
    self.len == 0
  }

  /// The record `offset` places into the batch, or `None` past its end.
  ///
  /// ```
  /// use ring_slot::TypedSlot;
  /// use ring_spsc::Ring;
  /// use ring_types::Capacity;
  ///
  /// let mut ring : Ring< TypedSlot< u32 > > = Ring::new( Capacity::new( 4 ).unwrap() );
  /// let ( mut producer, mut consumer ) = ring.split();
  ///
  /// producer.try_push( 9 ).unwrap();
  /// let batch = consumer.drain();
  ///
  /// assert_eq!( batch.get( 0 ).and_then( TypedSlot::get ), Some( &9 ) );
  /// assert!( batch.get( 1 ).is_none() );
  /// ```
  #[must_use]
  pub fn get(&self, offset: usize) -> Option<&S> {
    if offset >= self.len {
      return None;
    }

    // SAFETY: `offset < self.len`, and the batch's whole range was published
    // and not committed when `drain` computed it. Its upper bound came from a
    // `GATING` load of the producer cursor, and its lower bound is this end's
    // own cursor, which only this value's `Drop` will advance.
    Some(unsafe { self.ring.slot(self.start.advanced_by(offset as u64)) })
  }

  /// The record `offset` places into the batch, mutably, or `None` past its end.
  ///
  /// **This is the only way to move a record out of a batch**, because taking a
  /// value from a slot needs `&mut`, and [`TypedSlot::take`] cannot be reached
  /// through [`get`]. A consumer that reads without consuming uses [`get`]; one
  /// that owns what it drained needs this.
  ///
  /// It was added for `ring_core`, whose uniform API hands the caller a `T`
  /// rather than a `&T` and so cannot be built on [`get`] alone.
  /// `tests/manual/readme.md` S10 records that the gap appeared only at the
  /// composition point, and only for this crate. `ring_mpsc`'s batch has
  /// always had its counterpart.
  ///
  /// [`get`]: Self::get
  ///
  /// Fix(ring_core_citation_pointed_at_docsrs_not_a_real_page): this line used
  /// to read `` `ring_core`: <https://docs.rs/ring_core> ``, a CommonMark
  /// autolink to an external page. docs.rs hosts a crate only once a version of
  /// it is published to crates.io, and rustdoc never checks an external URL, so
  /// nothing in the build said whether the page existed. Root cause: it was left
  /// over from a draft reference-style link that never got the rest of its
  /// syntax (`[ring_core]: ...` with a matching `[ring_core]` consumer
  /// elsewhere in the comment). Nothing in this comment ever consumed it, and
  /// the paragraph two lines up already cites `ring_core` correctly as plain
  /// backtick text, so the line was dead weight rendered as a live link.
  /// Pitfall: a doc-comment line shaped like a citation reads as
  /// already-verified and gets checked far less than prose does. The family
  /// cites a target as plain backtick text instead of a link.
  ///
  /// ```
  /// use ring_slot::TypedSlot;
  /// use ring_spsc::Ring;
  /// use ring_types::Capacity;
  ///
  /// let mut ring : Ring< TypedSlot< u32 > > = Ring::new( Capacity::new( 4 ).unwrap() );
  /// let ( mut producer, mut consumer ) = ring.split();
  ///
  /// producer.try_push( 9 ).unwrap();
  /// let mut batch = consumer.drain();
  ///
  /// assert_eq!( batch.get_mut( 0 ).and_then( TypedSlot::take ), Some( 9 ) );
  /// assert_eq!( batch.get_mut( 0 ).and_then( TypedSlot::take ), None, "taken once" );
  /// assert!( batch.get_mut( 1 ).is_none() );
  /// ```
  #[must_use]
  pub fn get_mut(&mut self, offset: usize) -> Option<&mut S> {
    if offset >= self.len {
      return None;
    }

    // SAFETY: as `get`, plus exclusivity. `&mut self` on the batch is the
    // consumer's own exclusive borrow, and the producer cannot reach this range
    // until this batch's `Drop` advances the consumer cursor past it.
    Some(unsafe { self.ring.slot_mut(self.start.advanced_by(offset as u64)) })
  }

  /// The batch's records, in the order they were published.
  ///
  /// ```
  /// use ring_slot::TypedSlot;
  /// use ring_spsc::Ring;
  /// use ring_types::Capacity;
  ///
  /// let mut ring : Ring< TypedSlot< u32 > > = Ring::new( Capacity::new( 4 ).unwrap() );
  /// let ( mut producer, mut consumer ) = ring.split();
  ///
  /// producer.try_push( 1 ).unwrap();
  /// producer.try_push( 2 ).unwrap();
  ///
  /// let drained : Vec< u32 > = consumer.drain().iter().filter_map( TypedSlot::get ).copied().collect();
  /// assert_eq!( drained, [ 1, 2 ] );
  /// ```
  pub fn iter(&self) -> impl Iterator<Item = &S> {
    (0..self.len).filter_map(|offset| self.get(offset))
  }
}

impl<S> Drop for Batch<'_, S> {
  /// Commit, with the one release store that is the consumer path's entire
  /// synchronization. It is one store for the whole batch, which is why the
  /// API is batch-shaped.
  ///
  /// The producer sees the whole batch freed at once. A full ring drained in
  /// one batch goes from full to empty in a single step, and anything watching
  /// [`Producer::free_capacity`] never sees the space come back gradually.
  fn drop(&mut self) {
    self
      .ring
      .cursors
      .consumer()
      .store(self.start.advanced_by(self.len as u64), HANDOFF);
  }
}
