//! `ring_shutdown` — close, drain-all, and reset.
//!
//! # What this file is arranged around
//!
//! This file carries the reached-test for
//! `docs/feature/184_close_reset_and_drain_all.md`, whose three operations are
//! stated together because they are only useful together: closing without
//! draining strands records, draining without closing may never finish, and
//! resetting without either hands the next run a dirty ring. So the reached-test
//! ([`the_three_operations_hand_back_a_ring_fit_for_the_next_run`]) exercises
//! the sequence rather than the three parts, and asserts the property the
//! feature actually promises — that the ring afterwards is indistinguishable
//! from a fresh one.
//!
//! # What is deliberately not here
//!
//! **No multi-threaded close race.** The flag is one `AtomicBool` with
//! `Release`/`Acquire`, and a test that spawns two threads and observes "it
//! worked" observes nothing: the interleaving that would break a weaker
//! ordering is not the one a test schedules. What is testable is that close is
//! idempotent and that a closed ring refuses, both of which are here.
//!
//! **No test that an unguarded producer is stopped.** It is not — that is this
//! crate's central limitation, not an oversight, and
//! [`an_unguarded_producer_publishes_straight_through_a_close`] asserts the
//! limitation rather than papering over it.

#![cfg(test)]
// Ordinary tests, compiled out under `--cfg loom`. That cfg swaps
// `ring_atomic`'s atomics for loom's instrumented ones across the whole
// family, and those panic the moment they are touched outside a
// `loom::model` closure — so without this gate a family-wide loom run dies
// here instead of reaching the models in `ring_spsc`, `ring_mpsc`,
// `ring_publish` and `ring_testkit`.
#![cfg(not(loom))]

use core::sync::atomic::Ordering;

use ring_config::RingConfig;
use ring_core::Ring;
use ring_cursor::{CursorPair, SeqCell};
use ring_shutdown::{Refusal, Shutdown, Wake, for_space_or_close, reset, wait_for_close};
use ring_types::{Capacity, OverflowPolicy, RingError, Seq, WaitKind};

/// A ring of `slots` capacity, in the default (SPSC, drop-newest) configuration.
fn ring(slots: usize) -> Ring<u32> {
  Ring::new(&RingConfig::new(slots).unwrap()).unwrap()
}

/// A ring of `slots` capacity that refuses rather than dropping.
///
/// The default overflow policy is `DropNewest`, under which a full ring
/// reports `Ok` and discards the record — so [`Refusal::Full`] is unreachable
/// there. Every test below that is *about* the `Full` arm uses this instead,
/// and [`a_full_drop_newest_ring_reports_success_and_keeps_nothing`] covers
/// the default's own behaviour rather than leaving it untested.
fn refusing_ring(slots: usize) -> Ring<u32> {
  let config = RingConfig::new(slots).unwrap().with_overflow(OverflowPolicy::Fail);
  Ring::new(&config).unwrap()
}

// ── The reached-test ──────────────────────────────────────────────────────

/// Feature 184's reached-test: close stops publication, drain recovers what
/// was published, and reset leaves a ring the next run cannot tell from new.
///
/// The last clause is the one that carries the feature's stated purpose
/// ("dirty tests"), so it is asserted against a *reference* ring built fresh
/// rather than against a remembered constant — a reset ring and a new ring are
/// driven through the same script and must agree at every step.
#[test]
fn the_three_operations_hand_back_a_ring_fit_for_the_next_run() {
  let mut used = ring(4);
  let mut fresh = ring(4);

  // Dirty the first ring: publish, then leave records unread.
  {
    let mut ends = used.ends();
    let (mut producer, mut consumer) = ends.split();
    assert_eq!(producer.try_push_batch(&mut [10, 11, 12].into_iter()), 3);

    let shutdown = Shutdown::new();
    let mut guarded = shutdown.guard(producer);

    // Close stops publication.
    let stopped = shutdown.close();
    assert!(guarded.try_push(13).unwrap_err().is_closed());

    // Drain recovers everything that was published, and nothing else.
    let mut recovered = Vec::new();
    assert_eq!(stopped.drain_all(&mut consumer, &mut recovered), 3);
    assert_eq!(recovered, [10, 11, 12], "every published record, in order");

    stopped.reopen();
  }

  // The two rings must now be indistinguishable under the same script.
  let script = [20, 21, 22, 23];
  let mut from_used = Vec::new();
  let mut from_fresh = Vec::new();

  for (ring, out) in [(&mut used, &mut from_used), (&mut fresh, &mut from_fresh)] {
    let mut ends = ring.ends();
    let (mut producer, mut consumer) = ends.split();
    assert_eq!(producer.try_push_batch(&mut script.into_iter()), 4, "a full capacity again");
    assert!(producer.is_full(), "and exactly full, not more");
    assert_eq!(consumer.try_recv_batch(out), 4, "the drain took the whole script");
  }

  assert_eq!(from_used, from_fresh, "a reset ring delivers what a fresh one delivers");
  assert_eq!(from_used, script);
}

