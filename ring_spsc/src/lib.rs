//! Single-producer single-consumer ring API.
//!
//! One of the ring family's 33 crates — the concurrency write-path implementation.
//! Depends on `ring_store`, `ring_config`, `ring_cursor`, `ring_slot`,
//! `ring_types`.
//!
//! # What single-producer buys, stated as a dependency list
//!
//! `ring_gating`, `ring_claim`, `ring_publish` and `ring_consume` are all
//! absent, and that absence *is* the crate's thesis rather than an omission.
//! Each of those exists to answer a question that only has an answer worth
//! computing when producers can overtake one another:
//!
//! - `ring_gating` takes the minimum over a set of consumer cursors. At one
//!   consumer the minimum over one value is that value.
//! - `ring_claim` moves the producer cursor by compare-exchange, because two
//!   producers may race for the same range. At one producer nothing races, so
//!   the cursor is moved by a plain store.
//! - `ring_publish` spins until the frontier reaches this range's start,
//!   because producer B may finish before producer A. At one producer, the
//!   frontier is always already there.
//! - `ring_consume` gates a commit on a `Barrier` over the cursors a batch
//!   depended on. At one consumer that barrier has one member, and it is this
//!   end's own cursor.
//!
//! What is left is two cursors and an array
//! (→ `docs/data_structure/001_two_cursor_ring.md`). No per-slot stamp, no
//! read-modify-write on any path, no loop that can spin on another thread's
//! progress.
//!
//! # Orderings, and why they are not uniform
//!
//! Each cursor has exactly one writer. That is what lets an end read **its
//! own** cursor at [`OWN`] (`Relaxed`) — this thread performed the store it is
//! reading back, and program order already sequences the load after it — while
//! reading the **other** end's cursor at [`ring_cursor::GATING`] (`Acquire`),
//! which is what makes the peer's slot writes visible.
//!
//! [`ring_cursor::CursorPair`] offers `free_slots`, `pending` and `may_claim`
//! over the same two cursors, and this crate deliberately does not call them in
//! the hot path: they read *both* cursors at `Acquire`, correctly, because a
//! `CursorPair` does not know which end is asking. Here every caller does know,
//! so the asymmetric form is available and the symmetric one would pay for a
//! fence the single-writer property already provides.
//!
//! # The unsafe, and where its argument lives
//!
//! The producer writes slots while the consumer reads slots, through shared
//! references to one allocation. That is interior mutability across a thread
//! boundary, and `unsafe` is the only way to express it —
//! [decision 123](../../../docs/decision/123_ring_shared_slot_storage_unsafe_sited.md)
//! rules that it belongs here rather than in `ring_store` or `ring_slot`,
//! because the invariant making it sound is stated entirely in terms of
//! cursors that those crates do not hold. The argument is in
//! `Ring::slot`'s and `Ring::slot_mut`'s safety sections, and the shape it
//! rests on is asserted rather than only described — partly by
//! `tests/spsc_test.rs`, and partly by the four cases below.
//!
//! # What the type system refuses
//!
//! `slot_mut`'s safety section says "the caller must be the producer end", and
//! that is worth nothing if a second producer is constructible. The properties
//! that make one unrepresentable are all *negative* — an absent `Clone`, an
//! absent `Sync`, a borrow that outlives nothing — so none of them can be
//! asserted at runtime. They are checked here instead, as `compile_fail` doc
//! tests, which is the only executable form a negative has.
//!
//! These live in the library rather than in `tests/spsc_test.rs` because
//! rustdoc collects doc tests from the library target only: the same five
//! blocks written in an integration test file are never compiled, and would be
//! five checks that silently check nothing. That is not a hypothetical — they
//! were written there first, and the doc test count did not move.
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
//! Nor by splitting twice while the first pair is still live — the pair borrows
//! the ring for as long as either end is used:
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
//! Nor by sharing one end with a second thread — `!Sync` is what stops it,
//! stated as the bound itself rather than as a `thread::spawn` that would
//! demonstrate it:
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
//! into — the property that makes the borrowed batch sound rather than merely
//! convenient:
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
//! let mut ring: Ring<TypedSlot<u32>> = Ring::new(Capacity::new(8).unwrap());
//! let (mut producer, mut consumer) = ring.split();
//!
//! producer.try_push(1).unwrap();
//! producer.try_push(2).unwrap();
//!
//! let batch = consumer.drain();
//! assert_eq!(batch.len(), 2);
//! assert_eq!(batch.get(0).and_then(TypedSlot::get), Some(&1));
//! ```
//!
//! Acceptance is binary and lives in a test, not here: feature 171 is Reached
//! when `tests/spsc_test.rs` exchanges 100 000 items with byte-parity, in
//! order, zero loss, and no lock in the path — and cites `docs/feature/171_`
//! textually, which is the only crate→feature edge the family records.
//!
//! Reasoning honed on this crate is conservative in `ring_mpsc`, not the
//! reverse: a habit formed here — treating the producer cursor as a frontier,
//! or a claim as impossible to overtake — is unsafe carried into `ring_mpsc`,
//! where more than one producer can be racing the same claim. The type
//! system stops the opposite mistake (a `ring_mpsc` shape will not compile
//! here); it stops nothing in this direction.

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
/// `Relaxed` is sound because each cursor has exactly one writer: the thread
/// performing this load is the thread that performed the store it is reading
/// back, and program order already sequences the two. No inter-thread edge is
/// being established, so none needs to be paid for.
///
/// Contrast [`ring_cursor::GATING`], which is what the *other* end's cursor is
/// read with — there the load is exactly what makes the peer's slot writes
/// visible, and `Acquire` is load-bearing.
///
/// ```
/// use core::sync::atomic::Ordering;
///
/// assert_eq!(ring_spsc::OWN, Ordering::Relaxed);
/// assert_eq!(ring_cursor::GATING, Ordering::Acquire);
/// ```
pub const OWN: Ordering = Ordering::Relaxed;

