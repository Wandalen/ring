//! Latency drivers. A ping-pong times the bare hand-off. An open loop sends each record at its
//! scheduled moment and stamps that moment, so a push held up by a full queue is charged to the
//! records it delayed instead of disappearing from the numbers.

use std::fmt::Write as _;
use std::hint::spin_loop;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use hdrhistogram::Histogram;
use serde_json::{Value, json};

use super::candidates::{Candidate, Record, Run, Rx, SEQ_BITS, Tx};
use super::driver::{self, Check, Mode};
use super::output::duration;

/// A record stamped with the moment it was meant to be sent, in nanoseconds after the run's epoch.
#[derive(Clone, Copy, Debug)]
pub struct Stamped {
  /// Producer and sequence, as in every other record.
  pub key: u64,
  /// When the producer's schedule said to send it.
  pub intended_ns: u64,
}

impl Record for Stamped {
  fn new(key: u64) -> Self {
    Self { key, intended_ns: 0 }
  }

  fn key(&self) -> u64 {
    self.key
  }
}

/// Mean one-way hand-off time, in nanoseconds, over `round_trips` echoes through two `C` queues of
/// `capacity`, timed in bulk: one clock read per run, not per record.
///
/// # Panics
///
/// Naming `C`, when an echo comes back as a different record.
pub fn ping_pong<C: Candidate<Record = u64>>(capacity: usize, round_trips: u64) -> f64 {
  let mut back = C::new(capacity);
  C::new(capacity).split(
    1,
    Forth {
      back: &mut back,
      round_trips,
      name: C::NAME,
    },
  )
}

struct Forth<'b, C> {
  back: &'b mut C,
  round_trips: u64,
  name: &'static str,
}

impl<C: Candidate<Record = u64>> Run<u64> for Forth<'_, C> {
  type Output = f64;

  fn run<T: Tx<u64> + Send, X: Rx<u64> + Send>(self, mut producers: Vec<T>, consumer: X) -> f64 {
    let tx = producers.pop().expect("one producer");
    self.back.split(
      1,
      Back {
        forth: (tx, consumer),
        round_trips: self.round_trips,
        name: self.name,
      },
    )
  }
}

struct Back<T, X> {
  forth: (T, X),
  round_trips: u64,
  name: &'static str,
}

impl<T: Tx<u64> + Send, X: Rx<u64> + Send> Run<u64> for Back<T, X> {
  type Output = f64;

  fn run<U: Tx<u64> + Send, Y: Rx<u64> + Send>(self, mut producers: Vec<U>, mut back_rx: Y) -> f64 {
    let Self {
      forth: (mut forth_tx, mut forth_rx),
      round_trips,
      name,
    } = self;
    let mut back_tx = producers.pop().expect("one producer");
    let ready = AtomicUsize::new(0);
    let ready = &ready;

    let (one_way, wrong) = thread::scope(|scope| {
      scope.spawn(move || {
        driver::start_line(ready, 2);
        for _ in 0..round_trips {
          let echo = receive(&mut forth_rx);
          send(&mut back_tx, echo);
        }
      });
      driver::start_line(ready, 2);
      let start = Instant::now();
      let mut wrong = 0;
      for i in 0..round_trips {
        send(&mut forth_tx, i);
        wrong += u64::from(receive(&mut back_rx) != i);
      }

      (start.elapsed().as_nanos() as f64 / (2 * round_trips) as f64, wrong)
    });
    assert_eq!(
      wrong, 0,
      "{name}: {wrong} of {round_trips} echoes came back as another record"
    );

    one_way
  }
}

fn send<R: Copy, T: Tx<R>>(tx: &mut T, record: R) {
  while !tx.try_push(record) {
    spin_loop();
  }
}

fn receive<R, X: Rx<R>>(rx: &mut X) -> R {
  loop {
    let mut got = None;
    rx.try_pop(&mut |record| got = Some(record));
    if let Some(record) = got {
      return record;
    }
    spin_loop();
  }
}

/// An open-loop run: `producers` threads offering `rate` records per second between them.
#[derive(Clone, Copy, Debug)]
pub struct Load {
  /// Slots, the same for every candidate.
  pub capacity: usize,
  /// Producer threads; the consumer is one more.
  pub producers: usize,
  /// Records per second, over all producers.
  pub rate: f64,
  /// Records per tick: 1 is a steady stream, the producers' ticks spread evenly; more is a burst,
  /// every producer's at the same moment.
  pub burst: usize,
  /// How long the schedule runs, the warm-up included.
  pub duration: Duration,
  /// Records scheduled in this first stretch are delivered and checked, not recorded.
  pub warm_up: Duration,
}

/// What an open-loop run measured.
#[derive(Debug)]
pub struct Latency {
  /// Receive time minus scheduled send time, in nanoseconds, after the warm-up.
  pub histogram: Histogram<u64>,
  /// Records per second actually delivered.
  pub achieved: f64,
  /// Whether delivery fell under 95 % of the offered rate: the percentiles then describe a
  /// growing backlog, not the queue.
  pub saturated: bool,
}

