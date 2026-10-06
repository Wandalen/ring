//! The exclusive writing end with a cached capacity gate.
//!
//! [`Producer`] is `Copy`, which is what lets any number of
//! threads write, and it is also what forbids private mutable state: a
//! `Copy` type cannot carry a per-handle cache. [`PrimaryProducer`] is the
//! one handle per ring that trades the sharing away for a faster claim — its
//! fast path grants from a private cache of both cursors and performs no
//! atomic load, where the ordinary producer reads the claim cursor as its
//! exchange's expected value and the consumer cursor for headroom on every
//! attempt.
//!
//! The handle is not `Clone` and not `Sync`: the cache is meaningful only
//! while one thread feeds it. `Send`, so it can be moved onto a thread.
//! Ordinary [`Producer`] copies stay usable beside it — the
//! claim cursor's compare-exchange arbitrates between them, and a cache that
//! lags can only make this end refuse a ring that has room, never overwrite
//! a record the consumer has not taken.

use super::*;

impl<'a, S> Producer<'a, S> {
  /// Mint the exclusive primary handle: a producer whose claim fast path
  /// performs no atomic load.
  ///
  /// The primary carries a private cache of both cursors (see the module
  /// documentation's "The primary producer's cursor cache"). Its claim grants
  /// from the cache and wins the slot with a compare-exchange whose expected
  /// value is the cached tail — a guess that is exact whenever this handle is
  /// the only active claimer, and that fails safely to the actual value when
  /// it is not. The ordinary [`Producer`] stays usable
  /// beside it: copies of it claim through the same cursor, and the
  /// compare-exchange arbitrates.
  ///
  /// The handle is not `Copy` and not `Clone` — the cache is meaningful only
  /// while one thread feeds it — and is not `Sync`. Minting consumes nothing:
  /// the `&mut self` borrow is what sequences this call against the cache's
  /// initialisation, and the borrow ends when the handle is returned.
  ///
  /// ```
  /// use ring_mpsc::Ring;
  /// use ring_slot::TypedSlot;
  /// use ring_types::Capacity;
  ///
  /// let mut ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 4 ).unwrap() );
  /// let mut ends = ring.ends();
  /// let ( mut producer, mut consumer ) = ends.split();
  ///
  /// let mut primary = producer.primary();
  /// primary.try_push( 7 ).unwrap();
  ///
  /// let mut batch = consumer.drain();
  /// assert_eq!( batch.get_mut( 0 ).and_then( TypedSlot::take ), Some( 7 ) );
  /// ```
  pub fn primary(&mut self) -> PrimaryProducer<'a, S> {
    PrimaryProducer {
      ring: self.ring,
      claimer: self.claimer,
      // The tail cache starts at the shared cursor's actual value: a guess
      // that is exact until someone else claims, and that the compare-
      // exchange's failure path corrects. The head cache starts at zero —
      // it can only lag a nonzero consumer cursor, and the first full
      // refreshes it.
      cached_tail: self.claimer.claimed(),
      cached_head: Seq::ZERO,
      _one_thread: PhantomData,
    }
  }
}

/// The exclusive writing end with a cached capacity gate.
///
/// [`Producer`] is `Copy`, which is what lets any number of
/// threads write, and it is also what forbids private mutable state: a
/// `Copy` type cannot carry a per-handle cache. [`PrimaryProducer`] is the
/// one handle per ring that trades the sharing away for a faster claim — its
/// fast path grants from a private cache of both cursors and performs no
/// atomic load, where the ordinary producer reads the claim cursor as its
/// exchange's expected value and the consumer cursor for headroom on every
/// attempt.
///
/// The handle is not `Clone` and not `Sync`: the cache is meaningful only
/// while one thread feeds it. `Send`, so it can be moved onto a thread.
/// Ordinary [`Producer`] copies stay usable beside it — the
/// claim cursor's compare-exchange arbitrates between them, and a cache that
/// lags can only make this end refuse a ring that has room, never overwrite
/// a record the consumer has not taken.
///
#[derive(Debug)]
pub struct PrimaryProducer<'a, S> {
  ring: &'a Ring<S>,
  claimer: &'a Claimer<'a>,
  /// The next sequence this end will claim — this handle's guess at the
  /// shared claim cursor's value. Advanced by a successful exchange, adopted
  /// from the exchange's failure value when the guess was behind, and equal
  /// to the shared cursor whenever this handle is the only active claimer.
  cached_tail: Seq,
  /// This handle's latest observation of the consumer cursor, taken at
  /// [`GATING`]. Monotone and conservative: it is only ever set to a value
  /// the consumer cursor actually held, so it can lag that cursor but never
  /// run ahead of it, and a claim granted on it can never overwrite a record
  /// the consumer has not taken.
  cached_head: Seq,
  /// See [`Producer`]'s field of the same name.
  _one_thread: PhantomData<Cell<()>>,
}

