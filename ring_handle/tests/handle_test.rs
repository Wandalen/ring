//! `ring_handle`: two ends, two values, disjoint capabilities.
//!
//! # What this file is arranged around
//!
//! This file carries the reached-test for
//! `docs/feature/179_producer_and_consumer_handles.md`, whose claim has two
//! halves: the capabilities are partitioned, and *"the two ends can be held by
//! different threads without sharing a mutable reference"*. The first half is a
//! compile-time property and is asserted in `tests/manual/readme.md` H1, where
//! the compiler errors are the evidence. The second half is a runtime property
//! and is [`the_two_ends_travel_to_separate_threads`].
//!
//! # What is deliberately not here
//!
//! **No test that a second `Producer` cannot be obtained.** There is no
//! `Clone` and no `try_clone`, and a test cannot call a method that does
//! not exist. H1 records the compiler's answer instead.
//!
//! **No test of the drop order.** `Split` owns the ring and the pair borrows
//! from an `Ends` that borrows from the `Split`, so the compiler already rejects
//! every order but the correct one. There is nothing left to check at runtime.
//!
//! **No `is_closed`.** It is absent by decision, not by omission. See
//! `docs/decisions/002`. The crate that owns the flag is `ring_shutdown`, and
//! depending on it here would put a parking operation within reach of the tick
//! path.

#![cfg(test)]
// Ordinary tests, compiled out under `--cfg loom`. That cfg swaps
// `ring_atomic`'s atomics for loom's instrumented ones across the whole
// family, and those panic the moment they are touched outside a
// `loom::model` closure, so without this gate a family-wide loom run dies
// here instead of reaching the models in `ring_spsc`, `ring_mpsc`,
// `ring_publish` and `ring_testkit`.
#![cfg(not(loom))]

use ring_config::RingConfig;
use ring_core::Ring;
use ring_handle::Split;
use ring_types::OverflowPolicy;

/// A ring of `slots` capacity, in the default (SPSC, drop-newest) configuration.
fn ring(slots: usize) -> Ring<u32> {
  Ring::new(&RingConfig::new(slots).unwrap()).unwrap()
}

/// A ring of `slots` capacity that refuses rather than dropping.
///
/// The default policy is `DropNewest`, under which a full push reports `Ok`
/// having discarded the record, so a test *about* a refusal needs this one.
fn refusing_ring(slots: usize) -> Ring<u32> {
  let config = RingConfig::new(slots).unwrap().with_overflow(OverflowPolicy::Fail);
  Ring::new(&config).unwrap()
}

// ── The reached-test ──────────────────────────────────────────────────────

/// Feature 179's reached-test, where the two ends go to two threads and neither
/// thread shares a mutable reference with the other.
///
/// The scoped threads are what make the claim observable. Each closure takes
/// one handle **by move**; there is no `&mut Ring` anywhere in the test, and
/// there could not be, because `Split::new` consumed it.
#[test]
fn the_two_ends_travel_to_separate_threads() {
  let mut split = Split::new(ring(64));
  let mut ends = split.ends();
  let (mut producer, mut consumer) = ends.split();

  let received = std::thread::scope(|scope| {
    scope.spawn(move || {
      let mut next: u32 = 0;
      while next < 32 {
        let moved = producer.try_push_batch(&mut (next..32));
        next += u32::try_from(moved).unwrap();
      }
    });

    scope
      .spawn(move || {
        let mut got = Vec::new();
        while got.len() < 32 {
          consumer.try_recv_batch(&mut got);
        }
        got
      })
      .join()
      .unwrap()
  });

  assert_eq!(received, (0..32).collect::<Vec<_>>(), "in order, and all of it");
}

// ── Producer ──────────────────────────────────────────────────────────────

/// A refusal hands the record back unchanged, so a caller can retry it.
#[test]
fn a_refused_push_hands_the_record_back_unchanged() {
  let mut split = Split::new(refusing_ring(2));
  let mut ends = split.ends();
  let (mut producer, mut consumer) = ends.split();

  producer.try_push(1).unwrap();
  producer.try_push(2).unwrap();
  let refused = producer.try_push(3).unwrap_err();
  assert_eq!(refused, 3, "the record itself, not a copy of a description of it");

  assert_eq!(consumer.try_recv(), Some(1));
  assert_eq!(producer.try_push(refused), Ok(()), "and it goes in once there is room");
}

