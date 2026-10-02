//! Multi-producer single-consumer ring API.
//!
//! One of the ring family's 33 crates — the concurrency write-path implementation.
//! Depends on `ring_atomic`, `ring_store`, `ring_claim`, `ring_config`,
//! `ring_cursor`, `ring_gating`, `ring_slot`, `ring_types`.
//!
//! Many producer threads claim a slot and publish into it concurrently; one
//! consumer thread drains published slots in total order — the ring/merge half
//! of a mechanism more than one independent consumer needs, factored out so
//! it is built once.
//!
//! # Publication is a per-slot stamp, and that is this crate's whole addition
//!
//! `ring_publish` exists and is deliberately **not** used here. Its own module
//! documentation says why, about this crate by name: publishing to the highest
//! contiguous point "is what a high-contention multi-producer ring eventually
//! needs; it is deliberately not here, because it is `ring_mpsc`'s problem at
//! S5". Its `Publisher::publish` spins until the *predecessor* producer has
//! published, which makes one producer's progress depend on another's — the one
//! coupling the contended-claim feature exists to remove.
//!
//! So publication is a `Release` store into `stamps[ seq & mask ]`, and a slot
//! is published exactly when its stamp equals the sequence addressing it. No
//! producer waits for another to publish; the consumer pays a scan instead.
//! [decision 124](../../../docs/decision/124_ring_mpsc_publication_stamped_not_cursor.md)
//! records the ruling and the three cross-seam assumptions that were measured
//! against their siblings' sources and found unmet on the way to it.
//!
//! # Why the stamp needs no sentinel
//!
//! A stamp holds the sequence whose payload occupies that slot. Slot `i` is
//! addressed by sequences `i`, `i + capacity`, `i + 2·capacity`, … — every lap
//! gives it a different one. So the drain's test is equality against the
//! sequence it is *looking for*, and a stale stamp from the previous lap fails
//! it for the same reason a never-written one does. Stamps are initialised to
//! [`UNSTAMPED`] only so that lap zero has something that is not a valid
//! sequence to fail against.
//!
//! This is why the stamp is a full [`Seq`] and not a one-bit ready flag. A flag
//! would need clearing on reclamation — a second write to the same line, on the
//! consumer's hot path, to re-establish what the sequence already encodes.
//!
//! # The claim is lock-free, not wait-free
//!
//! [`ring_claim::Claimer::claim`] is a compare-exchange loop, because it checks
//! headroom against the consumer and grants in one step. That is not an
//! implementation shortcut that a better claim could remove: a `fetch_add`
//! claim on a *bounded* ring hands out sequences past the consumer's tail and
//! then has to undo them, and there is no wait-free undo. **Wait-freedom and
//! bounded capacity are exclusive at the claim** — this crate takes bounded
//! capacity, which is the property
//! `docs/non_functional_requirement/002_bounded_capacity_backpressure.md`
//! requires and the mechanism it replaces does not have.
//!
//! What remains true, and is the point of separating the claim out, is that
//! only the claim is contended. The payload write happens through
//! [`Reserved`]'s `DerefMut` with no synchronization at all, and the publish is
//! a single store.
//!
//! # The unsafe, and where its argument lives
//!
//! Producers write slots while the consumer reads slots, through shared
//! references to one allocation.
//! [decision 123](../../../docs/decision/123_ring_shared_slot_storage_unsafe_sited.md)
//! rules that the `unsafe` belongs here rather than in `ring_store` or
//! `ring_slot`, because the invariant making it sound is stated entirely in
//! terms of cursors and stamps those crates do not hold. The argument is in
//! `Ring::slot`'s and `Ring::slot_mut`'s safety sections, and the shape it
//! rests on is asserted in `tests/mpsc_test.rs` rather than only described.
//!
//! # What the type system refuses
//!
//! Decision 123 ruling 4 requires a test of the *shape* the soundness argument
//! rests on. Here that shape is the asymmetry: [`Producer`] is `Copy` and
//! `Sync` — that is what "multi-producer" means — while [`Consumer`] is
//! neither, because the drain's read-scan-then-commit is not re-entrant.
//!
//! These are `compile_fail` doc tests rather than integration tests because
//! rustdoc collects doc tests from the library target only; the same blocks in
//! `tests/mpsc_test.rs` would never be compiled.
//!
//! A second consumer cannot be made by cloning the first:
//!
//! ```compile_fail
//! use ring_mpsc::{ Consumer, Ring };
//! use ring_slot::TypedSlot;
//!
//! fn assert_clone< T : Clone >() {}
//! assert_clone::< Consumer< 'static, TypedSlot< u8 > > >();
//! ```
//!
//! Nor by sharing one with a second thread:
//!
//! ```compile_fail
//! use ring_mpsc::Consumer;
//! use ring_slot::TypedSlot;
//!
//! fn assert_sync< T : Sync >() {}
//! assert_sync::< Consumer< 'static, TypedSlot< u8 > > >();
//! ```
//!
//! Nor by splitting twice while the first pair is live:
//!
//! ```compile_fail
//! use ring_mpsc::Ring;
//! use ring_slot::TypedSlot;
//! use ring_types::Capacity;
//!
//! let mut ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 4 ).unwrap() );
//! let mut ends = ring.ends();
//! let ( _producer, _consumer ) = ends.split();
//! let ( _again, _also ) = ends.split();
//! ```
//!
//! Nor by holding two [`Ends`] from one ring at once — `ends` takes `&mut
//! self`, so a second call cannot borrow while the first is still live, which
//! is the fact [`Ring`]'s `unsafe impl Sync` argument rests on
//! (→ [`../docs/workaround/002`](../docs/workaround/002_an_unsafe_impl_sync_the_compiler_cannot_derive.md)):
//!
//! ```compile_fail
//! use ring_mpsc::Ring;
//! use ring_slot::TypedSlot;
//! use ring_types::Capacity;
//!
//! let mut ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 4 ).unwrap() );
//! let ends1 = ring.ends();
//! let ends2 = ring.ends();
//! let _ = ( ends1, ends2 );
//! ```
//!
//! And a [`Reserved`] cannot outlive the batch it would publish into, because
//! it borrows the ring:
//!
//! ```compile_fail
//! use ring_mpsc::Ring;
//! use ring_slot::TypedSlot;
//! use ring_types::Capacity;
//!
//! let reserved =
//! {
//!   let mut ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 4 ).unwrap() );
//!   let mut ends = ring.ends();
//!   let ( producer, _consumer ) = ends.split();
//!   producer.claim().unwrap()
//! };
//! drop( reserved );
//! ```
//!
//! # Using it
//!
//! ```
//! use ring_mpsc::Ring;
//! use ring_slot::TypedSlot;
//! use ring_types::Capacity;
//!
//! let mut ring : Ring< TypedSlot< u32 > > = Ring::new( Capacity::new( 8 ).unwrap() );
//! let mut ends = ring.ends();
//! let ( producer, mut consumer ) = ends.split();
//!
//! std::thread::scope( | scope |
//! {
//!   for tag in 0 .. 4_u32
//!   {
//!     // `move` copies the producer rather than moving it — that is `Copy`,
//!     // and it is the whole difference from a single-producer ring.
//!     scope.spawn( move || producer.push( tag ).unwrap() );
//!   }
//! } );
//!
//! let mut seen = Vec::new();
//! while seen.len() < 4
//! {
//!   let mut batch = consumer.drain();
//!   for offset in 0 .. batch.len()
//!   {
//!     seen.push( batch.get_mut( offset ).and_then( TypedSlot::take ).unwrap() );
//!   }
//! }
//! seen.sort_unstable();
//! assert_eq!( seen, vec![ 0, 1, 2, 3 ] );
//! ```
//!
//! Acceptance is binary and lives in a test, not here: feature 172 is Reached
//! when `tests/mpsc_test.rs` has four producers exchange 100 000 items with
//! byte-parity, no sequence granted twice, and each producer's own items in its
//! issue order — and cites `docs/feature/172_` textually, which is the only
//! crate→feature edge the family records.