// ── Close ─────────────────────────────────────────────────────────────────

/// Close is idempotent, and `admit` reports it.
#[test]
fn close_is_idempotent_and_admit_reports_it() {
  let shutdown = Shutdown::new();
  assert!(!shutdown.is_closed());
  assert_eq!(shutdown.admit(), Ok(()));

  let first = shutdown.close();
  assert!(shutdown.is_closed());
  assert_eq!(shutdown.admit(), Err(RingError::Closed));
  assert!(
    core::ptr::eq(first.shutdown(), &shutdown),
    "the token points back at its flag"
  );

  let second = shutdown.close();
  assert!(shutdown.is_closed(), "closing twice is not an error");

  second.reopen();
  assert!(!shutdown.is_closed());
  assert_eq!(shutdown.admit(), Ok(()));
}

/// `Closed` is not transient, which is what distinguishes it from `Full`.
///
/// Asserted here rather than left to `ring_types` because it is *this* crate's
/// contract that gives the distinction teeth: a producer that retries on
/// transient errors and stops on the rest behaves correctly only if `Closed`
/// falls on the right side.
#[test]
fn a_close_is_not_something_retrying_clears() {
  assert!(!RingError::Closed.is_transient());
  assert!(RingError::Full.is_transient());
}

/// `Default` and `new` agree.
#[test]
fn default_is_an_open_shutdown() {
  assert!(!Shutdown::default().is_closed());
  assert!(!Shutdown::new().is_closed());
}

// ── The guarded producer ──────────────────────────────────────────────────

/// A guarded producer refuses once closed, and hands the record back.
#[test]
fn a_guarded_producer_refuses_a_closed_ring_and_returns_the_record() {
  let mut ring = ring(4);
  let mut ends = ring.ends();
  let (producer, _consumer) = ends.split();

  let shutdown = Shutdown::new();
  let mut guarded = shutdown.guard(producer);

  assert_eq!(guarded.try_push(1), Ok(()));
  assert!(!guarded.is_blocked());
  assert_eq!(guarded.free_capacity(), 3);
  assert!(core::ptr::eq(guarded.shutdown(), &shutdown));

  let _ = shutdown.close();

  let refused = guarded.try_push(2).unwrap_err();
  assert!(refused.is_closed());
  assert_eq!(refused.reason(), RingError::Closed);
  assert_eq!(refused.into_record(), 2, "the record comes back intact");
  assert!(guarded.is_blocked(), "blocked by the close, not by occupancy");
  assert!(guarded.free_capacity() > 0, "and there was room the whole time");
}

/// A full ring refuses with `Full`, which is the arm a caller retries on.
///
/// Under `OverflowPolicy::Fail` only — see
/// [`a_full_drop_newest_ring_reports_success_and_keeps_nothing`] for what the
/// *default* policy does instead, which is the one that surprises people.
#[test]
fn a_full_guarded_producer_refuses_with_the_transient_arm() {
  let mut ring = refusing_ring(2);
  let mut ends = ring.ends();
  let (producer, _consumer) = ends.split();

  let shutdown = Shutdown::new();
  let mut guarded = shutdown.guard(producer);

  assert_eq!(guarded.try_push(1), Ok(()));
  assert_eq!(guarded.try_push(2), Ok(()));
  assert!(guarded.is_blocked(), "blocked by occupancy this time");

  let refused = guarded.try_push(3).unwrap_err();
  assert!(!refused.is_closed());
  assert_eq!(refused.reason(), RingError::Full);
  assert_eq!(refused, Refusal::Full(3));
  assert_eq!(refused.into_record(), 3);
}

