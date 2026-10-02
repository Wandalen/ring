//! A batch costs one operation, and the slots it buys are contiguous.
//!
//! Claims `docs/feature/177_batch_claim_and_batch_drain.md`, whose reached-test
//! reads: "A claim of 64 slots issues one fence, not 64 (asserted against a
//! counting ordering shim); the 64 sequences returned are contiguous; a batch
//! drain reads them in issue order."
//!
//! All three clauses are asserted literally below, the first against
//! `ring_atomic::CountingSeq`, the counting ordering shim the criterion names.
//! It is not a mock. The same real `AtomicU64` performs the same real
//! operation, with a `Relaxed` counter alongside. An assertion made against it
//! is therefore a statement about the code under test rather than about a
//! substitute for it.
//!
//! The fourth thing tested here is not in the criterion and matters as much.
//! `claim_gated` must distinguish "ask for less" from "wait". Collapsing both
//! into `Full` would make an impossible request look transient, and a caller's
//! retry loop would spin forever on it.

// Ordinary tests, compiled out under `--cfg loom`. That cfg swaps
// `ring_atomic`'s atomics for loom's instrumented ones across the whole
// family, and those panic the moment they are touched outside a
// `loom::model` closure, so without this gate a family-wide loom run dies
// here instead of reaching the models in `ring_spsc`, `ring_mpsc`,
// `ring_publish` and `ring_testkit`.
#![cfg(not(loom))]

use core::sync::atomic::Ordering;
use std::collections::HashSet;
use std::thread;

use ring_atomic::{AtomicSeq, CountingSeq, SeqCell};
use ring_batch::{BatchClaim, claim, claim_gated, drain_order};
use ring_types::{Capacity, RingError, Seq, SlotIndex};

fn cap(n: usize) -> Capacity {
  Capacity::new(n).expect("test capacities are powers of two")
}

// ------------------------------------------------- the criterion, clause by clause

#[test]
fn a_claim_of_sixty_four_issues_one_operation_not_sixty_four() {
  let cursor = CountingSeq::default();
  let batch = claim(&cursor, 64, Ordering::AcqRel);

  assert_eq!(batch.len(), 64);
  assert_eq!(cursor.counts().fetch_adds, 1, "one fetch_add bought all 64");
  assert_eq!(cursor.counts().total, 1, "and nothing else was touched");
}

#[test]
fn the_cost_of_a_claim_does_not_depend_on_its_size() {
  // The stronger form is "every size costs 1", not only "64 costs 1". A loop that
  // advanced one at a time would pass the 64 case only by accident of the
  // assertion's shape.
  for count in [0usize, 1, 2, 7, 64, 1024] {
    let cursor = CountingSeq::default();
    let batch = claim(&cursor, count, Ordering::AcqRel);

    assert_eq!(batch.len(), count);
    assert_eq!(cursor.counts().total, 1, "a claim of {count} must cost exactly one operation");
  }
}

#[test]
fn the_sequences_returned_are_contiguous() {
  let cursor = CountingSeq::new(Seq(100));
  let batch = claim(&cursor, 64, Ordering::AcqRel);

  let seqs: Vec<u64> = batch.sequences().map(|s| s.0).collect();
  assert_eq!(seqs.len(), 64);
  assert_eq!(seqs, (100..164).collect::<Vec<_>>());

  for pair in seqs.windows(2) {
    assert_eq!(pair[1], pair[0] + 1, "a gap makes the claim non-contiguous");
  }
}

#[test]
fn a_batch_drain_reads_in_issue_order() {
  let capacity = cap(16);
  let batch = BatchClaim::new(Seq(14), 6);

  let drained: Vec<(Seq, SlotIndex)> = drain_order(&batch, capacity).collect();

  assert_eq!(
    drained,
    vec![
      (Seq(14), SlotIndex(14)),
      (Seq(15), SlotIndex(15)),
      (Seq(16), SlotIndex(0)),
      (Seq(17), SlotIndex(1)),
      (Seq(18), SlotIndex(2)),
      (Seq(19), SlotIndex(3)),
    ],
    "issue order survives the wrap; slot order does not, and must not"
  );
}

#[test]
fn drain_order_folds_through_ring_index_and_not_a_second_implementation() {
  let capacity = cap(8);
  let batch = BatchClaim::new(Seq(3), 20);

  for (seq, slot) in drain_order(&batch, capacity) {
    assert_eq!(slot, ring_index::of(seq, capacity), "the fold must be ring_index's");
  }
}

#[test]
fn an_empty_claim_drains_to_nothing() {
  let batch = BatchClaim::new(Seq(5), 0);
  assert_eq!(drain_order(&batch, cap(4)).count(), 0);
}

// ------------------------------------------------------------ the claim range

