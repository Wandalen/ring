//! Accumulating is free; landing costs exactly one operation.
//!
//! Claims `docs/feature/175_thread_local_buffer_and_flush_into.md`, whose
//! reached-test reads: "A `TlsBuffer` accumulates `N` items with zero atomic
//! operations (asserted by a counting allocator/atomic shim), and one
//! `flush_into` moves all `N` into the ring as a single contiguous claim."
//!
//! Both halves are asserted against `ring_atomic::CountingSeq`, the counting
//! shim the criterion names. The pairing matters more than either half alone.
//! A buffer satisfying only the first would flush item by item and pay the same
//! `N` atomics at the end that it saved during. The traffic is moved rather than
//! removed. `one_flush_lands_everything_for_one_atomic_operation` is the test
//! that closes that hole, and `accumulating_is_free_and_landing_is_one_op`
//! measures both sides of the same run so neither can be satisfied in isolation.
//!
//! The `zero` and the `one` are literal assertions, not thresholds.

// Ordinary tests, compiled out under `--cfg loom`. That cfg swaps
// `ring_atomic`'s atomics for loom's instrumented ones across the whole
// family, and those panic the moment they are touched outside a
// `loom::model` closure. Without this gate a family-wide loom run dies
// here instead of reaching the models in `ring_spsc`, `ring_mpsc`,
// `ring_publish` and `ring_testkit`.
#![cfg(not(loom))]

use core::sync::atomic::Ordering;
use std::collections::HashSet;
use std::thread;

use ring_atomic::{AtomicSeq, CountingSeq, SeqCell};
use ring_batch::BatchClaim;
use ring_event::{drain_from, publish_into};
use ring_slot::TypedSlot;
use ring_store::Buffer;
use ring_tls::TlsBuffer;
use ring_types::{Capacity, RingError, Seq};

fn cap(n: usize) -> Capacity {
  Capacity::new(n).expect("test capacities are powers of two")
}

// ------------------------------------------------------- the criterion's halves

#[test]
fn accumulating_n_items_costs_zero_atomic_operations() {
  let cursor = CountingSeq::default();
  let mut buffer = TlsBuffer::with_capacity(512);

  for i in 0..512u32 {
    buffer.push(i).expect("within capacity");
  }

  assert_eq!(buffer.len(), 512);
  assert_eq!(cursor.counts().total, 0, "512 pushes must touch no atomic at all");
}

#[test]
fn one_flush_lands_everything_for_one_atomic_operation() {
  let cursor = CountingSeq::default();
  let mut buffer = TlsBuffer::with_capacity(64);
  for i in 0..64u32 {
    buffer.push(i).unwrap();
  }

  let flush = buffer.flush_into(&cursor, Ordering::AcqRel);
  let claim = flush.claim();
  let landed: Vec<(Seq, u32)> = flush.collect();

  assert_eq!(claim.len(), 64, "all 64 in one claim");
  assert_eq!(landed.len(), 64);
  assert_eq!(cursor.counts().fetch_adds, 1);
  assert_eq!(cursor.counts().total, 1, "64 items, one atomic operation");
}

#[test]
fn accumulating_is_free_and_landing_is_one_op_in_the_same_run() {
  // Both halves measured across one uninterrupted run, so neither can be met
  // by a buffer that merely relocates the traffic it claims to avoid.
  let cursor = CountingSeq::default();
  let mut buffer = TlsBuffer::with_capacity(128);

  for i in 0..128u64 {
    buffer.push(i).unwrap();
  }
  let after_accumulation = cursor.counts().total;

  let count = buffer.flush_into(&cursor, Ordering::AcqRel).count();
  let after_flush = cursor.counts().total;

  assert_eq!(after_accumulation, 0);
  assert_eq!(after_flush, 1);
  assert_eq!(count, 128, "and every item actually landed");
}

