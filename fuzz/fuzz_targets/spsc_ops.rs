//! Operation-sequence fuzzing for the SPSC ring: every `try_push` must either
//! land (and come back out in FIFO order) or report `Full` exactly when the
//! ring is full, and every `claim` + write + drop must do the same through
//! the [`Reservation`] guard path.
//!
//! Single-threaded on purpose: thread interleavings are loom's beat
//! (`ring_spsc`'s model tests); this covers the input space loom never sees —
//! capacity edges, wrap-around over many laps, refused claims, partial batch
//! drains.
//!
//! [`Reservation`]: ring_spsc::Reservation

#![no_main]

use libfuzzer_sys::fuzz_target;
use ring_slot::TypedSlot;
use ring_spsc::Ring;
use ring_types::Capacity;
use std::collections::VecDeque;

#[path = "common.rs"]
mod common;

use common::Reader;

const CAPACITIES: [usize; 6] = [1, 2, 4, 8, 32, 128];
const OPS: usize = 1024;

/// Drain everything and check it against the oracle, in order.
fn drain_all(
  consumer: &mut ring_spsc::Consumer<'_, TypedSlot<u64>>,
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
  let mut ring: Ring<TypedSlot<u64>> = Ring::new(Capacity::new(capacity).unwrap());
  let (mut producer, mut consumer) = ring.split();
  let mut oracle: VecDeque<u64> = VecDeque::new();

  for _ in 0..OPS {
    match reader.byte() % 5 {
      // Fused push: lands and is remembered, or comes back with Full reported
      // exactly when the ring is full.
      0 => {
        let record = reader.intake();
        match producer.try_push(record) {
          Ok(()) => oracle.push_back(record),
          Err(returned) => {
            assert_eq!(returned, record, "a refused push must hand the record back");
            assert_eq!(oracle.len(), capacity, "Full only when full");
          }
        }
      }
      // Claim + write + publish through the guard: the same contract as the
      // fused push, through the reservation path that owns the unsafe.
      1 => {
        let record = reader.intake();
        match producer.claim() {
          Ok(mut reservation) => {
            reservation.set(record);
            drop(reservation);
            oracle.push_back(record);
          }
          Err(ring_types::RingError::Full) => {
            assert_eq!(oracle.len(), capacity, "Full only when full");
          }
          Err(other) => panic!("unexpected claim error: {other:?}"),
        }
      }
      // Drain everything, in order.
      2 => drain_all(&mut consumer, &mut oracle),
      // Bounded drain: the first `n` records, in order.
      3 => {
        let max = reader.byte() as usize % (capacity + 1);
        let mut batch = consumer.drain_up_to(max);
        for offset in 0..batch.len() {
          let got = batch.get_mut(offset).and_then(|slot| slot.take());
          assert_eq!(got, oracle.pop_front(), "records must come back FIFO");
        }
      }
      // Refused claim on purpose: fill accounting must already agree.
      _ => {
        if oracle.len() == capacity {
          assert!(producer.claim().is_err(), "a full ring refuses the claim");
        }
      }
    }
  }

  drain_all(&mut consumer, &mut oracle);
  assert!(oracle.is_empty(), "everything pushed must be drainable");
});