#[test]
fn a_claim_reports_its_own_extent() {
  let batch = BatchClaim::new(Seq(10), 3);

  assert_eq!(batch.start(), Seq(10));
  assert_eq!(batch.len(), 3);
  assert_eq!(batch.end(), Seq(13));
  assert!(!batch.is_empty());
}

#[test]
fn an_empty_claim_is_a_success_not_a_failure() {
  // A flush of an empty thread-local buffer takes the same path as a full one.
  // That only works if claiming nothing succeeds.
  let batch = BatchClaim::new(Seq(4), 0);

  assert!(batch.is_empty());
  assert_eq!(batch.start(), batch.end());
  assert_eq!(batch.sequences().count(), 0);
}

#[test]
fn containment_includes_the_start_and_excludes_the_end() {
  let batch = BatchClaim::new(Seq(10), 3);

  assert!(batch.contains(Seq(10)));
  assert!(batch.contains(Seq(11)));
  assert!(batch.contains(Seq(12)));
  assert!(!batch.contains(Seq(13)), "end is exclusive");
  assert!(!batch.contains(Seq(9)));
}

#[test]
fn an_empty_claim_contains_nothing_at_all() {
  let batch = BatchClaim::new(Seq(10), 0);
  for raw in 8..13 {
    assert!(!batch.contains(Seq(raw)));
  }
}

#[test]
fn overlap_is_exactly_range_intersection() {
  let a = BatchClaim::new(Seq(4), 4); // 4..8

  assert!(!a.overlaps(&BatchClaim::new(Seq(0), 4)), "abutting below");
  assert!(!a.overlaps(&BatchClaim::new(Seq(8), 4)), "abutting above");
  assert!(a.overlaps(&BatchClaim::new(Seq(7), 4)), "one sequence shared");
  assert!(a.overlaps(&BatchClaim::new(Seq(1), 4)), "one sequence shared, below");
  assert!(a.overlaps(&a), "a claim overlaps itself");
  assert!(a.overlaps(&BatchClaim::new(Seq(0), 16)), "fully contained");
}

#[test]
fn an_empty_claim_overlaps_nothing_even_inside_another() {
  // An empty range at sequence 5 is not a use of sequence 5. Treating it as one
  // would make every flush of an empty buffer look like a protocol violation.
  let empty = BatchClaim::new(Seq(5), 0);
  let wide = BatchClaim::new(Seq(0), 16);

  assert!(!empty.overlaps(&wide));
  assert!(!wide.overlaps(&empty));
  assert!(!empty.overlaps(&empty));
}

#[test]
fn claims_are_copied_not_moved_and_compare_by_value() {
  let a = BatchClaim::new(Seq(1), 2);
  let b = a;
  assert_eq!(a, b, "Copy, so the original is still usable");
  assert_ne!(a, BatchClaim::new(Seq(1), 3));
  assert!(format!("{a:?}").contains("count: 2"));
}

// ------------------------------------------------------------- the gated form

#[test]
fn a_gated_claim_succeeds_while_the_ring_has_room() {
  let capacity = cap(8);
  let producer = CountingSeq::default();
  let consumer = AtomicSeq::default();

  let batch = claim_gated(&producer, &consumer, 8, capacity, Ordering::AcqRel).expect("an empty ring has room for a full lap");

  assert_eq!(batch.start(), Seq::ZERO);
  assert_eq!(batch.len(), 8);
  assert_eq!(producer.counts().fetch_adds, 1, "gating adds a load, not a second advance");
}

#[test]
fn a_full_ring_refuses_with_full_and_advances_nothing() {
  let capacity = cap(8);
  let producer = AtomicSeq::default();
  let consumer = AtomicSeq::default();

  claim_gated(&producer, &consumer, 8, capacity, Ordering::AcqRel).unwrap();
  let before = producer.load(Ordering::Acquire);

  assert_eq!(
    claim_gated(&producer, &consumer, 1, capacity, Ordering::AcqRel),
    Err(RingError::Full)
  );
  assert_eq!(producer.load(Ordering::Acquire), before, "a refused claim must not advance");
}

#[test]
fn an_oversized_request_is_a_configuration_error_not_back_pressure() {
  // This is the distinction the caller's retry loop depends on. `Full` clears
  // when a consumer moves, and `BatchTooLarge` never clears at all.
  let capacity = cap(8);
  let producer = AtomicSeq::default();
  let consumer = AtomicSeq::default();

  let outcome = claim_gated(&producer, &consumer, 9, capacity, Ordering::AcqRel);

  assert_eq!(
    outcome,
    Err(RingError::BatchTooLarge {
      requested: 9,
      capacity: 8
    })
  );
  let error = outcome.unwrap_err();
  assert!(error.is_configuration(), "a caller must not retry this");
  assert!(!error.is_transient());
  assert!(RingError::Full.is_transient(), "and must retry the other");
}

