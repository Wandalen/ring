//! Consumer barrier over the minimum of dependent cursors.
//!
//! Tier 5 of the ring family's 33 crates, which implement the concurrency write-path.
//! Depends on `ring_types`, `ring_cursor`, `ring_wait`.
//!
//! The sequence barrier has two halves, and this is the consumer's.
//! `ring_gating` asks *how far may the producer advance*; this crate asks *how
//! far may this consumer read*. They read the same kind of cursors and they are
//! not the same question.
//!
//! ## Why this is not `ring_gating` with the sign flipped
//!
//! A gating answer is bounded by the ring's capacity. The producer may run one
//! full lap ahead of the slowest consumer and no further, because at that point
//! the next slot it would claim is the one that consumer is reading. Capacity
//! is in the answer.
//!
//! A barrier answer has no capacity in it at all. A consumer may read up to
//! whatever its dependencies have finished: the producer's published sequence,
//! or an upstream consumer's position in a chain. The number of slots the ring
//! happens to have does not enter into it. Sharing one function between
//! the two would mean one of the callers passing a capacity it does not have,
//! or receiving a bound that has been clamped for a reason that does not apply
//! to it.
//!
//! What they *do* share is the fold over a slice of cursors, which is
//! [`ring_cursor::slowest`] and lives in neither of them.
//!
//! ## Why the dependencies are a bare slice
//!
//! A [`Barrier`] borrows `&[PaddedCursor]`, not a `ring_gating::GatingSet`.
//! A `GatingSet` is a producer-side aggregate. It owns consumer cursors and
//! carries the capacity that bounds the producer, and a barrier's dependencies
//! are neither owned by it nor related to capacity. They are wherever they
//! happen to live: `ring_publish::Publisher::cursor` for a consumer reading
//! behind a producer, `GatingSet::cursors` for a consumer chained behind other
//! consumers, a bare array in a test.
//!
//! Taking the aggregate instead would have meant a publisher's cursor could
//! never be depended on at all, since there is no way to move an existing
//! cursor into a set that owns its own. That is not a hypothetical. It is what
//! made the four-operation handshake in `ring_publish/tests/handshake_test.rs`
//! unwireable until this signature changed.
//!
//! ## Why the empty set has no frontier
//!
//! [`Barrier::frontier`] returns `None` for a barrier with no dependencies, and
//! [`Barrier::available`] then returns zero. That is the opposite of `ring_gating`,
//! where an empty set means *unbounded*. The asymmetry is not an inconsistency.
//! A producer with nobody reading behind it can write freely, while a consumer
//! with nothing published in front of it has nothing to read. In both cases the
//! empty set means "no constraint from dependencies", and in both cases that
//! resolves to the value a dependency-free participant has available.

#![deny(missing_docs)]

use ring_cursor::PaddedCursor;
use ring_types::{RingError, Seq, WaitKind};

/// How far a consumer may read, given what it depends on.
///
/// Borrows its dependencies rather than owning them. The cursors belong to
/// whoever advances them, such as a publisher or an upstream consumer, and a
/// barrier that owned copies would be reading positions nobody was writing.
///
/// ```
/// use core::sync::atomic::Ordering;
/// use ring_barrier::Barrier;
/// use ring_cursor::{ PaddedCursor, SeqCell };
/// use ring_types::Seq;
///
/// let published = [ PaddedCursor::default() ];
/// published[ 0 ].store( Seq( 5 ), Ordering::Release );
///
/// let barrier = Barrier::over( &published );
/// assert_eq!( barrier.frontier(), Some( Seq( 5 ) ) );
/// assert_eq!( barrier.available( Seq( 2 ) ), 3, "sequences 2, 3 and 4" );
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Barrier<'a> {
  dependencies: &'a [PaddedCursor],
}

