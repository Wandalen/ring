//! Tests for `ring_claim`'s exclusive sequence-range grants.
//!
//! Two features rest on this crate.
//! `docs/feature/170_claim_publish_available_commit_handshake.md`'s claim half
//! is here; its full reached-test lives in `ring_publish/tests/handshake_test.rs`,
//! because the handshake is only observable once publishing exists.
//! `docs/feature/172_multi_producer_claim.md` requires that no two producers
//! are ever granted the same sequence. This file asserts that directly, and
//! `ring_mpsc/tests/mpsc_test.rs` asserts it again end-to-end.
//!
//! ## What a sequential test cannot show
//!
//! Exclusivity is a property of concurrent claims, and a single-threaded suite
//! passes it trivially. The five multi-threaded tests below therefore do the
//! real work: `no_two_producers_are_ever_granted_the_same_sequence`
//! collects every granted range from four threads and asserts the union is a
//! partition; `no_grant_ever_passes_the_limit_under_contention` drives claims
//! against a gate tight enough that a check-then-advance implementation
//! overruns it; `claims_under_contention_lose_no_sequences` asserts the
//! ranges are contiguous with no gap, which is the failure a naive CAS retry
//! produces when it recomputes the range but not the start;
//! `claim_up_to_under_contention_loses_no_sequences_either` asserts the same
//! partition property for the partial-grant path; and
//! `each_producers_own_claims_stay_in_issue_order` asserts each producer's own
//! claims arrive in increasing order, without asserting a global order across
//! producers.
//!
//! Concurrency tests are probabilistic. Each is sized to fail reliably rather
//! than occasionally against the implementation it targets, and none of them
//! proves absence of a race. That is `loom`'s job in
//! `ring_publish/tests/handshake_test.rs`.

// Ordinary tests, compiled out under `--cfg loom`. That cfg swaps
// `ring_atomic`'s atomics for loom's instrumented ones across the whole
// family, and those panic the moment they are touched outside a
// `loom::model` closure. Without this gate a family-wide loom run dies
// here instead of reaching the models in `ring_spsc`, `ring_mpsc`,
// `ring_publish` and `ring_testkit`.
#![cfg(not(loom))]

use core::sync::atomic::Ordering;

use ring_claim::{Claim, Claimer};
use ring_cursor::SeqCell;
use ring_gating::GatingSet;
use ring_types::{Capacity, RingError, Seq};

fn cap(slots: usize) -> Capacity {
  Capacity::new(slots).expect("test capacities are powers of two")
}

// ── Claim, the range itself ────────────────────────────────────────────────

#[test]
fn a_claim_is_half_open() {
  let claim = Claim::new(Seq(4), 3);

  assert_eq!(claim.start(), Seq(4));
  assert_eq!(claim.end(), Seq(7));
  assert_eq!(claim.len(), 3);
  assert!(claim.contains(Seq(4)));
  assert!(claim.contains(Seq(6)));
  assert!(!claim.contains(Seq(7)), "end is exclusive");
  assert!(!claim.contains(Seq(3)));
}

#[test]
fn an_empty_claim_contains_nothing_and_overlaps_nothing() {
  let empty = Claim::new(Seq(5), 0);

  assert!(empty.is_empty());
  assert_eq!(empty.start(), empty.end());
  assert!(!empty.contains(Seq(5)));
  assert_eq!(empty.sequences().count(), 0);
  assert!(!empty.overlaps(Claim::new(Seq(0), 100)), "a zero-width range covers no slot");
  assert!(!Claim::new(Seq(0), 100).overlaps(empty), "and the check is symmetric");
}

#[test]
fn sequences_yields_exactly_the_range() {
  for start in 0..8u64 {
    for len in 0..8usize {
      let claim = Claim::new(Seq(start), len);
      let yielded: Vec<Seq> = claim.sequences().collect();

      assert_eq!(yielded.len(), len, "start {start}, len {len}");
      assert!(yielded.iter().all(|s| claim.contains(*s)));
      assert_eq!(yielded.first().copied(), (len > 0).then_some(claim.start()));
    }
  }
}

#[test]
fn adjacent_claims_do_not_overlap() {
  // The boundary case exclusivity turns on. Two producers handed consecutive
  // ranges must not be reported as colliding, or the property test that guards
  // multi-producer exclusivity would fail on correct behaviour.
  let first = Claim::new(Seq(0), 4);
  let second = Claim::new(Seq(4), 4);

  assert!(!first.overlaps(second));
  assert!(!second.overlaps(first));
}

