//! Single-consumer available-range computation and commit.
//!
//! Part of the ring family's concurrency write path.
//!
//! This crate holds the consumer's two operations in the claim/publish
//! handshake. One says what may be read, the other reports that it has been
//! read. The feature's reached-test wires all four operations together in
//! `ring_publish/tests/handshake_test.rs`.
//!
//! ## Why `available` and `commit` are separate calls
//!
//! A consumer reads a batch and then reports it. Between those, it holds slots
//! the producer must not overwrite, and the only thing preventing that is that
//! its cursor has *not* advanced yet. An `available_and_commit` that did both
//! would advance the cursor before the caller had read a byte, which frees
//! those slots for the producer while they are still being read. That is the
//! exact corruption the gating set exists to prevent, reintroduced above it.
//!
//! The split makes the dangerous window explicit. Everything between the two
//! calls is a read of borrowed slots.
//!
//! The alternative is a guard whose `Drop` performs the commit,
//! `fn read( &self ) -> ReadGuard< '_ >`. It would close the window instead of
//! only naming it, but it has costs of its own: it forces a scope, it makes an
//! early commit impossible rather than only wrong, and it does not compose with
//! a caller that wants to commit part of a run. Those costs are why `available`
//! returns a plain `Copy` value instead.
//!
//! ## Why commit is monotonic and clamped
//!
//! [`Consumer::commit`] refuses to move backwards and refuses to move past what
//! is available. Both refusals matter for the same reason. Neither is defensive
//! programming.
//!
//! Moving backwards re-reads slots the producer has already been told it may
//! reuse. Moving past `available` tells the producer that slots the consumer
//! has not read are free. The producer will believe it, because the consumer
//! cursor *is* the gating signal. From the producer's side, a consumer that
//! over-commits looks the same as one that read faster.
//!
//! ## Why this is single-consumer
//!
//! The crate uses one cursor and plain stores, with no compare-exchange. Two
//! consumers sharing a [`Consumer`] would each advance the same cursor and each
//! believe they had read what the other did. To fan out to several independent
//! consumers, give each its own cursor in the gating set. That is what
//! `ring_gating::GatingSet` already is, and why this crate does not need to
//! know about it.
//!
//! Nothing in the type system enforces this. [`Consumer::new`] takes a
//! `&PaddedCursor`, so calling it twice over the *same* cursor compiles
//! cleanly. The result is two [`Consumer`]s. Each believes it owns the only
//! view, each computes an available run that overlaps the other's, and each
//! commits over the other. That is the premise's exact failure mode. Two lines
//! of safe code reach it, and no check in this crate or the family reports it.
//!
//! ## Why the cursor is borrowed rather than owned
//!
//! [`Consumer::new`] takes a `&PaddedCursor` from somewhere else, and the
//! somewhere else is almost always a producer's `ring_gating::GatingSet`. That
//! is the whole mechanism. The producer decides what it may overwrite by
//! reading the cursors in its set. A consumer whose position lived in a cursor
//! it owned privately would gate nothing, because the producer never reads
//! that cursor. A ring wired that way runs, passes every single-threaded test,
//! and overwrites unread slots on the first lap.

#![deny(missing_docs)]

use ring_barrier::Barrier;
use ring_cursor::{GATING, PaddedCursor, SeqCell};
use ring_types::{RingError, Seq};

/// The ordering a commit is made visible at.
///
/// `Release`, paired with the producer's `Acquire` gating read. The producer
/// must not observe this consumer's advance before the reads that justified it.
/// A `Relaxed` store here lets the producer see freed slots and overwrite them
/// while the reads that freed them are still in flight.
const COMMIT: core::sync::atomic::Ordering = core::sync::atomic::Ordering::Release;

/// A contiguous run of sequences a consumer may read.
///
/// Half-open, like `ring_claim::Claim`, and for the same reason. `end` is
/// directly the sequence to commit once the run has been read.
///
/// ```
/// use ring_consume::Available;
/// use ring_types::Seq;
///
/// let run = Available::new( Seq( 2 ), 3 );
/// assert_eq!( run.start(), Seq( 2 ) );
/// assert_eq!( run.end(), Seq( 5 ) );
/// assert_eq!( run.len(), 3 );
/// assert_eq!( run.sequences().collect::< Vec< _ > >(), vec![ Seq( 2 ), Seq( 3 ), Seq( 4 ) ] );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Available {
  start: Seq,
  len: u64,
}

