//! Tests for `ring_stats` — the counters a run is judged by.
//!
//! Claims `docs/feature/185_ring_stats.md`. Its acceptance criterion, filed at
//! `ring/bench_harness/docs/acceptance/001_feature_reached_tests.md`, is that a
//! drop is *counted*, not silently absorbed — a benchmark whose throughput number
//! came partly from discarded work is a wrong number, and the only way to tell
//! the two apart afterwards is a counter incremented at the moment of the drop.
//!
//! Every counter is `Relaxed`, which is deliberate and is the thing tested
//! hardest below: a statistics counter must never introduce ordering the
//! algorithm did not already need. So these tests assert only what `Relaxed`
//! actually guarantees — per-counter monotonicity, and exact totals once all
//! writers have joined — never a coherent cross-counter snapshot mid-run.

use ring_stats::RingStats;
use ring_types::OverflowPolicy;

/// A fresh set reads zero everywhere, so a run's numbers are its own.
#[test]
fn a_fresh_set_is_all_zero() {
  for stats in [RingStats::new(), RingStats::default()] {
    assert_eq!(stats.claimed(), 0);
    assert_eq!(stats.published(), 0);
    assert_eq!(stats.consumed(), 0);
    assert_eq!(stats.wait_nanos(), 0);
    assert_eq!(stats.dropped_total(), 0);
    assert_eq!(stats.in_flight(), 0);
    for policy in OverflowPolicy::ALL {
      assert_eq!(stats.dropped(policy), 0, "{policy:?}");
    }
  }
}

/// Each recorder moves exactly its own counter — the property that makes a
/// per-policy drop breakdown meaningful rather than a single opaque total.
#[test]
fn each_recorder_moves_exactly_one_counter() {
  let stats = RingStats::new();

  stats.record_claim(1);
  assert_eq!(stats.claimed(), 1);
  assert_eq!(stats.published(), 0);
  assert_eq!(stats.consumed(), 0);
  assert_eq!(stats.dropped_total(), 0);
  assert_eq!(stats.wait_nanos(), 0);

  stats.record_publish(1);
  assert_eq!(stats.published(), 1);
  assert_eq!(stats.claimed(), 1);
  assert_eq!(stats.consumed(), 0);

  stats.record_consume(1);
  assert_eq!(stats.consumed(), 1);
  assert_eq!(stats.published(), 1);

  stats.record_wait(700);
  assert_eq!(stats.wait_nanos(), 700);
  assert_eq!(stats.consumed(), 1);
  assert_eq!(stats.dropped_total(), 0);
}

/// A drop lands under the policy that caused it and under no other — the
/// distinction the crate exists to preserve, since a ring dropping newest and
/// one evicting oldest are in completely different trouble.
#[test]
fn a_drop_lands_under_its_own_policy_only() {
  for recorded in OverflowPolicy::ALL {
    let stats = RingStats::new();
    stats.record_drop(recorded, 3);

    for policy in OverflowPolicy::ALL {
      let expected = if policy == recorded { 3 } else { 0 };
      assert_eq!(stats.dropped(policy), expected, "recorded {recorded:?}, read {policy:?}");
    }
    assert_eq!(stats.dropped_total(), 3);
  }
}

/// The total is the sum over every policy, so a caller asking "how much work was
/// lost" and one asking "which policy lost it" read consistent numbers.
#[test]
fn the_drop_total_is_the_sum_over_every_policy() {
  let stats = RingStats::new();
  stats.record_drop(OverflowPolicy::DropNewest, 5);
  stats.record_drop(OverflowPolicy::DropOldest, 3);
  stats.record_drop(OverflowPolicy::Fail, 2);

  assert_eq!(stats.dropped(OverflowPolicy::DropNewest), 5);
  assert_eq!(stats.dropped(OverflowPolicy::DropOldest), 3);
  assert_eq!(stats.dropped(OverflowPolicy::Fail), 2);
  assert_eq!(stats.dropped_total(), 10);

  let summed: u64 = OverflowPolicy::ALL.iter().map(|p| stats.dropped(*p)).sum();
  assert_eq!(summed, stats.dropped_total());
}

