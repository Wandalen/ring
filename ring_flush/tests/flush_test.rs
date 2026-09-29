//! Feature 176's claiming test — `docs/feature/176_flush_policy.md`.
//!
//! The criterion: each of the three policies — `OnFull`, `OnBarrier`,
//! `OnBatch( n )` — fires at exactly its stated trigger **and at no other
//! point**, demonstrated by a scripted sequence against a recorded flush log.
//!
//! # Why half these tests assert an absence
//!
//! This is the only row among the acceptance table's twenty-two whose criterion
//! is negative in the temporal sense: not "the output is correct" but "no event
//! occurred outside this set." `ring_handle`'s row is negative too, but its
//! negatives are compile-time absences a compiler proves. These are runtime
//! non-events, and the only proof available is enumeration — record every
//! flush, then find the set complete.
//!
//! So each policy gets a pair: one test that it fires when it should, and one
//! that it does not fire when the *other two* policies' conditions hold. The
//! second of each pair is the one that would be skipped, because asserting an
//! absence does not feel like testing, and it is the one that catches the
//! failure the crate is most likely to have — `OnBarrier` degenerating into
//! flush-on-every-drive.
//!
//! **What that failure is actually caught by is not what this paragraph
//! originally claimed.** It said "no positive test can see it." Injecting the
//! degeneration turned four tests red, two of them positive
//! (`tests/manual/readme.md`'s F1). The real detector is any assertion that a
//! particular drive returns `FlushOutcome::NotTriggered`, wherever it appears —
//! and it appears incidentally, in tests written to measure something else, as
//! the setup line before the interesting call. A suite that only ever asserted
//! `Flushed` after the triggering call would miss it entirely, positive and
//! negative tests alike. The pairing above is still the right shape; the reason
//! given for it was the wrong one.
//!
//! # What the log is for, and what it rests on
//!
//! `FlushEntry` carries the `FlushOutcome` the same call returned, so the log
//! and the caller cannot be told different stories. What remains unguarded is
//! narrower than it was: a firing path that never called `record` would log
//! nothing, and only reading `Flusher::run` shows that every exit from it goes
//! through `record`. That is a code-reading obligation, not a test.

// Ordinary tests, compiled out under `--cfg loom`. That cfg swaps
// `ring_atomic`'s atomics for loom's instrumented ones across the whole
// family, and those panic the moment they are touched outside a
// `loom::model` closure — so without this gate a family-wide loom run dies
// here instead of reaching the models in `ring_spsc`, `ring_mpsc`,
// `ring_publish` and `ring_testkit`.
#![cfg(not(loom))]

use ring_config::RingConfig;
use ring_core::Ring;
use ring_flush::{ConfigError, FlushCause, FlushOutcome, FlushPolicy, Flusher};
use ring_tls::TlsBuffer;
use ring_types::{OverflowPolicy, RingError};

/// A ring that refuses rather than dropping, so a rejected flush is reachable.
///
/// `RingConfig::new` defaults to `OverflowPolicy::DropNewest`, under which a
/// full push returns `Ok` having silently discarded the record — which would
/// make `FlushOutcome::Rejected` unreachable and the capacity check untestable.
fn ring(slots: usize) -> Ring<u32> {
  let config = RingConfig::new(slots).unwrap().with_overflow(OverflowPolicy::Fail);
  Ring::new(&config).unwrap()
}

/// Bind `policy` over a `staged`-record buffer, with a log, onto `$ring`.
///
/// `$ends` is assigned rather than declared here because it must outlive the
/// handles it hands out, and a `let` inside the macro's own block would not.
macro_rules! flusher {
  ( $ring : ident, $ends : ident, $staged : expr, $policy : expr ) => {{
    $ends = $ring.ends();
    let (producer, consumer) = $ends.split();
    let buffer = TlsBuffer::<u32>::with_capacity($staged);
    (Flusher::new(buffer, producer, $policy).unwrap().with_log(), consumer)
  }};
}

// ---------------------------------------------------------------- OnFull

/// M1 for `OnFull`: it fires when the buffer cannot accept another record, and
/// the entry names `Full` as the cause.
#[test]
fn on_full_fires_when_the_buffer_is_full() {
  let mut r = ring(16);
  let mut ends;
  let (mut flusher, mut consumer) = flusher!(r, ends, 4, FlushPolicy::OnFull);

  for i in 0..3u32 {
    flusher.append(i).unwrap();
    assert_eq!(flusher.drive(), FlushOutcome::NotTriggered, "fired below capacity");
  }
  assert!(
    flusher.log().unwrap().is_empty(),
    "three drives below the trigger logged something"
  );

  flusher.append(3).unwrap();
  assert_eq!(flusher.drive(), FlushOutcome::Flushed { count: 4 });

  let entries = flusher.log().unwrap().entries();
  assert_eq!(entries.len(), 1, "exactly one flush");
  assert_eq!(entries[0].cause, FlushCause::Full);
  assert_eq!(entries[0].policy, FlushPolicy::OnFull);
  assert_eq!(entries[0].count(), 4);

  let mut landed = Vec::new();
  assert_eq!(consumer.try_recv_batch(&mut landed), 4);
  assert_eq!(landed, vec![0, 1, 2, 3], "staging order survived the flush");
}

