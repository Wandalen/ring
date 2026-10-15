//! Behavioural suite for `ring_bench`.
//!
//! Claims `docs/feature/186_ring_benchmark_harness.md`, which asks for one
//! workload run against every candidate write path, with the comparison as the
//! deliverable. This suite also claims the `ring_stats` counters
//! (`docs/feature/185_ring_stats.md`), in their diagnosis role. They
//! turn a result into something readable rather than a single number.
//!
//! # What this suite deliberately never asserts
//!
//! **Which candidate is fastest.** An assertion about wall-clock ordering is a
//! flaky test on any machine with other processes on it, and a flaky test
//! inside a benchmark harness discredits the measurement the harness exists to
//! produce. The suite asserts record accounting instead: offered, reported,
//! received, dropped. Those counts are deterministic, and they are what makes a
//! timing number mean anything.
//!
//! **That policy has a price, and it was measured rather than estimated.**
//! `bench_harness/gate/mutant_survey.sh` broke this crate 180 ways and three
//! mutations survived the whole suite. All three are timing-only, and none can
//! be defended without breaking the rule above:
//!
//! - `write_nanos()` can return a constant. Its own doc comment says never to
//!   assert on it, so nothing does, so nothing notices.
//! - `run_mutex_queue`'s batch trigger can flip from `==` to `!=`, which makes
//!   the candidate commit one record at a time instead of in batches. Every
//!   count survives it exactly: the same records are offered, reported,
//!   received and dropped, on both the roomy and the cramped fixture. Batching
//!   changes how long the work takes and nothing else. Only a clock separates
//!   the two, and this suite refuses to use one.
//!
//! These are recorded as accepted, not as work outstanding. The cost is real.
//! A candidate could silently stop batching and the comparison would still
//! read as valid while measuring something other than what it names. The
//! alternative is a flaky assertion inside a benchmark harness, which is worse.
//! The third survivor, the closing partial-batch commit after
//! `run_mutex_queue`'s producer loop, is the one that *was* defendable. It was
//! defended, and G12's `s1_closing_batch_count` mutant now holds it instead of
//! this note.
//!
//! | Claim | Test |
//! |---|---|
//! | A degenerate workload is refused before it can tie every candidate at zero | `a_workload_refuses_every_degenerate_dimension` |
//! | The producer count and the backend it needs are one number | `the_producer_count_sets_the_backend_it_needs` |
//! | Every candidate names itself and transcribes its ceiling | `every_candidate_declares_a_name_and_a_ceiling` |
//! | The Contract door caps multi-producer structures at one producer | `the_contract_door_caps_a_multi_producer_structure_at_one_producer` |
//! | A roomy run keeps everything and returns everything | `a_roomy_run_keeps_everything_and_returns_it` |
//! | A partial final batch is published, not abandoned | `a_partial_final_batch_is_published_rather_than_abandoned` |
//! | A cramped run drops, and the drop is counted from the drain | `a_cramped_run_drops_and_the_drop_is_counted_from_the_drain` |
//! | A `DropNewest` ring reports successes it did not keep | `a_dropnewest_ring_reports_successes_it_did_not_keep` |
//! | Under `Fail` the reported/received gap closes everywhere | `a_failing_policy_closes_the_gap_for_every_candidate` |
//! | A path that dropped records cannot be fastest | `a_path_that_dropped_records_is_not_eligible_to_be_fastest` |
//! | Refusals are listed, not omitted | `a_comparison_lists_refusals_rather_than_shortening_the_table` |
//! | The same workload twice repeats every count, but not a duration | `a_comparison_of_the_same_workload_repeats_its_counts` |
//! | The counters are the run's own totals | `the_counters_are_the_runs_own_totals` |
//! | A policy refusal names the crate that refused | `a_policy_refusal_names_the_crate_that_refused` |
//! | `with_config` ignores the policy the factory refuses | `the_direct_doors_ignore_the_policy_the_contract_door_refuses` |
//! | Every error renders for a human | `every_error_renders` |
//! | The flush relay is unreachable while the batch ties the buffer | `the_flush_relay_is_unreachable_while_the_batch_ties_the_buffer` |
//! | The report names every candidate and every refusal | `the_report_names_every_candidate_and_every_refusal` |
//! | The candidate list matches a copy written outside the declaration | `the_candidate_list_matches_a_copy_written_outside_the_declaration` |
//! | The example switches on a name a candidate returns | `the_example_switches_on_a_name_a_candidate_returns` |
//! | The batch a workload reports is the batch its config carries | `the_batch_reported_is_the_batch_the_config_carries` |
//! | Both ordering subtractions are guarded unconditionally | `both_ordering_subtractions_are_guarded_unconditionally` |
//! | Both halves of the Copy coupling are named | `both_halves_of_the_copy_coupling_are_named` |
//! | The semantics axis defaults to `Set` and one cell, and both are settable | `the_accumulator_axis_defaults_to_set_and_one_cell` |
//! | Every candidate agrees on the `Delta` table for the same workload | `every_candidate_agrees_on_the_delta_table_at_one_producer` |
//! | `Set` keeps only the last write once a cell is shared | `set_semantics_keeps_only_the_last_write_when_one_cell_is_shared` |
//! | `Delta` sums correctly across many producers with no lock between them | `delta_sums_correctly_across_many_producers_without_a_lock` |
//! | A drained record that decodes outside its producer range is caught, not folded in | `every_drained_record_decodes_to_a_producer_the_workload_describes` |
//! | The overflow gap never reaches the accumulator table | `the_overflow_gap_never_reaches_the_accumulator_table` |
//! | A lock poisoned by a panicking holder recovers instead of propagating | `a_lock_poisoned_by_a_panicking_holder_recovers_instead_of_propagating` |

// Ordinary tests, compiled out under `--cfg loom`. That cfg swaps
// `ring_atomic`'s atomics for loom's instrumented ones across the whole
// family, and those panic the moment they are touched outside a
// `loom::model` closure. Without this gate a family-wide loom run dies
// here instead of reaching the models in `ring_spsc`, `ring_mpsc`,
// `ring_publish` and `ring_testkit`.
#![cfg(not(loom))]

use ring_bench::{AccumulatorSemantics, Candidate, Comparison, Outcome, RunError, Workload, WorkloadError, run};
use ring_factory::RingConfig;
use ring_types::OverflowPolicy;

/// 256 records into 4096 slots, where every candidate keeps everything.
fn roomy() -> Workload {
  Workload::new(RingConfig::new(4096).unwrap())
    .with_records_per_producer(256)
    .unwrap()
    .with_batch(32)
    .unwrap()
}

/// 256 records into 16 slots, where every candidate drops by a different mechanism.
fn cramped() -> Workload {
  Workload::new(RingConfig::new(16).unwrap())
    .with_records_per_producer(256)
    .unwrap()
    .with_batch(32)
    .unwrap()
}

/// Four producers, 1024 records, 4096 slots. Only the unbounded candidates run.
fn parallel() -> Workload {
  Workload::new(RingConfig::new(4096).unwrap())
    .with_producers(4)
    .unwrap()
    .with_records_per_producer(256)
    .unwrap()
    .with_batch(32)
    .unwrap()
}

