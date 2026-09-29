//! The claim → publish → available → commit handshake, end to end.
//!
//! This is the reached-test for
//! `docs/feature/170_claim_publish_available_commit_handshake.md`, stated in
//! `ring/bench_harness/docs/acceptance/001_feature_reached_tests.md` as: a
//! slot claimed but not published is never returned by `available()`; after
//! publish it is; `commit()` advances the consumer cursor and never past
//! `available()` — asserted over every interleaving of one claim and one drain
//! under `loom`.
//!
//! It lives in `ring_publish` rather than in any of the other three crates
//! because publication is the moment the other three become observable
//! together: before it, a claim is invisible; after it, the consumer's whole
//! contract is decided.
//!
//! ## Why the wiring below is the test, as much as the assertions are
//!
//! Four crates, four cursors, and only two of them shared:
//!
//! - the **claimed** cursor is private to [`ring_claim::Claimer`]; nobody reads it
//! - the **published** cursor belongs to [`Publisher`] and is what the consumer's
//!   barrier waits on
//! - the **consumer position** lives in the producer's `GatingSet`, which is what
//!   makes the producer stop rather than lap it
//! - the ring's own capacity is in that same set
//!
//! A wiring that gives the consumer a private cursor compiles, runs, and passes
//! every single-threaded test — and gates nothing, because the producer is
//! reading a cursor nobody advances. `the_consumer_position_the_producer_gates_on_is_the_one_commit_moves`
//! asserts the wiring itself rather than trusting it.
//!
//! ## Two harnesses, and why both
//!
//! Under `--cfg loom` this file is the loom model and nothing else; otherwise it
//! is a set of ordinary threaded tests. They are not redundant.
//!
//! Real threads run the true code on the true hardware, at sizes loom could
//! never enumerate (thousands of items, several producers) — but they only ever
//! sample the interleavings the scheduler happens to pick, and on x86 the
//! hardware supplies orderings the code failed to ask for. `loom` runs a
//! deliberately tiny case — one claim, one drain — and checks *every*
//! interleaving of it against a memory model weaker than any real machine, so
//! it catches the missing `Release` that x86 would hide. Neither subsumes the
//! other: one has scale without coverage, the other coverage without scale.
//!
//! Run the second explicitly:
//!
//! ```text
//! RUSTFLAGS="--cfg loom" cargo test -p ring_publish --test handshake_test
//! ```

// ── every interleaving of one claim and one drain ──────────────────────────

/// The `loom` model named by the acceptance criterion.
///
/// `loom` replaces `ring_atomic`'s atomics with instrumented ones (see that
/// crate's module documentation on the seam) and re-runs the closure once per
/// distinct interleaving, so an assertion inside it is an assertion about all
/// of them rather than about the one the scheduler picked.
#[ cfg( loom ) ]
mod exhaustive
{
  use loom::sync::atomic::{ AtomicUsize, Ordering };
  use loom::sync::Arc;
  use ring_barrier::Barrier;
  use ring_claim::Claimer;
  use ring_consume::Consumer;
  use ring_cursor::SeqCell;
  use ring_gating::GatingSet;
  use ring_publish::Publisher;
  use ring_types::{ Capacity, Seq };

  /// What the producer writes into the slot. Any value the slot cannot hold by
  /// accident; zero would be indistinguishable from "never written".
  const WRITTEN : usize = 0xABC;