/// M2 for `OnFull`: the other two policies' conditions do not fire it.
///
/// A barrier is announced repeatedly and far more records than any batch size
/// pass through — all below capacity. Zero entries.
#[test]
fn on_full_ignores_barriers_and_counts() {
  let mut r = ring(64);
  let mut ends;
  let (mut flusher, _consumer) = flusher!(r, ends, 8, FlushPolicy::OnFull);

  for i in 0..7u32 {
    flusher.append(i).unwrap();
    assert_eq!(flusher.drive_at_barrier(), FlushOutcome::NotTriggered);
  }

  assert!(
    flusher.log().unwrap().is_empty(),
    "OnFull fired for a reason that is not fullness"
  );
  assert_eq!(flusher.staged(), 7, "records left the buffer without a flush");
}

// -------------------------------------------------------------- OnBarrier

/// M1 for `OnBarrier`: it fires on an announcement, and only through
/// `drive_at_barrier`.
#[test]
fn on_barrier_fires_only_when_a_barrier_is_announced() {
  let mut r = ring(16);
  let mut ends;
  let (mut flusher, mut consumer) = flusher!(r, ends, 8, FlushPolicy::OnBarrier);

  flusher.append(10).unwrap();
  flusher.append(11).unwrap();
  assert_eq!(flusher.drive(), FlushOutcome::NotTriggered, "a plain drive fired OnBarrier");

  assert_eq!(flusher.drive_at_barrier(), FlushOutcome::Flushed { count: 2 });

  let entries = flusher.log().unwrap().entries();
  assert_eq!(entries.len(), 1);
  assert_eq!(entries[0].cause, FlushCause::Barrier);

  let mut landed = Vec::new();
  assert_eq!(consumer.try_recv_batch(&mut landed), 2);
  assert_eq!(landed, vec![10, 11]);
}

/// M2 for `OnBarrier` — **the measurement the whole design rests on.**
///
/// Fill the buffer to capacity, drive repeatedly, never announce. If
/// `OnBarrier` had degenerated into flush-on-every-drive, or had quietly
/// inherited `OnFull`'s trigger as a convenience, this test notices.
///
/// It is not the *only* test that would — measured, not assumed: injecting the
/// degeneration turns four red (`tests/manual/readme.md`'s F1). It is the only
/// one that would notice *on purpose*. The other three catch it through a
/// `NotTriggered` assertion they happen to make while setting up something else,
/// and would stop catching it the moment that setup line were rewritten. This
/// one has no other reason to exist, so it cannot be refactored away by
/// accident.
///
/// The buffer is deliberately driven past full, so `RingError::Full` on the
/// append path is also exercised — an `OnBarrier` buffer that fills before a
/// barrier arrives applies backpressure to the writer rather than publishing at
/// a point nobody chose.
#[test]
fn on_barrier_never_fires_without_an_announcement() {
  let mut r = ring(64);
  let mut ends;
  let (mut flusher, _consumer) = flusher!(r, ends, 4, FlushPolicy::OnBarrier);

  for i in 0..4u32 {
    flusher.append(i).unwrap();
    assert_eq!(flusher.drive(), FlushOutcome::NotTriggered);
  }

  assert_eq!(flusher.append(4), Err(RingError::Full), "a full buffer accepted a record");

  for _ in 0..20 {
    assert_eq!(flusher.drive(), FlushOutcome::NotTriggered);
  }

  assert!(
    flusher.log().unwrap().is_empty(),
    "OnBarrier fired without an announcement — the failure this test exists for"
  );
  assert_eq!(flusher.staged(), 4);
}

// --------------------------------------------------------------- OnBatch

/// M1 and M5 for `OnBatch( n )`: nothing at `n - 1`, exactly one at `n`.
#[test]
fn on_batch_fires_at_the_boundary_and_not_before() {
  let mut r = ring(16);
  let mut ends;
  let (mut flusher, mut consumer) = flusher!(r, ends, 8, FlushPolicy::OnBatch(3));

  flusher.append(0).unwrap();
  flusher.append(1).unwrap();
  assert_eq!(flusher.drive(), FlushOutcome::NotTriggered, "fired at n-1");
  assert!(flusher.log().unwrap().is_empty());

  flusher.append(2).unwrap();
  assert_eq!(flusher.drive(), FlushOutcome::Flushed { count: 3 });

  let entries = flusher.log().unwrap().entries();
  assert_eq!(entries.len(), 1, "exactly one entry at n");
  assert_eq!(entries[0].cause, FlushCause::Batch);
  assert_eq!(entries[0].policy, FlushPolicy::OnBatch(3));

  let mut landed = Vec::new();
  assert_eq!(consumer.try_recv_batch(&mut landed), 3);
  assert_eq!(landed, vec![0, 1, 2]);
}

/// M2 for `OnBatch`: a barrier does not fire it, however often announced.
///
/// **Fullness is deliberately not part of this test, and the reason is a
/// finding rather than an omission.** An `OnBatch( n )` policy cannot observe a
/// full buffer: records staged since the last flush *is* the buffer's
/// occupancy, and validation caps `n` at capacity — so the batch trigger fires
/// no later than the buffer fills, and after it fires the buffer is empty. The
/// "`OnBatch` also honours `OnFull`" defect the flush log's `cause` field is
/// said to exist for is, for this policy, unreachable.
///
/// That was established by injecting the defect and watching all 23 tests pass
/// (→ [`tests/manual/readme.md`]'s F3). A test written against the original
/// premise would have asserted an absence that no implementation could
/// violate — green forever, and worth nothing.
#[test]
fn on_batch_ignores_barriers() {
  let mut r = ring(64);
  let mut ends;
  let (mut flusher, _consumer) = flusher!(r, ends, 8, FlushPolicy::OnBatch(4));

  for i in 0..3u32 {
    flusher.append(i).unwrap();
    assert_eq!(flusher.drive_at_barrier(), FlushOutcome::NotTriggered);
    assert_eq!(flusher.drive(), FlushOutcome::NotTriggered);
  }

  assert!(flusher.log().unwrap().is_empty(), "OnBatch fired for a barrier");
  assert_eq!(flusher.staged(), 3);
}