impl<'a, S> PrimaryProducer<'a, S> {
  /// Reserve the next sequence, or report that the ring is full.
  ///
  /// The capacity decision runs on this handle's cached cursors, and the fast
  /// path performs no atomic load: the cached tail is this handle's own
  /// bookkeeping, and a cached head that lags the consumer can only make the
  /// cache report less room than there is. When the cache reports full, one
  /// [`GATING`] load of the consumer cursor refreshes it — the single read of
  /// the consumer's line on this path — and the refreshed observation decides
  /// honestly. The claim itself is a compare-exchange whose expected value is
  /// the cached tail: exact whenever this handle is the only active claimer,
  /// failed safely to the actual value when it is not.
  ///
  /// # Errors
  ///
  /// [`RingError::Full`] when no slot is free. A refresh that still finds the
  /// ring full reports it; a refresh that finds room grants. Nothing waits;
  /// a full ring is reported.
  ///
  /// ```
  /// use ring_mpsc::Ring;
  /// use ring_slot::TypedSlot;
  /// use ring_types::{ Capacity, RingError };
  ///
  /// let mut ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 2 ).unwrap() );
  /// let mut ends = ring.ends();
  /// let ( mut producer, _consumer ) = ends.split();
  ///
  /// let mut primary = producer.primary();
  /// let first = primary.claim().unwrap();
  /// let second = primary.claim().unwrap();
  /// assert_eq!( primary.claim().err(), Some( RingError::Full ) );
  /// drop( ( first, second ) );
  /// ```
  pub fn claim(&mut self) -> Result<Reserved<'a, S>, RingError> {
    let capacity = self.ring.capacity().get() as u64;

    loop {
      if self.cached_head.distance_to(self.cached_tail) >= capacity {
        // The cache says full. The cache can only lag the consumer, so the
        // ring may still have room: refresh the observation once — the one
        // `Acquire` load the fast path exists to avoid. This load is also
        // the edge that orders the consumer's take of the lapped record
        // before this end's overwrite of its slot.
        self.cached_head = self.ring.consumer_cursor().load(GATING);

        if self.cached_head.distance_to(self.cached_tail) >= capacity {
          return Err(RingError::Full);
        }
      }

      let expected = self.cached_tail;
      match self.claimer.claim_guessed(expected) {
        // The guess was current: the sequence is ours, the cursor is already
        // advanced by this exchange, and the cache mirrors it — no cursor
        // load was paid for any of it.
        Ok(claim) => {
          self.cached_tail = claim.start().next();
          return Ok(Reserved {
            ring: self.ring,
            seq: claim.start(),
          });
        }
        Err(actual) => {
          // Another producer moved the cursor between our reads — adopt the
          // fresh value and retry. `actual` is monotone across failures, so
          // the loop makes progress the way `Claimer::claim`'s does.
          self.cached_tail = actual;
          core::hint::spin_loop();
        }
      }
    }
  }

  /// The next sequence this handle would claim, as the shared cursor holds it.
  ///
  /// The real cursor, not the cache: between a claim and its publish the two
  /// disagree by exactly the live reservation.
  ///
  /// ```
  /// use ring_mpsc::Ring;
  /// use ring_slot::TypedSlot;
  /// use ring_types::{ Capacity, Seq };
  ///
  /// let mut ring : Ring< TypedSlot< u8 > > = Ring::new( Capacity::new( 4 ).unwrap() );
  /// let mut ends = ring.ends();
  /// let ( mut producer, _consumer ) = ends.split();
  ///
  /// let mut primary = producer.primary();
  /// let reserved = primary.claim().unwrap();
  /// assert_eq!( reserved.sequence(), Seq::ZERO );
  /// assert_eq!( primary.claimed(), Seq( 1 ), "the claim advances before the publish" );
  /// drop( reserved );
  /// ```
  #[must_use]
  pub fn claimed(&self) -> Seq {
    self.claimer.claimed()
  }

  /// Room a claim may consume, as an **advisory** figure.
  ///
  /// The real cursors, not the cache — the same advisory contract
  /// [`Producer::free_capacity`](crate::Producer::free_capacity) documents.
  #[must_use]
  pub fn free_capacity(&self) -> usize {
    self.claimer.headroom()
  }

  /// The ring this end writes into.
  #[must_use]
  pub const fn ring(&self) -> &'a Ring<S> {
    self.ring
  }
}

impl<T: Send> PrimaryProducer<'_, TypedSlot<T>> {
  /// Put one record into the ring through the cached fast path.
  ///
  /// # Errors
  ///
  /// [`RingError::Full`], with `record` returned to the caller — exactly as
  /// `Producer::try_push`, which this mirrors over the cached gate.
  pub fn try_push(&mut self, record: T) -> Result<(), T> {
    let Ok(mut reservation) = self.claim() else {
      return Err(record);
    };

    drop(reservation.set(record));

    Ok(())
  }
}