  #[ test ]
  fn a_claimed_slot_is_invisible_until_published_over_every_interleaving()
  {
    loom::model( ||
    {
      let capacity = Capacity::new( 2 ).expect( "a power of two" );

      // One consumer cursor, held by the producer's gating set. The consumer
      // reports into it; the producer gates on it. One cursor, both roles.
      let consumers = Arc::new( GatingSet::new( capacity, 1 ) );
      let publisher = Arc::new( Publisher::new() );

      // The slot itself, so that "did the consumer read something the producer
      // had not written" is an observable question rather than a claim about
      // sequence numbers.
      let slot = Arc::new( AtomicUsize::new( 0 ) );

      let producer = loom::thread::spawn(
      {
        let consumers = Arc::clone( &consumers );
        let publisher = Arc::clone( &publisher );
        let slot = Arc::clone( &slot );
        move ||
        {
          let claimer = Claimer::new( &consumers );
          let claim = claimer.claim( 1 ).expect( "an empty ring admits one slot" );
          assert_eq!( claim.start(), Seq::ZERO );

          // Between here and the publish below, the slot is claimed and the
          // consumer must not see it. That window is what loom explores.
          slot.store( WRITTEN, Ordering::Release );
          publisher.publish( claim.start(), claim.len() );
        }
      } );

      let drain = loom::thread::spawn(
      {
        let consumers = Arc::clone( &consumers );
        let publisher = Arc::clone( &publisher );
        let slot = Arc::clone( &slot );
        move ||
        {
          let barrier = Barrier::over( core::slice::from_ref( publisher.cursor() ) );
          let consumer = Consumer::new( consumers.cursor( 0 ).expect( "one consumer" ), barrier );

          // One look, not a loop. Every point at which the look could land is
          // a separate execution loom already runs, so spinning here would only
          // add unbounded executions without adding a single new observation.
          let run = consumer.available();
          assert!( run.len() <= 1, "available reported {run:?}, past the only claim ever made" );

          if run.is_empty()
          {
            return;
          }

          // Clause 1, made observable: `available` offered this slot, so the
          // producer's write to it must already have happened. A published
          // cursor advanced before the slot was written fails here.
          assert_eq!( run.start(), Seq::ZERO );
          assert_eq!(
            slot.load( Ordering::Acquire ),
            WRITTEN,
            "available offered a slot the producer had claimed but not written"
          );

          // Clause 3: commit takes exactly what was offered, and lands there.
          let reached = consumer.commit( run.end() ).expect( "committing what was offered" );
          assert_eq!( reached, run.end() );
          assert_eq!( consumer.position(), run.end() );
        }
      } );

      producer.join().expect( "the producer never panics" );
      drain.join().expect( "the drain never panics" );

      // Clause 2, deterministically now that the producer is done: what was
      // published is available. A consumer starting fresh sees exactly one slot.
      assert_eq!( publisher.published(), Seq( 1 ), "one slot was claimed and published" );

      let barrier = Barrier::over( core::slice::from_ref( publisher.cursor() ) );
      let fresh = ring_cursor::PaddedCursor::default();
      assert_eq!( Consumer::new( &fresh, barrier ).available().len(), 1 );

      // And the drain either took it or did not — never anything else.
      let position = consumers.cursor( 0 ).expect( "one consumer" ).load( Ordering::Acquire );
      assert!( position == Seq::ZERO || position == Seq( 1 ), "the drain ended at {position:?}" );
    } );
  }

  #[ test ]
  fn a_full_ring_stops_the_producer_over_every_interleaving()
  {
    // The other side of the same handshake: the consumer's commit is what
    // creates room. With capacity 1, the producer's second claim can only
    // succeed after the drain has committed the first — in every interleaving
    // where it succeeds at all.
    loom::model( ||
    {
      let capacity = Capacity::new( 2 ).expect( "a power of two" );
      let consumers = Arc::new( GatingSet::new( capacity, 1 ) );
      let publisher = Arc::new( Publisher::new() );

      let drain = loom::thread::spawn(
      {
        let consumers = Arc::clone( &consumers );
        let publisher = Arc::clone( &publisher );
        move ||
        {
          let barrier = Barrier::over( core::slice::from_ref( publisher.cursor() ) );
          let consumer = Consumer::new( consumers.cursor( 0 ).expect( "one consumer" ), barrier );
          consumer.commit_available();
        }
      } );

      let producer = loom::thread::spawn(
      {
        let consumers = Arc::clone( &consumers );
        let publisher = Arc::clone( &publisher );
        move ||
        {
          // The whole producer side in one thread, because a `Claimer` owns the
          // claimed cursor — a second one built elsewhere would start from zero
          // and hand out sequences the first had already given away.
          let claimer = Claimer::new( &consumers );

          let lap = claimer.claim( capacity.get() ).expect( "an empty ring admits a full lap" );
          assert_eq!( lap.start(), Seq::ZERO );
          publisher.publish( lap.start(), lap.len() );

          match claimer.claim( 1 )
          {
            // Granted, so the drain must already have released a slot. Read the
            // consumer position *after* the grant: it only ever rises, so a
            // correct grant can never trip this, and an incorrect one always
            // does. Reading before would fail whenever the drain moved in
            // between, which is not a bug.
            Ok( claim ) =>
            {
              assert_eq!( claim.start(), Seq( capacity.get() as u64 ) );
              let released = consumers.cursor( 0 ).unwrap().load( Ordering::Acquire );
              assert!(
                released.0 >= 1,
                "a slot past the first lap was granted while the drain was still at {released:?}"
              );
            },
            // Refused because it had not. Back-pressure, not a failure.
            Err( refused ) => assert!( !refused.is_configuration(), "{refused:?}" ),
          }
        }
      } );

      drain.join().expect( "the drain never panics" );
      producer.join().expect( "the producer never panics" );
    } );
  }
}

