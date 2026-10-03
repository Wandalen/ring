//! Tests for `ring_poll`, which gives bounded, non-parking progress on the tick path.
//!
//! # What this file is arranged around
//!
//! This file carries the reached-test for
//! `docs/feature/183_try_only_operations_on_the_tick_path.md`, whose claim is
//! *structural*. The parking operations are not reachable from where they would
//! deadlock. A claim about reachability cannot be tested by calling something,
//! because there is nothing to call. So
//! [`the_tick_path_cannot_reach_a_parking_operation`] asserts it the only way it
//! is assertable, against the dependency graph, and
//! [`a_large_budget_spins_rather_than_sleeping`] asserts the behavioural half,
//! that what *is* reachable costs spin-time rather than sleep-time.
//!
//! The feature also constrains `ring_handle` without being claimed by it.
//! Nothing reachable from a handle may park. This file enforces that constraint
//! by name, because `ring_handle`'s own suite would stay green if it broke.
//!
//! # What is deliberately not here
//!
//! **No test that a parking call fails to compile.** It would not fail to
//! compile; it would fail to *resolve*, because the crate is not a dependency,
//! and a test cannot name a crate it cannot see. The manifest scan below is the
//! honest form of that assertion, and `tests/manual/readme.md` P1 records what
//! the compiler says when the dependency is added back.
//!
//! **No multi-threaded drain race.** A budget bounds attempts, not time, and a
//! two-thread test that observes "it returned" observes what the single-thread
//! test already established. What multiple threads would add is a *rate*
//! measurement, which belongs to `ring_bench` at S8.

#![cfg(test)]
// Ordinary tests, compiled out under `--cfg loom`. That cfg swaps
// `ring_atomic`'s atomics for loom's instrumented ones across the whole
// family, and those panic the moment they are touched outside a
// `loom::model` closure. Without this gate a family-wide loom run dies
// here instead of reaching the models in `ring_spsc`, `ring_mpsc`,
// `ring_publish` and `ring_testkit`.
#![cfg(not(loom))]

use std::{
  fs,
  path::Path,
  time::{Duration, Instant},
};

use ring_config::RingConfig;
use ring_core::Ring;
use ring_poll::{Budget, PARKING_CRATES, Progress, Tick, drain_up_to, push_batch_within, push_within, recv_within};
use ring_types::OverflowPolicy;

/// A ring of `slots` capacity, in the default (SPSC, `Fail`) configuration.
fn ring(slots: usize) -> Ring<u32> {
  Ring::new(&RingConfig::new(slots).unwrap()).unwrap()
}

/// A ring of `slots` capacity that refuses rather than dropping.
///
/// `Fail` is the default, so this pins what [`ring`] already does. Every test
/// below that is *about* running out of budget uses this, so it keeps running
/// out whatever the default is.
fn refusing_ring(slots: usize) -> Ring<u32> {
  let config = RingConfig::new(slots).unwrap().with_overflow(OverflowPolicy::Fail);
  Ring::new(&config).unwrap()
}

/// A ring of `slots` capacity that discards the incoming record when full.
///
/// Under `DropNewest` a full ring reports `Ok` and discards the record. That
/// opt-in is still tested, by
/// [`push_within_under_drop_newest_reports_success_and_keeps_nothing`].
fn dropping_ring(slots: usize) -> Ring<u32> {
  let config = RingConfig::new(slots).unwrap().with_overflow(OverflowPolicy::DropNewest);
  Ring::new(&config).unwrap()
}

/// Counting records, `size` at a time, with one `None` after each burst.
///
/// `Iterator` permits a `Some` after a `None`, and `try_push_batch` stops at the
/// first `None`. So against a ring with room, each batch attempt moves exactly
/// one burst, and the count published reveals how many attempts ran.
fn bursts_of(size: u32) -> impl Iterator<Item = u32> {
  let mut next = 0;
  let mut in_burst = 0;
  std::iter::from_fn(move || {
    if in_burst == size {
      in_burst = 0;
      return None;
    }
    in_burst += 1;
    next += 1;
    Some(next - 1)
  })
}