/// The ordering a publication and a commit are made visible with.
///
/// One release store ends every producer operation and every consumer batch.
/// It is the only synchronization either path performs, and it is what pairs
/// with the peer's [`ring_cursor::GATING`] load: everything this thread wrote
/// to slots before the store is visible to a peer that observes the new cursor
/// value.
///
/// ```
/// use core::sync::atomic::Ordering;
///
/// assert_eq!(ring_spsc::HANDOFF, Ordering::Release);
/// ```
pub const HANDOFF: Ordering = Ordering::Release;

/// A ring one thread writes and one thread reads.
///
/// Three fields' worth of state in two: the slot array, and a
/// [`CursorPair`] holding the producer and consumer cursors on separate cache
/// lines. Everything else the ring needs — occupancy, free capacity, the slot
/// a sequence maps to — is derived rather than stored
/// (→ `docs/data_structure/001_two_cursor_ring.md`).
///
/// The ring owns the storage; the two ends [`split`] hands out borrow it. It
/// is deliberately not itself an API for pushing or draining: doing so would
/// mean a `&Ring` from which a second producer could be made, and one producer
/// is the whole precondition.
///
/// [`split`]: Self::split
///
/// ```
/// use ring_slot::BytesSlot;
/// use ring_spsc::Ring;
/// use ring_types::Capacity;
///
/// let ring: Ring<BytesSlot<16>> = Ring::new(Capacity::new(4).unwrap());
/// assert_eq!(ring.capacity().get(), 4);
/// assert!(ring.on_distinct_lines(), "the two cursors do not share a line");
/// ```
pub struct Ring<S> {
    /// The slot array, behind an `UnsafeCell` because the producer writes into it
    /// through a shared reference while the consumer reads from it.
    ///
    /// The cell is on each slot, not around the whole `Buffer`.
    ///
    /// It wrapped the buffer until `Buffer::new` was bounded on `Default` rather
    /// than `Slot`, which was the only thing making `Buffer< UnsafeCell< S > >`
    /// unconstructible. The outer form was unsound: reaching a slot through
    /// `( *cell.get() ).at_mut( seq )` materialises `&mut Buffer< S >`, an
    /// exclusive claim over the *entire* allocation, so a producer writing one
    /// slot and a consumer reading a different one aliased the whole buffer.
    /// Miri's data-race detector reports it as a retag conflict on `Buffer< S >`
    /// itself rather than on any slot — the two threads never touched the same
    /// record. A per-slot cell claims exactly the slot being touched, which is
    /// what the cursor invariant below actually guarantees to be disjoint.
    ///
    /// `ring_store` still knows nothing of this crate: it stores whatever
    /// element type it is given, and every `unsafe` stays here, which is
    /// decision 123's whole point.
    slots: Buffer<UnsafeCell<S>>,
    cursors: CursorPair,
}

