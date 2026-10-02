//! Tests for `ring_seqno`'s comparison and span arithmetic over unfolded
//! sequences.
//!
//! Claims the never-wraps half of
//! `docs/feature/167_sequence_slot_index_and_power_of_two_capacity.md`. The
//! sequence is what a gate compares across laps, so every function here must
//! stay correct when the two positions are more than one capacity apart. That
//! is exactly the case a folded position cannot express, and is why the crate
//! exists separately from `ring_index`.
//!
//! Also exercises the lap-bug boundary that
//! `docs/feature/178_sequence_barrier_and_gating_set.md` names, at the
//! sequence-arithmetic level; the structural half is `ring_barrier`'s.

use ring_seqno::{free_slots, laps_between, may_claim, pending, slowest};
use ring_types::{Capacity, Seq};

fn cap(n: usize) -> Capacity {
  Capacity::new(n).expect("test capacities are powers of two")
}

/// A lap is a full capacity of publications, and the count is exact at the
/// boundary rather than off by one in either direction.
#[test]
fn laps_count_whole_capacities() {
  let c = cap(8);
  assert_eq!(laps_between(Seq(0), Seq(0), c), 0);
  assert_eq!(laps_between(Seq(0), Seq(7), c), 0);
  assert_eq!(laps_between(Seq(0), Seq(8), c), 1);
  assert_eq!(laps_between(Seq(0), Seq(15), c), 1);
  assert_eq!(laps_between(Seq(0), Seq(16), c), 2);
}

/// Laps are measured from wherever the pair starts, not from zero. That is the
/// case that matters, since a long-lived ring's cursors are never near zero.
#[test]
fn laps_are_relative_not_absolute() {
  let c = cap(8);
  assert_eq!(laps_between(Seq(1_000_000), Seq(1_000_008), c), 1);
  assert_eq!(laps_between(Seq(1_000_001), Seq(1_000_008), c), 0);
}

/// A backward pair reads zero rather than an enormous number. But that zero
/// is not an obviously-wrong value. It is the same reading `may_claim` treats
/// as "room to publish" (see `docs/decisions/002_saturating_rather_than_signed.md`),
/// so a caller that swapped its arguments here gets a plausible, permissive
/// answer, not a visibly broken one.
#[test]
fn laps_backward_read_zero() {
  assert_eq!(laps_between(Seq(100), Seq(4), cap(8)), 0);
}

/// The lap-bug boundary: a producer exactly one capacity ahead of its consumer
/// must not claim, because the next slot is the one the consumer is on.
#[test]
fn claim_boundary_is_exclusive_at_exactly_one_lap() {
  let c = cap(4);
  assert!(may_claim(Seq(0), Seq(0), c));
  assert!(may_claim(Seq(3), Seq(0), c));
  assert!(!may_claim(Seq(4), Seq(0), c), "one full lap ahead must be refused");
  assert!(!may_claim(Seq(5), Seq(0), c), "beyond a lap must be refused");
  // The consumer advancing by one frees exactly one slot.
  assert!(may_claim(Seq(4), Seq(1), c));
}

/// Free slots and the claim predicate agree at every position across two full
/// laps. This invariant stops the two readings drifting apart.
#[test]
fn free_slots_agrees_with_may_claim_across_two_laps() {
  let c = cap(8);
  for consumer in 0..8u64 {
    for producer in consumer..consumer + 20 {
      let free = free_slots(Seq(producer), Seq(consumer), c);
      let claimable = may_claim(Seq(producer), Seq(consumer), c);
      assert_eq!(
        free > 0,
        claimable,
        "producer {producer}, consumer {consumer}: free={free}, may_claim={claimable}"
      );
    }
  }
}

/// An empty ring has every slot free; a full one has none, and neither reading
/// goes negative or wraps.
#[test]
fn free_slots_spans_zero_to_capacity() {
  let c = cap(4);
  assert_eq!(free_slots(Seq(0), Seq(0), c), 4);
  assert_eq!(free_slots(Seq(1), Seq(0), c), 3);
  assert_eq!(free_slots(Seq(4), Seq(0), c), 0);
  assert_eq!(free_slots(Seq(100), Seq(0), c), 0, "saturates rather than wrapping");
}

/// Pending counts published-but-unread items and is zero when caught up.
#[test]
fn pending_counts_unread() {
  assert_eq!(pending(Seq(5), Seq(2)), 3);
  assert_eq!(pending(Seq(2), Seq(2)), 0);
  assert_eq!(pending(Seq(2), Seq(5)), 0, "a consumer cannot be ahead");
}

/// The slowest cursor bounds the producer, whichever position it holds in the
/// set. It is a minimum, not a first or last.
#[test]
fn slowest_is_the_minimum_wherever_it_sits() {
  assert_eq!(slowest(&[Seq(4)]), Some(Seq(4)));
  assert_eq!(slowest(&[Seq(4), Seq(9)]), Some(Seq(4)));
  assert_eq!(slowest(&[Seq(9), Seq(4)]), Some(Seq(4)));
  assert_eq!(slowest(&[Seq(9), Seq(4), Seq(7)]), Some(Seq(4)));
  assert_eq!(slowest(&[Seq(7), Seq(7)]), Some(Seq(7)));
}

/// No consumers is distinguishable from a consumer at the start, because the
/// two call for opposite decisions: an ungated ring may publish freely.
#[test]
fn slowest_of_nothing_is_none_not_zero() {
  assert_eq!(slowest(&[]), None);
  assert_eq!(slowest(&[Seq::ZERO]), Some(Seq::ZERO));
}

/// The whole point of not folding: two positions many laps apart still compare
/// correctly, where a folded pair would be indistinguishable.
#[test]
fn positions_many_laps_apart_stay_comparable() {
  let c = cap(8);
  let consumer = Seq(8);
  let producer = Seq(800);
  // Folded, both are slot 0. They are identical, and the gate would see "caught up".
  assert_eq!(producer.0 % 8, consumer.0 % 8);
  // Unfolded, the producer is 99 laps ahead and must be refused.
  assert_eq!(laps_between(consumer, producer, c), 99);
  assert!(!may_claim(producer, consumer, c));
  assert_eq!(free_slots(producer, consumer, c), 0);
}

/// SQ2: the one edge the boundary lattice left unpinned, `may_claim` against
/// `laps_between == 0`. It is swept across the same positions as
/// `free_slots_agrees_with_may_claim_across_two_laps`, in the argument order a
/// caller has to reverse to state it (`algorithm/001` SQ2).
#[test]
fn laps_between_zero_agrees_with_may_claim_across_two_laps() {
  let c = cap(8);
  for consumer in 0..8u64 {
    for producer in consumer..consumer + 20 {
      let claimable = may_claim(Seq(producer), Seq(consumer), c);
      let no_full_lap = laps_between(Seq(consumer), Seq(producer), c) == 0;
      assert_eq!(
        no_full_lap, claimable,
        "producer {producer}, consumer {consumer}: laps_between==0 is {no_full_lap}, may_claim={claimable}"
      );
    }
  }
}