// ── The reached-test ──────────────────────────────────────────────────────

/// The feature's reached-test asserts that the crates from which a parking
/// operation is reachable are exactly the three declared, and that no tick-path
/// crate is among them.
///
/// The assertion runs against the manifests on disk rather than against
/// [`PARKING_CRATES`] alone, so the roster cannot drift away from the graph it
/// describes. Adding `ring_wait` to `ring_handle` fails here. That is the exact
/// mistake the feature exists to prevent, and `ring_handle`'s own green suite
/// would not notice it.
#[test]
fn the_tick_path_cannot_reach_a_parking_operation() {
  let family = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();

  let mut measured: Vec<String> = Vec::new();
  for entry in fs::read_dir(family).unwrap() {
    let crate_dir = entry.unwrap().path();
    let name = crate_dir.file_name().unwrap().to_string_lossy().to_string();
    if !name.starts_with("ring_") {
      continue;
    }
    let manifest = crate_dir.join("Cargo.toml");
    if !manifest.exists() {
      continue;
    }
    if fs::read_to_string(&manifest).unwrap().contains("ring_wait") {
      measured.push(name);
    }
  }
  measured.sort();

  assert_eq!(
    measured, PARKING_CRATES,
    "the crates reaching a parking operation drifted from the declared roster"
  );

  for tick_path in ["ring_poll", "ring_handle", "ring_core"] {
    assert!(
      !measured.iter().any(|name| name == tick_path),
      "{tick_path} is on the tick path and must not reach a parking operation"
    );
  }
}

/// The roster's guard reads direct declarations, and reachability is wider.
///
/// `the_tick_path_cannot_reach_a_parking_operation` scans each `Cargo.toml` for
/// the string `ring_wait`, so it sees the three crates that declare it and
/// cannot see a crate that reaches it through an intermediate. Two already do:
/// `ring_consume` through `ring_barrier`, and `ring_testkit` through
/// `ring_shutdown`. This walks the `[dependencies]` graph to its closure and
/// asserts both sets, so the gap between them is a measured quantity rather
/// than an unexamined one. A *new* transitive path, which the other test would
/// pass straight through, fails here.
///
/// This test excludes `[dev-dependencies]` on purpose. `ring_publish` reaches
/// `ring_wait` that way, and a dev-dependency is not on any caller's tick path.
#[test]
fn the_transitive_reach_is_wider_than_the_manifest_scan_can_see() {
  let family = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();

  let mut direct: Vec<(String, Vec<String>)> = Vec::new();
  for entry in fs::read_dir(family).unwrap() {
    let crate_dir = entry.unwrap().path();
    let name = crate_dir.file_name().unwrap().to_string_lossy().to_string();
    if !name.starts_with("ring_") {
      continue;
    }
    let manifest = crate_dir.join("Cargo.toml");
    if !manifest.exists() {
      continue;
    }
    let text = fs::read_to_string(&manifest).unwrap();
    // The `[dependencies]` table only, up to whatever table follows it.
    let mut deps = Vec::new();
    let mut inside = false;
    for line in text.lines() {
      let trimmed = line.trim();
      if trimmed.starts_with('[') {
        inside = trimmed == "[dependencies]";
        continue;
      }
      if !inside {
        continue;
      }
      if let Some(key) = trimmed.split([' ', '=']).next()
        && key.starts_with("ring_")
      {
        deps.push(key.to_string());
      }
    }
    direct.push((name, deps));
  }

  let declares_directly = |who: &str| -> bool { direct.iter().any(|(n, d)| n == who && d.iter().any(|x| x == "ring_wait")) };

  // In the closure, a crate reaches `ring_wait` if it *is* `ring_wait`, declares
  // it, or declares something that reaches it. The graph is small and acyclic;
  // iterate to a fixed point rather than assuming a depth.
  let mut reaches: Vec<String> = direct
    .iter()
    .filter(|(n, _)| n == "ring_wait" || declares_directly(n))
    .map(|(n, _)| n.clone())
    .collect();
  loop {
    let before = reaches.len();
    for (name, deps) in &direct {
      if reaches.iter().any(|n| n == name) {
        continue;
      }
      if deps.iter().any(|d| reaches.iter().any(|n| n == d)) {
        reaches.push(name.clone());
      }
    }
    if reaches.len() == before {
      break;
    }
  }
  reaches.sort();

  // What the string scan finds, reconstructed from the parsed graph: the crates
  // that declare `ring_wait`, plus `ring_wait` itself. The scan matches
  // `ring_wait` only because its own `name =` line contains the string it
  // searches for, not because of any edge.
  let mut declared: Vec<String> = direct
    .iter()
    .filter(|(n, _)| n == "ring_wait" || declares_directly(n))
    .map(|(n, _)| n.clone())
    .collect();
  declared.sort();

  assert_eq!(
    declared, PARKING_CRATES,
    "the direct declarations drifted from the declared roster"
  );
  assert_eq!(
    reaches,
    ["ring_barrier", "ring_consume", "ring_shutdown", "ring_testkit", "ring_wait"],
    "the transitive reach drifted; the roster's doc comment names the two extras"
  );
  assert!(
    reaches.len() > declared.len(),
    "if these ever matched, the roster's doc comment about the gap is wrong"
  );
  for tick_path in ["ring_poll", "ring_handle", "ring_core"] {
    assert!(
      !reaches.iter().any(|name| name == tick_path),
      "{tick_path} is on the tick path and must not reach a parking operation, \
       even transitively"
    );
  }
}