// SAFETY: `Ring` is only ever shared between exactly two threads, and only as
// the `&Ring` held by a `Producer` and a `Consumer` — `split` takes `&mut
// self`, so no second pair can exist while a first is alive, and neither end is
// `Clone` or `Sync`. Those two threads touch disjoint slots at every instant:
// the producer only writes the slot for the sequence it has claimed and not yet
// published, the consumer only reads slots that are published and not yet
// committed, and a published-not-committed sequence is by construction below
// the producer's current position. The cursors are atomics and carry the
// happens-before edge (`HANDOFF` store, `GATING` load) that makes each side's
// slot writes visible to the other before the other may touch that slot.
// `S : Send` is required because a record is written on the producer's thread
// and read on the consumer's.
//
// `ring_mpsc` declares this identical line under a different justification —
// many producers on disjoint slots, not exactly two threads — so the hazard
// is reading one crate's SAFETY comment while editing the other's
// (→ `docs/workaround/002_an_unsafe_impl_sync_on_a_type_whose_ends_are_not_sync.md`
// SP52).
unsafe impl<S: Send> Sync for Ring<S> {}

impl<S: Slot + Default> Ring<S> {
    /// Allocate a ring of `capacity` slots, with both cursors at zero.
    ///
    /// ```
    /// use ring_slot::TypedSlot;
    /// use ring_spsc::Ring;
    /// use ring_types::Capacity;
    ///
    /// let ring: Ring<TypedSlot<u8>> = Ring::new(Capacity::new(16).unwrap());
    /// assert_eq!(ring.capacity().get(), 16);
    /// ```
    #[must_use]
    pub fn new(capacity: Capacity) -> Self {
        Self { slots: Buffer::new(capacity), cursors: CursorPair::new(capacity) }
    }

    /// Allocate a ring sized by a [`RingConfig`].
    ///
    /// Only the capacity is read here. A config's wait strategy and overflow
    /// policy describe what a *caller* does when the ring is full or empty, and
    /// this crate never waits and never drops — it reports
    /// (→ `docs/api/001_producer_surface.md`).
    ///
    /// ```
    /// use ring_config::RingConfig;
    /// use ring_slot::TypedSlot;
    /// use ring_spsc::Ring;
    ///
    /// let config = RingConfig::new(32).unwrap();
    /// let ring: Ring<TypedSlot<u8>> = Ring::with_config(&config);
    /// assert_eq!(ring.capacity(), config.capacity());
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
    /// let ring: Ring<TypedSlot<u8>> = Ring::new(Capacity::new(64).unwrap());
    /// assert_eq!(ring.capacity().get(), 64);
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
    /// let ring: Ring<TypedSlot<u8>> = Ring::new(Capacity::new(2).unwrap());
    /// assert!(ring.on_distinct_lines());
    /// ```
    #[must_use]
    pub fn on_distinct_lines(&self) -> bool {
        self.cursors.on_distinct_lines()
    }

    /// The two ends, to be moved onto two threads.
    ///
    /// **`&mut self` is what enforces the cardinality.** The returned ends borrow
    /// the ring for as long as they live, so a second call cannot happen while a
    /// first pair exists — there is no moment at which two `Producer` values name
    /// one ring. Neither end is `Clone` and neither is `Sync`, so neither can be
    /// shared with a second thread after the split either
    /// (→ `docs/lifecycle/002_producer_consumer_pairing.md`).
    ///
    /// ```
    /// use ring_slot::TypedSlot;
    /// use ring_spsc::Ring;
    /// use ring_types::Capacity;
    ///
    /// let mut ring: Ring<TypedSlot<u32>> = Ring::new(Capacity::new(4).unwrap());
    /// let (mut producer, mut consumer) = ring.split();
    ///
    /// std::thread::scope(|scope| {
    ///     scope.spawn(move || producer.try_push(7).unwrap());
    ///     scope.spawn(move || {
    ///         while consumer.is_empty() {
    ///             std::thread::yield_now();
    ///         }
    ///         assert_eq!(consumer.drain().len(), 1);
    ///     });
    /// });
    /// ```
    pub fn split(&mut self) -> (Producer<'_, S>, Consumer<'_, S>) {
        let shared: &Self = self;

        (
            Producer { ring: shared, _one_thread: PhantomData },
            Consumer { ring: shared, _one_thread: PhantomData },
        )
    }

