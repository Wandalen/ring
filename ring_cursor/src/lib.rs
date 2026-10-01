//! Producer and consumer sequence cursors, cache-line separated.
//!
//! Tier 3 of the ring family's 33 crates — the concurrency write-path implementation.
//! Depends on `ring_types`, `ring_seqno`, `ring_atomic`, `ring_align`.
//!
//! `docs/feature/169_padded_cursor.md` states the whole subject: a producer
//! cursor and a consumer cursor that land on one cache line make every write by
//! either invalidate the other's cached copy, so two cores contend on a line
//! neither is sharing data through. Throughput then falls as core count rises,
//! which is the opposite of what adding cores is for.
//!
//! ## The padding is the crate, and it is one attribute
//!
//! `ring_align` holds the attribute; this crate holds the cursors that wear it.
//! That split looks like ceremony until you notice what it buys: the padding
//! decision is made in one place, for one reason, and every cursor in the
//! family inherits it without a second author deciding 64 was probably fine.
//! [`PaddedCursor`] is [`ring_atomic::AtomicSeq`] inside
//! [`ring_align::CacheAligned`] and nothing else — no field of its own, no
//! logic of its own.
//!
//! ## Why size and alignment are both asserted
//!
//! Alignment alone does not separate two cursors. A 64-aligned type of size 8
//! placed in an array would still put two neighbours 8 bytes apart, because
//! alignment constrains where a value may *start*, not how much room it takes.
//! `#[ repr( align( 64 ) ) ]` happens to round the size up too, so both hold —
//! but the acceptance criterion names both because only their conjunction says
//! "one per line", and a future layout change could break the second while
//! leaving the first intact.
//!
//! [`CursorPair::on_distinct_lines`] is the third assertion, and the only one
//! taken from real addresses rather than from the type. `size_of` is a promise
//! about a type; two fields being 64 bytes apart is the fact the promise was
//! made about.
//!
//! ## Which orderings this crate names and which it fixes
//!
//! `ring_atomic` refuses to choose an ordering, because for a bare cell the
//! caller genuinely has a choice and a defaulted `SeqCst` would make every
//! benchmark meaningless. [`PaddedCursor`] inherits that: its [`SeqCell`] impl
//! forwards whatever the caller names.
//!
//! [`CursorPair`]'s gating readings are the opposite case, and fix `Acquire`
//! rather than take a parameter — the same choice `ring_batch::claim_gated`
//! makes, for the same reason. A caller asking "may I claim?" is about to
//! overwrite a slot on the answer; a `Relaxed` load there would let it act on a
//! stale barrier and overwrite a slot the consumer had not finished with. There
//! is no legitimate second option to offer, so offering one would only be a way
//! to get it wrong.

#![deny(missing_docs)]

use core::sync::atomic::Ordering;

use ring_align::{CacheAligned, on_distinct_lines};
use ring_atomic::AtomicSeq;
/// Re-exported from `ring_atomic`, because a [`PaddedCursor`] is unusable
/// without it.
///
/// Every read and write of a cursor is a [`SeqCell`] method, so a crate holding
/// a `PaddedCursor` and not this trait holds a value it cannot load. Making
/// each such crate declare `ring_atomic` itself would put a dependency in
/// four manifests to import one trait — and would say, wrongly, that those
/// crates have business with the atomic layer beyond the cursor they were
/// handed.
pub use ring_atomic::SeqCell;
use ring_types::{Capacity, Seq};

/// The ordering every gating read in the family uses.
///
/// Named rather than written inline at each of the seven individual loads
/// across four functions (`slowest`, `free_slots`, `pending`, `may_claim`), so
/// that the decision argued in the module documentation has one place to be
/// changed and one place to be read.
///
/// Public because the same decision governs every crate that reads a cursor to
/// decide whether a slot is safe to touch — `ring_claim`, `ring_consume`,
/// `ring_mpsc`, `ring_publish` and `ring_spsc` all import it for exactly that
/// reason. Each writing `Ordering::Acquire` inline would be the same argument
/// made independently in several places, which is how a family ends up with
/// one crate relaxed and the rest not.
///
/// ```
/// use core::sync::atomic::Ordering;
/// assert_eq!( ring_cursor::GATING, Ordering::Acquire );
/// ```
pub const GATING: Ordering = Ordering::Acquire;