/// This crate's own manifest, baked into the test binary at compile time, names
/// no parking crate.
///
/// It uses `include_str!` rather than a read, so a manifest that changed after
/// the binary was built cannot satisfy the assertion.
#[test]
fn this_crate_declares_no_parking_dependency() {
  let manifest = include_str!("../Cargo.toml");
  for parking in PARKING_CRATES {
    assert!(
      !manifest.contains(parking),
      "ring_poll's manifest names {parking}, which can park"
    );
  }
}

/// A large budget costs spin-time, not sleep-time.
///
/// The arithmetic is the assertion. `ring_wait`'s `Park` variant sleeps 50µs
/// per attempt, so 20 000 attempts through a parking implementation would cost
/// about **1.0 s**. Through a spin hint the same 20 000 attempts cost single-
/// digit milliseconds. The 500 ms bound sits 2× under the parking cost and
/// roughly 100× over the spinning one. That gap is what lets the bound tell
/// spinning from parking.
#[test]
fn a_large_budget_spins_rather_than_sleeping() {
  let mut ring = refusing_ring(4);
  let mut ends = ring.ends();
  let (mut producer, _consumer) = ends.split();
  assert_eq!(push_batch_within(&mut producer, &mut (0..4), Budget::once()), Ok(4));

  let started = Instant::now();
  let refused = push_within(&mut producer, 99, Budget::new(20_000));
  let elapsed = started.elapsed();

  assert_eq!(refused, Err(99), "the ring never drained, so the budget ran out");
  assert!(
    elapsed < Duration::from_millis(500),
    "20 000 attempts took {elapsed:?} — a sleeping pause would cost about 1.0 s"
  );
}

// ── Budget ────────────────────────────────────────────────────────────────

/// Zero attempts is clamped to one rather than accepted.
#[test]
fn a_budget_is_at_least_one_attempt() {
  assert_eq!(Budget::new(0).attempts(), 1);
  assert_eq!(Budget::new(1).attempts(), 1);
  assert_eq!(Budget::new(7).attempts(), 7);
  assert_eq!(Budget::once().attempts(), 1);
}