/// `dropped_total` and `snapshot`'s `dropped_total` field must never wrap past
/// `u64::MAX` — the sum saturates instead, so an enormous real loss is never
/// misreported as a small or nonexistent one.
///
/// Root Cause: `dropped_total` folded the three per-policy counters with
/// `Iterator::sum`, and `snapshot`'s `dropped_total` field used plain `+` on
/// the same three values — both ordinary `u64` addition. `record_drop` takes
/// an unbounded `n : u64` with no upper-bound check, so two calls whose counts
/// add past `u64::MAX` (`u64::MAX` then `1`) made the sum panic in a debug
/// build and silently wrap to a small number in release — in this exact case,
/// to `0`, which a caller reads as "nothing was ever dropped".
///
/// Why Not Caught: every existing test recorded small literal counts (`5`,
/// `3`, `2`, or `1` repeated in a loop), and the crate's own concurrency tests
/// drive the counters from real per-thread loops bounded by ordinary test
/// sizes (tens of thousands), never from a single call passing a value near
/// `u64::MAX`. `record_drop`'s doc states no bound on `n`, so a single direct
/// call is enough — no sustained traffic or contention is needed to reach it.
///
/// Fix Applied: `dropped_total` now folds with `u64::saturating_add` instead
/// of `Iterator::sum`, and `snapshot`'s `dropped_total` field is built from
/// two chained `saturating_add` calls instead of `+`. Both now floor at
/// `u64::MAX` rather than panicking or wrapping.
///
/// Prevention: a diagnostic counter's whole purpose is staying readable under
/// conditions the counted system itself may never reach validly — it must
/// saturate rather than wrap or panic on its own derived readings, matching
/// `in_flight`'s already-established `saturating_sub` choice in this same
/// file.
///
/// Pitfall: an unbounded `record_*( n : u64 )` parameter puts every downstream
/// sum of that counter outside any bound the crate can assume — audit every
/// place counters written through such a method are later combined, not just
/// the individual `fetch_add` call sites.
#[test]
fn dropped_total_saturates_instead_of_overflowing() {
  let stats = RingStats::new();
  stats.record_drop(OverflowPolicy::DropNewest, u64::MAX);
  stats.record_drop(OverflowPolicy::DropOldest, 1);

  assert_eq!(stats.dropped_total(), u64::MAX, "the fold must saturate, not wrap past it");
  assert_eq!(stats.snapshot().dropped_total, u64::MAX, "snapshot's field must saturate too");
}

/// `Fail` is counted even though it loses nothing: how often a `Fail` ring
/// handed the decision back is its pressure signal, and an uncounted refusal
/// would make a saturated ring look idle.
#[test]
fn refusals_are_counted_even_though_nothing_is_lost() {
  let stats = RingStats::new();
  for _ in 0..4 {
    stats.record_drop(OverflowPolicy::Fail, 1);
  }

  assert_eq!(stats.dropped(OverflowPolicy::Fail), 4);
  assert_eq!(stats.dropped_total(), 4);
  assert_eq!(stats.published(), 0, "a refusal publishes nothing");
  assert_eq!(stats.consumed(), 0);
}

/// The acceptance criterion in its plainest form: a dropped item stays visible
/// afterwards. A run that published 93 and lost 7 cannot be mistaken for a run
/// that published 93 cleanly.
#[test]
fn a_drop_is_counted_not_absorbed() {
  let lossy = RingStats::new();
  lossy.record_publish(93);
  lossy.record_drop(OverflowPolicy::DropNewest, 7);

  let clean = RingStats::new();
  clean.record_publish(93);

  assert_eq!(
    lossy.published(),
    clean.published(),
    "throughput alone cannot tell them apart"
  );
  assert_ne!(lossy.dropped_total(), clean.dropped_total(), "the counter must");
  assert_eq!(lossy.dropped_total(), 7);
  assert_eq!(clean.dropped_total(), 0);
}

/// Recording a batch is one call, and equals the same count recorded singly —
/// so a batched publisher and an item-at-a-time one produce comparable numbers.
#[test]
fn batched_and_single_recording_agree() {
  let batched = RingStats::new();
  batched.record_publish(64);
  batched.record_drop(OverflowPolicy::DropOldest, 4);

  let singly = RingStats::new();
  for _ in 0..64 {
    singly.record_publish(1);
  }
  for _ in 0..4 {
    singly.record_drop(OverflowPolicy::DropOldest, 1);
  }

  assert_eq!(batched.published(), singly.published());
  assert_eq!(batched.dropped_total(), singly.dropped_total());
}

/// Recording zero is a no-op rather than an error, so a caller need not branch
/// around an empty batch.
#[test]
fn recording_zero_changes_nothing() {
  let stats = RingStats::new();
  stats.record_claim(0);
  stats.record_publish(0);
  stats.record_consume(0);
  stats.record_wait(0);
  for policy in OverflowPolicy::ALL {
    stats.record_drop(policy, 0);
  }

  assert_eq!(stats.claimed(), 0);
  assert_eq!(stats.published(), 0);
  assert_eq!(stats.dropped_total(), 0);
  assert_eq!(stats.in_flight(), 0);
}

