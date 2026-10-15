//! Overflow policies at a full ring: what `Fail` and `DropNewest` cost per
//! refused arrival, on each in-house backend through `ring_core`.
//!
//! `DropOldest` has no bench here. The in-house backends refuse it at
//! construction and only the interim crossbeam backend honours it (covered by
//! its own test, not by a bench): there is no shippable native configuration
//! to measure.

use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use ring_config::RingConfig;
use ring_core::Ring;
use ring_types::OverflowPolicy;

const CAPACITY: usize = 8;

/// A full ring: every slot pushed, so each further arrival exercises the
/// policy rather than the fast path.
fn full_ring(producers: usize, overflow: OverflowPolicy) -> Ring<u64> {
  let config = RingConfig::new(CAPACITY)
    .unwrap()
    .with_overflow(overflow)
    .with_producers(producers);
  let mut ring: Ring<u64> = Ring::new(&config).unwrap();
  let mut ends = ring.ends();
  let (mut producer, _consumer) = ends.split();
  for record in 0..CAPACITY as u64 {
    producer.try_push(record).unwrap();
  }
  ring
}

/// The refused push under `Fail`: the record comes back, the ring is
/// unchanged. What a retry loop pays per attempt.
fn fail_retry(c: &mut Criterion) {
  let mut group = c.benchmark_group("fail_retry");
  group.throughput(Throughput::Elements(1));
  for (name, producers) in [("spsc", 1), ("mpsc", 4)] {
    let mut ring = full_ring(producers, OverflowPolicy::Fail);
    let mut ends = ring.ends();
    let (mut producer, _consumer) = ends.split();
    group.bench_function(name, |b| {
      b.iter(|| assert!(producer.try_push(black_box(0)).is_err()));
    });
  }
  group.finish();
}

/// The discarded push under `DropNewest`: success reported, the incoming
/// record dropped, the ring unchanged.
fn drop_newest(c: &mut Criterion) {
  let mut group = c.benchmark_group("drop_newest");
  group.throughput(Throughput::Elements(1));
  for (name, producers) in [("spsc", 1), ("mpsc", 4)] {
    let mut ring = full_ring(producers, OverflowPolicy::DropNewest);
    let mut ends = ring.ends();
    let (mut producer, _consumer) = ends.split();
    group.bench_function(name, |b| {
      b.iter(|| assert!(producer.try_push(black_box(0)).is_ok()));
    });
  }
  group.finish();
}

criterion_group!(benches, fail_retry, drop_newest);
criterion_main!(benches);