/// The tick-path default is a single attempt, and the derives behave.
#[test]
fn a_budget_defaults_to_a_single_attempt() {
  assert_eq!(Budget::default(), Budget::once());
  assert!(Budget::new(2) > Budget::once(), "budgets order by attempt count");
  assert!(format!("{:?}", Budget::once()).contains('1'));

  let mut seen = std::collections::HashSet::new();
  assert!(seen.insert(Budget::once()));
  assert!(!seen.insert(Budget::new(1)), "equal budgets hash together");
}

// ── Progress ──────────────────────────────────────────────────────────────

/// A count of zero is `None`, so `Made( 0 )` is never constructed.
#[test]
fn progress_of_zero_is_no_progress() {
  assert_eq!(Progress::of(0), Progress::None);
  assert!(!Progress::of(0).is_made());
  assert_eq!(Progress::of(0).count(), 0);

  assert_eq!(Progress::of(4), Progress::Made(4));
  assert!(Progress::of(4).is_made());
  assert_eq!(Progress::of(4).count(), 4);
  assert!(format!("{:?}", Progress::of(4)).contains("Made"));
}

/// Two steps of one tick combine by summing, and stay `None` when neither moved.
#[test]
fn progress_sums_across_the_steps_of_one_tick() {
  assert_eq!(Progress::of(2).then(Progress::of(3)), Progress::Made(5));
  assert_eq!(Progress::None.then(Progress::of(1)), Progress::Made(1));
  assert_eq!(Progress::None.then(Progress::None), Progress::None);
}

/// `then` must never report `None` when the true combined count is
/// astronomically large, which is the exact false negative [`Progress`] exists
/// to rule out.
///
/// Root Cause: `then` combined the two counts with plain `self.count() +
/// other.count()`, and [`Progress::of`] is a public, unconstrained `usize`
/// constructor, so a caller combining `usize::MAX` progress with any more
/// panicked in a debug build. In release it silently wrapped to `0`, and
/// `Self::of( 0 )` is `Progress::None`.
///
/// Why Not Caught: every existing test summed small literal counts
/// (`progress_sums_across_the_steps_of_one_tick` uses `2`, `3`, `1`). The
/// crate's one previously-known raw-`+=` overflow risk (`Tick::moved`) is a
/// private field reachable only through real ring traffic, and was dismissed
/// as impractical to reach. That holds for `Tick::moved` but not for
/// `Progress::of`, a public constructor one direct call away from the same
/// class of bug with no ring involved at all.
///
/// Fix Applied: `then` now composes with `saturating_add` instead of `+`, so
/// the sum caps at `usize::MAX`. That is still `Progress::Made`, never a
/// spurious `Progress::None`.
///
/// Prevention: a type whose entire purpose is ruling out a false-negative
/// reading must saturate rather than wrap (or panic) on its own composition.
/// Crashing the tick path is as unusable there as the false reading would be.
///
/// Pitfall: a public, unconstrained constructor over a raw integer
/// (`Progress::of`) puts arithmetic on its output outside any bound the
/// defining crate controls; audit every site that combines two of its values,
/// not only the crate's own internal counters.
#[test]
fn then_saturates_instead_of_reporting_false_no_progress() {
  let huge = Progress::of(usize::MAX);
  let one = Progress::of(1);

  assert_eq!(huge.then(one), Progress::Made(usize::MAX), "must saturate, not wrap to None");
  assert!(
    huge.then(one).is_made(),
    "an enormous amount of progress must never read as none"
  );
}

// ── push_within ───────────────────────────────────────────────────────────

/// Room on the first attempt is the ordinary case.
#[test]
fn push_within_publishes_when_there_is_room() {
  let mut ring = ring(4);
  let mut ends = ring.ends();
  let (mut producer, mut consumer) = ends.split();

  assert_eq!(push_within(&mut producer, 5, Budget::once()), Ok(()));
  assert_eq!(consumer.try_recv(), Some(5));
}