#[test]
fn the_claim_is_contiguous_and_in_staging_order() {
  let cursor = AtomicSeq::new(Seq(1_000));
  let mut buffer = TlsBuffer::with_capacity(8);
  for c in ['a', 'b', 'c', 'd', 'e'] {
    buffer.push(c).unwrap();
  }

  let landed: Vec<(Seq, char)> = buffer.flush_into(&cursor, Ordering::AcqRel).collect();

  assert_eq!(
    landed,
    vec![
      (Seq(1_000), 'a'),
      (Seq(1_001), 'b'),
      (Seq(1_002), 'c'),
      (Seq(1_003), 'd'),
      (Seq(1_004), 'e'),
    ],
    "staging order against ascending contiguous sequences"
  );
  assert_eq!(cursor.load(Ordering::Acquire), Seq(1_005));
}

#[test]
fn the_flush_lands_in_real_storage_in_the_order_it_was_staged() {
  // The end-to-end shape: stage, claim once, write each item through the shared
  // publish path, read the ring back. Order survives the whole trip.
  let capacity = cap(16);
  let cursor = CountingSeq::default();
  let mut ring: Buffer<TypedSlot<u32>> = Buffer::new(capacity);
  let mut staging = TlsBuffer::with_capacity(8);

  for i in 0..8u32 {
    staging.push(i * 11).unwrap();
  }

  for (seq, item) in staging.flush_into(&cursor, Ordering::AcqRel) {
    publish_into(ring.at_mut(seq), item).unwrap();
  }

  assert_eq!(cursor.counts().total, 1, "the whole landing cost one operation");
  for i in 0..8u64 {
    assert_eq!(drain_from(ring.at(Seq(i))), Some(&(i as u32 * 11)));
  }
}

// -------------------------------------------------------------- the boundedness

#[test]
fn a_full_buffer_refuses_rather_than_growing() {
  let mut buffer = TlsBuffer::with_capacity(3);

  assert_eq!(buffer.push(1u8), Ok(()));
  assert_eq!(buffer.push(2u8), Ok(()));
  assert_eq!(buffer.push(3u8), Ok(()));
  assert_eq!(buffer.push(4u8), Err(RingError::Full));

  assert_eq!(buffer.len(), 3, "the refused push added nothing");
  assert!(buffer.is_full());
}

#[test]
fn the_refusal_is_transient_because_a_flush_clears_it() {
  let cursor = AtomicSeq::default();
  let mut buffer = TlsBuffer::with_capacity(2);
  buffer.push(1u8).unwrap();
  buffer.push(2u8).unwrap();

  assert_eq!(buffer.push(3u8), Err(RingError::Full));
  assert!(RingError::Full.is_transient(), "so a caller knows to flush and retry");

  let _ = buffer.flush_into(&cursor, Ordering::AcqRel).count();
  assert_eq!(buffer.push(3u8), Ok(()));
}

#[test]
fn is_full_is_the_signal_a_flush_policy_reads() {
  let mut buffer = TlsBuffer::with_capacity(4);

  for expected in [false, false, false, false] {
    assert_eq!(buffer.is_full(), expected);
    buffer.push(0u8).unwrap();
  }
  assert!(buffer.is_full(), "full exactly at capacity, not one before or after");
}

#[test]
fn a_buffer_reports_its_own_shape() {
  let mut buffer = TlsBuffer::with_capacity(4);

  assert_eq!(buffer.capacity(), 4);
  assert!(buffer.is_empty());
  assert_eq!(buffer.len(), 0);

  buffer.push(1u8).unwrap();

  assert!(!buffer.is_empty());
  assert_eq!(buffer.len(), 1);
  assert_eq!(buffer.capacity(), 4, "capacity does not move");
}

