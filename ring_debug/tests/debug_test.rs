//! `ring_debug` — the checks, and the measurement that justifies them.
//!
//! `docs/feature/185_ring_stats.md`'s acceptance criterion is "`ring_debug`'s
//! invariant check catches a deliberately corrupted cursor" — one of the three
//! that feature carries, the other two belonging to `ring_stats` and
//! `ring_trace`. Every corruption here is performed through
//! `ring_cursor`'s own public `store`, against a real `CursorPair`, rather than
//! by constructing a fixture that describes a corrupt state. That distinction is
//! the criterion: a fixture would prove the checker can read a struct, not that
//! it catches something the family can actually do to itself.
//!
//! # What each group covers
//!
//! | Group | Invariant | Doc instance |
//! |---|---|---|
//! | `check` | D1, D2 | `docs/invariant/001` V1, V2 |
//! | `Watch` | D3 | `docs/invariant/001` V3, `docs/state_machine/001` |
//! | The measurement | — | `docs/pitfall/001` — pins what is *not* detected without this crate |
//! | `check_ends` | Two readings of one ring | `docs/invariant/001`'s `ReadingsDisagree` |

// Ordinary tests, compiled out under `--cfg loom`. That cfg swaps
// `ring_atomic`'s atomics for loom's instrumented ones across the whole
// family, and those panic the moment they are touched outside a
// `loom::model` closure — so without this gate a family-wide loom run dies
// here instead of reaching the models in `ring_spsc`, `ring_mpsc`,
// `ring_publish` and `ring_testkit`.
#![cfg(not(loom))]

use core::sync::atomic::Ordering;

use ring_atomic::SeqCell;
use ring_config::RingConfig;
use ring_core::Ring;
use ring_cursor::CursorPair;
use ring_debug::{Cursor, Violation, Watch, check, check_ends};
use ring_types::{Capacity, Seq};

fn cap(n: usize) -> Capacity {
    Capacity::new(n).expect("a valid capacity")
}

/// A pair with both cursors placed where the caller asks.
///
/// The corruption vector, and deliberately not hidden behind anything: `store`
/// is the same public method a producer publishes with, which is precisely
/// `docs/invariant/001`'s E1 gap — the family cannot distinguish publishing
/// from corrupting, because they are the same call.
fn pair_at(capacity: usize, producer: u64, consumer: u64) -> CursorPair {
    let pair = CursorPair::new(cap(capacity));
    pair.producer().store(Seq(producer), Ordering::Release);
    pair.consumer().store(Seq(consumer), Ordering::Release);
    pair
}

// ---------------------------------------------------------------------------
// check — D1 and D2
// ---------------------------------------------------------------------------

/// V1 — a consumer that has read past what the producer published.
///
/// The violation this crate exists for. Nothing else in the family reports it;
/// see `the_arithmetic_reports_an_empty_ring_for_a_consumer_ahead_cursor` below
/// for what the family *does* report about this exact pair.
#[test]
fn a_consumer_ahead_of_its_producer_is_caught() {
    let pair = pair_at(8, 3, 9);

    assert_eq!(
        check(&pair),
        Err(Violation::ConsumerAheadOfProducer { producer: Seq(3), consumer: Seq(9) })
    );
}

/// A consumer exactly level with its producer is the empty ring, not a
/// violation.
///
/// The off-by-one that would make the checker useless in the other direction:
/// `c == p` is every ring that has been fully drained, so a `>=` here would fire
/// on the most ordinary state there is.
#[test]
fn a_consumer_level_with_its_producer_is_not_a_violation() {
    assert!(check(&pair_at(8, 5, 5)).is_ok(), "a fully drained ring was called corrupt");
}

/// V2 — a producer more than a full lap ahead.
#[test]
fn a_producer_more_than_a_lap_ahead_is_caught() {
    let pair = pair_at(8, 30, 0);

    assert_eq!(
        check(&pair),
        Err(Violation::ProducerLappedConsumer { producer: Seq(30), consumer: Seq(0), capacity: 8 })
    );
}

