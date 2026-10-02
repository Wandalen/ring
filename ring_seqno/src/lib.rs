//! Sequence numbers and their comparison across laps.
//!
//! Part of the ring family's concurrency write path.
//!
//! The crate's whole subject is a single distinction that is easy to lose:
//! **the sequence does not wrap, the slot index does**.
//! A ring holds `capacity` slots, so two publications `capacity` apart land on
//! the same slot, but their [`ring_types::Seq`] values differ by exactly
//! `capacity`, and that difference is what a gate reads to decide whether the
//! older one has been consumed yet. Fold the sequence and the information is
//! gone; the gate can no longer tell "one lap behind" from "caught up".
//!
//! This crate was described as owning "wrapping arithmetic", which read as a
//! contradiction of the requirement that the sequence "never wraps". What wraps
//! is the index derived downstream, not the sequence handled here.
//!
//! Every function here is arithmetic over two values its caller already
//! loaded, so how stale an answer can be depends on the loads.
//! `ring_cursor::CursorPair` and `ring_cursor::slowest` state it.
//!
//! # Invariant: the sequence is never folded here
//!
//! Everything here is comparison and span arithmetic over unfolded sequences.
//! The folding itself is `ring_index`'s job, deliberately in another crate so
//! that no function can accidentally do both.
//!
//! **Excluded.** A type-level wall between counts and slots. [`free_slots`]
//! returns a plain `usize`, the type [`ring_types::SlotIndex`] wraps, so
//! nothing stops a caller indexing a buffer with it.
//!
//! **Enforced by.** The manifest, which does not list `ring_index`, so no
//! function here can call the folding. Nothing rejects a `%` or a
//! `& capacity.mask()` written inline.
//! `positions_many_laps_apart_stay_comparable` catches one that reaches the
//! readings, because its two positions fold to the same slot.
//!
//! # Invariant: every reading is total
//!
//! Every function here returns a value for every input. None returns a
//! `Result` and none can panic. Two of the guarantees come from `ring_types`.
//!
//! - [`laps_between`] divides by the capacity, which cannot be zero. A
//!   [`Capacity`] has a private field, so [`Capacity::new`] is the only way to
//!   make one, and it rejects `0`.
//! - [`laps_between`], [`may_claim`], [`free_slots`] and [`pending`] start from
//!   [`Seq::distance_to`], which saturates, so a pair given in the wrong order
//!   measures zero apart instead of underflowing.

#![deny(missing_docs)]

use ring_types::{Capacity, Seq};

/// How far apart two sequences are, in laps of a given capacity.
///
/// A ring is safe to publish into exactly while the producer is less than one
/// full lap ahead of the slowest consumer. `laps_between` is the reading that
/// decides it.
///
/// Takes `( earlier, later, .. )`, the opposite order from [`may_claim`],
/// [`free_slots`] and [`pending`], which all take the later (producer)
/// position first. A swapped call still compiles and still returns a
/// plausible-looking `0` rather than an error.
///
/// ```
/// use ring_types::{ Capacity, Seq };
/// use ring_seqno::laps_between;
///
/// let cap = Capacity::new( 8 ).unwrap();
/// assert_eq!( laps_between( Seq( 0 ), Seq( 7 ), cap ), 0 );
/// assert_eq!( laps_between( Seq( 0 ), Seq( 8 ), cap ), 1 );
/// assert_eq!( laps_between( Seq( 0 ), Seq( 17 ), cap ), 2 );
/// ```
#[must_use]
pub fn laps_between(earlier: Seq, later: Seq, capacity: Capacity) -> u64 {
  earlier.distance_to(later) / capacity.get() as u64
}