/// A batch that does not fit is partially accepted, and the count says so.
#[test]
fn try_push_batch_reports_partial_acceptance() {
  let mut split = Split::new(refusing_ring(4));
  let mut ends = split.ends();
  let (mut producer, consumer) = ends.split();

  assert_eq!(producer.try_push_batch(&mut (0..10)), 4);
  assert_eq!(consumer.len(), 4);
}

/// `free_capacity` and `is_full` are one contract in two spellings.
#[test]
fn free_capacity_and_is_full_agree() {
  let mut split = Split::new(refusing_ring(4));
  let mut ends = split.ends();
  let (mut producer, _consumer) = ends.split();

  assert_eq!(producer.free_capacity(), 4);
  assert!(!producer.is_full());

  producer.try_push_batch(&mut (0..4));
  assert_eq!(producer.free_capacity(), 0);
  assert!(producer.is_full(), "is_full is free_capacity() == 0, literally");
}

// ── Consumer ──────────────────────────────────────────────────────────────

/// A batch drain moves what is waiting and reports how much.
#[test]
fn try_recv_batch_moves_what_is_waiting() {
  let mut split = Split::new(ring(8));
  let mut ends = split.ends();
  let (mut producer, mut consumer) = ends.split();
  producer.try_push_batch(&mut (0..5));
  assert!(!consumer.is_empty(), "nothing was waiting, so receiving proves nothing");

  let mut out = Vec::new();
  assert_eq!(consumer.try_recv_batch(&mut out), 5);
  assert_eq!(out, vec![0, 1, 2, 3, 4]);
  assert!(consumer.is_empty());
  assert_eq!(consumer.len(), 0);
}

/// The drain's bound is fixed at the call that made it, not as it runs.
///
/// This is the guarantee that separates a bounded drain from a live-lock, and
/// it is asserted **without threads**. The producer is a separate value, so it
/// can publish while the `Drain` iterator is alive. A fourth record published
/// after the iterator exists is not yielded by it.
#[test]
fn drain_is_bounded_at_the_call_that_made_it() {
  let mut split = Split::new(ring(16));
  let mut ends = split.ends();
  let (mut producer, mut consumer) = ends.split();
  producer.try_push_batch(&mut (0..3));

  let taken: Vec<u32> = {
    let mut drain = consumer.drain();
    assert_eq!(drain.size_hint(), (0, Some(3)), "the bound was read once, up front");

    producer.try_push(99).unwrap();

    let taken = drain.by_ref().collect();
    assert_eq!(drain.size_hint(), (0, Some(0)));
    taken
  };

  assert_eq!(taken, vec![0, 1, 2], "the record published mid-drain is not in it");
  assert_eq!(consumer.try_recv(), Some(99), "it is still there, for the next drain");
}

/// A drain stops when the ring empties, in lockstep with its own bound.
///
/// `remaining` is read once, at creation, from a count `Consumer::len` documents
/// as only ever growing under single-consumer cardinality. So for a `Drain`
/// that nothing refills mid-iteration, the bound and the ring's contents are
/// consumed in exact lockstep. `size_hint` below proves `remaining` has already
/// reached zero *before* the final call, which is what makes that call return
/// `None` from the bound check in `Drain::next` rather than from a `try_recv`
/// that finds nothing.
#[test]
fn drain_stops_when_the_ring_empties_first() {
  let mut split = Split::new(ring(8));
  let mut ends = split.ends();
  let (mut producer, mut consumer) = ends.split();
  producer.try_push_batch(&mut (0..2));

  let mut drain = consumer.drain();
  assert_eq!(drain.next(), Some(0));
  assert_eq!(drain.next(), Some(1));
  assert_eq!(drain.size_hint(), (0, Some(0)), "the bound already reached zero");
  assert_eq!(drain.next(), None, "so this call returns via the bound check, not a receive");
}

/// Draining an empty ring yields nothing and costs no ring operation.
#[test]
fn drain_of_an_empty_ring_yields_nothing() {
  let mut split = Split::new(ring(8));
  let mut ends = split.ends();
  let (_producer, mut consumer) = ends.split();

  let mut drain = consumer.drain();
  assert_eq!(drain.size_hint(), (0, Some(0)));
  assert_eq!(drain.next(), None);
}

