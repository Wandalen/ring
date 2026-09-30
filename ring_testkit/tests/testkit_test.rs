//! `docs/feature/188_loom_and_testkit_helpers.md` — the scripted-fixture half.
//!
//! Feature 188 asks for two things: a model checker exploring interleavings
//! exhaustively, and fixtures driving a ring through a scripted sequence of
//! claims and drains. This file covers the second and asserts the first is
//! reachable; the model itself is `tests/exhaustive_test.rs`, which only
//! compiles under `RUSTFLAGS="--cfg loom"`.
//!
//! ```text
//! cargo nextest run -p ring_testkit
//! RUSTFLAGS="--cfg loom" cargo test -p ring_testkit --test exhaustive_test
//! ```
//!
//! # The one worth reading
//!
//! [`neither_the_count_nor_the_delivered_records_sees_a_drop_alone`] is the
//! crate's reason to exist, and it pins a result that contradicted the
//! prediction it was written to confirm — see `tests/manual/readme.md` M1.

// The inverse of `exhaustive_test.rs`'s `#![ cfg( loom ) ]`. `--cfg loom` swaps
// `ring_atomic`'s atomics for loom's instrumented ones across the whole graph,
// and those panic the moment they are touched outside a `loom::model` closure —
// which every `Ring` here does. The `--test exhaustive_test` narrowing in the
// invocation above sidesteps that for this crate alone; this gate is what lets
// a whole-crate or whole-workspace loom run reach the model instead of dying
// here first.
#![cfg(not(loom))]

use ring_config::RingConfig;
use ring_core::Ring;
use ring_testkit::{
    Anomaly,
    Outcome,
    Script,
    Step,
    audit_received,
    audit_received_unordered,
    leak,
};
use ring_types::OverflowPolicy;

/// A ring of `slots`, refusing rather than dropping when full.
fn failing_ring(slots: usize) -> Ring<u32> {
    let config =
        RingConfig::new(slots).expect("a power of two").with_overflow(OverflowPolicy::Fail);
    Ring::new(&config).expect("Fail is an accepted policy")
}

/// A ring of `slots`, discarding the incoming record when full.
fn dropping_ring(slots: usize) -> Ring<u32> {
    let config =
        RingConfig::new(slots).expect("a power of two").with_overflow(OverflowPolicy::DropNewest);
    Ring::new(&config).expect("DropNewest is an accepted policy")
}

// ── the acceptance clauses ────────────────────────────────────────────────

/// **Clause 1 — a scripted sequence reproduces identical outcomes every run.**
///
/// Two fresh rings, one script, and equality of the whole `Outcome` rather than
/// of a chosen field: a comparison that looked only at `received` would pass
/// for a script whose refusal counts drifted, which is the failure the next
/// test shows is easy to have.
#[test]
fn one_script_run_twice_produces_equal_outcomes() {
    let script = Script::new(4)
        .then(Step::PushMany(6))
        .then(Step::RecvMany(3))
        .then(Step::StageMany(5))
        .then(Step::Flush)
        .then(Step::DrainAll);

    let first = script.run(&mut failing_ring(4));
    let again = script.run(&mut failing_ring(4));

    assert_eq!(first, again, "the same script on equivalent rings");
    assert_eq!(first.audit(), Ok(()));
}

/// **Clause 1, the part a single re-run cannot show.** Ten runs, all equal.
///
/// One repeat can agree by coincidence if the outcome depends on something that
/// happens to be stable within a process — an allocator address, a `HashMap`
/// seed. Ten does not prove determinism either, but it is the cheap half of the
/// claim; the exhaustive half is loom's.
#[test]
fn ten_runs_of_one_script_all_agree() {
    let script = Script::new(3).then(Step::StageMany(3)).then(Step::Flush).then(Step::DrainAll);

    let reference = script.run(&mut failing_ring(8));
    for run in 1..=10 {
        assert_eq!(script.run(&mut failing_ring(8)), reference, "run {run}");
    }
}

