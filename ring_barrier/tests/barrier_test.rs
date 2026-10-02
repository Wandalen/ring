//! Tests for `ring_barrier`, the consumer half of the sequence barrier.
//!
//! This file carries half the reached-test for
//! `docs/feature/178_sequence_barrier_and_gating_set.md`, stated in
//! `ring/bench_harness/docs/acceptance/001_feature_reached_tests.md` as:
//! `ring_barrier` returns the minimum across a gating set of 1, 2 and 3
//! cursors. (`ring_gating`'s own file carries the refuses-a-claim and
//! stalled-consumer halves.)
//!
//! ## Why 1, 2 and 3 are each written out
//!
//! The acceptance criterion names three set sizes rather than "several", and
//! the three are not interchangeable. A one-cursor set is passed by an
//! implementation that reads `cursors[0]` and never folds. A two-cursor set is
//! passed by a fold that is right for pairs and wrong for the general case. A
//! three-cursor set with the minimum in the middle is the first size where the
//! minimum is neither the first element nor the last, so it is the first size
//! where an implementation cannot get the right answer by accident.
//!
//! ## The asymmetry with `ring_gating` is asserted, not assumed
//!
//! An empty gating set means *unbounded* for a producer and *nothing readable*
//! for a consumer. The two crates give opposite answers to the same-shaped
//! question, which reads like an inconsistency until the reason is stated.
//! So `an_empty_barrier_and_an_empty_gating_set_answer_oppositely` states it as
//! an assertion rather than leaving it to the prose.
//!
//! ## Why `ring_gating` is a dev-dependency and not a dependency
//!
//! `ring_barrier` itself does not depend on `ring_gating`. A barrier is over a
//! slice of cursors from wherever they live. Three tests here still need a
//! real `GatingSet`, because what they assert is the relationship between the
//! two crates' answers: that the same cursors read from both sides
//! give opposite answers on an empty set, and differently-clamped answers on a
//! full one. Asserting that against a hand-rolled stand-in would be asserting
//! it against nothing.

// Ordinary tests, compiled out under `--cfg loom`. That cfg swaps
// `ring_atomic`'s atomics for loom's instrumented ones across the whole
// family, and those panic the moment they are touched outside a
// `loom::model` closure. Without this gate a family-wide loom run dies
// here instead of reaching the models in `ring_spsc`, `ring_mpsc`,
// `ring_publish` and `ring_testkit`.
#![cfg(not(loom))]

use core::sync::atomic::Ordering;

use ring_barrier::Barrier;
use ring_cursor::{PaddedCursor, SeqCell};
use ring_gating::GatingSet;
use ring_types::{Capacity, RingError, Seq, WaitKind};

fn cap(slots: usize) -> Capacity {
  Capacity::new(slots).expect("test capacities are powers of two")
}

/// Dependency cursors sitting at the given positions.
fn deps_at(positions: &[u64]) -> Vec<PaddedCursor> {
  positions.iter().map(|p| PaddedCursor::new(Seq(*p))).collect()
}

/// `count` dependency cursors, all at zero.
fn deps_zeroed(count: usize) -> Vec<PaddedCursor> {
  deps_at(&vec![0u64; count])
}

// ── feature 178: the minimum across 1, 2 and 3 cursors ─────────────────────

#[test]
fn the_frontier_of_one_cursor_is_that_cursor() {
  let deps = deps_at(&[7]);
  assert_eq!(Barrier::over(&deps).frontier(), Some(Seq(7)));
}

#[test]
fn the_frontier_of_two_cursors_is_the_lower() {
  // Both orders, so a fold that returns the last element rather than the
  // minimum fails one of them.
  assert_eq!(Barrier::over(&deps_at(&[3, 9])).frontier(), Some(Seq(3)));
  assert_eq!(Barrier::over(&deps_at(&[9, 3])).frontier(), Some(Seq(3)));
}