/// A workload with a zero in any dimension is refused at description time.
///
/// Each zero degrades into a *working* run rather than an obviously broken one,
/// which is why they are refused rather than clamped. Zero producers or zero
/// records ties every candidate at zero nanoseconds, and zero batch makes the
/// staged candidate publish nothing while leaving the unstaged ones untouched.
/// All three produce a report that looks like a result.
#[test]
fn a_workload_refuses_every_degenerate_dimension() {
  let base = Workload::new(RingConfig::new(64).unwrap());

  assert_eq!(base.with_producers(0).unwrap_err(), WorkloadError::ZeroProducers);
  assert_eq!(base.with_records_per_producer(0).unwrap_err(), WorkloadError::ZeroRecords);
  assert_eq!(base.with_batch(0).unwrap_err(), WorkloadError::ZeroBatch);
  assert_eq!(base.with_cells(0).unwrap_err(), WorkloadError::ZeroCells);

  assert_eq!(
    WorkloadError::ZeroProducers.to_string(),
    "a workload needs at least one producer",
  );
  assert_eq!(
    WorkloadError::ZeroRecords.to_string(),
    "a workload needs at least one record per producer",
  );
  assert_eq!(
    WorkloadError::ZeroBatch.to_string(),
    "a workload needs a batch size of at least one",
  );
  assert_eq!(
    WorkloadError::ZeroCells.to_string(),
    "a workload needs at least one accumulator cell",
  );

  let boxed: Box<dyn core::error::Error> = Box::new(WorkloadError::ZeroBatch);
  assert!(boxed.to_string().contains("batch size"));
}

/// `RingConfig` has its own `producers` field, which selects the backend rather
/// than the thread count.
///
/// Two numbers with one name is the trap this test exists to keep shut. There
/// is no setter that moves one without the other, so a four-thread workload
/// cannot be run against a ring configured single-producer. The batch is tied
/// the same way, for the same reason.
#[test]
fn the_producer_count_sets_the_backend_it_needs() {
  let workload = parallel();

  assert_eq!(workload.producers(), 4);
  assert_eq!(workload.config().producers(), 4);
  assert!(workload.config().is_multi_producer());

  assert_eq!(workload.batch(), 32);
  assert_eq!(workload.config().batch(), 32);

  assert_eq!(workload.records_per_producer(), 256);
  assert_eq!(workload.offered(), 1024);
  assert_eq!(workload.capacity(), 4096);

  // Each producer's records are disjoint, so a drained record identifies its
  // writer. Nothing asserts on the values yet; the property is what makes an
  // ordering question askable later of a run already recorded.
  assert_eq!(workload.records_of(0), 0..256);
  assert_eq!(workload.records_of(3), 768..1024);
}

/// Every candidate names itself and states what it admits.
#[test]
fn every_candidate_declares_a_name_and_a_ceiling() {
  let mut names: Vec<&str> = Candidate::ALL.iter().map(|c| c.name()).collect();
  let unique = names.len();
  names.sort_unstable();
  names.dedup();
  assert_eq!(names.len(), unique, "two candidates share a name: {names:?}");

  for candidate in Candidate::ALL {
    assert!(candidate.admits(1), "{} refuses a single producer", candidate.name());

    match candidate.producer_ceiling() {
      Some(ceiling) => assert!(!candidate.admits(ceiling + 1)),
      None => assert!(candidate.admits(64)),
    }
  }

  // Fix(ceiling_literals_are_a_transcription_check): these are a transcription
  // check, not a guard against the ceiling going stale. `producer_ceiling` is a
  // hand-written `match` over literals. `ring_handle`, the crate that imposes
  // the `1` on `ContractRing`, `TlsOverRing` and `OffTheShelf`, is not a
  // dependency of this crate at all. If that door widened, nothing here would
  // notice, and correcting the literals afterwards would mean editing this
  // test, which is the opposite of what a guard does. What these lines do catch
  // is a local edit to the `match`.
  //
  // Root cause: a value owned by another crate, transcribed by hand, cannot be
  // asserted against its source by a suite that does not depend on that source.
  // Pitfall: an assertion pinning a transcription reads like an assertion
  // pinning the thing transcribed.
  assert_eq!(Candidate::MutexQueue.producer_ceiling(), None);
  assert_eq!(Candidate::DirectMpsc.producer_ceiling(), None);
  assert_eq!(Candidate::ContractRing.producer_ceiling(), Some(1));
  assert_eq!(Candidate::TlsOverRing.producer_ceiling(), Some(1));
  assert_eq!(Candidate::DirectSpsc.producer_ceiling(), Some(1));
  // Fix(ceiling_literals_are_a_transcription_check): the sixth. `OffTheShelf`'s
  // `1` has the same off-crate owner as the other two and had no literal of its
  // own, only the generic loop above, whose `None` branch a widened ceiling
  // would pass.
  #[cfg(feature = "crossbeam")]
  assert_eq!(Candidate::OffTheShelf.producer_ceiling(), Some(1));

  assert!(format!("{:?}", Candidate::MutexQueue).contains("MutexQueue"));
}

/// The Contract door caps at one producer a structure that has no such cap.
///
/// This is the crate's headline finding and the reason `DirectMpsc` exists.
/// `ContractRing` at four producers is refused, and the refusal has nothing to
/// do with the ring. `ring_factory::build` hands back a `ring_handle::Split`,
/// whose `Ends::split` yields one producer with no `try_clone` beside it.
/// `DirectMpsc` is the *same ring in the same configuration*, reached two
/// levels lower, and it runs.
///
/// The feature (`docs/feature/186_ring_benchmark_harness.md`) requires the
/// candidates be compared "under the same producer counts". Through one door
/// they cannot be, and this test asserts where the door stops.
#[test]
fn the_contract_door_caps_a_multi_producer_structure_at_one_producer() {
  let workload = parallel();
  assert!(workload.config().is_multi_producer(), "the config asks for the MPSC backend");

  let refused = run(Candidate::ContractRing, &workload).unwrap_err();
  assert_eq!(
    refused,
    RunError::ProducerCeiling {
      candidate: Candidate::ContractRing,
      requested: 4,
      ceiling: 1
    },
  );

  // Same ring, same config, no ceiling, reached below the Contract.
  let direct = run(Candidate::DirectMpsc, &workload).unwrap();
  assert_eq!(direct.producers(), 4);
  assert_eq!(direct.offered(), 1024);
  assert!(direct.is_lossless(), "4096 slots hold 1024 records");
  assert!(direct.conserved());

  // And the baseline, which never had a ceiling to begin with.
  let mutex = run(Candidate::MutexQueue, &workload).unwrap();
  assert!(mutex.is_lossless());
  assert!(mutex.conserved());
}

/// With room for everything, every candidate keeps everything and gives it back.
#[test]
fn a_roomy_run_keeps_everything_and_returns_it() {
  let workload = roomy();

  for candidate in Candidate::ALL {
    let outcome = run(*candidate, &workload).unwrap();

    assert_eq!(outcome.candidate(), *candidate);
    assert_eq!(outcome.producers(), 1);
    assert_eq!(outcome.offered(), 256);
    assert_eq!(
      outcome.reported(),
      256,
      "{} refused a record with room to spare",
      candidate.name()
    );
    assert_eq!(outcome.received(), 256, "{} could not return what it took", candidate.name());
    assert_eq!(outcome.dropped(), 0);
    assert_eq!(outcome.silently_discarded(), 0);
    assert!(outcome.is_lossless());
    assert!(outcome.conserved());

    // Read, never bounded. A duration nobody reads is a duration that can stop
    // being written without any test noticing; a duration anyone bounds is a
    // flaky test. Reading it and asserting nothing avoids both.
    let _elapsed = outcome.write_nanos();
  }
}

