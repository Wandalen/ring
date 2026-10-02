//! Slot translators that fill a claimed slot.
//!
//! Part of the ring family's concurrency write path.
//!
//! The typed-and-bytes-slot feature asks for something stronger than two slot
//! shapes that both work: "`TypedSlot<T>` and `BytesSlot` both round-trip
//! through the **identical** claim/publish/drain path." Two shapes each with
//! their own write call satisfies the first reading and not the second. With
//! two write calls the paths would only resemble one another, and a divergence
//! between them would be a change nobody's test would notice.
//!
//! This crate makes the two shapes share one path. [`Fill`] is the write half,
//! where a payload knows how to enter a slot. [`Peek`] is the read half, where a
//! slot knows what a reader gets back. [`publish_into`] and [`drain_from`] are
//! then written once, generically, so a ring's publish and drain *could* call
//! those through a single body regardless of slot shape. **`ring_core`, the
//! crate that assembles a ring, calls neither today.** It reaches
//! `TypedSlot::set`/`TypedSlot::take` directly, so this crate's one path is
//! declared but not yet the one production rings execute. Neither function
//! can tell the two slot shapes apart, which is exactly the property that
//! feature is asking for, once something calls them.
//!
//! ## Why the read half is a GAT
//!
//! The two shapes return different things: a `TypedSlot<T>` hands
//! back a `&T`, a `BytesSlot<N>` a `&[u8]` whose length is the payload's, not
//! the slot's. Flattening both into one concrete return type would mean
//! copying, and a `&[u8]` copied out of a slot is the allocation the whole
//! family exists to avoid. An associated type with a lifetime lets one function
//! serve both without either paying for the other's shape.

#![deny(missing_docs)]

use ring_slot::{BytesSlot, Slot, TypedSlot};
use ring_types::RingError;

/// A payload that knows how to enter a slot of shape `S`.
///
/// Implemented on the *payload*, not the slot, so adding a payload kind never
/// touches the slot types, and `publish_into` needs no match, no downcast and
/// no enum of shapes.
///
/// ```
/// use ring_event::Fill;
/// use ring_slot::{ Slot, TypedSlot };
///
/// let mut slot = TypedSlot::< u32 >::empty();
/// 7u32.fill( &mut slot ).unwrap();
/// assert_eq!( slot.get(), Some( &7 ) );
/// ```
pub trait Fill<S> {
  /// Write `self` into `slot`, replacing whatever it held.
  ///
  /// # Errors
  ///
  /// Whatever the slot shape refuses. For a byte payload longer than the slot
  /// that is [`RingError::BatchTooLarge`], a configuration error rather than
  /// back-pressure, since no amount of draining makes the payload fit. A typed
  /// payload cannot fail, and says so by never returning `Err`.
  fn fill(self, slot: &mut S) -> Result<(), RingError>;
}

impl<T> Fill<TypedSlot<T>> for T {
  fn fill(self, slot: &mut TypedSlot<T>) -> Result<(), RingError> {
    // Discarding the displaced value is not an oversight. `fill`'s documented
    // contract is "write `self` into `slot`, replacing whatever it held". A
    // caller that needs the old record calls `TypedSlot::set` directly and
    // binds it; this trait exists to give both slot shapes one signature, and
    // `BytesSlot` has nothing to hand back.
    slot.set(self);
    Ok(())
  }
}

impl<const N: usize> Fill<BytesSlot<N>> for &[u8] {
  fn fill(self, slot: &mut BytesSlot<N>) -> Result<(), RingError> {
    slot.write(self)
  }
}

/// A slot that knows what a reader gets back from it.
///
/// The read half of the shared path. `Out` is a lifetime-parameterised
/// associated type because the two shapes return borrows of different things.
/// The module documentation explains why neither is flattened into the other.
///
/// ```
/// use ring_event::Peek;
/// use ring_slot::TypedSlot;
///
/// let mut slot = TypedSlot::empty();
/// assert_eq!( slot.peek(), None, "an unpublished slot reads as nothing" );
/// slot.set( 3u8 );
/// assert_eq!( slot.peek(), Some( &3 ) );
/// ```
pub trait Peek {
  /// What a reader is handed when the slot holds something.
  type Out<'a>
  where
    Self: 'a;

