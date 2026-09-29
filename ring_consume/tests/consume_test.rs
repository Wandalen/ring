//! `ring_consume` — what a consumer may read, and saying it has.
//!
//! The consumer half of
//! `docs/feature/170_claim_publish_available_commit_handshake.md`. The
//! feature's own reached-test wires all four operations together in
//! `ring_publish/tests/handshake_test.rs`; this file covers `available` and
//! `commit` on their own, where the boundary conditions are cheap to state
//! exhaustively and expensive to reason about in a four-crate integration.
//!
//! ## The two commits that must be refused
//!
//! Both refusals free slots that were not read, and both are single-line
//! mistakes:
//!
//! - Committing **past** what is available tells the producer that
//!   claimed-but-unread slots are free. The producer believes it, because the
//!   consumer cursor *is* the signal.
//! - Committing **backwards** re-reads slots the producer has already been
//!   cleared to reuse.
//!
//! Neither is caught by a test that only ever commits exactly what `available`
//! returned, which is what a natural happy-path suite does throughout.

// Ordinary tests, compiled out under `--cfg loom`. That cfg swaps
// `ring_atomic`'s atomics for loom's instrumented ones across the whole
// family, and those panic the moment they are touched outside a
// `loom::model` closure — so without this gate a family-wide loom run dies
// here instead of reaching the models in `ring_spsc`, `ring_mpsc`,
// `ring_publish` and `ring_testkit`.
#![ cfg( not( loom ) ) ]

use core::sync::atomic::Ordering;
use ring_barrier::Barrier;
use ring_consume::{ Available, Consumer };
use ring_cursor::{ PaddedCursor, SeqCell };
use ring_types::{ RingError, Seq };

/// One published cursor sitting at `at`.
fn published_at( at : u64 ) -> [ PaddedCursor; 1 ]
{
  [ PaddedCursor::new( Seq( at ) ) ]
}

/// `count` published cursors, all at zero.
fn published_cursors( count : usize ) -> Vec< PaddedCursor >
{
  ( 0..count ).map( | _ | PaddedCursor::default() ).collect()
}

// ── Available, the run itself ──────────────────────────────────────────────

#[ test ]
fn an_available_run_is_half_open()
{
  let run = Available::new( Seq( 3 ), 4 );

  assert_eq!( run.start(), Seq( 3 ) );
  assert_eq!( run.end(), Seq( 7 ) );
  assert_eq!( run.len(), 4 );
  assert!( !run.is_empty() );
  assert_eq!( run.sequences().count(), 4 );
  assert_eq!( run.sequences().last(), Some( Seq( 6 ) ), "end is exclusive" );
}

#[ test ]
fn an_empty_run_starts_and_ends_in_the_same_place()
{
  let run = Available::new( Seq( 9 ), 0 );

  assert!( run.is_empty() );
  assert_eq!( run.start(), run.end() );
  assert_eq!( run.sequences().count(), 0 );
}

#[ test ]
fn sequences_yields_exactly_the_run()
{
  for start in 0..6u64
  {
    for len in 0..6u64
    {
      let run = Available::new( Seq( start ), len );
      let yielded : Vec< u64 > = run.sequences().map( | s | s.0 ).collect();

      assert_eq!( yielded, ( start..start + len ).collect::< Vec< _ > >(), "start {start}, len {len}" );
    }
  }
}

// ── available ──────────────────────────────────────────────────────────────

#[ test ]
fn nothing_is_available_before_anything_is_published()
{
  let published = published_at( 0 );
  let position = PaddedCursor::default();
  let consumer = Consumer::new( &position, Barrier::over( &published ) );

  assert!( consumer.available().is_empty() );
  assert_eq!( consumer.available().start(), Seq::ZERO );
  assert_eq!( consumer.position(), Seq::ZERO );
}

#[ test ]
fn available_grows_as_publication_advances()
{
  let published = published_at( 0 );
  let position = PaddedCursor::default();
  let consumer = Consumer::new( &position, Barrier::over( &published ) );

  for frontier in 0..32u64
  {
    published[ 0 ].store( Seq( frontier ), Ordering::Release );
    assert_eq!( consumer.available().len(), frontier, "published through {frontier}" );
    assert_eq!( consumer.available().start(), Seq::ZERO, "and always from where we are" );
  }
}

#[ test ]
fn available_shrinks_as_the_consumer_commits()
{
  let published = published_at( 10 );
  let position = PaddedCursor::default();
  let consumer = Consumer::new( &position, Barrier::over( &published ) );

  for committed in 0..=10u64
  {
    assert_eq!( consumer.available().len(), 10 - committed, "after committing {committed}" );
    if committed < 10
    {
      consumer.commit( Seq( committed + 1 ) ).expect( "one more is always available here" );
    }
  }
  assert!( consumer.available().is_empty() );
}