#![deny(missing_docs)]
#![allow(unsafe_code)]

use core::cell::{Cell, UnsafeCell};
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::Ordering;

// `SeqCell` is one trait with two re-exports — `ring_cursor` republishes
// `ring_atomic`'s. It is taken from `ring_atomic` here because both cell types
// this crate touches need it: the unpadded stamps and the padded consumer
// cursor.
use ring_atomic::{AtomicSeq, SeqCell};
use ring_claim::{Claim, Claimer};
use ring_config::RingConfig;
use ring_cursor::{GATING, PaddedCursor};
use ring_gating::GatingSet;
use ring_slot::{Slot, TypedSlot};
use ring_store::Buffer;
use ring_types::{Capacity, RingError, Seq};

/// The stamp value of a slot no producer has published into yet.
///
/// Not a sentinel the protocol depends on — see the module documentation on why
/// the stamp needs none. It exists so that lap zero, where a slot's stamp has
/// never been written, has a value that is not a sequence any drain will ever
/// look for.
///
/// ```
/// use ring_types::Seq;
///
/// assert_eq!( ring_mpsc::UNSTAMPED, Seq( u64::MAX ) );
/// ```
pub const UNSTAMPED: Seq = Seq(u64::MAX);

/// The ordering a producer's stamp store is made visible with.
///
/// `Release`, paired with [`OBSERVE`]: everything the producer wrote into the
/// slot before this store is visible to a consumer that observes the stamp.
/// Weakening it to `Relaxed` produces a ring that works on x86, where the
/// hardware supplies the ordering the code failed to ask for, and races on
/// aarch64 — `docs/invariant/002_publication_ordering.md`'s Pair 1.
///
/// ```
/// use core::sync::atomic::Ordering;
///
/// assert_eq!( ring_mpsc::PUBLISH, Ordering::Release );
/// ```
pub const PUBLISH: Ordering = Ordering::Release;

/// The ordering the consumer reads a stamp with.
///
/// `Acquire`, the other half of [`PUBLISH`]. This load is what makes the
/// producer's slot writes visible; the drain's payload reads must not be
/// hoisted above it.
///
/// ```
/// use core::sync::atomic::Ordering;
///
/// assert_eq!( ring_mpsc::OBSERVE, Ordering::Acquire );
/// ```
pub const OBSERVE: Ordering = Ordering::Acquire;

/// The ordering the consumer releases drained slots with.
///
/// `Release`, paired with the producers' [`ring_cursor::GATING`] read of the
/// same cursor inside [`ring_claim::Claimer::claim`]'s headroom check —
/// `docs/invariant/002_publication_ordering.md`'s Pair 2. The consumer's
/// payload reads precede this store in program order and must not sink below
/// it, or a producer that observes the advance overwrites a slot still being
/// read.
///
/// The external design corpus gets this one wrong, and the divergence is
/// deliberate: message 663 advances the read cursor `Relaxed`, justified as
/// "the Mutator is the only one who changes tail". Sole-writership answers a
/// different question than reclamation ordering asks.
///
/// ```
/// use core::sync::atomic::Ordering;
///
/// assert_eq!( ring_mpsc::COMMIT, Ordering::Release );
/// ```
pub const COMMIT: Ordering = Ordering::Release;

/// The ordering the consumer reads **its own** cursor with.
///
/// `Relaxed` is sound because the consumer cursor has exactly one writer: the
/// thread performing this load is the thread that performed the store it is
/// reading back, and program order already sequences the two. Contrast
/// [`ring_cursor::GATING`], which is what the *producers* read that same cursor
/// with, and where `Acquire` is load-bearing.
///
/// ```
/// use core::sync::atomic::Ordering;
///
/// assert_eq!( ring_mpsc::OWN, Ordering::Relaxed );
/// assert_eq!( ring_cursor::GATING, Ordering::Acquire );
/// ```
pub const OWN: Ordering = Ordering::Relaxed;

/// A ring many threads write and one thread reads.
///
/// Four fields: the slot array, one stamp per slot, the gating set holding
/// the single consumer cursor, and the claim cursor. The claim cursor lives
/// here rather than inside the [`Claimer`] that [`ends`] constructs, because
/// a claimer is built fresh per `ends` call and a cursor that started at
/// zero every time would hand the second generation's producers sequences
/// the first generation had already moved past — records pushed through
/// them answer `Ok` and are never delivered. Owned by the ring, the cell
/// persists: each `ends` builds its claimer over the same cell, which
/// continues from wherever the previous generation stopped. `Claimer` still
/// borrows the gating set it checks headroom against — one ring, two
/// borrows, no self-reference.
///
/// [`ends`]: Self::ends
///
/// ```
/// use ring_mpsc::Ring;
/// use ring_slot::BytesSlot;
/// use ring_types::Capacity;
///
/// let ring : Ring< BytesSlot< 16 > > = Ring::new( Capacity::new( 4 ).unwrap() );
/// assert_eq!( ring.capacity().get(), 4 );
/// assert_eq!( ring.stamps().len(), 4, "one stamp per slot, not one per lap" );
/// ```
pub struct Ring<S> {
  /// The slot array, behind an `UnsafeCell` because producers write into it
  /// through a shared reference while the consumer reads from it.
  ///
  /// The cell is on each slot, not around the whole `Buffer`.
  ///
  /// It wrapped the buffer until `Buffer::new` was bounded on `Default` rather
  /// than `Slot`, which was the only thing making `Buffer< UnsafeCell< S > >`
  /// unconstructible. The outer form was unsound: reaching a slot through
  /// `( *cell.get() ).at_mut( seq )` materialises `&mut Buffer< S >`, an
  /// exclusive claim over the *entire* allocation, so two producers writing
  /// two different slots aliased the whole buffer. Miri's data-race detector
  /// reports it as a retag conflict on `Buffer< S >` itself rather than on any
  /// slot — the producers never touched the same record. A per-slot cell claims
  /// exactly the slot being written, which is what the claim protocol below
  /// actually guarantees to be exclusive.
  ///
  /// `ring_store` still knows nothing of this crate: it stores whatever
  /// element type it is given, and every `unsafe` stays here, which is
  /// decision 123's whole point.
  slots: Buffer<UnsafeCell<S>>,
  /// One stamp per slot, holding the sequence whose payload currently occupies
  /// it. Unpadded on purpose: [`PaddedCursor`] would make this array 64 times
  /// the size of the payload array for a small `S`, to prevent a false-sharing
  /// contention that does not arise — two producers writing adjacent stamps are
  /// two producers that claimed adjacent sequences, which is a handful of
  /// stores on one line rather than a contended loop.
  stamps: Box<[AtomicSeq]>,
  consumers: GatingSet,
  /// The claim cursor, owned by the ring so a claim grant persists across
  /// `ends` generations. On its own cache line — it is the line every
  /// producer's compare-exchange moves, and it must not share one with the
  /// consumer cursor those exchanges read.
  claim_cursor: PaddedCursor,
}