/// The batch trigger fires no later than the buffer fills — the property that
/// makes the fullness case above unreachable, asserted directly rather than
/// left as reasoning.
///
/// For every legal batch size against a fixed capacity, drive after each
/// append: the flush must arrive at exactly `n` records, and the buffer must
/// never have been full at any point where the trigger had not already held.
#[test]
fn the_batch_trigger_arrives_no_later_than_the_buffer_fills() {
  const CAPACITY: usize = 6;

  for n in 1..=CAPACITY {
    let mut r = ring(64);
    let mut ends;
    let (mut flusher, _consumer) = flusher!(r, ends, CAPACITY, FlushPolicy::OnBatch(n));

    for appended in 1..=n {
      assert!(
        flusher.staged() < CAPACITY,
        "n={n}: the buffer filled before the batch trigger held"
      );
      flusher.append(appended as u32).unwrap();

      let outcome = flusher.drive();
      if appended < n {
        assert_eq!(outcome, FlushOutcome::NotTriggered, "n={n} fired at {appended}");
      } else {
        assert_eq!(outcome, FlushOutcome::Flushed { count: n }, "n={n} did not fire at n");
      }
    }

    assert_eq!(flusher.staged(), 0, "n={n}: the flush left records staged");
  }
}

/// Overshoot — P4 of [`docs/state_machine/002`].
///
/// The trigger is `staged >= n`, not `staged == n`, and the two differ whenever
/// the caller appends several records between drives. A batch of `2n` publishes
/// as **one** flush of `2n` records, not two of `n`.
///
/// This is also `docs/integration/001`'s E3, from the other side: the claim
/// width `ring_tls` sees is whatever accumulated, so an `OnBatch( n )` policy
/// does not bound it. `n` sets a floor on batch size, never a ceiling.
#[test]
fn an_overshooting_batch_publishes_everything_staged_in_one_claim() {
  let mut r = ring(32);
  let mut ends;
  let (mut flusher, mut consumer) = flusher!(r, ends, 16, FlushPolicy::OnBatch(4));

  // Eight appends with no drive between them — twice the batch size.
  for i in 0..8u32 {
    flusher.append(i).unwrap();
  }
  assert_eq!(flusher.staged(), 8, "an append fired without a drive");

  assert_eq!(
    flusher.drive(),
    FlushOutcome::Flushed { count: 8 },
    "the flush was capped at n"
  );

  let entries = flusher.log().unwrap().entries();
  assert_eq!(entries.len(), 1, "2n records published as more than one batch");
  assert_eq!(entries[0].count(), 8);
  assert_eq!(entries[0].cause, FlushCause::Batch);

  let mut landed = Vec::new();
  assert_eq!(consumer.try_recv_batch(&mut landed), 8);
  assert_eq!(landed, vec![0, 1, 2, 3, 4, 5, 6, 7]);
}

/// A rejected flush leaves the policy armed — P6 of [`docs/state_machine/002`].
///
/// The failure this excludes is a driver that treats "I tried" as "I fired":
/// clearing its trigger on a `Rejected` outcome, so the records stay staged
/// and nothing ever retries them. Structurally it cannot happen here — the
/// trigger is a pure function of the buffer, and a rejection leaves the buffer
/// untouched — but that is an argument about the implementation, and P6 asks
/// for a measurement.
///
/// Distinct from [`a_refused_final_drain_keeps_the_records`], which retries
/// `drain_final`. `drain_final` ignores the policy, so it cannot show the
/// policy is still armed. This drives.
#[test]
fn a_rejected_flush_leaves_the_policy_armed() {
  // A 4-slot ring with 3 slots already occupied by another writer, so the
  // refusal comes from free capacity rather than from total capacity — which
  // is what makes it recoverable without rebuilding anything.
  let mut r = ring(4);
  let mut ends = r.ends();
  let (mut producer, mut consumer) = ends.split();
  for i in 100..103u32 {
    producer.try_push(i).unwrap();
  }

  let mut buffer = TlsBuffer::with_capacity(8);
  for i in 0..4u32 {
    buffer.push(i).unwrap();
  }
  let mut flusher = Flusher::new(buffer, producer, FlushPolicy::OnBatch(4)).unwrap().with_log();

  assert_eq!(
    flusher.drive(),
    FlushOutcome::Rejected { staged: 4 },
    "1 free slot took 4 records"
  );
  assert_eq!(flusher.staged(), 4);

  // Same trigger, same refusal — the policy did not disarm itself on a rejection.
  assert_eq!(flusher.drive(), FlushOutcome::Rejected { staged: 4 });
  assert_eq!(flusher.staged(), 4);

  // Nothing of ours reached the ring; only the other writer's records are there.
  let mut landed = Vec::new();
  assert_eq!(consumer.try_recv_batch(&mut landed), 3);
  assert_eq!(landed, vec![100, 101, 102], "a rejected flush published anyway");

  // Room now exists, and the still-armed policy fires on the very next drive
  // with the same records, still as one batch.
  assert_eq!(
    flusher.drive(),
    FlushOutcome::Flushed { count: 4 },
    "the retry lost the trigger"
  );
  assert_eq!(flusher.staged(), 0);

  let mut arrived = Vec::new();
  assert_eq!(consumer.try_recv_batch(&mut arrived), 4);
  assert_eq!(arrived, vec![0, 1, 2, 3], "the records changed across the retry");

  // Two refusals and one success, all three recorded and distinguishable.
  let causes: Vec<_> = flusher.log().unwrap().entries().iter().map(|e| e.outcome).collect();
  assert_eq!(
    causes,
    vec![
      FlushOutcome::Rejected { staged: 4 },
      FlushOutcome::Rejected { staged: 4 },
      FlushOutcome::Flushed { count: 4 },
    ]
  );
}

