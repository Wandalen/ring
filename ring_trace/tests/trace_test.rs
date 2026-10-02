//! One entry per operation when enabled; zero when not.
//!
//! Claims the `ring_trace` clause of `docs/feature/185_ring_stats.md`. Its
//! acceptance criterion, filed at
//! `ring/bench_harness/docs/acceptance/001_feature_reached_tests.md`, reads
//! in part: "`ring_trace` records one entry per sequence operation when
//! enabled and zero when not."
//!
//! Two numbers, both literal. The `zero` half is the one that matters most.
//! The ring family's whole output is a measured comparison between candidate
//! ring implementations, and a trace that recorded anything at all when
//! switched off would put a lock and an allocation on the path being measured.
//!
//! `ring_stats/tests/stats_test.rs` claims the counter halves of the same
//! feature. The two crates are tested apart because they are not two
//! implementations of one thing. A counter answers "how many" in constant space
//! and is always on. A trace answers "which, in what order" and is off by
//! default because it is not.

use std::thread;

use ring_trace::{Trace, TraceEntry, TraceOp};
use ring_types::Seq;

// -------------------------------------------------------- the two numbers

#[test]
fn an_enabled_trace_records_exactly_one_entry_per_operation() {
  let trace = Trace::enabled();

  for i in 0..50u64 {
    trace.record(TraceOp::Publish, Seq(i), 1);
  }

  // Fix(weak_len_assert_sweep_1633): both assertions here are counts of the
  // same underlying `Vec` (`len()` is a field read, `entries().len()` reads
  // the cloned log), so together they still prove nothing about which 50
  // entries resulted. A log of 50 copies of `Seq(0)` would pass identically to
  // the 50 distinct sequences the test's own name promises. Root cause:
  // count-only assertions on `trace`/`trace.entries()`. Fix: pin the whole
  // entry log against `record`'s own behavior (`entries_guard().push(...)`,
  // no coalescing), proving each of the 50 calls produced its own entry for
  // its own sequence, in order. Prevention: "one entry per operation" must
  // check that each operation's own entry survived, not only that the tally
  // matches the call count.
  assert_eq!(trace.len(), 50, "50 operations, 50 entries");
  assert_eq!(trace.entries().len(), 50);
  let expected: Vec<TraceEntry> = (0..50u64)
    .map(|i| TraceEntry {
      op: TraceOp::Publish,
      seq: Seq(i),
      count: 1,
    })
    .collect();
  assert_eq!(
    trace.entries(),
    expected,
    "each of the 50 operations must produce its own entry, for its own sequence, in order"
  );
}

#[test]
fn a_disabled_trace_records_zero() {
  let trace = Trace::disabled();

  for i in 0..50u64 {
    trace.record(TraceOp::Publish, Seq(i), 1);
  }

  assert_eq!(trace.len(), 0, "disabled means zero, not few");
  assert!(trace.is_empty());
  assert!(trace.entries().is_empty());
}

#[test]
fn a_disabled_trace_records_zero_of_every_operation_kind() {
  // Exhaustive over the discriminants, so a kind added later without a
  // corresponding early-return cannot slip through.
  let trace = Trace::disabled();

  for op in TraceOp::ALL {
    trace.record(op, Seq(0), 1);
    assert_eq!(trace.count_of(op), 0, "{op} was recorded despite the trace being off");
  }
  assert_eq!(trace.len(), 0);
}

#[test]
fn an_enabled_trace_records_one_of_every_operation_kind() {
  let trace = Trace::enabled();

  for op in TraceOp::ALL {
    trace.record(op, Seq(0), 1);
  }

  assert_eq!(trace.len(), TraceOp::ALL.len());
  for op in TraceOp::ALL {
    assert_eq!(trace.count_of(op), 1);
  }
}

#[test]
fn the_default_trace_is_off() {
  // A ring nobody asked to trace must not be tracing. The default is the whole
  // reason the disabled path is the one that has to be free.
  let trace = Trace::default();

  assert!(!trace.is_enabled());
  trace.record(TraceOp::Claim, Seq(0), 1);
  assert_eq!(trace.len(), 0);
}

#[test]
fn enabled_and_disabled_report_their_own_state() {
  assert!(Trace::enabled().is_enabled());
  assert!(!Trace::disabled().is_enabled());
}

