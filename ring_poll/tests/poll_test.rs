//! `ring_poll` — bounded, non-parking progress on the tick path.
//!
//! # What this file is arranged around
//!
//! This file carries the reached-test for
//! `docs/feature/183_try_only_operations_on_the_tick_path.md`, whose claim is
//! *structural*: the parking operations are not reachable from where they would
//! deadlock. A claim about reachability cannot be tested by calling something —
//! there is nothing to call. So
//! [`the_tick_path_cannot_reach_a_parking_operation`] asserts it the only way it
//! is assertable, against the dependency graph, and
//! [`a_large_budget_spins_rather_than_sleeping`] asserts the behavioural half:
//! that what *is* reachable costs spin-time rather than sleep-time.
//!
//! The feature also constrains `ring_handle` without being claimed by it —
//! nothing reachable from a handle may park. That constraint is enforced here,
//! by name, because `ring_handle`'s own suite would stay green if it broke.
//!
//! # What is deliberately not here
//!
//! **No test that a parking call fails to compile.** It would not fail to
//! compile; it would fail to *resolve*, because the crate is not a dependency —
//! and a test cannot name a crate it cannot see. The manifest scan below is the
//! honest form of that assertion, and `tests/manual/readme.md` P1 records what
//! the compiler actually says when the dependency is added back.
//!
//! **No multi-threaded drain race.** A budget bounds attempts, not time, and a
//! two-thread test that observes "it returned" observes what the single-thread
//! test already established. What multiple threads would add is a *rate*
//! measurement, which belongs to `ring_bench` at S8.

#![cfg(test)]
// Ordinary tests, compiled out under `--cfg loom`. That cfg swaps
// `ring_atomic`'s atomics for loom's instrumented ones across the whole
// family, and those panic the moment they are touched outside a
// `loom::model` closure — so without this gate a family-wide loom run dies
// here instead of reaching the models in `ring_spsc`, `ring_mpsc`,
// `ring_publish` and `ring_testkit`.
#![cfg(not(loom))]

use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use ring_config::RingConfig;
use ring_core::Ring;
use ring_poll::{
    Budget,
    PARKING_CRATES,
    Progress,
    Tick,
    drain_up_to,
    push_batch_within,
    push_within,
    recv_within,
};
use ring_types::OverflowPolicy;

/// A ring of `slots` capacity, in the default (SPSC, drop-newest) configuration.
fn ring(slots: usize) -> Ring<u32> {
    Ring::new(&RingConfig::new(slots).unwrap()).unwrap()
}

/// A ring of `slots` capacity that refuses rather than dropping.
///
/// The default overflow policy is `DropNewest`, under which a full ring reports
/// `Ok` and discards the record — so every test below that is *about* running
/// out of budget uses this instead. The default's own behaviour is covered by
/// [`push_within_under_drop_newest_reports_success_and_keeps_nothing`] rather
/// than left untested.
fn refusing_ring(slots: usize) -> Ring<u32> {
    let config = RingConfig::new(slots).unwrap().with_overflow(OverflowPolicy::Fail);
    Ring::new(&config).unwrap()
}

// ── The reached-test ──────────────────────────────────────────────────────