/// A producer exactly a lap ahead is a full ring, not a lapped one.
///
/// The boundary D2 is stated at: `p - c <= capacity`. A full ring has
/// `p - c == capacity` and every slot legitimately occupied; one more is the
/// first record that overwrites something unread.
#[test]
fn a_producer_exactly_a_lap_ahead_is_full_not_lapped() {
    assert!(check(&pair_at(8, 8, 0)).is_ok(), "a full ring was called lapped");
    assert!(check(&pair_at(8, 9, 0)).is_err(), "one past full was not caught");
}

/// A fresh pair and an ordinary in-flight pair both pass.
///
/// Without this the suite would be satisfied by a `check` that returned an error
/// unconditionally.
#[test]
fn healthy_pairs_pass() {
    assert!(check(&CursorPair::new(cap(8))).is_ok(), "a new ring failed");
    assert!(check(&pair_at(8, 3, 0)).is_ok(), "a partly-filled ring failed");
    assert!(check(&pair_at(8, 1_000, 995)).is_ok(), "a long-running ring failed");
}

/// D1 is reported in preference to D2 when a pair breaks both.
///
/// `producer 0, consumer 40` on a capacity of 8 is a consumer ahead by 40 — and
/// were the operands read the other way round it would also look like a lap.
/// The order matters because D1 is the one with no other symptom: reporting D2
/// here would send an investigation looking for a runaway producer that does not
/// exist.
#[test]
fn a_pair_breaking_both_reports_the_invisible_one() {
    assert_eq!(
        check(&pair_at(8, 0, 40)),
        Err(Violation::ConsumerAheadOfProducer { producer: Seq(0), consumer: Seq(40) })
    );
}

/// Checking does not move either cursor.
///
/// `check` is a diagnostic run against a live ring, so a check that perturbed
/// what it measured would be worse than no check. Asserted rather than assumed
/// because `PaddedCursor` exposes `store` and nothing structurally prevents it.
#[test]
fn checking_leaves_both_cursors_where_they_were() {
    let pair = pair_at(8, 5, 2);

    for _ in 0..100 {
        let _ = check(&pair);
    }

    assert_eq!(pair.producer().load(Ordering::Acquire), Seq(5));
    assert_eq!(pair.consumer().load(Ordering::Acquire), Seq(2));
}

// ---------------------------------------------------------------------------
// The measurement — docs/pitfall/001
// ---------------------------------------------------------------------------

/// The family reports a D1-corrupted ring as empty, healthy, and claimable.
///
/// `docs/pitfall/001`'s measurement, re-taken as an assertion. This is the test
/// that makes the crate's existence falsifiable: it pins the three readings a
/// caller would otherwise trust, so a future change to
/// `ring_types::Seq::distance_to` that stopped the masking would fail *here* and
/// be noticed — rather than silently making this crate redundant while every
/// other test in the file still passed.
///
/// Note `may_claim` in particular. It is not merely uninformative; it is the
/// reading a producer acts on, and it says go ahead.
#[test]
fn the_arithmetic_reports_an_empty_ring_for_a_consumer_ahead_cursor() {
    let corrupt = pair_at(8, 3, 9);
    let empty = CursorPair::new(cap(8));

    assert_eq!(corrupt.free_slots(), 8, "a corrupt ring did not report itself empty");
    assert_eq!(corrupt.pending(), 0);
    assert!(corrupt.may_claim(), "the reading a producer acts on");

    assert_eq!(
        (corrupt.free_slots(), corrupt.pending(), corrupt.may_claim()),
        (empty.free_slots(), empty.pending(), empty.may_claim()),
        "if these ever differ, the masking is gone and this crate's D1 check is redundant"
    );

    assert!(check(&corrupt).is_err(), "and this is the only thing that says otherwise");
}

/// D2 degrades in the opposite direction — visible, and declining to claim.
///
/// The other half of the pitfall's asymmetry table. Both corruptions are equally
/// wrong; only one of them lies about it.
#[test]
fn the_arithmetic_reports_a_lapped_ring_as_full_and_unclaimable() {
    let lapped = pair_at(8, 30, 0);

    assert_eq!(lapped.free_slots(), 0);
    assert_eq!(lapped.pending(), 30, "pending exceeding capacity is the visible evidence");
    assert!(!lapped.may_claim(), "the family declines to make a lap worse");
}