impl Available {
  /// A run of `len` sequences beginning at `start`.
  ///
  /// ```
  /// use ring_consume::Available;
  /// use ring_types::Seq;
  /// assert!( Available::new( Seq::ZERO, 0 ).is_empty() );
  /// ```
  #[must_use]
  pub const fn new(start: Seq, len: u64) -> Self {
    Self { start, len }
  }

  /// The first readable sequence.
  ///
  /// ```
  /// use ring_consume::Available;
  /// use ring_types::Seq;
  /// assert_eq!( Available::new( Seq( 7 ), 2 ).start(), Seq( 7 ) );
  /// ```
  #[must_use]
  pub const fn start(self) -> Seq {
    self.start
  }

  /// One past the last readable sequence, which is the value to commit.
  ///
  /// ```
  /// use ring_consume::Available;
  /// use ring_types::Seq;
  /// assert_eq!( Available::new( Seq( 7 ), 2 ).end(), Seq( 9 ) );
  /// ```
  #[must_use]
  pub const fn end(self) -> Seq {
    self.start.advanced_by(self.len)
  }

  /// How many sequences are readable.
  ///
  /// ```
  /// use ring_consume::Available;
  /// use ring_types::Seq;
  /// assert_eq!( Available::new( Seq::ZERO, 4 ).len(), 4 );
  /// ```
  #[must_use]
  pub const fn len(self) -> u64 {
    self.len
  }

  /// Whether there is nothing to read.
  ///
  /// ```
  /// use ring_consume::Available;
  /// use ring_types::Seq;
  /// assert!( Available::new( Seq( 3 ), 0 ).is_empty() );
  /// assert!( !Available::new( Seq( 3 ), 1 ).is_empty() );
  /// ```
  #[must_use]
  pub const fn is_empty(self) -> bool {
    self.len == 0
  }

  /// Every readable sequence, in order.
  ///
  /// ```
  /// use ring_consume::Available;
  /// use ring_types::Seq;
  ///
  /// let seen : Vec< u64 > = Available::new( Seq( 5 ), 2 ).sequences().map( | s | s.0 ).collect();
  /// assert_eq!( seen, vec![ 5, 6 ] );
  /// ```
  pub fn sequences(self) -> impl Iterator<Item = Seq> {
    (self.start.0..self.end().0).map(Seq)
  }
}

/// One consumer's position, and the barrier bounding how far it may read.
///
/// ```
/// use core::sync::atomic::Ordering;
/// use ring_barrier::Barrier;
/// use ring_consume::Consumer;
/// use ring_cursor::{ PaddedCursor, SeqCell };
/// use ring_types::Seq;
///
/// let published = [ PaddedCursor::default() ];
/// published[ 0 ].store( Seq( 3 ), Ordering::Release );
///
/// // In a wired ring this cursor comes from the producer's gating set.
/// let position = PaddedCursor::default();
/// let consumer = Consumer::new( &position, Barrier::over( &published ) );
///
/// assert_eq!( consumer.available().len(), 3 );
/// assert_eq!( consumer.commit( Seq( 3 ) ), Ok( Seq( 3 ) ) );
/// assert!( consumer.available().is_empty(), "caught up" );
/// assert_eq!( position.load( Ordering::Acquire ), Seq( 3 ), "the producer sees it" );
/// ```
#[derive(Debug)]
pub struct Consumer<'a> {
  cursor: &'a PaddedCursor,
  barrier: Barrier<'a>,
}

impl<'a> Consumer<'a> {
  /// A consumer reporting into `cursor`, bounded by `barrier`.
  ///
  /// `cursor` is not reset. A consumer built over a cursor that has already
  /// advanced resumes from there, which makes it safe to construct one around a
  /// position the producer is already gating on.
  ///
  /// ```
  /// use core::sync::atomic::Ordering;
  /// use ring_barrier::Barrier;
  /// use ring_consume::Consumer;
  /// use ring_cursor::{ PaddedCursor, SeqCell };
  /// use ring_types::Seq;
  ///
  /// let published = [ PaddedCursor::default() ];
  /// let position = PaddedCursor::new( Seq( 7 ) );
  ///
  /// assert_eq!( Consumer::new( &position, Barrier::over( &published ) ).position(), Seq( 7 ) );
  /// ```
  #[must_use]
  pub const fn new(cursor: &'a PaddedCursor, barrier: Barrier<'a>) -> Self {
    Self { cursor, barrier }
  }

