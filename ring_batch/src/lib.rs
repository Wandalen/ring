//! Batch claim objects spanning a contiguous sequence range.
//!
//! Part of the ring family's concurrency write path.
//!
//! The batch feature states the claim this crate has to make true: "the memory
//! fences that make the handshake correct are paid per operation, not per item,
//! so a batch of sixty-four costs roughly what a single item costs."
//!
//! Measured, the claim holds and improves with more producers. But below
//! roughly eight items there is nothing to amortise. A batch of one costs the
//! same as an unbatched claim of one, and a batch of zero costs a full atomic
//! borne entirely by other threads.
//!
//! Two properties follow, and neither is optional:
//!
//! 1. **One operation, not N.** [`claim`] issues exactly one `fetch_add`
//!    whatever the batch size. Asserted directly, against `ring_atomic`'s
//!    [`CountingSeq`](ring_atomic::CountingSeq), because "roughly what a single
//!    item costs" is otherwise a claim nobody checks.
//! 2. **Contiguous.** The sequences a claim returns are consecutive, which is
//!    what preserves a thread-local buffer's internal order when it lands.
//!    That is the requirement that a system's own writes survive the merge in
//!    order.
//!
//! A [`BatchClaim`] is a *range*, not a buffer. It says which sequences the
//! caller owns; what goes in them is `ring_store`'s and `ring_event`'s
//! business. That split is why this crate needs no storage dependency.

#![deny(missing_docs)]

use core::sync::atomic::Ordering;

use ring_atomic::SeqCell;
use ring_index::of;
use ring_seqno::free_slots;
use ring_types::{Capacity, RingError, Seq, SlotIndex};

/// A contiguous run of sequences one caller owns.
///
/// Ownership here is by construction rather than by enforcement. The cell was
/// advanced past this range, so no other claimer can be handed a sequence
/// inside it.
///
/// ```
/// use ring_batch::BatchClaim;
/// use ring_types::Seq;
///
/// let claim = BatchClaim::new( Seq( 10 ), 3 );
/// assert_eq!( claim.start(), Seq( 10 ) );
/// assert_eq!( claim.len(), 3 );
/// assert_eq!( claim.sequences().collect::< Vec< _ > >(), vec![ Seq( 10 ), Seq( 11 ), Seq( 12 ) ] );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatchClaim {
  start: Seq,
  count: usize,
}

impl BatchClaim {
  /// A claim of `count` sequences beginning at `start`.
  ///
  /// ```
  /// use ring_batch::BatchClaim;
  /// use ring_types::Seq;
  /// assert_eq!( BatchClaim::new( Seq( 0 ), 0 ).len(), 0 );
  /// ```
  #[must_use]
  pub const fn new(start: Seq, count: usize) -> Self {
    Self { start, count }
  }

  /// The first sequence owned.
  #[must_use]
  pub const fn start(&self) -> Seq {
    self.start
  }

  /// How many sequences are owned.
  #[must_use]
  pub const fn len(&self) -> usize {
    self.count
  }

  /// Whether the claim owns nothing.
  ///
  /// An empty claim is legal and distinct from a failed one. Asking for zero
  /// slots succeeds and yields nothing, which lets a flush of an empty
  /// thread-local buffer take the same path as a full one.
  ///
  /// ```
  /// use ring_batch::BatchClaim;
  /// use ring_types::Seq;
  /// assert!( BatchClaim::new( Seq( 4 ), 0 ).is_empty() );
  /// ```
  #[must_use]
  pub const fn is_empty(&self) -> bool {
    self.count == 0
  }

  /// One past the last sequence owned, as a property of this claim alone.
  /// Under contention the cell itself will usually have moved past it by
  /// the time this is read, sometimes by a large margin.
  ///
  /// # Panics
  ///
  /// In a debug build, if `start.0 + count` overflows `u64`. That is unreachable
  /// via the cursor in practice (2⁶⁴ sequences at one claim per nanosecond is
  /// 584 years), but reachable in one line through [`BatchClaim::new`], which is
  /// public, `const`, and takes both fields unvalidated. In a release build
  /// the addition wraps instead of panicking, and every method routing
  /// through `end` (`contains`, `sequences`, `overlaps`) then answers as
  /// though the claim were empty, including for its own `start`.
  ///
  /// ```
  /// use ring_batch::BatchClaim;
  /// use ring_types::Seq;
  /// assert_eq!( BatchClaim::new( Seq( 10 ), 3 ).end(), Seq( 13 ) );
  /// ```
  #[must_use]
  pub const fn end(&self) -> Seq {
    Seq(self.start.0 + self.count as u64)
  }