    /// The slot `seq` maps to, for reading.
    ///
    /// # Safety
    ///
    /// The caller must be the consumer end, and `seq` must be **published and not
    /// yet committed** — at or after this end's own cursor and strictly before
    /// the producer's, as read at [`GATING`]. Those two bounds are what place the
    /// slot outside the producer's writable set: the producer writes only at its
    /// own cursor position, which is at or above the exclusive upper bound here.
    ///
    /// The `GATING` load that established the upper bound is also what makes the
    /// producer's write to this slot visible; reading a slot on the strength of a
    /// bound obtained any other way is a data race even if the arithmetic holds.
    unsafe fn slot(&self, seq: Seq) -> &S {
        // SAFETY: the caller guarantees `seq` is published and not committed, so no
        // `&mut` to this slot exists — the producer's only outstanding `&mut` is to
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
    /// whole life — no other `&S`/`&mut S` to the same slot may be live at the
    /// same time. Two call sites establish that, on their own terms, and a
    /// finding recorded while auditing this function is that a reader who stops
    /// at the first is left thinking the second one violates this doc:
    ///
    /// - **The producer end, on a claimed-not-yet-published `seq`** — equal to
    ///   this end's own cursor, which the producer has not yet advanced. That
    ///   places the slot outside the consumer's readable set, whose exclusive
    ///   upper bound is that same cursor value. It must additionally be free: at
    ///   least one lap behind the consumer's cursor as read at [`GATING`], or the
    ///   slot still holds a record the consumer has not read. `Producer::claim`
    ///   is the only such caller and checks exactly that before constructing the
    ///   [`Reservation`] this is reached through.
    /// - **The consumer end, on a published-not-yet-committed `seq`** — reached
    ///   through [`Batch::get_mut`], between `drain`/`drain_up_to` and the
    ///   batch's `Drop`. Publication is the producer cursor advancing past
    ///   `seq`, which is also the producer releasing its own claim on it, and the
    ///   producer cannot lap back onto `seq` until this batch's `Drop` advances
    ///   the consumer cursor past it in turn.
    ///
    /// Returning `&mut S` from `&self` is the one place the two-thread design is
    /// not expressible in safe Rust, and it is why decision 123 sites the opt-out
    /// in this crate.
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
    /// Cursor positions and capacity — never slot contents.
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
/// fact about the program rather than a documented precondition
/// (→ `docs/invariant/001_exactly_one_producer_one_consumer.md`). `Send`, so it
/// can be moved onto a thread.
///
/// This declaration's exact text is pinned outside this crate: a `ring_handle`
/// trybuild fixture (`tests/ui/producer_shared_across_threads.stderr`) expects
/// this line verbatim in a compiler error, with no dependency edge recording
/// why (→ `docs/decisions/001_the_switching_cost_argument_undercounts_its_own_blast_radius.md`
/// SP13).
#[derive(Debug)]
pub struct Producer<'a, S> {
    ring: &'a Ring<S>,
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
    /// use ring_types::{Capacity, Seq};
    ///
    /// let mut ring: Ring<TypedSlot<u8>> = Ring::new(Capacity::new(4).unwrap());
    /// let (mut producer, _consumer) = ring.split();
    ///
    /// assert_eq!(producer.position(), Seq::ZERO);
    /// producer.try_push(1).unwrap();
    /// assert_eq!(producer.position(), Seq(1));
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
    /// (→ `docs/pitfall/001_spsc_correctness_does_not_transfer.md`).
    ///
    /// ```
    /// use ring_slot::TypedSlot;
    /// use ring_spsc::Ring;
    /// use ring_types::Capacity;
    ///
    /// let mut ring: Ring<TypedSlot<u8>> = Ring::new(Capacity::new(4).unwrap());
    /// let (mut producer, _consumer) = ring.split();
    ///
    /// assert_eq!(producer.free_capacity(), 4);
    /// producer.try_push(1).unwrap();
    /// assert_eq!(producer.free_capacity(), 3);
    /// ```
    ///
    /// Fix(free_capacity_underflow_on_a_precondition_violation): this
    /// subtraction used to be `self.ring.capacity().get() - self.occupancy() as
    /// usize`, unguarded — `ring_debug/docs/invariant/002` DB35 and
    /// `ring_debug/docs/pattern/001`'s family-wide census both name it as one of
    /// eight unguarded subtractions left in `ring/ring_*/src`. `occupancy()`
    /// cannot itself underflow (`Seq::distance_to` is `saturating_sub`), but if
    /// occupancy ever exceeds capacity — the D2 state `docs/invariant/001`
    /// describes as an unenforceable-at-runtime consequence of a caller
    /// violating the single-producer precondition, not something this crate can
    /// check — this line panicked in a dev build and wrapped to a
    /// near-`usize::MAX` value in release (the workspace sets no
    /// `overflow-checks` anywhere). Root cause: the guard that would make the
    /// subtraction sound (`occupancy() <= capacity`) is structural, enforced by
    /// `ring_handle`'s non-`Clone` handles above this crate rather than checked
    /// at this call site, so nothing here stood between the precondition's
    /// absence and the arithmetic. Pitfall: a subtraction guarded only by a
    /// precondition the crate cannot itself verify is indistinguishable, at the
    /// call site, from one that is always safe — `saturating_sub` makes the D2
    /// case return `0` (still an accurate answer: zero slots are safely
    /// claimable) instead of panicking or lying, at zero behavioural cost for
    /// every contract-respecting caller, matching the convention
    /// `ring_seqno::free_slots` already uses for the same computation.
    #[must_use]
    pub fn free_capacity(&self) -> usize {
        self.ring.capacity().get().saturating_sub(self.occupancy() as usize)
    }

    /// Whether the next claim would fail.
    ///
    /// ```
    /// use ring_slot::TypedSlot;
    /// use ring_spsc::Ring;
    /// use ring_types::Capacity;
    ///
    /// let mut ring: Ring<TypedSlot<u8>> = Ring::new(Capacity::new(2).unwrap());
    /// let (mut producer, _consumer) = ring.split();
    ///
    /// assert!(!producer.is_full());
    /// producer.try_push(1).unwrap();
    /// producer.try_push(2).unwrap();
    /// assert!(producer.is_full());
    /// ```
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.occupancy() as usize == self.ring.capacity().get()
    }

