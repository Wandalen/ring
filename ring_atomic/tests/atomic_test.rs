//! The sequence cell, and the counting shim two acceptance criteria need.
//!
//! `ring_atomic` is claimed jointly with feature 170's handshake, which is
//! stage S4's — so this file deliberately does **not** cite that feature path:
//! citing it here would make the family's feature gate report 170 claimed
//! before a single line of the handshake exists.
//!
//! What it supplies is the instrument two S2 criteria are stated in terms
//! of. It claims neither — each is about its own crate's operation and is
//! asserted there, against the real API:
//!
//! - `docs/feature/175_thread_local_buffer_and_flush_into.md` — "accumulates N
//!   items with **zero** atomic operations", claimed by
//!   `ring_tls/tests/tls_test.rs` through a real `TlsBuffer`
//! - `docs/feature/177_batch_claim_and_batch_drain.md` — "a claim of 64 slots
//!   issues **one** fence, not 64", claimed by
//!   `ring_batch/tests/batch_test.rs` through a real `claim`
//!
//! Both criteria name the shim in their own text, so both are only as sound
//! as it is — and that soundness is establishable only here. Both are
//! negative claims about traffic, and neither is observable from outside an
//! `AtomicU64`. `CountingSeq` makes them observable. The tests below are
//! therefore in two halves: the first establishes that `CountingSeq` behaves
//! identically to `AtomicSeq` — without which every assertion made against it
//! is worthless — and the second establishes that its counts are actually
//! right. Where that second half restates 175's and 177's numbers it does so
//! at the level of the primitive; the criteria themselves belong to the
//! crates above.

// Ordinary tests, compiled out under `--cfg loom`. That cfg swaps
// `ring_atomic`'s atomics for loom's instrumented ones across the whole
// family, and those panic the moment they are touched outside a
// `loom::model` closure — so without this gate a family-wide loom run dies
// here instead of reaching the models in `ring_spsc`, `ring_mpsc`,
// `ring_publish` and `ring_testkit`.
#![ cfg( not( loom ) ) ]

use core::sync::atomic::Ordering;
use ring_atomic::{ AtomicSeq, CountingSeq, OpCounts, SeqCell };
use ring_types::Seq;
use std::thread;

// ---------------------------------------------------------------- production

#[ test ]
fn a_fresh_cell_reads_zero()
{
  assert_eq!( AtomicSeq::default().load( Ordering::Acquire ), Seq::ZERO );
  assert_eq!( CountingSeq::default().load( Ordering::Acquire ), Seq::ZERO );
}

#[ test ]
fn a_cell_reads_back_what_it_was_built_with()
{
  assert_eq!( AtomicSeq::new( Seq( 9 ) ).load( Ordering::Acquire ), Seq( 9 ) );
  assert_eq!( CountingSeq::new( Seq( 9 ) ).load( Ordering::Acquire ), Seq( 9 ) );
}

#[ test ]
fn store_overwrites_and_load_observes()
{
  let cell = AtomicSeq::default();
  cell.store( Seq( 41 ), Ordering::Release );
  assert_eq!( cell.load( Ordering::Acquire ), Seq( 41 ) );
  cell.store( Seq( 0 ), Ordering::Release );
  assert_eq!( cell.load( Ordering::Acquire ), Seq::ZERO );
}

#[ test ]
fn fetch_add_returns_the_value_before_the_advance()
{
  // The property the whole claim protocol rests on: the returned sequence is
  // the first one the caller owns, not the first one the *next* caller owns.
  let cell = AtomicSeq::new( Seq( 10 ) );
  assert_eq!( cell.fetch_add( 4, Ordering::AcqRel ), Seq( 10 ) );
  assert_eq!( cell.load( Ordering::Acquire ), Seq( 14 ) );
  assert_eq!( cell.fetch_add( 0, Ordering::AcqRel ), Seq( 14 ), "a zero advance still reports" );
  assert_eq!( cell.load( Ordering::Acquire ), Seq( 14 ) );
}

