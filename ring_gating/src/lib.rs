//! Producer gating so it never laps the slowest consumer.
//!
//! Part of the ring family's concurrency write path.
//!
//! The sequence barrier has two halves. This crate is the producer's half.
//! Given a set of consumer cursors, it answers how far a producer may advance
//! without landing on a slot one of them has not finished with. `ring_barrier`
//! is the consumer's half, and reads the same minimum from the other side.
//!
//! ## Why the slowest consumer, and only the slowest
//!
//! A ring has one copy of each slot. A producer that laps *any* consumer
//! overwrites data that consumer has not read, so the bound is the minimum
//! across the whole gating set. It is not the average, the median, or the
//! consumer that happens to be asking. One stalled consumer stops the producer
//! for everyone, which is the correct behaviour and the reason a stalled
//! consumer is a problem worth detecting rather than routing around.
//!
//! ## The empty set is not a consumer at zero
//!
//! `ring_cursor::slowest` returns `None` for an empty set rather than `Seq::ZERO`,
//! and this crate carries that distinction through. A ring nobody is reading
//! has no data anyone can lose, so [`GatingSet::headroom`] returns a full
//! capacity rather than zero. Collapsing the two would deadlock every ungated
//! ring at the first lap. The producer would gate against a consumer that does
//! not exist and wait forever for it to move.
//!
//! ## Full versus BatchTooLarge
//!
//! [`GatingSet::check`] distinguishes them, and the distinction is the whole
//! reason it returns a `Result` rather than a `bool`. `Full` is back-pressure.
//! The caller should retry, because a consumer will move. `BatchTooLarge` is a
//! configuration error. A claim wider than the ring can never fit no matter who
//! moves, and a retry loop that could not tell them apart would spin forever on
//! the second. `RingError::is_configuration` is the caller's test.

#![deny(missing_docs)]

use ring_cursor::PaddedCursor;
use ring_types::{Capacity, RingError, Seq};

/// The consumer cursors a producer is gated by, and the ring size they are
/// positions in.
///
/// Owns its cursors rather than borrowing them, so that the set and the
/// cursors cannot get out of sync. A set holding references to cursors that
/// outlive it, or fewer cursors than it was built for, is a bound that reads
/// correctly and gates nothing.
///
/// ```
/// use core::sync::atomic::Ordering;
/// use ring_cursor::SeqCell;
/// use ring_gating::GatingSet;
/// use ring_types::{ Capacity, Seq };
///
/// let set = GatingSet::new( Capacity::new( 8 ).unwrap(), 2 );
/// assert_eq!( set.len(), 2 );
/// assert_eq!( set.slowest(), Some( Seq::ZERO ) );
///
/// set.cursor( 0 ).unwrap().store( Seq( 5 ), Ordering::Release );
/// assert_eq!( set.slowest(), Some( Seq::ZERO ), "the other one has not moved" );
///
/// set.cursor( 1 ).unwrap().store( Seq( 3 ), Ordering::Release );
/// assert_eq!( set.slowest(), Some( Seq( 3 ) ) );
/// ```
#[derive(Debug)]
pub struct GatingSet {
  cursors: Vec<PaddedCursor>,
  capacity: Capacity,
}

impl GatingSet {
  /// A set of `consumers` cursors, all at zero, gating a ring of `capacity`
  /// slots.
  ///
  /// `consumers` of zero is legal and means ungated. The module documentation
  /// explains why that is not the same as one consumer at zero.
  ///
  /// ```
  /// use ring_gating::GatingSet;
  /// use ring_types::Capacity;
  ///
  /// let ungated = GatingSet::new( Capacity::new( 4 ).unwrap(), 0 );
  /// assert!( ungated.is_empty() );
  /// assert_eq!( ungated.slowest(), None );
  /// ```
  #[must_use]
  pub fn new(capacity: Capacity, consumers: usize) -> Self {
    let mut cursors = Vec::with_capacity(consumers);
    cursors.resize_with(consumers, PaddedCursor::default);
    Self { cursors, capacity }
  }

  /// How many consumers gate this producer.
  ///
  /// ```
  /// use ring_gating::GatingSet;
  /// use ring_types::Capacity;
  /// assert_eq!( GatingSet::new( Capacity::new( 2 ).unwrap(), 3 ).len(), 3 );
  /// ```
  #[must_use]
  pub fn len(&self) -> usize {
    self.cursors.len()
  }