#[test]
fn a_zero_capacity_buffer_accepts_nothing_and_is_full_from_the_start() {
  // Degenerate but reachable. A flush policy configured to flush every item
  // would produce it. It must refuse rather than panic or grow.
  let mut buffer = TlsBuffer::with_capacity(0);

  assert!(buffer.is_full());
  assert!(buffer.is_empty());
  assert_eq!(buffer.push(1u8), Err(RingError::Full));
}

// -------------------------------------------------------------- flush and discard

#[test]
fn flushing_an_empty_buffer_claims_nothing_but_still_costs_its_operation() {
  // Documented behaviour, asserted so it cannot drift into a silent skip. A
  // conditional would make the operation count depend on the data, and the
  // whole value of the counting shim is that it does not.
  let cursor = CountingSeq::default();
  let mut buffer = TlsBuffer::<u8>::with_capacity(4);

  let flush = buffer.flush_into(&cursor, Ordering::AcqRel);
  let claim = flush.claim();
  let landed = flush.count();

  assert_eq!(claim, BatchClaim::new(Seq::ZERO, 0));
  assert_eq!(landed, 0);
  assert_eq!(cursor.counts().total, 1);
  assert_eq!(
    cursor.load(Ordering::Acquire),
    Seq::ZERO,
    "and advanced the cursor by nothing"
  );
}

#[test]
fn a_flush_empties_the_buffer_even_when_the_iterator_is_dropped_unread() {
  // The sequences are already claimed by then. Leaving the items staged would
  // let them be claimed a second time, and two claims on one sequence is the
  // one thing the protocol may never produce.
  let cursor = AtomicSeq::default();
  let mut buffer = TlsBuffer::with_capacity(8);
  for i in 0..5u8 {
    buffer.push(i).unwrap();
  }

  drop(buffer.flush_into(&cursor, Ordering::AcqRel));

  assert!(buffer.is_empty(), "the staged items are gone");
  assert_eq!(cursor.load(Ordering::Acquire), Seq(5), "and the cursor stayed advanced");
}

#[test]
fn a_partially_consumed_flush_still_empties_the_buffer() {
  let cursor = AtomicSeq::default();
  let mut buffer = TlsBuffer::with_capacity(8);
  for i in 0..6u8 {
    buffer.push(i).unwrap();
  }

  {
    let mut flush = buffer.flush_into(&cursor, Ordering::AcqRel);
    assert_eq!(flush.next(), Some((Seq::ZERO, 0)));
    assert_eq!(flush.next(), Some((Seq(1), 1)));
  }

  assert!(buffer.is_empty());
  assert_eq!(cursor.load(Ordering::Acquire), Seq(6));
}

#[test]
fn discard_drops_the_items_and_advances_no_cursor() {
  // The shutdown path. A discard that claimed sequences would leave a permanent
  // hole nobody ever publishes into, and a consumer would wait on it forever.
  let cursor = AtomicSeq::default();
  let mut buffer = TlsBuffer::with_capacity(8);
  for i in 0..4u8 {
    buffer.push(i).unwrap();
  }

  buffer.discard();

  assert!(buffer.is_empty());
  assert_eq!(cursor.load(Ordering::Acquire), Seq::ZERO, "no sequence was claimed");
  assert_eq!(buffer.capacity(), 8, "and the buffer is reusable");
}

#[test]
fn drain_takes_the_items_and_advances_no_cursor() {
  // The unfused half of the pair. `flush_into` claims and drains together,
  // which is right when the destination is a sequenced ring; `drain` is for a
  // destination that decides for itself whether it can accept the batch, and
  // must therefore be asked *before* the buffer is emptied. `ring_flush` is
  // that caller. Its rejected flushes leave the records staged, which is only
  // possible because taking them and claiming sequences are separate calls.
  let cursor = AtomicSeq::default();
  let mut buffer = TlsBuffer::with_capacity(8);
  for i in 0..4u8 {
    buffer.push(i).unwrap();
  }

  let taken: Vec<u8> = buffer.drain().collect();

  assert_eq!(taken, vec![0, 1, 2, 3], "staging order");
  assert!(buffer.is_empty(), "and the buffer is writable again");
  assert_eq!(cursor.load(Ordering::Acquire), Seq::ZERO, "no sequence was claimed");
  assert_eq!(buffer.capacity(), 8);
}

