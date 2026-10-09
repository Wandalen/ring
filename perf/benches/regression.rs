//! The latency regression gate: the ring's p99 hand-off against the mutex
//! baseline, measured back-to-back on the same runner.
//!
//! Absolute nanosecond thresholds are meaningless on shared CI hardware —
//! yesterday's 90 ns is today's 140 ns with a noisy neighbour — so this gate
//! never names one. It measures both candidates in the same process, seconds
//! apart, and requires the ring's p99 to beat the mutex's p99 outright. The
//! headroom is the whole point: the comparison harness documents the ring
//! beating the mutex baseline several times over, so anything that closes
//! that gap to zero is a real regression, not noise.
//!
//! `Mpsc` runs the same one-producer hand-off through its shared claim path,
//! which is the configuration whose regression would hurt first.
//!
//! Fails by asserting inside the bench: a red `cargo bench` is the gate.

mod harness;

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use harness::candidates::{Candidate, Mpsc, MutexDeque, Spsc};
use harness::latency::ping_pong;
use hdrhistogram::Histogram;

/// Round trips per sample and samples per candidate: enough for a stable p99
/// in about a minute per candidate on hosted hardware.
const CAPACITY: usize = 1024;
const ROUND_TRIPS: u64 = 2_000;
const SAMPLES: usize = 300;

/// p99 of `SAMPLES` mean hand-off times for `C`, in nanoseconds.
fn p99<C: Candidate<Record = u64>>() -> f64 {
  let mut histogram = Histogram::<u64>::new(3).expect("histogram range");
  for _ in 0..SAMPLES {
    let mean_ns = black_box(ping_pong::<C>(CAPACITY, ROUND_TRIPS));
    histogram.record(mean_ns as u64).expect("sample within range");
  }
  histogram.value_at_quantile(0.99) as f64
}

/// The ring must hand off faster than the mutex at p99, same runner, same
/// minutes. `Spsc` first, then `Mpsc` through the shared path.
fn regression(c: &mut Criterion) {
  let mut group = c.benchmark_group("regression");
  group.sample_size(10);
  group.bench_function("spsc_vs_mutex_p99", |b| {
    b.iter(|| {
      let baseline = p99::<MutexDeque>();
      let ring = p99::<Spsc>();
      assert!(
        ring <= baseline,
        "spsc p99 {ring:.1} ns must beat mutex p99 {baseline:.1} ns on the same runner"
      );
    });
  });
  group.bench_function("mpsc_vs_mutex_p99", |b| {
    b.iter(|| {
      let baseline = p99::<MutexDeque>();
      let ring = p99::<Mpsc>();
      assert!(
        ring <= baseline,
        "mpsc p99 {ring:.1} ns must beat mutex p99 {baseline:.1} ns on the same runner"
      );
    });
  });
  group.finish();
}

criterion_group!(benches, regression);
criterion_main!(benches);