/// A partial final batch is published by the closing drain, not abandoned.
///
/// Both fixtures above offer 256 records in batches of 32, which divides
/// exactly. So the staged candidate's `drain_final` had nothing left to do, and
/// nothing in this file exercised the tail. The arithmetic accident is the
/// finding. Every test above would still have reported a staged path that
/// dropped its tail as lossless, and it would have looked lossy only on
/// workload dimensions nobody had written. Coverage caught it; no assertion
/// did.
#[test]
fn a_partial_final_batch_is_published_rather_than_abandoned() {
  let workload = Workload::new(RingConfig::new(4096).unwrap())
    .with_records_per_producer(250)
    .unwrap()
    .with_batch(32)
    .unwrap();

  assert_ne!(
    workload.records_per_producer() % workload.batch(),
    0,
    "the point of this fixture is the 26-record remainder",
  );

  let staged = run(Candidate::TlsOverRing, &workload).unwrap();
  assert_eq!(staged.received(), 250, "the last 26 records were staged, not lost");
  assert!(staged.is_lossless());
  assert!(staged.conserved());

  // Every other candidate lands the same odd total, which is what makes the
  // staged one's result an assertion about `drain_final` rather than about the
  // workload being small enough for anything to succeed.
  for candidate in Candidate::ALL {
    let outcome = run(*candidate, &workload).unwrap();
    assert!(outcome.is_lossless(), "{} lost the partial tail", candidate.name());
    assert_eq!(outcome.received(), 250);

    // `conserved()` is the only assertion here that reads `reported`, and it
    // has to run on *this* fixture rather than the dividing ones. A candidate
    // that miscounts its closing partial batch is off by the remainder, which
    // is zero wherever the batch divides. Checking it only on `TlsOverRing`
    // above left the other five counting their tail unchecked. A survey
    // mutation flipped `MutexQueue`'s closing `+=` to `-=`, putting
    // `reported` 26 below `received`, and all 18 tests stayed green.
    assert!(
      outcome.conserved(),
      "{} reported {} for {} records it actually kept",
      candidate.name(),
      outcome.reported(),
      outcome.received(),
    );
  }
}

/// Under back-pressure every candidate drops, and the drop is counted from the
/// drain rather than from what the write API said.
///
/// The invariant that must hold for every candidate on every workload is
/// `received <= reported <= offered`. Nothing stronger is available, because
/// the middle term can equal the last while the first is a fraction of it. The
/// next test covers that case.
#[test]
fn a_cramped_run_drops_and_the_drop_is_counted_from_the_drain() {
  let workload = cramped();

  for candidate in Candidate::ALL {
    let outcome = run(*candidate, &workload).unwrap();

    assert_eq!(outcome.offered(), 256);
    assert!(
      outcome.reported() <= 256,
      "{} took more than it was offered",
      candidate.name()
    );
    assert!(
      outcome.received() <= outcome.reported(),
      "{} drained more than it took",
      candidate.name()
    );
    assert!(
      outcome.received() <= 16,
      "{} drained more than the ring holds",
      candidate.name()
    );
    assert!(!outcome.is_lossless(), "{} kept 256 records in 16 slots", candidate.name());
    assert_eq!(outcome.dropped(), 256 - outcome.received());
    assert_eq!(outcome.silently_discarded(), outcome.reported() - outcome.received());
  }

  // The staged candidate is the one that discards nothing. `Flusher` checks
  // `free_capacity` before touching the buffer and reports `Rejected` rather
  // than handing a record to a full ring. `ring_flush` documents that a
  // rejection must be retried and that it will not retry for you; a harness
  // that retried would be measuring its own retry loop. So this candidate
  // loses records to refusals it declines to retry, and never to a silent
  // discard.
  //
  // Fix(workload_batch_was_unclamped): this used to assert `reported() == 0`.
  // The staged candidate kept nothing at all because `Workload::batch()`
  // returned an unclamped 32 against a 16-slot ring, so the very first
  // `free_capacity` check failed, the buffer stayed full, and every later
  // `append` was refused. That permanent stall read as a measurement. With
  // `batch()` now read through the config it is the clamped 16, one flush fits
  // exactly, and the candidate measures one batch instead of nothing.
  let staged = run(Candidate::TlsOverRing, &workload).unwrap();
  assert_eq!(staged.reported(), 16, "one full batch fits the ring exactly and is published");
  assert_eq!(staged.received(), 16, "and is drained — the flush was accepted, not refused");
  assert_eq!(
    staged.silently_discarded(),
    0,
    "the free_capacity pre-check is what rules this out"
  );
  assert!(staged.conserved());
}

/// A `DropNewest` ring reports successes for records it did not keep.
///
/// The finding that reshaped this crate. `OverflowPolicy::default()` is
/// `DropNewest`, so a bare `RingConfig::new( n )`, the form the family's own
/// documentation uses everywhere, produces a ring whose `try_push` returns
/// `Ok` for a discarded record. The first working version of this harness
/// counted those `Ok`s and reported `contract_ring` as **lossless at 256
/// records in a 16-slot ring**.
///
/// It would also have reported it as *fast*, and correctly, because discarding
/// is the cheapest thing a queue can do. So the failure mode is the harness
/// recommending the candidate that threw the workload away, not just a wrong
/// number in a column. That is the verdict this crate exists to make
/// trustworthy, which is why this test asserts it rather than noting it.
#[test]
fn a_dropnewest_ring_reports_successes_it_did_not_keep() {
  let workload = cramped();
  assert_eq!(
    workload.config().overflow(),
    OverflowPolicy::DropNewest,
    "the default policy, which nobody in this test set",
  );

  let through_the_factory = run(Candidate::ContractRing, &workload).unwrap();
  assert_eq!(through_the_factory.reported(), 256, "every push returned Ok");
  assert!(through_the_factory.received() <= 16, "the ring has 16 slots");
  assert!(through_the_factory.silently_discarded() >= 240);
  assert!(!through_the_factory.conserved());
  assert!(
    !through_the_factory.is_lossless(),
    "the judgement that reading `reported` instead of `received` would invert",
  );

  // This test pins the counters, on the one candidate where `reported` and
  // `received` differ, because `the_counters_are_the_runs_own_totals` cannot
  // pin them. It runs `MutexQueue`, whose two counts are equal, so the whole
  // mapping is invisible to it. Manual stage B5 caught this by reintroducing
  // the original defect and watching all 18 tests stay green.
  let stats = through_the_factory.stats();
  assert_eq!(stats.claimed(), through_the_factory.received() as u64);
  assert_eq!(stats.published(), through_the_factory.received() as u64);
  assert_eq!(stats.consumed(), through_the_factory.received() as u64);
  assert_eq!(
    stats.dropped_total(),
    through_the_factory.dropped() as u64,
    "240 records the ring never took, not 0",
  );
  // Structural, not measured. `in_flight` is `claimed - published` and `run`
  // hands both the same expression, so this reads 0 for every mapping that
  // keeps them equal. That includes the wrong one that maps `reported` through
  // both. It was tried, and it left this line green while the three assertions
  // above caught it. The 240-leak reading needs `claimed` and `published` to
  // diverge, which no mapping in this harness's history did.
  assert_eq!(
    stats.in_flight(),
    0,
    "the two slot counters agree, as every mapping this harness has used makes them",
  );

  // Four candidates on the identical workload hand their refusals back. The
  // mutex queue checks its own length, the two direct backends return `Err`
  // because `ring_core` applies the policy above them and it never reaches
  // them, and the staged candidate refuses before it writes.
  for candidate in [
    Candidate::MutexQueue,
    Candidate::DirectSpsc,
    Candidate::DirectMpsc,
    Candidate::TlsOverRing,
  ] {
    let outcome = run(candidate, &workload).unwrap();
    assert!(outcome.conserved(), "{} absorbed a record it reported", candidate.name());
    assert_eq!(outcome.silently_discarded(), 0);
  }

  // The mixed run is the dangerous one. Some candidates report their drops and
  // some absorb them, so a ranking read off `reported` is ordered wrong, not
  // just imprecise.
  let comparison = Comparison::run(workload);
  assert!(!comparison.conserved());
  assert!(comparison.silently_discarded() >= 240);
  assert!(comparison.fastest().is_none(), "no candidate kept the workload");
}