/// The position of the furthest-behind cursor, or `None` when there are none.
///
/// Two crates ask this question of the same kind of slice and mean opposite
/// things by the answer — `ring_gating` reads a set of consumers to bound a
/// producer, `ring_barrier` reads a set of dependencies to bound a consumer.
/// The *questions* differ, and are argued in those crates; the fold does not,
/// and lives here so that "read every cursor at [`GATING`] and take the
/// minimum" is written once. A second copy is how one of them ends up reading
/// `Relaxed`.
///
/// `None` rather than [`Seq::ZERO`] for an empty slice, because the two callers
/// resolve *no dependencies* to opposite values — full headroom on one side,
/// nothing readable on the other — and a fold that picked either would be
/// wrong for one of them.
///
/// ```
/// use core::sync::atomic::Ordering;
/// use ring_cursor::{ PaddedCursor, SeqCell };
/// use ring_types::Seq;
///
/// assert_eq!( ring_cursor::slowest( &[] ), None );
///
/// let cursors = [ PaddedCursor::new( Seq( 9 ) ), PaddedCursor::new( Seq( 4 ) ) ];
/// assert_eq!( ring_cursor::slowest( &cursors ), Some( Seq( 4 ) ) );
///
/// cursors[ 1 ].store( Seq( 12 ), Ordering::Release );
/// assert_eq!( ring_cursor::slowest( &cursors ), Some( Seq( 9 ) ) );
/// ```
#[must_use]
pub fn slowest(cursors: &[PaddedCursor]) -> Option<Seq> {
  cursors.iter().map(|c| c.load(GATING)).min()
}

/// One atomic sequence with a cache line to itself.
///
/// `size_of` and `align_of` are both [`ring_align::CACHE_LINE`], which is what
/// makes two of them in one struct land on different lines rather than merely
/// at different addresses.
///
/// ```
/// use core::sync::atomic::Ordering;
/// use ring_atomic::SeqCell;
/// use ring_cursor::PaddedCursor;
/// use ring_types::Seq;
///
/// let cursor = PaddedCursor::new( Seq( 5 ) );
/// assert_eq!( cursor.load( Ordering::Acquire ), Seq( 5 ) );
///
/// assert_eq!( core::mem::size_of::< PaddedCursor >(), 64 );
/// assert_eq!( core::mem::align_of::< PaddedCursor >(), 64 );
/// ```
#[derive(Debug, Default)]
pub struct PaddedCursor(CacheAligned<AtomicSeq>);

impl PaddedCursor {
  /// A cursor at `value`.
  ///
  /// `const` in an ordinary build, and not under `--cfg loom` — inherited from
  /// [`ring_atomic::AtomicSeq::new`], where the seam and its cost are argued.
  ///
  /// ```
  /// use ring_cursor::PaddedCursor;
  /// use ring_types::Seq;
  /// let _ = PaddedCursor::new( Seq( 1 ) );
  /// ```
  #[cfg(not(loom))]
  #[must_use]
  pub const fn new(value: Seq) -> Self {
    Self(CacheAligned::new(AtomicSeq::new(value)))
  }

  /// A cursor at `value` — the `--cfg loom` build, where it is not `const`.
  #[cfg(loom)]
  #[must_use]
  pub fn new(value: Seq) -> Self {
    Self(CacheAligned::new(AtomicSeq::new(value)))
  }

  /// Where this cursor sits in memory.
  ///
  /// The input to [`CursorPair::on_distinct_lines`], and the only way to check
  /// the padding against reality rather than against `size_of`. Returned as a
  /// plain integer because the answer is arithmetic on line numbers, not
  /// anything a caller should dereference.
  ///
  /// ```
  /// use ring_cursor::PaddedCursor;
  ///
  /// let cursor = PaddedCursor::default();
  /// assert_eq!( cursor.addr() % 64, 0, "a 64-aligned value starts on a line boundary" );
  /// ```
  #[must_use]
  pub fn addr(&self) -> usize {
    core::ptr::from_ref(self) as usize
  }
}

// Each forward below assumes `CacheAligned::get` stays a free, no-op
// accessor. A debug assertion or a counter added to it in `ring_align` would
// add that cost to all four methods here, and nothing in this crate's suite
// would notice — every assertion here is about layout or arithmetic, never
// about cost; the assumption is cheap by convention across the crate
// boundary, with no contract enforcing it on either side.
impl SeqCell for PaddedCursor {
  fn load(&self, order: Ordering) -> Seq {
    self.0.get().load(order)
  }