/// Dropping a driver with records staged publishes **nothing** — Z5 of
/// [`docs/integration/002`], and the crate's one documented way to lose data.
///
/// Z5 asked for a compile-time check that no `Drop` impl exists. That is the
/// wrong instrument twice over: `mem::needs_drop` is true for a `Flusher`
/// regardless, because the optional log owns a `Vec`, and a `Drop` impl that
/// existed but published nothing would be harmless. What matters is the
/// behaviour, so that is what this asserts.
///
/// The result is a *hazard*, not a guarantee, and it is deliberate.
/// Rust has no linear types, so nothing can force the final drain. A `Drop`
/// impl that flushed would publish at a point nobody chose — the exact failure
/// [`docs/invariant/002`] exists to exclude — and would need a producer that
/// might refuse, with no caller left to hear about it.
#[test]
fn dropping_a_driver_with_records_staged_publishes_nothing() {
  let mut r = ring(16);
  let mut ends = r.ends();
  let (producer, mut consumer) = ends.split();

  {
    let mut buffer = TlsBuffer::with_capacity(8);
    for i in 0..5u32 {
      buffer.push(i).unwrap();
    }
    let flusher = Flusher::new(buffer, producer, FlushPolicy::OnBarrier).unwrap();
    assert_eq!(flusher.staged(), 5);
  } // dropped here, with five records staged and no final drain

  assert_eq!(
    consumer.len(),
    0,
    "a drop published — the publication point is not the caller's"
  );
  let mut landed = Vec::new();
  assert_eq!(consumer.try_recv_batch(&mut landed), 0);
}

/// Batches are consecutive rather than cumulative — an `OnBatch( 2 )` policy
/// fires at 2, 4, 6, not at 2 and then every append after.
///
/// This test was named `the_batch_counter_restarts_after_each_flush` and there
/// is no counter to restart. What resets is the staging buffer, which the flush
/// empties; `OnBatch( n )` reads `buffer.len() >= n`, so consecutiveness is a
/// consequence of the flush having drained rather than of a separate variable
/// being zeroed. The behaviour asserted below did not change — only the reason
/// it holds, which is why the old name would have survived the counter's
/// deletion unnoticed.
#[test]
fn batches_are_consecutive_not_cumulative() {
  let mut r = ring(16);
  let mut ends;
  let (mut flusher, mut consumer) = flusher!(r, ends, 8, FlushPolicy::OnBatch(2));

  for round in 0..3u32 {
    flusher.append(round * 2).unwrap();
    assert_eq!(flusher.drive(), FlushOutcome::NotTriggered, "round {round} fired at 1");
    flusher.append(round * 2 + 1).unwrap();
    assert_eq!(flusher.drive(), FlushOutcome::Flushed { count: 2 });
  }

  let entries = flusher.log().unwrap().entries();
  assert_eq!(entries.len(), 3, "three batches, three entries");
  assert!(entries.iter().all(|e| e.cause == FlushCause::Batch && e.count() == 2));

  let mut landed = Vec::new();
  assert_eq!(consumer.try_recv_batch(&mut landed), 6);
  assert_eq!(landed, vec![0, 1, 2, 3, 4, 5]);
}

// ------------------------------------------------------- outcomes and log

/// M3 — every entry's cause matches the configured policy's trigger.
///
/// Asserted across all three policies in one place, because the failure this
/// catches is a flush firing for the wrong reason, which is only visible when
/// the causes are compared against the policies that produced them.
#[test]
fn every_entry_names_its_own_policys_trigger() {
  let cases = [
    (FlushPolicy::OnFull, FlushCause::Full),
    (FlushPolicy::OnBarrier, FlushCause::Barrier),
    (FlushPolicy::OnBatch(2), FlushCause::Batch),
  ];

  for (policy, expected) in cases {
    let mut r = ring(16);
    let mut ends;
    let (mut flusher, _consumer) = flusher!(r, ends, 2, policy);

    flusher.append(0).unwrap();
    flusher.append(1).unwrap();
    let outcome = flusher.drive_at_barrier();
    assert_eq!(outcome, FlushOutcome::Flushed { count: 2 }, "{policy:?} did not fire");

    let entries = flusher.log().unwrap().entries();
    assert_eq!(entries.len(), 1, "{policy:?}");
    assert_eq!(entries[0].cause, expected, "{policy:?} fired for the wrong reason");
  }
}