/// In-flight is claimed minus published — a nonzero reading at rest means a
/// producer took a slot and abandoned it, which leaks ring capacity.
#[test]
fn in_flight_is_claimed_minus_published() {
  let stats = RingStats::new();
  stats.record_claim(10);
  assert_eq!(stats.in_flight(), 10, "ten slots taken, none published");

  stats.record_publish(4);
  assert_eq!(stats.in_flight(), 6);

  stats.record_publish(6);
  assert_eq!(stats.in_flight(), 0, "a balanced ring has nothing in flight");
}

/// In-flight floors at zero rather than wrapping. Publishing more than was
/// claimed is a caller bug, and `u64` subtraction would turn it into an
/// 18-quintillion-slot reading that looks like catastrophic leakage. The floor
/// is not only a backstop for that bug: on a busy, correct ring it also
/// absorbs the ordinary read-order race between the two loads, one to two
/// percent of the time — without it, a healthy ring would report the same
/// catastrophic reading this test builds by hand.
#[test]
fn in_flight_saturates_rather_than_wrapping() {
  let stats = RingStats::new();
  stats.record_claim(2);
  stats.record_publish(5);
  assert_eq!(stats.in_flight(), 0);

  let never_claimed = RingStats::new();
  never_claimed.record_publish(1);
  assert_eq!(never_claimed.in_flight(), 0);
}

/// Consuming does not affect in-flight: the reading is about the *claim*
/// handshake, not about how far a reader has got.
#[test]
fn consuming_does_not_affect_in_flight() {
  let stats = RingStats::new();
  stats.record_claim(8);
  stats.record_publish(8);
  stats.record_consume(8);
  assert_eq!(stats.in_flight(), 0);

  stats.record_claim(3);
  stats.record_consume(3);
  assert_eq!(stats.in_flight(), 3, "consumption cannot close an open claim");
}

/// Reset returns every counter to the fresh state, so a recycled ring does not
/// carry the previous world's numbers — the property `ring_shutdown`'s reset
/// depends on, per `docs/feature/184_close_reset_and_drain_all.md`.
#[test]
fn reset_returns_every_counter_to_the_fresh_state() {
  let stats = RingStats::new();
  stats.record_claim(9);
  stats.record_publish(8);
  stats.record_consume(7);
  stats.record_wait(6);
  for policy in OverflowPolicy::ALL {
    stats.record_drop(policy, 5);
  }

  assert_ne!(stats.claimed(), 0);
  assert_ne!(stats.dropped_total(), 0);

  stats.reset();

  assert_eq!(stats.claimed(), 0);
  assert_eq!(stats.published(), 0);
  assert_eq!(stats.consumed(), 0);
  assert_eq!(stats.wait_nanos(), 0);
  assert_eq!(stats.dropped_total(), 0);
  assert_eq!(stats.in_flight(), 0);
  for policy in OverflowPolicy::ALL {
    assert_eq!(stats.dropped(policy), 0, "{policy:?} survived the reset");
  }
}

/// A set stays usable after a reset — it is a rewind, not a poison.
#[test]
fn a_reset_set_counts_again() {
  let stats = RingStats::new();
  stats.record_publish(100);
  stats.reset();
  stats.record_publish(2);
  assert_eq!(stats.published(), 2);
}

/// Recording takes `&self`, so a shared reference suffices — the property that
/// lets every producer thread count without the set becoming a lock.
#[test]
fn recording_needs_only_a_shared_reference() {
  let stats = RingStats::new();
  let borrowed: &RingStats = &stats;
  borrowed.record_publish(1);
  borrowed.record_publish(1);
  assert_eq!(stats.published(), 2);
}

/// Totals are exact under real contention: four threads counting 25_000 items
/// each must read 100_000, no more and no less. `Relaxed` suffices — a
/// `fetch_add` is atomic regardless of ordering; ordering governs only what
/// *other* memory a reader may observe alongside it.
#[test]
fn counts_are_exact_under_contention() {
  const PRODUCERS: usize = 4;
  const PER_PRODUCER: usize = 25_000;

  let stats = RingStats::new();

  std::thread::scope(|scope| {
    for _ in 0..PRODUCERS {
      scope.spawn(|| {
        for _ in 0..PER_PRODUCER {
          stats.record_claim(1);
          stats.record_publish(1);
        }
      });
    }
  });

  assert_eq!(stats.claimed() as usize, PRODUCERS * PER_PRODUCER);
  assert_eq!(stats.published() as usize, PRODUCERS * PER_PRODUCER);
  assert_eq!(stats.in_flight(), 0, "every claim was published");
}

