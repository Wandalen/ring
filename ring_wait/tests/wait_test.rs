//! `ring_wait` — the four wait strategies and the one loop they share.
//!
//! This file carries the reached-test for
//! `docs/feature/173_wait_kind_and_strategies.md`, stated in
//! `ring/bench_harness/docs/acceptance/001_feature_reached_tests.md` as three
//! clauses: `WaitKind` has exactly the four discriminants `Spin`, `Yield`,
//! `Park`, `None` in `ring_types`; this crate supplies one handler per
//! discriminant; and `WaitKind::None` returns without blocking when the ring is
//! empty, asserted by a bounded-time test.
//!
//! ## The first clause is asserted here as well as in `ring_types`
//!
//! Deliberately, and it is not duplication. `ring_types/tests/types_test.rs`
//! asserts the enum's shape as a fact about that crate. This file asserts it as
//! a *precondition of the second clause*: "one handler per discriminant" is
//! meaningless without knowing how many discriminants there are, and a fifth
//! variant added later would leave this crate silently one handler short. The
//! two assertions would survive each other's deletion and mean different
//! things.
//!
//! ## What "bounded time" means here, and why it is not a timeout
//!
//! The third clause is the only one with a stopwatch in it, and a stopwatch in
//! a test is usually a flake waiting for a loaded CI box. It is used here
//! because the property genuinely is temporal — `None` must not block — and
//! because the bound is chosen to be absurd rather than tight: a whole second
//! for an operation whose honest cost is a single predicate evaluation. A run
//! that exceeds it has not been slow, it has blocked.
//!
//! The non-temporal half of the same property is asserted alongside it, and is
//! the assertion that would actually catch a regression: `None` evaluates the
//! predicate **exactly once**. A `None` that looped 1024 times before giving up
//! would still finish inside a second on any machine, and only the count says
//! so.

// Ordinary tests, compiled out under `--cfg loom`. That cfg swaps
// `ring_atomic`'s atomics for loom's instrumented ones across the whole
// family, and those panic the moment they are touched outside a
// `loom::model` closure — so without this gate a family-wide loom run dies
// here instead of reaching the models in `ring_spsc`, `ring_mpsc`,
// `ring_publish` and `ring_testkit`.
#![ cfg( not( loom ) ) ]

use core::sync::atomic::Ordering;
use ring_cursor::{ CursorPair, SeqCell };
use ring_types::{ Capacity, RingError, Seq, WaitKind };
use ring_wait::{ escalation_hint, for_data, for_space, pause, wait, wait_until, DEFAULT_SPINS };

fn cap( slots : usize ) -> Capacity
{
  Capacity::new( slots ).expect( "test capacities are powers of two" )
}

// ── feature 173, clause 1: the discriminant set ────────────────────────────

#[ test ]
fn there_are_exactly_four_wait_kinds()
{
  assert_eq!( WaitKind::ALL.len(), 4 );
  assert_eq!(
    WaitKind::ALL,
    [ WaitKind::Spin, WaitKind::Yield, WaitKind::Park, WaitKind::None ],
    "in discriminant order — a reordering changes what a serialized config means"
  );
}

// ── feature 173, clause 2: one handler per discriminant ────────────────────

#[ test ]
fn every_discriminant_has_a_handler_that_runs()
{
  // The handler set is `pause`'s match. An unhandled variant is a compile
  // error there, so what this asserts is the weaker but non-trivial claim
  // that each arm actually returns rather than diverging.
  for kind in WaitKind::ALL
  {
    let keep_going = pause( kind, 0 );
    assert_eq!(
      keep_going,
      !kind.is_non_blocking(),
      "{kind:?} must keep going iff it is a blocking strategy"
    );
  }
}

#[ test ]
fn every_discriminant_drives_the_same_loop_to_the_same_answer()
{
  // The behavioural difference between the four is confined to the pause. The
  // loop's result must not depend on which strategy ran it.
  for kind in WaitKind::ALL
  {
    assert_eq!( wait_until( kind, 4, || true ), Ok( 0 ), "{kind:?} on a ready predicate" );
    assert_eq!(
      wait_until( kind, 4, || false ),
      Err( RingError::Empty ),
      "{kind:?} on a predicate that never becomes ready"
    );
  }
}

#[ test ]
fn exactly_one_discriminant_is_non_blocking()
{
  let non_blocking : Vec< _ > = WaitKind::ALL.into_iter().filter( | k | !pause( *k, 0 ) ).collect();

  assert_eq!( non_blocking, vec![ WaitKind::None ], "the tick path has exactly one option" );
}

// ── feature 173, clause 3: None does not block ─────────────────────────────