  /// Whether the ring is ungated.
  ///
  /// ```
  /// use ring_gating::GatingSet;
  /// use ring_types::Capacity;
  /// assert!( GatingSet::new( Capacity::new( 2 ).unwrap(), 0 ).is_empty() );
  /// ```
  #[must_use]
  pub fn is_empty(&self) -> bool {
    self.cursors.is_empty()
  }

  /// One consumer's cursor, for that consumer to advance.
  ///
  /// The `&self` receiver does not make the return value read-only. A
  /// `PaddedCursor` wraps an atomic, so `&PaddedCursor` is enough to store
  /// through it. Membership is fixed once the set is constructed; a cursor's
  /// stored position is not.
  ///
  /// ```
  /// use ring_gating::GatingSet;
  /// use ring_types::Capacity;
  ///
  /// let set = GatingSet::new( Capacity::new( 4 ).unwrap(), 1 );
  /// assert!( set.cursor( 0 ).is_some() );
  /// assert!( set.cursor( 1 ).is_none() );
  /// ```
  #[must_use]
  pub fn cursor(&self, index: usize) -> Option<&PaddedCursor> {
    self.cursors.get(index)
  }

  /// Every cursor in the set.
  ///
  /// ```
  /// use ring_gating::GatingSet;
  /// use ring_types::Capacity;
  /// assert_eq!( GatingSet::new( Capacity::new( 4 ).unwrap(), 3 ).cursors().len(), 3 );
  /// ```
  #[must_use]
  pub fn cursors(&self) -> &[PaddedCursor] {
    &self.cursors
  }

  /// The ring size these cursors are positions in.
  ///
  /// ```
  /// use ring_gating::GatingSet;
  /// use ring_types::Capacity;
  /// assert_eq!( GatingSet::new( Capacity::new( 16 ).unwrap(), 1 ).capacity().get(), 16 );
  /// ```
  #[must_use]
  pub const fn capacity(&self) -> Capacity {
    self.capacity
  }

  /// The position of the slowest consumer, or `None` when the set is empty.
  ///
  /// The fold itself is [`ring_cursor::slowest`], shared with `ring_barrier`,
  /// which asks the opposite question of the same kind of slice. That function
  /// reads every cursor at [`ring_cursor::GATING`], the family's one statement
  /// of the ordering a gating read uses, for the reason argued there. The caller
  /// is about to overwrite a slot on the answer, and a `Relaxed` load would let
  /// it act on a barrier the consumer has already moved past, or, worse, one it
  /// has not yet reached.
  ///
  /// ```
  /// use core::sync::atomic::Ordering;
  /// use ring_cursor::SeqCell;
  /// use ring_gating::GatingSet;
  /// use ring_types::{ Capacity, Seq };
  ///
  /// let set = GatingSet::new( Capacity::new( 8 ).unwrap(), 3 );
  /// for ( i, cursor ) in set.cursors().iter().enumerate()
  /// {
  ///   cursor.store( Seq( i as u64 + 4 ), Ordering::Release );
  /// }
  /// assert_eq!( set.slowest(), Some( Seq( 4 ) ) );
  /// ```
  #[must_use]
  pub fn slowest(&self) -> Option<Seq> {
    ring_cursor::slowest(&self.cursors)
  }

  /// How many slots a producer at `producer` may claim right now.
  ///
  /// Zero when the ring is full. A full capacity when the set is empty, since
  /// a ring nobody reads has no data anyone can lose.
  ///
  /// ```
  /// use core::sync::atomic::Ordering;
  /// use ring_cursor::SeqCell;
  /// use ring_gating::GatingSet;
  /// use ring_types::{ Capacity, Seq };
  ///
  /// let set = GatingSet::new( Capacity::new( 4 ).unwrap(), 1 );
  /// assert_eq!( set.headroom( Seq::ZERO ), 4 );
  /// assert_eq!( set.headroom( Seq( 3 ) ), 1 );
  /// assert_eq!( set.headroom( Seq( 4 ) ), 0, "a full lap ahead" );
  ///
  /// set.cursor( 0 ).unwrap().store( Seq( 2 ), Ordering::Release );
  /// assert_eq!( set.headroom( Seq( 4 ) ), 2, "the consumer released two slots" );
  /// ```
  #[must_use]
  pub fn headroom(&self, producer: Seq) -> usize {
    self.slowest().map_or(self.capacity.get(), |slowest| {
      ring_seqno::free_slots(producer, slowest, self.capacity)
    })
  }