/// Distinct policy counters do not interfere under contention — a lost update
/// across counters would show up as a short total.
#[test]
fn distinct_policy_counters_do_not_interfere_under_contention() {
  const PER_THREAD: usize = 10_000;

  let stats = RingStats::new();
  // A shared reference, so each `move` closure copies the borrow rather than
  // taking the set itself — which is the whole point of `&self` recording.
  let stats = &stats;

  std::thread::scope(|scope| {
    for policy in OverflowPolicy::ALL {
      scope.spawn(move || {
        for _ in 0..PER_THREAD {
          stats.record_drop(policy, 1);
        }
      });
    }
  });

  for policy in OverflowPolicy::ALL {
    assert_eq!(stats.dropped(policy) as usize, PER_THREAD, "{policy:?}");
  }
  assert_eq!(stats.dropped_total() as usize, OverflowPolicy::ALL.len() * PER_THREAD);
}

/// Every counter is monotone while writers run: a reader sampling twice never
/// sees the second reading below the first. This is the strongest statement
/// `Relaxed` supports per counter — it covers a display of claimed, published
/// or dropped, but not the derived `in_flight` gauge, whose subtraction moves
/// in both directions under this same kind of concurrent sampling.
#[test]
fn each_counter_is_monotone_while_writers_run() {
  let stats = RingStats::new();

  std::thread::scope(|scope| {
    scope.spawn(|| {
      for _ in 0..50_000 {
        stats.record_publish(1);
      }
    });

    scope.spawn(|| {
      let mut last = 0u64;
      for _ in 0..1_000 {
        let now = stats.published();
        assert!(now >= last, "published went backwards: {last} then {now}");
        last = now;
      }
    });
  });

  assert_eq!(stats.published(), 50_000);
}

/// A snapshot's own numbers agree with each other, which separate reads cannot
/// promise.
///
/// `dropped_total()` and the three `dropped()` calls a reader would check beside
/// it are four independent loads at four moments, so under traffic the total
/// need not be the sum of the breakdown printed with it — measured at three to
/// five percent of breakdowns showing a spread the ring never held, the widest
/// running to 3,325 on counters kept within one of each other. `snapshot` loads
/// each counter once and derives from those same locals, so the relation holds
/// by construction. This test is what stops that derivation from quietly
/// becoming a re-read.
#[test]
fn a_snapshot_agrees_with_itself_while_writers_run() {
  const PER_THREAD: usize = 20_000;
  const READS: usize = 50_000;

  let stats = RingStats::new();
  let stats = &stats;

  std::thread::scope(|scope| {
    for policy in OverflowPolicy::ALL {
      scope.spawn(move || {
        for _ in 0..PER_THREAD {
          stats.record_drop(policy, 1);
        }
      });
    }

    scope.spawn(|| {
      for _ in 0..PER_THREAD {
        stats.record_claim(1);
        stats.record_publish(1);
      }
    });

    scope.spawn(|| {
      for _ in 0..READS {
        let counts = stats.snapshot();
        assert_eq!(
          counts.dropped_total,
          counts.dropped_newest + counts.dropped_oldest + counts.failed,
          "a snapshot's total disagreed with its own breakdown: {counts:?}"
        );
        assert_eq!(
          counts.in_flight,
          counts.claimed.saturating_sub(counts.published),
          "a snapshot's in_flight disagreed with its own two counters: {counts:?}"
        );
      }
    });
  });

  let settled = stats.snapshot();
  assert_eq!(settled.dropped_total as usize, 3 * PER_THREAD);
  assert_eq!(settled.claimed as usize, PER_THREAD);
  assert_eq!(settled.in_flight, 0);
}