/// Under `OverflowPolicy::Fail` the gap closes and every candidate agrees.
///
/// The counterpart to the test above, and the reason the crate reports the gap
/// instead of designing it out. The gap is a property of the configuration, not
/// of the paths. Same capacity, same records, one field changed.
#[test]
fn a_failing_policy_closes_the_gap_for_every_candidate() {
  let workload = Workload::new(RingConfig::new(16).unwrap().with_overflow(OverflowPolicy::Fail))
    .with_records_per_producer(256)
    .unwrap()
    .with_batch(32)
    .unwrap();

  let comparison = Comparison::run(workload);
  assert!(comparison.conserved(), "every candidate reported exactly what it kept");
  assert_eq!(comparison.silently_discarded(), 0);
  assert!(comparison.fastest().is_none(), "16 slots still cannot hold 256 records");

  for outcome in comparison.outcomes() {
    assert_eq!(
      outcome.reported(),
      outcome.received(),
      "{} diverged under Fail",
      outcome.candidate().name(),
    );
  }
}

/// A path that dropped records is not eligible to be fastest.
///
/// The quickest way to finish a write phase is to refuse every record, so a
/// harness that ranks only by elapsed time ranks the worst candidate first and
/// prints a plausible number while doing it. This test asserts that the *set*
/// of eligible candidates is right, never which one wins.
#[test]
fn a_path_that_dropped_records_is_not_eligible_to_be_fastest() {
  let roomy_run = Comparison::run(roomy());
  assert!(roomy_run.conserved());
  let fastest = roomy_run.fastest().expect("every candidate was lossless");
  assert!(fastest.is_lossless());
  assert_eq!(roomy_run.outcomes().len(), Candidate::ALL.len());
  assert!(roomy_run.refusals().is_empty());

  let cramped_run = Comparison::run(cramped());
  assert!(cramped_run.outcomes().iter().all(|o| !o.is_lossless()));
  assert!(cramped_run.fastest().is_none(), "no candidate kept the workload");

  assert_eq!(cramped_run.workload().capacity(), 16);
}

/// A candidate the producer count excludes is listed as refused, not omitted.
///
/// A shorter table looks like a comparison of fewer candidates. A table plus a
/// refusal list says which paths could not be reached at this producer count
/// and why. Given the previous test's finding, that list is the more
/// interesting half of a four-producer run.
#[test]
fn a_comparison_lists_refusals_rather_than_shortening_the_table() {
  let comparison = Comparison::run(parallel());

  let ran: Vec<Candidate> = comparison.outcomes().iter().map(Outcome::candidate).collect();
  assert!(ran.contains(&Candidate::MutexQueue));
  assert!(ran.contains(&Candidate::DirectMpsc));
  assert!(!ran.contains(&Candidate::ContractRing));

  assert_eq!(
    comparison.outcomes().len() + comparison.refusals().len(),
    Candidate::ALL.len(),
    "every candidate is accounted for exactly once",
  );

  for refusal in comparison.refusals() {
    assert!(
      matches!(
        refusal,
        RunError::ProducerCeiling {
          requested: 4,
          ceiling: 1,
          ..
        }
      ),
      "unexpected refusal: {refusal}",
    );
  }
}

/// The same workload twice produces the same everything except the duration.
///
/// This is the reproducibility half of the requirement's criterion, and the
/// point is where it stops. The counts are exact across runs, the refusal list
/// is exact, and the *eligible set*, meaning the candidates that kept the whole
/// workload, is exact. No test here or anywhere asserts which of those eligible
/// candidates came out fastest, because it is the one output of this crate that
/// legitimately varies with machine load.
///
/// So everything a downstream reader would treat as a fact is reproducible, and
/// the single quantity that is not is the one nobody is allowed to assert on.
#[test]
fn a_comparison_of_the_same_workload_repeats_its_counts() {
  let first = Comparison::run(cramped());
  let second = Comparison::run(cramped());

  assert_eq!(first.outcomes().len(), second.outcomes().len());
  assert_eq!(first.refusals(), second.refusals());
  assert_eq!(first.conserved(), second.conserved());

  for (a, b) in first.outcomes().iter().zip(second.outcomes()) {
    assert_eq!(a.candidate(), b.candidate());
    assert_eq!(a.offered(), b.offered(), "{:?}", a.candidate());
    assert_eq!(a.reported(), b.reported(), "{:?}", a.candidate());
    assert_eq!(a.received(), b.received(), "{:?}", a.candidate());
    assert_eq!(a.dropped(), b.dropped(), "{:?}", a.candidate());
    assert_eq!(a.silently_discarded(), b.silently_discarded(), "{:?}", a.candidate());
    assert_eq!(a.is_lossless(), b.is_lossless(), "{:?}", a.candidate());
  }

  // The durations are read and deliberately not compared. A value nobody reads
  // can stop being written, and a value compared across runs is a flaky test.
  for outcome in first.outcomes() {
    let _ = outcome.write_nanos();
  }

  // Eligibility is deterministic even though the winner is not. On the cramped
  // workload nothing is eligible, which is itself the stable fact.
  assert!(first.fastest().is_none());
  assert!(second.fastest().is_none());
}

/// The counters carry the run's own totals, written after the clock.
///
/// Incrementing an atomic per record would instrument the very loop under
/// measurement, so every counter is written once, from totals, outside the
/// timed region. The cost is that no intra-run distribution is available; the
/// benefit is that the number reported is of the write path rather than of the
/// counting.
///
/// The mapping asserted here is narrower than it first looks, and the first
/// version of it was wrong. `claimed` and `published` are *slot* lifecycle
/// counters. `RingStats::in_flight()` is defined as `claimed - published`, so
/// feeding them workload totals (`claimed = offered`, `published = accepted`)
/// made 240 refused records read as 240 leaked slots on a ring that had only
/// 16. A record the ring never took never occupied a slot, so it was never
/// claimed either. All three lifecycle counters therefore take `received`, and
/// the records that did not survive are recorded once, as drops.
#[test]
fn the_counters_are_the_runs_own_totals() {
  let workload = cramped();
  let policy = workload.config().overflow();
  let outcome = run(Candidate::MutexQueue, &workload).unwrap();

  assert!(outcome.received() > 0, "the queue kept something");
  assert!(outcome.dropped() > 0, "16 slots, 256 records");

  let stats = outcome.stats();
  assert_eq!(stats.claimed(), outcome.received() as u64);
  assert_eq!(stats.published(), outcome.received() as u64);
  assert_eq!(stats.consumed(), outcome.received() as u64);
  assert_eq!(stats.dropped(policy), outcome.dropped() as u64);
  assert_eq!(stats.dropped_total(), outcome.dropped() as u64);

  // A slot claimed, published, and drained is a slot nobody still holds. The
  // assertion uses the ring's own vocabulary, not the workload's. It would
  // read 240 if the refused records were mapped through the lifecycle.
  //
  // It is structurally zero. `run` hands `claimed` and `published` the same
  // expression, so no run of this harness can make it anything else. This
  // line pins the mapping's consequence and cannot fail by itself. The
  // assertion that guards the mapping is in
  // `a_dropnewest_ring_reports_successes_it_did_not_keep`, on the one candidate
  // where `reported` and `received` differ.
  assert_eq!(stats.in_flight(), 0);

  // Never recorded. No candidate blocks, so there is no wait to time. Asserting
  // the zero marks the omission as deliberate rather than forgotten.
  assert_eq!(stats.wait_nanos(), 0);

  // The offered total survives on the outcome rather than in the counters,
  // which is why `Outcome` carries it separately.
  assert_eq!(outcome.received() + outcome.dropped(), outcome.offered());
}