#[ test ]
fn compare_exchange_succeeds_on_the_expected_value_and_reports_the_actual_otherwise()
{
  let cell = AtomicSeq::new( Seq( 5 ) );

  assert_eq!
  (
    cell.compare_exchange( Seq( 5 ), Seq( 6 ), Ordering::AcqRel, Ordering::Acquire ),
    Ok( Seq( 5 ) ),
    "success returns the value replaced"
  );
  assert_eq!( cell.load( Ordering::Acquire ), Seq( 6 ) );

  assert_eq!
  (
    cell.compare_exchange( Seq( 5 ), Seq( 7 ), Ordering::AcqRel, Ordering::Acquire ),
    Err( Seq( 6 ) ),
    "failure returns what was actually there — the retry's input"
  );
  assert_eq!( cell.load( Ordering::Acquire ), Seq( 6 ), "and changes nothing" );
}

#[ test ]
fn concurrent_fetch_adds_partition_the_sequence_space()
{
  // No two threads may be handed the same sequence. Collect every returned
  // start and assert the set is exactly what was issued, with no repeats.
  const THREADS : usize = 4;
  const EACH : usize = 5_000;

  let cell = AtomicSeq::default();
  let cell = &cell;

  let claimed : Vec< u64 > = thread::scope
  ( | scope |
    {
      let handles : Vec< _ > = ( 0..THREADS )
        .map( |_| scope.spawn( move ||
        {
          ( 0..EACH ).map( |_| cell.fetch_add( 1, Ordering::AcqRel ).0 ).collect::< Vec< _ > >()
        } ) )
        .collect();
      handles.into_iter().flat_map( |h| h.join().expect( "claimer panicked" ) ).collect()
    }
  );

  let mut sorted = claimed;
  sorted.sort_unstable();
  let expected : Vec< u64 > = ( 0..( THREADS * EACH ) as u64 ).collect();
  assert_eq!( sorted, expected, "every sequence issued exactly once" );
  assert_eq!( cell.load( Ordering::Acquire ), Seq( ( THREADS * EACH ) as u64 ) );
}

// ------------------------------------------------------------------ counting

#[ test ]
fn a_counting_cell_starts_at_zero_of_everything()
{
  let counts = CountingSeq::default().counts();
  assert_eq!( counts, OpCounts::default() );
  assert_eq!( counts.total, 0 );
}

#[ test ]
fn each_operation_increments_exactly_its_own_counter()
{
  let cell = CountingSeq::default();

  cell.load( Ordering::Relaxed );
  assert_eq!( cell.counts(), OpCounts { loads : 1, total : 1, ..OpCounts::default() } );

  cell.store( Seq( 1 ), Ordering::Relaxed );
  assert_eq!( cell.counts(), OpCounts { loads : 1, stores : 1, total : 2, ..OpCounts::default() } );

  let _ = cell.fetch_add( 1, Ordering::Relaxed );
  assert_eq!
  (
    cell.counts(),
    OpCounts { loads : 1, stores : 1, fetch_adds : 1, total : 3, ..OpCounts::default() }
  );

  let _ = cell.compare_exchange( Seq( 2 ), Seq( 3 ), Ordering::Relaxed, Ordering::Relaxed );
  assert_eq!
  (
    cell.counts(),
    OpCounts { loads : 1, stores : 1, fetch_adds : 1, compare_exchanges : 1, total : 4 }
  );
}

#[ test ]
fn a_failed_compare_exchange_still_counts()
{
  // It cost the same fence whether or not it won. A shim that only counted
  // winners would under-report a contended claim, which is precisely the
  // measurement the shim exists for.
  let cell = CountingSeq::new( Seq( 1 ) );
  let outcome = cell.compare_exchange( Seq( 99 ), Seq( 100 ), Ordering::AcqRel, Ordering::Acquire );

  assert_eq!( outcome, Err( Seq( 1 ) ) );
  assert_eq!( cell.counts().compare_exchanges, 1 );
  assert_eq!( cell.counts().total, 1 );
}