#[test]
fn the_frontier_of_three_cursors_is_the_lowest_wherever_it_sits() {
  // The minimum placed at each of the three positions in turn. The middle case
  // is the one an implementation cannot pass by reading an end.
  assert_eq!(Barrier::over(&deps_at(&[2, 8, 9])).frontier(), Some(Seq(2)));
  assert_eq!(Barrier::over(&deps_at(&[8, 2, 9])).frontier(), Some(Seq(2)));
  assert_eq!(Barrier::over(&deps_at(&[8, 9, 2])).frontier(), Some(Seq(2)));
}

#[test]
fn every_set_size_from_one_to_eight_folds_to_the_minimum() {
  // 1, 2 and 3 are what the criterion names; the fold should not stop working
  // at 4. The minimum is placed at every index of every size.
  for size in 1..=8usize {
    for minimum_at in 0..size {
      let mut positions = vec![100u64; size];
      positions[minimum_at] = 5;
      let deps = deps_at(&positions);

      assert_eq!(
        Barrier::over(&deps).frontier(),
        Some(Seq(5)),
        "size {size}, minimum at index {minimum_at}"
      );
    }
  }
}

#[test]
fn a_single_lagging_dependency_holds_the_whole_barrier() {
  let deps = deps_at(&[50, 50, 50]);
  let barrier = Barrier::over(&deps);
  assert_eq!(barrier.available(Seq::ZERO), 50);

  deps[1].store(Seq(4), Ordering::Release);
  assert_eq!(
    barrier.available(Seq::ZERO),
    4,
    "one dependency fell back and took the barrier with it"
  );
}

#[test]
fn the_frontier_follows_its_dependencies_as_they_advance() {
  let deps = deps_at(&[0, 0]);
  let barrier = Barrier::over(&deps);

  for step in 1..20u64 {
    deps[0].store(Seq(step), Ordering::Release);
    assert_eq!(
      barrier.frontier(),
      Some(Seq(step - 1)),
      "the other one is still one behind, and it is the minimum"
    );

    deps[1].store(Seq(step), Ordering::Release);
    assert_eq!(barrier.frontier(), Some(Seq(step)), "now both have arrived");
  }
}

// ── available ──────────────────────────────────────────────────────────────

#[test]
fn available_counts_the_sequences_between_here_and_the_frontier() {
  let deps = deps_at(&[10]);
  let barrier = Barrier::over(&deps);

  for from in 0..=10u64 {
    assert_eq!(barrier.available(Seq(from)), 10 - from, "from {from}");
  }
}

#[test]
fn available_is_zero_at_and_past_the_frontier() {
  // Past the frontier is a state a correct consumer never reaches, but the
  // arithmetic must saturate rather than wrap into a huge count. A consumer
  // acting on a wrapped count reads slots that were never published.
  let deps = deps_at(&[6]);
  let barrier = Barrier::over(&deps);

  for from in 6..40u64 {
    assert_eq!(barrier.available(Seq(from)), 0, "from {from}");
  }
}

#[test]
fn available_ignores_capacity_entirely() {
  // The distinction the module documentation argues, read off one set of
  // cursors from both sides at once. A gating answer is clamped by capacity, a
  // barrier answer is not. Same cursor, same position, two answers.
  let set = GatingSet::new(cap(4), 1);
  set.cursor(0).unwrap().store(Seq(1_000), Ordering::Release);

  assert_eq!(set.headroom(Seq(1_000)), 4, "the producer is clamped to one lap");
  assert_eq!(
    Barrier::over(set.cursors()).available(Seq::ZERO),
    1_000,
    "the consumer is not clamped at all"
  );
}

#[test]
fn admits_and_available_never_disagree() {
  let deps = deps_zeroed(2);
  let barrier = Barrier::over(&deps);

  for first in 0..12u64 {
    deps[0].store(Seq(first), Ordering::Release);
    for second in 0..12u64 {
      deps[1].store(Seq(second), Ordering::Release);
      for from in 0..12u64 {
        for count in 0..12u64 {
          assert_eq!(
            barrier.admits(Seq(from), count),
            count <= barrier.available(Seq(from)),
            "deps {first}/{second}, from {from}, count {count}"
          );
        }
      }
    }
  }
}

