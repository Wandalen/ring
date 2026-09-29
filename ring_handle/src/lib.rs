//! Producer and consumer handles.
//!
//! One of the ring family's 33 crates — the concurrency write-path implementation.
//! Depends on `ring_core`.
//!
//! # What this crate adds
//!
//! [Feature 179](../../../docs/feature/179_producer_and_consumer_handles.md)
//! asks for the ring's two ends as two separate values, with capability
//! following ownership. `ring_core` already partitions the capabilities — its
//! `Producer` cannot drain and its `Consumer` cannot publish — so this crate is
//! not where that split is invented. It is where four things are added that
//! `ring_core` does not do, each of them a *narrowing*:
//!
//! | Added | Why `ring_core` does not have it |
//! |---|---|
//! | The ring is taken **by value** | `Ring::ends` borrows, so the caller keeps the ring and can split it again later. [`Split::new`] consumes it |
//! | `try_clone` is **withheld** | `ring_core::Producer::try_clone` exists and can succeed on an MPSC backend. A `ring_handle::Producer` cannot be duplicated at all |
//! | [`Consumer::drain`] | A drain whose bound is fixed at call time, so it terminates under a live producer |
//! | Nothing reaches the backend | No `Deref`, no `inner()`, no public field, on any of the three types |
//!
//! The overlap with `ring_core` is real and is documented rather than
//! minimised — see `docs/decisions/001`.
//!
//! # Two things the pre-implementation spec asked for that are not here
//!
//! **`is_closed()` on both handles.** It would need a `ring_shutdown`
//! dependency, and `ring_shutdown` depends on `ring_wait` — which would put a
//! parking operation within reach of the tick path and break
//! [feature 183](../../../docs/feature/183_try_only_operations_on_the_tick_path.md),
//! asserted in `ring_poll`'s suite. Two features in tension, resolved by
//! measurement; see `docs/decisions/002`.
//!
//! **`&self` receivers.** `&mut self` is what makes "exactly one producer"
//! hold: a shared `&Producer` could be used from two threads at once, which is
//! precisely the cardinality violation `ring_spsc` cannot otherwise detect.
//! Recorded in `docs/invariant/001`.

#![ deny( missing_docs ) ]

use ring_core::Ring;

/// A ring that has been given up, and can now only be split.
///
/// Taking the ring **by value** is the whole mechanism of
/// `docs/algorithm/001`: afterwards there is no route to the ring except
/// through this value, and this value offers exactly one operation.
///
/// ```
/// use ring_config::RingConfig;
/// use ring_core::Ring;
/// use ring_handle::Split;
///
/// let ring : Ring< u32 > = Ring::new( &RingConfig::new( 8 ).unwrap() ).unwrap();
/// let mut split = Split::new( ring );
/// let mut ends = split.ends();
/// let ( mut producer, mut consumer ) = ends.split();
///
/// producer.try_push( 1 ).unwrap();
/// assert_eq!( consumer.try_recv(), Some( 1 ) );
/// ```
#[ derive( Debug ) ]
pub struct Split< T >
{
  ring : Ring< T >,
}

impl< T : Send > Split< T >
{
  /// Give up the ring.
  ///
  /// The ring is moved in and is not reachable again — not through a getter,
  /// not through `Deref`, and not by splitting twice, since [`Split::ends`]
  /// borrows exclusively.
  pub const fn new( ring : Ring< T > ) -> Self
  {
    Self { ring }
  }

  /// Borrow the two ends, which [`Ends::split`] then separates.
  ///
  /// The two-step shape is inherited: the pair borrows from the `Ends` value,
  /// so the caller has to hold it. Collapsing the steps would need a
  /// self-referential struct.
  pub fn ends( &mut self ) -> Ends< '_, T >
  {
    Ends { inner : self.ring.ends() }
  }
}

/// The two ends, before they are separated.
#[ derive( Debug ) ]
pub struct Ends< 'a, T >
{
  inner : ring_core::Ends< 'a, T >,
}

impl< 'a, T : Send > Ends< 'a, T >
{
  /// Separate the ends into a publishing handle and a draining one.
  ///
  /// Exactly one of each. There is no operation here that yields a second
  /// producer, which is the difference from `ring_core::Producer::try_clone`.
  pub fn split( &'a mut self ) -> ( Producer< 'a, T >, Consumer< 'a, T > )
  {
    let ( producer, consumer ) = self.inner.split();
    ( Producer { inner : producer }, Consumer { inner : consumer } )
  }
}

// ── Producer ──────────────────────────────────────────────────────────────

/// The publishing end. Cannot drain, cannot be cloned, cannot reach the ring.
///
/// The absences are the contract; the present methods are ordinary forwarding.
/// See `docs/api/001` for the full absent-operations table and what each
/// addition would cost.
#[ derive( Debug ) ]
pub struct Producer< 'a, T >
{
  inner : ring_core::Producer< 'a, T >,
}

impl< T : Send > Producer< '_, T >
{
  /// Publish one record, or hand it back.
  ///
  /// Never blocks. On a full ring the ring's own `OverflowPolicy` decides:
  /// `Fail` returns the record, `DropNewest` discards it and reports success —
  /// so an `Ok` is not by itself evidence the record was kept. That trap is
  /// documented once, at
  /// [`ring_shutdown/docs/pitfall/002`](../../../ring_shutdown/docs/pitfall/002_ok_does_not_mean_kept_under_drop_newest.md).
  ///
  /// # Errors
  ///
  /// Returns the record when the ring is full and the policy refuses.
  pub fn try_push( &mut self, record : T ) -> Result< (), T >
  {
    self.inner.try_push( record )
  }

