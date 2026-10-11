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
//! Fails fast, before timing: one back-to-back comparison per candidate panics
//! on a closed gap, so a red `cargo bench` is the gate. The benches below only
//! report the numbers.

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

/// One back-to-back comparison: panic here, with both numbers in the message.
/// Exactly one comparison per candidate per invocation, deliberately — an
/// `assert!` inside `b.iter` would re-run once per criterion sample, a
/// repetition count set by criterion's calibration rather than by us.
fn gate(name: &str, baseline: f64, ring: f64) {
  assert!(
    ring <= baseline,
    "{name} p99 {ring:.1} ns must beat mutex p99 {baseline:.1} ns on the same runner"
  );
}

/// The ring must hand off faster than the mutex at p99, same runner, same
/// minutes. `Spsc` first, then `Mpsc` through the shared path.
///
/// The pass/fail decision is made above, before criterion spends minutes
/// timing; the benches below only report the numbers. `quick::1` (`--test`)
/// validates the wiring without paying for 300-sample histograms.
fn regression(c: &mut Criterion) {
  if !harness::output::quick() {
    gate("spsc", p99::<MutexDeque>(), p99::<Spsc>());
    gate("mpsc", p99::<MutexDeque>(), p99::<Mpsc>());
  }
  let mut group = c.benchmark_group("regression");
  group.sample_size(10);
  group.bench_function("spsc_vs_mutex_p99", |b| {
    b.iter(|| black_box((p99::<MutexDeque>(), p99::<Spsc>())));
  });
  group.bench_function("mpsc_vs_mutex_p99", |b| {
    b.iter(|| black_box((p99::<MutexDeque>(), p99::<Mpsc>())));
  });
  group.finish();
}

criterion_group!(benches, regression);
criterion_main!(benches);