/// Whether `producer` may claim without overwriting a slot `consumer` has not
/// yet reached.
///
/// True exactly while the producer is strictly less than one lap ahead. At
/// exactly one lap the next claim would land on the consumer's current slot,
/// so the boundary is exclusive, which guards against the off-by-one known as
/// a lap bug.
///
/// ```
/// use ring_types::{ Capacity, Seq };
/// use ring_seqno::may_claim;
///
/// let cap = Capacity::new( 4 ).unwrap();
/// assert!( may_claim( Seq( 3 ), Seq( 0 ), cap ) );  // 3 ahead of 4 slots: room
/// assert!( !may_claim( Seq( 4 ), Seq( 0 ), cap ) ); // exactly one lap: no room
/// assert!( may_claim( Seq( 4 ), Seq( 1 ), cap ) );  // consumer moved on
/// ```
#[must_use]
pub fn may_claim(producer: Seq, consumer: Seq, capacity: Capacity) -> bool {
  consumer.distance_to(producer) < capacity.get() as u64
}

/// How many slots are free for a producer at `producer` given a consumer at
/// `consumer`.
///
/// Zero when the ring is full, and also when the producer is more than a lap
/// ahead, because the subtraction saturates. A producer that appears behind
/// its consumer reads as a whole capacity free, since [`Seq::distance_to`]
/// saturates too. `ring_debug::Violation::ConsumerAheadOfProducer` exists for
/// that state.
///
/// # Algorithm: why the distance stays `u64`
///
/// The distance is subtracted from the capacity in `u64`, and only the result
/// is narrowed to `usize`. The result is at most `capacity`, itself a `usize`,
/// so that narrowing is exact on every target. Narrowing the distance first
/// would truncate it wherever `usize` is narrower than 64 bits. On a 32-bit
/// target a producer `2^32 + 4` ahead of its consumer would read as 4 in
/// flight, and this would report free slots while [`may_claim`] on the same
/// pair said no.
///
/// No test catches a regression here on a 64-bit host, where both orders
/// compile to the same code, and the workspace does not enable
/// `clippy::cast_possible_truncation`.
///
/// ```
/// use ring_types::{ Capacity, Seq };
/// use ring_seqno::free_slots;
///
/// let cap = Capacity::new( 4 ).unwrap();
/// assert_eq!( free_slots( Seq( 0 ), Seq( 0 ), cap ), 4 );
/// assert_eq!( free_slots( Seq( 3 ), Seq( 0 ), cap ), 1 );
/// assert_eq!( free_slots( Seq( 4 ), Seq( 0 ), cap ), 0 );
/// ```
#[must_use]
pub fn free_slots(producer: Seq, consumer: Seq, capacity: Capacity) -> usize {
  let in_flight = consumer.distance_to(producer);
  (capacity.get() as u64).saturating_sub(in_flight) as usize
}

/// How many published items a consumer at `consumer` has not yet read, given a
/// producer that has published up to but not including `producer`.
///
/// ```
/// use ring_types::Seq;
/// use ring_seqno::pending;
///
/// assert_eq!( pending( Seq( 5 ), Seq( 2 ) ), 3 );
/// assert_eq!( pending( Seq( 2 ), Seq( 2 ) ), 0 );
/// ```
#[must_use]
pub fn pending(producer: Seq, consumer: Seq) -> u64 {
  consumer.distance_to(producer)
}

/// The lowest position across a gating set, or `None` for an empty set.
///
/// A producer is bounded by its slowest consumer, so the minimum is the only
/// reading that matters. Returning `None` rather than `Seq::ZERO` for an empty
/// set keeps "no consumers" distinguishable from "a consumer at the start".
/// The two call for opposite decisions, since an ungated ring may publish
/// freely.
///
/// ```
/// use ring_types::Seq;
/// use ring_seqno::slowest;
///
/// assert_eq!( slowest( &[ Seq( 9 ), Seq( 4 ), Seq( 7 ) ] ), Some( Seq( 4 ) ) );
/// assert_eq!( slowest( &[] ), None );
/// ```
#[must_use]
pub fn slowest(cursors: &[Seq]) -> Option<Seq> {
  cursors.iter().copied().min()
}