/// M4 — the log entry and the returned outcome agree on every drive call.
///
/// Guaranteed by construction: the entry carries the outcome. The test is here
/// because "by construction" is a claim about the code as it stands, and a
/// later edit that re-derived the entry from anything else would pass every
/// other test in this file.
#[test]
fn the_log_agrees_with_every_outcome_it_recorded() {
  let mut r = ring(4);
  let mut ends;
  let (mut flusher, mut consumer) = flusher!(r, ends, 8, FlushPolicy::OnBarrier);

  let mut fired = Vec::new();

  // Empty trigger, a flush that lands, and a flush the ring refuses.
  fired.push(flusher.drive_at_barrier());

  for i in 0..4u32 {
    flusher.append(i).unwrap();
  }
  fired.push(flusher.drive_at_barrier());

  for i in 4..8u32 {
    flusher.append(i).unwrap();
  }
  fired.push(flusher.drive_at_barrier());

  assert_eq!(
    fired,
    vec![
      FlushOutcome::TriggeredEmpty,
      FlushOutcome::Flushed { count: 4 },
      FlushOutcome::Rejected { staged: 4 },
    ],
    "the three non-NotTriggered outcomes were not all reached"
  );

  let recorded: Vec<_> = flusher.log().unwrap().entries().iter().map(|e| e.outcome).collect();
  assert_eq!(recorded, fired, "log and outcomes disagree");

  // The rejected batch is still staged, and lands once the ring has room.
  assert_eq!(flusher.staged(), 4, "a rejected flush lost records");
  let mut landed = Vec::new();
  assert_eq!(consumer.try_recv_batch(&mut landed), 4);
  assert_eq!(landed, vec![0, 1, 2, 3]);

  assert_eq!(flusher.drive_at_barrier(), FlushOutcome::Flushed { count: 4 });
  let mut second = Vec::new();
  assert_eq!(consumer.try_recv_batch(&mut second), 4);
  assert_eq!(second, vec![4, 5, 6, 7], "the retry published what the refusal held");
}

/// A `NotTriggered` call leaves no entry, which is what makes `is_empty()` the
/// direct form of "and at no other point".
#[test]
fn a_call_that_did_not_fire_is_not_recorded() {
  let mut r = ring(16);
  let mut ends;
  let (mut flusher, _consumer) = flusher!(r, ends, 8, FlushPolicy::OnBarrier);

  for _ in 0..50 {
    assert_eq!(flusher.drive(), FlushOutcome::NotTriggered);
  }

  assert_eq!(flusher.log().unwrap().len(), 0);
}

/// A trigger that finds nothing staged is recorded, and is not the same event
/// as a trigger that never fired.
#[test]
fn an_empty_trigger_is_recorded_and_is_not_a_non_trigger() {
  let mut r = ring(16);
  let mut ends;
  let (mut flusher, _consumer) = flusher!(r, ends, 8, FlushPolicy::OnBarrier);

  assert_eq!(flusher.drive(), FlushOutcome::NotTriggered);
  assert_eq!(flusher.drive_at_barrier(), FlushOutcome::TriggeredEmpty);

  let entries = flusher.log().unwrap().entries();
  assert_eq!(entries.len(), 1, "the non-trigger and the empty trigger were conflated");
  assert_eq!(entries[0].outcome, FlushOutcome::TriggeredEmpty);
  assert_eq!(entries[0].count(), 0);
}

/// The log can be reset between scenarios, through either route.
#[test]
fn the_log_can_be_cleared_between_scenarios() {
  let mut r = ring(16);
  let mut ends;
  let (mut flusher, _consumer) = flusher!(r, ends, 8, FlushPolicy::OnBarrier);

  flusher.append(1).unwrap();
  assert_eq!(flusher.drive_at_barrier(), FlushOutcome::Flushed { count: 1 });
  assert_eq!(flusher.log().unwrap().len(), 1);
  assert!(
    !flusher.log().unwrap().is_empty(),
    "the log was already empty, so clearing proves nothing"
  );

  flusher.clear_log();
  assert!(flusher.log().unwrap().is_empty());

  let mut standalone = ring_flush::FlushLog::new();
  assert!(standalone.is_empty());
  standalone.clear();
  assert_eq!(standalone, ring_flush::FlushLog::default());
}

/// Without `with_log`, no log exists — the default, and the reason the crate
/// needs no cargo feature to keep a release build free of one.
#[test]
fn a_driver_has_no_log_unless_asked() {
  let mut r = ring(16);
  let mut ends = r.ends();
  let (producer, _consumer) = ends.split();
  let buffer = TlsBuffer::<u32>::with_capacity(4);
  let mut flusher = Flusher::new(buffer, producer, FlushPolicy::OnBarrier).unwrap();

  flusher.append(1).unwrap();
  assert_eq!(flusher.drive_at_barrier(), FlushOutcome::Flushed { count: 1 });
  assert!(flusher.log().is_none(), "a log appeared without being asked for");

  // Clearing a log that does not exist is a no-op, not a panic.
  flusher.clear_log();
  assert!(flusher.log().is_none());
}

// ------------------------------------------------------------ validation