#[ test ]
fn available_starts_where_the_consumer_stands()
{
  let published = published_at( 20 );
  let position = PaddedCursor::default();
  let consumer = Consumer::new( &position, Barrier::over( &published ) );
  consumer.commit( Seq( 7 ) ).unwrap();

  let run = consumer.available();
  assert_eq!( run.start(), Seq( 7 ) );
  assert_eq!( run.end(), Seq( 20 ) );
  assert_eq!( run.len(), 13 );
}

#[ test ]
fn a_consumer_with_no_dependencies_never_has_anything_available()
{
  // An empty barrier means nothing has been published, not "everything" —
  // the asymmetry with `ring_gating` that `ring_barrier`'s own suite asserts,
  // carried through to the operation a caller actually uses.
  let position = PaddedCursor::default();
  let consumer = Consumer::new( &position, Barrier::over( &[] ) );

  assert!( consumer.available().is_empty() );
  assert_eq!( consumer.commit_available(), Seq::ZERO );
}

#[ test ]
fn available_is_bounded_by_the_slowest_of_several_dependencies()
{
  let published = published_cursors( 3 );
  for ( cursor, at ) in published.iter().zip( [ 12u64, 4, 30 ] )
  {
    cursor.store( Seq( at ), Ordering::Release );
  }

  let position = PaddedCursor::default();
  let consumer = Consumer::new( &position, Barrier::over( &published ) );
  assert_eq!( consumer.available().len(), 4, "the slowest dependency bounds it" );
}

#[ test ]
fn available_up_to_caps_without_changing_the_start()
{
  let published = published_at( 9 );
  let position = PaddedCursor::default();
  let consumer = Consumer::new( &position, Barrier::over( &published ) );

  for max in 0..12u64
  {
    let run = consumer.available_up_to( max );
    assert_eq!( run.start(), Seq::ZERO, "max {max} moved the start" );
    assert_eq!( run.len(), max.min( 9 ), "max {max}" );
  }
}

#[ test ]
fn available_up_to_zero_is_empty_not_everything()
{
  let published = published_at( 100 );
  let position = PaddedCursor::default();
  let consumer = Consumer::new( &position, Barrier::over( &published ) );

  assert!( consumer.available_up_to( 0 ).is_empty() );
}

// ── commit ─────────────────────────────────────────────────────────────────

#[ test ]
fn committing_exactly_what_is_available_succeeds()
{
  let published = published_at( 5 );
  let position = PaddedCursor::default();
  let consumer = Consumer::new( &position, Barrier::over( &published ) );

  assert_eq!( consumer.commit( Seq( 5 ) ), Ok( Seq( 5 ) ) );
  assert_eq!( consumer.position(), Seq( 5 ) );
}

#[ test ]
fn committing_past_what_is_available_is_refused_and_moves_nothing()
{
  let published = published_at( 4 );
  let position = PaddedCursor::default();
  let consumer = Consumer::new( &position, Barrier::over( &published ) );

  for beyond in 5..20u64
  {
    assert_eq!( consumer.commit( Seq( beyond ) ), Err( RingError::Empty ), "committed to {beyond}" );
    assert_eq!( consumer.position(), Seq::ZERO, "a refused commit still moved the cursor" );
  }
}

#[ test ]
fn committing_backwards_is_refused_and_moves_nothing()
{
  let published = published_at( 20 );
  let position = PaddedCursor::default();
  let consumer = Consumer::new( &position, Barrier::over( &published ) );
  consumer.commit( Seq( 10 ) ).unwrap();

  for behind in 0..10u64
  {
    assert_eq!( consumer.commit( Seq( behind ) ), Err( RingError::Empty ), "committed back to {behind}" );
    assert_eq!( consumer.position(), Seq( 10 ), "a refused commit rewound the cursor" );
  }
}

#[ test ]
fn committing_where_you_already_are_is_allowed_and_is_a_no_op()
{
  // Not an error: a drain loop that read zero items and commits its position
  // is doing nothing wrong, and making it an error would put a special case in
  // every caller.
  let published = published_at( 6 );
  let position = PaddedCursor::default();
  let consumer = Consumer::new( &position, Barrier::over( &published ) );
  consumer.commit( Seq( 3 ) ).unwrap();

  assert_eq!( consumer.commit( Seq( 3 ) ), Ok( Seq( 3 ) ) );
  assert_eq!( consumer.position(), Seq( 3 ) );
}

