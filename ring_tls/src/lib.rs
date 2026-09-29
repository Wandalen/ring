//! Per-thread, bump-allocated, zero-lock append log.
//!
//! Tier 2 of the ring family's 33 crates — the concurrency write-path implementation.
//! Depends on `ring_types`, `ring_atomic` and `ring_batch`.
//!
//! Every thread owns its own buffer and appends to it with no atomics and no
//! mutexes; a single consolidation step later moves each thread's buffer into
//! the ring. The thread-local-buffer half of a mechanism more than one
//! independent consumer needs, factored out to its payload-agnostic append
//! discipline alone — never the payload vocabulary either consumer encodes
//! into it.
//!
//! `docs/feature/175_thread_local_buffer_and_flush_into.md` states the claim in
//! two halves, and the second is what makes the first worth anything:
//!
//! 1. **Accumulation is free.** [`TlsBuffer::push`] performs *zero* atomic
//!    operations. Not "few" — zero, asserted against `ring_atomic`'s
//!    [`CountingSeq`](ring_atomic::CountingSeq).
//! 2. **The landing is one operation.** [`TlsBuffer::flush_into`] moves all `N`
//!    accumulated items into the ring as a **single contiguous claim**, so `N`
//!    items cost one `fetch_add` between them.
//!
//! Half 1 alone would be satisfied by a buffer that then flushed item by item,
//! paying `N` atomics at the end instead of during — the same total traffic,
//! moved rather than removed. Half 2 is what actually removes it, and it is
//! `ring_batch`'s contiguous claim that supplies it.
//!
//! ## What this crate deliberately does not do
//!
//! It does not write to the ring. [`TlsBuffer::flush_into`] takes a cursor and
//! returns the claimed sequences paired with the items — where those items land
//! is `ring_store`'s and `ring_event`'s business. A `TlsBuffer` that knew how
//! to write a slot would be a second write path, and feature 182 asks for
//! exactly one.
//!
//! It also does not decide *when* to flush. The buffer reports full and refuses
//! the push; the policy that reacts is `ring_flush`'s (feature 176).

#![deny(missing_docs)]

use core::sync::atomic::Ordering;

use ring_atomic::SeqCell;
use ring_batch::{BatchClaim, claim};
use ring_types::{RingError, Seq};

/// A thread's private staging area: append cheaply, land once.
///
/// Owned by one thread and never shared — there is no interior mutability and
/// no `Sync` requirement on `T`, because the type's whole reason to exist is
/// that nothing crosses a thread boundary until [`TlsBuffer::flush_into`] runs.
///
/// ```
/// use core::sync::atomic::Ordering;
/// use ring_atomic::{ CountingSeq, SeqCell };
/// use ring_tls::TlsBuffer;
///
/// let cursor = CountingSeq::default();
/// let mut buffer = TlsBuffer::with_capacity( 64 );
///
/// for i in 0..64u32 { buffer.push( i ).unwrap(); }
/// assert_eq!( cursor.counts().total, 0, "accumulation touched no atomic" );
///
/// let flush = buffer.flush_into( &cursor, Ordering::AcqRel );
/// assert_eq!( flush.claim().len(), 64 );
/// assert_eq!( flush.count(), 64 );
/// assert_eq!( cursor.counts().total, 1, "64 items, one atomic operation" );
/// ```
#[derive(Debug)]
pub struct TlsBuffer<T> {
  items: Vec<T>,
  limit: usize,
}

impl<T> TlsBuffer<T> {
  /// A buffer that accepts `limit` items before refusing.
  ///
  /// Bounded on purpose. An unbounded staging buffer converts back-pressure
  /// into memory growth: the ring stops accepting, the buffer keeps taking, and
  /// the failure surfaces as an allocation rather than as the `Full` the
  /// caller's overflow policy knows how to handle.
  ///
  /// The allocation happens here, once, so no `push` can allocate.
  ///
  /// ```
  /// use ring_tls::TlsBuffer;
  /// let buffer = TlsBuffer::< u8 >::with_capacity( 4 );
  /// assert_eq!( buffer.capacity(), 4 );
  /// assert!( buffer.is_empty() );
  /// ```
  #[must_use]
  pub fn with_capacity(limit: usize) -> Self {
    Self {
      items: Vec::with_capacity(limit),
      limit,
    }
  }