/// A refused policy relays the crate that refused it, not a copy of its ruling.
///
/// `ring_core` refuses `OverflowPolicy::DropOldest`, because evicting an
/// unread record contradicts the exactly-once delivery both in-house backends
/// guarantee. Two candidates reach that refusal by different routes, and the
/// error says which route. Through the factory it arrives as
/// `RunError::Build`. The staged candidate builds its ring directly, because
/// `ring_flush::Flusher` needs a `ring_core::Producer` the Contract cannot hand
/// it, so its refusal arrives as `RunError::Ring`.
#[test]
fn a_policy_refusal_names_the_crate_that_refused() {
  let evicting = Workload::new(RingConfig::new(64).unwrap().with_overflow(OverflowPolicy::DropOldest))
    .with_records_per_producer(32)
    .unwrap();

  let built = run(Candidate::ContractRing, &evicting).unwrap_err();
  let direct = run(Candidate::TlsOverRing, &evicting).unwrap_err();

  assert!(matches!(
    built,
    RunError::Build {
      candidate: Candidate::ContractRing,
      ..
    }
  ));
  assert!(matches!(
    direct,
    RunError::Ring {
      candidate: Candidate::TlsOverRing,
      ..
    }
  ));

  // Fix(relayed_refusals_dropped_the_candidate): the two assertions above are
  // variant tags, and this test's name is a claim about what a reader sees.
  // These two check that claim.
  assert!(
    built.to_string().starts_with("contract_ring: "),
    "a relayed refusal renders without its candidate: {built}",
  );
  assert!(
    direct.to_string().starts_with("tls_over_ring: "),
    "a relayed refusal renders without its candidate: {direct}",
  );
}

/// The direct doors ignore the policy the Contract door refuses.
///
/// `ring_spsc::Ring::with_config` and `ring_mpsc::Ring::with_config` read one
/// field of the five and build a ring regardless. So the identical
/// `RingConfig` that `ring_factory::build` rejects outright produces a working
/// ring one level down, with the policy silently absent.
///
/// This divergence was an open question about `with_config`, measured here
/// rather than described. Its *name* claims more than it does, and it is the
/// one construction path that reads as compliance while bypassing the factory.
/// This test is the evidence any answer needs, and it belongs here because this
/// is the first crate that reaches both doors with one config.
#[test]
fn the_direct_doors_ignore_the_policy_the_contract_door_refuses() {
  let evicting = Workload::new(RingConfig::new(64).unwrap().with_overflow(OverflowPolicy::DropOldest))
    .with_records_per_producer(32)
    .unwrap();

  assert!(run(Candidate::ContractRing, &evicting).is_err(), "the factory refuses");

  for candidate in [Candidate::DirectSpsc, Candidate::DirectMpsc] {
    let outcome = run(candidate, &evicting).unwrap();
    assert!(outcome.is_lossless(), "{} built and filled a ring anyway", candidate.name());
  }
}

/// Every error this crate can produce renders for a human. None of them
/// chains. `core::error::Error::source()` is unimplemented family-wide
/// (`RunError` and its three wrapped error types alike), so a caller
/// holding a `Box< dyn Error >` learns nothing beyond what `Display` says.
#[test]
fn every_error_renders() {
  let ceiling = RunError::ProducerCeiling {
    candidate: Candidate::DirectSpsc,
    requested: 8,
    ceiling: 1,
  };
  assert_eq!(ceiling.to_string(), "direct_spsc admits 1 producer(s), asked for 8");

  let evicting = Workload::new(RingConfig::new(64).unwrap().with_overflow(OverflowPolicy::DropOldest));
  let build = run(Candidate::ContractRing, &evicting).unwrap_err();
  let ring = run(Candidate::TlsOverRing, &evicting).unwrap_err();
  assert!(build.to_string().contains("polic"), "{build}");
  assert!(ring.to_string().contains("polic"), "{ring}");

  let boxed: Box<dyn core::error::Error> = Box::new(ceiling);
  assert!(boxed.to_string().contains("direct_spsc"));

  assert!(format!("{ceiling:?}").contains("ProducerCeiling"));
}

/// The flush relay is unreachable while the batch ties the buffer.
///
/// `RunError::Flush` exists and no input reaches it. `run_tls_over_ring` builds
/// its `TlsBuffer` with `workload.batch()` slots and binds
/// `FlushPolicy::OnBatch( workload.batch() )`. `ring_flush` has two binding
/// refusals, a zero batch and a batch above the buffer's capacity, and
/// construction excludes both. `Workload` refuses a zero batch, and `n > n` is
/// false.
///
/// The variant stays because the tie is one edit from being broken and nothing
/// in either type enforces it. It is kept and named as dead, rather than
/// replaced by an `expect` that would turn a future configuration mistake into
/// a panic inside a measurement.
#[test]
fn the_flush_relay_is_unreachable_while_the_batch_ties_the_buffer() {
  let workload = roomy();
  assert_eq!(
    workload.batch(),
    workload.config().batch(),
    "the buffer's capacity and the flush trigger are the same number",
  );

  // Constructed directly, since no workload produces it.
  let relay = RunError::Flush {
    candidate: Candidate::TlsOverRing,
    error: ring_flush::ConfigError::ZeroBatch,
  };
  assert_eq!(relay.to_string(), "tls_over_ring: OnBatch( 0 ) fires on every append");
}

/// The report names every candidate that ran and every one that did not.
#[test]
fn the_report_names_every_candidate_and_every_refusal() {
  let roomy_report = Comparison::run(roomy()).report();
  for candidate in Candidate::ALL {
    assert!(roomy_report.contains(candidate.name()), "missing {}", candidate.name());
  }
  assert!(roomy_report.contains("fastest lossless: "));
  assert!(roomy_report.contains("capacity 4096"));

  let cramped_report = Comparison::run(cramped()).report();
  assert!(cramped_report.contains("every candidate dropped records"));

  let parallel_report = Comparison::run(parallel()).report();
  assert!(parallel_report.contains("refused: contract_ring"));
  assert!(parallel_report.contains("4 producer(s) x 256 records"));

  // Fix(relayed_refusals_dropped_the_candidate): "every refusal" in this test's
  // name used to be one refusal, `ProducerCeiling`, the only variant that
  // carried a name. A `DropOldest` workload refuses two candidates through two
  // relaying variants, and both lines must now carry a candidate for the report
  // to be keyed on names.
  let evicting = Comparison::run(
    Workload::new(RingConfig::new(64).unwrap().with_overflow(OverflowPolicy::DropOldest))
      .with_records_per_producer(32)
      .unwrap(),
  )
  .report();
  assert!(evicting.contains("refused: contract_ring: "), "{evicting}");
  assert!(evicting.contains("refused: tls_over_ring: "), "{evicting}");
}

/// The candidate list is written down somewhere other than its own declaration.
///
/// **Root Cause.** `Candidate::ALL` is declared twice by hand under opposite
/// `cfg`s. Nothing compared the two, and the two tests that touched the list's
/// shape both compared a length back to `Candidate::ALL.len()`. That is a
/// tautology in either build, and it would pass if the copies listed different
/// candidates in a different order.
///
/// **Why Not Caught.** Order is the list's meaning here, and nothing
/// type-checks an order. `Comparison::fastest` breaks a tie by taking the first
/// minimum, so declaration order decides the verdict.
///
/// **Fix Applied.** This test spells out the expected names and their order, so
/// the assertion compares the declaration against an independent copy rather
/// than against itself.
///
/// **Prevention.** Reordering either copy, or adding a candidate to one and not
/// the other, fails here. The crossbeam arm also pins that the extra candidate
/// is appended rather than inserted.
///
/// **Pitfall.** Asserting a collection's length against its own `len()` proves
/// the collection exists. To check a list, compare it to a list written
/// somewhere else.
#[test]
fn the_candidate_list_matches_a_copy_written_outside_the_declaration() {
  const COMMON: [&str; 5] = ["mutex_queue", "contract_ring", "tls_over_ring", "direct_spsc", "direct_mpsc"];

  let names: Vec<&str> = Candidate::ALL.iter().map(|c| c.name()).collect();

  assert_eq!(
    &names[..COMMON.len()],
    &COMMON[..],
    "the five candidates both cfg arms declare, in the order fastest() breaks ties by",
  );

  #[cfg(feature = "crossbeam")]
  assert_eq!(
    names.len(),
    COMMON.len() + 1,
    "the crossbeam arm appends off_the_shelf rather than inserting it",
  );
  #[cfg(feature = "crossbeam")]
  assert_eq!(names[COMMON.len()], "off_the_shelf");

  #[cfg(not(feature = "crossbeam"))]
  assert_eq!(names.len(), COMMON.len(), "the default arm declares exactly the five");
}