/// Feature 183's reached-test: the crates from which a parking operation is
/// reachable are exactly the three declared, and no tick-path crate is among
/// them.
///
/// The assertion runs against the manifests on disk rather than against
/// [`PARKING_CRATES`] alone, so the roster cannot drift away from the graph it
/// describes. Adding `ring_wait` to `ring_handle` — the exact mistake feature
/// 183 exists to prevent, and one `ring_handle`'s own green suite would not
/// notice — fails here.
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
/// than an unexamined one — and so that a *new* transitive path, which the
/// other test would pass straight through, fails here.
///
/// `[dev-dependencies]` is deliberately excluded: `ring_publish` reaches
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

    let declares_directly = |who: &str| -> bool {
        direct.iter().any(|(n, d)| n == who && d.iter().any(|x| x == "ring_wait"))
    };

    // Closure: a crate reaches `ring_wait` if it *is* `ring_wait`, declares it,
    // or declares something that reaches it.  The graph is small and acyclic;
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
    // that declare `ring_wait`, plus `ring_wait` itself — which the scan matches
    // only because its own `name =` line contains the string it searches for, not
    // because of any edge.
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
/// `include_str!` rather than a read, so the assertion cannot be satisfied by a
/// manifest that changed after the binary was built.
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
/// roughly 100× over the spinning one, which is what makes it discriminate
/// rather than merely pass.
#[test]
fn a_large_budget_spins_rather_than_sleeping() {
    let mut ring = refusing_ring(4);
    let mut ends = ring.ends();
    let (mut producer, _consumer) = ends.split();
    assert_eq!(push_batch_within(&mut producer, &mut (0..4), Budget::once()), 4);

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
/// astronomically large — the exact false negative [`Progress`] exists to
/// rule out.
///
/// Root Cause: `then` combined the two counts with plain `self.count() +
/// other.count()`, and [`Progress::of`] is a public, unconstrained `usize`
/// constructor, so a caller combining `usize::MAX` progress with any more
/// panicked in a debug build and silently wrapped to `0` in release —
/// `Self::of( 0 )` is `Progress::None`.
///
/// Why Not Caught: every existing test summed small literal counts
/// (`progress_sums_across_the_steps_of_one_tick` uses `2`, `3`, `1`), and the
/// crate's one previously-known raw-`+=` overflow risk (`Tick::moved`,
/// `docs/data_structure/002` PL12) is a private field reachable only through
/// real ring traffic and was explicitly dismissed there as impractical to
/// reach — which is true of `Tick::moved` but not of `Progress::of`, a public
/// constructor one direct call away from the same class of bug with no ring
/// involved at all.
///
/// Fix Applied: `then` now composes with `saturating_add` instead of `+`, so
/// the sum floors at `usize::MAX` — still `Progress::Made`, never a spurious
/// `Progress::None`.
///
/// Prevention: a type whose entire purpose is ruling out a false-negative
/// reading must saturate rather than wrap (or panic) on its own composition —
/// crashing the tick path is as unusable there as the false reading would be.
///
/// Pitfall: a public, unconstrained constructor over a primitive
/// (`Progress::of`) puts arithmetic on its output outside any bound the
/// defining crate controls; audit every site that combines two of its values,
/// not only the crate's own internal counters.
#[test]
fn then_saturates_instead_of_reporting_false_no_progress() {
    let huge = Progress::of(usize::MAX);
    let one = Progress::of(1);

    assert_eq!(huge.then(one), Progress::Made(usize::MAX), "must saturate, not wrap to None");
    assert!(huge.then(one).is_made(), "an enormous amount of progress must never read as none");
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

    // And once there is room, the same budget succeeds — so the failure above was
    // back-pressure and not the retry loop being broken.
    assert_eq!(consumer.try_recv(), Some(1));
    assert_eq!(push_within(&mut producer, 3, Budget::new(4)), Ok(()));
}

/// Under the default policy a full push reports success and keeps nothing.
///
/// This is `OverflowPolicy::DropNewest`'s behaviour, surfacing through every
/// helper here. Recorded in `docs/pitfall/002` and first found in
/// `ring_shutdown`, whose `Refusal::Full` arm is unreachable for the same reason.
#[test]
fn push_within_under_drop_newest_reports_success_and_keeps_nothing() {
    let mut ring = ring(2);
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

    assert_eq!(push_batch_within(&mut producer, &mut (0..5), Budget::once()), 5);
    assert_eq!(consumer.len(), 5);
}

/// An attempt that moves nothing ends the loop, whatever the budget says.
#[test]
fn push_batch_within_stops_at_the_first_attempt_that_moves_nothing() {
    let mut ring = refusing_ring(2);
    let mut ends = ring.ends();
    let (mut producer, _consumer) = ends.split();
    producer.try_push(1).unwrap();
    producer.try_push(2).unwrap();

    assert_eq!(push_batch_within(&mut producer, &mut (0..4), Budget::new(9)), 0);
}

/// A second attempt runs when the first one moved records, and stops when it
/// stops moving them.
#[test]
fn push_batch_within_spends_a_second_attempt_after_a_productive_first() {
    let mut ring = refusing_ring(4);
    let mut ends = ring.ends();
    let (mut producer, consumer) = ends.split();

    // Twelve records into four slots: attempt one takes four, attempt two takes
    // none and breaks.
    let mut records = 0..12;
    assert_eq!(push_batch_within(&mut producer, &mut records, Budget::new(3)), 4);
    assert_eq!(consumer.len(), 4);
    assert_eq!(
        records.next(),
        Some(6),
        "two attempts, two records eaten — 4 by the first refusal, 5 by the second",
    );
}

/// Every attempt after the first costs one more record.
///
/// `ring_core::Producer::try_push_batch` pulls from the iterator before it can
/// know whether there is room, so a refusal consumes and drops the record it
/// could not place — that crate's own doctest pins it as intended. What nothing
/// asserted is the consequence for a *budget*: retrying is not free, and a
/// budget of N against a full ring destroys N records rather than one.
///
/// The iterator is the only place that cost is visible. The published count and
/// the consumer's length are identical whether one attempt ran or two, which is
/// why the test above could spend a second attempt without noticing it had paid
/// for it. Read together the two tests price the budget: one attempt eats
/// record 4, two attempts eat 4 and 5.
#[test]
fn push_batch_within_eats_one_record_per_attempt() {
    let mut ring = refusing_ring(4);
    let mut ends = ring.ends();
    let (mut producer, _consumer) = ends.split();

    let mut records = 0..12;
    assert_eq!(push_batch_within(&mut producer, &mut records, Budget::once()), 4);
    assert_eq!(
        records.next(),
        Some(5),
        "one attempt: 0-3 published, 4 eaten by the refusal, 5 still pending",
    );
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
    assert_eq!(producer.try_push_batch(&mut (0..5)), 5, "the arrange filled the ring");

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
    assert_eq!(producer.try_push_batch(&mut (0..2)), 2, "the arrange filled the ring");

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
    assert_eq!(producer.try_push_batch(&mut (0..3)), 3, "the arrange filled the ring");

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
    assert_eq!(tick.push_batch(&mut producer, &mut (2..6)), 4);
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

/// A tick reports what its batch destroyed, and `progress` still does not.
///
/// `push_batch_within` returns one number and the records it consumed and
/// dropped are not in it — the free function cannot report the loss because the
/// caller keeps the iterator. `Tick::push_batch` counts the iterator on the way
/// past, so the difference is available without changing the free function's
/// signature. The point of the second assertion is that `progress()` is
/// deliberately *unchanged*: it still counts arrivals only.
#[test]
fn a_tick_records_what_a_refused_batch_destroyed() {
    let mut ring = refusing_ring(4);
    let mut ends = ring.ends();
    let (mut producer, _consumer) = ends.split();

    let mut tick = Tick::new(Budget::once());
    let mut records = 0..12;
    assert_eq!(tick.push_batch(&mut producer, &mut records), 4);

    assert_eq!(tick.progress(), Progress::Made(4), "four arrived");
    assert_eq!(tick.lost(), 1, "and record 4 was eaten by the refusal");
    assert_eq!(records.next(), Some(5), "5 is still pending, as ever");
}

/// A batch against a ring that is already full moves nothing on its one
/// attempt, and the record that attempt ate is still reported as lost.
///
/// `offered` is 1 and `moved` is 0 here — the one case the other `push_batch`
/// tests never hit, because in each of them at least one record gets through.
/// The loss must be derived from that pair without dividing by the zero.
#[test]
fn a_tick_records_a_fully_refused_batch_as_lost_not_moved() {
    let mut ring = refusing_ring(1);
    let mut ends = ring.ends();
    let (mut producer, _consumer) = ends.split();
    producer.try_push(0).unwrap();

    let mut tick = Tick::new(Budget::once());
    let mut records = 1..5;
    assert_eq!(tick.push_batch(&mut producer, &mut records), 0, "the ring was already full");

    assert_eq!(tick.progress(), Progress::None, "nothing arrived");
    assert_eq!(tick.lost(), 1, "one attempt, one record eaten by the refusal");
    assert_eq!(records.next(), Some(2), "record 1 was eaten, 2 is still pending");
}

/// Nothing else can produce a loss, so `lost` stays at zero for every other path.
#[test]
fn only_a_batch_can_lose_a_record() {
    let mut ring = refusing_ring(1);
    let mut ends = ring.ends();
    let (mut producer, mut consumer) = ends.split();

    let mut tick = Tick::new(Budget::once());
    tick.push(&mut producer, 1).unwrap();
    assert_eq!(tick.push(&mut producer, 2), Err(2), "handed back, not lost");
    assert_eq!(tick.recv(&mut consumer), Some(1));
    assert_eq!(tick.recv(&mut consumer), None);
    assert_eq!(tick.drain(&mut consumer, &mut Vec::new(), 4), 0);

    assert_eq!(tick.progress(), Progress::Made(2));
    assert_eq!(tick.lost(), 0);
}

/// `reset` clears both counters and keeps the budget.
///
/// A tick is a per-frame object with no `Drop` and no by-value method, so
/// nothing makes a frame end. Without `reset` the only way to get a fresh count
/// is to build a new tick, which discards a budget the scheduler may have
/// computed — and which is what the suite itself did before this existed.
#[test]
fn a_reset_tick_reports_no_progress_and_keeps_its_budget() {
    let mut ring = refusing_ring(2);
    let mut ends = ring.ends();
    let (mut producer, _consumer) = ends.split();

    // Two slots, a budget of three, and eight records. The first attempt fills
    // both slots and eats one more finding out the ring is full; the second
    // attempt eats one and moves nothing, which ends the loop before the third.
    // So four records leave the iterator, two arrive, and two are destroyed.
    let mut tick = Tick::new(Budget::new(3));
    let mut records = 0..8;
    assert_eq!(tick.push_batch(&mut producer, &mut records), 2);
    assert_eq!(tick.progress(), Progress::Made(2));
    assert_eq!(tick.lost(), 2);
    assert_eq!(records.next(), Some(4), "records 2 and 3 were consumed and dropped");

    tick.reset();

    assert_eq!(tick.progress(), Progress::None, "the frame ended");
    assert_eq!(tick.lost(), 0, "including the cost");
    assert_eq!(tick.budget(), Budget::new(3), "and the budget survived it");
}

/// A clone accumulates separately, which is why `Tick` is not `Copy`.
///
/// Under `Copy` this duplication would happen at every by-value use —
/// `fn run_systems( tick : Tick )` is an ordinary signature — and the caller
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

/// One budget buys one push and a whole batch — the unit is not the same size.
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
        4,
        "one attempt, four records — the budget did not bound this",
    );
}

/// Reaching past the tick for the free function leaves its count short.
///
/// `Tick::budget()` hands out exactly the argument the free functions want, so
/// this is the convenient thing to write, and nothing repairs the count
/// afterwards — `moved` is private with no setter. The suite had three
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