  fn store(&self, value: Seq, order: Ordering) {
    self.0.get().store(value, order);
  }

  fn fetch_add(&self, n: u64, order: Ordering) -> Seq {
    self.0.get().fetch_add(n, order)
  }

  fn compare_exchange(&self, current: Seq, new: Seq, success: Ordering, failure: Ordering) -> Result<Seq, Seq> {
    self.0.get().compare_exchange(current, new, success, failure)
  }

  fn compare_exchange_weak(&self, current: Seq, new: Seq, success: Ordering, failure: Ordering) -> Result<Seq, Seq> {
    self.0.get().compare_exchange_weak(current, new, success, failure)
  }
}

/// A ring's two cursors, on two cache lines, with the capacity that relates
/// them.
///
/// The capacity is held here rather than passed to each reading because a pair
/// is always a pair *for* a ring of some size: every question worth asking of
/// two cursors — how many slots are free, how many items are pending, whether a
/// claim is safe — is unanswerable without it, and a caller supplying it per
/// call could supply a different one each time.
///
/// ```
/// use ring_atomic::SeqCell;
/// use ring_cursor::CursorPair;
/// use ring_types::{ Capacity, Seq };
///
/// let pair = CursorPair::new( Capacity::new( 4 ).unwrap() );
/// assert_eq!( pair.free_slots(), 4 );
/// assert!( pair.may_claim() );
///
/// pair.producer().store( Seq( 4 ), core::sync::atomic::Ordering::Release );
/// assert_eq!( pair.free_slots(), 0, "a full lap ahead is a full ring" );
/// assert!( !pair.may_claim() );
/// assert_eq!( pair.pending(), 4 );
/// ```
#[derive(Debug)]
pub struct CursorPair {
  producer: PaddedCursor,
  consumer: PaddedCursor,
  capacity: Capacity,
}

impl CursorPair {
  /// Both cursors at zero, for a ring of `capacity` slots.
  ///
  /// `const` in an ordinary build, and not under `--cfg loom` — inherited from
  /// [`PaddedCursor::new`].
  ///
  /// ```
  /// use ring_cursor::CursorPair;
  /// use ring_types::Capacity;
  ///
  /// let pair = CursorPair::new( Capacity::new( 8 ).unwrap() );
  /// assert_eq!( pair.capacity().get(), 8 );
  /// ```
  #[cfg(not(loom))]
  #[must_use]
  pub const fn new(capacity: Capacity) -> Self {
    Self {
      producer: PaddedCursor::new(Seq::ZERO),
      consumer: PaddedCursor::new(Seq::ZERO),
      capacity,
    }
  }

  /// Both cursors at zero — the `--cfg loom` build, where it is not `const`.
  #[cfg(loom)]
  #[must_use]
  pub fn new(capacity: Capacity) -> Self {
    Self {
      producer: PaddedCursor::new(Seq::ZERO),
      consumer: PaddedCursor::new(Seq::ZERO),
      capacity,
    }
  }

  /// How far the producer has published.
  ///
  /// `GATING` is fixed only for the pair's own three readings
  /// (`free_slots`, `pending`, `may_claim`) — this accessor hands back the
  /// raw cursor itself, whose `SeqCell` impl forwards whatever ordering the
  /// caller names. The pair decides for the questions it answers, not for
  /// the fields it lends out.
  ///
  /// ```
  /// use core::sync::atomic::Ordering;
  /// use ring_atomic::SeqCell;
  /// use ring_cursor::CursorPair;
  /// use ring_types::{ Capacity, Seq };
  ///
  /// let pair = CursorPair::new( Capacity::new( 2 ).unwrap() );
  /// assert_eq!( pair.producer().load( Ordering::Acquire ), Seq::ZERO );
  /// ```
  #[must_use]
  pub const fn producer(&self) -> &PaddedCursor {
    &self.producer
  }

  /// How far the consumer has read.
  ///
  /// Same boundary as [`producer`](Self::producer): fixed `GATING` covers
  /// this pair's own readings, not a direct load or store through the
  /// returned cursor.
  ///
  /// ```
  /// use core::sync::atomic::Ordering;
  /// use ring_atomic::SeqCell;
  /// use ring_cursor::CursorPair;
  /// use ring_types::{ Capacity, Seq };
  ///
  /// let pair = CursorPair::new( Capacity::new( 2 ).unwrap() );
  /// pair.consumer().store( Seq( 1 ), Ordering::Release );
  /// assert_eq!( pair.consumer().load( Ordering::Acquire ), Seq( 1 ) );
  /// ```
  #[must_use]
  pub const fn consumer(&self) -> &PaddedCursor {
    &self.consumer
  }

