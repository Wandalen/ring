//! `ring_cursor`'s padded cursor and the pair it comes in.
//!
//! This file carries the reached-test for `docs/feature/169_padded_cursor.md`,
//! stated in `ring/bench_harness/docs/acceptance/001_feature_reached_tests.md`
//! as three clauses: `align_of::<PaddedCursor>() == 64`,
//! `size_of::<PaddedCursor>() == 64`, and two `PaddedCursor` values in one
//! struct sitting at least 64 bytes apart.
//!
//! ## Why three clauses and not one
//!
//! Each of the first two is satisfiable while the feature fails.
//!
//! A type of size 8 with `align_of == 64` satisfies the first clause and packs
//! two neighbours 8 bytes apart in an array, because alignment says where a value may
//! start, not how much room it occupies. A type of size 64 with `align_of == 8`
//! satisfies the second and can start at offset 8, straddling two lines and
//! sharing both. Only the conjunction says "one per line".
//!
//! The third clause is a different kind of statement altogether. The first two
//! are about a type; the third is about two real fields at two real addresses,
//! and it is the one that would notice if a future `CursorPair` layout packed
//! the cursors together despite each still measuring 64 bytes on its own.
//!
//! ## What is not asserted here, and why
//!
//! That the padding *makes anything faster*. That is feature 186's job and
//! `ring_bench`'s file. A padded-versus-unpadded verdict is a measurement under
//! contention, not a unit test, and asserting a timing here would produce a
//! test that fails on a loaded CI box for reasons that have nothing to do with
//! the code. What this file establishes is that the layout the measurement will
//! be taken on is the layout claimed.

// Ordinary tests, compiled out under `--cfg loom`. That cfg swaps
// `ring_atomic`'s atomics for loom's instrumented ones across the whole
// family, and those panic the moment they are touched outside a
// `loom::model` closure, so without this gate a family-wide loom run dies
// here instead of reaching the models in `ring_spsc`, `ring_mpsc`,
// `ring_publish` and `ring_testkit`.
#![cfg(not(loom))]

use core::sync::atomic::Ordering;

use ring_align::CACHE_LINE;
use ring_atomic::SeqCell;
use ring_cursor::{CursorPair, PaddedCursor};
use ring_types::{Capacity, Seq};

fn cap(slots: usize) -> Capacity {
  Capacity::new(slots).expect("test capacities are powers of two")
}

// ── feature 169: the three-clause reached-test ─────────────────────────────

#[test]
fn a_padded_cursor_occupies_exactly_one_cache_line() {
  assert_eq!(
    core::mem::align_of::<PaddedCursor>(),
    64,
    "clause 1: a cursor starts on a line boundary"
  );
  assert_eq!(
    core::mem::size_of::<PaddedCursor>(),
    64,
    "clause 2: and takes the whole line, so the next value starts on the next one"
  );
  assert_eq!(
    core::mem::size_of::<PaddedCursor>(),
    CACHE_LINE,
    "the 64 above is ring_align::CACHE_LINE, not a coincidence"
  );
}

#[test]
fn two_cursors_in_one_struct_are_at_least_a_line_apart() {
  let pair = CursorPair::new(cap(8));
  let gap = pair.producer().addr().abs_diff(pair.consumer().addr());

  assert!(
    gap >= CACHE_LINE,
    "clause 3: producer and consumer are {gap} bytes apart, need >= {CACHE_LINE}"
  );
  assert!(
    pair.on_distinct_lines(),
    "and the gap actually puts them on different lines, not merely far apart"
  );
}

#[test]
fn the_gap_survives_the_pair_being_moved() {
  // A struct's field offsets are fixed at compile time, so this cannot fail
  // for a live `CursorPair`, and that is the point. The assertion exists to
  // catch a future layout where the two cursors stop being separate fields.
  let pair = CursorPair::new(cap(2));
  let boxed = Box::new(pair);

  assert!(boxed.on_distinct_lines(), "still separated after a move onto the heap");
}

#[test]
fn every_cursor_starts_on_a_line_boundary() {
  let pair = CursorPair::new(cap(4));

  for (name, addr) in [("producer", pair.producer().addr()), ("consumer", pair.consumer().addr())] {
    assert_eq!(addr % CACHE_LINE, 0, "{name} starts mid-line at {addr}");
  }
}

#[test]
fn an_array_of_cursors_gives_each_its_own_line() {
  // The array case is where alignment-without-size would fail. `align_of`
  // constrains only the first element's address; `size_of` constrains the
  // stride between all of them.
  let cursors: [PaddedCursor; 4] = Default::default();

  for window in cursors.windows(2) {
    let gap = window[1].addr() - window[0].addr();
    assert_eq!(gap, CACHE_LINE, "consecutive cursors are one line apart, not {gap}");
  }
}

// ── the cursor as a sequence cell ──────────────────────────────────────────

#[test]
fn a_cursor_holds_the_sequence_it_was_built_with() {
  assert_eq!(PaddedCursor::new(Seq(7)).load(Ordering::Acquire), Seq(7));
  assert_eq!(PaddedCursor::default().load(Ordering::Acquire), Seq::ZERO);
}