#[test]
fn overlap_is_symmetric_and_detects_every_shared_sequence() {
  for a_start in 0..6u64 {
    for a_len in 0..5usize {
      for b_start in 0..6u64 {
        for b_len in 0..5usize {
          let a = Claim::new(Seq(a_start), a_len);
          let b = Claim::new(Seq(b_start), b_len);

          let shares = a.sequences().any(|s| b.contains(s));
          assert_eq!(a.overlaps(b), shares, "{a:?} vs {b:?}");
          assert_eq!(a.overlaps(b), b.overlaps(a), "asymmetric: {a:?} vs {b:?}");
        }
      }
    }
  }
}

// ── claiming, sequentially ─────────────────────────────────────────────────

#[test]
fn successive_claims_are_contiguous_and_never_overlap() {
  let consumers = GatingSet::new(cap(64), 1);
  let claimer = Claimer::new(&consumers);

  let mut previous_end = Seq::ZERO;
  for count in [1usize, 3, 2, 8, 5] {
    let claim = claimer.claim(count).expect("room in a 64-slot ring");
    assert_eq!(claim.start(), previous_end, "a gap or an overlap");
    assert_eq!(claim.len(), count);
    previous_end = claim.end();
  }
  assert_eq!(claimer.claimed(), previous_end);
}

#[test]
fn claiming_stops_exactly_at_the_gate() {
  const CAPACITY: usize = 8;
  let consumers = GatingSet::new(cap(CAPACITY), 1);
  let claimer = Claimer::new(&consumers);

  let mut granted = 0;
  while let Ok(claim) = claimer.claim(1) {
    granted += claim.len();
    assert!(granted <= CAPACITY, "claimed past a full lap");
  }

  assert_eq!(granted, CAPACITY);
  assert_eq!(claimer.claim(1), Err(RingError::Full));
}

#[test]
fn a_consumer_advancing_releases_exactly_that_many_slots() {
  let consumers = GatingSet::new(cap(4), 1);
  let claimer = Claimer::new(&consumers);

  let _first = claimer.claim(4).unwrap();
  assert_eq!(claimer.claim(1), Err(RingError::Full));

  consumers.cursor(0).unwrap().store(Seq(3), Ordering::Release);
  assert_eq!(claimer.claim(3).unwrap().start(), Seq(4));
  assert_eq!(claimer.claim(1), Err(RingError::Full), "and no more than that");
}

#[test]
fn a_claim_wider_than_the_ring_is_a_configuration_error() {
  let consumers = GatingSet::new(cap(4), 1);
  let claimer = Claimer::new(&consumers);

  let err = claimer.claim(5).unwrap_err();
  assert_eq!(
    err,
    RingError::BatchTooLarge {
      requested: 5,
      capacity: 4
    }
  );
  assert!(err.is_configuration(), "a retry loop must stop rather than spin");
}

#[test]
fn a_claim_that_does_not_fit_yet_is_back_pressure() {
  let consumers = GatingSet::new(cap(4), 1);
  let claimer = Claimer::new(&consumers);
  let _taken = claimer.claim(3).unwrap();

  assert_eq!(claimer.claim(2), Err(RingError::Full));
  assert!(!RingError::Full.is_configuration(), "this one is worth retrying");
}

#[test]
fn a_failed_claim_advances_nothing() {
  // The property that makes retrying safe. A claim that moved the cursor and
  // then reported failure would leak a slot per attempt.
  let consumers = GatingSet::new(cap(4), 1);
  let claimer = Claimer::new(&consumers);
  let _taken = claimer.claim(4).unwrap();

  for _ in 0..100 {
    assert!(claimer.claim(1).is_err());
  }
  assert_eq!(claimer.claimed(), Seq(4), "100 failures moved the cursor");
}

#[test]
fn claiming_zero_succeeds_and_moves_nothing() {
  let consumers = GatingSet::new(cap(4), 1);
  let claimer = Claimer::new(&consumers);
  let _full = claimer.claim(4).unwrap();

  let empty = claimer.claim(0).expect("zero always fits, even on a full ring");
  assert!(empty.is_empty());
  assert_eq!(claimer.claimed(), Seq(4));
}

// ── claim_up_to ────────────────────────────────────────────────────────────