#[test]
fn a_request_for_zero_is_always_admitted() {
  let nothing_published = deps_at(&[0]);
  assert!(Barrier::over(&nothing_published).admits(Seq::ZERO, 0));

  assert!(Barrier::over(&[]).admits(Seq::ZERO, 0), "even with no dependencies at all");
}

// ── the empty barrier ──────────────────────────────────────────────────────

#[test]
fn an_empty_barrier_has_no_frontier_and_nothing_available() {
  let barrier = Barrier::over(&[]);

  assert!(barrier.is_empty());
  assert_eq!(barrier.len(), 0);
  assert_eq!(barrier.frontier(), None);
  for from in [0u64, 1, 8, 1_000] {
    assert_eq!(barrier.available(Seq(from)), 0, "from {from}");
  }
}

#[test]
fn an_empty_barrier_and_an_empty_gating_set_answer_oppositely() {
  // Stated as an assertion because it reads like an inconsistency. The same
  // empty set means "unbounded" to a producer and "nothing readable" to a
  // consumer. Both are "no constraint from dependencies" resolved to what a
  // dependency-free participant has.
  let empty = GatingSet::new(cap(8), 0);

  assert_eq!(empty.headroom(Seq(8)), 8, "a producer with nobody behind it may write");
  assert_eq!(
    Barrier::over(empty.cursors()).available(Seq::ZERO),
    0,
    "a consumer with nobody ahead of it may not read"
  );
}

#[test]
fn waiting_on_an_empty_barrier_fails_rather_than_hanging() {
  assert_eq!(
    Barrier::over(&[]).wait_for(Seq::ZERO, 1, WaitKind::None, 4),
    Err(RingError::Empty)
  );
}

#[test]
fn admits_a_zero_request_but_still_refuses_to_wait_for_it_when_empty() {
  // Stated as an assertion for the same reason as the gating-set asymmetry
  // above. On its own this reads like a bug. `admits` is a pure bool and
  // answers `true` for a zero-length request even with no dependencies at
  // all (`a_request_for_zero_is_always_admitted`), but `wait_for` promises a
  // frontier on success, and an empty barrier never has one to give,
  // however trivially satisfied the count was.
  let empty = Barrier::over(&[]);
  assert!(empty.admits(Seq::ZERO, 0), "a zero-length request is trivially satisfied");
  assert_eq!(
    empty.wait_for(Seq::ZERO, 0, WaitKind::None, 4),
    Err(RingError::Empty),
    "admitted, but there is still no frontier to report"
  );

  // Confirms the fallback that was *not* taken is not a missed generalization.
  // A non-empty barrier reports its true frontier at count == 0 too, never
  // `from`, so returning `from` for the empty case would be a different,
  // fabricated rule rather than the same one extended to an edge.
  let published = deps_at(&[100]);
  assert_eq!(
    Barrier::over(&published).wait_for(Seq::ZERO, 0, WaitKind::None, 4),
    Ok(Seq(100)),
    "count == 0 still reports the real frontier, not `from`"
  );
}

// ── waiting ────────────────────────────────────────────────────────────────

#[test]
fn wait_for_returns_the_frontier_and_not_the_requested_count() {
  // A consumer that waited for one and found six should drain six.
  let deps = deps_at(&[6]);
  assert_eq!(Barrier::over(&deps).wait_for(Seq::ZERO, 1, WaitKind::None, 1), Ok(Seq(6)));
}

#[test]
fn wait_for_gives_up_with_empty_when_the_budget_runs_out() {
  let deps = deps_at(&[2]);
  let barrier = Barrier::over(&deps);

  assert_eq!(barrier.wait_for(Seq::ZERO, 2, WaitKind::Spin, 8), Ok(Seq(2)));
  assert_eq!(barrier.wait_for(Seq::ZERO, 3, WaitKind::Spin, 8), Err(RingError::Empty));
}