#[ test ]
fn total_is_the_sum_of_the_four_and_not_an_independent_counter()
{
  let cell = CountingSeq::default();
  for _ in 0..3 { cell.load( Ordering::Relaxed ); }
  for _ in 0..5 { cell.store( Seq( 0 ), Ordering::Relaxed ); }
  for _ in 0..7 { let _ = cell.fetch_add( 1, Ordering::Relaxed ); }
  for _ in 0..11
  {
    let _ = cell.compare_exchange( Seq( 0 ), Seq( 0 ), Ordering::Relaxed, Ordering::Relaxed );
  }

  let c = cell.counts();
  assert_eq!( ( c.loads, c.stores, c.fetch_adds, c.compare_exchanges ), ( 3, 5, 7, 11 ) );
  assert_eq!( c.total, 3 + 5 + 7 + 11 );
}

#[ test ]
fn one_fetch_add_buys_a_whole_batch()
{
  // Feature 177's criterion, at the level of the primitive: 64 slots, one
  // operation. `ring_batch` asserts the same thing through its own `claim`.
  let cell = CountingSeq::default();
  let first = cell.fetch_add( 64, Ordering::AcqRel );

  assert_eq!( first, Seq::ZERO );
  assert_eq!( cell.load( Ordering::Acquire ), Seq( 64 ) );
  assert_eq!( cell.counts().fetch_adds, 1, "64 slots cost one fetch_add" );
}

#[ test ]
fn a_cell_never_touched_counts_zero()
{
  // Feature 175's criterion, at the level of the primitive: the shim reports
  // zero for code that did nothing, so a zero elsewhere means something.
  let cell = CountingSeq::default();
  let mut staged : Vec< u32 > = Vec::with_capacity( 64 );
  for i in 0..64 { staged.push( i ); }

  assert_eq!( staged.len(), 64 );
  assert_eq!( cell.counts().total, 0, "accumulating off to one side is free" );
}

#[ test ]
fn resetting_the_counts_leaves_the_sequence_alone()
{
  let cell = CountingSeq::default();
  cell.store( Seq( 77 ), Ordering::Release );
  cell.load( Ordering::Acquire );
  assert_eq!( cell.counts().total, 2 );

  cell.reset_counts();

  assert_eq!( cell.counts(), OpCounts::default() );
  assert_eq!( cell.load( Ordering::Acquire ), Seq( 77 ), "the value survived the reset" );
  assert_eq!( cell.counts().loads, 1, "and counting resumed from zero" );
}

#[ test ]
fn the_counting_cell_is_the_production_cell_plus_bookkeeping()
{
  // The load-bearing test of this file. Every assertion made against
  // CountingSeq elsewhere is a statement about production only if the two
  // cells agree on every operation. Drive an identical script through both and
  // compare at every step.
  let real = AtomicSeq::new( Seq( 3 ) );
  let shim = CountingSeq::new( Seq( 3 ) );

  assert_eq!( real.load( Ordering::Acquire ), shim.load( Ordering::Acquire ) );
  assert_eq!( real.fetch_add( 5, Ordering::AcqRel ), shim.fetch_add( 5, Ordering::AcqRel ) );
  assert_eq!( real.load( Ordering::Acquire ), shim.load( Ordering::Acquire ) );

  real.store( Seq( 20 ), Ordering::Release );
  shim.store( Seq( 20 ), Ordering::Release );
  assert_eq!( real.load( Ordering::Acquire ), shim.load( Ordering::Acquire ) );

  assert_eq!
  (
    real.compare_exchange( Seq( 20 ), Seq( 21 ), Ordering::AcqRel, Ordering::Acquire ),
    shim.compare_exchange( Seq( 20 ), Seq( 21 ), Ordering::AcqRel, Ordering::Acquire )
  );
  assert_eq!
  (
    real.compare_exchange( Seq( 99 ), Seq( 0 ), Ordering::AcqRel, Ordering::Acquire ),
    shim.compare_exchange( Seq( 99 ), Seq( 0 ), Ordering::AcqRel, Ordering::Acquire ),
    "including the failure case"
  );
  assert_eq!( real.load( Ordering::Acquire ), shim.load( Ordering::Acquire ) );
}

