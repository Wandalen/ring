//! Claim, write, publish: the producer path in three stages.
//!
//! `micro::push_pop` measures the fused push. This splits it open, because
//! the stages scale differently: `claim` is the only contended step (a cached
//! check on the exclusive path, a compare-exchange on the shared one),
//! the slot write is unsynchronised, and publish is exactly one release
//! store — [`Reservation`]'s and [`Reserved`]'s drops, which is why publish
//! has no bench of its own and is measured inside every `claim` bench as the
//! guard's drop. Comparing `claim` (no write) against `fused` (claim, write,
//! publish) attributes the write's share by subtraction.
//!
//! [`Reservation`]: ring_spsc::Reservation
//! [`Reserved`]: ring_mpsc::Reserved

use std::hint::black_box;

use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use ring_slot::TypedSlot;
use ring_types::Capacity;

const CAPACITY: usize = 1024;
const BURST: usize = 256;
const GROUP: usize = 32;

/// Claim without write: the reservation round-trips empty, so the write's
/// share drops out and what is left is claim plus the publish store on drop.
fn claim_spsc_fast(c: &mut Criterion) {
  let mut group = c.benchmark_group("claim");
  group.throughput(Throughput::Elements(BURST as u64));
  let mut ring: ring_spsc::Ring<TypedSlot<u64>> = ring_spsc::Ring::new(Capacity::new(CAPACITY).unwrap());
  let (mut producer, mut consumer) = ring.split();
  group.bench_function("spsc_fast", |b| {
    b.iter_batched(
      || {
        let _ = consumer.drain();
      },
      |_| {
        for _ in 0..BURST {
          drop(black_box(producer.claim()).unwrap());
        }
      },
      BatchSize::PerIteration,
    );
  });
  group.finish();
}

/// The refused claim: the cache reports full, one `Acquire` load of the
/// consumer cursor refreshes it, the ring is still full. That single load is
/// the whole measurement.
fn claim_spsc_refused(c: &mut Criterion) {
  let mut group = c.benchmark_group("claim");
  group.throughput(Throughput::Elements(1));
  let mut ring: ring_spsc::Ring<TypedSlot<u64>> = ring_spsc::Ring::new(Capacity::new(CAPACITY).unwrap());
  let (mut producer, _consumer) = ring.split();
  for record in 0..CAPACITY as u64 {
    producer.try_push(record).unwrap();
  }
  group.bench_function("spsc_refused", |b| {
    b.iter(|| assert!(black_box(producer.claim()).is_err()));
  });
  group.finish();
}

/// The shared claim: one compare-exchange per sequence, uncontended here.
fn claim_mpsc_shared(c: &mut Criterion) {
  let mut group = c.benchmark_group("claim");
  group.throughput(Throughput::Elements(BURST as u64));
  let mut ring: ring_mpsc::Ring<TypedSlot<u64>> = ring_mpsc::Ring::new(Capacity::new(CAPACITY).unwrap());
  let mut ends = ring.ends();
  let (producer, mut consumer) = ends.split();
  group.bench_function("mpsc_shared", |b| {
    b.iter_batched(
      || {
        let _ = consumer.drain();
      },
      |_| {
        for _ in 0..BURST {
          drop(black_box(producer.claim()).unwrap());
        }
      },
      BatchSize::PerIteration,
    );
  });
  group.finish();
}

/// One gate check and one exchange per grant of up to 32, amortised here per
/// sequence against a drained ring, which always grants the full width.
fn claim_mpsc_batch(c: &mut Criterion) {
  let mut group = c.benchmark_group("claim");
  group.throughput(Throughput::Elements(GROUP as u64));
  let mut ring: ring_mpsc::Ring<TypedSlot<u64>> = ring_mpsc::Ring::new(Capacity::new(CAPACITY).unwrap());
  let mut ends = ring.ends();
  let (producer, mut consumer) = ends.split();
  group.bench_function("mpsc_batch32", |b| {
    b.iter_batched(
      || {
        let _ = consumer.drain();
      },
      |_| {
        let grant = producer.claim_batch(GROUP).unwrap();
        assert_eq!(grant.len(), GROUP);
        drop(grant);
      },
      BatchSize::PerIteration,
    );
  });
  group.finish();
}

/// The primary's exclusive claim: the cached-cursor fast path with no
/// compare-exchange.
fn claim_mpsc_primary(c: &mut Criterion) {
  let mut group = c.benchmark_group("claim");
  group.throughput(Throughput::Elements(BURST as u64));
  let mut ring: ring_mpsc::Ring<TypedSlot<u64>> = ring_mpsc::Ring::new(Capacity::new(CAPACITY).unwrap());
  let mut ends = ring.ends();
  let (mut tx, mut consumer) = ends.split();
  let mut primary = tx.primary();
  group.bench_function("mpsc_primary", |b| {
    b.iter_batched(
      || {
        let _ = consumer.drain();
      },
      |_| {
        for _ in 0..BURST {
          drop(black_box(primary.claim()).unwrap());
        }
      },
      BatchSize::PerIteration,
    );
  });
  group.finish();
}

