//! One thread, no coherence traffic: what each protocol costs per operation.

mod harness;

use std::hint::black_box;
use std::time::Duration;

use criterion::measurement::WallTime;
use criterion::{BenchmarkGroup, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use harness::candidates::{Candidate, CrossbeamQueue, Mpsc, MutexDeque, Rtrb, Run, Rx, Spsc, SyncChannel, Tx};
use harness::driver::{Mode, fill, pop};

const CAPACITIES: [usize; 3] = [64, 1024, 16384];

/// Capacity of the queues whose fill level does not matter to the benchmark.
const CAPACITY: usize = 1024;

/// Run `$visitor` on a fresh single-producer queue of every candidate.
macro_rules! every_candidate {
  ($group:expr, $visitor:ident) => {
    every_candidate!(@ $group, $visitor, Spsc, Rtrb, Mpsc, SyncChannel, CrossbeamQueue, MutexDeque)
  };
  (@ $group:expr, $visitor:ident, $($candidate:ident),*) => {
    $(
      <$candidate>::new(CAPACITY).split(1, $visitor { group: &mut $group, name: <$candidate>::NAME });
    )*
  };
}

/// Push one record, pop it back: the queue never holds more than one.
fn push_pop(c: &mut Criterion) {
  let mut group = group(c, "push_pop");
  group.throughput(Throughput::Elements(1));
  every_candidate!(group, PushPop);
  group.finish();
}

struct PushPop<'g, 'c> {
  group: &'g mut BenchmarkGroup<'c, WallTime>,
  name: &'static str,
}

impl Run<u64> for PushPop<'_, '_> {
  type Output = ();

  fn run<T: Tx<u64> + Send, X: Rx<u64> + Send>(self, mut producers: Vec<T>, mut consumer: X) {
    let Self { group, name } = self;
    let tx = &mut producers[0];
    let mut next = 0;
    group.bench_function(name, |b| {
      b.iter(|| {
        assert!(tx.try_push(black_box(next)), "{name}: push refused on an empty queue");
        let mut popped = None;
        consumer.try_pop(&mut |record| popped = Some(record));
        assert_eq!(popped, Some(next), "{name}: the pushed record did not come back");
        next += 1;
      });
    });
  }
}

/// Push into a full queue: the price of a refusal, paid on every retry of a spinning producer.
fn push_full(c: &mut Criterion) {
  let mut group = group(c, "push_full");
  group.throughput(Throughput::Elements(1));
  every_candidate!(group, PushFull);
  group.finish();
}

struct PushFull<'g, 'c> {
  group: &'g mut BenchmarkGroup<'c, WallTime>,
  name: &'static str,
}

impl Run<u64> for PushFull<'_, '_> {
  type Output = ();

  // The consumer is held, not used: some crates treat a dropped end as an abandoned queue.
  fn run<T: Tx<u64> + Send, X: Rx<u64> + Send>(self, mut producers: Vec<T>, _consumer: X) {
    let Self { group, name } = self;
    let tx = &mut producers[0];
    let mut held = 0;
    while tx.try_push(held) {
      held += 1;
    }
    assert_eq!(held, CAPACITY as u64, "{name}: a queue of {CAPACITY} took {held} records");
    group.bench_function(name, |b| {
      b.iter(|| assert!(!tx.try_push(black_box(held)), "{name}: a full queue took a record"));
    });
  }
}

/// Pop from an empty queue, one record or everything available: what a spinning consumer pays
/// per look.
fn pop_empty(c: &mut Criterion) {
  let mut group = group(c, "pop_empty");
  group.throughput(Throughput::Elements(1));
  every_candidate!(group, PopEmpty);
  group.finish();
}

struct PopEmpty<'g, 'c> {
  group: &'g mut BenchmarkGroup<'c, WallTime>,
  name: &'static str,
}

impl Run<u64> for PopEmpty<'_, '_> {
  type Output = ();

  fn run<T: Tx<u64> + Send, X: Rx<u64> + Send>(self, _producers: Vec<T>, mut consumer: X) {
    let Self { group, name } = self;
    for (mode, label) in [(Mode::PUSH1_POP1, "pop1"), (Mode::PUSH1_POPN, "popN")] {
      group.bench_function(format!("{name}/{label}"), |b| {
        b.iter(|| assert_eq!(pop(&mut consumer, mode, &mut |_| ()), 0, "{name}: popped from an empty queue"));
      });
    }
  }
}

/// Fill the queue to capacity, then drain it; per record, in every mode the candidate has.
fn fill_drain(c: &mut Criterion) {
  let mut group = group(c, "fill_drain");
  for capacity in CAPACITIES {
    group.throughput(Throughput::Elements(capacity as u64));
    fill_drain_on::<Spsc>(&mut group, capacity);
    fill_drain_on::<Rtrb>(&mut group, capacity);
    fill_drain_on::<Mpsc>(&mut group, capacity);
    fill_drain_on::<SyncChannel>(&mut group, capacity);
    fill_drain_on::<CrossbeamQueue>(&mut group, capacity);
    fill_drain_on::<MutexDeque>(&mut group, capacity);
  }
  group.finish();
}

fn fill_drain_on<C: Candidate<Record = u64>>(group: &mut BenchmarkGroup<'_, WallTime>, capacity: usize) {
  for mode in Mode::of::<C>() {
    C::new(capacity).split(
      1,
      FillDrain {
        group: &mut *group,
        name: C::NAME,
        mode,
        capacity,
      },
    );
  }
}

struct FillDrain<'g, 'c> {
  group: &'g mut BenchmarkGroup<'c, WallTime>,
  name: &'static str,
  mode: Mode,
  capacity: usize,
}

impl Run<u64> for FillDrain<'_, '_> {
  type Output = ();

  fn run<T: Tx<u64> + Send, X: Rx<u64> + Send>(self, mut producers: Vec<T>, mut consumer: X) {
    let Self {
      group,
      name,
      mode,
      capacity,
    } = self;
    let tx = &mut producers[0];
    let records: Vec<u64> = (0..capacity as u64).collect();
    let expected: u64 = records.iter().sum();
    group.bench_function(BenchmarkId::new(format!("{name}/{}", mode.label()), capacity), |b| {
      b.iter(|| {
        fill(tx, black_box(&records), mode);
        let (mut taken, mut sum) = (0, 0u64);
        while taken < capacity as u64 {
          let n = pop(&mut consumer, mode, &mut |record| sum = sum.wrapping_add(record));
          assert!(n > 0, "{name}: empty with records outstanding");
          taken += n;
        }
        assert_eq!(sum, expected, "{name}: drained records differ from the pushed ones");
      });
    });
  }
}

// Single-thread operations take nanoseconds; criterion's defaults would spend most of their time
// re-confirming a stable number.
fn group<'c>(c: &'c mut Criterion, name: &str) -> BenchmarkGroup<'c, WallTime> {
  let mut group = c.benchmark_group(name);
  group.warm_up_time(Duration::from_secs(1));
  group.measurement_time(Duration::from_secs(2));

  group
}

criterion_group!(benches, push_pop, push_full, pop_empty, fill_drain);
criterion_main!(benches);