#[ test ]
fn none_returns_immediately_from_an_empty_ring()
{
  let pair = CursorPair::new( cap( 8 ) );
  let started = std::time::Instant::now();

  let outcome = for_data( &pair, 1, WaitKind::None, DEFAULT_SPINS );

  assert_eq!( outcome, Err( RingError::Empty ), "nothing was published" );
  assert!(
    started.elapsed() < std::time::Duration::from_secs( 1 ),
    "None blocked for {:?} on an empty ring",
    started.elapsed()
  );
}

#[ test ]
fn none_evaluates_the_predicate_exactly_once()
{
  // The assertion that would actually catch a regression. A `None` that spun
  // the full DEFAULT_SPINS budget before giving up still finishes instantly
  // and passes the stopwatch above; only the count distinguishes "did not
  // block" from "looped fast".
  let mut looks = 0;
  let outcome = wait_until( WaitKind::None, DEFAULT_SPINS, ||
  {
    looks += 1;
    false
  } );

  assert_eq!( outcome, Err( RingError::Empty ) );
  assert_eq!( looks, 1, "None looked {looks} times at a budget of {DEFAULT_SPINS}" );
}

#[ test ]
fn none_still_looks_once_and_can_succeed()
{
  // "Returns immediately" must not degrade into "returns without looking".
  // A `None` that never evaluated the predicate would pass every timing
  // assertion and always report empty.
  let pair = CursorPair::new( cap( 8 ) );
  pair.producer().store( Seq( 2 ), Ordering::Release );

  assert_eq!( for_data( &pair, 2, WaitKind::None, DEFAULT_SPINS ), Ok( 0 ) );
  assert_eq!( for_space( &pair, WaitKind::None, DEFAULT_SPINS ), Ok( 0 ) );
}

#[ test ]
fn the_three_blocking_kinds_do_loop()
{
  // The mirror of the previous test: if `None`'s single look were the
  // behaviour of all four, the enum would carry no information.
  for kind in [ WaitKind::Spin, WaitKind::Yield, WaitKind::Park ]
  {
    let mut looks = 0;
    let _ = wait_until( kind, 3, ||
    {
      looks += 1;
      false
    } );

    assert_eq!( looks, 3, "{kind:?} gave up after {looks} of 3 attempts" );
  }
}

// ── the loop's own contract ────────────────────────────────────────────────

#[ test ]
fn a_wait_reports_how_many_attempts_it_took()
{
  let mut looks = 0;
  let attempts = wait_until( WaitKind::Spin, 16, ||
  {
    looks += 1;
    looks == 5
  } )
  .expect( "ready on the fifth look" );

  assert_eq!( attempts, 4, "zero-based: ready on look 5 is 4 pauses" );
}

#[ test ]
fn a_wait_stops_asking_the_moment_it_is_ready()
{
  let mut looks = 0;
  let _ = wait_until( WaitKind::Spin, 100, ||
  {
    looks += 1;
    looks == 2
  } );

  assert_eq!( looks, 2, "asked {looks} times after the answer arrived on look 2" );
}

#[ test ]
fn a_zero_budget_still_looks_once()
{
  // A budget of zero is a caller bug, and the useful behaviour is one look
  // rather than an unconditional failure — the alternative reports "not
  // ready" about a ring nobody ever asked.
  let mut looks = 0;
  let outcome = wait_until( WaitKind::Spin, 0, ||
  {
    looks += 1;
    true
  } );

  assert_eq!( outcome, Ok( 0 ) );
  assert_eq!( looks, 1 );
}

#[ test ]
fn the_budget_is_a_count_and_is_honoured_exactly()
{
  for budget in 1..8usize
  {
    let mut looks = 0;
    let _ = wait_until( WaitKind::Spin, budget, ||
    {
      looks += 1;
      false
    } );

    assert_eq!( looks, budget, "budget {budget} produced {looks} looks" );
  }
}

#[ test ]
fn the_default_budget_is_the_documented_constant()
{
  let mut looks = 0;
  let _ = wait( WaitKind::Spin, ||
  {
    looks += 1;
    false
  } );

  assert_eq!( looks, DEFAULT_SPINS );
  assert_eq!( DEFAULT_SPINS, 1024 );
}

// ── the two named questions ────────────────────────────────────────────────

#[ test ]
fn space_and_data_fail_with_different_errors()
{
  // The one place the two wrappers differ, and it matters: a producer handed
  // `Empty` reads it as "nothing to do" rather than "back-pressure", and
  // stops producing.
  let pair = CursorPair::new( cap( 2 ) );
  pair.producer().store( Seq( 2 ), Ordering::Release );

  assert_eq!( for_space( &pair, WaitKind::None, 1 ), Err( RingError::Full ) );

  let empty = CursorPair::new( cap( 2 ) );
  assert_eq!( for_data( &empty, 1, WaitKind::None, 1 ), Err( RingError::Empty ) );
}