// SAFETY: `Ring` is shared as the `&Ring` held by any number of `Producer`s and
// exactly one `Consumer` — `ends` takes `&mut self` and `split` takes `&mut
// Ends`, so no second consumer can exist while a first is alive, and `Consumer`
// is neither `Clone` nor `Sync`. Producers and the consumer touch disjoint slots
// at every instant: a producer holds `&mut` to exactly the slot of the sequence
// it claimed and has not yet stamped, and the consumer reads only slots whose
// stamp equals the sequence addressing them — a stamp is written after the
// payload and before the claim of the next lap's sequence for that slot, which
// `Claimer`'s headroom check gates behind the consumer's commit. The stamp
// carries the producer→consumer happens-before edge (`PUBLISH` store, `OBSERVE`
// load) and the consumer cursor carries the consumer→producer one (`COMMIT`
// store, `GATING` load). `S : Send` is required because a record is written on
// a producer's thread and read on the consumer's.
unsafe impl<S: Send> Sync for Ring<S> {}

impl<S: Slot + Default> Ring<S> {
  /// Allocate a ring of `capacity` slots, with every stamp [`UNSTAMPED`].
  ///
  /// ```
  /// use ring_mpsc::Ring;
  /// use ring_slot::TypedSlot;
  /// use ring_types::Capacity;
  ///
  /// let ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 16 ).unwrap() );
  /// assert_eq!( ring.capacity().get(), 16 );
  /// assert_eq!( ring.published_through(), None, "nothing is published yet" );
  /// ```
  #[must_use]
  pub fn new(capacity: Capacity) -> Self {
    let stamps = (0..capacity.get())
      .map(|_| AtomicSeq::new(UNSTAMPED))
      .collect::<Vec<_>>()
      .into_boxed_slice();

    Self {
      slots: Buffer::new(capacity),
      stamps,
      consumers: GatingSet::new(capacity, 1),
      claim_cursor: PaddedCursor::default(),
    }
  }

  /// Allocate a ring sized by a [`RingConfig`].
  ///
  /// Only the capacity is read. A config's wait strategy and overflow policy
  /// describe what a *caller* does when the ring is full, and this crate never
  /// waits and never drops — it reports [`RingError::Full`] and lets the caller
  /// choose, which is the one of
  /// `docs/non_functional_requirement/002_bounded_capacity_backpressure.md`'s
  /// three policies that preserves the exactly-once contract without
  /// surrendering producer progress.
  ///
  /// `producers` is likewise not read. A ring that trusted it would be trusting
  /// a number no caller can be held to; the claim is correct for any number of
  /// producers because it is a compare-exchange, not because it was told one.
  ///
  /// ```
  /// use ring_config::RingConfig;
  /// use ring_mpsc::Ring;
  /// use ring_slot::TypedSlot;
  ///
  /// let config = RingConfig::new( 32 ).unwrap().with_producers( 4 );
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
  /// use ring_mpsc::Ring;
  /// use ring_slot::TypedSlot;
  /// use ring_types::Capacity;
  ///
  /// let ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 64 ).unwrap() );
  /// assert_eq!( ring.capacity().get(), 64 );
  /// ```
  #[must_use]
  pub fn capacity(&self) -> Capacity {
    self.consumers.capacity()
  }

  /// The stamp array, one entry per slot.
  ///
  /// Exposed so a test can assert the publication protocol directly rather than
  /// through its effect on a drain — the two differ exactly when the drain is
  /// wrong, which is the case worth being able to see.
  ///
  /// ```
  /// use ring_mpsc::{ Ring, UNSTAMPED };
  /// use ring_atomic::SeqCell;
  /// use ring_slot::TypedSlot;
  /// use ring_types::Capacity;
  /// use core::sync::atomic::Ordering;
  ///
  /// let ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 4 ).unwrap() );
  /// assert!( ring.stamps().iter().all( | s | s.load( Ordering::Relaxed ) == UNSTAMPED ) );
  /// ```
  #[must_use]
  pub fn stamps(&self) -> &[AtomicSeq] {
    &self.stamps
  }

  /// The consumer's cursor — the exclusive upper bound of what it has drained.
  ///
  /// ```
  /// use ring_mpsc::Ring;
  /// use ring_slot::TypedSlot;
  /// use ring_types::{ Capacity, Seq };
  ///
  /// let ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 4 ).unwrap() );
  /// assert_eq!( ring.committed(), Seq::ZERO );
  /// ```
  #[must_use]
  pub fn committed(&self) -> Seq {
    self.consumer_cursor().load(GATING)
  }

  /// The highest sequence published with no gap below it, or `None` when the
  /// slot at the consumer's position is not published.
  ///
  /// **This is the published watermark, and it stops at the first gap rather
  /// than at the highest stamped sequence.** The two differ under out-of-order
  /// publication — producer B stamping before producer A leaves A's sequence
  /// unpublished below B's — and only the former preserves the total order
  /// `docs/invariant/001_single_consumer_total_order.md` states.
  ///
  /// Scanning from the consumer's position bounds the walk by the ring's
  /// capacity, because a producer more than `capacity` ahead could not have
  /// claimed.
  ///
  /// ```
  /// use ring_mpsc::Ring;
  /// use ring_slot::TypedSlot;
  /// use ring_types::{ Capacity, Seq };
  ///
  /// let mut ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 4 ).unwrap() );
  /// let mut ends = ring.ends();
  /// let ( producer, _consumer ) = ends.split();
  ///
  /// let first = producer.claim().unwrap();
  /// let second = producer.claim().unwrap();
  /// drop( second );                        // sequence 1 published, 0 is not
  /// assert_eq!( producer.ring().published_through(), None );
  ///
  /// drop( first );                         // the gap closes
  /// assert_eq!( producer.ring().published_through(), Some( Seq( 1 ) ) );
  /// ```
  #[must_use]
  pub fn published_through(&self) -> Option<Seq> {
    let from = self.committed();
    let end = self.contiguous_end(from, self.capacity().get());

    if end == from { None } else { Some(Seq(end.0 - 1)) }
  }