#[test]
fn a_non_blocking_wait_looks_exactly_once() {
  let deps = deps_zeroed(1);
  let barrier = Barrier::over(&deps);

  let started = std::time::Instant::now();
  assert!(barrier.wait_for(Seq::ZERO, 1, WaitKind::None, usize::MAX).is_err());

  // The budget is `usize::MAX`; only the non-blocking contract stops this.
  // The bound is deliberately absurd rather than tight. A tight bound is a
  // flaky test, and anything under a second proves the loop did not run.
  assert!(started.elapsed() < std::time::Duration::from_secs(1));
}

#[test]
fn a_consumer_waiting_on_a_producer_thread_makes_progress() {
  const TOTAL: u64 = 512;
  let deps = deps_zeroed(1);
  let barrier = Barrier::over(&deps);

  std::thread::scope(|scope| {
    scope.spawn(|| {
      for published in 1..=TOTAL {
        deps[0].store(Seq(published), Ordering::Release);
      }
    });

    let mut position = Seq::ZERO;
    while position.0 < TOTAL {
      if let Ok(frontier) = barrier.wait_for(position, 1, WaitKind::Yield, 10_000) {
        assert!(frontier.0 <= TOTAL, "read past what was ever published");
        position = frontier;
      }
    }

    assert_eq!(position, Seq(TOTAL));
  });
}

#[test]
fn a_barrier_never_reports_a_frontier_a_dependency_has_not_reached() {
  // The safety property, under concurrency. Whatever the barrier reports, the
  // dependency was at least there. A `Relaxed` read could report a position
  // the consumer has no happens-before edge to.
  const TOTAL: u64 = 2_000;
  let deps = deps_zeroed(2);
  let barrier = Barrier::over(&deps);

  std::thread::scope(|scope| {
    scope.spawn(|| {
      for published in 1..=TOTAL {
        deps[0].store(Seq(published), Ordering::Release);
      }
    });

    for _ in 0..20_000 {
      let frontier = barrier.frontier().expect("two dependencies");
      assert_eq!(frontier, Seq::ZERO, "cursor 1 never moved, so it is always the minimum");
    }
  });
}

// ── the borrow ─────────────────────────────────────────────────────────────

#[test]
fn two_barriers_over_one_set_agree() {
  // A `Barrier` borrows rather than owning, so two views of one set cannot
  // drift. Were it to own a copy of the cursors, this would fail the moment
  // one dependency advanced.
  let deps = deps_at(&[4, 7]);
  let first = Barrier::over(&deps);
  let second = Barrier::over(&deps);

  assert_eq!(first.frontier(), second.frontier());

  deps[0].store(Seq(9), Ordering::Release);
  assert_eq!(first.frontier(), Some(Seq(7)));
  assert_eq!(second.frontier(), first.frontier());
}

#[test]
fn a_barrier_exposes_the_same_cursors_it_was_given() {
  let deps = deps_at(&[1, 2]);
  let barrier = Barrier::over(&deps);

  for (index, dependency) in deps.iter().enumerate() {
    assert!(
      core::ptr::eq(barrier.cursor(index).unwrap(), dependency),
      "the barrier handed out a different cursor than it was given"
    );
  }
  assert!(barrier.cursor(2).is_none());
  assert_eq!(barrier.dependencies().len(), 2);
}

#[test]
fn a_barrier_over_a_gating_set_reads_that_set_and_not_a_copy() {
  // This composition makes a chained consumer possible. The cursors a
  // producer gates on are the same objects a downstream barrier waits on.
  let set = GatingSet::new(cap(8), 2);
  let barrier = Barrier::over(set.cursors());

  set.cursor(0).unwrap().store(Seq(5), Ordering::Release);
  set.cursor(1).unwrap().store(Seq(3), Ordering::Release);

  assert_eq!(barrier.frontier(), set.slowest());
  assert_eq!(barrier.frontier(), Some(Seq(3)));
}
