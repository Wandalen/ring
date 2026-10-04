//! `ring_gating` tests for the producer half of the sequence barrier.
//!
//! This file carries half the reached-test for
//! `docs/feature/178_sequence_barrier_and_gating_set.md`. The half asserted
//! here is that `ring_gating` refuses a claim that would advance past the
//! gating minimum, and a producer never overwrites an uncommitted slot, over a
//! full lap with a deliberately stalled consumer. (`ring_barrier`'s own file
//! carries the minimum-across-a-set half.)
//!
//! ## The stalled consumer is the test, not the setup
//!
//! A gate that is never under pressure is a gate that never gates. A `headroom`
//! that returns `capacity` unconditionally passes every sequential test below,
//! because an empty ring has room for anything and most tests start with an
//! empty ring. `a_stalled_consumer_stops_the_producer_at_exactly_one_lap`
//! is the one it cannot pass. That test holds one consumer still while the
//! producer runs a full lap, and asserts the producer stops on the boundary
//! rather than one slot past it.
//!
//! ## Why the off-by-one gets its own tests
//!
//! The lap boundary is exclusive. At a distance of exactly `capacity` the next
//! claim lands on the slot the slowest consumer is currently reading. The tests
//! assert both adjacent cases explicitly, in both directions, because an
//! inclusive boundary passes every "the ring fills up" test and corrupts
//! exactly one slot per lap under load. That failure is the hardest to
//! reproduce and the easiest to write.

// Ordinary tests, compiled out under `--cfg loom`. That cfg swaps
// `ring_atomic`'s atomics for loom's instrumented ones across the whole
// family, and those panic the moment they are touched outside a
// `loom::model` closure. Without this gate a family-wide loom run dies
// here instead of reaching the models in `ring_spsc`, `ring_mpsc`,
// `ring_publish` and `ring_testkit`.
#![cfg(not(loom))]

use core::sync::atomic::Ordering;

use ring_cursor::SeqCell;
use ring_gating::GatingSet;
use ring_types::{Capacity, RingError, Seq};

fn cap(slots: usize) -> Capacity {
  Capacity::new(slots).expect("test capacities are powers of two")
}

/// A set whose consumers sit at the given positions.
fn set_at(capacity: usize, positions: &[u64]) -> GatingSet {
  let set = GatingSet::new(cap(capacity), positions.len());
  for (cursor, position) in set.cursors().iter().zip(positions) {
    cursor.store(Seq(*position), Ordering::Release);
  }
  set
}

// ── refusing a claim past the minimum ──────────────────────────────────────

#[test]
fn a_stalled_consumer_stops_the_producer_at_exactly_one_lap() {
  // The clause the acceptance criterion names: a full lap with a deliberately
  // stalled consumer. Two consumers, one of which never moves.
  const CAPACITY: usize = 8;
  let set = set_at(CAPACITY, &[0, 0]);

  let mut producer = Seq::ZERO;
  let mut admitted = 0;

  // Let the fast consumer race ahead. It must buy the producer nothing.
  set.cursor(1).unwrap().store(Seq(1_000), Ordering::Release);

  while set.admits(producer, 1) {
    producer = producer.next();
    admitted += 1;
    assert!(admitted <= CAPACITY, "the gate let the producer past a full lap");
  }

  assert_eq!(admitted, CAPACITY, "one lap of room and not one slot more");
  assert_eq!(producer, Seq(CAPACITY as u64));
  assert_eq!(set.check(producer, 1), Err(RingError::Full));
}

#[test]
fn releasing_one_slot_admits_exactly_one_more_claim() {
  let set = set_at(4, &[0]);
  let producer = Seq(4);

  assert!(!set.admits(producer, 1), "full");

  set.cursor(0).unwrap().store(Seq(1), Ordering::Release);
  assert!(set.admits(producer, 1));
  assert!(!set.admits(producer, 2), "one slot released is one slot, not two");
}

#[test]
fn the_lap_boundary_is_exclusive_on_both_sides() {
  let set = set_at(4, &[0]);

  assert!(set.admits(Seq(3), 1), "3 ahead of 4 slots: the last slot is free");
  assert!(
    !set.admits(Seq(4), 1),
    "4 ahead of 4 slots: the next claim would land on the consumer"
  );
  assert_eq!(set.headroom(Seq(3)), 1);
  assert_eq!(set.headroom(Seq(4)), 0);
}