  /// The slot's contents, or `None` when nothing was published into it.
  ///
  /// `None` is not an error. A claimed-but-unpublished slot is the state
  /// the claim/publish handshake is built to keep a consumer out of, and this
  /// is how a drain observes it.
  ///
  /// # A `BytesSlot` cannot distinguish empty from zero-length
  ///
  /// A [`BytesSlot`] records a length and nothing more, so a deliberately
  /// published zero-byte payload reads back as `None`, identical to a slot
  /// nobody has touched. This is a real limitation, not an oversight, and it is
  /// not worth a flag byte per slot to remove. The ring already carries the
  /// distinction in the published-sequence handshake, and a caller that needs
  /// "somebody published nothing" must read it there rather than from the slot.
  /// A [`TypedSlot<()>`](TypedSlot) does not share the limitation, and is the
  /// cheaper way to send a payload-free signal.
  fn peek(&self) -> Option<Self::Out<'_>>;
}

impl<T> Peek for TypedSlot<T> {
  type Out<'a>
    = &'a T
  where
    T: 'a;

  fn peek(&self) -> Option<&T> {
    self.get()
  }
}

impl<const N: usize> Peek for BytesSlot<N> {
  type Out<'a> = &'a [u8];

  fn peek(&self) -> Option<&[u8]> {
    if self.is_empty() { None } else { Some(self.read()) }
  }
}

/// Publish `payload` into `slot` through the one write path both shapes take.
///
/// Deliberately trivial. What matters is that there is only one of it. A ring's
/// publish *path* passes through here, at the step where a claimed slot receives
/// its payload, so no slot shape can acquire a publish path of its own without
/// the signature changing. This is not `ring_publish`'s `Publisher::publish`,
/// which moves a cursor over sequence numbers and never touches a slot; that is
/// the other half of the same operation, one level up.
///
/// # Errors
///
/// Whatever [`Fill`] refuses for this pairing.
///
/// ```
/// use ring_event::{ drain_from, publish_into };
/// use ring_slot::{ BytesSlot, TypedSlot };
///
/// // The same two calls, over two unrelated slot shapes.
/// let mut typed = TypedSlot::empty();
/// publish_into( &mut typed, 42u16 ).unwrap();
/// assert_eq!( drain_from( &typed ), Some( &42 ) );
///
/// let mut bytes = BytesSlot::< 8 >::empty();
/// publish_into( &mut bytes, &b"hi"[ .. ] ).unwrap();
/// assert_eq!( drain_from( &bytes ), Some( &b"hi"[ .. ] ) );
/// ```
pub fn publish_into<S, P>(slot: &mut S, payload: P) -> Result<(), RingError>
where
  P: Fill<S>,
{
  payload.fill(slot)
}

/// Read `slot` through the one read path both shapes take.
///
/// **This does not empty the slot, and the name is the trap.** It takes `&S`
/// and hands back a borrow, so after it returns the slot still reports
/// non-empty and still holds the payload. Freeing the slot for the next lap is
/// a separate, mandatory third call to [`recycle`]. Omitting it is neither a
/// compile error nor a runtime error, and produces a drained ring every slot
/// of which reads occupied. The borrow is deliberate. `BytesSlot` cannot hand
/// back an owned payload without copying it, so the one signature that fits
/// both shapes is the borrowing one. Consuming the slot is left to the caller,
/// which knows when the read is finished.
///
/// ```
/// use ring_event::{ drain_from, publish_into, recycle };
/// use ring_slot::{ BytesSlot, Slot };
///
/// let slot = BytesSlot::< 4 >::empty();
/// assert_eq!( drain_from( &slot ), None );
///
/// let mut slot = BytesSlot::< 4 >::empty();
/// publish_into( &mut slot, &b"ab"[ .. ] ).unwrap();
/// assert_eq!( drain_from( &slot ), Some( &b"ab"[ .. ] ) );
/// assert!( !slot.is_empty(), "reading it did not free it" );
///
/// recycle( &mut slot );
/// assert!( slot.is_empty(), "this is the call that does" );
/// ```
pub fn drain_from<S>(slot: &S) -> Option<S::Out<'_>>
where
  S: Peek,
{
  slot.peek()
}

/// Empty `slot` through the shared path.
///
/// The third of the three operations a ring performs on a slot, here for the
/// same reason as the other two. A shape-specific reset would be a fourth path
/// the identical-path claim does not cover.
///
/// ```
/// use ring_event::{ drain_from, publish_into, recycle };
/// use ring_slot::TypedSlot;
///
/// let mut slot = TypedSlot::empty();
/// publish_into( &mut slot, 1u8 ).unwrap();
/// recycle( &mut slot );
/// assert_eq!( drain_from( &slot ), None );
/// ```
pub fn recycle<S>(slot: &mut S)
where
  S: Slot,
{
  slot.clear();
}