  /// Whether a producer at `producer` may claim `count` contiguous slots.
  ///
  /// ```
  /// use ring_gating::GatingSet;
  /// use ring_types::{ Capacity, Seq };
  ///
  /// let set = GatingSet::new( Capacity::new( 4 ).unwrap(), 1 );
  /// assert!( set.admits( Seq::ZERO, 4 ) );
  /// assert!( !set.admits( Seq::ZERO, 5 ), "wider than the ring" );
  /// assert!( !set.admits( Seq( 2 ), 3 ), "only two slots left" );
  /// ```
  #[must_use]
  pub fn admits(&self, producer: Seq, count: usize) -> bool {
    count <= self.headroom(producer)
  }

  /// Admit a claim of `count` slots at `producer`, or say why not.
  ///
  /// # Errors
  ///
  /// [`RingError::BatchTooLarge`] when `count` exceeds the ring's capacity.
  /// That is a configuration error no consumer's progress can fix, so a retry
  /// loop must stop rather than spin. [`RingError::Full`] when the claim would
  /// fit in an empty ring but does not fit now. That is back-pressure, so a
  /// retry loop should keep going.
  ///
  /// # Not a step in a claim
  ///
  /// `ring_claim::Claimer::claim` open-codes both of these guards rather than
  /// calling this method, and the duplication is deliberate. This answers *is
  /// there room right now*, once, against a `producer` the caller supplies.
  /// A claim needs *is there room at the value I am about to exchange
  /// against*, re-evaluated on every retry against the sequence the failed
  /// compare-exchange returned. Substituting this call in would hoist the
  /// gate out of the retry and restore the check-then-act window
  /// `ring_claim`'s module doc rejects. Correct callers are the ones that
  /// want an answer rather than a claim: a diagnostic, an admission test, a
  /// caller deciding whether to attempt anything at all.
  ///
  /// ```
  /// use ring_gating::GatingSet;
  /// use ring_types::{ Capacity, RingError, Seq };
  ///
  /// let set = GatingSet::new( Capacity::new( 4 ).unwrap(), 1 );
  /// assert!( set.check( Seq::ZERO, 4 ).is_ok() );
  ///
  /// let too_wide = set.check( Seq::ZERO, 5 ).unwrap_err();
  /// assert!( too_wide.is_configuration(), "never retry this one" );
  ///
  /// assert_eq!( set.check( Seq( 3 ), 2 ), Err( RingError::Full ) );
  /// assert!( !RingError::Full.is_configuration(), "but do retry this one" );
  /// ```
  pub fn check(&self, producer: Seq, count: usize) -> Result<(), RingError> {
    if count > self.capacity.get() {
      return Err(RingError::BatchTooLarge {
        requested: count,
        capacity: self.capacity.get(),
      });
    }
    if count > self.headroom(producer) {
      return Err(RingError::Full);
    }
    Ok(())
  }

  /// The sequence a producer must not reach or pass.
  ///
  /// One lap beyond the slowest consumer. The producer may occupy every slot
  /// up to but not including this. `None` for an ungated ring, which has no
  /// such limit.
  ///
  /// Exposed separately from [`headroom`] because a diagnostic wants the
  /// absolute position. "Blocked at sequence 128" localises a stall, while
  /// "0 slots free" does not.
  ///
  /// [`headroom`]: Self::headroom
  ///
  /// ```
  /// use core::sync::atomic::Ordering;
  /// use ring_cursor::SeqCell;
  /// use ring_gating::GatingSet;
  /// use ring_types::{ Capacity, Seq };
  ///
  /// let set = GatingSet::new( Capacity::new( 8 ).unwrap(), 1 );
  /// assert_eq!( set.limit(), Some( Seq( 8 ) ) );
  ///
  /// set.cursor( 0 ).unwrap().store( Seq( 5 ), Ordering::Release );
  /// assert_eq!( set.limit(), Some( Seq( 13 ) ) );
  /// ```
  #[must_use]
  pub fn limit(&self) -> Option<Seq> {
    self.slowest().map(|s| s.advanced_by(self.capacity.get() as u64))
  }
}