// ---------------------------------------------------------------------------
// Watch — D3
// ---------------------------------------------------------------------------

/// V3 — a cursor that goes backwards.
#[test]
fn a_cursor_that_goes_backwards_is_caught() {
    let pair = pair_at(8, 5, 1);
    let mut watch = Watch::new(&pair).expect("a valid pair");

    pair.producer().store(Seq(7), Ordering::Release);
    assert!(watch.observe(&pair).is_ok(), "forward movement was rejected");

    pair.producer().store(Seq(2), Ordering::Release);
    assert_eq!(
        watch.observe(&pair),
        Err(Violation::CursorWentBackwards { cursor: Cursor::Producer, was: Seq(7), now: Seq(2) })
    );
}

/// A consumer going backwards is caught, and named as the consumer.
///
/// Separate from the producer case because the `cursor` field is the entire
/// diagnostic value of the variant — a report that says "a cursor moved
/// backwards" without saying which one leaves the reader exactly where they
/// started.
#[test]
fn a_backwards_consumer_is_named_as_the_consumer() {
    let pair = pair_at(8, 20, 15);
    let mut watch = Watch::new(&pair).expect("a valid pair");

    pair.consumer().store(Seq(4), Ordering::Release);
    assert_eq!(
        watch.observe(&pair),
        Err(Violation::CursorWentBackwards { cursor: Cursor::Consumer, was: Seq(15), now: Seq(4) })
    );
}

/// A single observation cannot see D3 — the reason `Watch` exists at all.
///
/// `docs/pitfall/001`'s P3, stated as a test. A pair reset to `(0, 0)` is a
/// perfectly valid pair; `check` passes it, correctly, and `Watch` does not.
/// If this ever started failing, `Watch` would be redundant and `check` would
/// have grown state it should not have.
///
/// Capacity 32, not 8: at 8 the baseline `(500, 480)` is itself a D2 violation
/// and `Watch::new` refuses it. That was this test's first version, and the
/// refusal caught it — `a_watch_refuses_to_baseline_a_broken_pair`'s guarantee
/// firing against the author of the test rather than against a corrupted ring.
#[test]
fn a_stateless_check_cannot_see_a_reset_and_a_watch_can() {
    let pair = pair_at(32, 500, 480);
    let mut watch = Watch::new(&pair).expect("a valid pair");

    pair.producer().store(Seq::ZERO, Ordering::Release);
    pair.consumer().store(Seq::ZERO, Ordering::Release);

    assert!(check(&pair).is_ok(), "a reset pair is a valid pair, seen on its own");
    assert!(watch.observe(&pair).is_err(), "and is not, seen against what came before");
}

/// A watch started against an already-broken pair says so immediately.
///
/// Otherwise the corrupt state becomes the baseline and the watch reports
/// nothing, forever — the failure mode where the instrument silently adopts what
/// it was installed to find.
#[test]
fn a_watch_refuses_to_baseline_a_broken_pair() {
    assert_eq!(
        Watch::new(&pair_at(8, 3, 9)),
        Err(Violation::ConsumerAheadOfProducer { producer: Seq(3), consumer: Seq(9) })
    );
}

/// A failed observation does not become the new baseline.
///
/// The same failure mode one step later: a watch that adopted the corrupt
/// reading would report a permanent fault exactly once and call it normal
/// afterwards.
#[test]
fn a_failed_observation_leaves_the_baseline_alone() {
    let pair = pair_at(8, 10, 4);
    let mut watch = Watch::new(&pair).expect("a valid pair");
    assert_eq!(watch.last(), (Seq(10), Seq(4)));

    pair.producer().store(Seq(1), Ordering::Release);
    assert!(watch.observe(&pair).is_err());
    assert_eq!(watch.last(), (Seq(10), Seq(4)), "the watch adopted the corrupt reading");

    // And it is still reporting, rather than having accepted the fault as normal.
    assert!(watch.observe(&pair).is_err(), "the second look went quiet");
}