/// The name the example switches on is a name a `Candidate` returns.
///
/// **Root Cause.** `examples/comparison.rs` selects a candidate by comparing
/// `name()` against the string literal `"contract_ring"`. `Candidate::name` is a
/// `match` over an enum, so renaming an arm compiles cleanly and silently turns
/// that comparison into one that never matches.
///
/// **Why Not Caught.** `### Validation` on `Candidate` says there is nothing to
/// validate because the type is a fieldless enum. The example is not a test
/// target. Nothing in `tests/` runs it, so the mismatch would first appear as a
/// report printing a `usize::MAX` spread and a `0.0x` ratio.
///
/// **Fix Applied.** This test reads the example's own source and asserts the
/// literal it switches on is exactly what `Candidate::ContractRing::name`
/// returns.
///
/// **Prevention.** Renaming either side fails here. `include_str!` reads the
/// real file at compile time, so there is no copy of the example to drift from.
///
/// **Pitfall.** "Fieldless enum, nothing to validate" is about the type. It says
/// nothing about the strings the type hands out, which code in files no test
/// runs may depend on.
#[test]
fn the_example_switches_on_a_name_a_candidate_returns() {
  let source = include_str!("../examples/comparison.rs");
  let literal = format!("== \"{}\"", Candidate::ContractRing.name());

  assert!(
    source.contains(&literal),
    "examples/comparison.rs no longer compares against {literal}",
  );
}

/// The batch a workload reports is the batch its config carries.
///
/// # Root Cause
///
/// `Workload` mirrored `RingConfig`'s `batch` in a field of its own.
/// `with_batch` wrote the caller's value to that field and the *clamped* value
/// to the config, so the two disagreed for any `batch > capacity`.
///
/// # Why Not Caught
///
/// Both existing assertions of the tie were handed `roomy()` and `parallel()`,
/// the two fixtures with capacity 4096, where a batch of 32 clamps to itself.
/// Nothing asserted on `cramped()`, which has capacity 16 and batch 32 and is
/// the only fixture that could show the split.
///
/// # Fix Applied
///
/// The mirror field is gone; `Workload::batch()` reads `self.config.batch()`.
/// This test asserts the tie on `cramped()` specifically, so a reintroduced
/// mirror fails here rather than in whichever fixture happens to be checked.
///
/// # Prevention
///
/// Assert an agreement on the input that could break it, not on the input that
/// makes it hold trivially.
///
/// # Pitfall
///
/// When a validating type already stores a value, a second copy outside it is
/// the unvalidated one, and it is the copy every caller reads.
#[test]
fn the_batch_reported_is_the_batch_the_config_carries() {
  for (name, workload) in [("roomy", roomy()), ("cramped", cramped()), ("parallel", parallel())] {
    assert_eq!(
      workload.batch(),
      workload.config().batch(),
      "{name}: the reported batch and the config's batch are one value",
    );
    assert!(
      workload.batch() <= workload.capacity(),
      "{name}: RingConfig documents batch as at most the capacity",
    );
  }

  let cramped = cramped();
  assert_eq!(cramped.capacity(), 16);
  assert_eq!(
    cramped.batch(),
    16,
    "the fixture asks for 32 against 16 slots; the clamp is what this test exists to see",
  );
}

/// Both ordering subtractions are guarded unconditionally.
///
/// # Root Cause
///
/// `Outcome::dropped` and `Outcome::silently_discarded` were bare `usize`
/// subtractions. The ordering invariant argued that a violation would be loud
/// because release panics on underflow. That holds only where
/// `overflow-checks = true`, which this workspace sets nowhere. In release the
/// subtraction wrapped instead.
///
/// # Why Not Caught
///
/// The claim is about a build profile, not about the code, and the suite runs
/// in debug, where the claim happens to hold. No assertion could observe the
/// difference.
///
/// # Fix Applied
///
/// Both bodies now begin with an `assert!` on the ordering, so the panic is
/// unconditional. `Outcome`'s fields are private and the type has no
/// constructor outside `run`, so a test cannot build a violating value. That
/// is why this test checks the guards at the source level rather than by
/// provoking one.
///
/// # Prevention
///
/// When a guard cannot be reached from a test, assert that the guard is present
/// rather than asserting nothing.
///
/// # Pitfall
///
/// A safety argument that names a compiler behaviour is really naming a profile
/// setting; check that the setting is set.
#[test]
fn both_ordering_subtractions_are_guarded_unconditionally() {
  let source = include_str!("../src/lib.rs");

  for guard in [
    "assert!(self.received <= self.offered, \"received exceeded offered\");",
    "assert!(self.received <= self.reported, \"received exceeded reported\");",
  ] {
    assert!(source.contains(guard), "src/lib.rs no longer contains {guard}");
  }

  assert!(
    !source.contains("debug_assert!( self.received"),
    "a debug_assert here would reinstate exactly the profile dependence BN22 names",
  );

  let manifest = include_str!("../Cargo.toml");
  assert!(
    !manifest.contains("overflow-checks"),
    "if this crate ever sets overflow-checks, revisit whether the guards are still the honest mechanism",
  );
}

/// The subtraction feeding `record_drop` is guarded before it runs, matching
/// the two `Outcome` accessors already guarded.
///
/// # Root Cause
///
/// `run`'s own `stats.record_drop( ..., ( offered - received ) as u64 )`
/// computes the identical `offered - received` subtraction as
/// `Outcome::dropped`, but it runs *before* `Outcome` exists. The accessor
/// guard sits on the two accessor methods that expose the subtraction after the
/// fact, and cannot cover a computation that happens earlier on the same
/// values.
///
/// # Why Not Caught
///
/// The accessor fix and its test
/// (`both_ordering_subtractions_are_guarded_unconditionally`) scoped the search
/// to `Outcome`'s two accessor methods, the site the defect was found at,
/// without re-checking every other place the same expression appears in this
/// crate. The invariant holds for every real run (no test can construct a
/// violating `Outcome`, same as for the accessors), so nothing exercises either
/// call site at runtime. Only a source-level check catches the asymmetry.
///
/// # Fix Applied
///
/// `run` now asserts `received <= offered` immediately before the
/// `record_drop` call, with the same message as `Outcome::dropped`'s own
/// guard. A violation therefore panics at the earliest point the bad value
/// would otherwise flow into `RingStats`, instead of silently wrapping to a
/// near-`u64::MAX` stat.
///
/// # Prevention
///
/// When you add a guard for one accessor's derived reading, grep for every
/// other place the same raw expression appears. A shared computation can have
/// more than one call site, and a fix at the accessor does not retroactively
/// cover an earlier internal one.
///
/// # Pitfall
///
/// `Outcome`'s fields are private and `run` is the only constructor, so, as
/// with the accessor guards, no suite can build a violating value to make
/// either guard fire. This test pins presence and ordering, not triggering,
/// like `both_ordering_subtractions_are_guarded_unconditionally`.
#[test]
fn the_record_drop_input_is_guarded_before_the_subtraction_runs() {
  let source = include_str!("../src/lib.rs");

  let guard = "assert!(received <= offered, \"received exceeded offered\");";
  let call = "stats.record_drop(workload.config().overflow(), (offered - received) as u64);";

  let guard_pos = source
    .find(guard)
    .expect("src/lib.rs no longer guards the subtraction feeding record_drop");
  let call_pos = source
    .find(call)
    .expect("src/lib.rs no longer contains the record_drop call site");
  assert!(guard_pos < call_pos, "the guard must run before record_drop, not after");

  assert!(
    !source.contains("debug_assert!( received <= offered"),
    "a debug_assert here would reinstate exactly the profile dependence BN22 named for the other two guards",
  );
}

