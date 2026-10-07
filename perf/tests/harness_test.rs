//! `perf`'s drivers: every candidate passes its own validation in every shape the benchmarks use,
//! and a candidate that loses, reorders or tears records is rejected by name rather than timed.

// Ordinary tests, compiled out under `--cfg loom`. That cfg swaps
// `ring_atomic`'s atomics for loom's instrumented ones across the whole
// graph, and those panic the moment they are touched outside a
// `loom::model` closure. Without this gate a family-wide loom run dies
// here before it can run a single model.
#![cfg(not(loom))]

#[path = "../benches/harness/mod.rs"]
mod harness;

use std::hint::spin_loop;
use std::marker::PhantomData;
use std::time::Duration;

use harness::candidates::{Candidate, CrossbeamQueue, Mpsc, MutexDeque, Record, Rtrb, Run, Rx, Spsc, SyncChannel, Tx, Wide};
use harness::driver::{self, Mode, Shape};
use harness::latency::{self, Load, Stamped};
use harness::topology;

const RECORDS: u64 = 40_000;

fn every_mode<C: Candidate>(producers: usize) {
  for capacity in [4, 1024] {
    for mode in Mode::of::<C>().into_iter().chain([Mode::batch(8), Mode::batch(128)]) {
      if mode.push > 1 && !C::PUSH_BATCH {
        continue;
      }
      driver::run::<C>(&Shape::new(capacity, producers, RECORDS, mode));
    }
  }
}

#[test]
fn spsc_passes_validation_in_every_mode() {
  every_mode::<Spsc>(1);
}

#[test]
fn rtrb_passes_validation_in_every_mode() {
  every_mode::<Rtrb>(1);
}

#[test]
fn mpsc_passes_validation_in_every_mode_and_producer_count() {
  every_mode::<Mpsc>(1);
  every_mode::<Mpsc>(3);
}

#[test]
fn sync_channel_passes_validation_in_every_mode_and_producer_count() {
  every_mode::<SyncChannel>(1);
  every_mode::<SyncChannel>(3);
}

#[test]
fn arrayqueue_passes_validation_in_every_mode_and_producer_count() {
  every_mode::<CrossbeamQueue>(1);
  every_mode::<CrossbeamQueue>(3);
}

#[test]
fn mutex_passes_validation_in_every_mode_and_producer_count() {
  every_mode::<MutexDeque>(1);
  every_mode::<MutexDeque>(3);
}

#[test]
fn wide_records_arrive_whole_through_every_candidate() {
  let shape = Shape::new(64, 1, RECORDS, Mode::PUSH1_POPN);
  driver::run::<Spsc<Wide<8>>>(&shape);
  driver::run::<Rtrb<Wide<32>>>(&shape);
  driver::run::<Mpsc<Wide<8>>>(&Shape {
    producers: 2,
    ..shape.clone()
  });
  driver::run::<SyncChannel<Wide<8>>>(&Shape {
    producers: 2,
    ..shape.clone()
  });
  driver::run::<CrossbeamQueue<Wide<8>>>(&Shape {
    producers: 2,
    ..shape.clone()
  });
  driver::run::<MutexDeque<Wide<32>>>(&shape);
}

#[test]
fn gaps_are_measured_only_when_asked_for() {
  let shape = Shape::new(1024, 2, RECORDS, Mode::PUSH1_POPN);
  assert_eq!(driver::run::<Mpsc>(&shape).worst_gap, Duration::ZERO);
  assert!(driver::run::<Mpsc>(&Shape { gaps: true, ..shape }).worst_gap > Duration::ZERO);
}

/// Machine-dependent: runs only where sysfs describes the CPUs.
#[test]
fn a_pinned_run_passes_validation_where_the_topology_is_known() {
  for (consumer, producer) in [topology::smt_siblings(), topology::separate_cores()].into_iter().flatten() {
    assert_ne!(consumer, producer);
    let shape = Shape {
      cpus: vec![consumer, producer],
      ..Shape::new(1024, 1, RECORDS, Mode::PUSH1_POP1)
    };
    driver::run::<Spsc>(&shape);
  }
}

#[test]
fn ping_pong_echoes_through_every_candidate() {
  assert!(latency::ping_pong::<Spsc>(4, 1_000) > 0.0);
  assert!(latency::ping_pong::<Rtrb>(4, 1_000) > 0.0);
  assert!(latency::ping_pong::<Mpsc>(4, 1_000) > 0.0);
  assert!(latency::ping_pong::<SyncChannel>(4, 1_000) > 0.0);
  assert!(latency::ping_pong::<CrossbeamQueue>(4, 1_000) > 0.0);
  assert!(latency::ping_pong::<MutexDeque>(4, 1_000) > 0.0);
}

fn load(producers: usize, rate: f64, burst: usize) -> Load {
  Load {
    capacity: 1024,
    producers,
    rate,
    burst,
    duration: Duration::from_millis(20),
    warm_up: Duration::from_millis(5),
  }
}

#[test]
fn an_open_loop_records_every_record_after_the_warm_up() {
  for burst in [1, 64] {
    let steady = load(1, 200_000.0, burst);
    assert!(!latency::open_loop::<Spsc<Stamped>>(&steady).histogram.is_empty());
    assert!(!latency::open_loop::<Rtrb<Stamped>>(&steady).histogram.is_empty());
    let shared = load(3, 200_000.0, burst);
    assert!(!latency::open_loop::<Mpsc<Stamped>>(&shared).histogram.is_empty());
    assert!(!latency::open_loop::<SyncChannel<Stamped>>(&shared).histogram.is_empty());
    assert!(!latency::open_loop::<CrossbeamQueue<Stamped>>(&shared).histogram.is_empty());
    assert!(!latency::open_loop::<MutexDeque<Stamped>>(&shared).histogram.is_empty());
  }
}

