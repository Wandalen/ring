//! Operation-sequence fuzzing for the MPSC ring: `push`, `push_batch` and
//! `claim_batch` against a FIFO oracle, including adaptive partial grants on
//! a nearly-full ring.
//!
//! Single-threaded on purpose, like `spsc_ops`: the contended interleavings
//! belong to loom; this owns the input space — batch widths against
//! headroom, partial grants, wrap-around, refused claims.

#![no_main]

use libfuzzer_sys::fuzz_target;
use ring_slot::TypedSlot;
use ring_types::{Capacity, RingError};
use std::collections::VecDeque;

#[path = "common.rs"]
mod common;

use common::Reader;

const CAPACITIES: [usize; 6] = [1, 2, 4, 8, 32, 128];
const OPS: usize = 1024;

fn drain_all(
  consumer: &mut ring_mpsc::Consumer<'_, TypedSlot<u64>>,
  oracle: &mut VecDeque<u64>,
) {
  let mut batch = consumer.drain();
  for offset in 0..batch.len() {
    let got = batch.get_mut(offset).and_then(|slot| slot.take());
    assert_eq!(got, oracle.pop_front(), "records must come back FIFO");
  }
}

fuzz_target!(|data: &[u8]| {
  let mut reader = Reader::new(data);
  let capacity = CAPACITIES[reader.byte() as usize % CAPACITIES.len()];
  let mut ring: ring_mpsc::Ring<TypedSlot<u64>> = ring_mpsc::Ring::new(Capacity::new(capacity).unwrap());
  let mut ends = ring.ends();
  let (producer, mut consumer) = ends.split();
  let mut oracle: VecDeque<u64> = VecDeque::new();

  for _ in 0..OPS {
    match reader.byte() % 6 {
      // Single push: lands or reports Full exactly when full. (`push`
      // reports `Full` as the error; the record stays with the caller only
      // in the batch form below.)
      0 => {
        let record = reader.intake();
        match producer.push(record) {
          Ok(_) => oracle.push_back(record),
          Err(RingError::Full) => {
            assert_eq!(oracle.len(), capacity, "Full only when full");
          }
          Err(other) => panic!("unexpected push error: {other:?}"),
        }
      }
      // Batch push: the granted prefix lands in order, the rest stays for
      // the next attempt; Full leaves the vector untouched.
      1 => {
        let width = 1 + reader.byte() as usize % 16;
        let mut records: Vec<u64> = (0..width).map(|_| reader.intake()).collect();
        let intended = records.clone();
        match producer.push_batch(&mut records) {
          Ok(landed) => {
            assert!(landed <= intended.len(), "cannot land more than offered");
            assert_eq!(&records[..], &intended[landed..], "the rest stays, in order");
            oracle.extend(intended.into_iter().take(landed));
          }
          Err(RingError::Full) => {
            assert_eq!(&records[..], &intended[..], "Full touches nothing");
            assert_eq!(oracle.len(), capacity, "Full only when full");
          }
          Err(other) => panic!("unexpected push_batch error: {other:?}"),
        }
      }
      // Batch claim + hand write + drop: the grant may be narrower than
      // asked (adaptive), every granted slot is written and published.
      2 => {
        let width = 1 + reader.byte() as usize % 16;
        match producer.claim_batch(width) {
          Ok(mut grant) => {
            let granted = grant.len();
            assert!((1..=width).contains(&granted), "grant within the asked width");
            for offset in 0..granted {
              let record = reader.intake();
              grant.slot_mut(offset).expect("offset within the grant").set(record);
              oracle.push_back(record);
            }
            drop(grant);
          }
          Err(RingError::Full) => {
            assert_eq!(oracle.len(), capacity, "Full only when full");
          }
          Err(other) => panic!("unexpected claim_batch error: {other:?}"),
        }
      }
      // Single claim + write + publish through the guard.
      3 => {
        let record = reader.intake();
        match producer.claim() {
          Ok(mut reserved) => {
            reserved.set(record);
            drop(reserved);
            oracle.push_back(record);
          }
          Err(RingError::Full) => {
            assert_eq!(oracle.len(), capacity, "Full only when full");
          }
          Err(other) => panic!("unexpected claim error: {other:?}"),
        }
      }
      // Drain everything, in order.
      4 => drain_all(&mut consumer, &mut oracle),
      // Bounded drain.
      _ => {
        let max = reader.byte() as usize % (capacity + 1);
        let mut batch = consumer.drain_up_to(max);
        for offset in 0..batch.len() {
          let got = batch.get_mut(offset).and_then(|slot| slot.take());
          assert_eq!(got, oracle.pop_front(), "records must come back FIFO");
        }
      }
    }
  }

  drain_all(&mut consumer, &mut oracle);
  assert!(oracle.is_empty(), "everything pushed must be drainable");
});
