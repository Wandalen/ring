//! Sequence-range claiming without waiting.
//!
//! Part of the ring family's concurrency write path.
//!
//! This crate does not depend on `ring_seqno`, though it was in the manifest
//! before the implementation existed. Every piece of sequence arithmetic claiming
//! needs is either `ring_types::Seq`'s own (`advanced_by`, and the `Ord` that
//! makes range containment a comparison rather than a subtraction) or already
//! inside `ring_gating::GatingSet::headroom`, which reaches `ring_seqno` on this
//! crate's behalf. Declaring it here as well would be a dependency carried for
//! the shape of the forest rather than for a call.
//!
//! Claiming is the first half of the claim/publish handshake. A producer takes
//! exclusive ownership of a range of sequences, writes into the slots they
//! index, and only then publishes. `ring_publish` is the second half.
//!
//! Multi-producer claiming requires exclusivity. No two producers may ever be
//! granted the same sequence, and that requirement is why this crate exists
//! instead of being two lines inside `ring_publish`.
//!
//! ## Invariant: no two producers hold one sequence
//!
//! Only one compare-exchange in [`Claimer::claim`] or [`Claimer::claim_up_to`]
//! can move the cursor off a given value. Every other caller adopts the
//! winner's value and computes a disjoint range from it. The gate plays no
//! part in this. With no consumer registered, exclusivity still holds and
//! producers simply run past where a consumer would be.
//!
//! **Excluded.** An order across producers. Each producer's own claims come
//! back in increasing order, while two producers' claims interleave however
//! their exchanges land. Also excluded are writes through [`Claimer::cursor`],
//! which can grant one range twice, as that accessor documents.
//!
//! **Enforced by.** `no_two_producers_are_ever_granted_the_same_sequence`,
//! `claims_under_contention_lose_no_sequences`,
//! `claim_up_to_under_contention_loses_no_sequences_either` and
//! `each_producers_own_claims_stay_in_issue_order`, with no consumer
//! registered, and `no_grant_ever_passes_the_limit_under_contention` under a
//! binding gate. No test asserts that claims refused by a binding gate and then
//! retried leave no hole in the sequence space.
//!
//! ## Invariant: a refused claim moves nothing
//!
//! A call that returns `Err` leaves [`Claimer::claimed`] where it found it,
//! because the only write is the exchange on the `Ok` path. That is what makes
//! retrying on [`RingError::Full`] safe. A claim that moved the cursor and then
//! failed would leak a slot on every attempt.
//!
//! **Enforced by.** `a_failed_claim_advances_nothing`.
//!
//! ## Why claiming never waits
//!
//! Every function here returns immediately, with `Ok` and a range or with
//! `Err` saying why not. That is what lets the same claim functions serve a
//! spinning producer, a parking producer, and the tick path that must not
//! block at all. A claim that waited internally would force the wait
//! strategy into this crate and make `WaitKind::None` unimplementable above
//! it.
//!
//! A caller that wants to wait calls `ring_wait::for_space`, then
//! [`Claimer::claim`].
//!
//! ## Why a claim is `#[must_use]` and carries no destructor
//!
//! A [`Claim`] is a promise the producer made to itself: these sequences are
//! mine and I will publish them. Dropping one without publishing strands the
//! range. The producer cursor has already advanced past it, so those slots
//! are never written and never reclaimed, and every consumer stalls at the
//! gap forever.
//!
//! There is deliberately no `Drop` impl that "releases" the claim, because
//! releasing is not possible. Another producer may already have claimed the
//! range beyond it, so rewinding the cursor would hand out sequences twice, the
//! one thing multi-producer claiming forbids outright. A `Drop` that published
//! instead would need the ring's publisher, and this crate does not depend on
//! `ring_publish`. The type is `#[must_use]`, which catches a bare discarded
//! value and nothing else. [`Claim`]'s pitfall lists the routes it misses.
//!
//! ## Why the CAS loop is not `fetch_add`
//!
//! A `fetch_add` claim is shorter and wrong. It advances the cursor
//! unconditionally, so the gating check has to happen *before* it. Between
//! that check and the add, another producer can take the space the check
//! just found. The producer then holds a range that overlaps a slot a
//! consumer is still reading. No later check can undo that, because the
//! range is already granted.
//!
//! The compare-exchange loop re-reads the gate inside the retry, so the
//! decision to grant and the granting itself are one atomic step. A granted
//! range never ends past the consumers' limit as it stood at the grant,
//! because the gate is read at the value being exchanged.
//! `no_grant_ever_passes_the_limit_under_contention` checks it.
//!
//! The retry has no budget. A failed exchange means a peer's claim landed, so
//! some producer always makes progress, but one producer can keep losing for
//! as long as its peers keep winning.

