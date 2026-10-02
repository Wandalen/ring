//! Tests for `ring_overflow`, which decides what a full ring does.
//!
//! Claims the handler half of
//! `docs/feature/174_overflow_policy_enum_and_handlers.md`; the enum half lives
//! in `ring_types` per `docs/decision/121_workstream_008_contract_gaps_ruled.md`
//! § 5, and is claimed by that crate's own tests.
//!
//! The acceptance criterion filed at
//! `ring/bench_harness/docs/acceptance/001_feature_reached_tests.md` is
//! negative: **exactly three discriminants, and no overwrite variant**. That
//! absence is what makes a successful publish mean something. Under every
//! policy here, a publish reporting success kept the item. A fourth variant that
//! overwrote unread data would silently break every consumer written against
//! the other three, so the exhaustiveness assertions below are the ones that
//! matter, not decoration.

use ring_overflow::{Resolution, resolve, would_resolve};
use ring_stats::RingStats;
use ring_types::{OverflowPolicy, RingError};

/// Each policy produces its own resolution, and the mapping is total.
#[test]
fn each_policy_maps_to_its_own_resolution() {
  assert_eq!(would_resolve(OverflowPolicy::DropNewest), Resolution::DroppedIncoming);
  assert_eq!(would_resolve(OverflowPolicy::DropOldest), Resolution::EvictedOldest);
  assert_eq!(would_resolve(OverflowPolicy::Fail), Resolution::Refused);
}

/// The mapping is injective: no two policies collapse onto one resolution, so
/// the resolution alone identifies what happened.
#[test]
fn distinct_policies_give_distinct_resolutions() {
  let mut seen = Vec::new();
  for policy in OverflowPolicy::ALL {
    let resolution = would_resolve(policy);
    assert!(!seen.contains(&resolution), "{policy:?} collided with an earlier policy");
    seen.push(resolution);
  }
  assert_eq!(seen.len(), 3);
}

/// The acceptance criterion, asserted structurally. Exhaustively matching three
/// named variants compiles, which it could not if a fourth existed. A test that
/// only listed three would still pass after someone added `Overwrite`.
///
/// The test also asserts against `Resolution::ALL`. The `match` alone catches
/// a fourth variant only here, in this one helper. It says nothing about the
/// count anywhere else, and a published `ALL` that had drifted out of step with
/// the enum would hand every iterating test a short list to loop over while this
/// one still compiled. Pinning both together is what makes `ALL` enforced
/// rather than decorative.
#[test]
fn resolution_has_exactly_three_variants_and_no_overwrite() {
  fn name(resolution: Resolution) -> &'static str {
    match resolution {
      Resolution::DroppedIncoming => "dropped_incoming",
      Resolution::EvictedOldest => "evicted_oldest",
      Resolution::Refused => "refused",
    }
  }

  assert_eq!(name(Resolution::DroppedIncoming), "dropped_incoming");
  assert_eq!(name(Resolution::EvictedOldest), "evicted_oldest");
  assert_eq!(name(Resolution::Refused), "refused");

  assert_eq!(Resolution::ALL.len(), 3, "ALL has drifted from the enum");
  for resolution in Resolution::ALL {
    assert!(
      !name(resolution).is_empty(),
      "{resolution:?} is in ALL but the exhaustive helper does not name it"
    );
  }
  let mut distinct = Vec::new();
  for resolution in Resolution::ALL {
    assert!(!distinct.contains(&resolution), "{resolution:?} appears twice in ALL");
    distinct.push(resolution);
  }
}

