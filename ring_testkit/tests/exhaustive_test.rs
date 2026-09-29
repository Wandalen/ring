//! `docs/feature/188_loom_and_testkit_helpers.md` — the exhaustive half.
//!
//! The other half of feature 188: "a model checker that explores interleavings
//! exhaustively rather than sampling whichever one the scheduler produced".
//! `tests/testkit_test.rs` runs one script many times and gets the same answer;
//! this file runs a deliberately tiny case against a memory model weaker than
//! any real hardware and checks *every* interleaving of it. Neither subsumes
//! the other — one has scale without coverage, the other coverage without
//! scale.
//!
//! ```text
//! RUSTFLAGS="--cfg loom" cargo test -p ring_testkit --test exhaustive_test
//! ```
//!
//! **The whole file is `cfg( loom )`.** Under `--cfg loom`, `ring_atomic`
//! swaps in loom's atomics, and those panic when touched outside a
//! `loom::model` — so a `ring_core::Ring` cannot even be *constructed* here
//! except inside the model closure. An ordinary `cargo test` compiles this file
//! to nothing, which is why the loom bridge itself (`leak`, `leak_ends`) is
//! also exercised by `testkit_test.rs` under the default configuration.
//!
//! # The declared bound
//!
//! One producer, one consumer, a ring of **2 slots**, and **one** push. That is
//! the bound, and it is small on purpose: loom's execution count grows with the
//! number of atomic operations, and the family's existing models (`ring_spsc`,
//! `ring_mpsc`) settled on the same shape for the same reason. What is being
//! explored is the *handshake* — a claim, a publish, a read — not throughput.

#![cfg(loom)]

use loom::sync::Arc;
use loom::sync::atomic::{AtomicUsize, Ordering};
use ring_config::RingConfig;
use ring_core::Ring;
use ring_testkit::{audit_received, leak_ends};

/// The declared bound: two slots.
const CAPACITY: usize = 2;

/// What the producer writes alongside the record. Any value a zeroed cell
/// cannot hold by accident — zero would be indistinguishable from "never
/// written".
const WRITTEN: usize = 0xABC;

/// The two ends of a fresh ring, both `'static`.
///
/// Constructed inside the model closure, never outside it: the cursors are
/// loom atomics under this cfg and panic if touched with no model running.
fn ends() -> (ring_core::Producer<'static, u32>, ring_core::Consumer<'static, u32>) {
  let config = RingConfig::new(CAPACITY).expect("a power of two");
  leak_ends(Ring::new(&config).expect("the default policy is accepted"))
}

/// **The reached-test.** No interleaving delivers a record that was never
/// published, delivers one twice, or delivers them out of order.
///
/// The consumer takes one look rather than spinning: every point at which that
/// look could land is a separate execution loom already runs, so a retry loop
/// would multiply executions without adding an observation.
#[test]
fn no_interleaving_delivers_a_record_that_was_not_published() {
  loom::model(|| {
    let (mut producer, mut consumer) = ends();

    let producing = loom::thread::spawn(move || {
      producer.try_push(0).expect("an empty ring admits one");
    });

    let draining = loom::thread::spawn(move || {
      let mut received = Vec::new();
      if let Some(record) = consumer.try_recv() {
        received.push(record);
      }

      assert!(received.len() <= 1, "drained {} past the only push ever made", received.len());
      assert_eq!(
        audit_received(&received, 1),
        Ok(()),
        "a drain saw records that could not have been published: {received:?}",
      );
    });

    producing.join().expect("the producer thread");
    draining.join().expect("the drain thread");
  });
}

/// A delivered record carries the write that preceded its publish.
///
/// The store below and the cursor store inside `try_push` bracket the window
/// the model explores. A publish that does not *release* lets the drain reach
/// its assertion with `payload` still zero — which is the failure this model
/// exists to rule out, and the reason a sampled test cannot stand in for it.
#[test]
fn a_delivered_record_carries_the_write_that_preceded_it() {
  loom::model(|| {
    let (mut producer, mut consumer) = ends();
    let payload = Arc::new(AtomicUsize::new(0));

    let producing = loom::thread::spawn({
      let payload = Arc::clone(&payload);
      move || {
        payload.store(WRITTEN, Ordering::Relaxed);
        producer.try_push(0).expect("an empty ring admits one");
      }
    });

    let draining = loom::thread::spawn({
      let payload = Arc::clone(&payload);
      move || {
        if consumer.try_recv().is_none() {
          return;
        }

        assert_eq!(
          payload.load(Ordering::Relaxed),
          WRITTEN,
          "a delivered record did not carry the write that preceded its publish",
        );
      }
    });

    producing.join().expect("the producer thread");
    draining.join().expect("the drain thread");
  });
}

/// The consumer never observes further than the producer published.
///
/// Distinct from the first test, which checks the *values* delivered. This one
/// checks the *count*: an occupancy reading above what was published means the
/// consumer is being offered a slot the producer has not finished with, which
/// is a different bug from delivering the wrong record and would survive the
/// value check on a ring whose slots happened to be zeroed.
#[test]
fn the_consumer_never_sees_further_than_the_producer_published() {
  loom::model(|| {
    let (mut producer, consumer) = ends();

    let producing = loom::thread::spawn(move || {
      producer.try_push(0).expect("an empty ring admits one");
    });

    let observing = loom::thread::spawn(move || {
      assert!(
        consumer.len() <= 1,
        "the ring reported {} records after one push",
        consumer.len()
      );
    });

    producing.join().expect("the producer thread");
    observing.join().expect("the observing thread");
  });
}
