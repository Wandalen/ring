//! Criterion glue for the threaded benchmarks, and the extras criterion has no column for: spins
//! on a full and on an empty queue, and the longest wait between two receives.

use std::fmt::Display;
use std::thread;
use std::time::Duration;

use criterion::measurement::WallTime;
use criterion::{BenchmarkGroup, BenchmarkId, Criterion, SamplingMode, Throughput};

use super::candidates::Candidate;
use super::driver::{self, Mode, Shape};
use super::output;

/// A group of threaded benchmarks: ten flat samples, one fresh run per iteration.
pub struct ThreadedGroup<'c> {
  group: BenchmarkGroup<'c, WallTime>,
  name: &'static str,
}

impl<'c> ThreadedGroup<'c> {
  /// The group `name`.
  pub fn new(c: &'c mut Criterion, name: &'static str) -> Self {
    let mut group = c.benchmark_group(name);
    group.sample_size(10);
    group.sampling_mode(SamplingMode::Flat);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(3));

    Self { group, name }
  }

  /// Bench `shape` on `C` as `<function>/<parameter>`, the clock summed over runs. The extras are
  /// printed after criterion's lines and saved to [`output::extras_file`].
  pub fn bench<C: Candidate>(&mut self, function: &str, shape: &Shape, parameter: impl Display) {
    let id = format!("{}/{function}/{parameter}", self.name);
    let (mut records, mut full, mut empty, mut worst) = (0, 0, 0, Duration::ZERO);
    self.group.throughput(Throughput::Elements(shape.records()));
    self.group.bench_function(BenchmarkId::new(function, &parameter), |b| {
      b.iter_custom(|iters| {
        let mut elapsed = Duration::ZERO;
        for _ in 0..iters {
          let outcome = driver::run::<C>(shape);
          elapsed += outcome.elapsed;
          full += outcome.full_spins;
          empty += outcome.empty_spins;
          worst = worst.max(outcome.worst_gap);
          records += shape.records();
        }
        elapsed
      });
    });
    // Zero when a filter skipped the benchmark.
    if records == 0 {
      return;
    }

    let (full, empty) = (full as f64 / records as f64, empty as f64 / records as f64);
    let worst_ns = shape.gaps.then_some(worst.as_nanos() as f64);
    let gap = worst_ns
      .map(|ns| format!(", worst gap {}", output::duration(ns)))
      .unwrap_or_default();
    println!("{:24}spins:  full {full:.3}/record, empty {empty:.3}/record{gap}", "");
    if !output::quick() {
      let extras = serde_json::json!({
        "id": id,
        "records": records,
        "full_per_record": full,
        "empty_per_record": empty,
        "worst_gap_ns": worst_ns,
      });
      output::save(&output::extras_file(&output::perf_dir(), &id), &extras.to_string());
    }
  }

  /// Close the group.
  pub fn finish(self) {
    self.group.finish();
  }
}

/// The function id of `C` under `mode`: `spsc/push1_pop1`.
pub fn function<C: Candidate>(mode: Mode) -> String {
  format!("{}/{}", C::NAME, mode.label())
}

/// Logical CPUs.
pub fn cores() -> usize {
  thread::available_parallelism().map_or(2, usize::from)
}

/// Producer counts to sweep: 1, 2, 4, 8 and one per remaining core, never more producers than
/// cores beside the consumer's.
pub fn producer_sweep() -> Vec<usize> {
  let spare = cores().saturating_sub(1).max(1);
  let mut sweep: Vec<usize> = [1, 2, 4, 8, spare].into_iter().filter(|&p| p <= spare).collect();
  sweep.dedup();

  sweep
}
