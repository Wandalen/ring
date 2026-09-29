//! `ring_registry` — naming, refusal, and ownership.
//!
//! `docs/feature/181_named_ring_registry.md`'s acceptance criterion has three
//! clauses, and each has its own test named after it:
//!
//! | Clause | Test |
//! |---|---|
//! | Retrievable by that name and by no other | `a_registered_ring_is_retrievable_by_its_name_and_by_no_other` |
//! | A second registration under a live name is refused | `a_second_registration_under_a_live_name_is_refused` |
//! | Dropping the registry drops every ring it owns, by a drop counter | `dropping_the_registry_drops_every_record_still_in_every_ring` |
//!
//! The third is the one worth reading. It asserts ownership transitively — the
//! registry drops the ring, and the ring drops the records still unread in it —
//! which is a property of `ring_core`, not of this crate. It is measured here
//! because this is where the claim is made.

// Ordinary tests, compiled out under `--cfg loom`. That cfg swaps
// `ring_atomic`'s atomics for loom's instrumented ones across the whole
// family, and those panic the moment they are touched outside a
// `loom::model` closure — so without this gate a family-wide loom run dies
// here instead of reaching the models in `ring_spsc`, `ring_mpsc`,
// `ring_publish` and `ring_testkit`.
#![ cfg( not( loom ) ) ]

use core::sync::atomic::{ AtomicUsize, Ordering };

use ring_config::RingConfig;
use ring_core::Ring;
use ring_handle::Split;
use ring_registry::{ Registry, RegistryError };

fn ring< T : Send >( slots : usize ) -> Split< T >
{
  let config = RingConfig::new( slots ).expect( "a valid size" );
  Split::new( Ring::new( &config ).expect( "a ring" ) )
}

// ---------------------------------------------------------------------------
// Naming
// ---------------------------------------------------------------------------

/// The criterion's first clause, both halves.
///
/// "By no other" is the half that is easy to leave untested and is the one that
/// would actually catch a bug — a registry that returned the sole ring for any
/// name would pass the first half perfectly.
#[ test ]
fn a_registered_ring_is_retrievable_by_its_name_and_by_no_other()
{
  let mut registry = Registry::new();
  registry.register( "events", ring::< u32 >( 16 ) ).expect( "a free name" );

  assert!( registry.get_mut( "events" ).is_some() );

  for wrong in [ "event", "eventss", "Events", "", "telemetry" ]
  {
    assert!( registry.get_mut( wrong ).is_none(), "{wrong:?} retrieved the ring" );
  }
}

/// Two rings under two names stay distinct.
///
/// The registry is a map, and the way a map goes wrong is by conflating keys.
/// Distinguished by capacity rather than by contents, because capacity is
/// observable without consuming anything.
#[ test ]
fn two_names_hold_two_distinct_rings()
{
  let mut registry = Registry::new();
  registry.register( "small", ring::< u32 >( 4 ) ).expect( "a free name" );
  registry.register( "large", ring::< u32 >( 64 ) ).expect( "a free name" );

  let free_of = | registry : &mut Registry< u32 >, name : &str |
  {
    let split = registry.get_mut( name ).expect( "registered" );
    let mut ends = split.ends();
    let ( producer, _consumer ) = ends.split();
    producer.free_capacity()
  };

  assert_eq!( free_of( &mut registry, "small" ), 4 );
  assert_eq!( free_of( &mut registry, "large" ), 64 );
  assert_eq!( registry.len(), 2 );
}

/// A retrieved ring is the same ring on the next retrieval.
///
/// The registry lends rather than copies: a record pushed through one borrow is
/// still there on the next. Worth pinning because "returns a ring for that
/// name" would also be satisfied by something that rebuilt one.
#[ test ]
fn a_retrieved_ring_keeps_what_was_written_to_it()
{
  let mut registry = Registry::new();
  registry.register( "events", ring::< u32 >( 16 ) ).expect( "a free name" );

  {
    let split = registry.get_mut( "events" ).expect( "registered" );
    let mut ends = split.ends();
    let ( mut producer, _consumer ) = ends.split();
    for i in 0 .. 3u32 { assert!( producer.try_push( i ).is_ok(), "within capacity" ); }
  }

  let split = registry.get_mut( "events" ).expect( "still registered" );
  let mut ends = split.ends();
  let ( _producer, consumer ) = ends.split();
  assert_eq!( consumer.len(), 3, "the registry handed back a different ring" );
}

// ---------------------------------------------------------------------------
// Refusal
// ---------------------------------------------------------------------------

/// The criterion's second clause.
#[ test ]
fn a_second_registration_under_a_live_name_is_refused()
{
  let mut registry = Registry::new();
  registry.register( "events", ring::< u32 >( 16 ) ).expect( "a free name" );

  let ( error, _rejected ) = registry
    .register( "events", ring::< u32 >( 16 ) )
    .expect_err( "a live name was taken twice" );

  assert_eq!( error, RegistryError::NameTaken { name : "events".to_string() } );
  assert_eq!( registry.len(), 1, "the refused registration still changed the registry" );
}