/// **Clause 2 — the fixture decides something a count cannot.**
///
/// The measurement the crate is built on, and it contradicted the prediction:
/// the two policies were expected to produce equal counts and different
/// delivered records. They produce the opposite. Both rings deliver exactly
/// `[0, 1, 2, 3]`; the counts differ, and differ in the direction that reads as
/// *more work done* rather than *records destroyed*.
///
/// `vanished` is the reading that is neither — accepted, minus delivered, minus
/// still held.
#[test]
fn neither_the_count_nor_the_delivered_records_sees_a_drop_alone() {
    let script = Script::new(4).then(Step::PushMany(8)).then(Step::DrainAll);

    let refusing = script.run(&mut failing_ring(4));
    let dropping = script.run(&mut dropping_ring(4));

    assert_eq!(refusing.received, dropping.received, "the delivered records are identical");
    assert_eq!(refusing.received, [0, 1, 2, 3]);

    assert_eq!(refusing.accepted, 4);
    assert_eq!(dropping.accepted, 8, "the dropping ring accepts strictly more");
    assert_eq!(refusing.refused_full, 4);
    assert_eq!(dropping.refused_full, 0, "and refuses nothing");

    assert_eq!(refusing.vanished(), 0);
    assert_eq!(dropping.vanished(), 4, "four records accepted and nowhere");

    assert_eq!(refusing.audit(), Ok(()), "vanishing is not an accounting failure");
    assert_eq!(dropping.audit(), Ok(()), "which is exactly why it needs its own reading");
}

/// A ring that never fills has nothing to vanish.
#[test]
fn a_ring_with_room_vanishes_nothing_under_either_policy() {
    let script = Script::new(2).then(Step::PushMany(4)).then(Step::DrainAll);

    assert_eq!(script.run(&mut failing_ring(8)).vanished(), 0);
    assert_eq!(script.run(&mut dropping_ring(8)).vanished(), 0);
}

// ── accounting ────────────────────────────────────────────────────────────

/// Every minted record is accepted, refused, or still staged — never nowhere.
///
/// Checked across four shapes rather than one, because the interesting failure
/// is a step that mints without accounting, and only the step that does it
/// would show it.
#[test]
fn every_minted_record_is_accounted_for() {
    let scripts = [
        Script::new(4).then(Step::PushMany(10)),
        Script::new(2).then(Step::StageMany(10)),
        Script::new(6).then(Step::StageMany(8)).then(Step::Flush),
        Script::new(4).then(Step::Close).then(Step::PushMany(3)).then(Step::Stage),
    ];

    for (index, script) in scripts.iter().enumerate() {
        let outcome = script.run(&mut failing_ring(4));
        assert_eq!(outcome.audit(), Ok(()), "script {index}: {outcome:?}");
    }
}

/// A staging buffer smaller than the script refuses the surplus, and says so.
///
/// Six slots offered eight records: two refused at the buffer, then the flush
/// offers six to a four-slot ring and two more are refused there. Both
/// refusals are separately counted, because they are separately recoverable —
/// a staging refusal never reached the ring at all.
#[test]
fn a_full_staging_buffer_refuses_before_the_ring_is_reached() {
    let script = Script::new(6).then(Step::StageMany(8)).then(Step::Flush).then(Step::DrainAll);
    let outcome = script.run(&mut failing_ring(4));

    assert_eq!(outcome.refused_staging, 2, "eight offered to six slots");
    assert_eq!(outcome.accepted, 4, "six flushed into four slots");
    assert_eq!(outcome.refused_full, 2);
    assert_eq!(outcome.staged_at_end, 0, "a flush empties the buffer either way");
    assert_eq!(outcome.audit(), Ok(()));
}

/// Staging without flushing leaves the records staged, and counted there.
#[test]
fn records_staged_and_never_flushed_are_still_accounted_for() {
    let script = Script::new(8).then(Step::StageMany(5));
    let outcome = script.run(&mut failing_ring(8));

    assert_eq!(outcome.staged_at_end, 5);
    assert_eq!(outcome.accepted, 0);
    assert_eq!(outcome.received, [] as [u32; 0]);
    assert_eq!(outcome.audit(), Ok(()));
}