/// Both checkable validation rules reject at binding time rather than degrading
/// into a different working policy.
#[test]
fn an_unusable_batch_size_is_refused_at_binding() {
  let mut r = ring(16);
  let mut ends = r.ends();
  let (producer, _consumer) = ends.split();

  let zero = Flusher::new(TlsBuffer::<u32>::with_capacity(4), producer, FlushPolicy::OnBatch(0));
  assert_eq!(zero.unwrap_err(), ConfigError::ZeroBatch);

  let mut second = ring(16);
  let mut ends = second.ends();
  let (producer, _consumer) = ends.split();

  let oversize = Flusher::new(TlsBuffer::<u32>::with_capacity(4), producer, FlushPolicy::OnBatch(5));
  assert_eq!(
    oversize.unwrap_err(),
    ConfigError::BatchExceedsCapacity {
      requested: 5,
      capacity: 4
    }
  );
}

/// A batch size exactly at capacity is legal — it is the largest size that can
/// still fire, and rejecting it would be an off-by-one that turns a valid
/// configuration into a refusal.
#[test]
fn a_batch_size_equal_to_capacity_is_accepted() {
  let mut r = ring(16);
  let mut ends;
  let (mut flusher, _consumer) = flusher!(r, ends, 4, FlushPolicy::OnBatch(4));

  for i in 0..4u32 {
    flusher.append(i).unwrap();
  }
  assert_eq!(flusher.drive(), FlushOutcome::Flushed { count: 4 });
}

/// Validation applies to `OnBatch` alone — the other two carry no parameter to
/// be wrong about, and a buffer of any capacity binds them.
#[test]
fn the_parameterless_policies_need_no_validation() {
  for policy in [FlushPolicy::OnFull, FlushPolicy::OnBarrier] {
    let mut r = ring(16);
    let mut ends = r.ends();
    let (producer, _consumer) = ends.split();
    let bound = Flusher::new(TlsBuffer::<u32>::with_capacity(1), producer, policy);
    assert!(bound.is_ok(), "{policy:?} was refused");
    assert_eq!(bound.unwrap().policy(), policy);
  }
}

// --------------------------------------------------------- the final drain

/// `drain_final` ignores the policy — the legitimate case behind the
/// `flush_now()` this crate refuses to expose.
#[test]
fn the_final_drain_publishes_whatever_the_policy_would_have_held() {
  let mut r = ring(16);
  let mut ends;
  let (mut flusher, mut consumer) = flusher!(r, ends, 8, FlushPolicy::OnBarrier);

  flusher.append(7).unwrap();
  flusher.append(8).unwrap();
  assert_eq!(
    flusher.drive(),
    FlushOutcome::NotTriggered,
    "the policy would have held these"
  );

  assert_eq!(flusher.drain_final(), FlushOutcome::Flushed { count: 2 });

  let entries = flusher.log().unwrap().entries();
  assert_eq!(entries.len(), 1);
  assert_eq!(
    entries[0].cause,
    FlushCause::Shutdown,
    "the final drain hid behind a policy cause"
  );
  assert_eq!(
    entries[0].policy,
    FlushPolicy::OnBarrier,
    "the entry lost which policy was bound"
  );

  let mut landed = Vec::new();
  assert_eq!(consumer.try_recv_batch(&mut landed), 2);
  assert_eq!(landed, vec![7, 8]);
}

/// A refused final drain keeps the records, and says how many — the outcome the
/// caller is obliged to retry and that nothing here will retry for it.
#[test]
fn a_refused_final_drain_keeps_the_records() {
  let mut r = ring(2);
  let mut ends;
  let (mut flusher, mut consumer) = flusher!(r, ends, 8, FlushPolicy::OnBarrier);

  for i in 0..4u32 {
    flusher.append(i).unwrap();
  }
  assert_eq!(flusher.drain_final(), FlushOutcome::Rejected { staged: 4 });
  assert_eq!(flusher.staged(), 4, "a rejected final drain discarded the records");

  // Nothing reached the ring, so there is nothing for the consumer to take.
  let mut landed = Vec::new();
  assert_eq!(consumer.try_recv_batch(&mut landed), 0);

  // And the retry the caller owes still works — the records were never claimed.
  assert_eq!(flusher.drain_final(), FlushOutcome::Rejected { staged: 4 });
  assert_eq!(flusher.staged(), 4);
}

/// What happens to an append after `drain_final` — Pending 5 of
/// [`docs/decisions/readme.md`], pinned rather than left to discovery.
///
/// That instance called L5 → L3 "unspecified" and named that the one answer
/// definitely wrong, because callers will find the behaviour by experiment and
/// depend on whatever they find. So this is the experiment, written down.
///
/// **The flusher is reusable.** `drain_final` empties the buffer and publishes;
/// it does not poison, close, or consume the driver. Appends after it are
/// ordinary appends, the policy is still bound, and the next trigger fires
/// normally. `drain_final` is a forced flush with a distinguishing cause, not a
/// terminal operation.
///
/// This is a *recorded default*, not a ruling. Pending 4's consuming signature
/// would make L5 unrepresentable and is still open; if it is ever taken, this
/// test is the thing that has to change, which is the point of having it.
#[test]
fn a_driver_still_works_after_a_final_drain() {
  let mut r = ring(16);
  let mut ends;
  let (mut flusher, mut consumer) = flusher!(r, ends, 8, FlushPolicy::OnBatch(2));

  flusher.append(1).unwrap();
  assert_eq!(flusher.drain_final(), FlushOutcome::Flushed { count: 1 });
  assert_eq!(flusher.staged(), 0);

  // The append is accepted, not refused, and does not itself publish.
  flusher.append(2).unwrap();
  assert_eq!(flusher.staged(), 1);
  assert_eq!(flusher.drive(), FlushOutcome::NotTriggered, "the policy was lost");

  // And the bound policy still fires at its own trigger.
  flusher.append(3).unwrap();
  assert_eq!(flusher.drive(), FlushOutcome::Flushed { count: 2 });

  let causes: Vec<_> = flusher.log().unwrap().entries().iter().map(|e| e.cause).collect();
  assert_eq!(causes, vec![FlushCause::Shutdown, FlushCause::Batch]);

  let mut landed = Vec::new();
  assert_eq!(consumer.try_recv_batch(&mut landed), 3);
  assert_eq!(landed, vec![1, 2, 3]);
}