#[test]
fn a_producer_never_passes_the_limit_over_a_full_lap_with_batches() {
  // The same property as the stall test, driven with multi-slot claims, so a
  // gate that is correct one-at-a-time and wrong in batch is caught.
  const CAPACITY: usize = 16;
  let set = set_at(CAPACITY, &[0]);
  let limit = set.limit().expect("one consumer, so a limit exists");

  let mut producer = Seq::ZERO;
  for batch in [5usize, 5, 5, 5, 5] {
    if set.check(producer, batch).is_ok() {
      producer = producer.advanced_by(batch as u64);
    }
    assert!(producer.0 <= limit.0, "producer at {producer:?} passed the limit {limit:?}");
  }

  assert_eq!(producer, Seq(15), "three batches of five fit; the fourth does not");
}

#[test]
fn the_slowest_consumer_sets_the_bound_regardless_of_position_in_the_set() {
  // The minimum must not depend on which index the slow consumer occupies.
  // An implementation reading `cursors[0]` passes half these cases.
  for slow_index in 0..3 {
    let mut positions = [100u64; 3];
    positions[slow_index] = 2;
    let set = set_at(8, &positions);

    assert_eq!(set.slowest(), Some(Seq(2)), "slow consumer at index {slow_index}");
    assert_eq!(set.headroom(Seq(8)), 2);
  }
}

#[test]
fn one_stalled_consumer_stops_the_producer_for_everyone() {
  let all_moving = set_at(8, &[8, 8, 8]);
  let one_stalled = set_at(8, &[8, 0, 8]);

  assert_eq!(all_moving.headroom(Seq(8)), 8);
  assert_eq!(one_stalled.headroom(Seq(8)), 0, "two fast consumers buy nothing");
}

// ── headroom ───────────────────────────────────────────────────────────────

#[test]
fn headroom_falls_by_one_per_slot_published() {
  let set = set_at(8, &[0]);

  for published in 0..=8u64 {
    assert_eq!(
      set.headroom(Seq(published)),
      8 - published as usize,
      "after publishing {published} of 8"
    );
  }
}

#[test]
fn headroom_never_goes_negative_or_wraps() {
  // `free_slots` saturates. A producer that appears further ahead than a lap
  // is a state the gate excludes, but the arithmetic must not underflow into
  // a huge headroom if it ever happens.
  let set = set_at(4, &[0]);

  for producer in 4..64u64 {
    assert_eq!(
      set.headroom(Seq(producer)),
      0,
      "producer {producer} against a stalled consumer"
    );
  }
}

#[test]
fn an_ungated_ring_has_a_full_capacity_of_headroom() {
  // The module documentation argues this distinction. An empty gating set is
  // not a consumer at zero. Were it one, this would read 0 after a lap and
  // every ungated ring would deadlock.
  let ungated = GatingSet::new(cap(4), 0);

  assert_eq!(ungated.slowest(), None);
  assert_eq!(ungated.limit(), None);
  for producer in [0u64, 4, 1_000, u32::MAX as u64] {
    assert_eq!(ungated.headroom(Seq(producer)), 4, "at producer {producer}");
    assert!(ungated.admits(Seq(producer), 4));
  }
}

#[test]
fn an_ungated_ring_and_a_consumer_at_zero_disagree_after_one_lap() {
  // The two states that a `Seq::ZERO`-for-empty implementation would conflate,
  // shown disagreeing.
  let ungated = GatingSet::new(cap(4), 0);
  let gated_at_zero = set_at(4, &[0]);

  assert_eq!(
    ungated.headroom(Seq::ZERO),
    gated_at_zero.headroom(Seq::ZERO),
    "same while empty"
  );
  assert_ne!(
    ungated.headroom(Seq(4)),
    gated_at_zero.headroom(Seq(4)),
    "and different after a lap"
  );
}

// ── check: back-pressure versus configuration error ────────────────────────

#[test]
fn a_claim_wider_than_the_ring_is_a_configuration_error() {
  let set = set_at(4, &[0]);
  let err = set.check(Seq::ZERO, 5).unwrap_err();

  assert_eq!(
    err,
    RingError::BatchTooLarge {
      requested: 5,
      capacity: 4
    }
  );
  assert!(err.is_configuration(), "no consumer's progress can ever make this fit");
}

