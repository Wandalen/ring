//! The threaded throughput driver: every thread released from one start line, the clock on the
//! consumer from that line to its last record, and the records checked after the clock stops.

use std::hint::spin_loop;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use super::candidates::{Candidate, Record, Run, Rx, SEQ_BITS, Tx};
use super::topology;

/// Records per batch push in the standard modes.
pub const BATCH: usize = 32;

/// How each side moves records.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mode {
  /// Records per push: 1 is [`Tx::try_push`], more is [`Tx::push_batch`].
  pub push: usize,
  /// Whether the consumer takes everything available per pop rather than one record.
  pub pop_all: bool,
}

impl Mode {
  /// One record per push and per pop.
  pub const PUSH1_POP1: Self = Self { push: 1, pop_all: false };
  /// One record per push, everything available per pop.
  pub const PUSH1_POPN: Self = Self { push: 1, pop_all: true };

  /// `push` records per push, everything available per pop.
  pub const fn batch(push: usize) -> Self {
    Self { push, pop_all: true }
  }

  /// The modes `C` has an API for: one record or everything per pop, one or [`BATCH`] per push.
  pub fn of<C: Candidate>() -> Vec<Self> {
    let mut modes = vec![Self::PUSH1_POP1, Self::PUSH1_POPN];
    if C::PUSH_BATCH {
      modes.extend([
        Self {
          push: BATCH,
          pop_all: false,
        },
        Self::batch(BATCH),
      ]);
    }

    modes
  }

  /// The label in benchmark ids: `push1_pop1`, `push32_popN`.
  pub fn label(self) -> String {
    format!("push{}_pop{}", self.push, if self.pop_all { "N" } else { "1" })
  }
}

/// Push `records` through `tx` the way `mode` says. Panics on a refusal: the caller has made room.
pub fn fill<R: Record, T: Tx<R>>(tx: &mut T, records: &[R], mode: Mode) {
  if mode.push > 1 {
    for chunk in records.chunks(mode.push) {
      assert_eq!(tx.push_batch(chunk), chunk.len(), "batch push refused with room left");
    }
  } else {
    for &record in records {
      assert!(tx.try_push(record), "push refused with room left");
    }
  }
}

/// One pop the way `mode` says; how many records it took.
pub fn pop<R, X: Rx<R>>(rx: &mut X, mode: Mode, sink: &mut impl FnMut(R)) -> u64 {
  if mode.pop_all {
    rx.pop_batch(usize::MAX, sink) as u64
  } else {
    u64::from(rx.try_pop(sink))
  }
}

/// A threaded run.
#[derive(Clone, Debug)]
pub struct Shape {
  /// Slots, the same for every candidate.
  pub capacity: usize,
  /// Producer threads; the consumer is one more.
  pub producers: usize,
  /// Records each producer sends.
  pub per_producer: u64,
  /// How each side moves records.
  pub mode: Mode,
  /// Logical CPUs for the consumer and then each producer; empty leaves placement to the scheduler.
  pub cpus: Vec<usize>,
  /// Whether the consumer stamps every receive to find the longest wait between two.
  pub gaps: bool,
}

impl Shape {
  /// `records` split evenly over `producers`, unpinned, without gap tracking.
  pub fn new(capacity: usize, producers: usize, records: u64, mode: Mode) -> Self {
    Self {
      capacity,
      producers,
      per_producer: records / producers as u64,
      mode,
      cpus: Vec::new(),
      gaps: false,
    }
  }

  /// Records the consumer receives.
  pub const fn records(&self) -> u64 {
    self.producers as u64 * self.per_producer
  }
}

/// A validated run.
#[derive(Clone, Copy, Debug)]
pub struct Outcome {
  /// From the start line to the consumer receiving the last record.
  pub elapsed: Duration,
  /// Pushes refused by a full queue, over all producers.
  pub full_spins: u64,
  /// Pops that found the queue empty.
  pub empty_spins: u64,
  /// The longest wait between two receives; zero unless [`Shape::gaps`] was set.
  pub worst_gap: Duration,
}