  /// The exclusive end of the published run starting at `from`, capped at `max`.
  ///
  /// The whole drain protocol, in one loop: a slot is published exactly when its
  /// stamp equals the sequence addressing it, so the scan stops at the first
  /// sequence whose stamp does not — whether because no producer wrote it yet,
  /// or because the stamp still holds the previous lap's sequence.
  ///
  /// **Do not weaken the comparison below.** `stamp != UNSTAMPED` and
  /// `stamp >= end` both read a stale stamp from the previous lap as
  /// published — see
  /// `docs/pitfall/002_a_stale_stamp_reads_as_unpublished_not_as_wrong.md`.
  /// Only equality is correct.
  fn contiguous_end(&self, from: Seq, max: usize) -> Seq {
    let mut end = from;

    for _ in 0..max {
      // Equality, not `!= UNSTAMPED` or `>= end` — a stamp from the previous
      // lap fails equality for the same reason an unwritten one does, and a
      // weaker comparison reads either as published.
      if self.stamp(end).load(OBSERVE) != end {
        break;
      }
      end = end.next();
    }

    end
  }

  /// The stamp cell addressing `seq`.
  fn stamp(&self, seq: Seq) -> &AtomicSeq {
    let index = (seq.0 as usize) & self.capacity().mask();
    // The mask is `capacity - 1` for a power-of-two capacity, which `Capacity`
    // enforces at construction, so the index is always in range.
    &self.stamps[index]
  }

  /// The single consumer cursor.
  fn consumer_cursor(&self) -> &PaddedCursor {
    self
      .consumers
      .cursor(0)
      .expect("a gating set built with one consumer has cursor 0")
  }

  /// The ends, to be split and moved onto threads.
  ///
  /// Two steps rather than one because [`Claimer`] borrows the [`GatingSet`]
  /// it checks headroom against and the [`PaddedCursor`] it exchanges — one
  /// ring, two borrows, no self-reference. `&mut self` is what keeps the
  /// claim cursor single-owner: there is no moment at which two `Ends` name
  /// one ring.
  ///
  /// The claim cursor is the ring's own cell, not a fresh one: a claimer
  /// built here continues from wherever the previous generation's claimer
  /// stopped. Before the cell moved into the ring, a second `ends` on a used
  /// ring started its producers back at sequence zero under a consumer
  /// cursor that had moved on — pushes answered `Ok` and were never
  /// delivered (→ [`pitfall/003`](../docs/pitfall/003_a_fresh_ends_restarted_the_claim_cursor.md)).
  ///
  /// ```
  /// use ring_mpsc::Ring;
  /// use ring_slot::TypedSlot;
  /// use ring_types::{ Capacity, Seq };
  ///
  /// let mut ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 4 ).unwrap() );
  /// let ends = ring.ends();
  /// assert_eq!( ends.ring().committed(), Seq::ZERO );
  /// ```
  pub fn ends(&mut self) -> Ends<'_, S> {
    let shared: &Self = self;

    Ends {
      ring: shared,
      claimer: Claimer::borrowed(&shared.consumers, &shared.claim_cursor),
    }
  }

  /// The slot `seq` maps to, for reading.
  ///
  /// # Safety
  ///
  /// The caller must be the consumer end, and `seq` must be **published and not
  /// yet committed** — at or after the consumer cursor and strictly before the
  /// published watermark obtained by an [`OBSERVE`] load of that slot's stamp.
  /// Those two bounds place the slot outside every producer's writable set: a
  /// producer holds `&mut` only to a sequence it has claimed and not stamped,
  /// and a stamped sequence is by definition not that.
  ///
  /// The `OBSERVE` load that established the upper bound is also what makes the
  /// producer's write to this slot visible; reading a slot on the strength of a
  /// bound obtained any other way is a data race even if the arithmetic holds.
  unsafe fn slot(&self, seq: Seq) -> &S {
    // SAFETY: the caller guarantees `seq` is published and not committed, so no
    // `&mut` to this slot exists — the only outstanding `&mut`s are to claimed,
    // unstamped sequences. `at` yields `&UnsafeCell< S >`, a shared borrow of
    // one slot rather than of the buffer, so a producer's concurrent write to a
    // different slot is not an alias of this borrow.
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
  /// - **An unpublished producer claim** — it called [`Producer::claim`] and
  ///   the returned [`Reserved`] has not yet been dropped. No other producer
  ///   can have claimed `seq`, because the claim is a compare-exchange over one
  ///   cursor; the consumer cannot be reading it, because the stamp has not
  ///   been stored; and no producer of a later lap can have claimed it, because
  ///   the claim's headroom check gates on the consumer's commit, which cannot
  ///   pass this sequence before it is even drained.
  /// - **A published, not-yet-committed consumer batch** — reached through
  ///   [`Batch::get_mut`], between `drain`/`drain_up_to` and the batch's
  ///   `Drop`. The producer that published `seq` has already released its own
  ///   `&mut` (the stamp store happens in [`Reserved`]'s `Drop`, strictly after
  ///   its last write), and no later producer may claim `seq` again until this
  ///   batch's `Drop` commits it — the same headroom gate as above, seen from
  ///   the consumer's side of it.
  #[allow(clippy::mut_from_ref)]
  unsafe fn slot_mut(&self, seq: Seq) -> &mut S {
    // SAFETY: the caller is the slot's sole owner under one of the two regimes
    // documented above (an unpublished producer claim, or a published
    // not-yet-committed consumer batch reached through `Batch::get_mut`), so no
    // other `&S`/`&mut S` to this slot is live. `at` yields `&UnsafeCell< S >`,
    // so the write permission this deref needs comes from that one slot's cell
    // and claims nothing about any other slot — which is what lets a second
    // producer write its own claimed slot, or the consumer drain a batch, at
    // the same instant.
    unsafe { &mut *self.slots.at(seq).get() }
  }
}

impl<S> core::fmt::Debug for Ring<S> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.debug_struct("Ring")
      .field("capacity", &self.capacity().get())
      .field("committed", &self.committed())
      .field("published_through", &self.published_through())
      .finish()
  }
}

/// A ring's two ends, before they are split.
///
/// Holds the [`Claimer`] — and therefore the claim cursor — that every producer
/// shares. It exists as a separate type only because `Claimer` borrows the
/// [`GatingSet`] inside the ring; see [`Ring::ends`].
#[derive(Debug)]
pub struct Ends<'a, S> {
  ring: &'a Ring<S>,
  claimer: Claimer<'a>,
}

impl<'a, S> Ends<'a, S> {
  /// The ring these ends belong to.
  ///
  /// ```
  /// use ring_mpsc::Ring;
  /// use ring_slot::TypedSlot;
  /// use ring_types::Capacity;
  ///
  /// let mut ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 4 ).unwrap() );
  /// let ends = ring.ends();
  /// assert_eq!( ends.ring().capacity().get(), 4 );
  /// ```
  #[must_use]
  pub const fn ring(&self) -> &'a Ring<S> {
    self.ring
  }

  /// One producer handle and the one consumer.
  ///
  /// The producer is [`Copy`] — copy it once per thread; that is what
  /// multi-producer means and why no `Clone` bound is needed. The consumer is
  /// not, and is not `Sync` either.
  ///
  /// `&mut self` is what makes the consumer unique: a second call would hand
  /// out a second `Consumer`, and two threads each scanning-then-committing the
  /// same cursor would each drain records the other had already taken.
  ///
  /// ```
  /// use ring_mpsc::Ring;
  /// use ring_slot::TypedSlot;
  /// use ring_types::Capacity;
  ///
  /// let mut ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 4 ).unwrap() );
  /// let mut ends = ring.ends();
  /// let ( producer, mut consumer ) = ends.split();
  ///
  /// producer.push( 7 ).unwrap();
  /// let mut batch = consumer.drain();
  /// assert_eq!( batch.get_mut( 0 ).and_then( TypedSlot::take ), Some( 7 ) );
  /// ```
  pub fn split(&'a mut self) -> (Producer<'a, S>, Consumer<'a, S>) {
    (
      Producer {
        ring: self.ring,
        claimer: &self.claimer,
      },
      Consumer {
        ring: self.ring,
        _one_thread: PhantomData,
      },
    )
  }
}