/// A refusal returns the ring it refused, rather than consuming it.
///
/// The reason `register` returns `( RegistryError, Split< T > )` and not a bare
/// error: a caller whose name collided still owns a perfectly good ring, and a
/// signature that swallowed it would force them to drop it — discarding
/// whatever was in it — as a side effect of choosing a name badly.
#[ test ]
fn a_refused_registration_hands_the_ring_back()
{
  let mut registry = Registry::new();
  registry.register( "events", ring::< u32 >( 16 ) ).expect( "a free name" );

  let ( _error, mut rejected ) = registry
    .register( "events", ring::< u32 >( 8 ) )
    .expect_err( "refused" );

  let mut ends = rejected.ends();
  let ( producer, _consumer ) = ends.split();
  assert_eq!( producer.free_capacity(), 8, "the rejected ring came back unusable" );
}

/// Registering does not replace, even silently.
///
/// The failure this guards is a `HashMap::insert` where an `Entry` belongs:
/// `insert` returns the displaced value and would drop it if ignored, so a name
/// collision would destroy the previously registered ring and everything unread
/// in it. Asserted through the drop counter, because "was not replaced" is
/// exactly the claim a capacity check cannot make.
#[ test ]
fn a_refused_registration_does_not_drop_the_ring_already_there()
{
  static DROPS : AtomicUsize = AtomicUsize::new( 0 );

  #[ derive( Debug ) ]
  struct Counted;
  impl Drop for Counted
  {
    fn drop( &mut self ) { DROPS.fetch_add( 1, Ordering::SeqCst ); }
  }

  let mut registry = Registry::new();
  let mut first = ring::< Counted >( 16 );
  {
    let mut ends = first.ends();
    let ( mut producer, _consumer ) = ends.split();
    for _ in 0 .. 4 { assert!( producer.try_push( Counted ).is_ok(), "within capacity" ); }
  }
  registry.register( "events", first ).expect( "a free name" );

  let ( _error, rejected ) = registry
    .register( "events", ring::< Counted >( 16 ) )
    .expect_err( "refused" );
  drop( rejected );

  assert_eq!
  (
    DROPS.load( Ordering::SeqCst ),
    0,
    "the refused registration dropped the records held by the ring already registered"
  );
}

// ---------------------------------------------------------------------------
// Ownership
// ---------------------------------------------------------------------------

/// The criterion's third clause, with the drop counter it asks for.
///
/// The claim is transitive and the test is written to match it: three rings,
/// each holding unread records, all owned by one registry. Dropping the
/// registry must account for every record — the registry drops each ring, and
/// each ring drops what is still in it.
///
/// That second step is `ring_core`'s property, not this crate's. It is measured
/// here because this is where the claim is made, and a change to `ring_core`
/// that leaked records would fail here rather than nowhere.
#[ test ]
fn dropping_the_registry_drops_every_record_still_in_every_ring()
{
  static DROPS : AtomicUsize = AtomicUsize::new( 0 );

  #[ derive( Debug ) ]
  struct Counted;
  impl Drop for Counted
  {
    fn drop( &mut self ) { DROPS.fetch_add( 1, Ordering::SeqCst ); }
  }

  let mut registry = Registry::new();

  for ( name, records ) in [ ( "a", 4usize ), ( "b", 7 ), ( "c", 2 ) ]
  {
    let mut split = ring::< Counted >( 16 );
    {
      let mut ends = split.ends();
      let ( mut producer, _consumer ) = ends.split();
      for _ in 0 .. records { assert!( producer.try_push( Counted ).is_ok(), "within capacity" ); }
    }
    registry.register( name, split ).expect( "a free name" );
  }

  assert_eq!( registry.len(), 3 );
  assert_eq!( DROPS.load( Ordering::SeqCst ), 0, "something was dropped on the way in" );

  drop( registry );

  assert_eq!( DROPS.load( Ordering::SeqCst ), 13, "4 + 7 + 2 records were not all dropped" );
}

/// Removing a name hands the ring back and frees the name.
///
/// What makes "a *live* name" a temporary condition — and therefore what makes
/// the refusal recoverable rather than a permanent loss of the name.
#[ test ]
fn removing_a_name_frees_it_for_reuse()
{
  let mut registry = Registry::new();
  registry.register( "events", ring::< u32 >( 4 ) ).expect( "a free name" );
  assert!( !registry.is_empty(), "nothing was registered, so removing proves nothing" );

  let taken = registry.remove( "events" ).expect( "registered" );
  assert!( registry.is_empty() );
  assert!( !registry.contains( "events" ) );
  drop( taken );

  registry.register( "events", ring::< u32 >( 64 ) ).expect( "the name was freed" );
  assert_eq!( registry.len(), 1 );
}

/// Removing an absent name is `None`, not a panic.
#[ test ]
fn removing_an_absent_name_is_none()
{
  let mut registry : Registry< u32 > = Registry::new();
  assert!( registry.remove( "nothing" ).is_none() );
  assert!( registry.is_empty() );
}