  /// Publish as many records as the ring will take, and report how many.
  ///
  /// Partial acceptance is the normal case; the iterator is left positioned at
  /// the first record that did not fit.
  pub fn try_push_batch( &mut self, records : &mut impl Iterator< Item = T > ) -> usize
  {
    self.inner.try_push_batch( records )
  }

  /// How much room there is, as a lower bound.
  ///
  /// Binding at SPSC cardinality, where this is the only producer; advisory at
  /// MPSC, where another producer may take the room first. One signature, two
  /// contracts, selected by a config field — see `docs/api/001`.
  #[ must_use ]
  pub fn free_capacity( &self ) -> usize
  {
    self.inner.free_capacity()
  }

  /// Whether there is no room at all.
  #[ must_use ]
  pub fn is_full( &self ) -> bool
  {
    self.inner.is_full()
  }
}

// ── Consumer ──────────────────────────────────────────────────────────────

/// The draining end. Cannot publish, cannot be cloned, cannot reach the ring.
///
/// Because there is exactly one of these, whoever holds it *is* the consume
/// point — which is why where this value lives is a correctness question and
/// not only a design one. See `docs/lifecycle/002`.
#[ derive( Debug ) ]
pub struct Consumer< 'a, T >
{
  inner : ring_core::Consumer< 'a, T >,
}

impl< 'a, T : Send > Consumer< 'a, T >
{
  /// Take one record, if one is waiting.
  ///
  /// `Option` rather than `Result`: an empty ring is not a failure, and
  /// modelling it as one makes every caller unwrap a non-failure.
  pub fn try_recv( &mut self ) -> Option< T >
  {
    self.inner.try_recv()
  }

  /// Take whatever is waiting into `out`, and report how many moved.
  pub fn try_recv_batch( &mut self, out : &mut Vec< T > ) -> usize
  {
    self.inner.try_recv_batch( out )
  }

  /// Take the records published *before this call*, as an iterator.
  ///
  /// **The bound is fixed here, not as the iterator runs.** An iterator that
  /// kept yielding whatever arrived would have no termination guarantee under a
  /// live producer — a live-lock that satisfies "never blocks" at every
  /// individual step. Reading the length once, up front, is what makes a full
  /// drain a bounded operation and what makes it reproducible given a fixed
  /// publication history.
  ///
  /// ```
  /// use ring_config::RingConfig;
  /// use ring_core::Ring;
  /// use ring_handle::Split;
  ///
  /// let ring : Ring< u32 > = Ring::new( &RingConfig::new( 8 ).unwrap() ).unwrap();
  /// let mut split = Split::new( ring );
  /// let mut ends = split.ends();
  /// let ( mut producer, mut consumer ) = ends.split();
  /// producer.try_push_batch( &mut ( 0..3 ) );
  ///
  /// let taken : Vec< u32 > = consumer.drain().collect();
  /// assert_eq!( taken, vec![ 0, 1, 2 ] );
  /// ```
  pub fn drain( &mut self ) -> Drain< '_, 'a, T >
  {
    let remaining = self.inner.len();
    Drain { consumer : self, remaining }
  }

  /// How many records are waiting, as a lower bound.
  ///
  /// It can only grow, never shrink, because this is the only consumer — the
  /// opposite direction from `Producer::free_capacity`'s bound at MPSC
  /// cardinality. The two read as duals and are not.
  #[ must_use ]
  pub fn len( &self ) -> usize
  {
    self.inner.len()
  }

  /// Whether nothing is waiting.
  #[ must_use ]
  pub fn is_empty( &self ) -> bool
  {
    self.inner.is_empty()
  }
}

/// The iterator [`Consumer::drain`] returns, bounded at the call that made it.
#[ derive( Debug ) ]
pub struct Drain< 'c, 'a, T >
{
  consumer : &'c mut Consumer< 'a, T >,
  remaining : usize,
}

impl< T : Send > Iterator for Drain< '_, '_, T >
{
  type Item = T;

  fn next( &mut self ) -> Option< T >
  {
    if self.remaining == 0
    {
      return None;
    }
    let record = self.consumer.try_recv()?;
    self.remaining -= 1;
    Some( record )
  }

  fn size_hint( &self ) -> ( usize, Option< usize > )
  {
    ( 0, Some( self.remaining ) )
  }
}