/// A handle any number of threads may write through.
///
/// `Copy` on purpose: a producer is two shared references, so copying one is
/// free and giving each thread its own is the intended use. Contrast
/// [`Consumer`], which is neither `Copy` nor `Sync`.
#[derive(Debug)]
pub struct Producer<'a, S> {
  ring: &'a Ring<S>,
  claimer: &'a Claimer<'a>,
}

impl<S> Clone for Producer<'_, S> {
  fn clone(&self) -> Self {
    *self
  }
}

impl<S> Copy for Producer<'_, S> {}

impl<'a, S> Producer<'a, S> {
  /// Reserve the next sequence, or report that the ring is full.
  ///
  /// The returned [`Reserved`] derefs to the slot and publishes when dropped.
  /// The claim is the only contended step; the write through the guard and the
  /// publish on drop are not.
  ///
  /// # Errors
  ///
  /// [`RingError::Full`] when no slot is free — back-pressure, so a retry loop
  /// should keep going. This is the *fail* policy of
  /// `docs/non_functional_requirement/002_bounded_capacity_backpressure.md`'s
  /// three; block and overwrite are not implemented, because both are per-
  /// priority-class decisions that instance explicitly declines to resolve, and
  /// overwrite additionally violates the exactly-once contract.
  ///
  /// ```
  /// use ring_mpsc::Ring;
  /// use ring_slot::TypedSlot;
  /// use ring_types::{ Capacity, RingError };
  ///
  /// let mut ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 2 ).unwrap() );
  /// let mut ends = ring.ends();
  /// let ( producer, _consumer ) = ends.split();
  ///
  /// let first = producer.claim().unwrap();
  /// let second = producer.claim().unwrap();
  /// assert_eq!( producer.claim().err(), Some( RingError::Full ) );
  /// drop( ( first, second ) );
  /// ```
  pub fn claim(&self) -> Result<Reserved<'a, S>, RingError> {
    let claim = self.claimer.claim(1)?;

    Ok(Reserved {
      ring: self.ring,
      seq: claim.start(),
    })
  }

  /// Reserve up to `max` sequences with one gate check and one exchange.
  ///
  /// The batched form of [`claim`]: the claimer grants `1..=max` contiguous
  /// sequences — whatever headroom the gate allows at the value the exchange
  /// runs against — and the whole grant costs one read of the consumer cursor
  /// and one compare-exchange, no matter how many sequences it covers. A
  /// producer that writes records in groups amortises the contended step over
  /// the group instead of paying it per record.
  ///
  /// The grant is *adaptive*: a ring with three slots free answers a
  /// `claim_batch( 64 )` with three sequences, so a producer under pressure
  /// keeps making progress at whatever width the ring allows rather than
  /// spinning until the full width appears.
  ///
  /// The returned [`ReservedBatch`] reaches its slots by offset and publishes
  /// the whole grant when dropped — including any offset that was never
  /// written, which publishes an empty slot exactly as a dropped [`Reserved`]
  /// does. **A held guard parks the consumer behind its whole range**: the
  /// drain stops at the first unpublished sequence of the claim, so hold the
  /// guard for one group of writes, not for longer.
  ///
  /// [`claim`]: Self::claim
  ///
  /// # Errors
  ///
  /// [`RingError::Full`] when nothing is granted — the ring is full, or `max`
  /// is zero. Nothing advances.
  ///
  /// ```
  /// use ring_mpsc::Ring;
  /// use ring_slot::TypedSlot;
  /// use ring_types::{ Capacity, RingError, Seq };
  ///
  /// let mut ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 4 ).unwrap() );
  /// let mut ends = ring.ends();
  /// let ( producer, _consumer ) = ends.split();
  ///
  /// let batch = producer.claim_batch( 3 ).unwrap();
  /// assert_eq!( batch.len(), 3 );
  /// assert_eq!( batch.sequence( 0 ), Some( Seq::ZERO ) );
  /// assert_eq!( producer.claim_batch( 0 ).err(), Some( RingError::Full ) );
  /// drop( batch );
  /// ```
  pub fn claim_batch(&self, max: usize) -> Result<ReservedBatch<'a, S>, RingError> {
    let claim = self.claimer.claim_up_to(max)?;

    Ok(ReservedBatch { ring: self.ring, claim })
  }

  /// Room a claim may consume — **advisory**.
  ///
  /// A caller reading this and then claiming performs two operations with a
  /// gap; other producers may consume the room between them. That is not a
  /// caveat but a structural property of a contended claim, and it is why the
  /// value is a hint rather than a guarantee: the only reliable question is
  /// whether [`claim`] succeeded.
  ///
  /// [`claim`]: Self::claim
  ///
  /// ```
  /// use ring_mpsc::Ring;
  /// use ring_slot::TypedSlot;
  /// use ring_types::Capacity;
  ///
  /// let mut ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 4 ).unwrap() );
  /// let mut ends = ring.ends();
  /// let ( producer, _consumer ) = ends.split();
  ///
  /// assert_eq!( producer.free_capacity(), 4 );
  /// let reserved = producer.claim().unwrap();
  /// assert_eq!( producer.free_capacity(), 3 );
  /// drop( reserved );
  /// ```
  #[must_use]
  pub fn free_capacity(&self) -> usize {
    self.claimer.headroom()
  }

  /// The next sequence a claim would grant.
  ///
  /// Producer-side, and deliberately not on [`Ends`]: `split` borrows the ends
  /// for the rest of their life, so anything observable *after* a split has to
  /// be reachable from a handle.
  ///
  /// ```
  /// use ring_mpsc::Ring;
  /// use ring_slot::TypedSlot;
  /// use ring_types::{ Capacity, Seq };
  ///
  /// let mut ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 4 ).unwrap() );
  /// let mut ends = ring.ends();
  /// let ( producer, _consumer ) = ends.split();
  ///
  /// assert_eq!( producer.claimed(), Seq::ZERO );
  /// let reserved = producer.claim().unwrap();
  /// assert_eq!( reserved.sequence(), Seq::ZERO );
  /// assert_eq!( producer.claimed(), Seq( 1 ), "the claim advances before the publish" );
  /// drop( reserved );
  /// ```
  #[must_use]
  pub fn claimed(&self) -> Seq {
    self.claimer.claimed()
  }

  /// Whether the claim cursor and the consumer cursor occupy different cache
  /// lines.
  ///
  /// `docs/integration/001_family_dependency_seam.md`'s seam I2 states padding
  /// as a contract this crate depends on but `ring_cursor` implements. Asserted
  /// from here rather than trusted, because a sibling change that dropped the
  /// alignment would cost this crate a contended line on its hottest path and
  /// break nothing that compiles.
  ///
  /// The two cursors are in different allocations — one in the ring's gating
  /// set, one in the claimer — so this is a check on `PaddedCursor`'s alignment
  /// rather than on their layout relative to each other.
  ///
  /// ```
  /// use ring_mpsc::Ring;
  /// use ring_slot::TypedSlot;
  /// use ring_types::Capacity;
  ///
  /// let mut ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 4 ).unwrap() );
  /// let mut ends = ring.ends();
  /// let ( producer, _consumer ) = ends.split();
  ///
  /// assert!( producer.on_distinct_lines() );
  /// ```
  #[must_use]
  pub fn on_distinct_lines(&self) -> bool {
    let claim = self.claimer.cursor().addr();
    let consume = self.ring.consumer_cursor().addr();

    claim.abs_diff(consume) >= 64
  }

  /// The ring this end writes into.
  #[must_use]
  pub const fn ring(&self) -> &'a Ring<S> {
    self.ring
  }
}

