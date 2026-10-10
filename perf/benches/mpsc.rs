//! Many producer threads, one consumer thread. The consumer takes everything available per pop
//! for every candidate; what differs is the producers' side. The `push32_popN` rows are producers
//! pushing 32 records per operation, where the crate has a batch push.

mod harness;

use criterion::{Criterion, criterion_group, criterion_main};
use harness::bench::{ThreadedGroup, cores, function, producer_sweep};
use harness::candidates::{Candidate, CrossbeamQueue, Mpsc, MpscPlain, MutexDeque, SyncChannel};
use harness::driver::{Mode, Shape};

/// Records per run, over all producers.
const RECORDS: u64 = 1 << 20;

/// Producer sweep at capacity 1024: 1, 2, 4, 8 and one per remaining core.
fn producers(c: &mut Criterion) {
  let mut group = ThreadedGroup::new(c, "mpsc_producers");
  for producers in producer_sweep() {
    let shape = Shape::new(1024, producers, RECORDS, Mode::PUSH1_POPN);
    contended::<Mpsc>(&mut group, &shape, producers);
    contended::<MpscPlain>(&mut group, &shape, producers);
    contended::<SyncChannel>(&mut group, &shape, producers);
    contended::<CrossbeamQueue>(&mut group, &shape, producers);
    contended::<MutexDeque>(&mut group, &shape, producers);
  }
  group.finish();
}

/// Four producers against 64 slots, always full, and against 16384, never full.
fn capacity(c: &mut Criterion) {
  let mut group = ThreadedGroup::new(c, "mpsc_capacity");
  let producers = 4.min(*producer_sweep().last().expect("at least one producer"));
  for capacity in [64, 16384] {
    let shape = Shape::new(capacity, producers, RECORDS, Mode::PUSH1_POPN);
    contended::<Mpsc>(&mut group, &shape, capacity);
    contended::<MpscPlain>(&mut group, &shape, capacity);
    contended::<SyncChannel>(&mut group, &shape, capacity);
    contended::<CrossbeamQueue>(&mut group, &shape, capacity);
    contended::<MutexDeque>(&mut group, &shape, capacity);
  }
  group.finish();
}

/// Twice as many producers as logical CPUs, so the scheduler preempts producers mid-operation. A
/// preempted lock holder stalls every other producer; a preempted claimer stalls the consumer at
/// the slot it has not published. The longest wait between two receives is reported beside the
/// time.
fn oversubscribed(c: &mut Criterion) {
  let mut group = ThreadedGroup::new(c, "mpsc_oversubscribed");
  let producers = 2 * cores();
  let shape = Shape {
    gaps: true,
    ..Shape::new(1024, producers, RECORDS / 4, Mode::PUSH1_POPN)
  };
  contended::<Mpsc>(&mut group, &shape, producers);
  contended::<SyncChannel>(&mut group, &shape, producers);
  contended::<CrossbeamQueue>(&mut group, &shape, producers);
  contended::<MutexDeque>(&mut group, &shape, producers);
  group.finish();
}

/// `shape` once per push mode `C` has, the consumer always taking everything available.
fn contended<C: Candidate>(group: &mut ThreadedGroup<'_>, shape: &Shape, parameter: usize) {
  for mode in Mode::of::<C>().into_iter().filter(|mode| mode.pop_all) {
    let shape = Shape { mode, ..shape.clone() };
    group.bench::<C>(&function::<C>(mode), &shape, parameter);
  }
}

criterion_group!(benches, producers, capacity, oversubscribed);
criterion_main!(benches);