/// Both halves of the `Copy` coupling are named.
///
/// # Root Cause
///
/// `RunError` derives `Copy` and three of its variants wrap another crate's
/// error, so the derive silently pins `ring_factory::BuildError`,
/// `ring_types::RingError` and `ring_flush::ConfigError` to `Copy` as well.
/// `RingError` is `#[ non_exhaustive ]`, so adding a variant there is
/// explicitly sanctioned, and adding one that carries a `String` would break a
/// crate three hops away.
///
/// # Why Not Caught
///
/// Neither side recorded the coupling, and a derive that breaks reports itself
/// as an unsatisfied bound on an expanded impl, naming neither the reason nor
/// the crate that imposed it.
///
/// # Fix Applied
///
/// `RingError`'s docs now name this crate and this derive; `RunError`'s docs
/// name the three pinned types. This test states the conjunction as an explicit
/// bound so the break arrives with a test name attached.
///
/// # Prevention
///
/// When a derive on a wrapper constrains a type you do not own, say so on both
/// declarations and pin it with a bound something can fail.
///
/// # Pitfall
///
/// `#[ non_exhaustive ]` reserves the right to add variants; it does not
/// reserve the right to add one that breaks a derive.
#[test]
fn both_halves_of_the_copy_coupling_are_named() {
  const fn requires_copy<T: Copy>() {}

  requires_copy::<RunError>();
  requires_copy::<ring_factory::BuildError>();
  requires_copy::<ring_types::RingError>();
  requires_copy::<ring_flush::ConfigError>();

  for (label, source, needle) in [
    (
      "ring_types",
      include_str!("../../ring_types/src/error.rs"),
      "ring_bench::RunError",
    ),
    (
      "ring_bench",
      include_str!("../src/lib.rs"),
      "ring_flush::ConfigError` are all pinned `Copy`",
    ),
  ] {
    assert!(source.contains(needle), "{label} no longer records the coupling");
  }
}

/// The accumulator axis defaults to `Set` and one cell, and both are settable
/// independently of every other dimension.
///
/// `Set` is what every candidate measured before this axis existed, so a
/// `Workload` that never calls `with_semantics`/`with_cells` must keep
/// measuring exactly that. This is the axis's own backward-compatibility
/// guarantee, pinned as a test rather than left implicit in `new`'s body.
#[test]
fn the_accumulator_axis_defaults_to_set_and_one_cell() {
  let workload = roomy();
  assert_eq!(workload.semantics(), AccumulatorSemantics::Set);
  assert_eq!(workload.cells(), 1);

  let delta = workload.with_semantics(AccumulatorSemantics::Delta).with_cells(4).unwrap();
  assert_eq!(delta.semantics(), AccumulatorSemantics::Delta);
  assert_eq!(delta.cells(), 4);

  // Setting the new axis perturbs nothing the old one reads.
  assert_eq!(delta.producers(), workload.producers());
  assert_eq!(delta.records_per_producer(), workload.records_per_producer());
  assert_eq!(delta.batch(), workload.batch());
  assert_eq!(delta.capacity(), workload.capacity());
}

/// Every candidate agrees on the `Delta` table for the same workload.
///
/// This is the pass criterion of "byte-identical final tables", made concrete
/// at the one producer count every candidate (all four named paths, five or six
/// concrete variants) can run.
///
/// Every candidate must be measured under both accumulator semantics, not only
/// `Set`.
/// This is the `Delta` half at the widest eligible producer count. See
/// `delta_sums_correctly_across_many_producers_without_a_lock` for the
/// multi-producer case, which only the two unbounded candidates can run.
#[test]
fn every_candidate_agrees_on_the_delta_table_at_one_producer() {
  let workload = roomy().with_semantics(AccumulatorSemantics::Delta).with_cells(1).unwrap();

  for candidate in Candidate::ALL {
    let outcome = run(*candidate, &workload).unwrap();
    assert!(
      outcome.is_lossless(),
      "{} lost records before the table could be checked",
      candidate.name()
    );

    // Producer 0 is even, so every one of its 256 records carries delta +1.
    // Summed, that is 256. Delta sums regardless of the order the drain
    // produced them in, which is what "byte-identical" means here.
    assert_eq!(
      outcome.table().cells()[0],
      256_i64,
      "{} disagreed with the closed-form sum",
      candidate.name(),
    );
  }
}

/// `Set` keeps only the last write once a cell is shared, which is why `Delta`
/// is necessary rather than a stylistic alternative.
///
/// Same workload as
/// `every_candidate_agrees_on_the_delta_table_at_one_producer`, `Set` semantics
/// instead. Every one of producer 0's 256 records carries the same delta (`+1`,
/// since producer 0 is even), so `Set`'s last-write-wins overwrite lands on `1`
/// regardless of which of the 256 writes physically happened last. It does not
/// land on `256`, which is what arrived. `Set` is safe for idempotent
/// overwrites and unsafe the moment a destination accumulates more than one
/// write it needed to keep.
#[test]
fn set_semantics_keeps_only_the_last_write_when_one_cell_is_shared() {
  let workload = roomy().with_cells(1).unwrap();
  assert_eq!(
    workload.semantics(),
    AccumulatorSemantics::Set,
    "Set is the default this test relies on"
  );

  for candidate in Candidate::ALL {
    let outcome = run(*candidate, &workload).unwrap();
    assert!(outcome.is_lossless());
    assert_eq!(outcome.received(), 256);

    assert_eq!(
      outcome.table().cells()[0],
      1_i64,
      "{} kept more than the last write under Set — the axis stopped distinguishing",
      candidate.name(),
    );
    assert_ne!(
      outcome.table().cells()[0],
      outcome.received() as i64,
      "{} accidentally summed under Set, which is Delta's job",
      candidate.name(),
    );
  }
}

/// `Delta` sums correctly across many producers with no lock between them.
///
/// This is the measured case for contention among producers with no serializing
/// lock, and the correctness half of false sharing between cursors (the shared
/// producer cursor `DirectMpsc` claims through is the kind of hot, contended
/// atomic false sharing would sit on). This crate never asserts on timing; see
/// the module documentation. So this test asserts the property false sharing
/// would not corrupt even if present. The algorithm stays correct under
/// contention, and only its speed would suffer.
///
/// Five producers, split three-even (`+1` each) and two-odd (`-1` each). This
/// mirrors a fee-and-payout wallet, where the final balance is correct
/// regardless of which side's write physically lands last. Net delta is `+1`
/// per record, so 100 records per producer nets `100` regardless of how five
/// threads interleaved to produce it.
#[test]
fn delta_sums_correctly_across_many_producers_without_a_lock() {
  let workload = Workload::new(RingConfig::new(4096).unwrap())
    .with_producers(5)
    .unwrap()
    .with_records_per_producer(100)
    .unwrap()
    .with_semantics(AccumulatorSemantics::Delta)
    .with_cells(1)
    .unwrap();

  for candidate in [Candidate::MutexQueue, Candidate::DirectMpsc] {
    let outcome = run(candidate, &workload).unwrap();
    assert!(
      outcome.is_lossless(),
      "{} dropped records under a 4096-slot ring",
      candidate.name()
    );
    assert_eq!(outcome.offered(), 500);

    // 3 even producers (0, 2, 4) at +1 each, 2 odd (1, 3) at -1 each, 100
    // records apiece: (3 - 2) * 100 = 100. That is correct no matter which
    // producer's write the scheduler landed last.
    assert_eq!(
      outcome.table().cells()[0],
      100_i64,
      "{} produced a scheduling-dependent sum, which Delta exists to rule out",
      candidate.name(),
    );
  }
}