impl<'a, T> Producer<'a, TypedSlot<T>> {
  /// Claim, write and publish one value.
  ///
  /// The convenience over [`claim`] for a payload that is already built. It
  /// gives up the ring's ability to construct a large `T` *in place* in the
  /// slot, which is the reason the guard is the primary surface.
  ///
  /// [`claim`]: Self::claim
  ///
  /// # Errors
  ///
  /// [`RingError::Full`], exactly as [`claim`] — the value is returned to the
  /// caller by never being consumed, so a retry may pass the same one again.
  ///
  /// ```
  /// use ring_mpsc::Ring;
  /// use ring_slot::TypedSlot;
  /// use ring_types::{ Capacity, Seq };
  ///
  /// let mut ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 4 ).unwrap() );
  /// let mut ends = ring.ends();
  /// let ( producer, _consumer ) = ends.split();
  ///
  /// assert_eq!( producer.push( 1 ).unwrap(), Seq::ZERO );
  /// assert_eq!( producer.push( 2 ).unwrap(), Seq( 1 ) );
  /// ```
  pub fn push(&self, value: T) -> Result<Seq, RingError> {
    let mut reserved = self.claim()?;
    let seq = reserved.sequence();
    // The displaced value is dropped, deliberately, and this is not the same
    // situation as `ring_core`'s identical line — which asserts the slot was
    // empty. It can assert that because its own consumer always drains with
    // `TypedSlot::take`. This crate hands out `Batch`, and a consumer that
    // reads through `get`/`peek` instead leaves the record in place, so on any
    // lap after the first the slot legitimately still holds it. Dropping it
    // here is what bounds the ring's storage.
    reserved.set(value);

    Ok(seq)
  }

  /// Claim, write and publish as many of `records` as the ring has room for.
  ///
  /// The batched convenience over [`claim_batch`](Producer::claim_batch): one
  /// gate check and one exchange for the whole group, the granted prefix
  /// drained from `records` and written into the slots, the grant published
  /// when the guard drops. Returns how many records went in; the rest stay in
  /// `records`, in order, for the next attempt. An empty `records` returns
  /// `Ok( 0 )` without touching the ring.
  ///
  /// # Errors
  ///
  /// [`RingError::Full`] when nothing was granted — `records` is left
  /// untouched, exactly as [`push`] leaves the value with its caller.
  ///
  /// [`push`]: Self::push
  ///
  /// ```
  /// use ring_mpsc::Ring;
  /// use ring_slot::TypedSlot;
  /// use ring_types::Capacity;
  ///
  /// let mut ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 4 ).unwrap() );
  /// let mut ends = ring.ends();
  /// let ( producer, mut consumer ) = ends.split();
  ///
  /// let mut records = vec![ 1, 2, 3 ];
  /// assert_eq!( producer.push_batch( &mut records ).unwrap(), 3 );
  /// assert!( records.is_empty() );
  ///
  /// let mut drained = consumer.drain();
  /// assert_eq!( drained.len(), 3 );
  /// assert_eq!( drained.get_mut( 2 ).and_then( TypedSlot::take ), Some( 3 ) );
  /// ```
  pub fn push_batch(&self, records: &mut Vec<T>) -> Result<usize, RingError> {
    if records.is_empty() {
      return Ok(0);
    }

    let mut guard = self.claim_batch(records.len())?;
    let granted = guard.len();
    for (offset, value) in records.drain(..granted).enumerate() {
      guard.slot_mut(offset).expect("offset within the granted range").set(value);
    }
    drop(guard);

    Ok(granted)
  }
}

/// A claimed, not-yet-published slot.
///
/// Derefs to the slot, and publishes it when dropped. The guard shape is what
/// makes the publish impossible to skip: a producer that claims and returns
/// early would otherwise wedge the ring permanently, since the consumer stops
/// at the first unpublished sequence and would never pass this one.
///
/// **A guard dropped without a write publishes an empty slot, not a torn one.**
/// The slot was left `Default` by the consumer that drained it, so a panic
/// between claim and write costs one empty record — an observable, defined
/// outcome rather than undefined behaviour. That is why no completion flag is
/// tracked: there is nothing for it to prevent.
#[derive(Debug)]
pub struct Reserved<'a, S> {
  ring: &'a Ring<S>,
  seq: Seq,
}

impl<S> Reserved<'_, S> {
  /// The sequence this guard owns.
  ///
  /// ```
  /// use ring_mpsc::Ring;
  /// use ring_slot::TypedSlot;
  /// use ring_types::{ Capacity, Seq };
  ///
  /// let mut ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 4 ).unwrap() );
  /// let mut ends = ring.ends();
  /// let ( producer, _consumer ) = ends.split();
  ///
  /// assert_eq!( producer.claim().unwrap().sequence(), Seq::ZERO );
  /// ```
  #[must_use]
  pub const fn sequence(&self) -> Seq {
    self.seq
  }
}

impl<S> Deref for Reserved<'_, S> {
  type Target = S;

  fn deref(&self) -> &S {
    // SAFETY: this guard holds an unpublished claim on `self.seq`, which is
    // `slot_mut`'s precondition; reading through it is strictly weaker.
    unsafe { self.ring.slot_mut(self.seq) }
  }
}

impl<S> DerefMut for Reserved<'_, S> {
  fn deref_mut(&mut self) -> &mut S {
    // SAFETY: this guard holds an unpublished claim on `self.seq` — it was
    // granted by a compare-exchange no other producer won, the consumer cannot
    // reach it before the stamp is stored, and the next lap's claim of the same
    // slot is gated behind the consumer's commit of this one.
    unsafe { self.ring.slot_mut(self.seq) }
  }
}

impl<S> Drop for Reserved<'_, S> {
  /// Publish, with the one `Release` store the whole protocol turns on.
  fn drop(&mut self) {
    self.ring.stamp(self.seq).store(self.seq, PUBLISH);
  }
}