#[test]
fn claim_up_to_takes_what_is_there() {
  let consumers = GatingSet::new(cap(8), 1);
  let claimer = Claimer::new(&consumers);

  assert_eq!(claimer.claim_up_to(3).unwrap().len(), 3, "less than available");
  assert_eq!(claimer.claim_up_to(100).unwrap().len(), 5, "capped at what is left");
  assert_eq!(claimer.claim_up_to(1), Err(RingError::Full));
}

#[test]
fn claim_up_to_never_reports_batch_too_large() {
  // The one behavioural difference from `claim` is that an over-wide `max` is
  // a cap, not an error, because the caller asked for "up to".
  let consumers = GatingSet::new(cap(4), 1);
  let claimer = Claimer::new(&consumers);

  let claim = claimer.claim_up_to(usize::MAX).unwrap();
  assert_eq!(claim.len(), 4);
  assert_eq!(claimer.claim_up_to(usize::MAX), Err(RingError::Full), "full, not too large");
}

#[test]
fn claim_up_to_of_zero_is_full_not_an_empty_claim() {
  // A caller asking for at most zero slots gets `Full` rather than a
  // zero-length success, because `claim_up_to`'s contract is "as many as are
  // available, down to one". There is no partial success at zero to report.
  let consumers = GatingSet::new(cap(4), 1);
  assert_eq!(Claimer::new(&consumers).claim_up_to(0), Err(RingError::Full));
}

// ── the claimer's own readings ─────────────────────────────────────────────

#[test]
fn headroom_tracks_what_claiming_consumed() {
  let consumers = GatingSet::new(cap(8), 1);
  let claimer = Claimer::new(&consumers);

  for taken in 0..8 {
    assert_eq!(claimer.headroom(), 8 - taken);
    let _claim = claimer.claim(1).unwrap();
  }
  assert_eq!(claimer.headroom(), 0);
}

#[test]
fn the_claimer_exposes_the_gate_it_was_built_over() {
  let consumers = GatingSet::new(cap(8), 2);
  let claimer = Claimer::new(&consumers);

  assert_eq!(claimer.consumers().len(), 2);
  assert_eq!(claimer.consumers().capacity().get(), 8);
}

#[test]
fn the_producer_cursor_is_readable_and_starts_at_zero() {
  let consumers = GatingSet::new(cap(8), 1);
  let claimer = Claimer::new(&consumers);

  assert_eq!(claimer.cursor().load(Ordering::Acquire), Seq::ZERO);
  let _claim = claimer.claim(3).unwrap();
  assert_eq!(claimer.cursor().load(Ordering::Acquire), Seq(3));
}

#[test]
fn the_producer_cursor_occupies_its_own_cache_line() {
  // Every producer writes a `Claimer`, and the gating side reads it. If its
  // cursor shared a line with anything else, the crate's hottest contention
  // point would sit in the same line as something read on a different path.
  assert_eq!(core::mem::size_of::<ring_cursor::PaddedCursor>(), 64);
}

// ── exclusivity under contention ───────────────────────────────────────────

#[test]
fn no_two_producers_are_ever_granted_the_same_sequence() {
  const PRODUCERS: usize = 4;
  const PER_PRODUCER: usize = 2_000;
  const TOTAL: usize = PRODUCERS * PER_PRODUCER;

  // Capacity exceeds the total, so nothing ever gates. This test is about
  // exclusivity alone, and a producer blocking on a full ring would only
  // reduce the contention it is trying to create.
  let consumers = GatingSet::new(cap(16_384), 0);
  let claimer = Claimer::new(&consumers);

  let ranges: Vec<Vec<Claim>> = std::thread::scope(|scope| {
    let handles: Vec<_> = (0..PRODUCERS)
      .map(|_| {
        scope.spawn(|| {
          (0..PER_PRODUCER)
            .map(|_| claimer.claim(1).expect("capacity exceeds the total"))
            .collect()
        })
      })
      .collect();

    handles.into_iter().map(|h| h.join().expect("no producer panicked")).collect()
  });

  let mut every: Vec<Seq> = ranges.iter().flatten().flat_map(|c| c.sequences()).collect();
  assert_eq!(every.len(), TOTAL, "a producer lost a grant");

  every.sort_unstable();
  every.dedup();
  assert_eq!(every.len(), TOTAL, "two producers were granted the same sequence");

  // Contiguous from zero: no gap means no sequence was skipped either.
  assert_eq!(every.first().copied(), Some(Seq::ZERO));
  assert_eq!(every.last().copied(), Some(Seq(TOTAL as u64 - 1)));
  assert_eq!(claimer.claimed(), Seq(TOTAL as u64));
}