  /// Append one item. No atomic, no lock, no allocation.
  ///
  /// # Errors
  ///
  /// [`RingError::Full`] when the buffer already holds `capacity()` items. The
  /// refused item is not returned to the caller: `item` is moved into this
  /// function, and on the refusal path it is bound, never read, and dropped
  /// when the function returns — the same as any other value that goes out of
  /// scope. Callers holding a `T` that owns a resource should check
  /// [`Self::is_full`] before calling.
  ///
  /// This is the sole insertion point into `items`. The zero-allocation
  /// guarantee — `with_capacity` allocates once and nothing after it grows —
  /// rests entirely on the `>=` check above running before every insertion; a
  /// second path into `items` that skipped it would silently restore the
  /// growth this type exists to forbid, since nothing else in the crate
  /// checks for one.
  ///
  /// ```
  /// use ring_tls::TlsBuffer;
  /// use ring_types::RingError;
  ///
  /// let mut buffer = TlsBuffer::with_capacity( 1 );
  /// assert_eq!( buffer.push( 1u8 ), Ok( () ) );
  /// assert_eq!( buffer.push( 2u8 ), Err( RingError::Full ) );
  /// ```
  pub fn push(&mut self, item: T) -> Result<(), RingError> {
    if self.items.len() >= self.limit {
      return Err(RingError::Full);
    }
    let reserved = self.items.capacity();
    self.items.push(item);
    debug_assert!(
      self.items.capacity() == reserved,
      "push must never grow TlsBuffer's allocation past its with_capacity reservation"
    );
    Ok(())
  }

  /// Items staged and not yet flushed.
  #[must_use]
  pub fn len(&self) -> usize {
    self.items.len()
  }

  /// Whether nothing is staged.
  #[must_use]
  pub fn is_empty(&self) -> bool {
    self.items.is_empty()
  }

  /// Whether a further [`TlsBuffer::push`] would be refused.
  ///
  /// The signal `ring_flush`'s `OnFull` policy reads. Exposed rather than left
  /// to a `len() == capacity()` comparison at each call site, so the definition
  /// of full lives with the thing that enforces it.
  ///
  /// ```
  /// use ring_tls::TlsBuffer;
  /// let mut buffer = TlsBuffer::with_capacity( 1 );
  /// assert!( !buffer.is_full() );
  /// buffer.push( 0u8 ).unwrap();
  /// assert!( buffer.is_full() );
  /// ```
  #[must_use]
  pub fn is_full(&self) -> bool {
    self.items.len() >= self.limit
  }

  /// How many items this buffer accepts before refusing.
  #[must_use]
  pub const fn capacity(&self) -> usize {
    self.limit
  }

  /// Discard everything staged without claiming anything.
  ///
  /// The shutdown path: a thread going away with staged items has no ring to
  /// land them in. Distinct from a flush precisely because it advances no
  /// cursor — sequences claimed for items nobody writes would leave a
  /// permanent hole a consumer would wait on forever.
  ///
  /// ```
  /// use ring_tls::TlsBuffer;
  /// let mut buffer = TlsBuffer::with_capacity( 4 );
  /// buffer.push( 1u8 ).unwrap();
  /// buffer.discard();
  /// assert!( buffer.is_empty() );
  /// ```
  pub fn discard(&mut self) {
    self.items.clear();
  }