// ------------------------------------------------------------ what an entry says

#[test]
fn entries_come_back_in_the_order_recorded() {
  let trace = Trace::enabled();
  trace.record(TraceOp::Claim, Seq(0), 4);
  trace.record(TraceOp::Publish, Seq(0), 1);
  trace.record(TraceOp::Consume, Seq(0), 1);
  trace.record(TraceOp::Commit, Seq(1), 1);

  let ops: Vec<TraceOp> = trace.entries().iter().map(|e| e.op).collect();
  assert_eq!(
    ops,
    vec![TraceOp::Claim, TraceOp::Publish, TraceOp::Consume, TraceOp::Commit],
    "order is the only thing a trace has that a counter does not"
  );
}

#[test]
fn a_batch_is_one_entry_carrying_its_count_not_n_entries() {
  // A trace that expanded a batch claim into 64 entries would contradict the
  // thing it is evidence of, feature 177's "one operation, not 64".
  let trace = Trace::enabled();
  trace.record(TraceOp::Claim, Seq(8), 64);

  let entries = trace.entries();
  assert_eq!(entries.len(), 1, "one operation, one entry");
  assert_eq!(entries[0].count, 64);
  assert_eq!(entries[0].seq, Seq(8));
  assert_eq!(entries[0].end(), Seq(72));
}

#[test]
fn an_entry_reports_the_range_it_covers() {
  let entry = TraceEntry {
    op: TraceOp::Publish,
    seq: Seq(5),
    count: 3,
  };
  assert_eq!(entry.end(), Seq(8));

  let empty = TraceEntry {
    op: TraceOp::Publish,
    seq: Seq(5),
    count: 0,
  };
  assert_eq!(empty.end(), Seq(5), "a zero-count operation covers nothing");
}

#[test]
fn entries_compare_by_value_and_print_readably() {
  let a = TraceEntry {
    op: TraceOp::Drop,
    seq: Seq(1),
    count: 1,
  };
  let b = TraceEntry {
    op: TraceOp::Drop,
    seq: Seq(1),
    count: 1,
  };
  let c = TraceEntry {
    op: TraceOp::Claim,
    seq: Seq(1),
    count: 1,
  };

  assert_eq!(a, b);
  assert_ne!(a, c);
  assert_eq!(a.to_string(), "drop 1..2");
  assert_eq!(
    TraceEntry {
      op: TraceOp::Claim,
      seq: Seq(8),
      count: 64
    }
    .to_string(),
    "claim 8..72"
  );
  assert!(format!("{a:?}").contains("Drop"));
}

#[test]
fn every_operation_kind_has_its_own_name() {
  let names: Vec<&str> = TraceOp::ALL.iter().map(|op| op.name()).collect();

  assert_eq!(names, vec!["claim", "publish", "consume", "commit", "drop"]);

  let mut unique = names.clone();
  unique.sort_unstable();
  unique.dedup();
  assert_eq!(unique.len(), names.len(), "two kinds sharing a name would be unreadable");
}

#[test]
fn an_operation_prints_as_its_name() {
  for op in TraceOp::ALL {
    assert_eq!(op.to_string(), op.name());
  }
}

#[test]
fn the_operation_kinds_are_exactly_the_five_declared() {
  // ALL must stay in step with the enum, but nothing here forces that. The
  // exhaustive match below fails to compile when a discriminant is *added* to
  // the enum, a guard `name()` in src/lib.rs already provides one compile
  // error earlier. The match says nothing about whether the new variant was
  // also added to ALL. Keeping ALL in step with the enum is a manual step, and
  // this test's exhaustiveness is a (harmless, redundant) copy of name()'s guard.
  fn covered(op: TraceOp) -> bool {
    match op {
      TraceOp::Claim | TraceOp::Publish | TraceOp::Consume | TraceOp::Commit | TraceOp::Drop => true,
    }
  }

  assert_eq!(TraceOp::ALL.len(), 5);
  for op in TraceOp::ALL {
    assert!(covered(op));
  }

  let mut unique = TraceOp::ALL.to_vec();
  unique.sort_unstable_by_key(|op| op.name());
  unique.dedup();
  assert_eq!(unique.len(), 5, "ALL must not repeat a kind");
}

// -------------------------------------------------------------------- counting