#[test]
fn padding_does_not_change_what_the_cell_does() {
  // The whole crate is `AtomicSeq` plus an attribute. Every `SeqCell` method
  // must behave exactly as the unpadded cell does, or the padding has stopped
  // being free.
  let cursor = PaddedCursor::new(Seq(10));

  assert_eq!(
    cursor.fetch_add(5, Ordering::AcqRel),
    Seq(10),
    "returns the pre-advance value"
  );
  assert_eq!(cursor.load(Ordering::Acquire), Seq(15));

  cursor.store(Seq(3), Ordering::Release);
  assert_eq!(cursor.load(Ordering::Acquire), Seq(3));

  assert_eq!(
    cursor.compare_exchange(Seq(3), Seq(4), Ordering::AcqRel, Ordering::Acquire),
    Ok(Seq(3))
  );
  assert_eq!(
    cursor.compare_exchange(Seq(3), Seq(9), Ordering::AcqRel, Ordering::Acquire),
    Err(Seq(4)),
    "a failed exchange reports what it actually found — the retry's input"
  );
}

#[test]
fn a_cursor_is_shared_by_reference_not_by_copy() {
  // Two `&PaddedCursor` to one cursor must see each other's writes. If the
  // padding wrapper had made the type `Copy`, a caller could hold two
  // independent cursors while believing it held one.
  let cursor = PaddedCursor::default();
  let (a, b) = (&cursor, &cursor);

  a.store(Seq(42), Ordering::Release);
  assert_eq!(b.load(Ordering::Acquire), Seq(42));
}

// ── the pair's gating readings ─────────────────────────────────────────────

#[test]
fn a_fresh_pair_has_the_whole_ring_free() {
  let pair = CursorPair::new(cap(16));

  assert_eq!(pair.free_slots(), 16);
  assert_eq!(pair.pending(), 0);
  assert!(pair.may_claim());
  assert_eq!(pair.capacity().get(), 16);
}

#[test]
fn free_slots_falls_as_the_producer_advances() {
  let pair = CursorPair::new(cap(4));

  for published in 0..=4u64 {
    pair.producer().store(Seq(published), Ordering::Release);
    assert_eq!(pair.free_slots(), 4 - published as usize, "after publishing {published} of 4");
  }
}

#[test]
fn exactly_one_lap_ahead_is_full_and_one_less_is_not() {
  // The off-by-one feature 178 calls a lap bug. At a distance of exactly
  // `capacity` the next claim lands on the slot the consumer is currently on.
  let pair = CursorPair::new(cap(4));

  pair.producer().store(Seq(3), Ordering::Release);
  assert!(pair.may_claim(), "3 ahead of a 4-slot ring: one slot left");

  pair.producer().store(Seq(4), Ordering::Release);
  assert!(!pair.may_claim(), "exactly one lap: no room");
  assert_eq!(pair.free_slots(), 0);
}

#[test]
fn a_consumer_moving_on_reopens_the_ring() {
  let pair = CursorPair::new(cap(4));
  pair.producer().store(Seq(4), Ordering::Release);
  assert!(!pair.may_claim());

  pair.consumer().store(Seq(1), Ordering::Release);
  assert!(pair.may_claim(), "one slot released is one claim allowed");
  assert_eq!(pair.free_slots(), 1);
}

#[test]
fn may_claim_and_free_slots_never_disagree() {
  // They are two readings of one state, and a caller may branch on either.
  // If they can ever disagree, one of them is lying.
  let pair = CursorPair::new(cap(8));

  for producer in 0..24u64 {
    for consumer in 0..=producer {
      pair.producer().store(Seq(producer), Ordering::Release);
      pair.consumer().store(Seq(consumer), Ordering::Release);

      assert_eq!(
        pair.may_claim(),
        pair.free_slots() > 0,
        "producer {producer}, consumer {consumer}"
      );
    }
  }
}

#[test]
fn pending_is_the_distance_the_consumer_still_has_to_travel() {
  let pair = CursorPair::new(cap(8));
  pair.producer().store(Seq(5), Ordering::Release);

  assert_eq!(pair.pending(), 5);
  pair.consumer().store(Seq(2), Ordering::Release);
  assert_eq!(pair.pending(), 3);
  pair.consumer().store(Seq(5), Ordering::Release);
  assert_eq!(pair.pending(), 0, "caught up");
}

#[test]
fn pending_ignores_capacity_and_free_slots_does_not() {
  // `pending` counts unread publications, which can exceed a ring's capacity
  // only if the producer overran, a state the gating exists to prevent. The
  // two readings answer different questions and must not be conflated.
  let small = CursorPair::new(cap(2));
  let large = CursorPair::new(cap(64));

  for pair in [&small, &large] {
    pair.producer().store(Seq(2), Ordering::Release);
  }

  assert_eq!(small.pending(), large.pending(), "same distance, same pending");
  assert_ne!(small.free_slots(), large.free_slots(), "different rings, different room");
}