/// A claimed, not-yet-published range of slots.
///
/// The batched form of [`Reserved`]: one compare-exchange grants the whole
/// contiguous range, [`slot_mut`](Self::slot_mut) reaches each slot by
/// offset, and the drop publishes every sequence of the grant. Where
/// [`Reserved`] exists to make one publish impossible to skip, this guard
/// exists to make *k* publishes impossible to skip — a range whose publish
/// could be partially skipped would wedge the ring at its first unwritten
/// sequence, exactly as a single skipped publish would.
///
/// **A guard dropped without writing every offset publishes an empty slot
/// per unwritten offset, not a torn one** — the same defined outcome
/// [`Reserved`] documents, one slot at a time. The consumer reads each slot
/// on its own and sees an empty payload where nothing was written.
///
/// **A held guard parks the consumer behind its whole range.** The drain
/// stops at the first unpublished sequence, which while this guard is alive
/// is its first one. The guard is a write group, not a long-lived
/// reservation: claim, write, drop.
#[derive(Debug)]
pub struct ReservedBatch<'a, S> {
  ring: &'a Ring<S>,
  claim: Claim,
}

impl<S> ReservedBatch<'_, S> {
  /// How many sequences the grant covers.
  ///
  /// ```
  /// use ring_mpsc::Ring;
  /// use ring_slot::TypedSlot;
  /// use ring_types::Capacity;
  ///
  /// let mut ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 4 ).unwrap() );
  /// let mut ends = ring.ends();
  /// let ( producer, _consumer ) = ends.split();
  ///
  /// assert_eq!( producer.claim_batch( 2 ).unwrap().len(), 2 );
  /// ```
  #[must_use]
  pub const fn len(&self) -> usize {
    self.claim.len()
  }

  /// Whether the grant covers nothing — never true: a claim that would grant
  /// nothing is refused as `Full` rather than handed out empty.
  #[must_use]
  pub const fn is_empty(&self) -> bool {
    self.claim.is_empty()
  }

  /// The first sequence of the grant.
  #[must_use]
  pub const fn start(&self) -> Seq {
    self.claim.start()
  }

  /// The sequence at `offset` within the grant, or `None` past its end.
  ///
  /// ```
  /// use ring_mpsc::Ring;
  /// use ring_slot::TypedSlot;
  /// use ring_types::{ Capacity, Seq };
  ///
  /// let mut ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 4 ).unwrap() );
  /// let mut ends = ring.ends();
  /// let ( producer, _consumer ) = ends.split();
  ///
  /// let batch = producer.claim_batch( 2 ).unwrap();
  /// assert_eq!( batch.sequence( 0 ), Some( Seq::ZERO ) );
  /// assert_eq!( batch.sequence( 1 ), Some( Seq( 1 ) ) );
  /// assert_eq!( batch.sequence( 2 ), None );
  /// ```
  #[must_use]
  pub fn sequence(&self, offset: usize) -> Option<Seq> {
    (offset < self.claim.len()).then(|| self.claim.start().advanced_by(offset as u64))
  }

  /// The slot at `offset`, for writing — [`Reserved`]'s `DerefMut`, by
  /// offset.
  ///
  /// `None` when `offset` is past the grant's end, which the caller can check
  /// with [`len`](Self::len) rather than catch.
  ///
  /// ```
  /// use ring_mpsc::Ring;
  /// use ring_slot::TypedSlot;
  /// use ring_types::Capacity;
  ///
  /// let mut ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 4 ).unwrap() );
  /// let mut ends = ring.ends();
  /// let ( producer, mut consumer ) = ends.split();
  ///
  /// let mut batch = producer.claim_batch( 2 ).unwrap();
  /// batch.slot_mut( 0 ).expect( "within the grant" ).set( 7 );
  /// drop( batch );
  ///
  /// let mut drained = consumer.drain();
  /// assert_eq!( drained.len(), 2 );
  /// assert_eq!( drained.get_mut( 0 ).and_then( TypedSlot::take ), Some( 7 ) );
  /// ```
  pub fn slot_mut(&mut self, offset: usize) -> Option<&mut S> {
    let seq = self.sequence(offset)?;
    let ring = self.ring;

    // SAFETY: the compare-exchange that granted `self.claim` gave this guard
    // sole ownership of every sequence in it — no other producer can win the
    // same range while the grant stands, the consumer cannot reach any of the
    // sequences before their stamps are stored by the drop, and the next
    // lap's claim of any sequence in the range is gated behind the consumer's
    // commit of this one. `Reserved`'s safety argument, per sequence of the
    // range.
    Some(unsafe { ring.slot_mut(seq) })
  }
}

impl<S> Drop for ReservedBatch<'_, S> {
  /// Publish the whole grant, one `Release` store per sequence, in issue
  /// order. Every sequence gets its stamp whether it was written or not — a
  /// range that stopped publishing at its first unwritten sequence would
  /// wedge the drain behind it forever, which is the exact failure the guard
  /// shape exists to make impossible.
  fn drop(&mut self) {
    for seq in self.claim.sequences() {
      self.ring.stamp(seq).store(seq, PUBLISH);
    }
  }
}

/// The one handle that drains.
///
/// Not `Clone` and not `Sync`: the drain reads the consumer cursor, scans
/// forward, hands out the records and only then commits, so two of these would
/// each hand out records the other had already taken.
#[derive(Debug)]
pub struct Consumer<'a, S> {
  ring: &'a Ring<S>,
  /// `Cell` is `Send` and not `Sync`, so this marker makes the end movable to a
  /// thread and unshareable between two. Without it the end would inherit
  /// `Sync` from `&Ring`, and two threads holding `&Consumer` could each drain
  /// the same records.
  _one_thread: PhantomData<Cell<()>>,
}

impl<'a, S> Consumer<'a, S> {
  /// The next sequence this end will drain.
  ///
  /// ```
  /// use ring_mpsc::Ring;
  /// use ring_slot::TypedSlot;
  /// use ring_types::{ Capacity, Seq };
  ///
  /// let mut ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 4 ).unwrap() );
  /// let mut ends = ring.ends();
  /// let ( producer, mut consumer ) = ends.split();
  ///
  /// assert_eq!( consumer.position(), Seq::ZERO );
  /// producer.push( 1 ).unwrap();
  /// drop( consumer.drain() );
  /// assert_eq!( consumer.position(), Seq( 1 ) );
  /// ```
  #[must_use]
  pub fn position(&self) -> Seq {
    self.ring.consumer_cursor().load(OWN)
  }

  /// How many records are published and undrained right now.
  ///
  /// ```
  /// use ring_mpsc::Ring;
  /// use ring_slot::TypedSlot;
  /// use ring_types::Capacity;
  ///
  /// let mut ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 4 ).unwrap() );
  /// let mut ends = ring.ends();
  /// let ( producer, consumer ) = ends.split();
  ///
  /// assert!( consumer.is_empty() );
  /// producer.push( 1 ).unwrap();
  /// producer.push( 2 ).unwrap();
  /// assert_eq!( consumer.available(), 2 );
  /// ```
  #[must_use]
  pub fn available(&self) -> usize {
    let from = self.position();
    let end = self.ring.contiguous_end(from, self.ring.capacity().get());

    from.distance_to(end) as usize
  }

