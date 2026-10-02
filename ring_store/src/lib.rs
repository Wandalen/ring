//! Power-of-two slot storage array.
//!
//! Part of the ring family's concurrency write path.
//!
//! The ring-buffer-storage feature defines this crate by what it
//! *refuses* to hold: "the buffer itself, and nothing else — a fixed allocation
//! of slots addressed by slot index. It holds no cursors, enforces no ordering,
//! and knows nothing about producers or consumers."
//!
//! That refusal is the whole design. The hard problem is how several rings
//! with different protocols share one storage type, and the answer is that
//! storage holds nothing that would differ between them. The moment a cursor
//! lands here, the buffer stops being shareable. An SPSC ring and an MPSC ring
//! want different cursor arrangements, so a buffer that owns one serves exactly
//! one of them.
//!
//! What is left is small enough to state completely: `capacity` slots allocated
//! once, addressed by [`ring_types::SlotIndex`], with the fold from a sequence
//! delegated to `ring_index`. There is no `unsafe`. A `Box<[S]>` of `Default`
//! slots is allocated in one go, which is what "exactly N slots once" asks for.

#![deny(missing_docs)]

use ring_index::of;
use ring_slot::Slot;
use ring_types::{Capacity, Seq, SlotIndex};

/// A fixed array of slots, addressed by index.
///
/// Generic over the slot shape rather than the payload, so one buffer type
/// serves both `ring_slot::TypedSlot` and `ring_slot::BytesSlot`. The slot
/// feature's constraint that "both use the same claim, gating and drain"
/// reaches down to storage too.
///
/// ```
/// use ring_store::Buffer;
/// use ring_slot::TypedSlot;
/// use ring_types::{ Capacity, SlotIndex };
///
/// let mut buffer : Buffer< TypedSlot< u32 > > = Buffer::new( Capacity::new( 4 ).unwrap() );
/// assert_eq!( buffer.capacity().get(), 4 );
///
/// buffer.get_mut( SlotIndex( 2 ) ).set( 7 );
/// assert_eq!( buffer.get( SlotIndex( 2 ) ).get(), Some( &7 ) );
/// ```
///
/// # The derived `Debug` renders every slot
///
/// `{ :? }` on a `Buffer` walks the full allocation. That is megabytes of output
/// for a realistic ring, and it prints every slot's current payload, including
/// records a consumer has already read but not taken out of the slot.
/// `BytesSlot`'s own `Debug` shows only the first `len` bytes, so bytes a
/// `clear` left behind do not appear.
/// `ring_spsc` and `ring_mpsc` both avoid this by giving their own ring type a
/// hand-written `Debug` that prints cursor positions only, never slot contents.
/// A type that embeds a `Buffer` and derives `Debug` in turn inherits this cost
/// and this leak silently.
#[derive(Debug)]
pub struct Buffer<S> {
  slots: Box<[S]>,
  capacity: Capacity,
}

impl<S: Default> Buffer<S> {
  /// Allocate exactly `capacity` empty slots, once.
  ///
  /// Bounded on `Default` alone rather than on `Slot`, because allocation needs
  /// nothing a slot offers beyond an empty value. The two crates that assemble a
  /// ring rely on that. They store `Buffer< UnsafeCell< S > >`, putting the cell
  /// on each slot rather than around the whole buffer, and `UnsafeCell< S >` is
  /// `Default` without being a `Slot`. Wrapping the buffer instead would mean a
  /// producer forming `&mut Buffer`, an exclusive claim over the *entire*
  /// allocation, to write one slot, and two producers writing different slots
  /// violate that. Per-slot cells make each write claim exactly the slot it
  /// touches. See [`Buffer::clear`] for the operations that do still need `Slot`.
  ///
  /// ```
  /// use ring_store::Buffer;
  /// use ring_slot::BytesSlot;
  /// use ring_types::Capacity;
  ///
  /// let buffer : Buffer< BytesSlot< 8 > > = Buffer::new( Capacity::new( 16 ).unwrap() );
  /// assert_eq!( buffer.len(), 16 );
  /// ```
  #[must_use]
  pub fn new(capacity: Capacity) -> Self {
    let mut slots = Vec::with_capacity(capacity.get());
    slots.resize_with(capacity.get(), S::default);
    Self {
      slots: slots.into_boxed_slice(),
      capacity,
    }
  }
}