  /// The ring size these two cursors are positions in.
  ///
  /// ```
  /// use ring_cursor::CursorPair;
  /// use ring_types::Capacity;
  ///
  /// assert_eq!( CursorPair::new( Capacity::new( 16 ).unwrap() ).capacity().get(), 16 );
  /// ```
  #[must_use]
  pub const fn capacity(&self) -> Capacity {
    self.capacity
  }

  /// How many slots a producer may still publish into.
  ///
  /// Both cursors are read `Acquire` — see the module documentation for why
  /// this is not a parameter.
  ///
  /// ```
  /// use core::sync::atomic::Ordering;
  /// use ring_atomic::SeqCell;
  /// use ring_cursor::CursorPair;
  /// use ring_types::{ Capacity, Seq };
  ///
  /// let pair = CursorPair::new( Capacity::new( 4 ).unwrap() );
  /// pair.producer().store( Seq( 3 ), Ordering::Release );
  /// assert_eq!( pair.free_slots(), 1 );
  /// ```
  #[must_use]
  pub fn free_slots(&self) -> usize {
    ring_seqno::free_slots(self.producer.load(GATING), self.consumer.load(GATING), self.capacity)
  }

  /// How many published items the consumer has not yet read.
  ///
  /// ```
  /// use core::sync::atomic::Ordering;
  /// use ring_atomic::SeqCell;
  /// use ring_cursor::CursorPair;
  /// use ring_types::{ Capacity, Seq };
  ///
  /// let pair = CursorPair::new( Capacity::new( 8 ).unwrap() );
  /// pair.producer().store( Seq( 5 ), Ordering::Release );
  /// pair.consumer().store( Seq( 2 ), Ordering::Release );
  /// assert_eq!( pair.pending(), 3 );
  /// ```
  #[must_use]
  pub fn pending(&self) -> u64 {
    ring_seqno::pending(self.producer.load(GATING), self.consumer.load(GATING))
  }

  /// Whether a producer may claim without overwriting a slot the consumer has
  /// not reached.
  ///
  /// Exactly [`free_slots`] being non-zero, expressed as the question a caller
  /// actually asks. Kept as its own method because the two readings answer
  /// different questions — "how much room" is a batch-sizing input, "may I"
  /// is a branch — and a caller that only needs the branch should not have to
  /// know that zero is the boundary.
  ///
  /// [`free_slots`]: Self::free_slots
  ///
  /// ```
  /// use core::sync::atomic::Ordering;
  /// use ring_atomic::SeqCell;
  /// use ring_cursor::CursorPair;
  /// use ring_types::{ Capacity, Seq };
  ///
  /// let pair = CursorPair::new( Capacity::new( 4 ).unwrap() );
  /// pair.producer().store( Seq( 4 ), Ordering::Release );
  /// assert!( !pair.may_claim(), "exactly one lap ahead is full" );
  ///
  /// pair.consumer().store( Seq( 1 ), Ordering::Release );
  /// assert!( pair.may_claim(), "the consumer moved on" );
  /// ```
  #[must_use]
  pub fn may_claim(&self) -> bool {
    ring_seqno::may_claim(self.producer.load(GATING), self.consumer.load(GATING), self.capacity)
  }

  /// Whether the two cursors actually occupy different cache lines.
  ///
  /// The feature's claim, checked against two real addresses rather than
  /// against the type's declared size. Always true for a `CursorPair` on the
  /// family's targets; exposed so a test can say so about a value rather than
  /// about a promise, and so a future layout change that silently packs the two
  /// together has something that fails.
  ///
  /// ```
  /// use ring_cursor::CursorPair;
  /// use ring_types::Capacity;
  ///
  /// let pair = CursorPair::new( Capacity::new( 2 ).unwrap() );
  /// assert!( pair.on_distinct_lines() );
  /// ```
  #[must_use]
  pub fn on_distinct_lines(&self) -> bool {
    on_distinct_lines(self.producer.addr(), self.consumer.addr())
  }
}
