//! Wait strategies: what each `WaitKind` costs between two looks.
//!
//! `micro::push_full` and `micro::pop_empty` measure what a spinning endpoint
//! pays per retry at the queue API. This measures the strategy layer
//! underneath: [`wait_until`] with an immediately-ready predicate (one look,
//! pure call overhead), with a never-ready one (the pause cost times the
//! budget), and the two production questions — [`for_space`] for a producer
//! and [`for_data`] for a consumer — over a real [`CursorPair`], ready and
//! exhausted. `Park` sleeps 50 µs per pause, so its exhausted budgets are
//! scaled down to keep an iteration under a millisecond.
//!
//! [`wait_until`]: ring_wait::wait_until
//! [`for_space`]: ring_wait::for_space
//! [`for_data`]: ring_wait::for_data
//! [`CursorPair`]: ring_cursor::CursorPair

use core::sync::atomic::Ordering;
use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use ring_cursor::{CursorPair, SeqCell};
use ring_types::{Capacity, Seq, WaitKind};
use ring_wait::{for_data, for_space, wait_until};

const KINDS: [(WaitKind, &str); 4] = [
  (WaitKind::Spin, "spin"),
  (WaitKind::Yield, "yield"),
  (WaitKind::Park, "park"),
  (WaitKind::None, "none"),
];

/// Budget for the never-ready case, per strategy. `None` evaluates once no
/// matter the budget; it is listed for the shape, not the count.
fn exhausted_spins(kind: WaitKind) -> usize {
  match kind {
    WaitKind::Spin => 256,
    WaitKind::Yield => 32,
    WaitKind::Park => 4,
    WaitKind::None => 64,
  }
}

/// Budget for the contended `for_space` / `for_data` case: shorter than the
/// never-ready one, because a real waiter would have been woken by now.
fn contended_spins(kind: WaitKind) -> usize {
  match kind {
    WaitKind::Spin => 64,
    WaitKind::Yield => 16,
    WaitKind::Park => 4,
    WaitKind::None => 64,
  }
}

/// One look: the predicate is ready immediately, so every strategy pays
/// exactly one evaluation plus its call overhead.
fn ready(c: &mut Criterion) {
  let mut group = c.benchmark_group("wait_ready");
  group.throughput(Throughput::Elements(1));
  for (kind, name) in KINDS {
    group.bench_with_input(BenchmarkId::from_parameter(name), &kind, |b, &kind| {
      b.iter(|| black_box(wait_until(kind, 64, || black_box(true))));
    });
  }
  group.finish();
}

/// The full budget against a never-ready predicate: pause cost times spins.
fn exhausted(c: &mut Criterion) {
  let mut group = c.benchmark_group("wait_exhausted");
  for (kind, name) in KINDS {
    let spins = exhausted_spins(kind);
    group.throughput(Throughput::Elements(spins as u64));
    group.bench_with_input(BenchmarkId::from_parameter(name), &kind, |b, &kind| {
      b.iter(|| black_box(wait_until(kind, spins, || black_box(false))));
    });
  }
  group.finish();
}

/// The producer's question against a real cursor pair: room on an empty ring,
/// then none on a full one.
fn space(c: &mut Criterion) {
  let mut group = c.benchmark_group("wait_space");
  group.throughput(Throughput::Elements(1));
  for (kind, name) in KINDS {
    let pair = CursorPair::new(Capacity::new(4).unwrap());
    group.bench_with_input(BenchmarkId::new("ready", name), &kind, |b, &kind| {
      b.iter(|| black_box(for_space(&pair, kind, 64)));
    });
    pair.producer().store(Seq(4), Ordering::Release);
    let spins = contended_spins(kind);
    group.bench_with_input(BenchmarkId::new("full", name), &kind, |b, &kind| {
      b.iter(|| black_box(for_space(&pair, kind, spins)));
    });
  }
  group.finish();
}

/// The consumer's question: three records pending, then none at all.
fn data(c: &mut Criterion) {
  let mut group = c.benchmark_group("wait_data");
  group.throughput(Throughput::Elements(1));
  for (kind, name) in KINDS {
    let pair = CursorPair::new(Capacity::new(8).unwrap());
    pair.producer().store(Seq(3), Ordering::Release);
    group.bench_with_input(BenchmarkId::new("ready", name), &kind, |b, &kind| {
      b.iter(|| black_box(for_data(&pair, 3, kind, 64)));
    });
    let empty = CursorPair::new(Capacity::new(8).unwrap());
    let spins = contended_spins(kind);
    group.bench_with_input(BenchmarkId::new("empty", name), &kind, |b, &kind| {
      b.iter(|| black_box(for_data(&empty, 1, kind, spins)));
    });
  }
  group.finish();
}

criterion_group!(benches, ready, exhausted, space, data);
criterion_main!(benches);