/// Run `load` through a fresh `C` and validate it as the throughput driver does.
///
/// # Panics
///
/// Naming `C`, when a record is lost, duplicated or reordered within its producer.
pub fn open_loop<C: Candidate<Record = Stamped>>(load: &Load) -> Latency {
  assert!(
    load.producers <= C::MAX_PRODUCERS,
    "{} takes at most {} producer(s)",
    C::NAME,
    C::MAX_PRODUCERS
  );

  C::new(load.capacity).split(load.producers, Open { load, name: C::NAME })
}

struct Open<'l> {
  load: &'l Load,
  name: &'static str,
}

/// How far ahead of the last arrival at the start line the schedule begins.
const LEAD_NS: u64 = 100_000;

impl Run<Stamped> for Open<'_> {
  type Output = Latency;

  fn run<T: Tx<Stamped> + Send, X: Rx<Stamped> + Send>(self, producers: Vec<T>, consumer: X) -> Latency {
    let Self { load, name } = self;
    let count = producers.len();
    let interval = load.burst as f64 * count as f64 * 1e9 / load.rate;
    let ticks = (load.duration.as_nanos() as f64 / interval) as u64;
    let mut check = Check::new(count, ticks * load.burst as u64);
    let mut histogram = Histogram::<u64>::new_with_bounds(1, 60_000_000_000, 3).expect("valid bounds");
    let mut batch = Vec::with_capacity(load.capacity);
    let epoch = Instant::now();
    let (arrived, base, finished) = (AtomicUsize::new(0), AtomicU64::new(0), AtomicUsize::new(0));

    let (mut consumer, check, histogram, span) = thread::scope(|scope| {
      let (arrived, base, finished) = (&arrived, &base, &finished);
      let receiving = scope.spawn(move || {
        let mut consumer = consumer;
        let start = start_at(arrived, base, count + 1, epoch);
        let warm = start + load.warm_up.as_nanos() as u64;
        let (mut taken, mut last) = (0, start);
        while taken < check.expected {
          let mut n = driver::pop(&mut consumer, Mode::PUSH1_POPN, &mut |record| batch.push(record));
          // Acquire: pairs with each producer's Release increment, as in the throughput driver.
          if n == 0 && finished.load(Ordering::Acquire) == count {
            n = driver::pop(&mut consumer, Mode::PUSH1_POPN, &mut |record| batch.push(record));
            if n == 0 {
              break;
            }
          }
          if n == 0 {
            spin_loop();
            continue;
          }
          // One clock read per pop: every record in it was received by now.
          let now = elapsed_ns(epoch);
          for record in batch.drain(..) {
            check.record(record.key(), record.whole());
            if record.intended_ns >= warm {
              histogram.saturating_record(now.saturating_sub(record.intended_ns));
            }
          }
          taken += n;
          last = now;
        }
        (consumer, check, histogram, last - start)
      });
      for (mut tx, id) in producers.into_iter().zip(0..) {
        scope.spawn(move || {
          let start = start_at(arrived, base, count + 1, epoch);
          let offset = if load.burst > 1 {
            0.0
          } else {
            interval * id as f64 / count as f64
          };
          let mut key = id << SEQ_BITS;
          // Read only while the last reading is short of the schedule: where a clock read is
          // slow (an HPET clocksource takes over a microsecond), a read per record caps the rate.
          let mut now = 0;
          for tick in 0..ticks {
            let intended = start + (offset + tick as f64 * interval) as u64;
            while now < intended {
              now = elapsed_ns(epoch);
              if now < intended {
                spin_loop();
              }
            }
            for _ in 0..load.burst {
              send(
                &mut tx,
                Stamped {
                  key,
                  intended_ns: intended,
                },
              );
              key += 1;
            }
          }
          // Release: pairs with the consumer's Acquire load.
          finished.fetch_add(1, Ordering::Release);
        });
      }
      receiving.join().expect("the consumer panicked")
    });

    let mut left = 0;
    while consumer.pop_batch(usize::MAX, &mut |_| left += 1) > 0 {}
    check.verify(name, left);
    let achieved = check.expected as f64 * 1e9 / span.max(1) as f64;

    Latency {
      histogram,
      achieved,
      saturated: achieved < 0.95 * load.rate,
    }
  }
}

/// The start line for an open loop: the last thread to arrive fixes when the schedule begins, a
/// little ahead, and every thread leaves with that moment.
fn start_at(arrived: &AtomicUsize, base: &AtomicU64, parties: usize, epoch: Instant) -> u64 {
  if arrived.fetch_add(1, Ordering::AcqRel) + 1 == parties {
    // Release: pairs with the Acquire load below. A non-zero base is the signal to go.
    base.store(elapsed_ns(epoch) + LEAD_NS, Ordering::Release);
  }
  loop {
    match base.load(Ordering::Acquire) {
      0 => spin_loop(),
      start => return start,
    }
  }
}

fn elapsed_ns(epoch: Instant) -> u64 {
  epoch.elapsed().as_nanos() as u64
}