/// A drained record that decodes outside its producer range would be caught
/// instead of being folded silently into a total that only looks plausible.
///
/// The measured case for torn or overlapping reads. `destination_of` asserts,
/// unconditionally, that every drained record decodes to a producer inside
/// `0..producers()` before its delta is folded into any table. Every test in
/// this file that reads `outcome.table()` already exercises that guard on every
/// drained record, so a torn value would fail loudly there, not only here. This
/// test names the property and exercises it on the fixtures most likely to
/// stress it: heavy overflow (many records never land) and multiple producers
/// (many records interleave), crossed with both semantics.
#[test]
fn every_drained_record_decodes_to_a_producer_the_workload_describes() {
  let overflow = cramped().with_cells(1).unwrap();
  let contention = Workload::new(RingConfig::new(4096).unwrap())
    .with_producers(4)
    .unwrap()
    .with_records_per_producer(256)
    .unwrap()
    .with_cells(4)
    .unwrap();

  for semantics in [AccumulatorSemantics::Set, AccumulatorSemantics::Delta] {
    // No `unwrap_or_else`/panic handling is needed to prove the point. A torn
    // read would panic inside `run` itself, before `unwrap` ever sees a value,
    // so a run that returns at all already decoded every record it drained.
    let single_producer = overflow.with_semantics(semantics);
    for candidate in Candidate::ALL {
      let outcome = run(*candidate, &single_producer).unwrap();
      assert!(outcome.table().cells()[0].unsigned_abs() <= outcome.received() as u64);
    }

    let many_producers = contention.with_semantics(semantics);
    for candidate in [Candidate::MutexQueue, Candidate::DirectMpsc] {
      let outcome = run(candidate, &many_producers).unwrap();
      assert!(outcome.is_lossless());
      for cell in outcome.table().cells() {
        assert!(cell.unsigned_abs() <= outcome.received() as u64);
      }
    }
  }
}

/// The overflow gap never reaches the accumulator table.
///
/// The measured case for what a producer that outruns a consumer should do and
/// what a harness must not do with the records that get dropped. `ContractRing`
/// and `OffTheShelf` report `Ok` for 256 records into a 16-slot `DropNewest`
/// ring and keep 16, which is the "Ok is not kept" trap. If the table folded
/// `reported` instead of `received` it would count 256 phantom writes the ring
/// never took. It folds `received`, so the table is exactly as correct as the
/// drain that fed it, regardless of how the drop policy discarded the other
/// 240.
#[test]
fn the_overflow_gap_never_reaches_the_accumulator_table() {
  let workload = cramped().with_semantics(AccumulatorSemantics::Delta).with_cells(1).unwrap();

  let through_the_factory = run(Candidate::ContractRing, &workload).unwrap();
  assert_eq!(
    through_the_factory.reported(),
    256,
    "contract_ring still reports Ok for a discard"
  );
  assert_eq!(through_the_factory.received(), 16, "the ring holds 16");
  assert_eq!(
    through_the_factory.table().cells()[0],
    16_i64,
    "contract_ring folded the 240 phantom successes into the table",
  );

  #[cfg(feature = "crossbeam")]
  {
    let off_the_shelf = run(Candidate::OffTheShelf, &workload).unwrap();
    assert_eq!(off_the_shelf.reported(), 256, "off_the_shelf still reports Ok for a discard");
    assert_eq!(off_the_shelf.received(), 16, "the ring holds 16");
    assert_eq!(
      off_the_shelf.table().cells()[0],
      16_i64,
      "off_the_shelf folded the 240 phantom successes into the table",
    );
  }
}

/// A producer thread that panics while holding the mutex baseline's lock does
/// not poison every later locker, because `commit_batch`'s own lock now
/// recovers instead of propagating.
///
/// Root Cause: `commit_batch` (`src/lib.rs`) shares one
/// `Mutex< VecDeque< Record > >` across every producer thread
/// `run_mutex_queue` spawns, and used to acquire it with
/// `.expect( "no producer panics while holding the lock" )`.
/// `std::sync::Mutex` poisons on *any* panic while a guard is held, by *any*
/// thread. A single rare panic in one producer's critical section (an
/// allocator failure inside `push_back`, say) would have turned every other
/// producer's next `commit_batch` call into a panic too, for the rest of the
/// run.
///
/// Why Not Caught: `commit_batch` and the `queue : Mutex< VecDeque< Record > >`
/// it locks are both private to `ring_bench`'s `src/lib.rs`. Nothing in
/// `tests/` can call `commit_batch` or reach its `Mutex` directly, and no
/// existing test drives a panic through the public `Comparison`/`run`
/// API (doing so would require forcing an allocator failure, which is
/// not a reachable input from here). The suite's own rule against timing
/// assertions (see this file's `# What this suite deliberately never
/// asserts`) also means nothing here ever induced a panic mid-run to notice
/// the cascade.
///
/// Fix Applied: `src/lib.rs`'s `commit_batch` now takes the lock with
/// `.unwrap_or_else( std::sync::PoisonError::into_inner )`, which recovers the
/// stale-but-valid guard instead of panicking.
/// `ring_trace::Trace::entries_guard` already uses the same idiom for its own
/// shared log. This test cannot call the private function, so it reproduces
/// the same shape: one `Mutex< VecDeque< Record > >` (`Record` is
/// `ring_bench`'s own public alias, the same one `commit_batch` uses) shared
/// between a thread that locks-then-panics and a locker that runs after it.
/// That shows the idiom used at the fix site recovers instead of
/// propagating.
///
/// Prevention: the guarded critical section here, like the real
/// `commit_batch`, never runs caller-supplied code, only `VecDeque` methods
/// on a plain `u64`. So there is no half-established invariant for poisoning
/// to protect, and recovering is unconditionally safe for this shape of
/// lock.
///
/// Pitfall: a poisoned `std::sync::Mutex` stays poisoned forever after. Every
/// later `.lock()` from *any* thread fails, not just a retry from the thread
/// that panicked. So one rare panic in one producer's tiny critical section is
/// enough to take down every other producer's next commit for the rest of a
/// long-running benchmark. It is easy to miss because the critical section
/// itself (`len` then `push_back`) looks far too trivial to ever panic. The
/// risk is that *some* other holder, anywhere, might panic; this call's own
/// logic is not the risk.
#[test]
fn a_lock_poisoned_by_a_panicking_holder_recovers_instead_of_propagating() {
  use std::collections::VecDeque;
  use std::sync::Mutex;

  // Mirrors `commit_batch`'s own guarded type exactly. `ring_bench::Record`
  // is the same `u64` alias `commit_batch` stages records as.
  let queue: Mutex<VecDeque<ring_bench::Record>> = Mutex::new(VecDeque::new());
  queue.lock().unwrap().push_back(1);

  // Suppress the panic hook's stderr backtrace for the two intentional
  // panics below, as `ring_claim/tests/claim_test.rs` does around its own
  // forced unwind.
  let hook = std::panic::take_hook();
  std::panic::set_hook(Box::new(|_| {}));

  let queue_ref = &queue;
  let poisoned = std::thread::scope(|scope| {
    scope
      .spawn(move || {
        let _guard = queue_ref.lock().unwrap();
        panic!("simulated allocator failure inside the guarded critical section");
      })
      .join()
  });

  assert!(
    poisoned.is_err(),
    "the spawned holder must actually have panicked while locked"
  );
  assert!(
    queue.is_poisoned(),
    "a panic while holding the lock must poison it for every later locker"
  );

  // The bug: the crate's pre-fix `.expect( "no producer panics while holding
  // the lock" )` would panic here too. The first thread's unrelated panic
  // would cascade into this separate access.
  let old_pattern_would_panic = std::panic::catch_unwind(|| {
    drop(queue.lock().expect("no producer panics while holding the lock"));
  });
  assert!(
    old_pattern_would_panic.is_err(),
    "the pre-fix `.expect(...)` idiom must panic on a poisoned lock — this is the bug",
  );

  std::panic::set_hook(hook);

  // The fix: `.unwrap_or_else( PoisonError::into_inner )`, `commit_batch`'s
  // own idiom after this fix, recovers the stale-but-valid guard instead.
  let recovered = queue.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
  assert_eq!(
    recovered.len(),
    1,
    "the queue's prior state survives an unrelated sibling panic"
  );
}