/// An empty ring answers immediately rather than spinning until something
/// arrives.
///
/// `docs/api/002`'s guarantee 1. "Returns `Option`" and "returns promptly" are
/// two claims, and a retry loop added inside `try_recv` would satisfy the first
/// while silently spending a frame budget, which is `docs/pitfall/001`'s F6. The bound
/// is loose on purpose. It discriminates a return from a park, not one
/// nanosecond from another.
#[test]
fn an_empty_ring_answers_promptly() {
  let mut split = Split::new(ring(8));
  let mut ends = split.ends();
  let (_producer, mut consumer) = ends.split();

  let started = std::time::Instant::now();
  for _ in 0..10_000 {
    assert_eq!(consumer.try_recv(), None);
  }
  let elapsed = started.elapsed();

  assert!(
    elapsed < std::time::Duration::from_millis(500),
    "10 000 empty drains took {elapsed:?}; a 50µs park each would be ~500ms"
  );
}

/// Dropping one handle leaves the other usable.
///
/// R3 of `docs/lifecycle/001`. The two handles borrow disjointly from the same
/// `Ends`, so neither's lifetime is tied to the other's. But that is a claim
/// about the shape rather than something the shape announces, and R3 asked for
/// it asserted rather than assumed.
#[test]
fn dropping_one_handle_leaves_the_other_usable() {
  let mut split = Split::new(refusing_ring(4));
  {
    let mut ends = split.ends();
    let (mut producer, consumer) = ends.split();

    producer.try_push(7).unwrap();
    {
      // The consumer's lifetime ends at this brace. Scoping it rather than
      // calling `drop` keeps clippy's `drop_non_drop` quiet, which is fair, since
      // neither handle has a destructor, and the point here is the *borrow*
      // ending, not a destructor running.
      let _gone = consumer;
    }
    producer.try_push(8).unwrap();
    assert_eq!(producer.free_capacity(), 2, "both landed, with the consumer already gone");
  }

  let mut ends = split.ends();
  let (_producer, mut consumer) = ends.split();
  assert_eq!(consumer.try_recv(), Some(7), "and the ring kept what was published");
  assert_eq!(consumer.try_recv(), Some(8));
}

/// Records left in the ring are dropped exactly once when the ring goes.
///
/// R4 of `docs/lifecycle/001`. Undrained records are the case that leaks
/// silently. Nothing observes them, so a double-drop or a missed drop shows up
/// as a corrupted allocator or a leak far from here. `u32` cannot catch it;
/// this needs a type whose destructor counts.
#[test]
fn undrained_records_are_dropped_exactly_once() {
  use std::sync::atomic::{AtomicUsize, Ordering};

  static DROPPED: AtomicUsize = AtomicUsize::new(0);

  struct Counted;

  impl Drop for Counted {
    fn drop(&mut self) {
      DROPPED.fetch_add(1, Ordering::Relaxed);
    }
  }

  let config = RingConfig::new(8).unwrap().with_overflow(OverflowPolicy::Fail);
  let mut split = Split::new(Ring::<Counted>::new(&config).unwrap());
  {
    let mut ends = split.ends();
    let (mut producer, mut consumer) = ends.split();

    for _ in 0..5 {
      producer.try_push(Counted).ok().unwrap();
    }
    drop(consumer.try_recv().unwrap());
    assert_eq!(DROPPED.load(Ordering::Relaxed), 1, "the drained one, and only it");
  }

  drop(split);
  assert_eq!(DROPPED.load(Ordering::Relaxed), 5, "the four still in the ring, once each");
}

/// Given the same publication sequence, two runs drain identically.
///
/// K4 of `docs/lifecycle/002`. That instance's finding is that the determinism
/// this crate is *credited* with enabling is enforced by the barrier rather
/// than here. What this crate can assert is the narrower half: the
/// ring itself introduces no ordering nondeterminism, so a fixed publication
/// history produces a fixed drain.
#[test]
fn draining_at_the_same_point_is_deterministic() {
  fn run() -> Vec<u32> {
    let mut split = Split::new(ring(16));
    let mut ends = split.ends();
    let (mut producer, mut consumer) = ends.split();

    producer.try_push_batch(&mut (0..5));
    let first: Vec<u32> = consumer.drain().collect();
    producer.try_push_batch(&mut (5..9));
    let second: Vec<u32> = consumer.drain().collect();

    first.into_iter().chain(second).collect()
  }

  assert_eq!(run(), run(), "same publication history, same drain");
  assert_eq!(run(), (0..9).collect::<Vec<_>>());
}

// ── Across the backends ───────────────────────────────────────────────────