    /// Published minus committed — how many slots are currently spoken for.
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
    /// The guard shape rather than a bare `claim`/`publish` pair, because an
    /// early return between the two wedges the ring permanently and no runtime
    /// check can distinguish "claimed and about to publish" from "claimed and
    /// abandoned" (→ `docs/lifecycle/002_producer_consumer_pairing.md`). Making
    /// the publish the drop makes the case unreachable, including on unwind.
    ///
    /// # Errors
    ///
    /// [`RingError::Full`] when the consumer has not committed far enough for a
    /// slot to be free. Nothing waits; a full ring is reported.
    ///
    /// ```
    /// use ring_slot::{BytesSlot, Slot};
    /// use ring_spsc::Ring;
    /// use ring_types::Capacity;
    ///
    /// let mut ring: Ring<BytesSlot<8>> = Ring::new(Capacity::new(2).unwrap());
    /// let (mut producer, mut consumer) = ring.split();
    ///
    /// // Written in place — the payload never passes through a temporary.
    /// producer.claim().unwrap().write(b"hi").unwrap();
    ///
    /// assert_eq!(consumer.drain().get(0).map(BytesSlot::read), Some(&b"hi"[..]));
    /// ```
    pub fn claim(&mut self) -> Result<Reservation<'_, S>, RingError> {
        if self.is_full() {
            return Err(RingError::Full);
        }