// ── the same handshake at a scale loom cannot enumerate ────────────────────

#[ cfg( not( loom ) ) ]
mod threaded
{
  use core::sync::atomic::{ AtomicU64, Ordering };
  use ring_barrier::Barrier;
  use ring_claim::Claimer;
  use ring_consume::Consumer;
  use ring_gating::GatingSet;
  use ring_publish::Publisher;
  use ring_types::{ Capacity, RingError, Seq };

  fn cap( slots : usize ) -> Capacity
  {
    Capacity::new( slots ).expect( "test capacities are powers of two" )
  }

  /// A ring's worth of slots, written by the producer and read by the consumer.
  ///
  /// Sequence numbers alone would let a broken `available` pass: the consumer
  /// would be handed a range of integers and check integers. Real slots make
  /// "was this written before it was offered" answerable.
  fn slots( capacity : Capacity ) -> Vec< AtomicU64 >
  {
    ( 0..capacity.get() ).map( | _ | AtomicU64::new( u64::MAX ) ).collect()
  }

  /// What a producer writes into the slot for sequence `seq`.
  ///
  /// Not `seq` itself: a slot that still holds its previous lap's value would
  /// then be indistinguishable from one correctly rewritten, and lapping is the
  /// exact failure a gating bug produces.
  fn payload( seq : u64 ) -> u64
  {
    seq.wrapping_mul( 0x9E37_79B9_7F4A_7C15 ) ^ 0x5DEE_CE66
  }

  #[ test ]
  fn the_four_operations_carry_every_item_across_in_order()
  {
    const TOTAL : u64 = 20_000;
    let capacity = cap( 16 );

    let consumers = GatingSet::new( capacity, 1 );
    let publisher = Publisher::new();
    let slots = slots( capacity );

    let received = std::thread::scope( | scope | -> Vec< u64 >
    {
      scope.spawn( ||
      {
        let claimer = Claimer::new( &consumers );
        let mut produced = 0u64;
        while produced < TOTAL
        {
          let Ok( claim ) = claimer.claim( 1 ) else { continue };

          let seq = claim.start();
          slots[ seq.0 as usize % capacity.get() ].store( payload( seq.0 ), Ordering::Release );
          publisher.publish( seq, claim.len() );
          produced += 1;
        }
      } );

      let barrier = Barrier::over( core::slice::from_ref( publisher.cursor() ) );
      let consumer = Consumer::new( consumers.cursor( 0 ).expect( "one consumer" ), barrier );

      let mut received = Vec::with_capacity( TOTAL as usize );
      while ( received.len() as u64 ) < TOTAL
      {
        let run = consumer.available();
        for seq in run.sequences()
        {
          received.push( slots[ seq.0 as usize % capacity.get() ].load( Ordering::Acquire ) );
        }
        if !run.is_empty()
        {
          consumer.commit( run.end() ).expect( "committing exactly what was offered" );
        }
      }
      received
    } );

    assert_eq!( received.len(), TOTAL as usize );
    for ( index, value ) in received.iter().enumerate()
    {
      assert_eq!(
        *value,
        payload( index as u64 ),
        "sequence {index} held the wrong payload — a slot was read unwritten, or lapped"
      );
    }
  }