#[ test ]
fn the_accepted_commits_are_exactly_the_available_range_inclusive_of_both_ends()
{
  // The full boundary map, in one sweep: for every published frontier and every
  // consumer position, exactly the values in `position..=frontier` are accepted.
  const FRONTIER : u64 = 8;
  let published = published_at( FRONTIER );

  for position in 0..=FRONTIER
  {
    for candidate in 0..FRONTIER + 4
    {
      // A fresh consumer per case rather than one per row: an accepted commit
      // moves the position, and reusing the consumer would silently change
      // which case the next iteration is actually testing.
      let position_cursor = PaddedCursor::default();
      let consumer = Consumer::new( &position_cursor, Barrier::over( &published ) );
      consumer.commit( Seq( position ) ).expect( "reachable in one step from zero" );

      let accepted = consumer.commit( Seq( candidate ) ).is_ok();
      let should_accept = candidate >= position && candidate <= FRONTIER;
      assert_eq!( accepted, should_accept, "at {position}, committing {candidate}" );

      let expected = if accepted { Seq( candidate ) } else { Seq( position ) };
      assert_eq!( consumer.position(), expected, "at {position}, committing {candidate}" );
    }
  }
}

#[ test ]
fn commit_available_takes_everything_and_reports_where_it_reached()
{
  let published = published_at( 12 );
  let position = PaddedCursor::default();
  let consumer = Consumer::new( &position, Barrier::over( &published ) );

  assert_eq!( consumer.commit_available(), Seq( 12 ) );
  assert_eq!( consumer.position(), Seq( 12 ) );
  assert!( consumer.available().is_empty() );

  // Idempotent when there is nothing new.
  assert_eq!( consumer.commit_available(), Seq( 12 ) );
}

#[ test ]
fn the_consumer_cursor_is_what_the_producer_would_gate_on()
{
  // A commit must land in the cursor the producer holds, not in one the
  // consumer kept to itself — a producer gating on a position that never moves
  // deadlocks after one lap. Read through `position` rather than through
  // `consumer.cursor()`, because `position` is what a producer actually has.
  let published = published_at( 7 );
  let position = PaddedCursor::default();
  let consumer = Consumer::new( &position, Barrier::over( &published ) );

  assert_eq!( position.load( Ordering::Acquire ), Seq::ZERO );
  consumer.commit( Seq( 4 ) ).unwrap();
  assert_eq!( position.load( Ordering::Acquire ), Seq( 4 ), "the producer sees the commit" );
  assert!( core::ptr::eq( consumer.cursor(), &position ) );
}

#[ test ]
fn the_consumer_exposes_the_barrier_it_was_built_over()
{
  let published = published_cursors( 2 );
  let position = PaddedCursor::default();
  let consumer = Consumer::new( &position, Barrier::over( &published ) );

  assert_eq!( consumer.barrier().len(), 2 );
  assert_eq!( consumer.barrier().frontier(), Some( Seq::ZERO ) );
}

// ── under a live producer ──────────────────────────────────────────────────

#[ test ]
fn a_consumer_never_reads_past_what_was_published()
{
  // The safety property, with a real writer moving the frontier underneath.
  // `available` may under-report — a publication that landed after the read is
  // simply not yet seen — but it must never over-report, because the caller
  // reads slots on the answer.
  const TOTAL : u64 = 4_000;
  let published = published_at( 0 );
  let position = PaddedCursor::default();
  let consumer = Consumer::new( &position, Barrier::over( &published ) );

  std::thread::scope( | scope |
  {
    scope.spawn( ||
    {
      for frontier in 1..=TOTAL
      {
        published[ 0 ].store( Seq( frontier ), Ordering::Release );
      }
    } );

    let mut drained = 0u64;
    while drained < TOTAL
    {
      let run = consumer.available();
      assert!( run.end().0 <= TOTAL, "available reached {:?}, past everything ever published", run.end() );

      if !run.is_empty()
      {
        consumer.commit( run.end() ).expect( "committing exactly what was offered" );
        drained = run.end().0;
      }
    }

    assert_eq!( consumer.position(), Seq( TOTAL ) );
  } );
}

#[ test ]
fn every_sequence_is_offered_exactly_once_across_a_full_drain()
{
  // A drain loop that commits what it was offered must see each sequence once:
  // twice means the cursor rewound, never means a run was skipped.
  const TOTAL : u64 = 2_000;
  let published = published_at( 0 );
  let position = PaddedCursor::default();
  let consumer = Consumer::new( &position, Barrier::over( &published ) );

  let seen = std::thread::scope( | scope | -> Vec< Seq >
  {
    scope.spawn( ||
    {
      for frontier in 1..=TOTAL
      {
        published[ 0 ].store( Seq( frontier ), Ordering::Release );
      }
    } );

    let mut seen = Vec::with_capacity( TOTAL as usize );
    while ( seen.len() as u64 ) < TOTAL
    {
      let run = consumer.available();
      seen.extend( run.sequences() );
      if !run.is_empty()
      {
        consumer.commit( run.end() ).unwrap();
      }
    }
    seen
  } );

  assert_eq!( seen.len(), TOTAL as usize );
  for ( position, seq ) in seen.iter().enumerate()
  {
    assert_eq!( *seq, Seq( position as u64 ), "gap or repeat at position {position}" );
  }
}