        let seq = self.ring.cursors.producer().load(OWN);

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
    /// [`RingError::Full`] — and `write` is not called, so a caller can retry
    /// without having consumed anything.
    ///
    /// ```
    /// use ring_slot::{BytesSlot, Slot};
    /// use ring_spsc::Ring;
    /// use ring_types::Capacity;
    ///
    /// let mut ring: Ring<BytesSlot<8>> = Ring::new(Capacity::new(2).unwrap());
    /// let (mut producer, mut consumer) = ring.split();
    ///
    /// producer.push_with(|slot| slot.write(b"abc")).unwrap().unwrap();
    /// assert_eq!(consumer.drain().get(0).map(BytesSlot::read), Some(&b"abc"[..]));
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
    /// is built *over* [`claim`] rather than under it: the guard is the
    /// primitive, so this convenience cannot introduce an abandonment case the
    /// primitive rules out.
    ///
    /// [`claim`]: Producer::claim
    ///
    /// # Errors
    ///
    /// [`RingError::Full`], with `record` returned to the caller — a push that
    /// fails must not swallow the record, or a caller applying backpressure has
    /// nothing left to retry with.
    ///
    /// ```
    /// use ring_slot::TypedSlot;
    /// use ring_spsc::Ring;
    /// use ring_types::Capacity;
    ///
    /// let mut ring: Ring<TypedSlot<&str>> = Ring::new(Capacity::new(1).unwrap());
    /// let (mut producer, _consumer) = ring.split();
    ///
    /// assert_eq!(producer.try_push("first"), Ok(()));
    /// assert_eq!(producer.try_push("second"), Err("second"), "full, and the record comes back");
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
/// Derefs to the slot, so the payload is written in place rather than moved
/// through a temporary. The publish is the drop, which is what makes an
/// abandoned claim unrepresentable — an early return, a `?`, or an unwind all
/// publish rather than wedge the ring.
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
    /// use ring_types::{Capacity, Seq};
    ///
    /// let mut ring: Ring<TypedSlot<u8>> = Ring::new(Capacity::new(4).unwrap());
    /// let (mut producer, _consumer) = ring.split();
    ///
    /// assert_eq!(producer.claim().unwrap().sequence(), Seq::ZERO);
    /// assert_eq!(producer.claim().unwrap().sequence(), Seq(1), "the first one published on drop");
    /// ```
    #[must_use]
    pub const fn sequence(&self) -> Seq {
        self.seq
    }
}

impl<S> Deref for Reservation<'_, S> {
    type Target = S;

    fn deref(&self) -> &S {
        // SAFETY: `slot_mut`'s precondition holds — see `deref_mut`. A shared
        // reference is strictly weaker than the `&mut` that call is entitled to,
        // and it is derived from the same claimed-not-published sequence.
        unsafe { self.ring.slot_mut(self.seq) }
    }
}