#[test]
fn claims_under_contention_lose_no_sequences() {
  // Multi-slot claims, where a CAS retry that recomputes the length but not
  // the start would produce a gap rather than an overlap. That is the same bug
  // seen from the other side, and a dedup check alone cannot see it.
  const PRODUCERS: usize = 4;
  const CLAIMS_EACH: usize = 500;
  const WIDTH: usize = 3;

  let consumers = GatingSet::new(cap(16_384), 0);
  let claimer = Claimer::new(&consumers);

  let ranges: Vec<Vec<Claim>> = std::thread::scope(|scope| {
    let handles: Vec<_> = (0..PRODUCERS)
      .map(|_| scope.spawn(|| (0..CLAIMS_EACH).map(|_| claimer.claim(WIDTH).expect("room")).collect()))
      .collect();

    handles.into_iter().map(|h| h.join().expect("no producer panicked")).collect()
  });

  let mut every: Vec<Seq> = ranges.iter().flatten().flat_map(|c| c.sequences()).collect();
  every.sort_unstable();

  let expected = PRODUCERS * CLAIMS_EACH * WIDTH;
  assert_eq!(every.len(), expected);
  for (position, seq) in every.iter().enumerate() {
    assert_eq!(*seq, Seq(position as u64), "gap or duplicate at position {position}");
  }
}

#[test]
fn claim_up_to_under_contention_loses_no_sequences_either() {
  // `claim_up_to` has its own CAS loop, and until this test existed nothing
  // ever made it retry. Every other test of it is single-threaded, so the
  // `Err( actual ) => current = actual` arm was unreached. That arm is where
  // the interesting bug lives. It must re-read `headroom` against the *new*
  // start, because the grant it computed was sized against the old one.
  //
  // A retry that kept the stale `granted` would hand out a range extending
  // past the gate. `no_grant_ever_passes_the_limit_under_contention` exists
  // to catch that defect for `claim`, and this test makes the same check for
  // the other constructor.
  const PRODUCERS: usize = 4;
  const CALLS_EACH: usize = 500;
  const MAX: usize = 3;

  let consumers = GatingSet::new(cap(16_384), 0);
  let claimer = Claimer::new(&consumers);

  let ranges: Vec<Vec<Claim>> = std::thread::scope(|scope| {
    let handles: Vec<_> = (0..PRODUCERS)
      .map(|_| scope.spawn(|| (0..CALLS_EACH).map(|_| claimer.claim_up_to(MAX).expect("room")).collect()))
      .collect();

    handles.into_iter().map(|h| h.join().expect("no producer panicked")).collect()
  });

  // Unlike the fixed-width test, lengths vary. `claim_up_to` grants what is
  // there, and near a gate boundary that is fewer than `MAX`. So the total is
  // not predictable, and the test asserts only the partition itself.
  let mut every: Vec<Seq> = ranges.iter().flatten().flat_map(|c| c.sequences()).collect();
  every.sort_unstable();

  assert!(!every.is_empty(), "every call was expected to succeed");
  for (position, seq) in every.iter().enumerate() {
    assert_eq!(*seq, Seq(position as u64), "gap or duplicate at position {position}");
  }
  assert_eq!(claimer.claimed(), Seq(every.len() as u64), "and the cursor agrees");

  for claim in ranges.iter().flatten() {
    assert!(claim.len() <= MAX, "granted {} past the requested maximum", claim.len());
    assert!(!claim.is_empty(), "a zero-length grant should have been Err( Full )");
  }
}

#[test]
fn each_producers_own_claims_stay_in_issue_order() {
  // The multi-producer claim feature's third clause. The test deliberately does
  // not assert global order across producers. Nothing promises it, and
  // asserting it would fail correct implementations.
  const PRODUCERS: usize = 3;
  const CLAIMS_EACH: usize = 1_000;

  let consumers = GatingSet::new(cap(8_192), 0);
  let claimer = Claimer::new(&consumers);

  let per_producer: Vec<Vec<Claim>> = std::thread::scope(|scope| {
    let handles: Vec<_> = (0..PRODUCERS)
      .map(|_| scope.spawn(|| (0..CLAIMS_EACH).map(|_| claimer.claim(1).expect("room")).collect()))
      .collect();

    handles.into_iter().map(|h| h.join().expect("no producer panicked")).collect()
  });

  for (producer, claims) in per_producer.iter().enumerate() {
    for pair in claims.windows(2) {
      assert!(
        pair[0].start() < pair[1].start(),
        "producer {producer} received {:?} before {:?}",
        pair[1],
        pair[0]
      );
    }
  }
}