/// DB38 — T6: a watch that reported a violation observes cleanly once the ring
/// recovers, and does not latch.
///
/// The state machine has six transitions; this is the one the suite reached
/// hardest for in prose (`docs/decisions/002` defers latching to a caller on
/// the strength of it) and tested least — no other test performs a failing
/// observation followed by a passing one. Without this test, a change that made
/// `Watch` absorbing (report the first violation forever, ignoring further
/// reality) would pass every other test in this file, including
/// `a_failed_observation_leaves_the_baseline_alone` above, which only asserts
/// that a *second* bad reading still fails — never that a *good* one afterward
/// succeeds.
#[test]
fn a_watch_that_faulted_reports_ok_once_the_ring_recovers() {
    let pair = pair_at(8, 10, 4);
    let mut watch = Watch::new(&pair).expect("a valid pair");

    // A transient D1 glitch: the consumer misreads ahead of the producer.
    pair.consumer().store(Seq(15), Ordering::Release);
    assert!(watch.observe(&pair).is_err(), "the glitch must be reported");
    assert_eq!(watch.last(), (Seq(10), Seq(4)), "the glitch must not become the baseline");

    // Recovery: both cursors legitimately advance past the pre-glitch baseline,
    // consistent with each other — exactly what a real producer/consumer pair
    // does after a corrupted read is superseded by the next real one.
    pair.producer().store(Seq(20), Ordering::Release);
    pair.consumer().store(Seq(12), Ordering::Release);
    assert!(
        watch.observe(&pair).is_ok(),
        "a legitimate reading after a transient fault must be accepted, not latched"
    );
    assert_eq!(watch.last(), (Seq(20), Seq(12)), "the recovered reading becomes the new baseline");

    // And the watch keeps working afterward rather than being left wedged.
    pair.producer().store(Seq(21), Ordering::Release);
    pair.consumer().store(Seq(13), Ordering::Release);
    assert!(watch.observe(&pair).is_ok(), "an ordinary step after recovery must still pass");
}

/// A watch follows an ordinary run without complaint.
///
/// The negative control for the whole stateful half: forty legitimate advances,
/// no findings.
#[test]
fn a_watch_follows_an_ordinary_run_quietly() {
    let pair = CursorPair::new(cap(8));
    let mut watch = Watch::new(&pair).expect("a new pair");

    for step in 1..=40u64 {
        pair.producer().store(Seq(step), Ordering::Release);
        pair.consumer().store(Seq(step.saturating_sub(3)), Ordering::Release);
        assert!(watch.observe(&pair).is_ok(), "step {step} was rejected");
    }

    assert_eq!(watch.last(), (Seq(40), Seq(37)));
}

/// A watch still catches D1 and D2, not only D3.
///
/// `Watch` adds monotonicity; it does not trade the stateless checks away for
/// it. Worth pinning because the implementation checks D3 first, and an early
/// return is exactly how the other two would get lost.
#[test]
fn a_watch_still_catches_the_stateless_violations() {
    let pair = pair_at(8, 5, 1);
    let mut watch = Watch::new(&pair).expect("a valid pair");

    // Forward for both cursors — D3 holds — but the consumer overtakes.
    pair.consumer().store(Seq(9), Ordering::Release);
    assert_eq!(
        watch.observe(&pair),
        Err(Violation::ConsumerAheadOfProducer { producer: Seq(5), consumer: Seq(9) })
    );
}

/// DB6 — a forked baseline is now something you have to ask for.
///
/// `Watch` was `Copy`, so `let w2 = w1;` produced a second independent baseline
/// and read, at the call site, exactly like a move. This is that fork written
/// deliberately, and it is also the reason the derive had to go: the watch left
/// behind reports a backwards cursor against a ring that only ever moved
/// forward. Nothing is wrong with the ring; the baseline is stale.
///
/// The `.clone()` below is the whole point. Without `Copy` it is required, so
/// the divergence this test demonstrates cannot happen by accident — and
/// `Clone` is kept rather than removed because checkpoint-then-compare is a
/// legitimate use, which is what the first half of this test is.
#[test]
fn a_cloned_watch_forks_the_baseline() {
    let pair = pair_at(8, 5, 1);
    let mut live = Watch::new(&pair).expect("a valid pair");
    let mut checkpoint = live.clone();

    pair.producer().store(Seq(9), Ordering::Release);
    assert!(live.observe(&pair).is_ok(), "the ring only moved forward");
    assert_eq!(live.last(), (Seq(9), Seq(1)));

    // The checkpoint never saw the move, so it is still holding the old baseline.
    assert_eq!(checkpoint.last(), (Seq(5), Seq(1)), "the fork is independent");
    assert!(checkpoint.observe(&pair).is_ok(), "catching up is forward too");

    // And this is the hazard, reproduced on purpose: rewind the live watch's view
    // by handing the stale fork a ring that has since moved on further, then back.
    pair.producer().store(Seq(6), Ordering::Release);
    assert_eq!(
        checkpoint.observe(&pair),
        Err(Violation::CursorWentBackwards { cursor: Cursor::Producer, was: Seq(9), now: Seq(6) }),
        "a stale baseline reports the ring, not itself"
    );
}