/// The handle API behaves the same whichever backend is beneath it.
///
/// Requirement 1 of `docs/integration/001`. One dependency conceals backends
/// whose contracts differ, and the only thing keeping the API uniform is a
/// test that runs against each. `ring_core` selects between the in-house rings
/// by producer count, so two of the three need no feature; the third is behind
/// `crossbeam` and is covered below.
#[test]
fn the_in_house_backends_behave_alike() {
  for producers in [1, 4] {
    let config = RingConfig::new(8)
      .unwrap()
      .with_producers(producers)
      .with_overflow(OverflowPolicy::Fail);
    let ring = Ring::<u32>::new(&config).unwrap();
    let backend = ring.backend();

    let mut split = Split::new(ring);
    let mut ends = split.ends();
    let (mut producer, mut consumer) = ends.split();

    assert_eq!(producer.try_push_batch(&mut (0..8)), 8, "{backend:?} took the batch");
    assert!(producer.is_full(), "{backend:?} reports full at capacity");
    assert_eq!(producer.try_push(99).unwrap_err(), 99, "{backend:?} hands the record back");

    let drained: Vec<u32> = consumer.drain().collect();
    assert_eq!(drained, (0..8).collect::<Vec<_>>(), "{backend:?} drained in order");
    assert!(consumer.is_empty(), "{backend:?} is empty afterwards");
  }
}

/// The same API, on feature 187's interim backend.
///
/// Separate from [`the_in_house_backends_behave_alike`] because it is reached
/// through `Ring::new_crossbeam` rather than by configuration, and because the
/// backend is behind a cargo feature this crate forwards rather than declares.
#[cfg(feature = "crossbeam")]
#[test]
fn the_crossbeam_backend_behaves_alike() {
  let config = RingConfig::new(8).unwrap().with_overflow(OverflowPolicy::Fail);
  let mut split = Split::new(Ring::<u32>::new_crossbeam(&config).unwrap());
  let mut ends = split.ends();
  let (mut producer, mut consumer) = ends.split();

  assert_eq!(producer.try_push_batch(&mut (0..8)), 8);
  assert!(producer.is_full());
  assert_eq!(producer.try_push(99).unwrap_err(), 99);

  let drained: Vec<u32> = consumer.drain().collect();
  assert_eq!(drained, (0..8).collect::<Vec<_>>());
  assert!(consumer.is_empty());
}

// ── The structure ─────────────────────────────────────────────────────────

/// The wrapper costs nothing, because each handle is exactly its backend handle.
///
/// `docs/data_structure/001` asked for "one pointer", which was written before
/// `ring_core`'s handles existed and is wrong for them, since they carry an enum
/// discriminant and an overflow policy. The property that matters is
/// the one `docs/algorithm/002` states: a handle adds no state of its own.
/// Equality of sizes is that property, measured.
#[test]
fn the_wrapper_costs_nothing() {
  use core::mem::size_of;

  assert_eq!(
    size_of::<ring_handle::Producer<'_, u32>>(),
    size_of::<ring_core::Producer<'_, u32>>(),
    "the producer newtype adds no field"
  );
  assert_eq!(
    size_of::<ring_handle::Consumer<'_, u32>>(),
    size_of::<ring_core::Consumer<'_, u32>>(),
    "nor does the consumer"
  );
}

/// Q1 and Q2 of `docs/non_functional_requirement/002_send_without_sync.md`:
/// both handles cross a thread boundary on their own, with no guard type.
///
/// `the_two_ends_travel_to_separate_threads` already exercises this, but it
/// would also fail for a dozen unrelated reasons. A static assertion names the
/// property, so a change that costs `Send` is reported as *that* rather than as
/// a thread test that stopped compiling.
#[test]
fn both_handles_are_send() {
  fn assert_send<T: Send>() {}

  assert_send::<ring_handle::Split<u32>>();
  assert_send::<ring_handle::Producer<'_, u32>>();
  assert_send::<ring_handle::Consumer<'_, u32>>();
}

// Q5, which says `Sync` is absent on both handles, is pinned by
// `tests/ui/producer_shared_across_threads.rs`, not here. "Not `Sync`" is not
// expressible as a static assertion. There is no negative trait bound, so the
// only mechanism that fails when `Sync` appears is a program that must not
// compile.