/// The caller bug and a balanced ring are one reading through `in_flight` and
/// two through `checked_in_flight`.
///
/// `saturating_sub` floors `published > claimed` — which no correct caller
/// produces — onto zero, the value a healthy ring gives. `checked_sub` is one
/// word different and keeps them apart, which is the whole reason the checked
/// form exists.
#[test]
fn checked_in_flight_tells_a_caller_bug_from_a_balanced_ring() {
  let balanced = RingStats::new();
  balanced.record_claim(5);
  balanced.record_publish(5);

  let over_published = RingStats::new();
  over_published.record_claim(2);
  over_published.record_publish(5);

  let never_claimed = RingStats::new();
  never_claimed.record_publish(1);

  // Through the saturating reading all three are the same number.
  assert_eq!(balanced.in_flight(), 0);
  assert_eq!(over_published.in_flight(), 0);
  assert_eq!(never_claimed.in_flight(), 0);

  // Through the checked one, only the balanced set is a real zero.
  assert_eq!(balanced.snapshot().checked_in_flight(), Some(0));
  assert_eq!(over_published.snapshot().checked_in_flight(), None);
  assert_eq!(never_claimed.snapshot().checked_in_flight(), None);
}

/// Every counter is reached by a snapshot and cleared by a reset, on values
/// distinct enough that a field wired to the wrong counter shows up.
///
/// `RingStats::COUNTERS` and the compile-time size assertion beside it are what
/// stop an eighth counter being added to the struct and forgotten by `reset`.
/// This is the runtime half: that the seven which exist are each reached,
/// individually, by both paths. The final comparison is against the whole
/// default value rather than seven hand-written assertions, so a counter `reset`
/// misses cannot be missed here too.
#[test]
fn a_snapshot_reaches_every_counter_and_a_reset_clears_every_one() {
  let distinct = [11u64, 22, 33, 44, 55, 66, 77];
  assert_eq!(distinct.len(), RingStats::COUNTERS, "one distinct value per counter");

  let stats = RingStats::new();
  stats.record_claim(11);
  stats.record_publish(22);
  stats.record_consume(33);
  stats.record_drop(OverflowPolicy::DropNewest, 44);
  stats.record_drop(OverflowPolicy::DropOldest, 55);
  stats.record_drop(OverflowPolicy::Fail, 66);
  stats.record_wait(77);

  let counts = stats.snapshot();
  assert_eq!(counts.claimed, 11);
  assert_eq!(counts.published, 22);
  assert_eq!(counts.consumed, 33);
  assert_eq!(counts.dropped_newest, 44);
  assert_eq!(counts.dropped_oldest, 55);
  assert_eq!(counts.failed, 66);
  assert_eq!(counts.wait_nanos, 77);
  assert_eq!(counts.dropped_total, 44 + 55 + 66);
  assert_eq!(counts.in_flight, 0, "22 published against 11 claimed floors at zero");
  assert_eq!(counts.checked_in_flight(), None);

  stats.reset();
  assert_eq!(stats.snapshot(), Default::default(), "a reset leaves the fresh value");
}

/// A reader running beside a reset sees only values the writer wrote — the
/// crate's first look at the reset window from a second thread.
///
/// `reset` is `COUNTERS` separate stores, not one operation, so a reader can land
/// inside it and see some counters cleared and others not. That much is
/// documented and intended. What must never appear is a value neither `new`, the
/// fill, nor the reset put there. Both reset tests before this one ran on a
/// single thread, which is the one condition in which the window cannot appear
/// at all.
#[test]
fn a_reader_beside_a_reset_sees_only_values_the_writer_wrote() {
  const FILL: u64 = 9;
  const ROUNDS: usize = 20_000;

  let stats = RingStats::new();
  let stats = &stats;
  let running = std::sync::atomic::AtomicBool::new(true);
  let running = &running;

  std::thread::scope(|scope| {
    scope.spawn(move || {
      for _ in 0..ROUNDS {
        stats.record_claim(FILL);
        stats.record_wait(FILL);
        stats.reset();
      }
      running.store(false, std::sync::atomic::Ordering::Relaxed);
    });

    scope.spawn(move || {
      let mut caught_the_reset_window = false;
      while running.load(std::sync::atomic::Ordering::Relaxed) {
        let claimed = stats.claimed();
        let wait_nanos = stats.wait_nanos();

        assert!(claimed == 0 || claimed == FILL, "claimed read {claimed}, which nothing wrote");
        assert!(
          wait_nanos == 0 || wait_nanos == FILL,
          "wait_nanos read {wait_nanos}, which nothing wrote"
        );

        caught_the_reset_window |= claimed == 0 && wait_nanos == FILL;
      }

      // Deliberately not asserted. The window is real — 5 to 101 hits per two
      // million paired reads across six runs — but far too rare to require here
      // without making the suite flaky. What is asserted above is the property
      // that must hold on every read regardless of where it lands.
      let _ = caught_the_reset_window;
    });
  });

  assert_eq!(stats.snapshot(), Default::default(), "the writer's last act was a reset");
}