/// The record comes back rather than being lost, so the caller still has a choice.
#[test]
fn push_within_returns_the_record_when_the_budget_runs_out() {
  let mut ring = refusing_ring(2);
  let mut ends = ring.ends();
  let (mut producer, _consumer) = ends.split();
  producer.try_push(1).unwrap();
  producer.try_push(2).unwrap();

  assert_eq!(push_within(&mut producer, 3, Budget::once()), Err(3));
}

/// A budget above one spins between attempts and still gives the record back.
#[test]
fn a_multi_attempt_budget_spends_every_attempt_before_giving_up() {
  let mut ring = refusing_ring(2);
  let mut ends = ring.ends();
  let (mut producer, mut consumer) = ends.split();
  producer.try_push(1).unwrap();
  producer.try_push(2).unwrap();

  assert_eq!(push_within(&mut producer, 3, Budget::new(4)), Err(3));

  // Once there is room, the same budget succeeds, so the failure above was
  // back-pressure and not a broken retry loop.
  assert_eq!(consumer.try_recv(), Some(1));
  assert_eq!(push_within(&mut producer, 3, Budget::new(4)), Ok(()));
}

/// Under `DropNewest` a full push reports success and keeps nothing.
///
/// This is `OverflowPolicy::DropNewest`'s behaviour, and every helper here passes
/// it through. It was first found in `ring_shutdown`, whose `Refusal::Full` arm
/// is unreachable for the same reason.
#[test]
fn push_within_under_drop_newest_reports_success_and_keeps_nothing() {
  let mut ring = dropping_ring(2);
  let mut ends = ring.ends();
  let (mut producer, mut consumer) = ends.split();
  producer.try_push(1).unwrap();
  producer.try_push(2).unwrap();

  assert_eq!(
    push_within(&mut producer, 3, Budget::once()),
    Ok(()),
    "drop-newest reports success"
  );

  let mut out = Vec::new();
  drain_up_to(&mut consumer, &mut out, 8);
  assert_eq!(out, vec![1, 2], "and 3 was never kept");
}

// ── push_batch_within ─────────────────────────────────────────────────────

/// An iterator that fits goes in on the first attempt.
#[test]
fn push_batch_within_publishes_the_whole_iterator_when_it_fits() {
  let mut ring = ring(8);
  let mut ends = ring.ends();
  let (mut producer, consumer) = ends.split();

  assert_eq!(push_batch_within(&mut producer, &mut (0..5), Budget::once()), Ok(5));
  assert_eq!(consumer.len(), 5);
}

/// An attempt that moves nothing ends the loop, whatever the budget says, and
/// the record it was refused comes back.
#[test]
fn push_batch_within_stops_at_the_first_attempt_that_moves_nothing() {
  let mut ring = refusing_ring(2);
  let mut ends = ring.ends();
  let (mut producer, _consumer) = ends.split();
  producer.try_push(1).unwrap();
  producer.try_push(2).unwrap();

  let mut records = 0..4;
  assert_eq!(push_batch_within(&mut producer, &mut records, Budget::new(9)), Err((0, 0)));
  assert_eq!(records.next(), Some(1), "only the refused record left the iterator");
}

/// Every productive attempt runs until the budget is spent, and no more.
///
/// The source yields one burst per attempt, so the published count is the
/// number of attempts times the burst: three attempts of two records each.
/// One attempt too many publishes eight, stopping on the productive attempt
/// publishes two, and a counter that never advances runs until the ring is
/// full at sixteen. The ring refuses rather than drops, which is what ends that
/// last loop instead of leaving it to spin.
#[test]
fn push_batch_within_spends_its_whole_budget_while_attempts_keep_moving_records() {
  let mut ring = refusing_ring(16);
  let mut ends = ring.ends();
  let (mut producer, consumer) = ends.split();

  let mut records = bursts_of(2);
  assert_eq!(
    push_batch_within(&mut producer, &mut records, Budget::new(3)),
    Ok(6),
    "three attempts, one burst of two each",
  );
  assert_eq!(consumer.len(), 6);
  assert_eq!(records.next(), Some(6), "and nothing was taken past the third burst");
}