/// The slot write with no synchronisation at all: one guard held outside the
/// loop, `set` timed alone.
fn write_spsc_slot(c: &mut Criterion) {
  let mut group = c.benchmark_group("write");
  group.throughput(Throughput::Elements(1));
  let mut ring: ring_spsc::Ring<TypedSlot<u64>> = ring_spsc::Ring::new(Capacity::new(CAPACITY).unwrap());
  let (mut producer, _consumer) = ring.split();
  let mut reservation = producer.claim().unwrap();
  let mut next = 0u64;
  group.bench_function("spsc_slot", |b| {
    b.iter(|| {
      next += 1;
      black_box(reservation.set(black_box(next)));
    });
  });
  group.finish();
}

/// The shared slot write, same shape through the `Reserved` guard.
fn write_mpsc_slot(c: &mut Criterion) {
  let mut group = c.benchmark_group("write");
  group.throughput(Throughput::Elements(1));
  let mut ring: ring_mpsc::Ring<TypedSlot<u64>> = ring_mpsc::Ring::new(Capacity::new(CAPACITY).unwrap());
  let mut ends = ring.ends();
  let (producer, _consumer) = ends.split();
  let mut reserved = producer.claim().unwrap();
  let mut next = 0u64;
  group.bench_function("mpsc_slot", |b| {
    b.iter(|| {
      next += 1;
      black_box(reserved.set(black_box(next)));
    });
  });
  group.finish();
}

/// Read every slot of a drained batch back, anchor the checksum in a
/// `black_box`, and assert its exact value, so the pushes that filled it
/// stay observable to the optimiser (see the fused benches). The assert is
/// the proof: with dead stores the slots would hold stale data and the
/// expected sum would miss loudly instead of benching nothing.
fn observe(len: usize, expected: u64, load: impl Fn(usize) -> Option<u64>) {
  let mut sum = 0u64;
  for offset in 0..len {
    if let Some(record) = load(offset) {
      sum = sum.wrapping_add(record);
    }
  }
  assert_eq!(black_box(sum), expected, "every pushed record must come back");
}

/// `0 + 1 + .. + n - 1`: the checksum every fused batch must read back,
/// since every routine pushes exactly the range `0..n`.
fn sum_to(n: u64) -> u64 {
  n * (n - 1) / 2
}

/// The fused push: claim, write, publish per record, the shape production
/// code actually calls.
///
/// The setup is an observing drain (see `observe`): it runs outside the
/// timing window under `PerIteration`, so observing costs the report
/// nothing.
fn fused_spsc_push(c: &mut Criterion) {
  let mut group = c.benchmark_group("fused");
  group.throughput(Throughput::Elements(BURST as u64));
  let mut ring: ring_spsc::Ring<TypedSlot<u64>> = ring_spsc::Ring::new(Capacity::new(CAPACITY).unwrap());
  let (mut producer, mut consumer) = ring.split();
  group.bench_function("spsc_push", |b| {
    b.iter_batched(
      || {
        let batch = consumer.drain();
        if batch.len() == BURST {
          observe(batch.len(), sum_to(BURST as u64), |o| {
            batch.get(o).and_then(|slot| slot.get()).copied()
          });
        }
      },
      |_| {
        for record in 0..BURST as u64 {
          assert!(producer.try_push(black_box(record)).is_ok());
        }
      },
      BatchSize::PerIteration,
    );
  });
  group.finish();
}

/// The fused shared push, same shape through `Producer::push`.
fn fused_mpsc_push(c: &mut Criterion) {
  let mut group = c.benchmark_group("fused");
  group.throughput(Throughput::Elements(BURST as u64));
  let mut ring: ring_mpsc::Ring<TypedSlot<u64>> = ring_mpsc::Ring::new(Capacity::new(CAPACITY).unwrap());
  let mut ends = ring.ends();
  let (producer, mut consumer) = ends.split();
  group.bench_function("mpsc_push", |b| {
    b.iter_batched(
      || {
        let batch = consumer.drain();
        if batch.len() == BURST {
          observe(batch.len(), sum_to(BURST as u64), |o| {
            batch.get(o).and_then(|slot| slot.get()).copied()
          });
        }
      },
      |_| {
        for record in 0..BURST as u64 {
          assert!(producer.push(black_box(record)).is_ok());
        }
      },
      BatchSize::PerIteration,
    );
  });
  group.finish();
}

/// The fused batch push: one grant per 32 records.
fn fused_mpsc_push_batch(c: &mut Criterion) {
  let mut group = c.benchmark_group("fused");
  group.throughput(Throughput::Elements(GROUP as u64));
  let mut ring: ring_mpsc::Ring<TypedSlot<u64>> = ring_mpsc::Ring::new(Capacity::new(CAPACITY).unwrap());
  let mut ends = ring.ends();
  let (producer, mut consumer) = ends.split();
  group.bench_function("mpsc_push_batch", |b| {
    b.iter_batched(
      || {
        let batch = consumer.drain();
        if batch.len() == GROUP {
          observe(batch.len(), sum_to(GROUP as u64), |o| {
            batch.get(o).and_then(|slot| slot.get()).copied()
          });
        }
        (0..GROUP as u64).map(black_box).collect::<Vec<_>>()
      },
      |mut records| {
        assert_eq!(producer.push_batch(&mut records).unwrap(), GROUP);
      },
      BatchSize::PerIteration,
    );
  });
  group.finish();
}

criterion_group!(
  benches,
  claim_spsc_fast,
  claim_spsc_refused,
  claim_mpsc_shared,
  claim_mpsc_batch,
  claim_mpsc_primary,
  write_spsc_slot,
  write_mpsc_slot,
  fused_spsc_push,
  fused_mpsc_push,
  fused_mpsc_push_batch
);
criterion_main!(benches);