/// The result row of a ping-pong.
pub fn ping_pong_row(candidate: &str, runs: &[f64]) -> Value {
  let mut sorted = runs.to_vec();
  sorted.sort_by(f64::total_cmp);

  json!({
    "test": "pingpong",
    "candidate": candidate,
    "one_way_ns": sorted[sorted.len() / 2],
    "min_ns": sorted[0],
    "max_ns": sorted[sorted.len() - 1],
  })
}

/// The result row of an open-loop run.
pub fn load_row(test: &str, candidate: &str, load: &Load, latency: &Latency) -> Value {
  let h = &latency.histogram;

  json!({
    "test": test,
    "candidate": candidate,
    "producers": load.producers,
    "rate": load.rate,
    "burst": load.burst,
    "achieved": latency.achieved,
    "saturated": latency.saturated,
    "p50_ns": h.value_at_quantile(0.5),
    "p90_ns": h.value_at_quantile(0.9),
    "p95_ns": h.value_at_quantile(0.95),
    "p99_ns": h.value_at_quantile(0.99),
    "p999_ns": h.value_at_quantile(0.999),
    "p9999_ns": h.value_at_quantile(0.9999),
    "max_ns": h.max(),
  })
}

/// The result row for the clock every latency is read with: what one read costs, and where the
/// operating system gets it.
pub fn clock_row() -> Value {
  const READS: u32 = 10_000;
  let start = Instant::now();
  let mut last = start;
  for _ in 0..READS {
    last = std::hint::black_box(Instant::now());
  }
  let source = std::fs::read_to_string("/sys/devices/system/clocksource/clocksource0/current_clocksource")
    .map_or_else(|_| "unknown".to_string(), |source| source.trim().to_string());

  json!({
    "test": "clock",
    "id": "clock",
    "read_ns": (last - start).as_nanos() as f64 / f64::from(READS),
    "source": source,
  })
}

/// The tests in report order, with what each measures.
pub const TESTS: [(&str, &str); 4] = [
  (
    "pingpong",
    "one-way hand-off: round trips over two queues, timed in bulk and halved",
  ),
  ("steady", "steady offered load: receive time minus scheduled send time"),
  (
    "oversubscribed",
    "steady offered load from twice as many producers as logical CPUs",
  ),
  ("bursts", "64 records per producer, every producer at once, every 200 µs"),
];

/// Markdown tables of latency `rows`, one per test, the lowest p99 of each load in bold.
pub fn markdown(rows: &[Value]) -> String {
  let mut out = String::new();
  if let Some(clock) = rows.iter().find(|row| row["test"] == "clock") {
    let read = duration(clock["read_ns"].as_f64().unwrap_or(f64::NAN));
    let _ = writeln!(
      out,
      "One clock read costs {read} here (clocksource {}); every latency below carries up to one.\n",
      text(clock, "source")
    );
  }
  for (test, about) in TESTS {
    let rows: Vec<&Value> = rows.iter().filter(|row| row["test"] == test).collect();
    if rows.is_empty() {
      continue;
    }
    let _ = writeln!(out, "### Latency: {test}\n\n{about}.\n");
    if test == "pingpong" {
      out.push_str("| candidate | one-way, median | fastest run | slowest run |\n|---|---:|---:|---:|\n");
      for row in rows {
        let ns = |field: &str| duration(row[field].as_f64().unwrap_or(f64::NAN));
        let _ = writeln!(
          out,
          "| {} | {} | {} | {} |",
          text(row, "candidate"),
          ns("one_way_ns"),
          ns("min_ns"),
          ns("max_ns")
        );
      }
    } else {
      out.push_str("| producers | offered | candidate | delivered | p50 | p90 | p95 | p99 | p99.9 | p99.99 | max |\n");
      out.push_str("|---:|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|\n");
      for row in &rows {
        let same_load = |other: &&&Value| other["producers"] == row["producers"] && other["rate"] == row["rate"];
        let best = rows
          .iter()
          .filter(same_load)
          .filter(|other| other["saturated"] == false)
          .filter_map(|other| other["p99_ns"].as_u64())
          .min();
        let rate = |field: &str| format!("{:.2} M/s", row[field].as_f64().unwrap_or(f64::NAN) / 1e6);
        let cells = if row["saturated"] == true {
          vec!["saturated".to_string(); 7]
        } else {
          ["p50_ns", "p90_ns", "p95_ns", "p99_ns", "p999_ns", "p9999_ns", "max_ns"]
            .iter()
            .map(|field| {
              // A saved row from before a quantile existed carries no such field; a missing
              // measurement must not render as a real near-zero one.
              let Some(ns) = row[*field].as_u64() else {
                return "—".to_string();
              };
              let cell = duration(ns as f64);
              if *field == "p99_ns" && Some(ns) == best {
                format!("**{cell}**")
              } else {
                cell
              }
            })
            .collect()
        };
        let _ = writeln!(
          out,
          "| {} | {} | {} | {} | {} |",
          row["producers"],
          rate("rate"),
          text(row, "candidate"),
          rate("achieved"),
          cells.join(" | ")
        );
      }
    }
    out.push('\n');
  }

  out
}

fn text<'v>(row: &'v Value, field: &str) -> &'v str {
  row[field].as_str().unwrap_or("?")
}