/// No parking-shaped name appears in this crate's own source.
///
/// `docs/invariant/002` names a residual gap that `ring_poll::PARKING_CRATES`
/// cannot see. That guard watches the **dependency graph**: `ring_handle` gains
/// a parking operation only by taking a dependency on `ring_wait`, and any such
/// edge fails `ring_poll`'s suite. A `std::thread::sleep` written inline in a
/// forwarding method adds no edge, because `std` is not a manifest entry, so
/// that guard cannot see it.
///
/// This is the cheap half of that gap, closed where the source is. It reads the
/// crate's own text rather than its manifest, which is a different instrument
/// answering a different question, and it is deliberately a substring scan
/// rather than a parse. A name that would need parsing to find is not the kind
/// of edit this catches.
///
/// **Comments are stripped before the scan, and that is not a detail.** The
/// first version of this test scanned the raw file for the bare substring
/// `park` and failed. It hit the module documentation explaining that no *parking*
/// operation may be reachable. A guard that fires on the prose describing it is
/// the defect shape `ring_poll`'s manual plan catalogues first, and it arrived
/// here within a minute of the guard being written. `tests/manual/readme.md` H5
/// records it.
///
/// **What it does not catch, stated so nobody mistakes it for more:** a busy
/// loop, a `Duration` arriving through a type alias, or a blocking call reached
/// through a dependency other than `ring_wait`. Row 3 of `invariant/002`'s
/// mechanism table, "if anyone looks", still covers those.
#[test]
fn no_parking_shaped_name_appears_in_the_source() {
  const FORBIDDEN: [&str; 7] = [
    "thread::sleep",
    "yield_now",
    "::park",
    "park(",
    "Condvar",
    "Duration",
    "Waker",
  ];

  let source = include_str!("../src/lib.rs");
  let code: String = source
    .lines()
    .map(|line| line.split("//").next().unwrap_or(""))
    .collect::<Vec<_>>()
    .join("\n");

  let found: Vec<&str> = FORBIDDEN.into_iter().filter(|name| code.contains(*name)).collect();

  assert!(
    found.is_empty(),
    "parking-shaped names in ring_handle's code: {found:?} — see docs/invariant/002"
  );
}

/// Every exported type is `Debug`, so a panic message can name what it held,
/// and printing one changes nothing about it.
///
/// The second half is C7 of [`docs/type/002_consumer.md`], the one rule in that
/// instance that types cannot enforce. `Debug` takes `&self`, so nothing stops
/// an implementation from draining into the formatter and reporting the
/// contents. The compiler would accept it. A test that formats a loaded
/// consumer and then checks `len()` is the only thing that would not.
///
/// The `Drain` line is the same rule at its sharpest. `Drain` is an iterator
/// over pending records; a `Debug` that enumerated them to print them would
/// consume the ring as a side effect of a log line, and the count afterwards is
/// what catches it.
#[test]
fn every_handle_can_be_printed() {
  let mut split = Split::new(ring(4));
  assert!(format!("{split:?}").contains("Split"));

  let mut ends = split.ends();
  assert!(format!("{ends:?}").contains("Ends"));

  let (mut producer, mut consumer) = ends.split();
  assert!(format!("{producer:?}").contains("Producer"));

  producer.try_push(1).unwrap();
  producer.try_push(2).unwrap();
  let loaded = consumer.len();
  assert_eq!(loaded, 2);

  assert!(format!("{consumer:?}").contains("Consumer"));
  assert_eq!(consumer.len(), loaded, "formatting the consumer drained it");

  assert!(format!("{:?}", consumer.drain()).contains("Drain"));
  assert_eq!(consumer.len(), loaded, "formatting a Drain drained it");
}

/// HD44: `Consumer`'s `#[ derive( Debug ) ]` forwards through `ring_core` and
/// `ring_spsc`'s own hand-written impl before "must not print slot contents"
/// is kept in code, three links down from every doc comment that states the
/// constraint. `every_handle_can_be_printed` cannot see a regression there. A
/// `Debug` that printed every pending record would still contain "Consumer"
/// and would not itself call `try_recv`, so that test would keep passing.
/// What such a regression cannot avoid is a formatted string that grows with
/// the item count. Bind that instead of the mechanism, the same shape as
/// `the_wrapper_costs_nothing`'s `size_of` equality.
#[test]
fn debug_output_length_does_not_grow_with_item_count() {
  let mut split_one = Split::new(ring(8));
  let mut ends_one = split_one.ends();
  let (mut producer_one, consumer_one) = ends_one.split();
  producer_one.try_push(1).unwrap();

  let mut split_many = Split::new(ring(8));
  let mut ends_many = split_many.ends();
  let (mut producer_many, consumer_many) = ends_many.split();
  producer_many.try_push_batch(&mut (0..8));

  assert_eq!(
    format!("{consumer_one:?}").len(),
    format!("{consumer_many:?}").len(),
    "Consumer's Debug output length grew with item count — it is printing slot contents"
  );
}
