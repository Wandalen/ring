//! Sequence-to-slot index mapping for power-of-two capacities.
//!
//! Tier 1 of the ring family's 33 crates — the concurrency write-path implementation.
//! Depends on `ring_types`.
//!
//! This is the half of `docs/feature/167_sequence_slot_index_and_power_of_two_capacity.md`
//! that folds: given a [`ring_types::Seq`] and a [`ring_types::Capacity`], produce
//! the [`ring_types::SlotIndex`] it addresses. The feature's whole reason for
//! constraining capacity to a power of two is that this fold is then a bitmask
//! rather than a division — an integer `%` costs on the order of 20–40 cycles on
//! current x86, and it sits on every operation that touches a slot. The claim
//! path is not one of them: `ring_claim`, `ring_publish`, `ring_consume`, and
//! `ring_cursor` work in sequence space end to end and never fold.
//!
//! The "20–40 cycles" figure above is asserted for x86, not measured in this
//! repository: on this crate's own build host the fold costs roughly 0.9
//! cycles against a runtime-divisor `%`'s 6.1–6.2, a 7x ratio rather than the
//! 22–44x the x86 figure implies. A same-literal benchmark (`seq % 1024`
//! beside `seq & 1023`) would also mislead: the family's capacity is never a
//! compile-time literal at a real fold site, so the runtime-divisor figure is
//! the one that applies, not the free constant-divisor one.
//!
//! The validation lives upstream in [`ring_types::Capacity`], not here. Because
//! a `Capacity` cannot exist unless it is a power of two, [`of`] needs no check
//! and no error path: it is total.

#![deny(missing_docs)]

use ring_types::{Capacity, Seq, SlotIndex};

/// The slot a sequence addresses.
///
/// Equal to `seq % capacity` for every input, computed as `seq & (capacity - 1)`.
/// Two sequences a whole number of laps apart return the same slot, which is
/// the aliasing the gating machinery exists to prevent from being *observed*.
///
/// ```
/// use ring_types::{ Capacity, Seq, SlotIndex };
/// use ring_index::of;
///
/// let cap = Capacity::new( 8 ).unwrap();
/// assert_eq!( of( Seq( 0 ), cap ), SlotIndex( 0 ) );
/// assert_eq!( of( Seq( 7 ), cap ), SlotIndex( 7 ) );
/// assert_eq!( of( Seq( 8 ), cap ), SlotIndex( 0 ) );   // one lap on
/// assert_eq!( of( Seq( 13 ), cap ), SlotIndex( 5 ) );
/// ```
#[must_use]
pub fn of(seq: Seq, capacity: Capacity) -> SlotIndex {
  SlotIndex((seq.0 as usize) & capacity.mask())
}

/// Whether two sequences address the same slot — true exactly when they are a
/// whole number of laps apart.
///
/// Aliasing is the ring's defining behaviour rather than a defect: a fixed
/// number of slots reused forever is what the structure *is*. What keeps it
/// from being observable is the gate — `ring_seqno`'s `may_claim`, and the
/// barrier built on it — which refuses a claim that would land on a slot a
/// consumer has not yet passed. This function reports the aliasing so a gate
/// can be tested against it; it does not prevent anything itself.
///
/// ```
/// use ring_types::{ Capacity, Seq };
/// use ring_index::aliases;
///
/// let cap = Capacity::new( 4 ).unwrap();
/// assert!( aliases( Seq( 1 ), Seq( 5 ), cap ) );
/// assert!( aliases( Seq( 1 ), Seq( 9 ), cap ) );
/// assert!( !aliases( Seq( 1 ), Seq( 2 ), cap ) );
/// ```
#[must_use]
pub fn aliases(a: Seq, b: Seq, capacity: Capacity) -> bool {
  of(a, capacity) == of(b, capacity)
}

/// The slots a contiguous run of `count` sequences starting at `start`
/// addresses, in order.
///
/// A batch claim is contiguous in sequence space, so its slots wrap at most
/// once — which is what preserves a staged buffer's relative order when it
/// lands, per `docs/feature/177_batch_claim_and_batch_drain.md`.
///
/// That guarantee belongs to the caller's batch claim, not to this function:
/// `run` neither clamps nor deduplicates, so a `count` larger than `capacity`
/// returns repeated slots rather than stopping at one wrap.
///
/// `run` also allocates once per call — exactly sized with no growth slack,
/// but an allocation regardless. That disqualifies it from the family's
/// lock-free claim and consume paths at any speed; `of` and `aliases`
/// measure zero allocations over 10,000 calls each and are what those paths
/// use instead.
///
/// At the sizes a real batch claim uses — `ring_config`'s default batch is 1,
/// and the family's own doctests run 2 to 8 — the returned `Vec`'s 24-byte
/// header outweighs its payload: one slot costs 24 bytes of bookkeeping to
/// carry 8 bytes of data.
///
/// # Panics
///
/// In a debug build, if `start.0 + ( count - 1 ) as u64` overflows `u64` for
/// `count > 0` — the last step `run` actually takes, reachable in one step
/// from `ring_mpsc::UNSTAMPED`, the family's `Seq( u64::MAX )` sentinel,
/// rather than only after `2^64` publications. In a release build the
/// addition wraps instead of panicking, which silently produces a slot index
/// for a sequence that never occurred.
///
/// ```
/// use ring_types::{ Capacity, Seq, SlotIndex };
/// use ring_index::run;
///
/// let cap = Capacity::new( 4 ).unwrap();
/// assert_eq!( run( Seq( 2 ), 4, cap ), vec![ SlotIndex( 2 ), SlotIndex( 3 ), SlotIndex( 0 ), SlotIndex( 1 ) ] );
/// assert_eq!( run( Seq( 0 ), 0, cap ), vec![] );
/// ```
#[must_use]
pub fn run(start: Seq, count: usize, capacity: Capacity) -> Vec<SlotIndex> {
  (0..count as u64).map(|n| of(start.advanced_by(n), capacity)).collect()
}
