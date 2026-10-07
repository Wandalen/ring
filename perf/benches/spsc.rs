//! One producer thread, one consumer thread.

mod harness;

use criterion::{Criterion, criterion_group, criterion_main};
use harness::bench::{ThreadedGroup, function};
use harness::candidates::{Candidate, CrossbeamQueue, MpscPrimary, MutexDeque, Rtrb, Spsc, SyncChannel, Wide};
use harness::driver::{Mode, Shape};
use harness::topology;

/// Records per run.
const RECORDS: u64 = 1 << 20;

/// Capacity 64 keeps the two ends trading cursors every few records; 16384 lets them run apart.
/// Every mode each candidate has an API for.
fn capacity(c: &mut Criterion) {
  let mut group = ThreadedGroup::new(c, "spsc");
  for capacity in [64, 1024, 16384] {
    every_mode::<Spsc>(&mut group, capacity);
    every_mode::<Rtrb>(&mut group, capacity);
    every_mode::<MutexDeque>(&mut group, capacity);
    every_mode::<SyncChannel>(&mut group, capacity);
    every_mode::<CrossbeamQueue>(&mut group, capacity);
    every_mode::<MpscPrimary>(&mut group, capacity);
  }
  group.finish();
}

fn every_mode<C: Candidate>(group: &mut ThreadedGroup<'_>, capacity: usize) {
  for mode in Mode::of::<C>() {
    group.bench::<C>(&function::<C>(mode), &Shape::new(capacity, 1, RECORDS, mode), capacity);
  }
}

/// Records of 8, 64 and 256 bytes through 1024 slots: the protocol's cost against the copy's.
fn payload(c: &mut Criterion) {
  let mut group = ThreadedGroup::new(c, "spsc_payload");
  sized::<Spsc<u64>>(&mut group);
  sized::<Spsc<Wide<8>>>(&mut group);
  sized::<Spsc<Wide<32>>>(&mut group);
  sized::<Rtrb<u64>>(&mut group);
  sized::<Rtrb<Wide<8>>>(&mut group);
  sized::<Rtrb<Wide<32>>>(&mut group);
  sized::<SyncChannel<u64>>(&mut group);
  sized::<SyncChannel<Wide<8>>>(&mut group);
  sized::<SyncChannel<Wide<32>>>(&mut group);
  sized::<CrossbeamQueue<u64>>(&mut group);
  sized::<CrossbeamQueue<Wide<8>>>(&mut group);
  sized::<CrossbeamQueue<Wide<32>>>(&mut group);
  sized::<MutexDeque<u64>>(&mut group);
  sized::<MutexDeque<Wide<8>>>(&mut group);
  sized::<MutexDeque<Wide<32>>>(&mut group);
  group.finish();
}

fn sized<C: Candidate>(group: &mut ThreadedGroup<'_>) {
  let mode = Mode::PUSH1_POPN;
  group.bench::<C>(
    &function::<C>(mode),
    &Shape::new(1024, 1, RECORDS, mode),
    size_of::<C::Record>(),
  );
}

/// The two threads on SMT siblings of one physical core, then on two physical cores: what the
/// cache-line hand-off costs when it leaves the core.
fn pinned(c: &mut Criterion) {
  let pairs = [("siblings", topology::smt_siblings()), ("cores", topology::separate_cores())];
  if pairs.iter().all(|(_, pair)| pair.is_none()) {
    eprintln!("spsc_pinned: no CPU topology to read, skipped");
    return;
  }
  let mut group = ThreadedGroup::new(c, "spsc_pinned");
  for (placement, pair) in pairs {
    let Some((consumer, producer)) = pair else {
      eprintln!("spsc_pinned/{placement}: no such pair of CPUs, skipped");
      continue;
    };
    let shape = Shape {
      cpus: vec![consumer, producer],
      ..Shape::new(1024, 1, RECORDS, Mode::PUSH1_POP1)
    };
    group.bench::<Spsc>(&function::<Spsc>(shape.mode), &shape, placement);
    group.bench::<Rtrb>(&function::<Rtrb>(shape.mode), &shape, placement);
    group.bench::<SyncChannel>(&function::<SyncChannel>(shape.mode), &shape, placement);
    group.bench::<CrossbeamQueue>(&function::<CrossbeamQueue>(shape.mode), &shape, placement);
    group.bench::<MutexDeque>(&function::<MutexDeque>(shape.mode), &shape, placement);
  }
  group.finish();
}

criterion_group!(benches, capacity, payload, pinned);
criterion_main!(benches);