#[test]
fn a_claim_that_merely_does_not_fit_yet_is_back_pressure() {
  let set = set_at(4, &[0]);

  assert_eq!(set.check(Seq(3), 2), Err(RingError::Full));
  assert!(
    !RingError::Full.is_configuration(),
    "a retry loop must keep going on this one"
  );
}

#[test]
fn the_two_failures_are_distinguished_at_the_boundary() {
  // A claim of exactly `capacity` on a full ring is `Full`, not `BatchTooLarge`,
  // because it would fit if the consumer caught up. Getting this backwards makes
  // a legitimate retry loop give up.
  let set = set_at(4, &[0]);

  assert_eq!(set.check(Seq(4), 4), Err(RingError::Full));
  assert!(set.check(Seq(4), 5).unwrap_err().is_configuration());
}

#[test]
fn check_and_admits_agree_across_the_whole_state_space() {
  // Two readings of one state. `check` adds the reason; it must not add a
  // different answer.
  let set = GatingSet::new(cap(8), 1);

  for consumer in 0..16u64 {
    set.cursor(0).unwrap().store(Seq(consumer), Ordering::Release);

    for producer in consumer..consumer + 12 {
      for count in 0..10usize {
        assert_eq!(
          set.check(Seq(producer), count).is_ok(),
          set.admits(Seq(producer), count),
          "consumer {consumer}, producer {producer}, count {count}"
        );
      }
    }
  }
}

#[test]
fn a_claim_of_zero_is_always_admitted() {
  // Not a special case in the implementation, and asserted so it does not
  // become one. A batch loop that computes a zero-length claim must not see
  // `Full` and conclude the ring is under pressure.
  let full = set_at(4, &[0]);

  assert!(full.check(Seq(4), 0).is_ok());
  assert!(full.admits(Seq(4), 0));
}

// ── the limit, as a diagnostic ─────────────────────────────────────────────

#[test]
fn the_limit_is_one_lap_past_the_slowest_consumer() {
  let set = set_at(8, &[5, 9, 12]);

  assert_eq!(set.slowest(), Some(Seq(5)));
  assert_eq!(set.limit(), Some(Seq(13)));
}

#[test]
fn the_limit_is_exactly_where_headroom_reaches_zero() {
  // The two readings must describe the same boundary. A diagnostic that
  // reported a different blocking point than the gate enforces is worse than
  // no diagnostic.
  let set = set_at(8, &[3]);
  let limit = set.limit().unwrap();

  assert_eq!(set.headroom(Seq(limit.0 - 1)), 1, "one slot left just below the limit");
  assert_eq!(set.headroom(limit), 0, "none at it");
}

// ── the set itself ─────────────────────────────────────────────────────────

#[test]
fn a_set_owns_one_cursor_per_consumer() {
  for consumers in 0..5 {
    let set = GatingSet::new(cap(4), consumers);

    assert_eq!(set.len(), consumers);
    assert_eq!(set.cursors().len(), consumers);
    assert_eq!(set.is_empty(), consumers == 0);
    assert!(set.cursor(consumers).is_none(), "no cursor past the end");
  }
}

#[test]
fn every_cursor_starts_at_zero() {
  let set = GatingSet::new(cap(4), 3);

  for cursor in set.cursors() {
    assert_eq!(cursor.load(Ordering::Acquire), Seq::ZERO);
  }
  assert_eq!(set.slowest(), Some(Seq::ZERO));
}

#[test]
fn the_cursors_in_a_set_are_cache_line_separated() {
  // A `GatingSet` is where several consumers' cursors are most likely to end
  // up adjacent, so it is where false sharing would bite. The `Vec` gives each
  // element a full stride because `PaddedCursor` is a whole line.
  let set = GatingSet::new(cap(4), 4);

  for window in set.cursors().windows(2) {
    assert_eq!(window[1].addr() - window[0].addr(), 128);
  }
}

#[test]
fn the_set_remembers_its_capacity() {
  assert_eq!(GatingSet::new(cap(32), 1).capacity().get(), 32);
}

