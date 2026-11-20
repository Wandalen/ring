//! `ring_mpsc` behind a one-line text protocol, for the deck in `../index.html`.
//!
//! JS writes a command into the input buffer (`io_input`), calls `io_run`, and reads the JSON
//! answer back from `io_output`. One ring, three producers (ids `1..=3`) writing through one
//! [`Producer`], and the one [`Consumer`]. A producer holds at most one claim at a time.
//!
//! Commands:
//! - `reset cap` — a fresh ring of `cap` slots
//! - `claim p` — producer `p` takes the next sequence and keeps its [`Reserved`] guard
//! - `publish p` — `p` writes its next letter into the held slot and drops the guard, which publishes
//! - `push p` — [`Producer::push`]: claim, write and publish in one call
//! - `drain` — [`Consumer::drain`]: every record up to the first unpublished sequence, then the commit
//! - `state`
//!
//! Every answer carries the ring as `ring_mpsc` reports it and two checks over everything drained
//! so far: the sequences arrived in claim order with no gap or repeat, and each record is the one
//! its producer wrote.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::atomic::Ordering;

use ring_atomic::SeqCell;
use ring_mpsc::{Consumer, Producer, Reserved, Ring, UNSTAMPED};
use ring_slot::TypedSlot;
use ring_types::{Capacity, RingError};

const PRODUCERS: usize = 3;

/// Who sent the record, and that producer's running count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Letter {
  from: usize,
  n: u32,
}

type Record = TypedSlot<Letter>;

struct Sim {
  ring: &'static Ring<Record>,
  producer: Producer<'static, Record>,
  consumer: Consumer<'static, Record>,
  held: [Option<Reserved<'static, Record>>; PRODUCERS],
  sent: [u32; PRODUCERS],
  /// Published and not yet drained, by sequence.
  written: BTreeMap<u64, Letter>,
  /// The sequence the next drained record must carry.
  next: u64,
  received: u64,
  full: u64,
  in_order: bool,
  intact: bool,
}

impl Sim {
  /// Leaks the ring and its ends: the guards held between commands borrow them for good.
  fn new(capacity: Capacity) -> Self {
    let ring = Box::leak(Box::new(Ring::new(capacity)));
    let ends = Box::leak(Box::new(ring.ends()));
    let shared = ends.ring();
    let (producer, consumer) = ends.split();
    Self {
      ring: shared,
      producer,
      consumer,
      held: std::array::from_fn(|_| None),
      sent: [0; PRODUCERS],
      written: BTreeMap::new(),
      next: 0,
      received: 0,
      full: 0,
      in_order: true,
      intact: true,
    }
  }