  #[ test ]
  fn a_claim_that_is_never_published_stops_the_consumer_at_it()
  {
    // Clause 1, single-threaded and exact: the claim exists, the slot may even
    // be written, and none of it is visible until publication.
    let consumers = GatingSet::new( cap( 8 ), 1 );
    let publisher = Publisher::new();
    let claimer = Claimer::new( &consumers );

    let barrier = Barrier::over( core::slice::from_ref( publisher.cursor() ) );
    let consumer = Consumer::new( consumers.cursor( 0 ).unwrap(), barrier );

    let first = claimer.claim( 3 ).unwrap();
    assert!( consumer.available().is_empty(), "a claim alone published nothing" );

    let second = claimer.claim( 2 ).unwrap();
    assert!( consumer.available().is_empty(), "two claims alone still published nothing" );

    publisher.publish( first.start(), first.len() );
    assert_eq!( consumer.available().len(), 3, "only the published claim" );
    assert_eq!( consumer.available().end(), Seq( 3 ) );

    publisher.publish( second.start(), second.len() );
    assert_eq!( consumer.available().len(), 5 );
  }

  #[ test ]
  fn publication_out_of_order_is_refused_rather_than_advancing_past_a_gap()
  {
    // Two claims, published in the wrong order. The later one must not advance
    // the frontier over the earlier one's unwritten slots.
    let consumers = GatingSet::new( cap( 8 ), 1 );
    let publisher = Publisher::new();
    let claimer = Claimer::new( &consumers );

    let first = claimer.claim( 2 ).unwrap();
    let second = claimer.claim( 2 ).unwrap();

    assert_eq!(
      publisher.try_publish( second.start(), second.len() ),
      Err( Seq::ZERO ),
      "the second claim cannot publish over the first"
    );

    let barrier = Barrier::over( core::slice::from_ref( publisher.cursor() ) );
    let consumer = Consumer::new( consumers.cursor( 0 ).unwrap(), barrier );
    assert!( consumer.available().is_empty(), "and nothing became available" );

    assert_eq!( publisher.try_publish( first.start(), first.len() ), Ok( Seq( 2 ) ) );
    assert_eq!( publisher.try_publish( second.start(), second.len() ), Ok( Seq( 4 ) ) );
    assert_eq!( consumer.available().len(), 4 );
  }

  #[ test ]
  fn the_consumer_position_the_producer_gates_on_is_the_one_commit_moves()
  {
    // The wiring assertion. A consumer holding a cursor the producer does not
    // read passes every other test in this file and gates nothing.
    let consumers = GatingSet::new( cap( 4 ), 1 );
    let publisher = Publisher::new();
    let barrier = Barrier::over( core::slice::from_ref( publisher.cursor() ) );
    let consumer = Consumer::new( consumers.cursor( 0 ).unwrap(), barrier );

    assert!(
      core::ptr::eq( consumer.cursor(), consumers.cursor( 0 ).unwrap() ),
      "the consumer is reporting into a cursor the producer never reads"
    );

    let claimer = Claimer::new( &consumers );
    let claim = claimer.claim( 4 ).expect( "an empty ring admits a full lap" );
    publisher.publish( claim.start(), claim.len() );

    assert_eq!( claimer.claim( 1 ), Err( ring_types::RingError::Full ), "one lap ahead" );

    consumer.commit( Seq( 2 ) ).unwrap();
    assert_eq!( consumers.slowest(), Some( Seq( 2 ) ), "the commit reached the gating set" );
    assert_eq!( claimer.claim( 2 ).map( | c | c.start() ), Ok( Seq( 4 ) ), "two slots were released" );
  }