#[test]
fn drain_empties_the_buffer_even_when_the_iterator_is_dropped_unread() {
  // `Vec::Drain`'s own contract, inherited whole, and the reason a caller that
  // might not accept every record has to decide before calling rather than
  // after. There is no point after the call at which the records are still
  // staged.
  let mut buffer = TlsBuffer::with_capacity(8);
  for i in 0..4u8 {
    buffer.push(i).unwrap();
  }

  drop(buffer.drain());

  assert!(buffer.is_empty(), "dropping the drain unread left records staged");

  let mut buffer = TlsBuffer::with_capacity(8);
  for i in 0..4u8 {
    buffer.push(i).unwrap();
  }
  let mut partial = buffer.drain();
  assert_eq!(partial.next(), Some(0));
  drop(partial);
  assert!(buffer.is_empty(), "a partly-read drain left the remainder staged");
}

#[test]
fn draining_an_empty_buffer_yields_nothing_and_costs_no_operation() {
  // Unlike `flush_into`, which claims a zero-length run and still pays one
  // atomic, `drain` touches no cursor at all. A caller polling an idle
  // buffer pays nothing for the poll.
  let mut buffer = TlsBuffer::<u8>::with_capacity(4);

  let taken: Vec<u8> = buffer.drain().collect();

  assert!(taken.is_empty());
  assert!(buffer.is_empty());
}

#[test]
fn the_claim_is_readable_before_the_items_are_consumed() {
  // So a caller can gate the range against a consumer barrier before it starts
  // writing into slots.
  let cursor = AtomicSeq::new(Seq(40));
  let mut buffer = TlsBuffer::with_capacity(4);
  for i in 0..3u8 {
    buffer.push(i).unwrap();
  }

  let flush = buffer.flush_into(&cursor, Ordering::AcqRel);
  let claim = flush.claim();

  assert_eq!(claim.start(), Seq(40));
  assert_eq!(claim.end(), Seq(43));
  assert_eq!(flush.len(), 3, "and the iterator still holds all three");
}

#[test]
fn a_flush_reports_its_remaining_length_exactly() {
  let cursor = AtomicSeq::default();
  let mut buffer = TlsBuffer::with_capacity(8);
  for i in 0..4u8 {
    buffer.push(i).unwrap();
  }

  let mut flush = buffer.flush_into(&cursor, Ordering::AcqRel);
  assert_eq!(flush.len(), 4);
  assert_eq!(flush.size_hint(), (4, Some(4)));

  flush.next();
  assert_eq!(flush.len(), 3);
  assert_eq!(flush.size_hint(), (3, Some(3)));
}

#[test]
fn successive_flushes_continue_where_the_last_one_stopped() {
  let cursor = AtomicSeq::default();
  let mut buffer = TlsBuffer::with_capacity(4);

  let mut all: Vec<(Seq, u8)> = Vec::new();
  for round in 0..3u8 {
    for i in 0..4u8 {
      buffer.push(round * 4 + i).unwrap();
    }
    all.extend(buffer.flush_into(&cursor, Ordering::AcqRel));
  }

  let seqs: Vec<u64> = all.iter().map(|(s, _)| s.0).collect();
  let items: Vec<u8> = all.iter().map(|(_, i)| *i).collect();

  assert_eq!(seqs, (0..12).collect::<Vec<_>>(), "no gap between flushes");
  assert_eq!(items, (0..12u8).collect::<Vec<_>>());
}

// ------------------------------------------------------- the ordering precondition