impl<S: Slot + Default> Buffer<S> {
  /// Empty every slot, keeping the allocation.
  ///
  /// A reset for a recycled ring. Reallocating would defeat the allocation
  /// behaviour the ring was chosen for, so this sweeps every slot back to
  /// empty in place instead. No consumer in the family calls this today.
  /// `ring_shutdown` does not depend on this crate, and its reopen story is a
  /// different state machine, over `Stopped`, not a payload sweep. The
  /// emptiness delivered is only as strong as `Slot::clear` for the shape in
  /// use: a `TypedSlot`'s previous payload is dropped, but a `BytesSlot`'s
  /// bytes stay resident and only the length marking them unreachable moves.
  ///
  /// ```
  /// use ring_store::Buffer;
  /// use ring_slot::{ Slot, TypedSlot };
  /// use ring_types::{ Capacity, SlotIndex };
  ///
  /// let mut buffer : Buffer< TypedSlot< u8 > > = Buffer::new( Capacity::new( 2 ).unwrap() );
  /// buffer.get_mut( SlotIndex( 0 ) ).set( 1 );
  /// buffer.clear();
  /// assert!( buffer.get( SlotIndex( 0 ) ).is_empty() );
  /// ```
  pub fn clear(&mut self) {
    for slot in &mut self.slots {
      slot.clear();
    }
  }

  /// Whether every slot is empty.
  ///
  /// ```
  /// use ring_store::Buffer;
  /// use ring_slot::TypedSlot;
  /// use ring_types::Capacity;
  ///
  /// let buffer : Buffer< TypedSlot< u8 > > = Buffer::new( Capacity::new( 2 ).unwrap() );
  /// assert!( buffer.all_empty() );
  /// ```
  #[must_use]
  pub fn all_empty(&self) -> bool {
    self.slots.iter().all(Slot::is_empty)
  }
}

impl<S> Buffer<S> {
  /// The capacity this buffer was built with.
  #[must_use]
  pub const fn capacity(&self) -> Capacity {
    self.capacity
  }

  /// Slots allocated, always equal to `capacity().get()` and never more.
  ///
  /// It is a separate reading from [`Buffer::capacity`] so a test can assert the
  /// two agree. A buffer that over-allocated would still report the requested
  /// capacity.
  ///
  /// ```
  /// use ring_store::Buffer;
  /// use ring_slot::TypedSlot;
  /// use ring_types::Capacity;
  ///
  /// let buffer : Buffer< TypedSlot< u8 > > = Buffer::new( Capacity::new( 8 ).unwrap() );
  /// assert_eq!( buffer.len(), buffer.capacity().get() );
  /// ```
  #[must_use]
  pub const fn len(&self) -> usize {
    self.slots.len()
  }

  /// Always false, since a `Capacity` cannot be zero and so a buffer always has slots.
  ///
  /// Exists because [`Buffer::len`] does; a `len` without an `is_empty` is a
  /// lint, and a hand-written `is_empty` that could disagree with `len` is
  /// worse than one that provably cannot.
  ///
  /// ```
  /// use ring_store::Buffer;
  /// use ring_slot::TypedSlot;
  /// use ring_types::Capacity;
  ///
  /// let buffer : Buffer< TypedSlot< u8 > > = Buffer::new( Capacity::new( 1 ).unwrap() );
  /// assert!( !buffer.is_empty() );
  /// ```
  #[must_use]
  pub const fn is_empty(&self) -> bool {
    self.slots.is_empty()
  }