#[test]
fn no_grant_ever_passes_the_limit_under_contention() {
  // The module documentation's argument for the CAS loop, as a test. The gate
  // is tight (4 slots) and contention is high, so a check-then-unconditionally-
  // advance implementation grants a range past the limit within a few
  // iterations.
  //
  // The test reads the bound *after* each grant, and that is what makes the
  // assertion both sound and able to fail. Sound: the limit only ever rises,
  // since a consumer only advances, so a claim granted legitimately against
  // some earlier limit is still within any later one. A correct implementation
  // can never trip this. Able to fail: a `fetch_add` grant overshoots the limit
  // that held at the moment it was made, and unless the consumer happens to
  // sweep past it before the read, it overshoots the later one too.
  //
  // Reading the limit *before* the claim would give the opposite, an assertion
  // that fails on correct code whenever the consumer advances in between.
  const CAPACITY: usize = 4;
  const RELEASES: u64 = 4_000;

  let consumers = GatingSet::new(cap(CAPACITY), 1);
  let claimer = Claimer::new(&consumers);

  std::thread::scope(|scope| {
    // A consumer releasing slots steadily, so producers keep finding room and
    // keep racing for it rather than all parking on a permanently full ring.
    scope.spawn(|| {
      for released in 1..=RELEASES {
        consumers.cursor(0).unwrap().store(Seq(released), Ordering::Release);
        // Yielding after each release keeps the gate tight for the whole run
        // rather than only at the tail. Without it the consumer finishes in a
        // burst, races far ahead of the producers, and a limit that has already
        // moved past an overrunning grant absorbs it. The defect then shows
        // only in the last few iterations.
        std::thread::yield_now();
      }
    });

    let handles: Vec<_> = (0..3)
      .map(|_| {
        scope.spawn(|| {
          let mut mine = Vec::new();
          for _ in 0..3_000 {
            if let Ok(claim) = claimer.claim(1) {
              let limit = consumers.limit().expect("one consumer");
              assert!(
                claim.end() <= limit,
                "granted {claim:?}, which ends past the limit {limit:?} that held after it"
              );
              mine.push(claim);
            }
          }
          mine
        })
      })
      .collect();

    let all: Vec<Claim> = handles.into_iter().flat_map(|h| h.join().unwrap()).collect();

    // Whatever was granted, it was granted exclusively.
    let mut every: Vec<Seq> = all.iter().flat_map(|c| c.sequences()).collect();
    let granted = every.len();
    every.sort_unstable();
    every.dedup();
    assert_eq!(every.len(), granted, "two producers were granted the same sequence");
    assert!(granted > 0, "no grant was made at all, so nothing was actually checked");
  });
}

/// Writing through `Claimer::cursor()` defeats both properties this crate
/// exists to hold, in safe code, from outside the crate.
///
/// An earlier finding recorded that `cursor()` returns a `&PaddedCursor`, that
/// `PaddedCursor : SeqCell` is a public trait, and that every `SeqCell` method
/// takes `&self`. Any caller holding the accessor's result can therefore reach
/// `store` and `fetch_add`. This test is that finding as executable evidence.
/// It asserts the hazard is *real*, not that the crate is broken, so it is
/// expected to keep passing.
///
/// It pins the boundary the manual `§ C2` check cannot see. `§ C2` greps
/// this crate's own source for exactly two atomic mutations, both
/// `compare_exchange`, with no `fetch_add` and no `store`. That check passes,
/// correctly, while the accessor exports both to every caller. Narrowing
/// `cursor()` to an address or to a read-only view would stop this test
/// compiling, which is the signal that the gap closed.
#[test]
fn writing_through_the_cursor_accessor_defeats_the_gate() {
  let consumers = GatingSet::new(cap(4), 1);
  let claimer = Claimer::new(&consumers);

  let first = claimer.claim(4).expect("an empty ring admits a full-capacity claim");
  assert_eq!(claimer.claim(1), Err(RingError::Full), "and then the gate refuses");

  // Past the gate that just refused, with no `unsafe` and no crate-private access.
  let stolen = claimer.cursor().fetch_add(4, Ordering::AcqRel);
  assert_eq!(stolen, Seq(4), "granted the sequence the gate had just withheld");

  // And back, so the same four sequences can be handed out a second time.
  claimer.cursor().store(Seq::ZERO, Ordering::Release);
  let again = claimer.claim(4).expect("a rewound cursor makes the gate admit again");

  assert!(
    first.overlaps(again),
    "two live claims cover the same sequences: {first:?} and {again:?}"
  );
}