#![deny(missing_docs)]

use ring_cursor::{GATING, PaddedCursor, SeqCell};
use ring_gating::GatingSet;
use ring_types::{RingError, Seq};

/// The ordering a successful claim publishes the new cursor value at.
///
/// `AcqRel` rather than `Release`, because a successful exchange releases the
/// cursor advance to other producers and also acquires whatever the producer
/// whose value we replaced had done. A bare `Release` would let this producer's
/// slot writes be reordered before it observed the previous producer's claim.
const CLAIM_SUCCESS: core::sync::atomic::Ordering = core::sync::atomic::Ordering::AcqRel;

/// A contiguous range of sequences granted to exactly one producer.
///
/// Half-open: `start..end`, so an empty claim and a one-slot claim are not the
/// same value, and `end` is directly the sequence the producer cursor now sits
/// at.
///
/// # Pitfall: a dropped claim stops the ring, and the lint rarely says so
///
/// **Trap.** Relying on `#[must_use]` to catch a claim that never reaches a
/// publish. It fires only on a value nobody binds. `let _ = claimer.claim( 4 )`,
/// an early return or `?` between the claim and the write, an unwind, and a
/// `continue` past the publish all compile without a warning. A bare
/// `claimer.claim( 4 );` does warn, but with `Result`'s generic message rather
/// than this type's, and the compiler suggests `let _ =`, which is the first
/// route above.
///
/// **Failure.** Every later publish waits on the lost range, as
/// `ring_publish::Publisher::publish` documents, and once the ring fills
/// [`Claimer::claim`] returns [`RingError::Full`] for good. That is the same
/// value healthy back-pressure returns, so a retry loop spins instead of
/// failing. One lost sequence stops the ring as completely as a thousand.
///
/// **Mitigation.** Publish on every path out of the scope that holds the claim,
/// including early returns and unwinds. A stall from a lost claim shows as
/// `ring_publish`'s published cursor staying behind [`Claimer::claimed`] for
/// longer than any slot write takes.
///
/// ```
/// use ring_claim::Claim;
/// use ring_types::Seq;
///
/// let claim = Claim::new( Seq( 4 ), 3 );
/// assert_eq!( claim.start(), Seq( 4 ) );
/// assert_eq!( claim.end(), Seq( 7 ) );
/// assert_eq!( claim.len(), 3 );
/// assert_eq!( claim.sequences().collect::< Vec< _ > >(), vec![ Seq( 4 ), Seq( 5 ), Seq( 6 ) ] );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use = "a claimed range that is never published strands its slots and stalls every consumer"]
pub struct Claim {
  start: Seq,
  len: usize,
}

impl Claim {
  /// A claim of `len` sequences beginning at `start`.
  ///
  /// Public because `ring_publish` and the test suites of both crates need to
  /// construct one directly. A producer gets real claims from
  /// [`Claimer::claim`], the only path that establishes exclusivity.
  ///
  /// ```
  /// use ring_claim::Claim;
  /// use ring_types::Seq;
  /// assert_eq!( Claim::new( Seq::ZERO, 0 ).len(), 0 );
  /// ```
  // No `#[ must_use ]` here. `Claim` itself already carries one *with a
  // message*, and a bare attribute on the constructor would only shadow it
  // with a less informative warning.
  pub const fn new(start: Seq, len: usize) -> Self {
    Self { start, len }
  }