  /// Whether `seq` is inside this claim.
  ///
  /// ```
  /// use ring_batch::BatchClaim;
  /// use ring_types::Seq;
  ///
  /// let claim = BatchClaim::new( Seq( 10 ), 3 );
  /// assert!( claim.contains( Seq( 12 ) ) );
  /// assert!( !claim.contains( Seq( 13 ) ), "end is exclusive" );
  /// ```
  #[must_use]
  pub const fn contains(&self, seq: Seq) -> bool {
    seq.0 >= self.start.0 && seq.0 < self.end().0
  }

  /// The owned sequences, in issue order.
  ///
  /// Issue order is the point. A batch drain reading these in order is what
  /// makes a staged buffer's contents arrive in the order they were staged.
  ///
  /// ```
  /// use ring_batch::BatchClaim;
  /// use ring_types::Seq;
  ///
  /// let seqs : Vec< _ > = BatchClaim::new( Seq( 5 ), 4 ).sequences().collect();
  /// assert_eq!( seqs, vec![ Seq( 5 ), Seq( 6 ), Seq( 7 ), Seq( 8 ) ] );
  /// ```
  pub fn sequences(&self) -> impl Iterator<Item = Seq> + use<> {
    (self.start.0..self.end().0).map(Seq)
  }

  /// Whether this claim and `other` share any sequence.
  ///
  /// Two overlapping claims mean two producers writing one slot. The
  /// whole-run contention test does not call this. It uses a `HashSet`
  /// over individual sequences instead, a stronger check that also catches
  /// an off-by-one at either end. This method's own two tests exercise it
  /// directly: abutting, shared, self, containment, and the empty-range
  /// guard below.
  ///
  /// ```
  /// use ring_batch::BatchClaim;
  /// use ring_types::Seq;
  ///
  /// let a = BatchClaim::new( Seq( 0 ), 4 );
  /// assert!( !a.overlaps( &BatchClaim::new( Seq( 4 ), 4 ) ) );
  /// assert!( a.overlaps( &BatchClaim::new( Seq( 3 ), 4 ) ) );
  /// ```
  #[must_use]
  pub const fn overlaps(&self, other: &Self) -> bool {
    !self.is_empty() && !other.is_empty() && self.start.0 < other.end().0 && other.start.0 < self.end().0
  }
}

/// Claim `count` contiguous sequences from `cursor`, in one atomic operation.
///
/// This is the amortisation the batch feature is about. The cost is one `fetch_add`
/// whether `count` is 1 or 64. The ordering is the caller's.
/// `ring_atomic`'s module documentation explains why this crate does not pick one.
///
/// Performs **no gating**. A claim taken without consulting a consumer barrier
/// can outrun the ring. [`claim_gated`] checks first, exactly for a single
/// producer and advisory under several (see its own `# One producer only`
/// section below). Both exist because the SPSC path knows its own consumer
/// and the MPSC path does not, and forcing the cheap case through the gated
/// signature would make every SPSC claim pay for a barrier read it does not
/// need.
///
/// ```
/// use core::sync::atomic::Ordering;
/// use ring_atomic::{ CountingSeq, SeqCell };
/// use ring_batch::claim;
/// use ring_types::Seq;
///
/// let cursor = CountingSeq::default();
/// let batch = claim( &cursor, 64, Ordering::AcqRel );
///
/// assert_eq!( batch.start(), Seq::ZERO );
/// assert_eq!( batch.len(), 64 );
/// assert_eq!( cursor.counts().total, 1, "64 slots, one atomic operation" );
/// assert_eq!( cursor.load( Ordering::Acquire ), Seq( 64 ) );
/// ```
#[must_use]
pub fn claim<C: SeqCell>(cursor: &C, count: usize, order: Ordering) -> BatchClaim {
  BatchClaim::new(cursor.fetch_add(count as u64, order), count)
}