/// `Claim`'s two predicates answer at compile time.
///
/// An earlier finding recorded that `contains` and `overlaps` were the two of
/// `Claim`'s seven methods that were not `const`, and that nothing about them
/// required it. Both compared `Seq` values through `PartialOrd`, which a
/// `const fn` cannot call, when the raw `u64` comparison underneath is
/// const-callable. `sequences`, three lines above `overlaps`, already showed
/// this by reaching through the newtype's public field.
///
/// They now compare `.0` directly. This test pins that at compile time. If
/// either method loses `const`, these `const` bindings fail to build, which
/// no runtime assertion could catch.
#[test]
fn claim_predicates_answer_in_a_const_context() {
  const A: Claim = Claim::new(Seq(4), 4);
  const B: Claim = Claim::new(Seq(6), 4);
  const C: Claim = Claim::new(Seq(8), 4);

  // The compiler evaluates these, not the runner. Each one fails the *build*
  // if its predicate loses `const` or starts answering differently.
  // That is the whole point of the test, and it has already happened by the
  // time the runner reports a pass.
  const { assert!(A.contains(Seq(7)), "Seq(7) is inside Seq(4)..Seq(8)") }
  const { assert!(!A.contains(Seq(8)), "half-open: a range excludes its own end") }
  const { assert!(A.overlaps(B), "Seq(4)..Seq(8) and Seq(6)..Seq(10) share two") }
  const { assert!(!A.overlaps(C), "Seq(4)..Seq(8) ends where Seq(8)..Seq(12) begins") }

  // The same four questions at run time, through values the compiler cannot
  // fold away, so removing the `const` blocks above would leave the behaviour
  // still covered rather than leaving this test vacuous.
  let start = Seq(std::hint::black_box(4u64));
  let a = Claim::new(start, 4);

  assert!(a.contains(Seq(7)));
  assert!(!a.contains(Seq(8)));
  assert!(a.overlaps(B));
  assert!(!a.overlaps(C));
}

/// The four routes by which a bound `Claim` fails to reach a `publish`, all
/// compiled here by name.
///
/// None of the four warns. The `#[ must_use ]` on `Claim` fires on an unused
/// *value*, and every one of these uses the value before losing it. That claim
/// used to rest on a recipe that ran `cargo build` over the workspace and
/// counted warnings, which compiled none of the four and so measured nothing.
/// This file compiles them instead, so the crate's own `-D warnings` build is
/// the measurement. If any route did warn, this file would not build.
#[test]
fn none_of_the_four_abandonment_routes_warns() {
  let consumers = GatingSet::new(cap(8), 1);
  let claimer = Claimer::new(&consumers);

  // 1 — explicitly discarded. `let _ =` is the idiom `must_use` reads as consent.
  let _ = claimer.claim(1);

  // 2 — an early return between the claim and the write.
  fn early_return(claimer: &Claimer<'_>) -> Result<Seq, RingError> {
    let claim = claimer.claim(1)?;
    if claim.len() == 1 {
      return Err(RingError::Full);
    }
    Ok(claim.end())
  }
  assert_eq!(early_return(&claimer), Err(RingError::Full), "route 2 lost its claim");

  // 3 — an unwind between the claim and the write.
  let hook = std::panic::take_hook();
  std::panic::set_hook(Box::new(|_| {}));
  let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
    let claim = claimer.claim(1).unwrap();
    assert!(claim.len() > 99, "forced");
  }));
  std::panic::set_hook(hook);
  assert!(unwound.is_err(), "route 3 must actually unwind");

  // 4 — a loop that skips the publish.
  let mut reached = 0;
  for i in 0..2 {
    let claim = claimer.claim(1).unwrap();
    if i == 0 {
      continue;
    }
    reached += claim.len();
  }
  assert_eq!(reached, 1, "route 4 dropped the first iteration's claim");

  // Four sequences claimed across the four routes, none of them published,
  // and the cursor has no idea.
  assert_eq!(claimer.claimed(), Seq(5), "five claims, five sequences, zero publications");
}