  #[ test ]
  fn a_stalled_consumer_stops_the_producer_after_exactly_one_lap()
  {
    // Feature 170's half of what 178 asserts from the gating side: with the
    // handshake wired, back-pressure is a property of the whole loop rather
    // than of `GatingSet` alone.
    let capacity = cap( 8 );
    let consumers = GatingSet::new( capacity, 1 );
    let publisher = Publisher::new();
    let claimer = Claimer::new( &consumers );

    for expected in 0..capacity.get() as u64
    {
      let claim = claimer.claim( 1 ).expect( "within the first lap" );
      assert_eq!( claim.start(), Seq( expected ) );
      publisher.publish( claim.start(), claim.len() );
    }

    for _ in 0..100
    {
      assert_eq!( claimer.claim( 1 ), Err( ring_types::RingError::Full ) );
    }
    assert_eq!( publisher.published(), Seq( capacity.get() as u64 ), "and nothing more was published" );
  }

  #[ test ]
  fn a_slow_consumer_and_a_fast_producer_never_lose_or_duplicate_an_item()
  {
    // A capacity far smaller than the item count, so the ring laps many times
    // and the gate is the only thing preventing an overwrite. The payload check
    // is what turns a lost gate into a failure rather than a shrug.
    const TOTAL : u64 = 8_000;
    let capacity = cap( 4 );

    let consumers = GatingSet::new( capacity, 1 );
    let publisher = Publisher::new();
    let slots = slots( capacity );

    std::thread::scope( | scope |
    {
      scope.spawn( ||
      {
        let claimer = Claimer::new( &consumers );
        let mut produced = 0u64;
        while produced < TOTAL
        {
          let Ok( claim ) = claimer.claim( 1 ) else
          {
            std::thread::yield_now();
            continue
          };

          let seq = claim.start();
          slots[ seq.0 as usize % capacity.get() ].store( payload( seq.0 ), Ordering::Release );
          publisher.publish( seq, claim.len() );
          produced += 1;
        }
      } );

      let barrier = Barrier::over( core::slice::from_ref( publisher.cursor() ) );
      let consumer = Consumer::new( consumers.cursor( 0 ).expect( "one consumer" ), barrier );

      let mut next = 0u64;
      while next < TOTAL
      {
        let run = consumer.available();
        assert_eq!( run.start(), Seq( next ), "the run did not resume where the last one ended" );

        for seq in run.sequences()
        {
          assert_eq!(
            slots[ seq.0 as usize % capacity.get() ].load( Ordering::Acquire ),
            payload( seq.0 ),
            "sequence {seq:?} was overwritten before it was read"
          );
        }

        if !run.is_empty()
        {
          consumer.commit( run.end() ).unwrap();
          next = run.end().0;
        }
      }
    } );

    assert_eq!( publisher.published(), Seq( TOTAL ) );
  }

  #[ test ]
  fn several_producers_and_one_drain_agree_on_every_sequence()
  {
    // `publish` rather than `try_publish`: with several producers the claims
    // complete out of order, and each waits for its predecessor. That wait is
    // what keeps the frontier from passing an unwritten slot.
    const PRODUCERS : u64 = 3;
    const PER_PRODUCER : u64 = 3_000;
    const TOTAL : u64 = PRODUCERS * PER_PRODUCER;

    let capacity = cap( 32 );
    let consumers = GatingSet::new( capacity, 1 );
    let publisher = Publisher::new();
    let claimer = Claimer::new( &consumers );
    let slots = slots( capacity );

    std::thread::scope( | scope |
    {
      for _ in 0..PRODUCERS
      {
        scope.spawn( ||
        {
          let mut produced = 0u64;
          while produced < PER_PRODUCER
          {
            let Ok( claim ) = claimer.claim( 1 ) else
            {
              std::thread::yield_now();
              continue
            };

            let seq = claim.start();
            slots[ seq.0 as usize % capacity.get() ].store( payload( seq.0 ), Ordering::Release );
            publisher.publish( seq, claim.len() );
            produced += 1;
          }
        } );
      }

      let barrier = Barrier::over( core::slice::from_ref( publisher.cursor() ) );
      let consumer = Consumer::new( consumers.cursor( 0 ).expect( "one consumer" ), barrier );

      let mut next = 0u64;
      while next < TOTAL
      {
        let run = consumer.available();
        for seq in run.sequences()
        {
          assert_eq!(
            slots[ seq.0 as usize % capacity.get() ].load( Ordering::Acquire ),
            payload( seq.0 ),
            "sequence {seq:?} was published before it was written, or lapped before it was read"
          );
        }
        if !run.is_empty()
        {
          consumer.commit( run.end() ).unwrap();
          next = run.end().0;
        }
      }
    } );

    assert_eq!( publisher.published(), Seq( TOTAL ) );
    assert_eq!( consumers.slowest(), Some( Seq( TOTAL ) ) );
  }