/// A staging buffer of zero slots refuses everything offered to it.
#[test]
fn a_zero_slot_staging_buffer_refuses_every_record() {
    let script = Script::new(0).then(Step::StageMany(3)).then(Step::Flush);
    let outcome = script.run(&mut failing_ring(8));

    assert_eq!(outcome.refused_staging, 3);
    assert_eq!(outcome.accepted, 0, "a flush of an empty buffer offers nothing");
    assert_eq!(outcome.audit(), Ok(()));
}

// ── shutdown ──────────────────────────────────────────────────────────────

/// A closed ring refuses pushes, and the refusal is not a full-ring refusal.
#[test]
fn a_closed_ring_refuses_with_a_reason_of_its_own() {
    let script = Script::new(2).then(Step::Close).then(Step::PushMany(3));
    let outcome = script.run(&mut failing_ring(8));

    assert_eq!(outcome.refused_closed, 3);
    assert_eq!(outcome.refused_full, 0, "there was room; the ring was shut, not full");
    assert!(outcome.closed_at_end);
}

/// Reopening admits publications again.
#[test]
fn reopening_admits_publications_again() {
    let script = Script::new(2)
        .then(Step::Close)
        .then(Step::PushMany(2))
        .then(Step::Reopen)
        .then(Step::Push)
        .then(Step::DrainAll);
    let outcome = script.run(&mut failing_ring(8));

    assert_eq!(outcome.refused_closed, 2);
    assert_eq!(outcome.accepted, 1);
    assert_eq!(outcome.received, [2], "the record minted after the reopen");
}

/// **`Reopen` on an already-open ring closes it first, and is still a no-op.**
///
/// `Stopped::reopen` consumes the token proving the ring closed, and
/// `Shutdown::close` is the only thing that mints one — so there is no way to
/// reopen without closing. In a single-threaded script the window is invisible,
/// which is precisely why it is pinned here rather than left to be discovered
/// by a concurrent caller.
#[test]
fn reopening_an_open_ring_is_observably_a_no_op() {
    let with_reopen =
        Script::new(2).then(Step::Reopen).then(Step::PushMany(2)).then(Step::DrainAll);
    let without = Script::new(2).then(Step::PushMany(2)).then(Step::DrainAll);

    assert_eq!(with_reopen.run(&mut failing_ring(8)), without.run(&mut failing_ring(8)));
}

/// A flush into a closed ring is refused as closed, not as full.
///
/// The staged records are still emptied out of the buffer — a flush does not
/// hold them back on refusal — so they end up counted in `refused_closed` and
/// in neither the buffer nor the ring. Room in the ring makes the reason
/// unambiguous: an eight-slot ring taking three records cannot be full.
#[test]
fn a_flush_into_a_closed_ring_is_refused_as_closed() {
    let script = Script::new(4).then(Step::StageMany(3)).then(Step::Close).then(Step::Flush);
    let outcome = script.run(&mut failing_ring(8));

    assert_eq!(outcome.refused_closed, 3);
    assert_eq!(outcome.refused_full, 0, "there was room for all three");
    assert_eq!(outcome.accepted, 0);
    assert_eq!(outcome.staged_at_end, 0, "the buffer is emptied even when nothing lands");
    assert_eq!(outcome.audit(), Ok(()));
}

/// `DrainAll` recovers everything and leaves the ring closed.
#[test]
fn drain_all_recovers_every_record_and_leaves_the_ring_closed() {
    let script = Script::new(2).then(Step::PushMany(5)).then(Step::DrainAll);
    let outcome = script.run(&mut failing_ring(8));

    assert_eq!(outcome.received, [0, 1, 2, 3, 4]);
    assert_eq!(outcome.in_ring_at_end, 0);
    assert!(outcome.closed_at_end);
}