/// A second `drain_final` on an already-drained driver is a no-op that says so.
///
/// The companion to the test above: `drain_final` is once-only "by convention"
/// (Pending 4), so what a redundant second call does is exactly the kind of
/// thing a caller retrying under uncertainty will hit. It reports
/// `TriggeredEmpty` — not `Flushed { count : 0 }`, which would claim a
/// publication that did not happen, and not `NotTriggered`, which is reserved
/// for a policy that declined.
#[test]
fn a_second_final_drain_is_an_empty_trigger() {
  let mut r = ring(16);
  let mut ends;
  let (mut flusher, _consumer) = flusher!(r, ends, 8, FlushPolicy::OnBarrier);

  flusher.append(1).unwrap();
  assert_eq!(flusher.drain_final(), FlushOutcome::Flushed { count: 1 });
  assert_eq!(flusher.drain_final(), FlushOutcome::TriggeredEmpty);
  assert_eq!(flusher.drain_final(), FlushOutcome::TriggeredEmpty);

  let entries = flusher.log().unwrap().entries();
  assert_eq!(entries.len(), 3, "a redundant final drain went unrecorded");
  assert!(entries.iter().all(|e| e.cause == FlushCause::Shutdown));
}

// ------------------------------------------------------------ properties

/// The policy is fixed for the driver's lifetime. There is no `set_policy`, so
/// this asserts that driving does not change it either.
#[test]
fn the_bound_policy_never_changes() {
  let mut r = ring(16);
  let mut ends;
  let (mut flusher, _consumer) = flusher!(r, ends, 4, FlushPolicy::OnBatch(2));

  assert_eq!(flusher.policy(), FlushPolicy::OnBatch(2));
  for i in 0..4u32 {
    flusher.append(i).unwrap();
    let _ = flusher.drive();
    assert_eq!(flusher.policy(), FlushPolicy::OnBatch(2));
  }
}

/// `staged()` tracks the buffer and drops to zero across a flush.
#[test]
fn staged_follows_the_buffer() {
  let mut r = ring(16);
  let mut ends;
  let (mut flusher, _consumer) = flusher!(r, ends, 4, FlushPolicy::OnBarrier);

  assert_eq!(flusher.staged(), 0);
  flusher.append(1).unwrap();
  assert_eq!(flusher.staged(), 1);
  flusher.append(2).unwrap();
  assert_eq!(flusher.staged(), 2);
  assert_eq!(flusher.drive_at_barrier(), FlushOutcome::Flushed { count: 2 });
  assert_eq!(flusher.staged(), 0);
}

/// Every exported type is `Debug`, so a panic message can name what it held.
#[test]
fn every_type_can_be_printed() {
  assert!(format!("{:?}", FlushPolicy::OnBatch(4)).contains("OnBatch"));
  assert!(format!("{:?}", FlushCause::Shutdown).contains("Shutdown"));
  assert!(format!("{:?}", FlushOutcome::Rejected { staged: 3 }).contains("Rejected"));
  assert!(format!("{:?}", ConfigError::ZeroBatch).contains("ZeroBatch"));
  assert!(format!("{:?}", ring_flush::FlushLog::new()).contains("FlushLog"));

  let mut r = ring(16);
  let mut ends;
  let (flusher, _consumer) = flusher!(r, ends, 4, FlushPolicy::OnFull);
  assert!(format!("{flusher:?}").contains("Flusher"));

  let entry = ring_flush::FlushEntry {
    policy: FlushPolicy::OnFull,
    cause: FlushCause::Full,
    outcome: FlushOutcome::Flushed { count: 1 },
  };
  assert!(format!("{entry:?}").contains("FlushEntry"));
  assert_eq!(entry.count(), 1);
}

/// `FlushEntry::count()` reads 0 from every outcome that is not `Flushed`.
///
/// Closes a decay gap rather than reproducing a bug: `Flushed` and
/// `TriggeredEmpty` were already exercised through `count()` elsewhere in this
/// file (`:114`, `:602`), but `Rejected` and `NotTriggered` were not — so a
/// regression narrowing the exhaustive match added by
/// `Fix(flush_entry_count_catchall_not_exhaustive)` back down to a `_ => 0`
/// covering fewer named variants would have had nothing here to catch it for
/// those two. `NotTriggered` cannot arise from `Flusher::run` (nothing calls
/// `record` with it), so it is built directly, as `every_type_can_be_printed`
/// above already does for `Flushed`.
#[test]
fn count_is_zero_for_every_non_flushed_outcome() {
  for outcome in [
    FlushOutcome::NotTriggered,
    FlushOutcome::TriggeredEmpty,
    FlushOutcome::Rejected { staged: 4 },
  ] {
    let entry = ring_flush::FlushEntry {
      policy: FlushPolicy::OnFull,
      cause: FlushCause::Full,
      outcome,
    };
    assert_eq!(entry.count(), 0, "{outcome:?} should report zero records moved");
  }
}

