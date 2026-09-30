//! Slot payload views — typed and raw bytes.
//!
//! Tier 1 of the ring family's 33 crates — the concurrency write-path implementation.
//! Depends on `ring_types`.
//!
//! `docs/feature/182_typed_slot_and_bytes_slot.md` asks for two slot shapes over
//! one ring: a typed slot for traffic whose shape is known at compile time, and
//! a bytes slot for traffic that arrives from outside and is decoded later.
//! The reason for both is a cost asymmetry — force everything through bytes and
//! in-process command traffic pays an encoding cost for nothing; type the ring
//! and opaque host traffic has nowhere to go without a second ring built for it.
//! That asymmetry is argued here, not measured: the family's own bench crate
//! times `TypedSlot` on both ring implementations and has never instantiated
//! `BytesSlot`, so the difference this paragraph claims has no number behind it.
//!
//! The feature's constraint is that "both use the same claim, gating, and drain
//! — the difference is confined to what a slot contains." That is enforced here
//! by [`Slot`]: one trait both shapes implement, so everything downstream is
//! written against the trait and cannot branch on which shape it has.
//!
//! No `unsafe`. A [`BytesSlot`] is a fixed-length buffer plus a length, so a
//! partially-filled slot reads back exactly what was written and nothing else,
//! without `MaybeUninit`.

#![deny(missing_docs)]

use ring_types::RingError;

/// What every slot shape can do, so the handshake never branches on which one
/// it holds.
///
/// ```
/// use ring_slot::{Slot, TypedSlot};
///
/// let mut slot = TypedSlot::<u32>::empty();
/// assert!(slot.is_empty());
/// slot.clear();
/// assert!(slot.is_empty());
/// ```
pub trait Slot {
    /// Whether this slot currently holds nothing.
    fn is_empty(&self) -> bool;

    /// Return the slot to its empty state.
    ///
    /// **Not a promise to overwrite.** For a shape that owns what it stores
    /// (`TypedSlot`), the old value's destructor runs, so nothing survives the
    /// call. For a shape that stores by copying into fixed storage
    /// (`BytesSlot`), the bytes are not zeroed — only the length that marks
    /// them unreachable through this trait's own API moves. Both are "empty"
    /// by [`Slot::is_empty`]; only one is empty in memory.
    ///
    /// **How long the residue lasts.** For `BytesSlot`, until a write of at
    /// least that length lands on the same slot, or the ring holding it is
    /// dropped — not until the next lap. A ring reuses slot `i` every lap and
    /// nothing sweeps the array in between: `ring_store::Buffer::clear` is the
    /// only bulk reset in the family and is wired to no lifecycle event.
    /// So a slot holds the longest payload ever written to its position, and
    /// clearing it moves a length rather than erasing anything. No public API
    /// on either shape can read past the length — `Debug` and `PartialEq` are
    /// written by hand on `BytesSlot` for exactly that reason — but the bytes
    /// are in the process's memory until overwritten, which is the property a
    /// caller handling secrets has to plan around.
    fn clear(&mut self);
}

/// A slot holding one value of a compile-time-known type.
///
/// The cheap shape: no encoding, no length, no copy beyond the move into the
/// slot. Used for in-process command traffic.
///
/// ```
/// use ring_slot::{Slot, TypedSlot};
///
/// let mut slot = TypedSlot::empty();
/// assert_eq!(slot.set(42u32), None);
/// assert_eq!(slot.get(), Some(&42));
/// assert_eq!(slot.take(), Some(42));
/// assert!(slot.is_empty());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedSlot<T>(Option<T>);

impl<T> TypedSlot<T> {
    /// An empty slot.
    ///
    /// ```
    /// use ring_slot::TypedSlot;
    /// assert!(TypedSlot::<u8>::empty().get().is_none());
    /// ```
    #[must_use]
    pub const fn empty() -> Self {
        Self(None)
    }

    /// Place `value` in the slot, returning whatever it held before.
    ///
    /// Returning the displaced value rather than dropping it keeps the door open
    /// for a future evict-oldest policy to hand a caller what it evicted instead
    /// of losing it silently — but `ring_overflow` does not depend on this crate
    /// and cannot reach this return value today. The one reader that binds it now
    /// is `ring_core`'s `debug_assert`, confirming a freshly claimed slot came
    /// back empty rather than handing anything back to a caller.
    ///
    /// Deliberately **not** `#[ must_use ]`, unlike [`TypedSlot::take`]: on a ring
    /// the displaced value belongs to a lap the consumer already finished, so
    /// dropping it is the ordinary case rather than a lost record. `core` marks
    /// neither `Option::replace` nor `Option::take` for the same kind of reason.
    ///
    /// ```
    /// use ring_slot::TypedSlot;
    /// let mut slot = TypedSlot::empty();
    /// assert_eq!(slot.set(1u8), None);
    /// assert_eq!(slot.set(2u8), Some(1));
    /// ```
    pub fn set(&mut self, value: T) -> Option<T> {
        self.0.replace(value)
    }