impl<S> DerefMut for Reservation<'_, S> {
    fn deref_mut(&mut self) -> &mut S {
        // SAFETY: `self.seq` is claimed and not published — it is the producer
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
/// Not `Clone` and not `Sync`, for the same reason as [`Producer`] — a second
/// consumer does not duplicate records, it silently divides them.
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
    /// use ring_types::{Capacity, Seq};
    ///
    /// let mut ring: Ring<TypedSlot<u8>> = Ring::new(Capacity::new(4).unwrap());
    /// let (mut producer, mut consumer) = ring.split();
    ///
    /// producer.try_push(1).unwrap();
    /// assert_eq!(consumer.position(), Seq::ZERO, "nothing committed yet");
    /// drop(consumer.drain());
    /// assert_eq!(consumer.position(), Seq(1));
    /// ```
    #[must_use]
    pub fn position(&self) -> Seq {
        self.ring.cursors.consumer().load(OWN)
    }

    /// How many published records have not been drained.
    ///
    /// Costs exactly what [`drain`]'s bound computation costs — one `GATING`
    /// load and a subtraction. A caller polling this before deciding whether to
    /// drain has paid for the drain's expensive step twice; [`is_empty`] is the
    /// cheap poll for a tick loop.
    ///
    /// [`drain`]: Self::drain
    /// [`is_empty`]: Self::is_empty
    ///
    /// ```
    /// use ring_slot::TypedSlot;
    /// use ring_spsc::Ring;
    /// use ring_types::Capacity;
    ///
    /// let mut ring: Ring<TypedSlot<u8>> = Ring::new(Capacity::new(4).unwrap());
    /// let (mut producer, consumer) = ring.split();
    ///
    /// producer.try_push(1).unwrap();
    /// producer.try_push(2).unwrap();
    /// assert_eq!(consumer.available(), 2);
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
    /// let mut ring: Ring<TypedSlot<u8>> = Ring::new(Capacity::new(4).unwrap());
    /// let (mut producer, consumer) = ring.split();
    ///
    /// assert!(consumer.is_empty());
    /// producer.try_push(1).unwrap();
    /// assert!(!consumer.is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ring.cursors.consumer().load(OWN) == self.ring.cursors.producer().load(GATING)
    }

    /// Take everything published since the last drain.
    ///
    /// Batch-shaped because the commit is one release store however many records
    /// it covers; an item-at-a-time primitive would pay per record for
    /// synchronization this pays once
    /// (→ `docs/algorithm/002_single_consumer_drain.md`).
    ///
    /// The returned batch commits on drop, and that is what makes the borrow
    /// sound: the slots become reusable at the commit, so a borrow that could
    /// outlive it would point at memory the producer may already be overwriting.
    ///
    /// ```
    /// use ring_slot::TypedSlot;
    /// use ring_spsc::Ring;
    /// use ring_types::Capacity;
    ///
    /// let mut ring: Ring<TypedSlot<u32>> = Ring::new(Capacity::new(4).unwrap());
    /// let (mut producer, mut consumer) = ring.split();
    ///
    /// producer.try_push(10).unwrap();
    /// producer.try_push(20).unwrap();
    ///
    /// let batch = consumer.drain();
    /// assert_eq!(batch.iter().filter_map(TypedSlot::get).copied().collect::<Vec<_>>(), [10, 20]);
    /// ```
    pub fn drain(&mut self) -> Batch<'_, S> {
        let start = self.ring.cursors.consumer().load(OWN);
        let produced = self.ring.cursors.producer().load(GATING);

        Batch { ring: self.ring, start, len: start.distance_to(produced) as usize }
    }

    /// Take at most `max` published records.
    ///
    /// The bounded form, for a caller that must not spend an unbounded amount of
    /// time in one drain — a frame budget, say. What is left over stays
    /// published and is returned by the next drain.
    ///
    /// ```
    /// use ring_slot::TypedSlot;
    /// use ring_spsc::Ring;
    /// use ring_types::Capacity;
    ///
    /// let mut ring: Ring<TypedSlot<u32>> = Ring::new(Capacity::new(8).unwrap());
    /// let (mut producer, mut consumer) = ring.split();
    ///
    /// for value in 0..5 {
    ///     producer.try_push(value).unwrap();
    /// }
    ///
    /// assert_eq!(consumer.drain_up_to(2).len(), 2);
    /// assert_eq!(consumer.drain_up_to(2).len(), 2);
    /// assert_eq!(consumer.drain_up_to(2).len(), 1, "and the remainder");
    /// ```
    pub fn drain_up_to(&mut self, max: usize) -> Batch<'_, S> {
        let start = self.ring.cursors.consumer().load(OWN);
        let produced = self.ring.cursors.producer().load(GATING);

        Batch { ring: self.ring, start, len: max.min(start.distance_to(produced) as usize) }
    }
}