  /// Whether nothing is drainable.
  ///
  /// Non-destructive — reads [`available`](Self::available) and drains
  /// nothing. Contrast [`Batch::is_empty`]: `consumer.drain().is_empty()`
  /// commits the whole drain as a side effect of taking it, discarding every
  /// currently published record along the way, whereas `consumer.is_empty()`
  /// never touches the ring.
  #[must_use]
  pub fn is_empty(&self) -> bool {
    self.available() == 0
  }

  /// Every published, undrained record, as one batch.
  ///
  /// The batch borrows the consumer, and commits when dropped — the slots are
  /// not released for reuse until the caller is finished reading them, which is
  /// what makes the `Release` on the commit meaningful.
  ///
  /// ```
  /// use ring_mpsc::Ring;
  /// use ring_slot::TypedSlot;
  /// use ring_types::Capacity;
  ///
  /// let mut ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 4 ).unwrap() );
  /// let mut ends = ring.ends();
  /// let ( producer, mut consumer ) = ends.split();
  ///
  /// producer.push( 1 ).unwrap();
  /// producer.push( 2 ).unwrap();
  ///
  /// let mut batch = consumer.drain();
  /// assert_eq!( batch.len(), 2 );
  /// assert_eq!( batch.get_mut( 1 ).and_then( TypedSlot::take ), Some( 2 ) );
  /// ```
  pub fn drain(&mut self) -> Batch<'_, S> {
    self.drain_up_to(self.ring.capacity().get())
  }

  /// At most `max` published, undrained records.
  ///
  /// For a consumer that wants a bounded amount of work per tick rather than
  /// whatever accumulated — `docs/pitfall/001_spinning_consumer_owns_a_core.md`
  /// is about that cadence.
  ///
  /// ```
  /// use ring_mpsc::Ring;
  /// use ring_slot::TypedSlot;
  /// use ring_types::Capacity;
  ///
  /// let mut ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 4 ).unwrap() );
  /// let mut ends = ring.ends();
  /// let ( producer, mut consumer ) = ends.split();
  ///
  /// producer.push( 1 ).unwrap();
  /// producer.push( 2 ).unwrap();
  /// producer.push( 3 ).unwrap();
  ///
  /// assert_eq!( consumer.drain_up_to( 2 ).len(), 2 );
  /// assert_eq!( consumer.drain_up_to( 2 ).len(), 1 );
  /// ```
  pub fn drain_up_to(&mut self, max: usize) -> Batch<'_, S> {
    let start = self.position();
    let end = self.ring.contiguous_end(start, max.min(self.ring.capacity().get()));

    Batch {
      ring: self.ring,
      start,
      len: start.distance_to(end) as usize,
    }
  }

  /// The ring this end drains.
  #[must_use]
  pub const fn ring(&self) -> &'a Ring<S> {
    self.ring
  }
}

/// A contiguous run of published records, committed when dropped.
///
/// Holds no copy of the records: `get` and `get_mut` reach into the ring's own
/// slots, which is what keeps a large payload from being moved on the drain
/// path. The slots stay reserved for exactly as long as this value lives.
#[derive(Debug)]
pub struct Batch<'a, S> {
  ring: &'a Ring<S>,
  start: Seq,
  len: usize,
}

impl<S> Batch<'_, S> {
  /// How many records the batch holds.
  #[must_use]
  pub const fn len(&self) -> usize {
    self.len
  }

  /// Whether the batch is empty — a drain of an empty ring.
  ///
  /// The batch already exists by the time this is checked: this answers a
  /// settled fact about a drain that already committed, not a live question
  /// about the ring. `consumer.drain().is_empty()` discards every currently
  /// published record as a side effect of the `drain()` call alone — for a
  /// non-destructive check, call [`Consumer::is_empty`] instead.
  #[must_use]
  pub const fn is_empty(&self) -> bool {
    self.len == 0
  }

  /// The first sequence in the batch.
  ///
  /// ```
  /// use ring_mpsc::Ring;
  /// use ring_slot::TypedSlot;
  /// use ring_types::{ Capacity, Seq };
  ///
  /// let mut ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 4 ).unwrap() );
  /// let mut ends = ring.ends();
  /// let ( producer, mut consumer ) = ends.split();
  ///
  /// producer.push( 1 ).unwrap();
  /// drop( consumer.drain() );
  /// producer.push( 2 ).unwrap();
  /// assert_eq!( consumer.drain().start(), Seq( 1 ) );
  /// ```
  #[must_use]
  pub const fn start(&self) -> Seq {
    self.start
  }

  /// Every sequence in the batch, in order.
  pub fn sequences(&self) -> impl Iterator<Item = Seq> + use<S> {
    let start = self.start;

    (0..self.len as u64).map(move |offset| start.advanced_by(offset))
  }

  /// The record at `offset`, or `None` past the end.
  #[must_use]
  pub fn get(&self, offset: usize) -> Option<&S> {
    if offset >= self.len {
      return None;
    }

    // SAFETY: the batch's whole range was published — every sequence in it
    // passed `contiguous_end`'s `OBSERVE` stamp comparison — and none of it is
    // committed, because the commit happens in this batch's `Drop`.
    Some(unsafe { self.ring.slot(self.start.advanced_by(offset as u64)) })
  }

  /// The record at `offset`, mutably — how a payload is moved out.
  ///
  /// Mutable access is the consumer's alone, and sound for the same reason
  /// `get` is: the producer of a batched sequence published it and released its
  /// own `&mut` before this batch could see it, and no later producer may claim
  /// the slot again until this batch's `Drop` commits.
  ///
  /// ```
  /// use ring_mpsc::Ring;
  /// use ring_slot::TypedSlot;
  /// use ring_types::Capacity;
  ///
  /// let mut ring : Ring< TypedSlot< String > > = Ring::new( Capacity::new( 4 ).unwrap() );
  /// let mut ends = ring.ends();
  /// let ( producer, mut consumer ) = ends.split();
  ///
  /// producer.push( "moved, not copied".to_string() ).unwrap();
  /// let mut batch = consumer.drain();
  /// assert_eq!( batch.get_mut( 0 ).and_then( TypedSlot::take ).as_deref(), Some( "moved, not copied" ) );
  /// ```
  #[must_use]
  pub fn get_mut(&mut self, offset: usize) -> Option<&mut S> {
    if offset >= self.len {
      return None;
    }

    // SAFETY: as `get`, plus exclusivity — `&mut self` on this batch, which is
    // the only handle to the range, is what makes the `&mut S` unique.
    Some(unsafe { self.ring.slot_mut(self.start.advanced_by(offset as u64)) })
  }

  /// Every record in the batch, in order.
  pub fn iter(&self) -> impl Iterator<Item = &S> {
    (0..self.len).filter_map(|offset| self.get(offset))
  }
}

impl<S> Drop for Batch<'_, S> {
  /// Commit, releasing the slots for reuse.
  ///
  /// The `Release` here pairs with the [`ring_cursor::GATING`] load inside
  /// every producer's headroom check. Weakening it lets a producer that sees
  /// the advance overwrite a slot whose read is still in flight — the same torn
  /// read as a missing publish barrier, arriving from the opposite direction.
  fn drop(&mut self) {
    self
      .ring
      .consumer_cursor()
      .store(self.start.advanced_by(self.len as u64), COMMIT);
  }
}