#[ test ]
fn neither_failure_is_a_configuration_error()
{
  // Both are back-pressure, so both must be retryable. `RingError` splits
  // configuration errors from transient ones, and a retry loop that could not
  // tell them apart would spin forever on a bad capacity.
  assert!( !RingError::Full.is_configuration() );
  assert!( !RingError::Empty.is_configuration() );
}

#[ test ]
fn for_data_counts_pending_and_not_capacity()
{
  let pair = CursorPair::new( cap( 4 ) );
  pair.producer().store( Seq( 3 ), Ordering::Release );

  for wanted in 1..=3u64
  {
    assert!( for_data( &pair, wanted, WaitKind::None, 1 ).is_ok(), "{wanted} of 3 pending" );
  }
  assert!( for_data( &pair, 4, WaitKind::None, 1 ).is_err(), "4 of 3 pending" );
}

#[ test ]
fn for_space_tracks_the_consumer_as_well_as_the_producer()
{
  let pair = CursorPair::new( cap( 4 ) );
  pair.producer().store( Seq( 4 ), Ordering::Release );
  assert!( for_space( &pair, WaitKind::None, 1 ).is_err() );

  pair.consumer().store( Seq( 1 ), Ordering::Release );
  assert!( for_space( &pair, WaitKind::None, 1 ).is_ok(), "the consumer released a slot" );
}

#[ test ]
fn a_blocking_wait_succeeds_when_another_thread_publishes()
{
  // The shape the strategies exist for. Not a timing assertion: the waiter's
  // budget is large enough that only a genuinely broken loop fails it.
  let pair = CursorPair::new( cap( 8 ) );

  std::thread::scope( | scope |
  {
    scope.spawn( ||
    {
      std::thread::sleep( std::time::Duration::from_millis( 5 ) );
      pair.producer().store( Seq( 1 ), Ordering::Release );
    } );

    assert!(
      for_data( &pair, 1, WaitKind::Yield, 100_000 ).is_ok(),
      "the waiter never saw the publication"
    );
  } );
}

// ── escalation ─────────────────────────────────────────────────────────────

#[ test ]
fn escalation_walks_from_cheapest_latency_to_cheapest_cpu()
{
  assert_eq!( escalation_hint( WaitKind::Spin ), Some( WaitKind::Yield ) );
  assert_eq!( escalation_hint( WaitKind::Yield ), Some( WaitKind::Park ) );
  assert_eq!( escalation_hint( WaitKind::Park ), None );
}

#[ test ]
fn none_never_escalates()
{
  // Escalating out of `None` would put a blocking strategy on the tick path,
  // which is the one thing feature 173 names `None` to prevent.
  assert_eq!( escalation_hint( WaitKind::None ), None );
}

#[ test ]
fn escalation_terminates_from_every_starting_point()
{
  for start in WaitKind::ALL
  {
    let mut kind = start;
    let mut steps = 0;

    while let Some( next ) = escalation_hint( kind )
    {
      kind = next;
      steps += 1;
      assert!( steps <= WaitKind::ALL.len(), "escalation from {start:?} cycles" );
    }
  }
}

// ── the pause itself ───────────────────────────────────────────────────────

#[ test ]
fn the_spin_pause_varies_with_the_attempt_and_always_continues()
{
  // The attempt index feeds a backoff. What must hold regardless is that spin
  // never reports "stop" — only `None` does that.
  for attempt in 0..32
  {
    assert!( pause( WaitKind::Spin, attempt ), "spin stopped at attempt {attempt}" );
  }
}

#[ test ]
fn none_reports_stop_at_every_attempt_index()
{
  for attempt in 0..8
  {
    assert!( !pause( WaitKind::None, attempt ), "None continued at attempt {attempt}" );
  }
}

#[ test ]
fn a_zero_count_is_already_satisfied_and_spends_no_attempt()
{
  // `for_data`'s predicate is `pending() >= count` over a `u64`, so a count of
  // zero is a tautology rather than a wait. It is the one input where
  // `for_data` cannot fail: the same empty ring that answers `Err( Empty )` for
  // one item answers `Ok( 0 )` for none. The zero inside `Ok` is the
  // load-bearing part — it is the attempt count, so it says the predicate was
  // satisfied before any strategy got to pause.
  let empty = CursorPair::new( cap( 8 ) );

  assert_eq!( for_data( &empty, 1, WaitKind::None, 1 ), Err( RingError::Empty ), "one of none pending" );

  for kind in WaitKind::ALL
  {
    assert_eq!( for_data( &empty, 0, kind, 4 ), Ok( 0 ), "{kind:?} spent an attempt on a count of zero" );
  }
}