/// A run of published records, committed when dropped.
///
/// Borrowed rather than copied out: the records stay in the slots, and the
/// commit that frees those slots is this value's drop. That pairing is the only
/// shape that is both zero-copy and sound
/// (→ `docs/api/002_consumer_surface.md`) — a bare `&[ S ]` returned from a
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
    /// use ring_types::{Capacity, Seq};
    ///
    /// let mut ring: Ring<TypedSlot<u8>> = Ring::new(Capacity::new(4).unwrap());
    /// let (mut producer, mut consumer) = ring.split();
    ///
    /// producer.try_push(1).unwrap();
    /// drop(consumer.drain());
    /// producer.try_push(2).unwrap();
    /// assert_eq!(consumer.drain().start(), Seq(1));
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

    /// Whether the batch covers nothing — the drain found an empty ring.
    ///
    /// ```
    /// use ring_slot::TypedSlot;
    /// use ring_spsc::Ring;
    /// use ring_types::Capacity;
    ///
    /// let mut ring: Ring<TypedSlot<u8>> = Ring::new(Capacity::new(4).unwrap());
    /// let (_producer, mut consumer) = ring.split();
    ///
    /// assert!(consumer.drain().is_empty());
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
    /// let mut ring: Ring<TypedSlot<u32>> = Ring::new(Capacity::new(4).unwrap());
    /// let (mut producer, mut consumer) = ring.split();
    ///
    /// producer.try_push(9).unwrap();
    /// let batch = consumer.drain();
    ///
    /// assert_eq!(batch.get(0).and_then(TypedSlot::get), Some(&9));
    /// assert!(batch.get(1).is_none());
    /// ```
    #[must_use]
    pub fn get(&self, offset: usize) -> Option<&S> {
        if offset >= self.len {
            return None;
        }

        // SAFETY: `offset < self.len`, and the batch's whole range was published
        // and not committed when `drain` computed it — its upper bound came from a
        // `GATING` load of the producer cursor, and its lower bound is this end's
        // own cursor, which only this value's `Drop` will advance.
        Some(unsafe { self.ring.slot(self.start.advanced_by(offset as u64)) })
    }

    /// The record `offset` places into the batch, mutably, or `None` past its end.
    ///
    /// **This is the only way to move a record out of a batch**, because taking a
    /// value from a slot needs `&mut` — [`TypedSlot::take`] cannot be reached
    /// through [`get`]. A consumer that reads without consuming uses [`get`]; one
    /// that owns what it drained needs this.
    ///
    /// It was added for `ring_core`, whose uniform surface hands the caller a
    /// `T` rather than a `&T` and so cannot be built on [`get`] alone. That the
    /// gap surfaced only at the composition point, and only for this crate —
    /// `ring_mpsc`'s batch has always had its counterpart — is recorded in
    /// `tests/manual/readme.md` S10.
    ///
    /// [`get`]: Self::get
    ///
    /// Fix(ring_core_citation_pointed_at_docsrs_not_a_real_page): this line used
    /// to read `` `ring_core`: <https://docs.rs/ring_core> `` — a CommonMark
    /// autolink, dead by construction, since `publish = false` means this crate
    /// is never on docs.rs and the address would never resolve to `ring_core`'s
    /// documentation even if it existed. Root cause: it was left over from a
    /// draft reference-style link that never got the rest of its syntax
    /// (`[ring_core]: ...` with a matching `[ring_core]` consumer elsewhere in
    /// the comment); nothing in this comment ever consumed it, and the
    /// paragraph two lines up already cites `ring_core` correctly as plain
    /// backtick text, so the line was pure dead weight rendered as a live link.
    /// Pitfall: a doc-comment line shaped like a citation reads as
    /// already-verified and gets checked far less than prose does; `ring_core`'s
    /// own module documentation carries the identical pattern for an unrelated
    /// citation (fixed there by citing the target as plain backtick text
    /// instead of a link), which is the family's actual convention.
    ///
    /// ```
    /// use ring_slot::TypedSlot;
    /// use ring_spsc::Ring;
    /// use ring_types::Capacity;
    ///
    /// let mut ring: Ring<TypedSlot<u32>> = Ring::new(Capacity::new(4).unwrap());
    /// let (mut producer, mut consumer) = ring.split();
    ///
    /// producer.try_push(9).unwrap();
    /// let mut batch = consumer.drain();
    ///
    /// assert_eq!(batch.get_mut(0).and_then(TypedSlot::take), Some(9));
    /// assert_eq!(batch.get_mut(0).and_then(TypedSlot::take), None, "taken once");
    /// assert!(batch.get_mut(1).is_none());
    /// ```
    #[must_use]
    pub fn get_mut(&mut self, offset: usize) -> Option<&mut S> {
        if offset >= self.len {
            return None;
        }

        // SAFETY: as `get`, plus exclusivity — `&mut self` on the batch is the
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
    /// let mut ring: Ring<TypedSlot<u32>> = Ring::new(Capacity::new(4).unwrap());
    /// let (mut producer, mut consumer) = ring.split();
    ///
    /// producer.try_push(1).unwrap();
    /// producer.try_push(2).unwrap();
    ///
    /// let drained: Vec<u32> = consumer.drain().iter().filter_map(TypedSlot::get).copied().collect();
    /// assert_eq!(drained, [1, 2]);
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = &S> {
        (0..self.len).filter_map(|offset| self.get(offset))
    }
}

impl<S> Drop for Batch<'_, S> {
    /// Commit, with the one release store that is the consumer path's entire
    /// synchronization — one store for the whole batch, which is why the surface
    /// is batch-shaped.
    fn drop(&mut self) {
        self.ring.cursors.consumer().store(self.start.advanced_by(self.len as u64), HANDOFF);
    }
}
