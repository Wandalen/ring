//! Latency, outside criterion: criterion reports a mean per iteration, this reports the tail per
//! record. Prints markdown tables and merges its rows into the results file `examples/report.rs`
//! reads. Arguments: `--test` runs every case briefly to validate it, a bare word filters case ids.

mod harness;

use std::time::Duration;

use harness::bench::cores;
use harness::candidates::{Candidate, CrossbeamQueue, Mpsc, MutexDeque, Rtrb, Spsc, SyncChannel};
use harness::latency::{self, Load, Stamped};
use harness::output;
use serde_json::Value;

/// Slots, for every case.
const CAPACITY: usize = 1024;

/// Records each producer sends per burst, and how often the bursts come.
const BURST: usize = 64;
const BURST_PERIOD: f64 = 200e-6;

struct Settings {
  quick: bool,
  filter: Option<String>,
}

impl Settings {
  fn from_args() -> Self {
    let mut settings = Self {
      quick: false,
      filter: None,
    };
    for arg in std::env::args().skip(1) {
      match arg.as_str() {
        "--test" => settings.quick = true,
        flag if flag.starts_with('-') => {}
        word => settings.filter = Some(word.to_string()),
      }
    }

    settings
  }

  fn wants(&self, id: &str) -> bool {
    self.filter.as_deref().is_none_or(|filter| id.contains(filter))
  }

  fn load(&self, producers: usize, rate: f64, burst: usize) -> Load {
    let (duration, warm_up) = if self.quick { (20, 0) } else { (1200, 200) };
    Load {
      capacity: CAPACITY,
      producers,
      rate,
      burst,
      duration: Duration::from_millis(duration),
      warm_up: Duration::from_millis(warm_up),
    }
  }

  /// Announce a finished case the way criterion does: `Testing <id>` under `--test`, the headline
  /// otherwise.
  fn done(&self, id: &str, headline: &str) {
    if self.quick {
      println!("Testing latency/{id}\nSuccess");
    } else {
      println!("latency/{id:<40} {headline}");
    }
  }
}

fn main() {
  let settings = Settings::from_args();
  let mut rows = vec![latency::clock_row()];

  let (runs, round_trips) = if settings.quick { (1, 1_000) } else { (5, 200_000) };
  ping::<Spsc>(&settings, &mut rows, runs, round_trips);
  ping::<Rtrb>(&settings, &mut rows, runs, round_trips);
  ping::<Mpsc>(&settings, &mut rows, runs, round_trips);
  ping::<SyncChannel>(&settings, &mut rows, runs, round_trips);
  ping::<CrossbeamQueue>(&settings, &mut rows, runs, round_trips);
  ping::<MutexDeque>(&settings, &mut rows, runs, round_trips);

  let spare = cores().saturating_sub(1).max(1);
  for producers in [1, 4, 8].into_iter().filter(|&p| p <= spare) {
    for rate in [1e6, 5e6, 10e6] {
      every_candidate(&settings, &mut rows, "steady", &settings.load(producers, rate, 1));
    }
  }
  for rate in [1e6, 5e6] {
    every_candidate(&settings, &mut rows, "oversubscribed", &settings.load(2 * cores(), rate, 1));
  }
  for producers in [1, 4, 8].into_iter().filter(|&p| p <= spare) {
    let rate = (producers * BURST) as f64 / BURST_PERIOD;
    every_candidate(&settings, &mut rows, "bursts", &settings.load(producers, rate, BURST));
  }

  if settings.quick || rows.len() == 1 {
    return;
  }
  println!("\n{}", latency::markdown(&rows));
  save(rows);
}

fn ping<C: Candidate<Record = u64>>(settings: &Settings, rows: &mut Vec<Value>, runs: usize, round_trips: u64) {
  let id = format!("pingpong/{}", C::NAME);
  if !settings.wants(&id) {
    return;
  }
  let one_way: Vec<f64> = (0..runs).map(|_| latency::ping_pong::<C>(CAPACITY, round_trips)).collect();
  let row = latency::ping_pong_row(C::NAME, &one_way);
  settings.done(
    &id,
    &format!("one-way {}", output::duration(row["one_way_ns"].as_f64().unwrap_or(f64::NAN))),
  );
  rows.push(with_id(row, &id));
}

fn every_candidate(settings: &Settings, rows: &mut Vec<Value>, test: &str, load: &Load) {
  offer::<Spsc<Stamped>>(settings, rows, test, load);
  offer::<Rtrb<Stamped>>(settings, rows, test, load);
  offer::<Mpsc<Stamped>>(settings, rows, test, load);
  offer::<SyncChannel<Stamped>>(settings, rows, test, load);
  offer::<CrossbeamQueue<Stamped>>(settings, rows, test, load);
  offer::<MutexDeque<Stamped>>(settings, rows, test, load);
}

fn offer<C: Candidate<Record = Stamped>>(settings: &Settings, rows: &mut Vec<Value>, test: &str, load: &Load) {
  if load.producers > C::MAX_PRODUCERS {
    return;
  }
  let id = format!("{test}/{}/{}/{:.2}M", C::NAME, load.producers, load.rate / 1e6);
  if !settings.wants(&id) {
    return;
  }
  let result = latency::open_loop::<C>(load);
  let h = &result.histogram;
  let headline = if result.saturated {
    format!("saturated at {:.2} M/s", result.achieved / 1e6)
  } else {
    let ns = |q: f64| output::duration(h.value_at_quantile(q) as f64);
    format!(
      "p50 {}  p99 {}  p99.99 {}  max {}",
      ns(0.5),
      ns(0.99),
      ns(0.9999),
      output::duration(h.max() as f64)
    )
  };
  settings.done(&id, &headline);
  rows.push(with_id(latency::load_row(test, C::NAME, load, &result), &id));
}

fn with_id(mut row: Value, id: &str) -> Value {
  row["id"] = Value::from(id);
  row
}

/// Merge `rows` into the results file by id, so a filtered run replaces only what it ran.
fn save(rows: Vec<Value>) {
  let path = output::latency_file(&output::perf_dir());
  let mut merged: Vec<Value> = std::fs::read_to_string(&path)
    .unwrap_or_default()
    .lines()
    .filter_map(|line| serde_json::from_str(line).ok())
    .collect();
  for row in rows {
    match merged.iter_mut().find(|old| old["id"] == row["id"]) {
      Some(old) => *old = row,
      None => merged.push(row),
    }
  }
  let lines: String = merged.iter().map(|row| format!("{row}\n")).collect();
  output::save(&path, &lines);
}