  /// The cursor this consumer reports into, which is the one a producer gates on.
  ///
  /// The producer reads this to decide what it may overwrite, which is why
  /// [`commit`] is the only thing that advances it.
  ///
  /// The returned reference is a full [`SeqCell`], so it also permits a
  /// direct `store`. That store bypasses `commit`'s guard and every guarantee
  /// this type provides. The only sound reason to call this accessor is to
  /// assert wiring identity, as the doctest below does with `ptr::eq`. Never
  /// write through it.
  ///
  /// [`commit`]: Self::commit
  ///
  /// ```
  /// use core::sync::atomic::Ordering;
  /// use ring_barrier::Barrier;
  /// use ring_consume::Consumer;
  /// use ring_cursor::{ PaddedCursor, SeqCell };
  /// use ring_types::Seq;
  ///
  /// let published = [ PaddedCursor::default() ];
  /// let position = PaddedCursor::default();
  /// let consumer = Consumer::new( &position, Barrier::over( &published ) );
  ///
  /// assert_eq!( consumer.cursor().load( Ordering::Acquire ), Seq::ZERO );
  /// assert!( core::ptr::eq( consumer.cursor(), &position ), "the very same cursor" );
  /// ```
  #[must_use]
  pub const fn cursor(&self) -> &'a PaddedCursor {
    self.cursor
  }

  /// The barrier bounding this consumer.
  ///
  /// ```
  /// use ring_barrier::Barrier;
  /// use ring_consume::Consumer;
  /// use ring_cursor::PaddedCursor;
  ///
  /// let published = [ PaddedCursor::default(), PaddedCursor::default() ];
  /// let position = PaddedCursor::default();
  ///
  /// assert_eq!( Consumer::new( &position, Barrier::over( &published ) ).barrier().len(), 2 );
  /// ```
  #[must_use]
  pub const fn barrier(&self) -> Barrier<'a> {
    self.barrier
  }

  /// How far this consumer has committed.
  ///
  /// ```
  /// use ring_barrier::Barrier;
  /// use ring_consume::Consumer;
  /// use ring_cursor::PaddedCursor;
  /// use ring_types::Seq;
  ///
  /// let published = [ PaddedCursor::default() ];
  /// let position = PaddedCursor::default();
  ///
  /// assert_eq!( Consumer::new( &position, Barrier::over( &published ) ).position(), Seq::ZERO );
  /// ```
  #[must_use]
  pub fn position(&self) -> Seq {
    self.cursor.load(GATING)
  }

  /// The run of sequences readable right now.
  ///
  /// Empty when the consumer has caught up. Never includes a
  /// claimed-but-unpublished slot, because the barrier is built over the
  /// *published* cursor. That is the whole of the handshake's first clause. It
  /// is a property of what the barrier was pointed at, not of this function.
  ///
  /// ```
  /// use core::sync::atomic::Ordering;
  /// use ring_barrier::Barrier;
  /// use ring_consume::Consumer;
  /// use ring_cursor::{ PaddedCursor, SeqCell };
  /// use ring_types::Seq;
  ///
  /// let published = [ PaddedCursor::default() ];
  /// let position = PaddedCursor::default();
  /// let consumer = Consumer::new( &position, Barrier::over( &published ) );
  /// assert!( consumer.available().is_empty() );
  ///
  /// published[ 0 ].store( Seq( 5 ), Ordering::Release );
  /// let run = consumer.available();
  /// assert_eq!( ( run.start(), run.end() ), ( Seq::ZERO, Seq( 5 ) ) );
  /// ```
  #[must_use]
  pub fn available(&self) -> Available {
    let position = self.position();
    let readable = self
      .barrier
      .frontier()
      .map_or(0, |frontier| ring_seqno::pending(frontier, position));

    Available::new(position, readable)
  }

  /// At most `max` of what is available, for a consumer with a batch limit.
  ///
  /// ```
  /// use core::sync::atomic::Ordering;
  /// use ring_barrier::Barrier;
  /// use ring_consume::Consumer;
  /// use ring_cursor::{ PaddedCursor, SeqCell };
  /// use ring_types::Seq;
  ///
  /// let published = [ PaddedCursor::default() ];
  /// published[ 0 ].store( Seq( 9 ), Ordering::Release );
  ///
  /// let position = PaddedCursor::default();
  /// let consumer = Consumer::new( &position, Barrier::over( &published ) );
  /// assert_eq!( consumer.available_up_to( 4 ).len(), 4 );
  /// assert_eq!( consumer.available_up_to( 100 ).len(), 9, "capped by what is there" );
  /// ```
  #[must_use]
  pub fn available_up_to(&self, max: u64) -> Available {
    let run = self.available();
    Available::new(run.start(), run.len().min(max))
  }

  /// Report that everything before `through` has been read.
  ///
  /// Frees those slots for the producer, so call it only after the reads are
  /// done. See the module documentation on the window between [`available`]
  /// and here.
  ///
  /// [`available`]: Self::available
  ///
  /// # Errors
  ///
  /// [`RingError::Empty`] when `through` is past what is available, because the
  /// consumer would be freeing slots it has not read. [`RingError::Empty`] also
  /// when `through` is behind the current position, which would re-read slots
  /// the producer has already been cleared to reuse.
  ///
  /// ```
  /// use core::sync::atomic::Ordering;
  /// use ring_barrier::Barrier;
  /// use ring_consume::Consumer;
  /// use ring_cursor::{ PaddedCursor, SeqCell };
  /// use ring_types::{ RingError, Seq };
  ///
  /// let published = [ PaddedCursor::default() ];
  /// published[ 0 ].store( Seq( 4 ), Ordering::Release );
  ///
  /// let position = PaddedCursor::default();
  /// let consumer = Consumer::new( &position, Barrier::over( &published ) );
  ///
  /// assert_eq!( consumer.commit( Seq( 5 ) ), Err( RingError::Empty ), "not read yet" );
  /// assert_eq!( consumer.commit( Seq( 3 ) ), Ok( Seq( 3 ) ) );
  /// assert_eq!( consumer.commit( Seq( 2 ) ), Err( RingError::Empty ), "backwards" );
  /// ```
  ///
  /// The ordinary call site never takes the error arm shown above. A value
  /// read from [`available`] is always still in range when it reaches
  /// `commit`, because between the two calls the run can only grow.
  ///
  /// ```
  /// use core::sync::atomic::Ordering;
  /// use ring_barrier::Barrier;
  /// use ring_consume::Consumer;
  /// use ring_cursor::{ PaddedCursor, SeqCell };
  /// use ring_types::Seq;
  ///
  /// let published = [ PaddedCursor::default() ];
  /// published[ 0 ].store( Seq( 4 ), Ordering::Release );
  ///
  /// let position = PaddedCursor::default();
  /// let consumer = Consumer::new( &position, Barrier::over( &published ) );
  ///
  /// let run = consumer.available();
  /// // ... read the slots in `run` ...
  /// assert_eq!( consumer.commit( run.end() ), Ok( Seq( 4 ) ) );
  /// ```
  pub fn commit(&self, through: Seq) -> Result<Seq, RingError> {
    let run = self.available();
    if through < run.start() || through > run.end() {
      return Err(RingError::Empty);
    }

    self.cursor.store(through, COMMIT);
    Ok(through)
  }

  /// Commit everything currently available, and report how far that reached.
  ///
  /// Call this only when the whole available run has been read. After a
  /// partial read, use [`commit`]`( first_unread )` instead. Committing
  /// everything here tells the producer that slots which were never read are
  /// free to overwrite.
  ///
  /// [`commit`]: Self::commit
  ///
  /// ```
  /// use core::sync::atomic::Ordering;
  /// use ring_barrier::Barrier;
  /// use ring_consume::Consumer;
  /// use ring_cursor::{ PaddedCursor, SeqCell };
  /// use ring_types::Seq;
  ///
  /// let published = [ PaddedCursor::default() ];
  /// published[ 0 ].store( Seq( 6 ), Ordering::Release );
  ///
  /// let position = PaddedCursor::default();
  /// let consumer = Consumer::new( &position, Barrier::over( &published ) );
  /// assert_eq!( consumer.commit_available(), Seq( 6 ) );
  /// assert_eq!( consumer.position(), Seq( 6 ) );
  /// ```
  // Duplicates `commit`'s store rather than delegating to it. `run.end()` is
  // always inside `commit`'s accepted range, so delegating would mean either
  // discarding an unreachable `Err` (`unwrap_or`, itself a smell) or changing
  // this function's return type to `Result` for an error that can never
  // occur. Two stores, kept in step by hand, was judged the smaller cost. So
  // anything added to `commit` later (a debug assertion, a counter, a trace
  // hook) must be added here too, by hand.
  pub fn commit_available(&self) -> Seq {
    let run = self.available();
    let end = run.end();
    debug_assert!(end >= run.start(), "available() must never return end < start");
    if end != run.start() {
      self.cursor.store(end, COMMIT);
    }
    end
  }
}