    /// Borrow the held value, if any.
    ///
    /// ```
    /// use ring_slot::TypedSlot;
    /// let mut slot = TypedSlot::empty();
    /// slot.set(7u8);
    /// assert_eq!(slot.get(), Some(&7));
    /// ```
    pub const fn get(&self) -> Option<&T> {
        self.0.as_ref()
    }

    /// Remove and return the held value, leaving the slot empty.
    ///
    /// This is the *only* way a payload leaves a `TypedSlot`, so discarding the
    /// result always destroys a record — which is why it is `#[ must_use ]` and
    /// [`TypedSlot::set`] is not. `set`'s return is a value the caller usually
    /// did not ask for and frequently should drop; on a ring, the slot it
    /// displaces belongs to a lap the consumer already finished.
    ///
    /// ```
    /// use ring_slot::TypedSlot;
    /// let mut slot = TypedSlot::empty();
    /// slot.set(7u8);
    /// assert_eq!(slot.take(), Some(7));
    /// assert_eq!(slot.take(), None);
    /// ```
    #[must_use = "this is the only way a payload leaves the slot; dropping it here destroys the record"]
    pub fn take(&mut self) -> Option<T> {
        self.0.take()
    }
}

// Written out rather than derived, and the difference is load-bearing.
// `#[ derive( Default ) ]` on a tuple struct emits `impl< T : Default >`,
// because it defaults every field — including the `Option< T >`, whose own
// `Default` is `None` and needs nothing from `T`. That bound would propagate
// to every `S : Slot + Default` consumer (`ring_store`, `ring_mpsc`,
// `ring_spsc` all use it), silently narrowing the ring to payloads that
// happen to be `Default`. `a_slot_is_default_for_a_payload_that_is_not`
// fails to compile if this is ever replaced by the derive.
impl<T> Default for TypedSlot<T> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<T> Slot for TypedSlot<T> {
    fn is_empty(&self) -> bool {
        self.0.is_none()
    }

    fn clear(&mut self) {
        self.0 = None;
    }
}

/// A slot holding an opaque byte payload of at most `N` bytes.
///
/// The shape for traffic arriving from outside the process, decoded later by
/// whoever knows the wire format. Fixed capacity because a ring's slots are
/// allocated once, so a slot that could grow would defeat the allocation
/// behaviour the ring was chosen for.
///
/// The two fields below read like `N + 8` bytes on the struct definition
/// alone. At the `N` this family actually instantiates, the `usize`'s 8-byte
/// alignment folds in as padding too, so the true cost sits closer to `2N`.
///
/// Every bit pattern of `[ u8; N ]` and of `usize` is valid, so this type has
/// no spare bit to hide a discriminant in — wrapping it in `Option` costs a
/// full extra word at any `N`, unlike `TypedSlot`, whose `Option` is free. No
/// caller does that today; a future fallible API returning this type by value
/// should prefer `Result` with a niche-carrying error over that wrapping.
///
/// `Clone` copies the full `[ u8; N ]` array regardless of how many bytes are
/// actually written, so its cost is proportional to `N`, not to `len`. A
/// generic bound of `Slot + Clone` accepts this shape at that flat cost
/// alongside `TypedSlot< T >`, whose clone tracks the payload instead and
/// exists only when `T` itself is `Clone`.
///
/// **The slot is its first `len` bytes, everywhere.** [`BytesSlot::read`],
/// [`BytesSlot::len`], [`BytesSlot::is_empty`] and this type's `Debug` and
/// `PartialEq` all agree on that. The last two are written by hand rather than
/// derived, because a derived pair compares and prints all `N` — including the
/// residue of longer payloads written to the same slot on earlier laps, which
/// [`Slot::clear`] does not erase. Derived, a cleared slot would print the
/// bytes it no longer holds and compare unequal to a fresh one; written by
/// hand, the residue is unreachable through every public API this type has.
/// `Clone` is still derived and still copies all `N` — it reproduces the
/// value's storage, not the value.
///
/// ```
/// use ring_slot::{BytesSlot, Slot};
///
/// let mut slot = BytesSlot::<16>::empty();
/// slot.write(b"hello").unwrap();
/// assert_eq!(slot.read(), b"hello");
/// assert_eq!(slot.len(), 5);
/// assert!(!slot.is_empty());
///
/// slot.clear();
/// assert_eq!(slot, BytesSlot::<16>::empty(), "clear returns it to a fresh slot's value");
/// assert_eq!(format!("{slot:?}"), "BytesSlot { payload: [] }");
/// ```
#[derive(Clone)]
pub struct BytesSlot<const N: usize> {
    bytes: [u8; N],
    len: usize,
}