/// DB5 — `observe` accepts a foreign pair, and answers about neither ring.
///
/// A `Watch` holds three scalars and no identity, so nothing stops a second
/// ring's pair being passed to it. Every comparison still runs and the result is
/// a `Result< (), Violation >` indistinguishable from a verdict about a real
/// ring: D3 against the *first* ring's baseline, D2 against the *first* ring's
/// cached capacity, D1 against the *second* ring's own two cursors.
///
/// This test does not assert that the behaviour is right — it is not. It pins
/// what it currently is, because the finding's second half was that the
/// behaviour was unobserved as well as unprevented, and an exposure nothing
/// exercises is one that changes shape without anyone noticing.
#[test]
fn observing_a_foreign_pair_answers_about_neither_ring() {
    let watched = pair_at(8, 20, 15);
    let foreign = pair_at(8, 4, 2);
    let mut watch = Watch::new(&watched).expect("a valid pair");

    // Nothing about `foreign` is broken — `check` passes it on its own terms.
    assert!(check(&foreign).is_ok(), "the foreign ring is healthy");

    // Handed to a watch baselined on a different ring, that same healthy pair
    // reports a backwards cursor, with sequences from two different rings in one
    // report: `was` is the watched ring's, `now` is the foreign ring's.
    assert_eq!(
        watch.observe(&foreign),
        Err(Violation::CursorWentBackwards { cursor: Cursor::Producer, was: Seq(20), now: Seq(4) }),
        "a confident answer to a question nobody asked"
    );
}

// ---------------------------------------------------------------------------
// check_ends — two readings of one live ring
// ---------------------------------------------------------------------------

/// A real ring's two ends agree, at rest and in flight.
///
/// DB51 — the capacity is read off the ring rather than re-typed as a literal,
/// which is the only shape a downstream caller should copy. `Ring::capacity`
/// takes `&self` and `Ring::ends` takes `&mut self`, so the binding has to be
/// made *before* the split; after it, the number is unreachable for as long as
/// the ends live. Three of this suite's four call sites used to pass `cap( 16 )`,
/// and a suite that re-types the size is a suite modelling the mistake its own
/// `check_ends` rustdoc warns about.
#[test]
fn the_two_ends_of_a_live_ring_agree() {
    let config = RingConfig::new(16).expect("a valid size");
    let mut r: Ring<u32> = Ring::new(&config).expect("a ring");
    let capacity = r.capacity();
    let mut ends = r.ends();
    let (mut producer, mut consumer) = ends.split();

    assert!(check_ends(capacity, &producer, &consumer).is_ok(), "an empty ring disagreed");

    for i in 0..9u32 {
        producer.try_push(i).expect("within capacity");
    }
    assert!(check_ends(capacity, &producer, &consumer).is_ok(), "a partly-filled ring disagreed");

    let mut landed = Vec::new();
    assert_eq!(consumer.try_recv_batch(&mut landed), 9);
    assert_eq!(landed.len(), 9);
    assert!(check_ends(capacity, &producer, &consumer).is_ok(), "a drained ring disagreed");

    assert_eq!(capacity, cap(16), "the derived capacity is the configured one");
}