/// A retry offers the record the last attempt was refused, not a new one.
///
/// Against a full ring a bigger budget buys more refusals and nothing else: the
/// held record is offered again, refused again, and handed back once the loop
/// ends. Whatever the budget, the caller gets record 4 back and the iterator
/// still yields 5.
#[test]
fn push_batch_within_hands_back_the_held_record_whatever_the_budget() {
  for attempts in [1, 3] {
    let mut ring = refusing_ring(4);
    let mut ends = ring.ends();
    let (mut producer, consumer) = ends.split();

    let mut records = 0..12;
    assert_eq!(
      push_batch_within(&mut producer, &mut records, Budget::new(attempts)),
      Err((4, 4)),
      "budget {attempts}: 0-3 published, 4 handed back",
    );
    assert_eq!(records.next(), Some(5), "budget {attempts}: 5 is still pending");
    assert_eq!(consumer.len(), 4);
  }
}

// ── recv_within ───────────────────────────────────────────────────────────

/// A record that is there comes back on the first attempt.
#[test]
fn recv_within_takes_a_record_that_is_there() {
  let mut ring = ring(4);
  let mut ends = ring.ends();
  let (mut producer, mut consumer) = ends.split();
  producer.try_push(8).unwrap();

  assert_eq!(recv_within(&mut consumer, Budget::once()), Some(8));
}

/// An empty ring answers `None` once the budget is spent, having spun between.
#[test]
fn recv_within_gives_up_on_an_empty_ring() {
  let mut ring = ring(4);
  let mut ends = ring.ends();
  let (_producer, mut consumer) = ends.split();

  assert_eq!(recv_within(&mut consumer, Budget::new(3)), None);
}

// ── drain_up_to ───────────────────────────────────────────────────────────

/// The ceiling holds even when more is available.
#[test]
fn drain_up_to_stops_at_the_limit() {
  let mut ring = ring(8);
  let mut ends = ring.ends();
  let (mut producer, mut consumer) = ends.split();
  assert_eq!(producer.try_push_batch(&mut (0..5)), Ok(5), "the arrange filled the ring");

  let mut out = Vec::new();
  assert_eq!(drain_up_to(&mut consumer, &mut out, 3), 3);
  assert_eq!(out, vec![0, 1, 2]);
  assert_eq!(consumer.len(), 2, "the rest is still there");
}

/// An empty read ends the drain before the ceiling is reached.
#[test]
fn drain_up_to_stops_when_the_ring_empties_first() {
  let mut ring = ring(8);
  let mut ends = ring.ends();
  let (mut producer, mut consumer) = ends.split();
  assert_eq!(producer.try_push_batch(&mut (0..2)), Ok(2), "the arrange filled the ring");

  let mut out = Vec::new();
  assert_eq!(drain_up_to(&mut consumer, &mut out, 10), 2);
  assert!(consumer.is_empty());
}

/// A ceiling of zero reads nothing at all.
#[test]
fn drain_up_to_zero_touches_nothing() {
  let mut ring = ring(8);
  let mut ends = ring.ends();
  let (mut producer, mut consumer) = ends.split();
  assert_eq!(producer.try_push_batch(&mut (0..3)), Ok(3), "the arrange filled the ring");

  let mut out = Vec::new();
  assert_eq!(drain_up_to(&mut consumer, &mut out, 0), 0);
  assert!(out.is_empty());
  assert_eq!(consumer.len(), 3, "the ring was not touched");
}

// ── Tick ──────────────────────────────────────────────────────────────────