#[ test ]
fn counts_are_exact_under_contention()
{
  // The bookkeeping is itself atomic, so a concurrent run must not lose
  // increments — a shim that under-counted would turn a real regression into
  // a passing "one operation" assertion.
  const THREADS : usize = 4;
  const EACH : usize = 10_000;

  let cell = CountingSeq::default();
  let cell = &cell;

  thread::scope( | scope |
  {
    for _ in 0..THREADS
    {
      scope.spawn( move ||
      {
        for _ in 0..EACH
        {
          let _ = cell.fetch_add( 1, Ordering::AcqRel );
        }
      } );
    }
  } );

  assert_eq!( cell.counts().fetch_adds, THREADS * EACH );
  assert_eq!( cell.load( Ordering::Acquire ), Seq( ( THREADS * EACH ) as u64 ) );
}

#[ test ]
fn a_cell_drives_through_the_trait_alone()
{
  // Both implementations must be usable behind `dyn SeqCell`, which is what
  // lets one piece of production code be run against either.
  fn advance( cell : &dyn SeqCell ) -> Seq
  {
    cell.fetch_add( 2, Ordering::AcqRel )
  }

  let real = AtomicSeq::default();
  let shim = CountingSeq::default();

  assert_eq!( advance( &real ), Seq::ZERO );
  assert_eq!( advance( &shim ), Seq::ZERO );
  assert_eq!( real.load( Ordering::Acquire ), shim.load( Ordering::Acquire ) );
  assert_eq!( shim.counts().fetch_adds, 1 );
}

#[ test ]
fn op_counts_are_comparable_and_printable()
{
  // Comparable so a test can assert a whole shape at once rather than four
  // fields; printable so a failure says what was actually counted.
  let a = OpCounts::default();
  let b = OpCounts { loads : 1, total : 1, ..OpCounts::default() };

  assert_ne!( a, b );
  assert_eq!( a, OpCounts::default() );
  assert!( format!( "{b:?}" ).contains( "loads: 1" ) );
}

// ------------------------------------------------------------ the boundaries

/// The top of `u64`, where advancing moves the cursor backwards.
///
/// Root Cause: `fetch_add` is a wrapping `u64` addition and the signature rules
/// nothing out, so `n = u64::MAX` retreats the cursor by one while returning a
/// value shaped exactly like a legitimate claim of a huge range.
/// Why Not Caught: the word *monotonic* did not appear in the crate that owns
/// every operation the family's monotonicity claims are about, and no test went
/// near the boundary — counting to `u64::MAX` is unreachable, so the state read
/// as unreachable rather than as one call away for anyone seeding a cursor.
/// Fix Applied: `SeqCell::fetch_add` gained a `# Monotonicity` section stating
/// that monotonicity is the caller's, not the method's; this test records what
/// the boundary actually does.
/// Prevention: the behaviour is now pinned, so a future runtime check — or its
/// removal — changes a test rather than passing unnoticed.
/// Pitfall: an unreachable-by-counting state is still reachable in one call by
/// whoever seeds the value.
#[ test ]
fn fetch_add_wraps_at_the_top_of_u64()
{
  // Seeded near the top, an ordinary advance still behaves.
  let cell = AtomicSeq::new( Seq( u64::MAX - 2 ) );
  assert_eq!( cell.fetch_add( 2, Ordering::AcqRel ), Seq( u64::MAX - 2 ) );
  assert_eq!( cell.load( Ordering::Acquire ), Seq( u64::MAX ) );

  // One more, and it is Seq::ZERO — the value a fresh ring reports.
  assert_eq!( cell.fetch_add( 1, Ordering::AcqRel ), Seq( u64::MAX ) );
  assert_eq!( cell.load( Ordering::Acquire ), Seq::ZERO, "the cursor wrapped to the empty reading" );

  // And the reverse direction the doc names: advancing by u64::MAX retreats.
  let back = AtomicSeq::new( Seq( 10 ) );
  assert_eq!( back.fetch_add( u64::MAX, Ordering::AcqRel ), Seq( 10 ) );
  assert_eq!( back.load( Ordering::Acquire ), Seq( 9 ), "advancing by u64::MAX moved the cursor back one" );

  // The counting cell wraps identically — it is the same atomic underneath.
  let shim = CountingSeq::new( Seq( u64::MAX ) );
  assert_eq!( shim.fetch_add( 1, Ordering::AcqRel ), Seq( u64::MAX ) );
  assert_eq!( shim.load( Ordering::Acquire ), Seq::ZERO );

  // Nothing refused any of it. The return is a claim like any other, and the
  // count says one advance was served, not that one was rejected.
  assert_eq!( shim.counts().fetch_adds, 1, "the wrap was served, not refused" );
}