  /// Borrow the slot at `index`.
  ///
  /// # Panics
  ///
  /// If `index` is at or beyond the capacity. This is not an error return. A
  /// `SlotIndex` is not validated against this buffer's capacity, and one built
  /// any other way than through `ring_index::of` for this same capacity is the
  /// caller's responsibility. An index that reaches here out of range comes
  /// from a caller that mixed two rings' capacities, which is a defect rather
  /// than a condition to handle.
  #[must_use]
  pub fn get(&self, index: SlotIndex) -> &S {
    &self.slots[index.get()]
  }

  /// Mutably borrow the slot at `index`.
  ///
  /// # Panics
  ///
  /// As [`Buffer::get`].
  #[must_use]
  pub fn get_mut(&mut self, index: SlotIndex) -> &mut S {
    &mut self.slots[index.get()]
  }

  /// Borrow the slot a sequence addresses, folding through `ring_index`.
  ///
  /// This convenience keeps the fold in one place. A caller that wrote its
  /// own `seq % capacity` here would be the second implementation of the thing
  /// `ring_index` exists to be the only one of.
  ///
  /// An ordinary shared borrow, as far as this crate is concerned. `ring_mpsc`
  /// and `ring_spsc` read more into it. With `S = UnsafeCell< T >` both call
  /// this under a claim guaranteeing no other caller holds the same `seq`, then
  /// `unsafe { &mut *at( seq ).get() }` the result into a `&mut T`. This
  /// function grants nothing beyond the one shared borrow it returns. The
  /// exclusivity that makes the consumers' unsafe deref sound is a contract
  /// they hold, and this crate neither enforces it nor sees it.
  ///
  /// ```
  /// use ring_store::Buffer;
  /// use ring_slot::TypedSlot;
  /// use ring_types::{ Capacity, Seq, SlotIndex };
  ///
  /// let mut buffer : Buffer< TypedSlot< u8 > > = Buffer::new( Capacity::new( 4 ).unwrap() );
  /// buffer.at_mut( Seq( 6 ) ).set( 1 );
  /// // 6 folds to slot 2.
  /// assert_eq!( buffer.get( SlotIndex( 2 ) ).get(), Some( &1 ) );
  /// ```
  #[must_use]
  pub fn at(&self, seq: Seq) -> &S {
    self.get(of(seq, self.capacity))
  }

  /// Mutably borrow the slot a sequence addresses.
  #[must_use]
  pub fn at_mut(&mut self, seq: Seq) -> &mut S {
    let index = of(seq, self.capacity);
    self.get_mut(index)
  }

  /// Iterate every slot in index order.
  ///
  /// ```
  /// use ring_store::Buffer;
  /// use ring_slot::{ Slot, TypedSlot };
  /// use ring_types::Capacity;
  ///
  /// let buffer : Buffer< TypedSlot< u8 > > = Buffer::new( Capacity::new( 4 ).unwrap() );
  /// assert_eq!( buffer.iter().filter( | s | s.is_empty() ).count(), 4 );
  /// ```
  pub fn iter(&self) -> core::slice::Iter<'_, S> {
    self.slots.iter()
  }

  /// Mutably iterate every slot in index order.
  pub fn iter_mut(&mut self) -> core::slice::IterMut<'_, S> {
    self.slots.iter_mut()
  }
}

impl<'a, S> IntoIterator for &'a Buffer<S> {
  type Item = &'a S;
  type IntoIter = core::slice::Iter<'a, S>;

  fn into_iter(self) -> Self::IntoIter {
    self.iter()
  }
}

impl<'a, S> IntoIterator for &'a mut Buffer<S> {
  type Item = &'a mut S;
  type IntoIter = core::slice::IterMut<'a, S>;

  fn into_iter(self) -> Self::IntoIter {
    self.iter_mut()
  }
}