/// Closing does not discard what is already in the ring.
#[test]
fn closing_leaves_published_records_where_they_are() {
    let script = Script::new(2).then(Step::PushMany(3)).then(Step::Close);
    let outcome = script.run(&mut failing_ring(8));

    assert_eq!(outcome.in_ring_at_end, 3);
    assert_eq!(outcome.received, [] as [u32; 0]);

    // The method that measures discarding, asserted in the one test whose name
    // claims nothing was discarded. `vanished` subtracts `received + in_ring`
    // from `accepted`, and every other test that reads it leaves `in_ring_at_end`
    // at zero — where adding that term and subtracting it give the same answer.
    // This is the only fixture in the suite where the two can disagree.
    assert_eq!(outcome.vanished(), 0, "records held by a closed ring are not destroyed");
}

// ── the single-record steps ───────────────────────────────────────────────

/// `Push`/`Recv`/`Stage` are the one-record forms of their `Many` counterparts.
#[test]
fn the_single_record_steps_match_a_many_of_one() {
    let singles = Script::new(4).then(Step::Push).then(Step::Stage).then(Step::Recv);
    let manys =
        Script::new(4).then(Step::PushMany(1)).then(Step::StageMany(1)).then(Step::RecvMany(1));

    assert_eq!(singles.run(&mut failing_ring(8)), manys.run(&mut failing_ring(8)));
}

/// A receive from an empty ring yields nothing and stops the step early.
///
/// `RecvMany( 10 )` against three records takes three: the step breaks on the
/// first empty read rather than spinning, so the count is a ceiling.
#[test]
fn a_receive_step_stops_at_the_first_empty_read() {
    let script = Script::new(2).then(Step::PushMany(3)).then(Step::RecvMany(10));
    let outcome = script.run(&mut failing_ring(8));

    assert_eq!(outcome.received, [0, 1, 2]);

    let empty = Script::new(2).then(Step::Recv);
    assert_eq!(empty.run(&mut failing_ring(8)).received, [] as [u32; 0]);
}

// ── the script value itself ───────────────────────────────────────────────

/// A script reports the steps and the staging limit it was built with.
#[test]
fn a_script_reports_what_it_was_built_from() {
    let script = Script::new(7).then(Step::Push).then(Step::Recv);

    assert_eq!(script.steps(), [Step::Push, Step::Recv]);
    assert_eq!(script.stage_limit(), 7);

    let empty = Script::new(0);
    assert_eq!(empty.steps(), [] as [Step; 0]);
    assert_eq!(empty.stage_limit(), 0);
}

/// An empty script does nothing and says so.
#[test]
fn an_empty_script_produces_an_empty_outcome() {
    let outcome = Script::new(4).run(&mut failing_ring(8));

    assert_eq!(
        outcome,
        Outcome {
            minted: 0,
            accepted: 0,
            refused_full: 0,
            refused_closed: 0,
            refused_staging: 0,
            received: Vec::new(),
            published: Vec::new(),
            in_ring_at_end: 0,
            staged_at_end: 0,
            closed_at_end: false,
        },
    );
    assert_eq!(outcome.audit(), Ok(()));
}

// ── the audit ─────────────────────────────────────────────────────────────

/// A clean list of consecutive records passes.
#[test]
fn an_ascending_list_of_minted_records_passes_the_audit() {
    assert_eq!(audit_received(&[0, 1, 2], 3), Ok(()));
    assert_eq!(audit_received(&[0, 5, 9], 10), Ok(()), "gaps are legal — records may be dropped");
    assert_eq!(audit_received(&[], 0), Ok(()));
    assert_eq!(audit_received(&[7], 8), Ok(()));
}

/// A record that was never minted is named, with the bound it broke.
#[test]
fn a_record_that_was_never_minted_is_caught() {
    assert_eq!(audit_received(&[0, 3], 3), Err(Anomaly::Unminted { value: 3, minted: 3 }));
    assert_eq!(audit_received(&[9], 0), Err(Anomaly::Unminted { value: 9, minted: 0 }));
}

/// A repeat and a reversal are the same anomaly, because they are the same
/// broken comparison.
#[test]
fn a_duplicate_and_a_reversal_are_one_anomaly() {
    assert_eq!(audit_received(&[1, 1], 4), Err(Anomaly::OutOfOrder { previous: 1, then: 1 }));
    assert_eq!(audit_received(&[2, 1], 4), Err(Anomaly::OutOfOrder { previous: 2, then: 1 }));
}