/// Under the default policy a full guarded push reports success and loses the
/// record — `is_blocked` is true and `try_push` still returns `Ok`.
///
/// This is not a defect in the guard. `OverflowPolicy::DropNewest` is
/// `RingConfig`'s default and its whole contract is to discard rather than
/// refuse, so [`Refusal::Full`] is genuinely unreachable there. The trap is
/// that the *close* refusal and the *full* non-refusal look nothing alike:
/// closing a ring makes pushes visibly fail, filling one does not.
#[test]
fn a_full_drop_newest_ring_reports_success_and_keeps_nothing() {
  let mut ring = ring(2);
  let mut ends = ring.ends();
  let (producer, mut consumer) = ends.split();

  let shutdown = Shutdown::new();
  let mut guarded = shutdown.guard(producer);

  assert_eq!(guarded.try_push(1), Ok(()));
  assert_eq!(guarded.try_push(2), Ok(()));
  assert!(guarded.is_blocked(), "the ring is full");

  assert_eq!(guarded.try_push(3), Ok(()), "and the push still reports success");

  let stopped = shutdown.close();
  let mut recovered = Vec::new();
  stopped.drain_all(&mut consumer, &mut recovered);
  assert_eq!(recovered, [1, 2], "record 3 was dropped, not stored");
}

/// A closed guard consumes nothing from the iterator it was handed.
///
/// The distinction matters: a caller resuming from the same iterator after a
/// close must find every record still there, or the close silently ate one.
#[test]
fn a_closed_batch_push_consumes_nothing() {
  let mut ring = ring(8);
  let mut ends = ring.ends();
  let (producer, _consumer) = ends.split();

  let shutdown = Shutdown::new();
  let mut guarded = shutdown.guard(producer);

  let mut open_records = [1, 2, 3].into_iter();
  assert_eq!(guarded.try_push_batch(&mut open_records), 3);

  let _ = shutdown.close();

  let mut closed_records = [4, 5, 6].into_iter();
  assert_eq!(guarded.try_push_batch(&mut closed_records), 0);
  assert_eq!(closed_records.collect::<Vec<_>>(), [4, 5, 6], "nothing was taken");
}

/// The guarantee ends where the wrapper does.
///
/// This asserts the crate's limitation on purpose. `into_inner` hands back a
/// raw producer, and a raw producer has no flag to consult — so it publishes
/// into a closed ring. Anything else would require `ring_core` to carry the
/// flag, which is exactly what this crate exists to avoid.
#[test]
fn an_unguarded_producer_publishes_straight_through_a_close() {
  let mut ring = ring(4);
  let mut ends = ring.ends();
  let (producer, mut consumer) = ends.split();

  let shutdown = Shutdown::new();
  let guarded = shutdown.guard(producer);
  let _ = shutdown.close();

  let mut raw = guarded.into_inner();
  assert_eq!(raw.try_push(99), Ok(()), "the raw producer does not consult the flag");
  assert_eq!(consumer.try_recv(), Some(99));
}

// ── Drain and discard ─────────────────────────────────────────────────────

/// Draining a ring that holds more than one batch still empties it.
///
/// `try_recv_batch` returns one batch, so `drain_all` must loop. A ring filled
/// and partly drained twice leaves records on both sides of a wrap, which a
/// single-batch drain would leave behind.
#[test]
fn drain_all_loops_until_the_ring_is_actually_empty() {
  let mut ring = ring(4);
  let mut ends = ring.ends();
  let (mut producer, mut consumer) = ends.split();

  // Fill, half-drain, refill — the tail now wraps.
  assert_eq!(producer.try_push_batch(&mut [1, 2, 3, 4].into_iter()), 4);
  assert_eq!(consumer.try_recv(), Some(1));
  assert_eq!(consumer.try_recv(), Some(2));
  assert_eq!(producer.try_push_batch(&mut [5, 6].into_iter()), 2);

  let shutdown = Shutdown::new();
  let stopped = shutdown.close();

  let mut recovered = Vec::new();
  assert_eq!(stopped.drain_all(&mut consumer, &mut recovered), 4);
  assert_eq!(recovered, [3, 4, 5, 6]);
  assert!(consumer.is_empty());

  // A second drain finds nothing and says so, rather than looping.
  let mut again = Vec::new();
  assert_eq!(stopped.drain_all(&mut consumer, &mut again), 0);
  assert!(again.is_empty());
}

/// `discard_all` empties without a sink, and reports the count.
#[test]
fn discard_all_empties_the_ring_and_counts_what_it_dropped() {
  let mut ring = ring(4);
  let mut ends = ring.ends();
  let (mut producer, mut consumer) = ends.split();
  assert_eq!(producer.try_push_batch(&mut [1, 2, 3].into_iter()), 3);

  let shutdown = Shutdown::new();
  let stopped = shutdown.close();

  assert_eq!(stopped.discard_all(&mut consumer), 3);
  assert_eq!(stopped.discard_all(&mut consumer), 0);
  assert!(consumer.is_empty());
}