  fn claim(&mut self, p: usize) -> String {
    if let Some(guard) = &self.held[p] {
      return format!(r#"{{"kind":"holding","p":{},"seq":{}}}"#, p + 1, guard.sequence().0);
    }
    match self.producer.claim() {
      Ok(guard) => {
        let seq = guard.sequence().0;
        self.held[p] = Some(guard);
        format!(r#"{{"kind":"claimed","p":{},"seq":{seq}}}"#, p + 1)
      }
      Err(e) => self.refused(p, e),
    }
  }

  fn publish(&mut self, p: usize) -> String {
    let Some(mut guard) = self.held[p].take() else {
      return format!(r#"{{"kind":"idle","p":{}}}"#, p + 1);
    };
    let seq = guard.sequence().0;
    let letter = self.letter(p);
    // The drain that freed this slot took its record, so nothing is displaced.
    self.intact &= guard.set(letter).is_none();
    self.written.insert(seq, letter);
    // Dropping the guard stores the stamp: this is the publish.
    drop(guard);
    format!(r#"{{"kind":"published","p":{},"seq":{seq},"n":{}}}"#, p + 1, letter.n)
  }

  fn push(&mut self, p: usize) -> String {
    if let Some(guard) = &self.held[p] {
      return format!(r#"{{"kind":"holding","p":{},"seq":{}}}"#, p + 1, guard.sequence().0);
    }
    let letter = Letter {
      from: p + 1,
      n: self.sent[p] + 1,
    };
    match self.producer.push(letter) {
      Ok(seq) => {
        self.sent[p] = letter.n;
        self.written.insert(seq.0, letter);
        format!(r#"{{"kind":"pushed","p":{},"seq":{},"n":{}}}"#, p + 1, seq.0, letter.n)
      }
      Err(e) => self.refused(p, e),
    }
  }

  fn drain(&mut self) -> String {
    let mut records = String::new();
    let mut batch = self.consumer.drain();
    for (offset, seq) in batch.sequences().enumerate() {
      let got = batch.get_mut(offset).and_then(TypedSlot::take);
      let wrote = self.written.remove(&seq.0);
      self.in_order &= seq.0 == self.next;
      self.intact &= got.is_some() && got == wrote;
      self.next = seq.0 + 1;
      self.received += 1;
      let Letter { from, n } = got.unwrap_or(Letter { from: 0, n: 0 });
      if offset > 0 {
        records.push(',');
      }
      let _ = write!(records, r#"{{"seq":{},"from":{from},"n":{n}}}"#, seq.0);
    }
    // Dropping the batch commits it: the slots are free for the next lap.
    drop(batch);
    let position = self.consumer.position().0;
    let blocked = self.holder(position).map_or("null".to_owned(), |p| (p + 1).to_string());
    format!(r#"{{"kind":"drained","records":[{records}],"next":{position},"blocked_by":{blocked}}}"#)
  }

  fn letter(&mut self, p: usize) -> Letter {
    self.sent[p] += 1;
    Letter {
      from: p + 1,
      n: self.sent[p],
    }
  }

  fn holder(&self, seq: u64) -> Option<usize> {
    self
      .held
      .iter()
      .position(|g| g.as_ref().is_some_and(|g| g.sequence().0 == seq))
  }

  fn refused(&mut self, p: usize, e: RingError) -> String {
    if e == RingError::Full {
      self.full += 1;
      return format!(r#"{{"kind":"full","p":{}}}"#, p + 1);
    }
    format!(r#"{{"kind":"error","p":{},"error":"{e}"}}"#, p + 1)
  }

  fn state(&self, out: &mut String) {
    let opt = |v: Option<u64>| v.map_or("null".to_owned(), |v| v.to_string());
    let list = |items: &mut dyn Iterator<Item = String>| items.collect::<Vec<_>>().join(",");
    let stamps = list(&mut self.ring.stamps().iter().map(|s| {
      let seq = s.load(Ordering::Acquire);
      opt((seq != UNSTAMPED).then_some(seq.0))
    }));
    let held = list(&mut self.held.iter().map(|g| opt(g.as_ref().map(|g| g.sequence().0))));
    let sent = list(&mut self.sent.iter().map(u32::to_string));
    let published = list(
      &mut self
        .written
        .iter()
        .map(|(seq, l)| format!(r#"{{"seq":{seq},"from":{},"n":{}}}"#, l.from, l.n)),
    );
    let _ = write!(
      out,
      concat!(
        r#""ring":{{"cap":{},"claimed":{},"committed":{},"published_through":{},"available":{},"stamps":[{}]}},"#,
        r#""held":[{}],"sent":[{}],"published":[{}],"#,
        r#""check":{{"received":{},"in_order":{},"intact":{},"full":{}}}"#,
      ),
      self.ring.capacity().get(),
      self.producer.claimed().0,
      self.ring.committed().0,
      opt(self.ring.published_through().map(|s| s.0)),
      self.consumer.available(),
      stamps,
      held,
      sent,
      published,
      self.received,
      self.in_order,
      self.intact,
      self.full,
    );
  }
}

thread_local! {
  static SIM: RefCell<Option<Sim>> = const { RefCell::new(None) };
  static INPUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
  static OUTPUT: RefCell<String> = const { RefCell::new(String::new()) };
}

/// Room for a `len`-byte command; JS writes it at the returned address.
#[unsafe(no_mangle)]
pub extern "C" fn io_input(len: usize) -> *mut u8 {
  INPUT.with_borrow_mut(|input| {
    input.clear();
    input.resize(len, 0);
    input.as_mut_ptr()
  })
}

/// Run the command in the input buffer; returns the answer's length in bytes.
#[unsafe(no_mangle)]
pub extern "C" fn io_run() -> usize {
  let command = INPUT.with_borrow(|input| String::from_utf8_lossy(input).into_owned());
  let answer = run(&command);
  OUTPUT.with_borrow_mut(|output| {
    *output = answer;
    output.len()
  })
}

/// The last answer's address.
#[unsafe(no_mangle)]
pub extern "C" fn io_output() -> *const u8 {
  OUTPUT.with_borrow(|output| output.as_ptr())
}

/// One command in, one JSON answer out.
#[must_use]
pub fn run(command: &str) -> String {
  let words: Vec<&str> = command.split_whitespace().collect();
  SIM.with_borrow_mut(|slot| {
    if words.first() == Some(&"reset") {
      let capacity = match words.get(1).map(|w| w.parse::<usize>()) {
        Some(Ok(n)) if words.len() == 2 => Capacity::new(n),
        _ => return error("usage: reset cap"),
      };
      return match capacity {
        Ok(capacity) => answer(r#"{"kind":"reset"}"#, slot.insert(Sim::new(capacity))),
        Err(e) => error(&e.to_string()),
      };
    }
    let Some(sim) = slot.as_mut() else {
      return error("no ring yet: reset first");
    };
    let producer = |w: Option<&&str>| match w.and_then(|w| w.parse::<usize>().ok()) {
      Some(p @ 1..=PRODUCERS) if words.len() == 2 => Ok(p - 1),
      _ => Err("producer must be 1, 2 or 3"),
    };
    let result = match words.first().copied() {
      Some("claim") => producer(words.get(1)).map(|p| sim.claim(p)),
      Some("publish") => producer(words.get(1)).map(|p| sim.publish(p)),
      Some("push") => producer(words.get(1)).map(|p| sim.push(p)),
      Some("drain") if words.len() == 1 => Ok(sim.drain()),
      Some("state") if words.len() == 1 => Ok(r#"{"kind":"state"}"#.to_owned()),
      _ => Err("unknown command"),
    };
    match result {
      Ok(result) => answer(&result, sim),
      Err(e) => error(e),
    }
  })
}

fn answer(result: &str, sim: &Sim) -> String {
  let mut out = format!(r#"{{"ok":true,"result":{result},"#);
  sim.state(&mut out);
  out.push('}');
  out
}

fn error(message: &str) -> String {
  format!(r#"{{"ok":false,"error":"{message}"}}"#)
}

#[cfg(test)]
mod tests {
  use super::run;

  #[test]
  fn the_drain_waits_for_the_first_claim_even_when_later_ones_are_published() {
    assert!(run("reset 8").contains(r#""kind":"reset""#));
    assert!(run("claim 1").contains(r#""seq":0"#));
    assert!(run("push 2").contains(r#""seq":1"#));
    assert!(run("push 3").contains(r#""seq":2"#));
    let waiting = run("drain");
    assert!(waiting.contains(r#""records":[],"next":0,"blocked_by":1"#), "{waiting}");
    assert!(run("publish 1").contains(r#""kind":"published","p":1,"seq":0"#));
    let drained = run("drain");
    assert!(
      drained.contains(r#""records":[{"seq":0,"from":1,"n":1},{"seq":1,"from":2,"n":1},{"seq":2,"from":3,"n":1}]"#),
      "{drained}"
    );
    assert!(drained.contains(r#""in_order":true,"intact":true"#), "{drained}");
  }

  #[test]
  fn a_full_ring_refuses_until_the_consumer_commits() {
    assert!(run("reset 4").contains(r#""kind":"reset""#));
    for _ in 0..4 {
      assert!(run("push 1").contains(r#""kind":"pushed""#));
    }
    assert!(run("push 2").contains(r#""kind":"full""#));
    assert!(run("drain").contains(r#""next":4,"blocked_by":null"#));
    for lap in 0..20 {
      assert!(run("push 2").contains(r#""kind":"pushed""#), "lap {lap}");
      assert!(
        run("drain").contains(r#""in_order":true,"intact":true,"full":1"#),
        "lap {lap}"
      );
    }
  }
}