/// No resolution both keeps the incoming item and destroys unread data without
/// saying so. That is the invariant an overwrite variant would break. Evicting is
/// permitted, but only because it is *reported* as a loss.
#[test]
fn no_resolution_overwrites_unread_data_silently() {
  let mut checked = 0;
  for policy in OverflowPolicy::ALL {
    let resolution = would_resolve(policy);
    if resolution.accepted_incoming() {
      checked += 1;
      assert!(
        resolution.lost_an_item(),
        "{resolution:?} accepted an item into a full ring without reporting a loss"
      );
    }
  }

  // **The antecedent is asserted, not assumed.** `accepted_incoming()` is true
  // for `EvictedOldest` alone, which needs `OverflowPolicy::DropOldest`, a
  // policy no default build accepts. Dropping it from `OverflowPolicy::ALL` on
  // exactly those grounds is a plausible future edit. This loop would then
  // iterate over policies none of which enter the branch, and the crate's
  // central safety test would be green while checking nothing. An implication
  // whose antecedent never holds is satisfied by anything, so a test of one has
  // to prove the antecedent held.
  assert_eq!(
    checked, 1,
    "no policy in OverflowPolicy::ALL produces an accepting resolution — \
     this test proved nothing"
  );
}

/// The two readings partition the three outcomes exactly, so a caller can
/// branch on either without a fallthrough case.
#[test]
fn the_two_readings_partition_the_outcomes() {
  assert!(Resolution::DroppedIncoming.lost_an_item());
  assert!(Resolution::EvictedOldest.lost_an_item());
  assert!(!Resolution::Refused.lost_an_item());

  assert!(!Resolution::DroppedIncoming.accepted_incoming());
  assert!(Resolution::EvictedOldest.accepted_incoming());
  assert!(!Resolution::Refused.accepted_incoming());
}

/// A refusal loses nothing. The whole reason `Fail` exists is that the caller
/// keeps the item and decides for itself.
#[test]
fn a_refusal_loses_nothing() {
  assert!(!Resolution::Refused.lost_an_item());
  assert!(!Resolution::Refused.accepted_incoming());
}

/// `resolve` returns what `would_resolve` predicts, on the two policies that
/// keep going. So the pure form is a faithful preview and not a second,
/// drifting implementation.
#[test]
fn resolve_agrees_with_would_resolve() {
  let stats = RingStats::new();
  assert_eq!(
    resolve(OverflowPolicy::DropNewest, &stats),
    Ok(would_resolve(OverflowPolicy::DropNewest))
  );
  assert_eq!(
    resolve(OverflowPolicy::DropOldest, &stats),
    Ok(would_resolve(OverflowPolicy::DropOldest))
  );
}

/// `Fail` is the one policy that hands the decision back, and it does so as
/// `RingError::Full` rather than as a resolution the caller might ignore.
#[test]
fn fail_hands_the_decision_back_as_an_error() {
  let stats = RingStats::new();
  assert_eq!(resolve(OverflowPolicy::Fail, &stats), Err(RingError::Full));
  assert_eq!(would_resolve(OverflowPolicy::Fail), Resolution::Refused);
}

/// Exactly one counter moves per call, on every policy, so a stats read
/// accounts for every full-ring event, not only the lossy ones.
#[test]
fn exactly_one_counter_moves_per_call() {
  for policy in OverflowPolicy::ALL {
    let stats = RingStats::new();
    let _ = resolve(policy, &stats);

    assert_eq!(stats.dropped(policy), 1, "{policy:?} did not count its own event");
    assert_eq!(stats.dropped_total(), 1, "{policy:?} moved more than one counter");
    assert_eq!(stats.published(), 0, "a full-ring event publishes nothing");
    assert_eq!(stats.claimed(), 0);
    assert_eq!(stats.consumed(), 0);
  }
}

/// A refused publish is counted too. An uncounted refusal would make a
/// saturated `Fail` ring indistinguishable from an idle one.
#[test]
fn a_refusal_is_counted_even_though_it_loses_nothing() {
  let stats = RingStats::new();
  for _ in 0..4 {
    assert!(resolve(OverflowPolicy::Fail, &stats).is_err());
  }
  assert_eq!(stats.dropped(OverflowPolicy::Fail), 4);
  assert_eq!(stats.dropped_total(), 4);
}