  /// The first sequence in the range.
  ///
  /// ```
  /// use ring_claim::Claim;
  /// use ring_types::Seq;
  /// assert_eq!( Claim::new( Seq( 9 ), 2 ).start(), Seq( 9 ) );
  /// ```
  #[must_use]
  pub const fn start(self) -> Seq {
    self.start
  }

  /// One past the last sequence in the range.
  ///
  /// ```
  /// use ring_claim::Claim;
  /// use ring_types::Seq;
  /// assert_eq!( Claim::new( Seq( 9 ), 2 ).end(), Seq( 11 ) );
  /// ```
  #[must_use]
  pub const fn end(self) -> Seq {
    self.start.advanced_by(self.len as u64)
  }

  /// How many sequences the range covers.
  ///
  /// ```
  /// use ring_claim::Claim;
  /// use ring_types::Seq;
  /// assert_eq!( Claim::new( Seq::ZERO, 5 ).len(), 5 );
  /// ```
  #[must_use]
  pub const fn len(self) -> usize {
    self.len
  }

  /// Whether the range covers nothing.
  ///
  /// ```
  /// use ring_claim::Claim;
  /// use ring_types::Seq;
  /// assert!( Claim::new( Seq( 3 ), 0 ).is_empty() );
  /// ```
  #[must_use]
  pub const fn is_empty(self) -> bool {
    self.len == 0
  }

  /// Whether `seq` falls inside the range.
  ///
  /// ```
  /// use ring_claim::Claim;
  /// use ring_types::Seq;
  ///
  /// let claim = Claim::new( Seq( 4 ), 2 );
  /// assert!( claim.contains( Seq( 4 ) ) );
  /// assert!( claim.contains( Seq( 5 ) ) );
  /// assert!( !claim.contains( Seq( 6 ) ), "half-open" );
  /// assert!( !claim.contains( Seq( 3 ) ) );
  /// ```
  #[must_use]
  pub const fn contains(self, seq: Seq) -> bool {
    // Compares raw `u64` values instead of `Seq`'s operators, because
    // `PartialOrd` is not callable in a `const fn`. Reaching through the newtype
    // for two comparisons is the entire cost of having this one at compile time.
    seq.0 >= self.start.0 && seq.0 < self.end().0
  }

  /// Every sequence in the range, in order.
  ///
  /// ```
  /// use ring_claim::Claim;
  /// use ring_types::Seq;
  ///
  /// let seen : Vec< u64 > = Claim::new( Seq( 2 ), 3 ).sequences().map( | s | s.0 ).collect();
  /// assert_eq!( seen, vec![ 2, 3, 4 ] );
  /// ```
  pub fn sequences(self) -> impl Iterator<Item = Seq> {
    (self.start.0..self.end().0).map(Seq)
  }

  /// Whether this range shares any sequence with `other`.
  ///
  /// This is the property multi-producer claiming forbids across producers. It
  /// is public so a test can assert it directly instead of reconstructing the
  /// comparison at every call site.
  ///
  /// ```
  /// use ring_claim::Claim;
  /// use ring_types::Seq;
  ///
  /// let first = Claim::new( Seq( 0 ), 4 );
  /// assert!( !first.overlaps( Claim::new( Seq( 4 ), 4 ) ), "adjacent, not overlapping" );
  /// assert!( first.overlaps( Claim::new( Seq( 3 ), 4 ) ) );
  /// assert!( !first.overlaps( Claim::new( Seq( 0 ), 0 ) ), "an empty claim covers nothing" );
  /// ```
  #[must_use]
  pub const fn overlaps(self, other: Self) -> bool {
    // Raw `u64` comparisons for the same reason `contains` uses them, and
    // with more at stake. The exclusivity tests assert with this predicate,
    // so having it answerable at compile time is worth reaching through the
    // newtype for.
    !self.is_empty() && !other.is_empty() && self.start.0 < other.end().0 && other.start.0 < self.end().0
  }
}