/// `total` is arithmetic over the same four reads, so it cannot audit them.
///
/// Root Cause: `counts()` performs four independent `Relaxed` loads and then
/// sums them into `total`, so `loads + stores + fetch_adds + compare_exchanges
/// == total` holds for a torn struct exactly as strongly as for a clean one.
/// Why Not Caught: the relation is the only cross-check the type offers, and it
/// is derived from the readings it would have to audit — a caller reaching for
/// it to detect tearing gets a certificate of soundness instead.
/// Fix Applied: `OpCounts` gained a `# Not a Coherent Observation` section
/// naming the second consequence explicitly; this test pins that the relation
/// is arithmetic rather than evidentiary.
/// Prevention: if `total` ever becomes an independently-maintained counter, the
/// hand-built case below stops holding and this test goes red — which is the
/// signal that the relation has become evidence.
/// Pitfall: an invariant that holds by construction proves the construction,
/// not the data.
#[ test ]
fn total_is_derived_from_the_same_four_reads()
{
  // A cell driven to a known shape: the relation holds, as it must.
  let cell = CountingSeq::default();
  cell.load( Ordering::Relaxed );
  cell.store( Seq( 3 ), Ordering::Release );
  let _ = cell.fetch_add( 1, Ordering::AcqRel );
  let _ = cell.compare_exchange( Seq( 4 ), Seq( 5 ), Ordering::AcqRel, Ordering::Acquire );

  let measured = cell.counts();
  assert_eq!(
    measured.loads + measured.stores + measured.fetch_adds + measured.compare_exchanges,
    measured.total,
  );

  // And a struct nobody measured — four numbers picked out of the air, summed
  // by hand — satisfies the identical relation. That is the whole point: the
  // check passes without any reading behind it, so passing it is not evidence
  // that a reading happened, let alone that four happened at one instant.
  let invented = OpCounts { loads : 900, stores : 7, fetch_adds : 41, compare_exchanges : 2, total : 950 };
  assert_eq!(
    invented.loads + invented.stores + invented.fetch_adds + invented.compare_exchanges,
    invented.total,
    "the consistency check certifies a struct that was never observed",
  );

  // The field is public and unvalidated, so a wrong total is representable too
  // — the relation is a convention this type does not enforce.
  let inconsistent = OpCounts { loads : 1, stores : 0, fetch_adds : 0, compare_exchanges : 0, total : 99 };
  assert_ne!(
    inconsistent.loads + inconsistent.stores + inconsistent.fetch_adds + inconsistent.compare_exchanges,
    inconsistent.total,
    "nothing rejects a total that does not match its own fields",
  );
}

/// Both cells, and the object form of the trait, are `Sync`.
///
/// Root Cause: `SeqCell` had no supertrait. Both implementors are `Sync` by
/// auto-derivation from the atomics inside them, so nothing had failed — but
/// `&dyn SeqCell` was not `Sync`, making the object form unusable for the one
/// thing the crate exists for, and three generic `C : SeqCell` bounds in
/// `ring_batch` and `ring_tls` relied on a property none of them asked for.
/// Why Not Caught: a property satisfied by accident is indistinguishable from
/// one that was required, right up until an implementor arrives without it.
/// Fix Applied: `pub trait SeqCell : Sync`. This test asserts the property at
/// the three places that depend on it — both concrete cells and the `dyn` form.
/// Prevention: a `!Sync` implementor now fails at its own `impl`, naming this
/// trait, rather than at a distant use site naming a `Cell< u64 >`.
/// Pitfall: object safety is not object *usability* — check what the `dyn` form
/// auto-implements, not only that it compiles.
#[ test ]
fn both_cells_and_the_object_form_are_sync()
{
  const fn requires_sync< T : Sync + ?Sized >() {}

  requires_sync::< AtomicSeq >();
  requires_sync::< CountingSeq >();
  requires_sync::< dyn SeqCell >();

  // Not a compile-time formality: two threads genuinely share one cell through
  // the object form, which is what `&dyn SeqCell` could not do before.
  let cell = AtomicSeq::default();
  let shared : &dyn SeqCell = &cell;

  thread::scope( | scope |
  {
    for _ in 0 .. 2
    {
      scope.spawn( move ||
      {
        for _ in 0 .. 1000
        {
          let _ = shared.fetch_add( 1, Ordering::AcqRel );
        }
      } );
    }
  } );

  assert_eq!( cell.load( Ordering::Acquire ), Seq( 2000 ) );
}