#[test]
#[should_panic(expected = "flush_into's order must include Release semantics")]
fn flush_into_rejects_an_order_without_release_semantics() {
  // `Relaxed` compiles and returns a valid-looking `Flush`, and the claim still
  // lands correct, non-overlapping sequences. But the items behind them
  // would never become visible to a consumer reading the cursor per this
  // function's own doc comment. The `debug_assert!` is the only thing
  // standing between that silent failure and a caller who passed the wrong
  // `Ordering`; until this test, nothing would notice if it were weakened or
  // deleted.
  let cursor = AtomicSeq::default();
  let mut buffer = TlsBuffer::with_capacity(4);
  buffer.push(1u8).unwrap();

  let _ = buffer.flush_into(&cursor, Ordering::Relaxed);
}

// ----------------------------------------------------------------- contention

#[test]
fn several_threads_flushing_never_receive_overlapping_sequences() {
  // This is the point of the whole design. Each thread accumulates privately
  // with no coordination at all, and the only shared step is the one fetch_add
  // per flush, which must still partition the sequence space exactly.
  const THREADS: usize = 4;
  const FLUSHES: usize = 200;
  const BATCH: usize = 8;

  let cursor = AtomicSeq::default();
  let cursor = &cursor;

  let landed: Vec<(Seq, usize)> = thread::scope(|scope| {
    let handles: Vec<_> = (0..THREADS)
      .map(|id| {
        scope.spawn(move || {
          let mut buffer = TlsBuffer::with_capacity(BATCH);
          let mut mine = Vec::with_capacity(FLUSHES * BATCH);
          for _ in 0..FLUSHES {
            for _ in 0..BATCH {
              buffer.push(id).unwrap();
            }
            mine.extend(buffer.flush_into(cursor, Ordering::AcqRel));
          }
          mine
        })
      })
      .collect();
    handles
      .into_iter()
      .flat_map(|h| h.join().expect("flusher panicked"))
      .collect()
  });

  assert_eq!(landed.len(), THREADS * FLUSHES * BATCH);

  let mut seen: HashSet<u64> = HashSet::with_capacity(landed.len());
  for (seq, _) in &landed {
    assert!(seen.insert(seq.0), "sequence {} was handed out twice", seq.0);
  }
  assert_eq!(seen.len(), landed.len());
  assert_eq!(cursor.load(Ordering::Acquire), Seq(landed.len() as u64));
}

#[test]
fn a_threads_own_items_stay_contiguous_within_each_flush() {
  // A system's own writes survive the merge in order. Between
  // threads the interleaving is arbitrary; within one flush it is not.
  const THREADS: usize = 4;
  const FLUSHES: usize = 100;
  const BATCH: usize = 6;

  let cursor = AtomicSeq::default();
  let cursor = &cursor;

  let per_thread: Vec<Vec<Vec<(Seq, usize)>>> = thread::scope(|scope| {
    let handles: Vec<_> = (0..THREADS)
      .map(|_| {
        scope.spawn(move || {
          let mut buffer = TlsBuffer::with_capacity(BATCH);
          (0..FLUSHES)
            .map(|round| {
              for i in 0..BATCH {
                buffer.push(round * BATCH + i).unwrap();
              }
              buffer.flush_into(cursor, Ordering::AcqRel).collect::<Vec<_>>()
            })
            .collect()
        })
      })
      .collect();
    handles.into_iter().map(|h| h.join().expect("flusher panicked")).collect()
  });

  for batches in &per_thread {
    for batch in batches {
      assert_eq!(batch.len(), BATCH);
      for pair in batch.windows(2) {
        assert_eq!(pair[1].0.0, pair[0].0.0 + 1, "a flush must be contiguous");
        assert_eq!(pair[1].1, pair[0].1 + 1, "and carry the items in staging order");
      }
    }
    for pair in batches.windows(2) {
      assert!(
        pair[0].last().unwrap().0.0 < pair[1][0].0.0,
        "a thread's later flush must start after its earlier one ended"
      );
    }
  }
}
