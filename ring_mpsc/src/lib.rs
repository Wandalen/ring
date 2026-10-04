//! Multi-producer single-consumer ring API.
//!
//! Part of the ring family's concurrency write path.
//!
//! Many producer threads claim a slot and publish into it concurrently, and one
//! consumer thread drains published slots in total order. This is the
//! ring/merge half of a mechanism that more than one independent consumer
//! needs, factored out so it is built once.
//!
//! # Publication is a per-slot stamp, and that is this crate's whole addition
//!
//! `ring_publish` exists and is deliberately **not** used here. Its own module
//! documentation says why, about this crate by name: tracking per-slot
//! availability "is what a high-contention multi-producer ring eventually
//! needs. It is deliberately not here, because it is `ring_mpsc`'s problem".
//! Its `Publisher::publish` spins until the *predecessor* producer has
//! published, which makes one producer's progress depend on another's. That is
//! the one coupling the contended-claim feature exists to remove.
//!
//! So publication is a `Release` store into `stamps[ seq & mask ]`, and a slot
//! is published exactly when its stamp equals the sequence addressing it. No
//! producer waits for another to publish; the consumer pays a scan instead.
//!
//! # Why the stamp needs no sentinel
//!
//! A stamp holds the sequence whose payload occupies that slot. Sequences
//! `i`, `i + capacity`, `i + 2·capacity`, … all address slot `i`, a different
//! one on every lap. So the drain's test is equality against the sequence it
//! is *looking for*, and a stale stamp from the previous lap fails it for the
//! same reason a never-written one does. Stamps are initialised to
//! [`UNSTAMPED`] only so that lap zero has something that is not a valid
//! sequence to fail against.
//!
//! This is why the stamp is a full [`Seq`] and not a one-bit ready flag. A flag
//! would need clearing on reclamation. That is a second write to the same line,
//! on the consumer's hot path, to re-establish what the sequence already
//! encodes.
//!
//! # The claim is lock-free, not wait-free
//!
//! [`ring_claim::Claimer::claim`] is a compare-exchange loop, because it checks
//! headroom against the consumer and grants in one step. A better claim could
//! not remove the loop. A `fetch_add` claim on a *bounded* ring hands out
//! sequences past the consumer's tail and then has to undo them, and there is
//! no wait-free undo. **Wait-freedom and bounded capacity are exclusive at the
//! claim.** This crate takes bounded capacity, which is the property the
//! backpressure requirement asks for and the mechanism it replaces does not
//! have.
//!
//! Only the claim is contended, and that is the point of separating it out.
//! The payload write happens through
//! [`Reserved`]'s `DerefMut` with no synchronization at all, and the publish is
//! a single store.
//!
//! # Invariant: the one consumer receives every published sequence once, in claim order
//!
//! Each published sequence is drained exactly once, and the drained order is
//! the order the claims were granted, which is the order the compare-exchanges
//! in [`ring_claim::Claimer::claim`] won. Producers finish writing in any
//! order. The drain turns that back into claim order by stopping at the first
//! sequence not yet published (see [`Ring::published_through`]). How the
//! consumer splits the stream into batches cannot change the order, because
//! every [`Batch`] walks its range in ascending sequence.
//!
//! **Excluded.** The interleaving between producers. It is whatever order their
//! claims won, so only each producer's own records keep its issue order.
//!
//! **Enforced by.** `four_producers_exchange_one_hundred_thousand_items_with_byte_parity`
//! (every record once, no sequence granted twice, each producer's issue order),
//! `every_slot_is_reused_across_many_laps_without_loss_or_duplication` and
//! `the_drain_stops_at_the_first_unpublished_sequence_not_the_highest_published`,
//! all in `tests/mpsc_test.rs`.
//!
//! # Invariant: a payload write happens before its read, and the read before the slot's reuse
//!
//! Two happens-before edges, one in each direction, made of the pairings
//! [`PUBLISH`], [`OBSERVE`] and [`COMMIT`] document. Nothing here uses
//! `SeqCst`, because nothing needs one order across every atomic in the
//! process.
//!
//! A broken edge is a data race. A broken publish edge shows the consumer the
//! slot as the previous lap left it, in whole or in part. That is an empty slot
//! if the consumer took the record, and otherwise some or all of the old
//! record. An old record seen whole passes any check of the record's shape and
//! arrives in place of the new one, so a count of records still balances. A
//! broken commit edge lets a producer overwrite a slot the consumer is still
//! reading, so the consumer sees part or all of the next lap's record instead.
//!
//! **Enforced by.** Not the compiler, which accepts any `Ordering`.
//! `the_orderings_are_the_ones_the_publication_invariant_names` pins the
//! ordering constants' values, and the loom models in `tests/mpsc_test.rs`'s
//! `exhaustive` module check the publish edge under `--cfg loom`.
//! `docs/workaround/readme.md` records that nothing behavioural checks
//! `COMMIT`.
//!
//! # The unsafe, and where its argument lives
//!
//! Producers write slots while the consumer reads slots, through shared
//! references to one allocation. `docs/workaround/readme.md` records that the
//! `unsafe` belongs here rather than in `ring_store` or `ring_slot`, because
//! the invariant making it sound is stated entirely in terms of cursors and
//! stamps those crates do not hold. The argument is in `Ring::slot`'s and
//! `Ring::slot_mut`'s safety sections, and the shape it rests on is asserted in
//! `tests/mpsc_test.rs` rather than only described.
//!
//! # What the type system refuses
//!
//! The soundness argument needs a test of the *shape* it rests on. Here that
//! shape is an asymmetry. [`Producer`] is `Copy` and `Sync`, which is what
//! "multi-producer" means. [`Consumer`] is neither, because the drain's
//! read-scan-then-commit is not re-entrant.
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
//! Nor by holding two [`Ends`] from one ring at once. `ends` takes `&mut self`,
//! so a second call cannot borrow while the first is still live. [`Ring`]'s
//! `unsafe impl Sync` argument rests on that fact
//! (→ `docs/workaround/readme.md`):
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
//! Acceptance is binary and lives in a test, not here. The crate's feature is
//! Reached when `tests/mpsc_test.rs` has four producers exchange 100 000 items
//! with byte-parity, no sequence granted twice, and each producer's own items
//! in its issue order, and when that file cites the feature textually. That
//! citation is the only crate→feature edge the family records.