// Hand-written over `read()` rather than derived over the fields. A derived
// impl reads all `N` bytes, including the tail past `len` that no accessor can
// reach and that `clear` deliberately does not zero — so it would print a
// cleared slot's former payload and distinguish two slots no caller can tell
// apart. Matching `Vec`'s convention: the spare capacity is storage, not value.
impl<const N: usize> core::fmt::Debug for BytesSlot<N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BytesSlot").field("payload", &self.read()).finish()
    }
}

impl<const N: usize> PartialEq for BytesSlot<N> {
    fn eq(&self, other: &Self) -> bool {
        self.read() == other.read()
    }
}

// Reflexive, symmetric and transitive because slice equality is, and `read()`
// is a pure function of the receiver. Written out rather than derived so it
// cannot silently re-acquire the field-wise bound the `PartialEq` above drops.
impl<const N: usize> Eq for BytesSlot<N> {}

impl<const N: usize> BytesSlot<N> {
    /// An empty slot of capacity `N`.
    ///
    /// ```
    /// use ring_slot::BytesSlot;
    /// assert_eq!(BytesSlot::<8>::empty().capacity(), 8);
    /// ```
    #[must_use]
    pub const fn empty() -> Self {
        Self { bytes: [0; N], len: 0 }
    }

    /// Bytes this slot can hold.
    ///
    /// ```
    /// use ring_slot::BytesSlot;
    /// assert_eq!(BytesSlot::<32>::empty().capacity(), 32);
    /// ```
    #[must_use]
    pub const fn capacity(&self) -> usize {
        N
    }

    /// Bytes currently held.
    ///
    /// ```
    /// use ring_slot::BytesSlot;
    /// let mut s = BytesSlot::<8>::empty();
    /// s.write(b"ab").unwrap();
    /// assert_eq!(s.len(), 2);
    /// ```
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Whether nothing has been written.
    ///
    /// Duplicates [`Slot::is_empty`] deliberately: a caller holding a concrete
    /// `BytesSlot` should not need the trait in scope to ask, and the trait impl
    /// below delegates here so the two can never disagree.
    ///
    /// ```
    /// use ring_slot::BytesSlot;
    /// let mut s = BytesSlot::<8>::empty();
    /// assert!(s.is_empty());
    /// s.write(b"a").unwrap();
    /// assert!(!s.is_empty());
    /// ```
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Copy `payload` into the slot, replacing what was there.
    ///
    /// # Errors
    ///
    /// [`RingError::BatchTooLarge`] when `payload` exceeds the slot's capacity —
    /// reusing that variant because the shape of the problem is identical: a
    /// request bigger than the fixed room available, which no amount of draining
    /// resolves. The rendered message still says "batch" and "ring capacity",
    /// wording that names a multi-slot reservation this call never makes —
    /// `RingError` has no byte-denominated variant to borrow instead.
    ///
    /// ```
    /// use ring_slot::BytesSlot;
    /// let mut s = BytesSlot::<4>::empty();
    /// assert!(s.write(b"abcd").is_ok());
    /// assert!(s.write(b"abcde").is_err());
    /// // A failed write leaves the previous contents intact.
    /// assert_eq!(s.read(), b"abcd");
    /// ```
    pub fn write(&mut self, payload: &[u8]) -> Result<(), RingError> {
        if payload.len() > N {
            return Err(RingError::BatchTooLarge { requested: payload.len(), capacity: N });
        }
        self.bytes[..payload.len()].copy_from_slice(payload);
        self.len = payload.len();
        Ok(())
    }

    /// The bytes written, and only those — never the unused tail.
    ///
    /// ```
    /// use ring_slot::BytesSlot;
    /// let mut s = BytesSlot::<8>::empty();
    /// s.write(b"xy").unwrap();
    /// assert_eq!(s.read(), b"xy");
    /// ```
    #[must_use]
    pub fn read(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

// Written out for the same reason as `TypedSlot`'s, though this shape could
// not use the derive even if the bound were harmless: `[ u8; N ]` implements
// `Default` only at the handful of `N` the standard library enumerates, not at
// a generic `N`, so `#[ derive( Default ) ]` here does not compile at all.
impl<const N: usize> Default for BytesSlot<N> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<const N: usize> Slot for BytesSlot<N> {
    fn is_empty(&self) -> bool {
        Self::is_empty(self)
    }

    fn clear(&mut self) {
        self.len = 0;
    }
}
