//! One producer thread, one consumer thread.

mod harness;

use criterion::{Criterion, criterion_group, criterion_main};
use harness::bench::{ThreadedGroup, function};
use harness::candidates::{
  Candidate,
  ConcurrentQueue,
  CrossbeamQueue,
  Heapless,
  Heapless64,
  Heapless1024,
  MpscPrimary,
  MutexDeque,
  QuetzalSpsc,
  RingBuf,
  Rtrb,
  Spsc,
  SpscPlain,
  SyncChannel,
  ThingBuf,
  Wide,
};
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
    every_mode::<SpscPlain>(&mut group, capacity);
    every_mode::<Rtrb>(&mut group, capacity);
    every_mode::<RingBuf>(&mut group, capacity);
    every_mode::<QuetzalSpsc>(&mut group, capacity);
    every_mode::<ConcurrentQueue>(&mut group, capacity);
    every_mode::<ThingBuf>(&mut group, capacity);
    if capacity == 64 {
      // Heapless capacity is fixed at compile time: one candidate per size,
      // measured where the size matches.
      every_mode::<Heapless64>(&mut group, capacity);
    }
    if capacity == 1024 {
      every_mode::<Heapless1024>(&mut group, capacity);
    }
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
  sized::<SpscPlain<u64>>(&mut group);
  sized::<SpscPlain<Wide<8>>>(&mut group);
  sized::<SpscPlain<Wide<32>>>(&mut group);
  sized::<Rtrb<u64>>(&mut group);
  sized::<Rtrb<Wide<8>>>(&mut group);
  sized::<Rtrb<Wide<32>>>(&mut group);
  sized::<RingBuf<u64>>(&mut group);
  sized::<RingBuf<Wide<8>>>(&mut group);
  sized::<RingBuf<Wide<32>>>(&mut group);
  sized::<QuetzalSpsc<u64>>(&mut group);
  sized::<QuetzalSpsc<Wide<8>>>(&mut group);
  sized::<QuetzalSpsc<Wide<32>>>(&mut group);
  sized::<ConcurrentQueue<u64>>(&mut group);
  sized::<ConcurrentQueue<Wide<8>>>(&mut group);
  sized::<ConcurrentQueue<Wide<32>>>(&mut group);
  sized::<ThingBuf<u64>>(&mut group);
  sized::<ThingBuf<Wide<8>>>(&mut group);
  sized::<ThingBuf<Wide<32>>>(&mut group);
  sized::<Heapless1024>(&mut group);
  sized::<Heapless<Wide<8>, 1025>>(&mut group);
  sized::<Heapless<Wide<32>, 1025>>(&mut group);
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
    group.bench::<RingBuf>(&function::<RingBuf>(shape.mode), &shape, placement);
    group.bench::<QuetzalSpsc>(&function::<QuetzalSpsc>(shape.mode), &shape, placement);
    group.bench::<ConcurrentQueue>(&function::<ConcurrentQueue>(shape.mode), &shape, placement);
    group.bench::<ThingBuf>(&function::<ThingBuf>(shape.mode), &shape, placement);
    group.bench::<Heapless1024>(&function::<Heapless1024>(shape.mode), &shape, placement);
    group.bench::<SyncChannel>(&function::<SyncChannel>(shape.mode), &shape, placement);
    group.bench::<CrossbeamQueue>(&function::<CrossbeamQueue>(shape.mode), &shape, placement);
    group.bench::<MutexDeque>(&function::<MutexDeque>(shape.mode), &shape, placement);
  }
  group.finish();
}

/// `push1_pop1` at 64 and 1024 slots with 0, 2 and 8 dependent multiply-adds of consumer work
/// per record (issue #56). The no-work cell is a regime of its own — a consumer that does nothing
/// but update the checker — and two-thread verdicts flip within a few steps of real per-record
/// work, so the suite measures the work dimension instead of the no-work point only. Rows carry
/// the work level, columns the capacity; `work0` keeps the worst case.
fn work(c: &mut Criterion) {
  let mut group = ThreadedGroup::new(c, "spsc_work");
  for capacity in [64, 1024] {
    for work in [0u64, 2, 8] {
      work_on::<Spsc>(&mut group, capacity, work);
      work_on::<SpscPlain>(&mut group, capacity, work);
      work_on::<Rtrb>(&mut group, capacity, work);
    }
  }
  group.finish();
}

fn work_on<C: Candidate>(group: &mut ThreadedGroup<'_>, capacity: usize, work: u64) {
  let shape = Shape {
    work,
    ..Shape::new(capacity, 1, RECORDS, Mode::PUSH1_POP1)
  };
  group.bench::<C>(&format!("{}/work{work}", function::<C>(Mode::PUSH1_POP1)), &shape, capacity);
}

criterion_group!(benches, capacity, payload, pinned, work);
criterion_main!(benches);