impl<'a> Barrier<'a> {
  /// A barrier over every cursor in `dependencies`.
  ///
  /// A slice, so that the one cursor a single-producer consumer waits on needs
  /// no aggregate to be wrapped in. `core::slice::from_ref` is the whole of it.
  ///
  /// ```
  /// use ring_barrier::Barrier;
  /// use ring_cursor::PaddedCursor;
  ///
  /// let pair = [ PaddedCursor::default(), PaddedCursor::default() ];
  /// assert_eq!( Barrier::over( &pair ).len(), 2 );
  ///
  /// let one = PaddedCursor::default();
  /// assert_eq!( Barrier::over( core::slice::from_ref( &one ) ).len(), 1 );
  /// ```
  #[must_use]
  pub const fn over(dependencies: &'a [PaddedCursor]) -> Self {
    Self { dependencies }
  }

  /// Every cursor this barrier waits on.
  ///
  /// ```
  /// use ring_barrier::Barrier;
  /// use ring_cursor::PaddedCursor;
  ///
  /// let cursors = [ PaddedCursor::default(), PaddedCursor::default() ];
  /// assert_eq!( Barrier::over( &cursors ).dependencies().len(), 2 );
  /// ```
  #[must_use]
  pub const fn dependencies(&self) -> &'a [PaddedCursor] {
    self.dependencies
  }

  /// How many cursors this barrier waits on.
  ///
  /// ```
  /// use ring_barrier::Barrier;
  /// use ring_cursor::PaddedCursor;
  ///
  /// let cursors = [ PaddedCursor::default(), PaddedCursor::default(), PaddedCursor::default() ];
  /// assert_eq!( Barrier::over( &cursors ).len(), 3 );
  /// assert!( !Barrier::over( &cursors ).is_empty() );
  /// ```
  #[must_use]
  pub const fn len(&self) -> usize {
    self.dependencies.len()
  }

  /// Whether this barrier waits on nothing.
  ///
  /// ```
  /// use ring_barrier::Barrier;
  /// assert!( Barrier::over( &[] ).is_empty() );
  /// ```
  #[must_use]
  pub const fn is_empty(&self) -> bool {
    self.dependencies.is_empty()
  }

  /// One dependency's cursor, for that dependency to advance.
  ///
  /// ```
  /// use ring_barrier::Barrier;
  /// use ring_cursor::PaddedCursor;
  ///
  /// let cursors = [ PaddedCursor::default() ];
  /// let barrier = Barrier::over( &cursors );
  /// assert!( barrier.cursor( 0 ).is_some() );
  /// assert!( barrier.cursor( 1 ).is_none() );
  /// ```
  #[must_use]
  pub fn cursor(&self, index: usize) -> Option<&'a PaddedCursor> {
    self.dependencies.get(index)
  }

  /// The furthest sequence every dependency has reached, or `None` when there
  /// are no dependencies.
  ///
  /// It takes the minimum for the same reason `ring_gating` does. A consumer
  /// that read past *any* dependency would be reading a slot that dependency
  /// has not finished producing or forwarding. One lagging dependency holds the
  /// whole barrier, which is the point of having one.
  ///
  /// ```
  /// use core::sync::atomic::Ordering;
  /// use ring_barrier::Barrier;
  /// use ring_cursor::{ PaddedCursor, SeqCell };
  /// use ring_types::Seq;
  ///
  /// let cursors = [ PaddedCursor::default(), PaddedCursor::default(), PaddedCursor::default() ];
  /// for ( i, cursor ) in cursors.iter().enumerate()
  /// {
  ///   cursor.store( Seq( 10 + i as u64 ), Ordering::Release );
  /// }
  /// assert_eq!( Barrier::over( &cursors ).frontier(), Some( Seq( 10 ) ) );
  /// ```
  #[must_use]
  pub fn frontier(&self) -> Option<Seq> {
    ring_cursor::slowest(self.dependencies)
  }

  /// How many sequences a consumer at `from` may read right now.
  ///
  /// Zero when the barrier has no dependencies. See the module documentation
  /// for why that is not the same answer `ring_gating` gives an empty set.
  ///
  /// ```
  /// use core::sync::atomic::Ordering;
  /// use ring_barrier::Barrier;
  /// use ring_cursor::{ PaddedCursor, SeqCell };
  /// use ring_types::Seq;
  ///
  /// let cursors = [ PaddedCursor::default() ];
  /// cursors[ 0 ].store( Seq( 4 ), Ordering::Release );
  ///
  /// let barrier = Barrier::over( &cursors );
  /// assert_eq!( barrier.available( Seq::ZERO ), 4 );
  /// assert_eq!( barrier.available( Seq( 4 ) ), 0, "caught up" );
  /// assert_eq!( barrier.available( Seq( 9 ) ), 0, "and never negative" );
  /// ```
  #[must_use]
  pub fn available(&self, from: Seq) -> u64 {
    self.frontier().map_or(0, |frontier| from.distance_to(frontier))
  }

  /// Whether a consumer at `from` may read `count` sequences without passing
  /// any dependency.
  ///
  /// ```
  /// use core::sync::atomic::Ordering;
  /// use ring_barrier::Barrier;
  /// use ring_cursor::{ PaddedCursor, SeqCell };
  /// use ring_types::Seq;
  ///
  /// let cursors = [ PaddedCursor::default() ];
  /// cursors[ 0 ].store( Seq( 3 ), Ordering::Release );
  ///
  /// let barrier = Barrier::over( &cursors );
  /// assert!( barrier.admits( Seq::ZERO, 3 ) );
  /// assert!( !barrier.admits( Seq::ZERO, 4 ) );
  /// ```
  #[must_use]
  pub fn admits(&self, from: Seq, count: u64) -> bool {
    count <= self.available(from)
  }

  /// Wait until at least `count` sequences are readable from `from`, then
  /// report the frontier.
  ///
  /// The returned sequence is the frontier as re-read immediately after the
  /// wait succeeded, not the frontier at the exact instant it succeeded. A
  /// dependency may have advanced between the two reads, so the value is
  /// only guaranteed to be at least as far as what was checked. It is also
  /// not `from + count`. A consumer that waited for one item and found six
  /// should drain six, and returning the requested count instead would
  /// throw away the batch that waiting just discovered.
  ///
  /// # Errors
  ///
  /// [`RingError::Empty`] when the `spins` budget runs out with fewer than
  /// `count` available, which for a consumer means exactly what it says.
  ///
  /// An empty barrier (no dependencies) also returns this error for
  /// `count == 0`, even though [`Barrier::admits`] answers `true` for a
  /// zero-length request regardless of dependencies. `admits` only has to
  /// return a bool. This method also has to report a frontier, and an
  /// empty barrier has none to report. Falling back to `from` in that one case
  /// would not generalize what non-empty barriers do, because a non-empty
  /// barrier reports its true frontier at `count == 0` too, never `from`. The
  /// fallback would read as consistent while being a different, fabricated
  /// rule. [`RingError::Empty`] is the honest answer.
  ///
  /// ```
  /// use core::sync::atomic::Ordering;
  /// use ring_barrier::Barrier;
  /// use ring_cursor::{ PaddedCursor, SeqCell };
  /// use ring_types::{ Seq, WaitKind };
  ///
  /// let cursors = [ PaddedCursor::default() ];
  /// let barrier = Barrier::over( &cursors );
  /// assert!( barrier.wait_for( Seq::ZERO, 1, WaitKind::None, 1 ).is_err() );
  ///
  /// cursors[ 0 ].store( Seq( 6 ), Ordering::Release );
  /// assert_eq!( barrier.wait_for( Seq::ZERO, 1, WaitKind::None, 1 ), Ok( Seq( 6 ) ) );
  /// ```
  pub fn wait_for(&self, from: Seq, count: u64, kind: WaitKind, spins: usize) -> Result<Seq, RingError> {
    ring_wait::wait_until(kind, spins, || self.admits(from, count))?;
    self.frontier().ok_or(RingError::Empty)
  }
}