/// An outcome whose counts do not add up is caught before its records are.
///
/// Built by hand rather than by a script, because a script that produced one
/// would be the bug this check exists to find.
#[test]
fn an_outcome_that_lost_a_record_fails_the_audit() {
    let outcome = Outcome {
        minted: 5,
        accepted: 2,
        refused_full: 1,
        refused_closed: 0,
        refused_staging: 0,
        received: vec![0, 1],
        published: vec![0, 1],
        in_ring_at_end: 0,
        staged_at_end: 0,
        closed_at_end: false,
    };

    assert_eq!(outcome.audit(), Err(Anomaly::Unaccounted { minted: 5, placed: 3 }));
}

/// An outcome that adds up but delivered a record it never minted still fails.
#[test]
fn an_outcome_that_adds_up_is_still_checked_for_its_records() {
    let outcome = Outcome {
        minted: 2,
        accepted: 2,
        refused_full: 0,
        refused_closed: 0,
        refused_staging: 0,
        received: vec![0, 4],
        published: vec![0, 4],
        in_ring_at_end: 0,
        staged_at_end: 0,
        closed_at_end: false,
    };

    assert_eq!(outcome.audit(), Err(Anomaly::Unminted { value: 4, minted: 2 }));
}

/// Every anomaly prints something a failure message can carry.
#[test]
fn every_anomaly_says_what_broke() {
    let messages = [
        Anomaly::Unaccounted { minted: 5, placed: 3 }.to_string(),
        Anomaly::Unminted { value: 4, minted: 2 }.to_string(),
        Anomaly::OutOfOrder { previous: 2, then: 1 }.to_string(),
        Anomaly::Overdelivered { accepted: 3, out: 6 }.to_string(),
    ];

    assert!(messages[0].contains('5') && messages[0].contains('3'));
    assert!(messages[1].contains('4') && messages[1].contains('2'));
    assert!(messages[2].contains('2') && messages[2].contains('1'));
    assert!(messages[3].contains('6') && messages[3].contains('3'));

    // The trait is implemented, so a caller can `?` an anomaly out of a test.
    let boxed: Box<dyn core::error::Error> = Box::new(Anomaly::OutOfOrder { previous: 2, then: 1 });
    assert!(!boxed.to_string().is_empty());
}

// ── the loom bridge ───────────────────────────────────────────────────────

/// `leak` produces ends that outlive the scope that built the ring.
///
/// The `'static` this returns is what a `loom::thread::spawn` closure needs and
/// what a borrowed local cannot give it. Asserted here under an ordinary build
/// because `tests/exhaustive_test.rs` only compiles under `--cfg loom`, and a
/// helper nothing checks in the default configuration is a helper that rots.
#[test]
fn a_leaked_ring_gives_ends_that_outlive_their_scope() {
    let ends: ring_core::Ends<'static, u32> = {
        let ring = failing_ring(2);
        leak(ring).ends()
    };

    let mut ends = ends;
    let (mut producer, mut consumer) = ends.split();

    assert!(producer.try_push(1).is_ok());
    assert_eq!(consumer.try_recv(), Some(1));
}

/// A script runs against a leaked ring exactly as against a borrowed one.
#[test]
fn a_script_runs_the_same_against_a_leaked_ring() {
    let script = Script::new(2).then(Step::PushMany(3)).then(Step::DrainAll);

    let borrowed = script.run(&mut failing_ring(8));
    let leaked = script.run(leak(failing_ring(8)));

    assert_eq!(borrowed, leaked);
}

/// **`leak_ends` gives both ends a lifetime that reaches a spawned thread.**
///
/// This is the shape a `loom::model` needs and the reason the helper exists:
/// two leaks, not one. `leak` alone leaves the `Ends` a local, and a `Producer`
/// borrowed from a local cannot be moved onto a `'static` closure.
///
/// Ordinary `std` threads here rather than loom's, because this file compiles
/// without `--cfg loom` — the lifetime is the property under test, and it is
/// the same lifetime either runtime demands. The loom model itself is
/// `tests/exhaustive_test.rs`.
#[test]
fn leak_ends_produces_ends_that_can_be_moved_onto_spawned_threads() {
    let (mut producer, mut consumer) = ring_testkit::leak_ends(failing_ring(2));

    let producing = std::thread::spawn(move || {
        assert!(producer.try_push(7).is_ok(), "an empty ring admits one");
    });
    producing.join().expect("the producer thread");

    let draining = std::thread::spawn(move || consumer.try_recv());
    assert_eq!(draining.join().expect("the drain thread"), Some(7));
}