#[test]
fn a_rate_no_queue_can_keep_is_reported_as_saturated() {
  let impossible = Load {
    duration: Duration::from_micros(100),
    warm_up: Duration::ZERO,
    ..load(1, 1e9, 1)
  };
  assert!(latency::open_loop::<MutexDeque<Stamped>>(&impossible).saturated);
}

#[test]
fn latency_rows_render_as_tables() {
  let spec = load(1, 200_000.0, 1);
  let rows = [
    latency::ping_pong_row("spsc", &[90.0, 80.0, 100.0]),
    latency::load_row("steady", "rtrb", &spec, &latency::open_loop::<Rtrb<Stamped>>(&spec)),
  ];
  let tables = latency::markdown(&rows);
  assert!(tables.contains("| spsc | 90.0 ns | 80.0 ns | 100.0 ns |"), "{tables}");
  assert!(tables.contains("| 1 | 0.20 M/s | rtrb |"), "{tables}");
}

#[test]
#[should_panic(expected = "faulty: records lost, duplicated, reordered or torn — received 39960 of 40000")]
fn a_candidate_that_drops_every_thousandth_record_is_rejected() {
  driver::run::<Faulty<DropsEveryThousandth, u64>>(&Shape::new(1024, 2, RECORDS, Mode::PUSH1_POPN));
}

/// Each producer's first pair arrives as 1, 0; its expected key then sticks at 1, so every later
/// one is out of order: 19 999 per producer.
#[test]
#[should_panic(expected = "received 40000 of 40000, 39998 out of order, 0 torn")]
fn a_candidate_that_swaps_records_within_a_producer_is_rejected() {
  driver::run::<Faulty<SwapsPairs, u64>>(&Shape::new(1024, 2, RECORDS, Mode::PUSH1_POP1));
}

#[test]
#[should_panic(expected = "received 40000 of 40000, 0 out of order, 40 torn")]
fn a_candidate_that_tears_a_record_is_rejected() {
  driver::run::<Faulty<TearsEveryThousandth, Wide<4>>>(&Shape::new(1024, 2, RECORDS, Mode::PUSH1_POPN));
}

/// A defect planted in a sending end.
trait Defect<R>: Default + Send {
  /// Push `record` through `inner`, defectively.
  fn push<T: Tx<R>>(&mut self, inner: &mut T, record: R) -> bool;
}

/// Every thousandth acknowledged push is reported as sent and never sent — a lossy overflow
/// policy answering `Ok`.
#[derive(Default)]
struct DropsEveryThousandth(u64);

impl<R: Record> Defect<R> for DropsEveryThousandth {
  fn push<T: Tx<R>>(&mut self, inner: &mut T, record: R) -> bool {
    if (self.0 + 1).is_multiple_of(1000) {
      self.0 += 1;
      return true;
    }
    let sent = inner.try_push(record);
    self.0 += u64::from(sent);

    sent
  }
}

/// Holds every other record back and sends it after the next one.
#[derive(Default)]
struct SwapsPairs(Option<u64>);

impl Defect<u64> for SwapsPairs {
  fn push<T: Tx<u64>>(&mut self, inner: &mut T, record: u64) -> bool {
    let Some(held) = self.0 else {
      self.0 = Some(record);
      return true;
    };
    if !inner.try_push(record) {
      return false;
    }
    while !inner.try_push(held) {
      spin_loop();
    }
    self.0 = None;

    true
  }
}

/// Every thousandth acknowledged record goes out with its last word changed — a torn copy.
#[derive(Default)]
struct TearsEveryThousandth(u64);

impl Defect<Wide<4>> for TearsEveryThousandth {
  fn push<T: Tx<Wide<4>>>(&mut self, inner: &mut T, mut record: Wide<4>) -> bool {
    if (self.0 + 1).is_multiple_of(1000) {
      record.0[3] = !record.0[3];
    }
    let sent = inner.try_push(record);
    self.0 += u64::from(sent);

    sent
  }
}

/// `MutexDeque` with `D` planted in every sending end.
struct Faulty<D, R>(MutexDeque<R>, PhantomData<D>);

impl<D: Defect<R>, R: Record> Candidate for Faulty<D, R> {
  type Record = R;

  const NAME: &'static str = "faulty";
  const PUSH_BATCH: bool = false;
  const MAX_PRODUCERS: usize = usize::MAX;

  fn new(capacity: usize) -> Self {
    Self(MutexDeque::new(capacity), PhantomData)
  }

  fn split<V: Run<R>>(&mut self, producers: usize, run: V) -> V::Output {
    self.0.split(producers, Plant::<V, D>(run, PhantomData))
  }
}

struct Plant<V, D>(V, PhantomData<D>);

impl<R: Record, V: Run<R>, D: Defect<R>> Run<R> for Plant<V, D> {
  type Output = V::Output;

  fn run<T: Tx<R> + Send, X: Rx<R> + Send>(self, producers: Vec<T>, consumer: X) -> V::Output {
    let planted = producers
      .into_iter()
      .map(|inner| FaultyTx {
        inner,
        defect: D::default(),
      })
      .collect();

    self.0.run(planted, consumer)
  }
}

struct FaultyTx<T, D> {
  inner: T,
  defect: D,
}

impl<R: Record, T: Tx<R>, D: Defect<R>> Tx<R> for FaultyTx<T, D> {
  fn try_push(&mut self, record: R) -> bool {
    self.defect.push(&mut self.inner, record)
  }
}