  #[ test ]
  fn a_consumer_that_never_commits_leaves_the_producer_exactly_one_lap_ahead()
  {
    // The dangerous window the split between `available` and `commit` exists to
    // create, held open: the consumer has read nothing, so the producer must
    // stop at one lap and stay there however long it tries.
    let capacity = cap( 4 );
    let consumers = GatingSet::new( capacity, 1 );
    let publisher = Publisher::new();
    let claimer = Claimer::new( &consumers );

    let claim = claimer.claim( capacity.get() ).expect( "a full lap" );
    publisher.publish( claim.start(), claim.len() );

    let barrier = Barrier::over( core::slice::from_ref( publisher.cursor() ) );
    let consumer = Consumer::new( consumers.cursor( 0 ).unwrap(), barrier );

    let run = consumer.available();
    assert_eq!( run.len(), capacity.get() as u64, "everything published is readable" );

    for _ in 0..1_000
    {
      assert!( claimer.claim( 1 ).is_err(), "the producer moved while the consumer was still reading" );
      assert_eq!( consumer.available(), run, "and what the consumer holds did not change under it" );
    }
  }

  /// One dropped claim kills the ring, and every signal keeps reporting health
  /// until the terminal state is an ordinary-looking `Full`.
  ///
  /// CL44 in `ring_claim/docs/pitfall/001_dropping_a_claim.md` describes this
  /// and could not host a reproduction: `ring_claim` has no dependency, normal
  /// or dev, on `ring_publish`, so its suite can never observe a publication
  /// that fails to arrive. This file can — `ring_claim` is one of its four
  /// dev-dependencies for exactly that reason — and the assertions below are
  /// that finding's table, row for row.
  ///
  /// Note what is *not* asserted: nothing here is a defect in either crate.
  /// Every call answers correctly for its own question. The finding is that no
  /// question anyone asks has "the ring is dead" as its answer.
  #[ test ]
  fn one_dropped_claim_pins_the_frontier_and_the_ring_dies_reporting_full()
  {
    let capacity = cap( 8 );
    let consumers = GatingSet::new( capacity, 1 );
    let publisher = Publisher::new();
    let claimer = Claimer::new( &consumers );

    // Strand exactly one sequence: claimed, never published, dropped.
    {
      let stranded = claimer.claim( 1 ).expect( "an empty ring admits one" );
      assert_eq!( stranded.start(), Seq::ZERO );
    }

    // Row 1 — `claim` reports Ok. The ring is already dead.
    let next = claimer.claim( 4 ).expect( "claiming is unaffected" );
    assert_eq!( next.start(), Seq( 1 ), "granted past a sequence nobody will publish" );

    // Row 2 — `claimed()` advances, healthily, past exactly that sequence.
    assert_eq!( claimer.claimed(), Seq( 5 ) );

    // Row 3 — `headroom()` reports three slots that can never be freed.
    assert_eq!( claimer.headroom(), 3 );

    // Row 4 — the producer that claimed correctly and wrote correctly is the
    // one that cannot proceed, and the error names a sequence it never touched.
    assert_eq!
    (
      publisher.try_publish( next.start(), next.len() ),
      Err( Seq::ZERO ),
      "the frontier is pinned behind the strand"
    );
    assert_eq!( publisher.published(), Seq::ZERO, "and nothing moved" );

    // And then it stops even looking like a hang. Once the ring fills behind
    // the strand, the symptom is the error documented as retryable
    // back-pressure — byte-identical to healthy contention, and the retry it
    // recommends is the action that never terminates.
    let _rest = claimer.claim( 3 ).expect( "the last three slots" );
    assert_eq!( claimer.headroom(), 0 );
    assert_eq!( claimer.claim( 1 ), Err( RingError::Full ) );
  }
}