/// `ConfigError` renders for a human and chains as an error.
///
/// `Debug` was enough while this crate's only consumer was its own suite, which
/// is why `every_type_can_be_printed` above checks `Debug` and stops there.
/// `ring_bench` is the first crate to consume all three of the Contract's error
/// types together, and it could not: `ring_types::RingError` and
/// `ring_factory::BuildError` both implement `Display` and `Error`, and this one
/// implemented neither — so a `RunError` relaying all three could not have a
/// single `Display` arm per variant. Caught by the compiler on that crate's
/// first build, not by anything here.
///
/// The message names the *consequence* rather than restating the variant. A
/// binding refusal is read by whoever configured the policy, and "OnBatch( 5 )
/// never fires in a 4-record buffer" tells them what to change; "batch exceeds
/// capacity" makes them work it out.
#[test]
fn a_binding_refusal_renders_and_chains() {
  assert_eq!(ConfigError::ZeroBatch.to_string(), "OnBatch( 0 ) fires on every append",);
  assert_eq!(
    ConfigError::BatchExceedsCapacity {
      requested: 5,
      capacity: 4
    }
    .to_string(),
    "OnBatch( 5 ) never fires in a 4-record buffer",
  );

  // The half `Display` alone does not give: folding into an error trait object,
  // which is what a consumer holding refusals from three crates has to do.
  let boxed: Box<dyn core::error::Error> = Box::new(ConfigError::ZeroBatch);
  assert!(boxed.to_string().contains("every append"));
}

/// A policy is consulted by value on the append path, so it must stay `Copy`
/// and small. A variant carrying a non-`Copy` payload breaks that silently.
#[test]
fn the_policy_is_a_value() {
  fn assert_copy<T: Copy>() {}
  assert_copy::<FlushPolicy>();
  assert_copy::<FlushCause>();
  assert_copy::<FlushOutcome>();
  assert_copy::<ring_flush::FlushEntry>();

  let policy = FlushPolicy::OnBatch(64);
  let taken = policy;
  assert_eq!(policy, taken, "the original was moved rather than copied");

  assert_eq!(
    core::mem::size_of::<FlushPolicy>(),
    2 * core::mem::size_of::<usize>(),
    "a discriminant plus one usize — a payload was added"
  );

  // C4 of `docs/non_functional_requirement/002` — nothing runs when a policy
  // goes out of scope. The instance asked for a trybuild case; this is the
  // same guarantee for one line and no build dependency, and it is strictly
  // stronger: it also rejects a variant carrying a payload that *itself* has a
  // `Drop` impl, which a check for `impl Drop for FlushPolicy` would miss.
  assert!(!core::mem::needs_drop::<FlushPolicy>(), "a policy acquired drop glue");
  assert!(!core::mem::needs_drop::<FlushCause>());
  assert!(!core::mem::needs_drop::<FlushOutcome>());
  assert!(!core::mem::needs_drop::<ring_flush::FlushEntry>());
}

/// The append path touches no ring and publishes nothing, however many records
/// pass through it. Driven, not self-firing, asserted rather than assumed.
#[test]
fn appending_never_publishes() {
  let mut r = ring(64);
  let mut ends;
  let (mut flusher, mut consumer) = flusher!(r, ends, 32, FlushPolicy::OnFull);

  for i in 0..32u32 {
    flusher.append(i).unwrap();
  }

  assert_eq!(consumer.len(), 0, "an append reached the ring");
  assert_eq!(flusher.staged(), 32);
  assert!(flusher.log().unwrap().is_empty(), "an append flushed");

  let mut landed = Vec::new();
  assert_eq!(consumer.try_recv_batch(&mut landed), 0);
}

/// A driver may cross threads only by being moved, never by being shared.
///
/// [`docs/pitfall/001`]'s F4 as a compile-time fact rather than a warning. The
/// pitfall's concern was that a barrier signal arriving on another thread would
/// force a cross-thread seal, needing a different mechanism than the one
/// `ring_tls` provides. It cannot arise: `Flusher` owns its buffer and its
/// producer, and every drive method takes `&mut self`, so another thread can
/// only drive a flusher it has been given outright. The announcement has to
/// reach the owning thread as data.
///
/// `Send` and `!Sync` together are exactly that statement — movable, not
/// shareable. Only the first is asserted below, because a negative trait bound
/// is not expressible on stable Rust. The second was **measured** rather than
/// assumed: adding `fn assert_sync< T : Sync >(){}` here and calling it with
/// this type fails to compile, and the compiler names the reason —
///
/// ```text
/// note: required because it appears within the type `Producer<'_, u32>`
/// note: required because it appears within the type `Flusher<'_, u32>`
/// ```
///
/// — `ring_core::ProducerInner` is not `Sync`, so neither is anything holding
/// one. That is inherited rather than chosen here, which is worth knowing: if
/// `ring_core` ever made `Producer` `Sync`, this crate would silently acquire
/// the shareability F4 depends on being absent, and nothing in this file would
/// notice. The assertion below would still pass.
#[test]
fn a_driver_is_movable_between_threads_and_never_shared() {
  fn assert_send<T: Send>() {}
  assert_send::<ring_flush::Flusher<'_, u32>>();
}