/// Root Cause: The original assertion `headroom <= CAPACITY` is true by
/// construction for every possible return value of `headroom`. Both the
/// empty-set arm (`map_or`'s default, `self.capacity.get()`) and the
/// `free_slots` arm (`saturating_sub` against `capacity`) are bounded by
/// `capacity` regardless of what the consumer thread does. The race was real;
/// the assertion could not fail.
///
/// Why Not Caught: The test's name and its own doc comment state the real
/// property ("must never report more room than existed"), so it reads as
/// covering it. A tautological assertion passes exactly as loudly whether the
/// implementation is correct or badly broken. An inclusive-boundary bug that
/// reports `CAPACITY` itself still satisfies `<=`, and a swapped argument or a
/// `Relaxed` read that reports a stale-but-still-in-bounds value passes too.
///
/// Fix Applied: Added the two assertions that close the gap. (1) `<=` tightened
/// to `<`. This producer never reaches `CAPACITY`, so the inclusive-boundary
/// bug is now caught deterministically. (2) A property assertion comparing the
/// gate's reading against a cursor load taken immediately after it. Consumers
/// only advance, so `headroom` must never exceed the position the cursor has
/// reached by the time it is re-read. This is the over-report check the name
/// promised.
///
/// Prevention: When a concurrent test's only assertion is a static bound
/// derivable from the callee's own signature (a `usize` capped by the
/// `usize` capacity it was given), it proves compilation and
/// absence-of-panic, not correctness. Before trusting a concurrent
/// assertion, check whether any plausible broken implementation would still
/// pass it.
///
/// Pitfall: A test whose name and comment describe the right property can
/// still assert a strictly weaker one. The mismatch is invisible until
/// someone asks what the assertion would need to look like to fail.
#[test]
fn a_gate_read_concurrently_with_a_consumer_never_over_reports_room() {
  // A producer reads the gate while consumers advance. It may under-report
  // headroom, because a consumer that moved after the read is not yet seen.
  // But it must never report more room than existed at the moment it looked,
  // because that is the reading a producer overwrites a live slot on.
  const CAPACITY: usize = 64;
  let set = GatingSet::new(cap(CAPACITY), 1);
  let producer = Seq(CAPACITY as u64);

  std::thread::scope(|scope| {
    scope.spawn(|| {
      for position in 0..CAPACITY as u64 {
        set.cursor(0).unwrap().store(Seq(position), Ordering::Release);
      }
    });

    for _ in 0..10_000 {
      let headroom = set.headroom(producer);
      assert!(headroom < CAPACITY, "reported {headroom} slots of a {CAPACITY}-slot ring");

      let after = set.cursor(0).unwrap().load(Ordering::Acquire);
      assert!(
        headroom as u64 <= after.0,
        "gate saw {headroom} slots free, but the cursor is only at {}",
        after.0
      );
    }
  });
}

/// The manual check M2 in `tests/manual/readme.md` is the only thing that would
/// notice a future direct cursor read naming an ordering here. Being manual,
/// it only fires if someone remembers to run it. This automates M2's
/// own recipe (`grep -vE "^[[:space:]]*(///|//!)" src/lib.rs | grep -oE
/// "Ordering::[A-Za-z]+|GATING|SeqCell"`) as a `#[ test ]`, so the same
/// omission fails `cargo test` instead of waiting on a human to grep for it.
/// This test excludes doc-comment lines exactly as M2 excludes them. Five
/// doctest lines legitimately write `Ordering::Release` to drive a cursor, and
/// a scan that counted those would reject correct code.
#[test]
fn crate_names_no_ordering_in_any_non_doc_line() {
  let source = include_str!("../src/lib.rs");
  let code: String = source
    .lines()
    .filter(|line| {
      let trimmed = line.trim_start();
      !(trimmed.starts_with("///") || trimmed.starts_with("//!"))
    })
    .collect::<Vec<_>>()
    .join("\n");

  let hits: Vec<&str> = ["Ordering::", "GATING", "SeqCell"]
    .into_iter()
    .filter(|needle| code.contains(*needle))
    .collect();

  assert!(
    hits.is_empty(),
    "ring_gating names an ordering or the family's gating-ordering constant in \
     a non-doc line: {hits:?}. See the crate's \"names no memory ordering\" invariant in src/lib.rs"
  );
}
