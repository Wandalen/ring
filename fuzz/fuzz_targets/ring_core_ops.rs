//! Operation-sequence fuzzing for the composed `ring_core` ring: the SPSC and
//! MPSC backends behind one surface, plus the overflow/batch surface the
//! backend crates never see alone.
//!
//! `spsc_ops` and `mpsc_ops` cover each backend's own API; this covers what
//! only exists here — backend selection by producer count, `try_push_batch`
//! with its hand-back-the-refused-record contract, `try_recv_batch`, and the
//! overflow policy mapped through `would_resolve`. Single-threaded like the
//! rest: interleavings belong to loom.

#![no_main]

use libfuzzer_sys::fuzz_target;
use ring_config::RingConfig;
use ring_core::Ring;
use ring_types::OverflowPolicy;
use std::collections::VecDeque;

#[path = "common.rs"]
mod common;

use common::{CAPACITIES, OPS, Reader};

fuzz_target!(|data: &[u8]| {
  let mut reader = Reader::new(data);
  let capacity = CAPACITIES[reader.byte() as usize % CAPACITIES.len()];
  let producers = if reader.byte() % 2 == 0 { 1 } else { 4 };
  let overflow = if reader.byte() % 2 == 0 {
    OverflowPolicy::Fail
  } else {
    OverflowPolicy::DropNewest
  };
  let config = RingConfig::new(capacity)
    .unwrap()
    .with_overflow(overflow)
    .with_producers(producers);
  let mut ring: Ring<u64> = Ring::new(&config).unwrap();
  let mut ends = ring.ends();
  let (mut producer, mut consumer) = ends.split();
  let mut oracle: VecDeque<u64> = VecDeque::new();

  for _ in 0..OPS {
    // Room for another record, by the oracle's own books.
    let room = || capacity - oracle.len();
    match reader.byte() % 5 {
      // Single push: lands when there is room; on a full ring Fail hands
      // the record back and DropNewest discards it with success.
      0 => {
        let record = reader.intake();
        let had_room = room() > 0;
        match producer.try_push(record) {
          Ok(()) => {
            if had_room {
              oracle.push_back(record);
            }
          }
          Err(returned) => {
            assert_eq!(returned, record, "a refused push must hand the record back");
            assert_eq!(room(), 0, "Fail refuses only when full");
          }
        }
      }
      // Batch push: `Ok(n)` means the iterator ran dry with `n` landed;
      // `Err((n, record))` hands back what did not go in. Under DropNewest
      // a full ring still answers `Ok`, discarding the roomless tail.
      1 => {
        let width = 1 + reader.byte() as usize % 16;
        let intended: Vec<u64> = (0..width).map(|_| reader.intake()).collect();
        let landable = room().min(width);
        let mut offering = intended.clone().into_iter();
        match producer.try_push_batch(&mut offering) {
          Ok(landed) => {
            if overflow == OverflowPolicy::Fail {
              assert_eq!(landed, width, "Ok means nothing was refused");
            }
            oracle.extend(intended.into_iter().take(landable));
          }
          Err((landed, record)) => {
            assert_eq!(overflow, OverflowPolicy::Fail, "only Fail refuses a batch");
            assert_eq!(landed, landable, "lands exactly what fits");
            assert_eq!(record, intended[landed], "the refused record is the first unlanded one");
            oracle.extend(intended.into_iter().take(landed));
          }
        }
      }
      // Single receive: FIFO or empty.
      2 => match consumer.try_recv() {
        Some(got) => assert_eq!(Some(got), oracle.pop_front(), "records must come back FIFO"),
        None => assert!(oracle.is_empty(), "an empty ring reads empty"),
      },
      // Batch receive: exactly what the oracle holds first.
      3 => {
        let mut out = Vec::new();
        let taken = consumer.try_recv_batch(&mut out);
        assert_eq!(taken, out.len(), "reported count matches what landed");
        let expected: Vec<u64> = oracle.drain(..taken).collect();
        assert_eq!(out, expected, "records must come back FIFO");
      }
      // Refusal probe: a full ring refuses under Fail, discards under
      // DropNewest. Either way nothing must be lost or invented.
      _ => {
        if room() == 0 {
          let record = reader.intake();
          match producer.try_push(record) {
            Ok(()) => assert_eq!(overflow, OverflowPolicy::DropNewest),
            Err(_) => assert_eq!(overflow, OverflowPolicy::Fail),
          }
        }
      }
    }
  }

  while let Some(got) = consumer.try_recv() {
    assert_eq!(Some(got), oracle.pop_front(), "records must come back FIFO");
  }
  assert!(oracle.is_empty(), "everything pushed must be drainable");
});