/// Run `shape` through a fresh `C` and validate it. Building, spawning and validation stay
/// outside the clock.
///
/// # Panics
///
/// Naming `C`, when a record is lost, duplicated, torn or reordered within its producer.
pub fn run<C: Candidate>(shape: &Shape) -> Outcome {
  assert!(
    shape.producers <= C::MAX_PRODUCERS,
    "{} takes at most {} producer(s)",
    C::NAME,
    C::MAX_PRODUCERS
  );
  assert!(shape.producers < 1 << (64 - SEQ_BITS) && shape.per_producer < 1 << SEQ_BITS);

  C::new(shape.capacity).split(shape.producers, Drive { shape, name: C::NAME })
}

struct Drive<'s> {
  shape: &'s Shape,
  name: &'static str,
}

struct Consumed {
  elapsed: Duration,
  empty: u64,
  worst_gap: Duration,
}

impl<R: Record> Run<R> for Drive<'_> {
  type Output = Outcome;

  fn run<T: Tx<R> + Send, X: Rx<R> + Send>(self, producers: Vec<T>, consumer: X) -> Outcome {
    let Self { shape, name } = self;
    let count = producers.len();
    let ready = AtomicUsize::new(0);
    let finished = AtomicUsize::new(0);
    let mut check = Check::new(count, shape.per_producer);

    let (mut consumer, check, consumed, full_spins) = thread::scope(|scope| {
      let (ready, finished) = (&ready, &finished);
      let receiving = scope.spawn(move || {
        let mut consumer = consumer;
        topology::pin(shape.cpus.first().copied());
        start_line(ready, count + 1);
        let consumed = if shape.gaps {
          consume::<R, X, true>(&mut consumer, &mut check, finished, count, shape.mode)
        } else {
          consume::<R, X, false>(&mut consumer, &mut check, finished, count, shape.mode)
        };
        (consumer, check, consumed)
      });
      let sending: Vec<_> = producers
        .into_iter()
        .zip(0..)
        .map(|(mut tx, id)| {
          scope.spawn(move || {
            let mut chunk = Vec::with_capacity(shape.mode.push);
            topology::pin(shape.cpus.get(1 + id as usize).copied());
            start_line(ready, count + 1);
            let full = produce(&mut tx, id, shape, &mut chunk);
            // Release: pairs with the consumer's Acquire load in `consume`, so a count that
            // includes this producer also shows its last record.
            finished.fetch_add(1, Ordering::Release);
            full
          })
        })
        .collect();
      let full = sending.into_iter().map(|t| t.join().expect("a producer panicked")).sum();
      let (consumer, check, consumed) = receiving.join().expect("the consumer panicked");

      (consumer, check, consumed, full)
    });

    // Every producer has returned, so whatever is still queued was never pushed.
    let mut left = 0;
    while consumer.pop_batch(usize::MAX, &mut |_| left += 1) > 0 {}
    check.verify(name, left);

    Outcome {
      elapsed: consumed.elapsed,
      full_spins,
      empty_spins: consumed.empty,
      worst_gap: consumed.worst_gap,
    }
  }
}

/// A spinning rendezvous: every thread leaves within nanoseconds of the last arrival, which a
/// parking `std::sync::Barrier` does not give.
pub fn start_line(ready: &AtomicUsize, parties: usize) {
  // AcqRel / Acquire: what a thread did before arriving happens-before every thread's start.
  ready.fetch_add(1, Ordering::AcqRel);
  while ready.load(Ordering::Acquire) < parties {
    spin_loop();
  }
}

/// Send the producer's records, spinning on a full queue; how many pushes were refused.
fn produce<R: Record, T: Tx<R>>(tx: &mut T, id: u64, shape: &Shape, chunk: &mut Vec<R>) -> u64 {
  let first = id << SEQ_BITS;
  let end = first + shape.per_producer;
  let mut full = 0;

  if shape.mode.push > 1 {
    let mut next = first;
    while next < end {
      chunk.clear();
      chunk.extend((next..end.min(next + shape.mode.push as u64)).map(R::new));
      let mut sent = 0;
      while sent < chunk.len() {
        match tx.push_batch(&chunk[sent..]) {
          0 => {
            full += 1;
            spin_loop();
          }
          n => sent += n,
        }
      }
      next += chunk.len() as u64;
    }
  } else {
    for key in first..end {
      let record = R::new(key);
      while !tx.try_push(record) {
        full += 1;
        spin_loop();
      }
    }
  }

  full
}