// ── the boundaries the findings named ─────────────────────────────────────

/// `vanished()` answers the healthy `0` for a state no run can produce, and
/// `audit` is what tells that zero from the real one.
///
/// The subtraction saturates: three records out of a ring that accepted two
/// clamps to `0`, which is the same reading a ring that destroyed nothing
/// gives. Nothing in the returned struct distinguishes them —
/// `Anomaly::Overdelivered` is the distinction, and it lives on the other
/// method. → `docs/data_structure/002` TK11.
#[test]
fn an_outcome_that_delivered_more_than_it_accepted_is_caught() {
    let outcome = Outcome {
        minted: 2,
        accepted: 2,
        refused_full: 0,
        refused_closed: 0,
        refused_staging: 0,
        received: vec![0, 1],
        published: vec![0, 1],
        in_ring_at_end: 1,
        staged_at_end: 0,
        closed_at_end: false,
    };

    assert_eq!(outcome.vanished(), 0, "the saturation reports the healthy value");
    assert_eq!(
        outcome.audit(),
        Err(Anomaly::Overdelivered { accepted: 2, out: 3 }),
        "and the audit reports the state the saturation hid",
    );
}

/// The mistake `Script::run`'s contract warns about, reached by a real script.
///
/// "A caller comparing two runs must supply two rings, not run twice on one."
/// Run twice on one and the second `Outcome` describes six records in a ring
/// that accepted three — every field of it individually plausible, the set of
/// them impossible, and `vanished()` reporting the healthy `0` throughout.
/// This is the second of TK11's two established routes to that state, and
/// unlike the hand-built `Outcome` above it needs no cooperation from the
/// caller beyond a mistake the contract already names.
#[test]
fn a_script_run_twice_on_one_ring_produces_an_outcome_that_fails_its_audit() {
    let script = Script::new(2).then(Step::PushMany(3));
    let mut ring = failing_ring(8);

    let first = script.run(&mut ring);
    assert_eq!(first.audit(), Ok(()), "one run over a fresh ring is coherent");
    assert_eq!(first.in_ring_at_end, 3);

    let second = script.run(&mut ring);
    assert_eq!(second.vanished(), 0, "the saturation reports the healthy value");
    assert_eq!(
        second.audit(),
        Err(Anomaly::Overdelivered { accepted: 3, out: 6 }),
        "the ring holds both runs, and only the audit says so",
    );
}

/// Two producers interleaving pass the unordered audit and fail the ordered one.
///
/// Producer A minted 0, 1, 2 and producer B minted 100, 101; the ring delivered
/// them in claim order. That is a correct delivery and not an ascending one, so
/// `audit_received` — the function whose stated reason for existing is the
/// concurrent case — reports it as an anomaly. → `docs/algorithm/002` TK3.
#[test]
fn interleaved_producers_pass_only_the_unordered_audit() {
    let delivered = [0, 100, 1, 101, 2];

    assert_eq!(audit_received_unordered(&delivered, 200), Ok(()));
    assert_eq!(
        audit_received(&delivered, 200),
        Err(Anomaly::OutOfOrder { previous: 100, then: 1 }),
        "the ascent check reports a correct concurrent delivery as an anomaly",
    );
}