#[test]
fn an_oversized_request_is_refused_before_the_ring_is_even_consulted() {
  // Ordering matters. Checking capacity first means an impossible request on an
  // empty ring still reports BatchTooLarge rather than succeeding by accident.
  let capacity = cap(4);
  let producer = CountingSeq::default();
  let consumer = CountingSeq::default();

  assert!(claim_gated(&producer, &consumer, 5, capacity, Ordering::AcqRel).is_err());
  assert_eq!(producer.counts().total, 0, "the cursors were never read");
  assert_eq!(consumer.counts().total, 0);
}

#[test]
fn a_consumer_advancing_reopens_the_gate() {
  let capacity = cap(4);
  let producer = AtomicSeq::default();
  let consumer = AtomicSeq::default();

  claim_gated(&producer, &consumer, 4, capacity, Ordering::AcqRel).unwrap();
  assert_eq!(
    claim_gated(&producer, &consumer, 1, capacity, Ordering::AcqRel),
    Err(RingError::Full)
  );

  consumer.store(Seq(2), Ordering::Release);

  let batch = claim_gated(&producer, &consumer, 2, capacity, Ordering::AcqRel).expect("two slots were freed");
  assert_eq!(batch, BatchClaim::new(Seq(4), 2));

  assert_eq!(
    claim_gated(&producer, &consumer, 1, capacity, Ordering::AcqRel),
    Err(RingError::Full),
    "and exactly two, not three"
  );
}

#[test]
fn a_gated_claim_of_zero_always_succeeds_even_on_a_full_ring() {
  let capacity = cap(2);
  let producer = AtomicSeq::default();
  let consumer = AtomicSeq::default();

  claim_gated(&producer, &consumer, 2, capacity, Ordering::AcqRel).unwrap();
  let batch = claim_gated(&producer, &consumer, 0, capacity, Ordering::AcqRel).expect("asking for nothing needs no room");

  assert!(batch.is_empty());
  assert_eq!(batch.start(), Seq(2));
}

// ----------------------------------------------------------------- contention

#[test]
fn concurrent_batch_claims_never_overlap() {
  // Disjointness of returned sequences is real, and free of charge from
  // fetch_add's own atomicity, but it is blind to this crate's actual race. A
  // claim can be disjoint from every other claim and still point at a slot
  // the consumer has not released yet (-> BA23, pitfall/001). Every claim
  // taken by every thread is collected and checked pairwise-disjoint by
  // sequence. That is stronger than checking the claims' ranges, because it
  // catches an off-by-one at either end.
  const THREADS: usize = 4;
  const BATCHES: usize = 500;
  const SIZE: usize = 8;

  let cursor = AtomicSeq::default();
  let cursor = &cursor;

  let claims: Vec<BatchClaim> = thread::scope(|scope| {
    let handles: Vec<_> = (0..THREADS)
      .map(|_| {
        scope.spawn(move || {
          (0..BATCHES)
            .map(|_| claim(cursor, SIZE, Ordering::AcqRel))
            .collect::<Vec<_>>()
        })
      })
      .collect();
    handles
      .into_iter()
      .flat_map(|h| h.join().expect("claimer panicked"))
      .collect()
  });

  assert_eq!(claims.len(), THREADS * BATCHES);

  let mut seen: HashSet<u64> = HashSet::with_capacity(THREADS * BATCHES * SIZE);
  for batch in &claims {
    assert_eq!(batch.len(), SIZE);
    for seq in batch.sequences() {
      assert!(seen.insert(seq.0), "sequence {} was claimed twice", seq.0);
    }
  }

  assert_eq!(seen.len(), THREADS * BATCHES * SIZE);
  assert_eq!(cursor.load(Ordering::Acquire), Seq((THREADS * BATCHES * SIZE) as u64));
}

#[test]
fn a_threads_own_batches_stay_in_its_issue_order() {
  // Hard problem 118's requirement: whatever the interleaving between threads,
  // one thread's own claims come back ascending.
  const THREADS: usize = 4;
  const BATCHES: usize = 250;

  let cursor = AtomicSeq::default();
  let cursor = &cursor;

  let per_thread: Vec<Vec<BatchClaim>> = thread::scope(|scope| {
    let handles: Vec<_> = (0..THREADS)
      .map(|_| scope.spawn(move || (0..BATCHES).map(|_| claim(cursor, 4, Ordering::AcqRel)).collect::<Vec<_>>()))
      .collect();
    handles.into_iter().map(|h| h.join().expect("claimer panicked")).collect()
  });

  for batches in &per_thread {
    for pair in batches.windows(2) {
      assert!(
        pair[0].end().0 <= pair[1].start().0,
        "a thread's later claim started before its earlier one ended"
      );
    }
  }
}