#![deny(missing_docs)]
#![allow(unsafe_code)]

use core::cell::{Cell, UnsafeCell};
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::Ordering;

// `SeqCell` is one trait with two re-exports, since `ring_cursor` republishes
// `ring_atomic`'s. This crate imports it from `ring_atomic` because both cell
// types it touches need it: the unpadded stamps and the padded consumer
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
/// The protocol does not depend on it as a sentinel; the module documentation
/// explains why the stamp needs none. It exists so that lap zero, where a
/// slot's stamp has never been written, has a value that is not a sequence any
/// drain will ever look for.
///
/// A zeroed stamp array would not do. Slot 0's first sequence is `Seq( 0 )`,
/// so a zero stamp there reads as published before any producer has written
/// it. The first drain then reads slot 0 while its producer may still be
/// writing it, and commits past record zero, which is never delivered.
/// `stamps_start_unstamped_and_there_is_exactly_one_per_slot` pins the value.
///
/// ```
/// use ring_types::Seq;
///
/// assert_eq!( ring_mpsc::UNSTAMPED, Seq( u64::MAX ) );
/// ```
pub const UNSTAMPED: Seq = Seq(u64::MAX);

/// The ordering a producer's stamp store is made visible with.
///
/// `Release`, paired with [`OBSERVE`]. Everything the producer wrote into the
/// slot before this store is visible to a consumer that observes the stamp.
/// Weakening it to `Relaxed` produces a ring that works on x86, where the
/// hardware supplies the ordering the code failed to ask for, and races on
/// aarch64.
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
/// same cursor inside [`ring_claim::Claimer::claim`]'s headroom check. The
/// consumer's payload reads precede this store in program order and must not
/// sink below it, or a producer that observes the advance overwrites a slot
/// still being read.
///
/// The external design corpus gets this one wrong, and this crate diverges from
/// it on purpose. It advances the read cursor `Relaxed`, justified as "the
/// Mutator is the only one who changes tail". Sole-writership answers a
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
/// `Relaxed` is sound because the consumer cursor has exactly one writer. The
/// thread performing this load is the thread that performed the store it is
/// reading back, and program order already sequences the two. A consumer moved
/// to another thread stays sound for the reason `ring_spsc::OWN` gives. Contrast
/// [`ring_cursor::GATING`], which is what the *producers* read that same cursor
/// with, and where `Acquire` is required.
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
/// # Lifecycle: construction and teardown
///
/// [`Ring::new`] allocates the slot array and the stamp array, and nothing
/// allocates after it. Claiming, publishing and draining reuse those slots, so
/// a full ring cannot grow and refuses instead.
///
/// Dropping the ring drops every record still in a slot, drained or not,
/// because the slot array drops its elements. This crate has no teardown code.
/// `every_record_written_is_destroyed_exactly_once` counts the drops.
///
/// There is no reset. Setting both cursors back to zero would leave stamps from
/// the old run that equal sequences the new run has not published yet, and the
/// drain would hand those slots out. Calling [`ends`] a second time is not a
/// reset either, and is unsound today, as its pitfall explains. Reusing a ring
/// means building a new one.
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
  /// unconstructible. The outer form was unsound. Reaching a slot through
  /// `( *cell.get() ).at_mut( seq )` materialises `&mut Buffer< S >`, an
  /// exclusive claim over the *entire* allocation, so two producers writing
  /// two different slots aliased the whole buffer. Miri's data-race detector
  /// reports it as a retag conflict on `Buffer< S >` itself rather than on any
  /// slot. The producers never touched the same record. A per-slot cell claims
  /// exactly the slot being written, which is what the claim protocol below
  /// guarantees to be exclusive.
  ///
  /// `ring_store` still knows nothing of this crate. It stores whatever
  /// element type it is given, and every `unsafe` stays here, which is the
  /// whole point of the opt-out in `docs/workaround/readme.md`.
  slots: Buffer<UnsafeCell<S>>,
  /// One stamp per slot, holding the sequence whose payload currently occupies
  /// it. Unpadded on purpose. [`PaddedCursor`] would make this array 64 times
  /// the size of the payload array for a small `S`, to prevent a false-sharing
  /// contention that does not arise. Two producers writing adjacent stamps are
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
// exactly one `Consumer`. `ends` takes `&mut self` and `split` takes `&mut
// Ends`, so no second consumer can exist while a first is alive, and `Consumer`
// is neither `Clone` nor `Sync`. Producers and the consumer touch disjoint slots
// at every instant. A producer holds `&mut` to exactly the slot of the sequence
// it claimed and has not yet stamped, and the consumer reads only slots whose
// stamp equals the sequence addressing them. The producer writes a stamp after
// the payload and before the claim of the next lap's sequence for that slot,
// and `Claimer`'s headroom check gates that claim behind the consumer's commit.
// The stamp carries the producer→consumer happens-before edge (`PUBLISH` store,
// `OBSERVE` load) and the consumer cursor carries the consumer→producer one
// (`COMMIT` store, `GATING` load). `S : Send` is required because a record is
// written on a producer's thread and read on the consumer's.
//
// The disjointness argument holds only for the first `ends` on a ring. A second
// call starts a new claim cursor at zero while the consumer cursor and stamps
// keep their values, so the headroom check no longer keeps two claims off one
// slot. `Ring::ends` documents it as a pitfall until the claim cursor carries
// over.
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
  /// This reads only the capacity. A config's wait strategy and overflow policy
  /// describe what a *caller* does when the ring is full, and this crate never
  /// waits and never drops. It reports [`RingError::Full`] and lets the caller
  /// choose, which is the one of the three backpressure policies that
  /// preserves the exactly-once contract without surrendering producer
  /// progress.
  ///
  /// It does not read `producers` either. A ring that trusted it would be
  /// trusting a number no caller can be held to; the claim is correct for any
  /// number of producers because it is a compare-exchange, not because it was
  /// told one.
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
  /// through its effect on a drain. The two differ exactly when the drain is
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

  /// The consumer's cursor, the exclusive upper bound of what it has drained.
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
  /// publication: producer B stamping before producer A leaves A's sequence
  /// unpublished below B's. Only the first-gap watermark preserves the total
  /// order.
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
  /// The whole drain protocol, in one loop. A slot is published exactly when its
  /// stamp equals the sequence addressing it, so the scan stops at the first
  /// sequence whose stamp does not. Either no producer wrote it yet, or the
  /// stamp still holds the previous lap's sequence.
  ///
  /// **Do not weaken the comparison below.** Only equality is correct.
  /// `stamp >= end` reads every unwritten stamp as published, since
  /// [`UNSTAMPED`] is the largest sequence, so it fails on the first lap.
  /// `stamp != UNSTAMPED` reads a previous lap's stamp as published and passes
  /// any test that never wraps the ring.
  /// `a_stale_stamp_from_the_previous_lap_does_not_read_as_published` catches
  /// it.
  fn contiguous_end(&self, from: Seq, max: usize) -> Seq {
    let mut end = from;

    for _ in 0..max {
      // Equality, not `!= UNSTAMPED` or `>= end`. A stamp from the previous
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
  /// ring, two borrows, no self-reference. `&mut self` is what makes the claim
  /// cursor unique: there is no moment at which two `Ends` name one ring.
  ///
  /// # Pitfall: a fresh claim cursor hands out dead sequences
  ///
  /// **Trap.** Calling `ends` again once the first [`Ends`] is gone, to run a
  /// second set of producers against the same ring.
  ///
  /// **Failure.** A [`Claimer`] built over a cursor that starts at zero grants
  /// sequences the first generation's producers already claimed, while the
  /// consumer cursor and the stamps keep the first run's values. The headroom
  /// check measures from the old consumer cursor, so the new claimer grants
  /// sequences up to a full capacity past it. Two live [`Reserved`] guards
  /// then address one slot, and safe code holds two exclusive references to
  /// it, which is undefined behaviour. Records claimed below the consumer
  /// cursor are never drained — pushes answer `Ok` and are never delivered.
  ///
  /// **Mitigation.** The claim cursor is the ring's own cell, not a fresh
  /// one: a claimer built here continues from wherever the previous
  /// generation's claimer stopped, so a second `ends` on a used ring
  /// continues rather than restarts. Nothing rejects the second call — the
  /// carried-over cursor is what makes it sound.
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
  /// yet committed**. That means at or after the consumer cursor, and strictly
  /// before the published watermark obtained by an [`OBSERVE`] load of that
  /// slot's stamp. Those two bounds place the slot outside every producer's
  /// writable set. A producer holds `&mut` only to a sequence it has claimed
  /// and not stamped, and a stamped sequence is by definition not that.
  ///
  /// The `OBSERVE` load that established the upper bound is also what makes the
  /// producer's write to this slot visible; reading a slot on the strength of a
  /// bound obtained any other way is a data race even if the arithmetic holds.
  unsafe fn slot(&self, seq: Seq) -> &S {
    // SAFETY: the caller guarantees `seq` is published and not committed, so no
    // `&mut` to this slot exists. The only outstanding `&mut`s are to claimed,
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
  /// whole life. No other `&S`/`&mut S` to the same slot may be live at the
  /// same time. Two call sites establish that, each on its own terms. An audit
  /// of this function recorded that a reader who stops at the first is left
  /// thinking the second one violates this doc. The two call sites are:
  ///
  /// - **An unpublished producer claim.** It called [`Producer::claim`] and
  ///   the returned [`Reserved`] has not yet been dropped. No other producer
  ///   can have claimed `seq`, because the claim is a compare-exchange over one
  ///   cursor. The consumer cannot be reading it, because the stamp has not
  ///   been stored. No producer of a later lap can have claimed it, because
  ///   the claim's headroom check gates on the consumer's commit, which cannot
  ///   pass this sequence before it is even drained.
  /// - **A published, not-yet-committed consumer batch.** Reached through
  ///   [`Batch::get_mut`], between `drain`/`drain_up_to` and the batch's
  ///   `Drop`. The producer that published `seq` has already released its own
  ///   `&mut` (the stamp store happens in [`Reserved`]'s `Drop`, strictly after
  ///   its last write). No later producer may claim `seq` again until this
  ///   batch's `Drop` commits it. That is the same headroom gate as above, seen
  ///   from the consumer's side.
  #[allow(clippy::mut_from_ref)]
  unsafe fn slot_mut(&self, seq: Seq) -> &mut S {
    // SAFETY: the caller is the slot's sole owner under one of the two regimes
    // documented above (an unpublished producer claim, or a published
    // not-yet-committed consumer batch reached through `Batch::get_mut`), so no
    // other `&S`/`&mut S` to this slot is live. `at` yields `&UnsafeCell< S >`,
    // so the write permission this deref needs comes from that one slot's cell
    // and claims nothing about any other slot. That is what lets a second
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
/// Holds the [`Claimer`], and therefore the claim cursor, that every producer
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
  /// The producer is [`Copy`], so copy it once per thread. That is what
  /// multi-producer means, and why no `Clone` bound is needed. The consumer is
  /// not, and is not `Sync` either.
  ///
  /// `&mut self` is what makes the consumer unique. A second call would hand
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
/// `Copy` on purpose. A producer is two shared references, so copying one is
/// free, and giving each thread its own is the intended use. Contrast
/// [`Consumer`], which is neither `Copy` nor `Sync`.
///
/// # Lifecycle: a producer attaches and detaches without the ring knowing
///
/// A `Producer` is a capability to claim, not a registration. Copying one
/// writes nothing and dropping one writes nothing, so the ring never knows how
/// many producers exist or whether any are left. A consumer cannot tell a
/// producer that is quiet from one that is gone. That signal has to come from
/// outside the ring, and `ring_shutdown`'s close flag is the family's way to
/// send it.
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
  /// [`RingError::Full`] when no slot is free. That is back-pressure, so a retry
  /// loop should keep going. This is the *fail* policy of the three
  /// backpressure policies. Block and overwrite are not implemented, because
  /// both are per-priority-class decisions that are deliberately left
  /// unresolved, and overwrite also violates the exactly-once contract.
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

  /// Room a claim may consume, as an **advisory** figure.
  ///
  /// A caller reading this and then claiming performs two operations with a
  /// gap; other producers may consume the room between them. That gap is a
  /// structural property of a contended claim, and it is why the value is a
  /// hint rather than a guarantee. The only reliable question is whether
  /// [`claim`] succeeded.
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
  /// Producer-side, and deliberately not on [`Ends`]. `split` borrows the ends
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
  /// Padding is a contract this crate depends on but `ring_cursor` implements.
  /// This crate asserts it here rather than trusting it, because a sibling
  /// change that dropped the alignment would cost this crate a contended line
  /// on its hottest path and break nothing that compiles.
  ///
  /// The two cursors are in different allocations, the claim cursor in the
  /// ring itself and the consumer cursor in its gating set's `Vec`. So this is
  /// a check on `PaddedCursor`'s alignment rather than on their layout relative
  /// to each other. The line test is `ring_align`'s, so it moves with
  /// `ring_align::CACHE_LINE` rather than restating a size.
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

    ring_align::on_distinct_lines(claim, consume)
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
  /// slot, which is the reason the guard is the primary API.
  ///
  /// [`claim`]: Self::claim
  ///
  /// # Errors
  ///
  /// [`RingError::Full`], exactly as [`claim`]. The value is returned to the
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
    // The displaced value is dropped on purpose. This is not the same
    // situation as `ring_core`'s identical line, which asserts the slot was
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
/// makes the publish impossible to skip. Without it, a producer that claims and
/// returns early would stall the ring permanently, since the consumer stops at
/// the first unpublished sequence and would never pass this one.
///
/// **A guard dropped without a write publishes the slot as it stands, not a
/// torn one.** On the first lap that is an empty slot. On a later lap it is
/// whatever record the previous lap left, if the consumer read it through
/// `get` rather than taking it, and the consumer then receives that old record
/// again under the new sequence. Either way the outcome is defined rather than
/// undefined behaviour. No completion flag is tracked, so the consumer cannot
/// tell a republished record from a fresh one.
///
/// # Pitfall: a held guard stalls every later record
///
/// **Trap.** Keeping a `Reserved` alive across slow work after claiming, such
/// as building the payload, taking a lock or waiting on I/O.
///
/// **Failure.** The consumer stops at the first unpublished sequence, so every
/// record claimed after this one, by any producer, waits for this guard to
/// drop. [`Consumer::available`] counts only the records before it, so the
/// consumer cannot tell a slow producer from a quiet ring.
/// `an_unpublished_claim_blocks_every_later_sequence_while_it_is_held` shows
/// it.
///
/// **Mitigation.** Build the payload first and claim last. [`Producer::push`]
/// does that for a value that is already built.
///
/// # Pitfall: a panic after a partial write publishes the partial record
///
/// **Trap.** Writing a record into the slot a piece at a time through the
/// guard, with code between the pieces that can panic.
///
/// **Failure.** Unwinding runs this guard's `Drop`, which publishes the slot.
/// The consumer receives a record with some parts new and some left from
/// before, and nothing marks it as incomplete.
///
/// **Mitigation.** Build the record completely and move it into the slot in one
/// write, as [`Producer::push`] does.
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
    // SAFETY: this guard holds an unpublished claim on `self.seq`. It was
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
/// Not `Clone` and not `Sync`. The drain reads the consumer cursor, scans
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
  /// A lower bound. Producers only add records and only this end drains them,
  /// so the reading can go stale only by being too low, and a drain that
  /// follows finds at least this many.
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
  /// Non-destructive. It reads [`available`](Self::available) and drains
  /// nothing. Contrast [`Batch::is_empty`]. `consumer.drain().is_empty()`
  /// commits the whole drain as a side effect of taking it, and discards every
  /// currently published record along the way. `consumer.is_empty()` never
  /// touches the ring.
  #[must_use]
  pub fn is_empty(&self) -> bool {
    self.available() == 0
  }

  /// Every published, undrained record, as one batch.
  ///
  /// The batch borrows the consumer, and commits when dropped. The slots are
  /// not released for reuse until the caller is finished reading them, which is
  /// what makes the `Release` on the commit meaningful.
  ///
  /// # Pitfall: looping on `drain` keeps a core busy
  ///
  /// **Trap.** Calling `drain` in a loop until records arrive. Nothing in the
  /// ring wakes a consumer when a producer publishes, so a loop is the shape the
  /// API suggests.
  ///
  /// **Failure.** On a workload that produces records once per tick or frame,
  /// the loop occupies a whole core between ticks and finds an empty batch
  /// almost every time. A throughput benchmark does not show the cost, because
  /// it measures records per second on a machine where that core was free.
  ///
  /// **Mitigation.** Drain once per tick where the workload has one. A
  /// continuous pipeline with no tick is the opposite case. There polling is
  /// right, and draining on a tick would delay every record by up to a tick.
  /// The ring picks neither. `drain` and [`drain_up_to`](Self::drain_up_to)
  /// always return at once, and a caller that wants to wait can call
  /// `ring_wait::wait_until` with
  /// [`WaitKind::Park`](ring_types::WaitKind::Park) and
  /// `|| !consumer.is_empty()`.
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
  /// whatever accumulated.
  ///
  /// # Algorithm: scan the stamps, commit once
  ///
  /// The drain loads the stamps forward from [`position`](Self::position), one
  /// load per slot, and stops at the first that is not yet published. It hands
  /// the run out as one [`Batch`], whose `Drop` commits it with a single store
  /// to the consumer cursor. Finding a run costs a load per record and releasing
  /// it costs one store per batch. A drain that finds nothing still pays the
  /// load at the gap and a `COMMIT` store to the consumer cursor, whose line
  /// every producer's headroom check reads.
  ///
  /// Two alternatives were weighed. Tracking publication with a second cursor
  /// would make each producer wait for the one before it, which the module
  /// documentation explains this crate exists to avoid. The stamp array already
  /// is a per-slot availability array. The usual alternative stores a lap
  /// number or a ready flag there instead of the full sequence, and the module
  /// documentation explains why this crate stores the sequence.
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
/// Holds no copy of the records. `get` and `get_mut` reach into the ring's own
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

  /// Whether the batch is empty, which is what a drain of an empty ring yields.
  ///
  /// The batch already exists by the time this is checked, so this answers a
  /// settled fact about a drain that already committed, not a live question
  /// about the ring. `consumer.drain().is_empty()` discards every currently
  /// published record as a side effect of the `drain()` call alone. For a
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

    // SAFETY: the batch's whole range was published, since every sequence in it
    // passed `contiguous_end`'s `OBSERVE` stamp comparison. None of it is
    // committed, because the commit happens in this batch's `Drop`.
    Some(unsafe { self.ring.slot(self.start.advanced_by(offset as u64)) })
  }

  /// The record at `offset`, mutably, which is how a payload is moved out.
  ///
  /// Mutable access is the consumer's alone, and sound for the same reason
  /// `get` is. The producer of a batched sequence published it and released its
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

    // SAFETY: as `get`, plus exclusivity. `&mut self` on this batch, which is
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
  /// the advance overwrite a slot whose read is still in flight. That is the
  /// same torn read as a missing publish barrier, arriving from the opposite
  /// direction.
  fn drop(&mut self) {
    self
      .ring
      .consumer_cursor()
      .store(self.start.advanced_by(self.len as u64), COMMIT);
  }
}