/// Receive until every record is in, or until every producer has returned and the queue is
/// empty — a lost record must end the run, not hang it.
fn consume<R: Record, X: Rx<R>, const GAPS: bool>(
  rx: &mut X,
  check: &mut Check,
  finished: &AtomicUsize,
  producers: usize,
  mode: Mode,
) -> Consumed {
  let expected = check.expected;
  let mut sink = |record: R| check.record(record.key(), record.whole());
  let (mut taken, mut empty, mut worst_gap) = (0, 0, Duration::ZERO);
  let start = Instant::now();
  let mut last = start;

  while taken < expected {
    let n = pop(rx, mode, &mut sink);
    if n > 0 {
      taken += n;
      if GAPS {
        let now = Instant::now();
        worst_gap = worst_gap.max(now - last);
        last = now;
      }
      continue;
    }
    // Acquire: pairs with each producer's Release increment. Once all have returned, one more
    // pop tells a race with the last push from records that are gone.
    if finished.load(Ordering::Acquire) == producers {
      match pop(rx, mode, &mut sink) {
        0 => break,
        n => taken += n,
      }
      continue;
    }
    empty += 1;
    spin_loop();
  }

  Consumed {
    elapsed: start.elapsed(),
    empty,
    worst_gap,
  }
}

/// The consumer's work per record, the same for every candidate and driver: a checksum and a
/// per-producer order check. Judged after the clock stops.
#[derive(Debug)]
pub struct Check {
  /// Per producer, the key it must deliver next.
  next: Vec<u64>,
  per_producer: u64,
  /// Records the run should deliver.
  pub expected: u64,
  received: u64,
  misordered: u64,
  torn: u64,
  sum: u64,
}

impl Check {
  /// Expect `per_producer` keys, in order, from each of `producers`.
  pub fn new(producers: usize, per_producer: u64) -> Self {
    Self {
      next: (0..producers as u64).map(|id| id << SEQ_BITS).collect(),
      per_producer,
      expected: producers as u64 * per_producer,
      received: 0,
      misordered: 0,
      torn: 0,
      sum: 0,
    }
  }

  /// Account for one received record by its key, and whether it arrived whole.
  pub fn record(&mut self, key: u64, whole: bool) {
    self.received += 1;
    self.torn += u64::from(!whole);
    match self.next.get_mut((key >> SEQ_BITS) as usize) {
      Some(next) if *next == key => *next += 1,
      _ => self.misordered += 1,
    }
    self.sum = self.sum.wrapping_add(key);
  }

  /// The checksum of every key each producer sends, modulo 2⁶⁴.
  fn expected_sum(&self) -> u64 {
    let n = u128::from(self.per_producer);
    let sequences = n * n.saturating_sub(1) / 2;
    let total: u128 = (0..self.next.len() as u128).map(|id| (id << SEQ_BITS) * n + sequences).sum();

    total as u64
  }

  /// Panic, naming the candidate, unless every key arrived once, whole and in order, with `left`
  /// records still queued after the producers returned.
  pub fn verify(&self, name: &str, left: u64) {
    let complete = self
      .next
      .iter()
      .zip(0u64..)
      .all(|(&next, id)| next == (id << SEQ_BITS) + self.per_producer);
    let expected_sum = self.expected_sum();
    let clean = self.received == self.expected && self.misordered == 0 && self.torn == 0 && left == 0;
    if !(complete && clean && self.sum == expected_sum) {
      // GitHub annotation — surfaces as a highlighted error, not just `exit 101`.
      eprintln!(
        "::error title=benchmark::{name} failed::benchmark::{name} torn/ordering failure — received {}/{}, {} out of order, {} torn, {} left, checksum {:#x} expected {expected_sum:#x} (per_producer={}, producers={}, expected={})",
        self.received,
        self.expected,
        self.misordered,
        self.torn,
        left,
        self.sum,
        self.per_producer,
        self.next.len(),
        self.expected,
      );
      panic!(
        "{name}: records lost, duplicated, reordered or torn — received {} of {}, {} out of order, {} torn, \
         {} left in the queue, checksum {:#x}, expected {expected_sum:#x} (per_producer={}, producers={})",
        self.received,
        self.expected,
        self.misordered,
        self.torn,
        left,
        self.sum,
        self.per_producer,
        self.next.len(),
      );
    }
  }
}