/// Every operation on a tick adds to the same count.
#[test]
fn a_tick_accumulates_across_its_operations() {
  let mut ring = ring(16);
  let mut ends = ring.ends();
  let (mut producer, mut consumer) = ends.split();

  let mut tick = Tick::new(Budget::new(2));
  assert_eq!(tick.budget(), Budget::new(2));
  assert_eq!(tick.progress(), Progress::None, "nothing has moved yet");

  tick.push(&mut producer, 1).unwrap();
  assert_eq!(tick.push_batch(&mut producer, &mut (2..6)), Ok(4));
  assert_eq!(tick.recv(&mut consumer), Some(1));

  let mut out = Vec::new();
  assert_eq!(tick.drain(&mut consumer, &mut out, 2), 2);

  assert_eq!(tick.progress(), Progress::Made(8), "1 + 4 + 1 + 2");
  assert!(format!("{tick:?}").contains("Tick"));
}

/// Operations that move nothing are not counted as progress.
#[test]
fn a_tick_counts_nothing_when_nothing_moved() {
  let mut ring = refusing_ring(1);
  let mut ends = ring.ends();
  let (mut producer, mut consumer) = ends.split();
  producer.try_push(1).unwrap();
  assert_eq!(consumer.try_recv(), Some(1));
  producer.try_push(2).unwrap();

  let mut tick = Tick::default();
  assert_eq!(tick.push(&mut producer, 3), Err(3), "the ring is full");
  assert_eq!(tick.drain(&mut consumer, &mut Vec::new(), 0), 0);
  assert_eq!(tick.progress(), Progress::None);

  assert_eq!(consumer.try_recv(), Some(2));
  let mut empty = Tick::default();
  assert_eq!(empty.recv(&mut consumer), None, "and an empty read is not progress");
  assert_eq!(empty.progress(), Progress::None);
}

/// The default tick is a single attempt, matching the default budget.
#[test]
fn a_default_tick_is_a_single_attempt() {
  assert_eq!(Tick::default().budget(), Budget::once());
  assert_eq!(Tick::new(Budget::once()).budget(), Tick::default().budget());
}

/// A tick counts the records that arrived, and the refused one comes back
/// rather than being counted.
#[test]
fn a_tick_counts_what_arrived_and_hands_back_what_was_refused() {
  let mut ring = refusing_ring(4);
  let mut ends = ring.ends();
  let (mut producer, _consumer) = ends.split();

  let mut tick = Tick::new(Budget::once());
  let mut records = 0..12;
  assert_eq!(tick.push_batch(&mut producer, &mut records), Err((4, 4)));

  assert_eq!(tick.progress(), Progress::Made(4), "four arrived, and 4 is not among them");
  assert_eq!(records.next(), Some(5), "5 is still pending");
}

/// A batch against a ring that is already full moves nothing, counts nothing,
/// and hands back the record its one attempt was refused.
#[test]
fn a_tick_counts_nothing_for_a_fully_refused_batch() {
  let mut ring = refusing_ring(1);
  let mut ends = ring.ends();
  let (mut producer, _consumer) = ends.split();
  producer.try_push(0).unwrap();

  let mut tick = Tick::new(Budget::once());
  let mut records = 1..5;
  assert_eq!(
    tick.push_batch(&mut producer, &mut records),
    Err((0, 1)),
    "the ring was already full, and record 1 came back",
  );

  assert_eq!(tick.progress(), Progress::None, "nothing arrived");
  assert_eq!(records.next(), Some(2), "2 is still pending");
}