/// Dropping the ascent check keeps the two failures a ring can actually produce.
#[test]
fn the_unordered_audit_keeps_the_two_failures_a_ring_can_produce() {
    assert_eq!(
        audit_received_unordered(&[5, 9, 5], 10),
        Err(Anomaly::OutOfOrder { previous: 5, then: 5 }),
        "a record delivered twice is still an anomaly",
    );
    assert_eq!(
        audit_received_unordered(&[0, 12], 10),
        Err(Anomaly::Unminted { value: 12, minted: 10 }),
        "a record no producer could have minted is still an anomaly",
    );
    assert_eq!(audit_received_unordered(&[], 10), Ok(()));
    assert_eq!(
        audit_received_unordered(&[4, 3, 2, 1, 0], 5),
        Ok(()),
        "and descending is not one, which is the whole difference",
    );
}

/// A `Many` count far above the ring's capacity is executed in full.
///
/// Nothing clamps it. The termination argument calls each step "bounded by a
/// constant in the step itself"; the constant is whichever number the caller
/// wrote, and a ring that refuses everything after the fourth record still
/// receives all thousand offers. → `docs/data_structure/001` TK10.
#[test]
fn a_many_count_far_above_capacity_is_executed_in_full() {
    let outcome = Script::new(0).then(Step::PushMany(1000)).run(&mut failing_ring(4));

    assert_eq!(outcome.minted, 1000, "every record the count asked for was minted");
    assert_eq!(outcome.accepted, 4);
    assert_eq!(outcome.refused_full, 996, "996 offers a clamp would have skipped");
    assert_eq!(outcome.audit(), Ok(()));

    // `RecvMany` is the one that stops early, and only because an empty read
    // ends its loop — not because the count was checked against anything.
    let drained = Script::new(0).then(Step::RecvMany(1000)).run(&mut failing_ring(4));
    assert_eq!(drained.received, [] as [u32; 0]);
}

/// A `Push` landing between a `Stage` and its `Flush` still audits clean —
/// the ring delivered records in the exact order it accepted them, and a
/// lower mint value flushed later must not be reported as an out-of-order
/// delivery.
///
/// Root Cause: `Outcome::audit` delegated straight to
/// `audit_received( &self.received, self.minted )`, which reports
/// `Anomaly::OutOfOrder` whenever `received` fails to ascend by raw mint
/// value. A `Stage` mints a record without publishing it, so a `Push` minted
/// afterward can reach the ring first; the record `Stage` minted keeps its
/// lower value and is offered to the ring only once `Flush` runs. Delivery
/// then descends by mint value for a run that did nothing wrong.
///
/// Why Not Caught: every existing script either pushed directly with no
/// staging at all, or staged and flushed with no push interleaved between
/// them — both keep mint order and push order identical by construction, so
/// no test before this one exercised the one interleaving where they differ.
///
/// Fix Applied: `Outcome` gained a `published` field recording accepted
/// records in actual push order, populated at both `try_push` call sites in
/// `Script::run`. `Outcome::audit` now checks `received` against `published`
/// through `audit_delivery_order`, which compares *positions* in `published`
/// rather than raw mint values — the reading a mixed script needs.
///
/// Prevention: an audit over a fixture's own output must be checked against
/// what the fixture actually recorded happening, not against a value (mint
/// order) that only coincides with it under an unstated assumption about
/// step ordering.
///
/// Pitfall: `audit_received`/`audit_received_unordered` are unchanged and
/// must stay that way — they are correct for their own documented callers (a
/// `Push`-only script, the loom model's direct `try_push`), where mint order
/// and push order are the same thing by construction. Do not fold
/// `audit_delivery_order`'s position-based check back into them; that would
/// require a `published` list neither caller has any way to obtain outside a
/// full `Outcome`.
#[test]
fn a_push_staged_between_a_stage_and_its_flush_still_audits_clean() {
    let script = Script::new(4)
    .then(Step::Stage) // mints 0, stages it — not yet in the ring
    .then(Step::Push) // mints 1, pushes it to the ring immediately
    .then(Step::Flush) // pushes record 0 to the ring second
    .then(Step::DrainAll);
    let outcome = script.run(&mut failing_ring(8));

    assert_eq!(outcome.received, [1, 0], "record 1 was pushed before record 0 was flushed");
    assert_eq!(outcome.published, [1, 0], "push order, not mint order");
    assert_eq!(
        outcome.audit(),
        Ok(()),
        "a correct FIFO delivery must not be reported as out of order"
    );
}