/// Claim `count` contiguous sequences only if the ring has room for them.
///
/// The gated form reads the slowest consumer's position, checks that `count`
/// slots are free, and only then advances the cursor. It is still one advancing
/// operation, because the barrier read is a load, not a fence on the claim path.
///
/// `order` governs the advance only. The two gating reads are always
/// `Ordering::Acquire`, chosen here rather than left to the caller, because
/// they are not free choices. Reading the consumer's position exists to
/// establish that its writes happened-before this claim, and a
/// `Relaxed` load would let a producer act on a stale barrier and overwrite a
/// slot the consumer had not finished with. This crate leaves the *advance*
/// ordering open because it varies with the protocol built on top. The
/// gating loads do not vary, so making them a parameter would offer a caller
/// a choice with exactly one correct answer.
///
/// # One producer only
///
/// The gate and the advance are two separate operations, and nothing holds the
/// cursor still between them. A second producer can take the free slots this
/// call just counted, before this call's `fetch_add` runs, and both claims are
/// then granted past the limit. This is safe for a single producer and racy for
/// several. The restriction is a real one rather than a caveat, because the
/// shape that makes it racy is the shape `ring_claim` exists to reject.
///
/// `ring_claim::Claimer::claim` is the multi-producer form. It applies the
/// same two guards, but moves the second inside a compare-exchange retry so
/// the space is re-checked against the value it is about to exchange against,
/// which is the only arrangement that closes this window. Prefer it whenever
/// more than one thread can claim against the same cursor.
///
/// # `producer` and `consumer` must be distinct
///
/// `P` and `C` share one bound and nothing else connects them. Nothing in
/// this signature stops the same cell from being passed as both. Doing so
/// makes `free_slots` see zero in-flight sequences on every call, so the gate
/// always reports the whole capacity free and never returns
/// [`RingError::Full`]. `producer` and `consumer` must be the two distinct ends
/// of one ring.
///
/// # Errors
///
/// - [`RingError::BatchTooLarge`] when `count` exceeds the capacity outright.
///   No amount of draining helps, so this is a configuration error rather than
///   back-pressure, and a caller must not retry it.
/// - [`RingError::Full`] when the ring currently has fewer than `count` free
///   slots. This one *is* back-pressure: the caller retries, waits, or applies
///   its overflow policy.
///
/// Separating the two is what lets a caller loop on one and give up on the
/// other. A single `Full` for both would make an impossible request look like a
/// transient one, and a retry loop would spin forever.
///
/// ```
/// use core::sync::atomic::Ordering;
/// use ring_atomic::{ AtomicSeq, SeqCell };
/// use ring_batch::claim_gated;
/// use ring_types::{ Capacity, RingError, Seq };
///
/// let capacity = Capacity::new( 8 ).unwrap();
/// let producer = AtomicSeq::default();
/// let consumer = AtomicSeq::default();
///
/// let batch = claim_gated( &producer, &consumer, 8, capacity, Ordering::AcqRel ).unwrap();
/// assert_eq!( batch.len(), 8 );
///
/// // The ring is now full: nothing has been consumed.
/// assert_eq!
/// (
///   claim_gated( &producer, &consumer, 1, capacity, Ordering::AcqRel ),
///   Err( RingError::Full )
/// );
///
/// // Asking for more than the ring holds is a different failure entirely.
/// assert_eq!
/// (
///   claim_gated( &producer, &consumer, 9, capacity, Ordering::AcqRel ),
///   Err( RingError::BatchTooLarge { requested : 9, capacity : 8 } )
/// );
/// ```
pub fn claim_gated<P: SeqCell, C: SeqCell>(
  producer: &P,
  consumer: &C,
  count: usize,
  capacity: Capacity,
  order: Ordering,
) -> Result<BatchClaim, RingError> {
  if count > capacity.get() {
    return Err(RingError::BatchTooLarge {
      requested: count,
      capacity: capacity.get(),
    });
  }

  let at = producer.load(Ordering::Acquire);
  let behind = consumer.load(Ordering::Acquire);
  if (free_slots(at, behind, capacity) as usize) < count {
    return Err(RingError::Full);
  }

  Ok(claim(producer, count, order))
}

/// The sequences of `claim`, paired with the slot each addresses, in issue
/// order.
///
/// The drain side of the batch feature. Returning the pair rather than just the
/// slot index keeps the sequence available to the consumer, which needs it to
/// advance its own cursor and to detect that it has been lapped.
///
/// ```
/// use ring_batch::{ drain_order, BatchClaim };
/// use ring_types::{ Capacity, Seq, SlotIndex };
///
/// let capacity = Capacity::new( 4 ).unwrap();
/// let claim = BatchClaim::new( Seq( 2 ), 4 );
///
/// let drained : Vec< _ > = drain_order( &claim, capacity ).collect();
/// assert_eq!
/// (
///   drained,
///   vec!
///   [
///     ( Seq( 2 ), SlotIndex( 2 ) ),
///     ( Seq( 3 ), SlotIndex( 3 ) ),
///     ( Seq( 4 ), SlotIndex( 0 ) ),
///     ( Seq( 5 ), SlotIndex( 1 ) ),
///   ]
/// );
/// ```
pub fn drain_order(claim: &BatchClaim, capacity: Capacity) -> impl Iterator<Item = (Seq, SlotIndex)> + use<> {
  claim.sequences().map(move |seq| (seq, of(seq, capacity)))
}