/// `reset` clears the count and keeps the budget.
///
/// A tick is a per-frame object with no `Drop` and no by-value method, so
/// nothing makes a frame end. Without `reset` the only way to get a fresh count
/// is to build a new tick, which discards a budget the scheduler may have
/// computed. The suite itself did that before `reset` existed.
#[test]
fn a_reset_tick_reports_no_progress_and_keeps_its_budget() {
  let mut ring = refusing_ring(2);
  let mut ends = ring.ends();
  let (mut producer, _consumer) = ends.split();

  // Two slots, a budget of three, and eight records. The first attempt fills
  // both slots and is refused record 2; the second attempt offers record 2
  // again and moves nothing, which ends the loop before the third.
  let mut tick = Tick::new(Budget::new(3));
  let mut records = 0..8;
  assert_eq!(tick.push_batch(&mut producer, &mut records), Err((2, 2)));
  assert_eq!(tick.progress(), Progress::Made(2));
  assert_eq!(records.next(), Some(3), "record 2 came back, so 3 is next");

  tick.reset();

  assert_eq!(tick.progress(), Progress::None, "the frame ended");
  assert_eq!(tick.budget(), Budget::new(3), "and the budget survived it");
}

/// A clone accumulates separately, which is why `Tick` is not `Copy`.
///
/// Under `Copy` this duplication would happen at every by-value use, such as
/// the ordinary signature `fn run_systems( tick : Tick )`, and the caller
/// would read `Progress::None` after the callee did real work. `Clone` keeps
/// the duplication available and makes it something a reader can see.
#[test]
fn a_cloned_tick_accumulates_separately() {
  let mut ring = ring(16);
  let mut ends = ring.ends();
  let (mut producer, _consumer) = ends.split();

  let mut tick = Tick::new(Budget::once());
  tick.push(&mut producer, 1).unwrap();

  let mut copy = tick.clone();
  copy.push(&mut producer, 2).unwrap();

  assert_eq!(copy.progress(), Progress::Made(2));
  assert_eq!(tick.progress(), Progress::Made(1), "the original did not see it");
}

/// One budget buys one push but a whole batch, so the unit is not the same size.
///
/// `Budget::once()` is the documented conservative default. Against an empty
/// four-slot ring it moves one record through `push_within` and four through
/// `push_batch_within`, because one attempt there is a whole `try_push_batch`
/// whose inner loop takes no limit from the budget.
#[test]
fn one_budget_buys_one_push_and_a_whole_batch() {
  let mut single = refusing_ring(4);
  let mut single_ends = single.ends();
  let (mut single_producer, _c1) = single_ends.split();
  for record in 0..4u32 {
    assert!(push_within(&mut single_producer, record, Budget::once()).is_ok());
  }

  let mut batch = refusing_ring(4);
  let mut batch_ends = batch.ends();
  let (mut batch_producer, _c2) = batch_ends.split();
  assert_eq!(
    push_batch_within(&mut batch_producer, &mut (0..4), Budget::once()),
    Ok(4),
    "one attempt, four records — the budget did not bound this",
  );
}

/// Reaching past the tick for the free function leaves its count short.
///
/// `Tick::budget()` hands out exactly the argument the free functions want, so
/// this is the convenient thing to write, and nothing repairs the count
/// afterwards, because `moved` is private with no setter. The suite had three
/// `Tick`-only tests and fourteen free-function-only tests and none that used
/// both against one producer, so this arrangement had no coverage in either
/// direction.
#[test]
fn reaching_past_the_tick_leaves_its_count_short() {
  let mut ring = ring(16);
  let mut ends = ring.ends();
  let (mut producer, mut consumer) = ends.split();

  let mut tick = Tick::new(Budget::once());
  tick.push(&mut producer, 1).unwrap();
  push_within(&mut producer, 2, tick.budget()).unwrap();
  push_within(&mut producer, 3, tick.budget()).unwrap();

  assert_eq!(
    tick.progress(),
    Progress::Made(1),
    "three records moved and the tick counted the one that took its own door",
  );
  assert_eq!(consumer.len(), 3, "all three are really in the ring");

  let mut out = Vec::new();
  assert_eq!(drain_up_to(&mut consumer, &mut out, 8), 3);
  assert_eq!(out, [1, 2, 3], "in order, exactly once — nothing is corrupted");
  assert_eq!(tick.progress(), Progress::Made(1), "and the drain was not its either");
}