/// `reset` is close, discard, and reopen — and leaves the flag open.
#[test]
fn reset_discards_and_leaves_the_ring_open() {
  let mut ring = ring(4);
  let mut ends = ring.ends();
  let (mut producer, mut consumer) = ends.split();
  assert_eq!(producer.try_push_batch(&mut [1, 2].into_iter()), 2);

  let shutdown = Shutdown::new();
  let _ = shutdown.close();
  assert!(shutdown.is_closed());

  assert_eq!(reset(&shutdown, &mut consumer), 2);
  assert!(!shutdown.is_closed(), "reset ends open regardless of how it started");
  assert!(consumer.is_empty());

  assert_eq!(reset(&shutdown, &mut consumer), 0, "resetting an empty ring is not an error");
}

// ── Waiter join ───────────────────────────────────────────────────────────

/// A waiter given a budget and an open ring exhausts the budget.
#[test]
fn wait_for_close_reports_the_budget_running_out() {
  let shutdown = Shutdown::new();
  assert_eq!(wait_for_close(&shutdown, WaitKind::None, 1), Err(RingError::Empty));

  let _ = shutdown.close();
  assert!(wait_for_close(&shutdown, WaitKind::None, 1).is_ok());
}

/// A close-aware space wait reports *which* exit it took.
///
/// The `Closed` case is the one that matters: room and a close can both be
/// true at once, and a producer told to stop must stop rather than publish.
#[test]
fn for_space_or_close_names_the_exit_it_took() {
  let pair = CursorPair::new(Capacity::new(4).unwrap());
  let shutdown = Shutdown::new();

  // Room, open: ready.
  assert_eq!(for_space_or_close(&pair, &shutdown, WaitKind::None, 1), Ok(Wake::Ready));

  // Room *and* closed: still closed, because stop wins over room.
  let _ = shutdown.close();
  assert_eq!(for_space_or_close(&pair, &shutdown, WaitKind::None, 1), Ok(Wake::Closed));
  assert!(pair.may_claim(), "there was room the whole time");
}

/// No room and still open: back-pressure, reported as `Full`.
#[test]
fn for_space_or_close_reports_back_pressure_as_full() {
  let pair = CursorPair::new(Capacity::new(4).unwrap());
  pair.producer().store(Seq(4), Ordering::Release);
  assert!(!pair.may_claim(), "the pair is full");

  let shutdown = Shutdown::new();
  assert_eq!(for_space_or_close(&pair, &shutdown, WaitKind::None, 1), Err(RingError::Full));

  // Closing releases the waiter even though the ring is still full — which is
  // the whole point of a close-aware wait.
  let _ = shutdown.close();
  assert_eq!(for_space_or_close(&pair, &shutdown, WaitKind::None, 1), Ok(Wake::Closed));
}

/// `Wake` reports which of its two arms it is.
#[test]
fn wake_distinguishes_ready_from_closed() {
  assert!(Wake::Ready.is_ready());
  assert!(!Wake::Closed.is_ready());
}

/// The bounded drain returns a count when it finishes and an error when it does
/// not — which is the distinction `drain_all` cannot make.
///
/// `drain_all` against a producer that is still publishing does not terminate,
/// and a caller watching it has no way to tell that from a slow drain. The
/// bounded form makes the two different values: `Ok( n )` means the ring was
/// observed empty, `Err( RingError::Empty )` means the budget ran out with
/// records still arriving. Both arms are asserted here against the same ring,
/// so the difference is the budget and nothing else.
#[test]
fn a_bounded_drain_separates_finishing_from_running_out() {
  let mut ring = ring(4);
  let mut ends = ring.ends();
  let (mut producer, mut consumer) = ends.split();

  assert_eq!(producer.try_push_batch(&mut [1, 2, 3].into_iter()), 3);

  let shutdown = Shutdown::new();
  let stopped = shutdown.close();

  // A budget of one buys one `try_recv_batch`, which takes everything present
  // and then has no attempt left to observe the ring empty with.
  let mut hurried = Vec::new();
  assert_eq!(
    stopped.drain_all_bounded(&mut consumer, &mut hurried, 1),
    Err(RingError::Empty),
    "the budget ran out before the drain could confirm the ring was empty",
  );
  assert_eq!(hurried, [1, 2, 3], "and what it did take is still in the sink");

  // Everything is already out, so a second call confirms empty on its first
  // attempt and reports the count it moved — zero.
  let mut confirming = Vec::new();
  assert_eq!(stopped.drain_all_bounded(&mut consumer, &mut confirming, 4), Ok(0));
  assert!(confirming.is_empty());

  // A zero budget is one attempt, not none — the same reading `ring_wait`
  // gives its own `spins`, so a caller computing the number cannot ask for no
  // work at all.
  assert_eq!(stopped.drain_all_bounded(&mut consumer, &mut confirming, 0), Ok(0));
}