/// `check_ends` on a ring genuinely filled to capacity, not merely partly full.
///
/// `the_two_ends_of_a_live_ring_agree` above reaches empty, 9-of-16, and
/// drained — never the fourth legitimate state a producer under backpressure
/// sits in every day: `free_capacity() == 0` with nothing corrupt about it.
/// That boundary is exactly where `ring_spsc::Producer::free_capacity`'s
/// `capacity - occupancy` would underflow if `occupancy` ever exceeded
/// `capacity` (`docs/invariant/002`'s DB35) — so a genuinely-full ring is the
/// legitimate state that sits closest to the corrupt one, and is worth pinning
/// on its own rather than trusting the partly-filled case to stand in for it.
#[test]
fn check_ends_on_a_genuinely_full_ring() {
    let config = RingConfig::new(16).expect("a valid size");
    let mut r: Ring<u32> = Ring::new(&config).expect("a ring");
    let capacity = r.capacity();
    let mut ends = r.ends();
    let (mut producer, consumer) = ends.split();

    for i in 0..16u32 {
        producer.try_push(i).unwrap_or_else(|_| panic!("push {i} should fit exactly at capacity"));
    }

    assert_eq!(producer.free_capacity(), 0, "a full ring should report no free capacity");
    assert_eq!(consumer.len(), 16, "a full ring should report every slot occupied");
    assert!(
        check_ends(capacity, &producer, &consumer).is_ok(),
        "a legitimately full ring disagreed"
    );
}

/// The check is against the capacity it is given, and notices a wrong one.
///
/// The only way to make two correct readings disagree without corrupting the
/// ring — and it is worth having, because a caller passing the wrong capacity is
/// a real mistake and the resulting `ReadingsDisagree` is exactly the right
/// report for it.
///
/// This is the one call site that keeps its literal, because passing the wrong
/// number by hand is precisely what it asserts about. Every other site derives
/// (DB51).
#[test]
fn a_ring_measured_against_the_wrong_capacity_disagrees() {
    let config = RingConfig::new(16).expect("a valid size");
    let mut r: Ring<u32> = Ring::new(&config).expect("a ring");
    let mut ends = r.ends();
    let (producer, consumer) = ends.split();

    assert_eq!(
        check_ends(cap(8), &producer, &consumer),
        Err(Violation::ReadingsDisagree { pending: 0, free: 16, capacity: 8 })
    );
}

/// `check_ends`' arithmetic passes a D1-corrupted ring, and `check` does not.
///
/// `docs/integration/001`'s J4, as a fact in the suite rather than a caveat in
/// prose. `check_ends` compares two *derived* readings, and those are exactly
/// the readings the saturating arithmetic masks: on a D1 pair they are
/// `pending 0` and `free 8` against a capacity of 8, which sums correctly and
/// passes.
///
/// This matters because `check_ends` is the **only** check reachable from a
/// live `ring_core::Ring` — nothing in the family hands out a `CursorPair`, and
/// `ring_core`'s ends expose no `position()`. A caller at the top who calls the
/// one check their ring supports and sees it pass has learned strictly less
/// than they think.
///
/// Asserted on a `CursorPair` rather than on a `ring_core::Ring` because the
/// latter's cursors cannot be reached to corrupt — which is the same boundary,
/// observed from the other side.
#[test]
fn check_ends_cannot_see_the_corruption_check_can() {
    let corrupt = pair_at(8, 3, 9);

    // What `check_ends` would compute, from the same two derived readings.
    let pending = corrupt.pending() as usize;
    let free = corrupt.free_slots();
    assert_eq!(
        pending + free,
        8,
        "the derived readings of a corrupt ring sum to its capacity, so the sum test passes"
    );

    assert!(check(&corrupt).is_err(), "and the raw comparison is what notices");
}

// ---------------------------------------------------------------------------
// Violation — the reported value
// ---------------------------------------------------------------------------