#[test]
fn a_capacity_of_one_still_has_room_for_one() {
  // The smallest legal ring. `Capacity::new(1)` is valid, since one is a power
  // of two, and the boundary arithmetic must not treat it as degenerate.
  let pair = CursorPair::new(cap(1));

  assert_eq!(pair.free_slots(), 1);
  assert!(pair.may_claim());

  pair.producer().store(Seq(1), Ordering::Release);
  assert_eq!(pair.free_slots(), 0);
  assert!(!pair.may_claim());
}

#[test]
fn the_pair_reads_both_cursors_for_every_reading() {
  // A reading that consulted only the producer would give the right answer for
  // a consumer at zero and the wrong one for every other consumer. Moving only
  // the consumer must change all three readings.
  let pair = CursorPair::new(cap(8));
  pair.producer().store(Seq(8), Ordering::Release);

  let (free, pending, claimable) = (pair.free_slots(), pair.pending(), pair.may_claim());
  pair.consumer().store(Seq(4), Ordering::Release);

  assert_ne!(pair.free_slots(), free);
  assert_ne!(pair.pending(), pending);
  assert_ne!(pair.may_claim(), claimable);
}

// ── the two cursors are independent ────────────────────────────────────────

#[test]
fn writing_one_cursor_leaves_the_other_alone() {
  // Separate lines are a performance property. Separate *values* are a
  // correctness one, and a wrapper that accidentally aliased them would fail
  // here long before any benchmark noticed.
  let pair = CursorPair::new(cap(8));

  pair.producer().store(Seq(9), Ordering::Release);
  assert_eq!(pair.consumer().load(Ordering::Acquire), Seq::ZERO);

  pair.consumer().store(Seq(4), Ordering::Release);
  assert_eq!(pair.producer().load(Ordering::Acquire), Seq(9));
}

#[test]
fn two_threads_advancing_two_cursors_do_not_lose_writes() {
  // The shape the padding is for: one thread on each cursor, concurrently.
  // The assertion is about correctness, not speed. Every increment must land.
  const PER_THREAD: u64 = 10_000;

  let pair = CursorPair::new(cap(1024));

  std::thread::scope(|scope| {
    scope.spawn(|| {
      for _ in 0..PER_THREAD {
        // Fix(AT5): the claim is deliberately discarded. This loop counts advances
        // and does not consume the ranges they hand out.
        let _ = pair.producer().fetch_add(1, Ordering::AcqRel);
      }
    });
    scope.spawn(|| {
      for _ in 0..PER_THREAD {
        // Fix(AT5): the claim is deliberately discarded. This loop counts advances
        // and does not consume the ranges they hand out.
        let _ = pair.consumer().fetch_add(1, Ordering::AcqRel);
      }
    });
  });

  assert_eq!(pair.producer().load(Ordering::Acquire), Seq(PER_THREAD));
  assert_eq!(pair.consumer().load(Ordering::Acquire), Seq(PER_THREAD));
  assert_eq!(pair.pending(), 0, "both advanced equally, so nothing is outstanding");
}

#[test]
fn many_producers_on_one_cursor_lose_nothing() {
  const THREADS: u64 = 4;
  const PER_THREAD: u64 = 5_000;

  let cursor = PaddedCursor::default();

  std::thread::scope(|scope| {
    for _ in 0..THREADS {
      scope.spawn(|| {
        for _ in 0..PER_THREAD {
          // Fix(AT5): the claim is deliberately discarded. This loop counts advances
          // and does not consume the ranges they hand out.
          let _ = cursor.fetch_add(1, Ordering::AcqRel);
        }
      });
    }
  });

  assert_eq!(cursor.load(Ordering::Acquire), Seq(THREADS * PER_THREAD));
}

// ── the crate's own shape ──────────────────────────────────────────────────

#[test]
fn a_padded_cursor_is_its_atomic_and_nothing_else() {
  // 64 bytes of which 8 carry a sequence. If the type ever grows a field, the
  // size stops being CACHE_LINE and the first test catches it. This one says
  // why that would be wrong. The padding is meant to be empty, not to be
  // budget for state that belongs elsewhere.
  assert_eq!(core::mem::size_of::<PaddedCursor>(), CACHE_LINE);
  assert_eq!(core::mem::size_of::<core::sync::atomic::AtomicU64>(), 8);
}

#[test]
fn a_pair_is_two_lines_plus_its_capacity() {
  // Not pinned to an exact number, because the capacity's own placement within the
  // struct's trailing padding is the compiler's business. What must hold is
  // that the pair is at least its two cursors, and that adding the capacity
  // did not cost a third line of padding.
  let size = core::mem::size_of::<CursorPair>();

  assert!(size >= 2 * CACHE_LINE, "two cursors at minimum, got {size}");
  assert!(
    size <= 3 * CACHE_LINE,
    "capacity may cost up to one additional cache line, got {size}"
  );
  assert_eq!(core::mem::align_of::<CursorPair>(), CACHE_LINE);
}