#[test]
fn count_of_counts_only_its_own_kind() {
  let trace = Trace::enabled();
  for _ in 0..3 {
    trace.record(TraceOp::Claim, Seq(0), 1);
  }
  for _ in 0..7 {
    trace.record(TraceOp::Drop, Seq(0), 1);
  }

  assert_eq!(trace.count_of(TraceOp::Claim), 3);
  assert_eq!(trace.count_of(TraceOp::Drop), 7);
  assert_eq!(trace.count_of(TraceOp::Publish), 0);
  assert_eq!(trace.len(), 10);
}

#[test]
fn the_per_kind_counts_sum_to_the_total() {
  let trace = Trace::enabled();
  for (i, op) in TraceOp::ALL.iter().enumerate() {
    for _ in 0..=i {
      trace.record(*op, Seq(0), 1);
    }
  }

  let summed: usize = TraceOp::ALL.iter().map(|op| trace.count_of(*op)).sum();
  assert_eq!(summed, trace.len(), "no entry is unaccounted for or double-counted");
  assert_eq!(summed, 1 + 2 + 3 + 4 + 5);
}

// -------------------------------------------------------------------- clearing

#[test]
fn clearing_empties_the_log_without_switching_it_off() {
  let mut trace = Trace::enabled();
  trace.record(TraceOp::Commit, Seq(0), 1);
  assert!(!trace.is_empty(), "nothing was recorded, so clearing proves nothing");
  trace.clear();

  assert!(trace.is_empty());
  assert!(trace.is_enabled(), "clearing is not disabling");

  trace.record(TraceOp::Commit, Seq(1), 1);
  assert_eq!(trace.len(), 1, "and recording resumes");
}

#[test]
fn clearing_a_disabled_trace_is_harmless() {
  let mut trace = Trace::disabled();
  trace.clear();
  trace.record(TraceOp::Claim, Seq(0), 1);
  trace.clear();

  assert!(trace.is_empty());
  assert!(!trace.is_enabled());
}

// ----------------------------------------------------------------- contention

#[test]
fn concurrent_recorders_lose_no_entry() {
  // A trace shared across producers must not drop an entry under contention.
  // A log with a hole in it reads exactly like a run where the operation never
  // happened, which is the one misreading a diagnostic must not invite.
  const THREADS: usize = 4;
  const EACH: usize = 2_000;

  let trace = Trace::enabled();
  let trace = &trace;

  thread::scope(|scope| {
    for id in 0..THREADS {
      scope.spawn(move || {
        for i in 0..EACH {
          trace.record(TraceOp::Publish, Seq((id * EACH + i) as u64), 1);
        }
      });
    }
  });

  assert_eq!(trace.len(), THREADS * EACH);

  let mut seqs: Vec<u64> = trace.entries().iter().map(|e| e.seq.0).collect();
  seqs.sort_unstable();
  assert_eq!(seqs, (0..(THREADS * EACH) as u64).collect::<Vec<_>>());
}

#[test]
fn a_disabled_trace_stays_empty_under_contention() {
  const THREADS: usize = 4;
  const EACH: usize = 2_000;

  let trace = Trace::disabled();
  let trace = &trace;

  thread::scope(|scope| {
    for _ in 0..THREADS {
      scope.spawn(move || {
        for i in 0..EACH {
          trace.record(TraceOp::Publish, Seq(i as u64), 1);
        }
      });
    }
  });

  assert_eq!(trace.len(), 0, "8000 calls, zero entries");
}

// -------------------------------------------------------------------- TR41

#[test]
fn end_saturates_instead_of_reading_backwards_at_the_top_of_u64() {
  // `ring_mpsc::UNSTAMPED` is `Seq( u64::MAX )`, the one sequence the family
  // publishes by name. `end()` used to compute this with a bare `+`, which
  // wraps to `0` in release and panics under debug assertions. Either way the
  // rendered line misleads or kills the diagnostic reading it (`pitfall/001`
  // TR41).
  let entry = TraceEntry {
    op: TraceOp::Publish,
    seq: Seq(u64::MAX),
    count: 1,
  };
  assert_eq!(entry.end(), Seq(u64::MAX), "saturates rather than wrapping past the top");
  assert_eq!(entry.to_string(), format!("publish {}..{}", u64::MAX, u64::MAX));
}