/// Every violation names its numbers when displayed.
///
/// The variants carry data rather than messages so a caller can match on state;
/// this asserts the other half of that bargain — that formatting for a human
/// does not throw the numbers away, which is the failure that turns a
/// diagnostic into "something went wrong".
#[test]
fn a_violation_reports_the_numbers_it_was_derived_from() {
    let cases = [
        (Violation::ConsumerAheadOfProducer { producer: Seq(3), consumer: Seq(9) }, vec!["3", "9"]),
        (
            // `consumer` is 5 rather than 0 so that the rendered distance, 25, is a
            // number no field of the variant carries. With a consumer of 0 the
            // subtraction could have been deleted and replaced by `producer` and this
            // assertion would not have noticed.
            Violation::ProducerLappedConsumer { producer: Seq(30), consumer: Seq(5), capacity: 8 },
            vec!["30", "25", "5", "8"],
        ),
        (
            Violation::CursorWentBackwards { cursor: Cursor::Consumer, was: Seq(15), now: Seq(4) },
            vec!["consumer", "15", "4"],
        ),
        (Violation::ReadingsDisagree { pending: 2, free: 3, capacity: 16 }, vec!["2", "3", "16"]),
    ];

    for (violation, expected) in cases {
        let rendered = violation.to_string();
        for needle in expected {
            assert!(rendered.contains(needle), "{violation:?} rendered as {rendered:?}");
        }
    }
}

/// DB28 — a lap report whose own cursors contradict it says so, rather than
/// rendering a distance of zero.
///
/// `Violation`'s fields are public and the enum is deliberately closed rather
/// than `#[ non_exhaustive ]` (DB4), so a caller can build this variant directly
/// and reach the formatter without passing `check_seqs`' guard. The distance was
/// computed `saturating_sub`, which absorbed the contradiction into
/// *"is 0 ahead of"* — a sentence that reads as a measurement, in the crate whose
/// founding measurement is that saturating arithmetic reports health it cannot
/// support.
#[test]
fn a_lap_report_that_contradicts_itself_does_not_fabricate_a_distance() {
    let impossible =
        Violation::ProducerLappedConsumer { producer: Seq(5), consumer: Seq(9), capacity: 8 };
    let rendered = impossible.to_string();

    assert!(rendered.contains("contradict"), "rendered as {rendered:?}");
    assert!(
        !rendered.contains("0 ahead"),
        "the saturation is back, and it reads as a measurement: {rendered:?}"
    );
}

/// DB27 — the claim in the D1 message is the crate's most load-bearing sentence,
/// and this is what holds it to the arithmetic.
///
/// `ConsumerAheadOfProducer` renders as *"... the ring reads as empty and permits
/// a claim"*. That clause is not a restatement of the violation — it is
/// `pitfall/001`'s measurement compiled into a string literal, in a crate that
/// does not depend on `ring_seqno` and had no test asserting it. If the family's
/// saturating arithmetic stopped behaving that way, the pitfall would be
/// re-measured and this sentence would not.
///
/// So the two halves are asserted together: the arithmetic on a D1 pair, and the
/// message that describes it. A change to either without the other now fails
/// here rather than leaving the crate telling every reader a consequence that had
/// stopped being true.
#[test]
fn the_d1_message_still_describes_what_the_arithmetic_does() {
    let corrupt = pair_at(8, 3, 9);

    // The arithmetic half — what the message claims, measured.
    assert_eq!(corrupt.pending(), 0, "a D1 pair reads as empty");
    assert_eq!(corrupt.free_slots(), 8, "and as entirely free");
    assert!(corrupt.may_claim(), "and permits a claim");

    // The message half — the same three facts, as the sentence a reader gets.
    let rendered = check(&corrupt).expect_err("a D1 pair is a violation").to_string();
    assert!(rendered.contains("reads as empty"), "rendered as {rendered:?}");
    assert!(rendered.contains("permits a claim"), "rendered as {rendered:?}");
}

/// A violation is an error, so `?` works in a caller that returns one.
///
/// The reason `check` returns `Result` rather than `Option`: a test or a tool
/// wants to propagate the finding, not just learn that there was one.
#[test]
fn a_violation_propagates_as_an_error() {
    fn caller(pair: &CursorPair) -> Result<(), Box<dyn core::error::Error>> {
        check(pair)?;
        Ok(())
    }

    assert!(caller(&pair_at(8, 1, 0)).is_ok());
    assert!(caller(&pair_at(8, 1, 5)).is_err());
}

/// The two cursor names render distinguishably.
///
/// Trivial, and the one thing `CursorWentBackwards` is useless without.
#[test]
fn the_cursor_names_are_distinct() {
    assert_eq!(Cursor::Producer.to_string(), "producer");
    assert_eq!(Cursor::Consumer.to_string(), "consumer");
}