/// A type outside this crate implements `SeqCell` and drives through it.
///
/// Root Cause: only the crate's own two types implemented the trait, so nothing
/// exercised it as a trait — every bound was satisfied by the same two impls a
/// reader could see, and the supertrait, the `#[ must_use ]` and the ordering
/// arguments were all untested as *requirements on an implementor*.
/// Why Not Caught: a trait with exactly as many implementors as it has intended
/// ones looks fully covered; the gap only shows when a third arrives.
/// Fix Applied: this test defines one — a cell that saturates instead of
/// wrapping — and drives it through the same generic function the real cells go
/// through.
/// Prevention: the trait is now used as a trait. A change that makes it
/// unimplementable from outside (a private supertrait, a sealed marker, a method
/// taking a crate-private type) fails here.
/// Pitfall: two in-crate impls test the implementations, not the abstraction.
#[ test ]
fn a_third_type_implements_the_trait_and_drives_through_it()
{
  use core::sync::atomic::AtomicU64;

  /// A cell that clamps at the top instead of wrapping — the behaviour
  /// `fetch_add_wraps_at_the_top_of_u64` shows the real cells do not have.
  #[ derive( Debug, Default ) ]
  struct SaturatingSeq( AtomicU64 );

  impl SeqCell for SaturatingSeq
  {
    fn load( &self, order : Ordering ) -> Seq
    {
      Seq( self.0.load( order ) )
    }

    fn store( &self, value : Seq, order : Ordering )
    {
      self.0.store( value.0, order );
    }

    fn fetch_add( &self, n : u64, order : Ordering ) -> Seq
    {
      // Saturating, so it needs the loop the wrapping intrinsic does not.
      let mut current = self.0.load( Ordering::Relaxed );
      loop
      {
        let next = current.saturating_add( n );
        match self.0.compare_exchange_weak( current, next, order, Ordering::Relaxed )
        {
          Ok( previous ) => return Seq( previous ),
          Err( actual ) => current = actual,
        }
      }
    }

    fn compare_exchange( &self, current : Seq, new : Seq, success : Ordering, failure : Ordering )
    -> Result< Seq, Seq >
    {
      self.0
        .compare_exchange( current.0, new.0, success, failure )
        .map( Seq )
        .map_err( Seq )
    }
  }

  // The generic form: one function, three implementors, no `dyn`.
  fn claim< C : SeqCell >( cell : &C, n : u64 ) -> Seq
  {
    cell.fetch_add( n, Ordering::AcqRel )
  }

  let outside = SaturatingSeq::default();
  outside.store( Seq( u64::MAX - 1 ), Ordering::Release );

  assert_eq!( claim( &outside, 1 ), Seq( u64::MAX - 1 ) );
  assert_eq!( outside.load( Ordering::Acquire ), Seq( u64::MAX ) );

  // Where the real cells wrap to zero, this one stops.
  assert_eq!( claim( &outside, 5 ), Seq( u64::MAX ) );
  assert_eq!( outside.load( Ordering::Acquire ), Seq( u64::MAX ), "saturated instead of wrapping" );

  let real = AtomicSeq::new( Seq( u64::MAX ) );
  assert_eq!( claim( &real, 5 ), Seq( u64::MAX ) );
  assert_eq!( real.load( Ordering::Acquire ), Seq( 4 ), "the crate's own cell wraps" );

  // And the supertrait applies to the outside type too — it had to be `Sync`
  // to write the `impl` at all, which is the enforcement AT7 asked for.
  const fn requires_sync< T : Sync >() {}
  requires_sync::< SaturatingSeq >();
}