/// The producer cursor and the gate it must respect, together.
///
/// Holds both because granting a claim requires reading the gate and advancing
/// the cursor as one step. The module documentation explains why the two cannot
/// be separated without handing out overlapping ranges.
///
/// ```
/// use ring_claim::Claimer;
/// use ring_gating::GatingSet;
/// use ring_types::{ Capacity, Seq };
///
/// let consumers = GatingSet::new( Capacity::new( 4 ).unwrap(), 1 );
/// let claimer = Claimer::new( &consumers );
///
/// let claim = claimer.claim( 3 ).expect( "an empty ring has room" );
/// assert_eq!( claim.start(), Seq::ZERO );
/// assert_eq!( claimer.claimed(), Seq( 3 ) );
/// ```
#[derive(Debug)]
pub struct Claimer<'a> {
  cursor: PaddedCursor,
  consumers: &'a GatingSet,
}

impl<'a> Claimer<'a> {
  /// A claimer starting at sequence zero, gated by `consumers`.
  ///
  /// ```
  /// use ring_claim::Claimer;
  /// use ring_gating::GatingSet;
  /// use ring_types::{ Capacity, Seq };
  ///
  /// let consumers = GatingSet::new( Capacity::new( 8 ).unwrap(), 1 );
  /// assert_eq!( Claimer::new( &consumers ).claimed(), Seq::ZERO );
  /// ```
  #[must_use]
  pub fn new(consumers: &'a GatingSet) -> Self {
    Self {
      cursor: PaddedCursor::default(),
      consumers,
    }
  }

  /// The producer cursor, for `ring_publish` to read and for a gating set on
  /// the other side of the ring to be built against.
  ///
  /// # This is a convention, not encapsulation
  ///
  /// `PaddedCursor` implements the public `SeqCell` trait, and every one of
  /// its methods takes `&self`. A `&PaddedCursor` is therefore enough to
  /// `store` the cursor backwards or `fetch_add` it past the gate. The second
  /// is the design this crate's module documentation rejects, and outside code
  /// reaches it in one line of safe code.
  /// `writing_through_the_cursor_accessor_defeats_the_gate` in
  /// `tests/claim_test.rs` demonstrates both failure modes it produces:
  /// a grant past a gate that had just returned `Full`, and two live `Claim`s
  /// that `Claim::overlaps` reports as covering the same sequences.
  ///
  /// Monotonicity is therefore a property of the *methods* `claim`,
  /// `claim_up_to` and `claimed`, not of the cursor itself. Read it, hand it
  /// to `ring_publish`, take its address for a layout assertion; do not write
  /// through it. The crate's manual `§ C2` check greps this crate's own source
  /// for `store` and `fetch_add` and passes. That result is correct and says
  /// nothing about callers.
  ///
  /// ```
  /// use ring_claim::Claimer;
  /// use ring_cursor::SeqCell;
  /// use ring_gating::GatingSet;
  /// use ring_types::{ Capacity, Seq };
  /// use core::sync::atomic::Ordering;
  ///
  /// let consumers = GatingSet::new( Capacity::new( 8 ).unwrap(), 1 );
  /// let claimer = Claimer::new( &consumers );
  /// assert_eq!( claimer.cursor().load( Ordering::Acquire ), Seq::ZERO );
  /// ```
  #[must_use]
  pub const fn cursor(&self) -> &PaddedCursor {
    &self.cursor
  }