/// A removed ring is dropped by its new owner, not by the registry.
///
/// The other side of ownership transfer: after `remove`, the registry is no
/// longer accountable for the records, and the caller is.
#[ test ]
fn a_removed_ring_carries_its_records_to_its_new_owner()
{
  static DROPS : AtomicUsize = AtomicUsize::new( 0 );

  #[ derive( Debug ) ]
  struct Counted;
  impl Drop for Counted
  {
    fn drop( &mut self ) { DROPS.fetch_add( 1, Ordering::SeqCst ); }
  }

  let mut registry = Registry::new();
  let mut split = ring::< Counted >( 16 );
  {
    let mut ends = split.ends();
    let ( mut producer, _consumer ) = ends.split();
    for _ in 0 .. 5 { assert!( producer.try_push( Counted ).is_ok(), "within capacity" ); }
  }
  registry.register( "events", split ).expect( "a free name" );

  let taken = registry.remove( "events" ).expect( "registered" );
  drop( registry );
  assert_eq!( DROPS.load( Ordering::SeqCst ), 0, "the registry dropped a ring it no longer owned" );

  drop( taken );
  assert_eq!( DROPS.load( Ordering::SeqCst ), 5 );
}

// ---------------------------------------------------------------------------
// The rest of the surface
// ---------------------------------------------------------------------------

/// An empty registry reports itself empty, and `Default` matches `new`.
#[ test ]
fn an_empty_registry_is_empty()
{
  let registry : Registry< u32 > = Registry::new();
  assert!( registry.is_empty() );
  assert_eq!( registry.len(), 0 );
  assert!( !registry.contains( "anything" ) );
  assert_eq!( registry.names().count(), 0 );

  let defaulted : Registry< u32 > = Registry::default();
  assert!( defaulted.is_empty() );
}

/// `names` lists exactly the live names, in whatever order.
///
/// Sorted before comparison because `HashMap` iteration order is unspecified —
/// asserting a fixed order here would be a test that passes for a reason the
/// crate does not promise, and would break on a hasher change.
#[ test ]
fn names_lists_every_live_name()
{
  let mut registry = Registry::new();
  for name in [ "a", "b", "c" ]
  {
    registry.register( name, ring::< u32 >( 4 ) ).expect( "a free name" );
  }
  registry.remove( "b" ).expect( "registered" );

  let mut names : Vec< &str > = registry.names().collect();
  names.sort_unstable();
  assert_eq!( names, vec![ "a", "c" ] );
}

/// A name is any string, including ones that look like nothing.
///
/// `register` takes `impl Into< String >`, so the empty string and a name with
/// spaces are both valid keys. Pinned rather than left implicit: a later
/// validation pass rejecting them would be a behaviour change, and this is the
/// test that would say so.
#[ test ]
fn unusual_names_are_ordinary_names()
{
  let mut registry = Registry::new();
  for name in [ "", " ", "a/b", "events\n" ]
  {
    registry.register( name, ring::< u32 >( 4 ) ).expect( "a free name" );
    assert!( registry.contains( name ), "{name:?} was not stored under itself" );
  }
  assert_eq!( registry.len(), 4 );
}

/// The error names the name it refused.
#[ test ]
fn the_error_names_the_taken_name()
{
  let error = RegistryError::NameTaken { name : "events".to_string() };
  assert!( error.to_string().contains( "events" ) );

  fn caller() -> Result< (), Box< dyn core::error::Error > >
  {
    Err( Box::new( RegistryError::NameTaken { name : "x".to_string() } ) )
  }
  assert!( caller().is_err(), "RegistryError does not satisfy Error" );
}

/// `get_mut` is E5's uncovered path: assignment through the borrow replaces
/// the ring in place, and nothing in the registry refuses it the way `E2`
/// refuses a second `register` under the same name.
///
/// → `docs/invariant/001_one_name_one_ring.md`, E5.
#[ test ]
fn assigning_through_get_mut_drops_the_ring_it_replaces()
{
  static DROPS : AtomicUsize = AtomicUsize::new( 0 );

  #[ derive( Debug ) ]
  struct Counted;
  impl Drop for Counted
  {
    fn drop( &mut self ) { DROPS.fetch_add( 1, Ordering::SeqCst ); }
  }

  let mut registry = Registry::new();
  let mut original = ring::< Counted >( 16 );
  {
    let mut ends = original.ends();
    let ( mut producer, _consumer ) = ends.split();
    for _ in 0 .. 5 { assert!( producer.try_push( Counted ).is_ok(), "within capacity" ); }
  }
  registry.register( "events", original ).expect( "a free name" );

  assert_eq!( DROPS.load( Ordering::SeqCst ), 0, "nothing dropped on the way in" );

  // No refusal, no return value, no `remove` — E1/E2's protection covers
  // `register`, not this.
  *registry.get_mut( "events" ).unwrap() = ring::< Counted >( 16 );

  assert_eq!
  (
    DROPS.load( Ordering::SeqCst ),
    5,
    "assigning through get_mut did not drop the 5 unread records it replaced"
  );
  assert_eq!( registry.len(), 1, "the name still resolves to exactly one ring, per R1" );
}