/// The guard hands out the capability to close, not just the identity of the
/// flag — and closing through it stops the guard that handed it over.
///
/// `Guarded::shutdown` is typed `&'a Shutdown`, so what comes back is the
/// whole surface including `close`. That route is what makes a guard passed
/// into a subsystem enough to shut the subsystem down; nothing else asserted
/// it, so the accessor read as a pointer-equality convenience.
#[test]
fn closing_through_the_guards_own_accessor_stops_the_guard() {
  let mut ring = ring(4);
  let mut ends = ring.ends();
  let (producer, _consumer) = ends.split();

  let shutdown = Shutdown::new();
  let mut guarded = shutdown.guard(producer);

  assert_eq!(guarded.try_push(1), Ok(()));

  // Nothing but the guard is in scope for this line — the capability arrives
  // through the accessor, and the returned token borrows the flag, not the
  // guard, so the guard is still usable below.
  let _stopped = guarded.shutdown().close();

  assert!(guarded.is_blocked());
  assert_eq!(guarded.try_push(2).unwrap_err().reason(), RingError::Closed);
  assert!(shutdown.is_closed(), "and it is the same flag, not a copy");
}

/// A refusing ring loses the record that hit the wall, and the batch count
/// does not say so.
///
/// `try_push_batch` moves each record into `try_push` and breaks on the first
/// error, so the refused record is consumed from the iterator and dropped
/// inside the call. The caller sees `0` accepted and an iterator that has
/// already given up one more than that — the loss is real, this is where it is
/// measured, and it belongs to `ring_core::Producer::try_push_batch` rather
/// than to the guard, which only forwards.
#[test]
fn a_batch_into_a_full_refusing_ring_destroys_the_record_that_was_refused() {
  let mut ring = refusing_ring(2);
  let mut ends = ring.ends();
  let (producer, mut consumer) = ends.split();

  let shutdown = Shutdown::new();
  let mut guarded = shutdown.guard(producer);

  let mut filling = [1, 2].into_iter();
  assert_eq!(guarded.try_push_batch(&mut filling), 2);
  assert!(guarded.is_blocked(), "the ring is full and the policy is Fail");

  let mut overflowing = [3, 4, 5].into_iter();
  assert_eq!(guarded.try_push_batch(&mut overflowing), 0, "nothing was accepted");
  assert_eq!(
    overflowing.collect::<Vec<_>>(),
    [4, 5],
    "record 3 is neither in the ring nor in the iterator — it was destroyed",
  );

  let mut recovered = Vec::new();
  shutdown.close().drain_all(&mut consumer, &mut recovered);
  assert_eq!(recovered, [1, 2]);
}

/// Under the default policy the batch count is not a count of what is stored.
///
/// `DropNewest` makes every push report `Ok`, so the loop never breaks and the
/// returned number is the length of the iterator rather than the number of
/// records the ring kept. A caller treating it as an accept-count over-reports
/// by exactly the overflow — three claimed, one stored, here.
#[test]
fn a_batch_into_a_full_drop_newest_ring_counts_records_it_did_not_keep() {
  let mut ring = ring(2);
  let mut ends = ring.ends();
  let (producer, mut consumer) = ends.split();

  let shutdown = Shutdown::new();
  let mut guarded = shutdown.guard(producer);

  assert_eq!(guarded.try_push(1), Ok(()));
  assert!(!guarded.is_blocked());

  let mut overflowing = [2, 3, 4].into_iter();
  assert_eq!(guarded.try_push_batch(&mut overflowing), 3, "three reported accepted");
  assert_eq!(overflowing.count(), 0, "and the whole iterator was consumed");

  let mut recovered = Vec::new();
  shutdown.close().drain_all(&mut consumer, &mut recovered);
  assert_eq!(recovered, [1, 2], "but only one of the three was kept");
}