/// `resolve` is not idempotent on its error path, and this test pins that
/// instead of leaving it to be discovered.
///
/// `resolve` increments the counter before it matches the policy, so `Err`
/// returns with shared state already changed. That goes against the usual
/// reading that an `Err` means nothing happened. A caller that treats `Full` as
/// retryable and calls again for the *same* arrival therefore records two
/// full-ring events for one. The behaviour is intended. What was missing was
/// anything stating it, so this test fails if someone "fixes" the ordering to
/// make `Err` effect-free and silently changes what every stats reader counts.
#[test]
fn retrying_a_refusal_counts_the_same_arrival_twice() {
  let stats = RingStats::new();
  let one_arrival_at_a_full_ring = OverflowPolicy::Fail;

  assert_eq!(resolve(one_arrival_at_a_full_ring, &stats), Err(RingError::Full));
  assert_eq!(stats.dropped(OverflowPolicy::Fail), 1);

  assert_eq!(resolve(one_arrival_at_a_full_ring, &stats), Err(RingError::Full));
  assert_eq!(
    stats.dropped(OverflowPolicy::Fail),
    2,
    "resolve became idempotent — every stats consumer's arithmetic just changed"
  );

  // `would_resolve` is the retry-safe half, and the reason the pairing exists.
  // Decide with it as often as you like, and record with `resolve` once.
  for _ in 0..8 {
    assert_eq!(would_resolve(one_arrival_at_a_full_ring), Resolution::Refused);
  }
  assert_eq!(stats.dropped(OverflowPolicy::Fail), 2);
}

/// Counts accumulate across calls and stay separated by policy, so a mixed run
/// reports which pressure it was under.
#[test]
fn counts_accumulate_and_stay_separated() {
  let stats = RingStats::new();
  for _ in 0..3 {
    let _ = resolve(OverflowPolicy::DropNewest, &stats);
  }
  for _ in 0..2 {
    let _ = resolve(OverflowPolicy::DropOldest, &stats);
  }
  let _ = resolve(OverflowPolicy::Fail, &stats);

  assert_eq!(stats.dropped(OverflowPolicy::DropNewest), 3);
  assert_eq!(stats.dropped(OverflowPolicy::DropOldest), 2);
  assert_eq!(stats.dropped(OverflowPolicy::Fail), 1);
  assert_eq!(stats.dropped_total(), 6);
}

/// `would_resolve` touches no counters. That makes it safe for a factory
/// validating a configuration, which must not fabricate pressure the ring
/// never experienced.
#[test]
fn would_resolve_touches_no_counters() {
  let stats = RingStats::new();
  for policy in OverflowPolicy::ALL {
    let _ = would_resolve(policy);
  }
  assert_eq!(stats.dropped_total(), 0);
}

/// The resolution is a plain value that is `Copy`, comparable and hashable, so
/// a caller can tabulate outcomes without cloning or borrowing.
#[test]
fn a_resolution_is_a_plain_comparable_value() {
  let a = Resolution::EvictedOldest;
  let b = a;
  assert_eq!(a, b);
  assert_ne!(a, Resolution::Refused);

  let mut counts = std::collections::HashMap::new();
  for policy in OverflowPolicy::ALL {
    *counts.entry(would_resolve(policy)).or_insert(0) += 1;
  }
  assert_eq!(counts.len(), 3, "three policies must key three distinct buckets");
}

/// The policy's own self-description agrees with what the handler does, so the
/// two crates cannot drift apart on which policies report failure.
#[test]
fn policy_self_description_agrees_with_the_handler() {
  let stats = RingStats::new();
  for policy in OverflowPolicy::ALL {
    let outcome = resolve(policy, &stats);
    assert_eq!(
      outcome.is_err(),
      policy.reports_failure(),
      "{policy:?} disagrees with its own reports_failure()"
    );
    assert_eq!(
      outcome.is_ok_and(Resolution::lost_an_item),
      policy.drops_silently(),
      "{policy:?} disagrees with its own drops_silently()"
    );
  }
}