  /// Take everything staged, in staging order, claiming no sequences.
  ///
  /// The `drain` primitive of the three
  /// [`docs/api/002_consolidator_read_surface.md`](../docs/api/002_consolidator_read_surface.md)
  /// specifies. [`TlsBuffer::flush_into`] is claim-and-drain fused, which is
  /// the right shape when the destination is a sequenced ring; it is the wrong
  /// shape when the destination decides for itself whether it can accept the
  /// batch, because the claim is unconditional and the buffer is emptied
  /// whether the records land or not.
  ///
  /// `ring_flush` is the caller that needs the split: it checks the ring's free
  /// capacity *before* touching the buffer, so that a refusal leaves the
  /// records staged and retryable rather than claimed and stranded
  /// (`ring_flush/docs/algorithm/002_sequencing_seal_drain_reset.md`'s O3/O4).
  ///
  /// **The buffer empties when the returned iterator drops, consumed or not** —
  /// `Vec::Drain`'s own contract, and the reason a caller that might not accept
  /// every record must decide before calling rather than after.
  ///
  /// ```
  /// use ring_tls::TlsBuffer;
  /// let mut buffer = TlsBuffer::with_capacity( 4 );
  /// buffer.push( 'a' ).unwrap();
  /// buffer.push( 'b' ).unwrap();
  ///
  /// let taken : Vec< char > = buffer.drain().collect();
  /// assert_eq!( taken, vec![ 'a', 'b' ] );
  /// assert!( buffer.is_empty(), "drain leaves the buffer writable" );
  /// ```
  pub fn drain(&mut self) -> impl Iterator<Item = T> + '_ {
    self.items.drain(..)
  }

  /// Claim one contiguous run for everything staged, and hand back the items
  /// paired with the sequences they now own.
  ///
  /// One `fetch_add` regardless of how many items are staged — the amortisation
  /// feature 175 is about. The buffer is left empty whether or not the returned
  /// [`Flush`] is fully consumed, because the sequences are already claimed:
  /// abandoning items mid-drain would leave sequences owned by nothing.
  ///
  /// Flushing an empty buffer is legal and claims nothing — a zero-length
  /// `fetch_add` still costs one atomic, which is why a caller in a hot loop
  /// should check [`TlsBuffer::is_empty`] first. This function does not check
  /// on the caller's behalf: a silent skip would make the operation count
  /// depend on the data, and the whole point of the counting shim is that it
  /// does not.
  ///
  /// `order` must include release semantics — `Release`, `AcqRel`, or
  /// `SeqCst` — or the claim still lands correct, non-overlapping sequences
  /// but the items behind them are never made visible to a consumer reading
  /// the cursor. `Relaxed` and `Acquire` compile and return a valid-looking
  /// [`Flush`]; the failure then surfaces downstream, in `ring_flush` or
  /// `ring_store`, as missing or torn data with nothing pointing back here.
  ///
  /// ```
  /// use core::sync::atomic::Ordering;
  /// use ring_atomic::AtomicSeq;
  /// use ring_tls::TlsBuffer;
  /// use ring_types::Seq;
  ///
  /// let cursor = AtomicSeq::new( Seq( 100 ) );
  /// let mut buffer = TlsBuffer::with_capacity( 4 );
  /// buffer.push( 'a' ).unwrap();
  /// buffer.push( 'b' ).unwrap();
  ///
  /// let landed : Vec< _ > = buffer.flush_into( &cursor, Ordering::AcqRel ).collect();
  /// assert_eq!( landed, vec![ ( Seq( 100 ), 'a' ), ( Seq( 101 ), 'b' ) ] );
  /// assert!( buffer.is_empty() );
  /// ```
  pub fn flush_into<C>(&mut self, cursor: &C, order: Ordering) -> Flush<'_, T>
  where
    C: SeqCell,
  {
    debug_assert!(
      matches!(order, Ordering::Release | Ordering::AcqRel | Ordering::SeqCst),
      "flush_into's order must include Release semantics ({order:?} given) or the flushed items are not visible to a consumer reading the cursor"
    );
    let claim = claim(cursor, self.items.len(), order);
    Flush {
      claim,
      next: claim.start().0,
      items: self.items.drain(..),
    }
  }
}

/// The staged items, each paired with the sequence it landed on.
///
/// Yields in staging order against ascending sequences, which is what preserves
/// a thread's own ordering through the merge — hard problem 118's requirement
/// that a system's writes survive consolidation in the order it made them.
///
/// Dropping this without consuming it still empties the buffer and still leaves
/// the cursor advanced; see [`TlsBuffer::flush_into`].
#[derive(Debug)]
pub struct Flush<'a, T> {
  claim: BatchClaim,
  next: u64,
  items: std::vec::Drain<'a, T>,
}

impl<T> Flush<'_, T> {
  /// The sequence range this flush claimed.
  ///
  /// Available before consuming the iterator, so a caller can gate on the range
  /// — check it against a consumer barrier, say — before it starts writing.
  #[must_use]
  pub const fn claim(&self) -> BatchClaim {
    self.claim
  }
}

impl<T> Iterator for Flush<'_, T> {
  type Item = (Seq, T);

  fn next(&mut self) -> Option<Self::Item> {
    if self.next >= self.claim.end().0 {
      return None;
    }
    let item = self.items.next()?;
    let seq = Seq(self.next);
    self.next += 1;
    Some((seq, item))
  }

  fn size_hint(&self) -> (usize, Option<usize>) {
    self.items.size_hint()
  }
}

impl<T> ExactSizeIterator for Flush<'_, T> {}