  /// The gating set this claimer respects.
  ///
  /// ```
  /// use ring_claim::Claimer;
  /// use ring_gating::GatingSet;
  /// use ring_types::Capacity;
  ///
  /// let consumers = GatingSet::new( Capacity::new( 8 ).unwrap(), 2 );
  /// assert_eq!( Claimer::new( &consumers ).consumers().len(), 2 );
  /// ```
  #[must_use]
  pub const fn consumers(&self) -> &'a GatingSet {
    self.consumers
  }

  /// How far claiming has advanced, as one past the last sequence handed out.
  ///
  /// This is *claimed*, not published, so a slot counted here may still be
  /// mid-write. `ring_publish` tracks the published frontier separately, and
  /// the claim/publish handshake forbids conflating the two.
  ///
  /// ```
  /// use ring_claim::Claimer;
  /// use ring_gating::GatingSet;
  /// use ring_types::{ Capacity, Seq };
  ///
  /// let consumers = GatingSet::new( Capacity::new( 8 ).unwrap(), 1 );
  /// let claimer = Claimer::new( &consumers );
  /// let _claim = claimer.claim( 2 ).unwrap();
  /// assert_eq!( claimer.claimed(), Seq( 2 ) );
  /// ```
  #[must_use]
  pub fn claimed(&self) -> Seq {
    self.cursor.load(GATING)
  }

  /// How many slots could be claimed right now.
  ///
  /// A hint only. Between this reading and a [`claim`] another producer may
  /// take the space. That is why `claim` re-checks rather than trusting a
  /// prior `headroom`.
  ///
  /// Costs one load plus one more per registered consumer. That is cheap for a
  /// diagnostic and worth avoiding in a hot loop with many consumers.
  ///
  /// [`claim`]: Self::claim
  ///
  /// ```
  /// use ring_claim::Claimer;
  /// use ring_gating::GatingSet;
  ///
  /// let consumers = GatingSet::new( ring_types::Capacity::new( 4 ).unwrap(), 1 );
  /// let claimer = Claimer::new( &consumers );
  /// assert_eq!( claimer.headroom(), 4 );
  ///
  /// let _claim = claimer.claim( 3 ).unwrap();
  /// assert_eq!( claimer.headroom(), 1 );
  /// ```
  #[must_use]
  pub fn headroom(&self) -> usize {
    self.consumers.headroom(self.claimed())
  }

  /// Claim exactly `count` contiguous sequences, or fail.
  ///
  /// Never waits and never claims fewer than asked. [`claim_up_to`] is the
  /// partial variant.
  ///
  /// [`claim_up_to`]: Self::claim_up_to
  ///
  /// # Errors
  ///
  /// [`RingError::BatchTooLarge`] when `count` exceeds the ring's capacity.
  /// That is a configuration error no consumer's progress can fix, so a retry
  /// loop must stop. [`RingError::Full`] when the space is not available
  /// *right now*. That is back-pressure, so a retry loop should keep going.
  /// `Full` does not say whether the ring was full at the first look or a peer
  /// took the last room during a retry. Both call for the same wait. One cause
  /// of `Full` never clears by waiting, and [`Claim`]'s pitfall describes it.
  ///
  /// A `count` of zero always succeeds, even on a full ring, because there is
  /// nothing for back-pressure to block. [`claim_up_to`] treats a zero grant
  /// as `Full` instead. The two functions disagree here on purpose. A zero
  /// claim is still not free. It runs the compare-exchange, swapping the cursor
  /// for its own value on the line every producer contends for, while a call
  /// refused at its first look never reaches it.
  ///
  /// ```
  /// use core::sync::atomic::Ordering;
  /// use ring_claim::Claimer;
  /// use ring_cursor::SeqCell;
  /// use ring_gating::GatingSet;
  /// use ring_types::{ Capacity, RingError, Seq };
  ///
  /// let consumers = GatingSet::new( Capacity::new( 4 ).unwrap(), 1 );
  /// let claimer = Claimer::new( &consumers );
  ///
  /// assert_eq!( claimer.claim( 4 ).unwrap().start(), Seq::ZERO );
  /// assert_eq!( claimer.claim( 1 ), Err( RingError::Full ) );
  ///
  /// consumers.cursor( 0 ).unwrap().store( Seq( 2 ), Ordering::Release );
  /// assert_eq!( claimer.claim( 2 ).unwrap().start(), Seq( 4 ) );
  /// ```
  pub fn claim(&self, count: usize) -> Result<Claim, RingError> {
    if count > self.consumers.capacity().get() {
      return Err(RingError::BatchTooLarge {
        requested: count,
        capacity: self.consumers.capacity().get(),
      });
    }

    // The gate is the loop condition, so the loop re-reads it on every
    // iteration. A failed exchange means another producer moved the cursor.
    // The headroom computed against the old value is then stale, and granting
    // on it would overlap that producer's range.
    let mut current = self.claimed();
    while count <= self.consumers.headroom(current) {
      let next = current.advanced_by(count as u64);
      match self.cursor.compare_exchange(current, next, CLAIM_SUCCESS, GATING) {
        Ok(_) => return Ok(Claim::new(current, count)),
        Err(actual) => current = actual,
      }
    }

    Err(RingError::Full)
  }

  /// Claim as many of `max` sequences as are available, down to one.
  ///
  /// For a batching producer that would rather write four items now than wait
  /// for room for eight.
  ///
  /// # Algorithm: the grant shrinks under contention
  ///
  /// The width is recomputed on every retry, as `max` capped by the headroom
  /// at the value the failed exchange returned. A producer that loses the
  /// exchange asks for whatever is left rather than for the same amount, so a
  /// retry can grant less than the first attempt would have. [`claim`] keeps
  /// `count` fixed, so its retry either grants that width or fails.
  ///
  /// # Errors
  ///
  /// [`RingError::Full`] when not even one slot is free. Never
  /// `BatchTooLarge`. A `max` wider than the ring is not an error here, only
  /// more than the call will grant.
  ///
  /// A `max` of zero is also `Full`, since there is no partial success at
  /// zero to report. This differs from [`claim`], which treats a `count`
  /// of zero as always satisfiable.
  ///
  /// # Pitfall: an empty batch reads as a full ring
  ///
  /// **Trap.** Passing a batch's length as `max` and treating `Full` as
  /// back-pressure to retry, as [`claim`] documents it.
  ///
  /// **Failure.** An empty batch makes `max` zero, and a `max` of zero returns
  /// `Full` on any ring, an empty one included. The retry loop waits for a
  /// consumer to free room that is already free, and never exits.
  ///
  /// **Mitigation.** Skip the call when there is nothing to write.
  ///
  /// [`claim`]: Self::claim
  ///
  /// ```
  /// use core::sync::atomic::Ordering;
  /// use ring_claim::Claimer;
  /// use ring_cursor::SeqCell;
  /// use ring_gating::GatingSet;
  /// use ring_types::{ Capacity, RingError, Seq };
  ///
  /// let consumers = GatingSet::new( Capacity::new( 4 ).unwrap(), 1 );
  /// let claimer = Claimer::new( &consumers );
  ///
  /// let claim = claimer.claim_up_to( 100 ).unwrap();
  /// assert_eq!( claim.len(), 4, "capped at what the ring holds" );
  /// assert_eq!( claimer.claim_up_to( 1 ), Err( RingError::Full ) );
  /// ```
  pub fn claim_up_to(&self, max: usize) -> Result<Claim, RingError> {
    // `granted @ 1..` binds the grant and gates on it in one expression. That
    // keeps the headroom re-read in the loop condition instead of duplicated
    // between a pre-loop computation and the retry arm. A grant of zero, from
    // no room or from a `max` of zero, exits to the `Full` below.
    let mut current = self.claimed();
    while let granted @ 1.. = max.min(self.consumers.headroom(current)) {
      let next = current.advanced_by(granted as u64);
      match self.cursor.compare_exchange(current, next, CLAIM_SUCCESS, GATING) {
        Ok(_) => return Ok(Claim::new(current, granted)),
        Err(actual) => current = actual,
      }
    }

    Err(RingError::Full)
  }
}
