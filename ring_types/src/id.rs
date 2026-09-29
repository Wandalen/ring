//! The ring family's two position types: a sequence number that counts
//! publications for the lifetime of a ring, and the slot index derived from it.
//!
//! They are separate types on purpose. `docs/feature/167_sequence_slot_index_and_power_of_two_capacity.md`
//! makes the point: keeping the sequence apart from the slot is what lets a
//! gate compare two positions that are a full lap apart, which is impossible
//! once both have been folded into `0..capacity`.

/// A publication's ordinal in a ring's whole history, counting from zero.
///
/// Monotonic and, for every reachable workload, non-wrapping: at 10⁹
/// publications per second a `u64` runs for roughly 584 years. The family
/// depends on that — `Seq` comparison is plain `<`, with no lap-aware
/// wrap-around logic anywhere, because the wrap point is unreachable.
///
/// What *does* wrap is the [`SlotIndex`] derived from it, which is a different
/// type for exactly this reason.
///
/// ```
/// use ring_types::Seq;
/// assert!( Seq( 0 ) < Seq( 1 ) );
/// assert_eq!( Seq( 3 ).next(), Seq( 4 ) );
/// ```
#[ derive( Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default ) ]
pub struct Seq( pub u64 );

impl Seq
{
  /// The position of a ring that has published nothing.
  pub const ZERO : Self = Self( 0 );

  /// The next sequence after this one.
  ///
  /// Panics on overflow in a debug build and wraps to zero in a release
  /// build — the standard `u64` addition behaviour. A wrapped `Seq` would
  /// silently invert every gate comparison in the family, which is why the
  /// non-wrapping argument has to hold: at 10⁹ publications per second a
  /// `u64` runs for roughly 584 years, well past any reachable workload.
  ///
  /// ```
  /// use ring_types::Seq;
  /// assert_eq!( Seq( 41 ).next(), Seq( 42 ) );
  /// ```
  #[ must_use ]
  pub const fn next( self ) -> Self
  {
    Self( self.0 + 1 )
  }

  /// This sequence advanced by `n`.
  ///
  /// Overflow behaves exactly as [`Seq::next`] documents — debug panics,
  /// release wraps to zero. The reachability argument does not carry over
  /// unchanged: `next` needs 2⁶⁴ increments to reach the wrap, while this
  /// takes `n` from the caller and reaches it in a single call from any
  /// position. A caller deriving `n` from a batch length or a configured
  /// count owns that bound; nothing here checks it.
  ///
  /// ```
  /// use ring_types::Seq;
  /// assert_eq!( Seq( 10 ).advanced_by( 5 ), Seq( 15 ) );
  /// assert_eq!( Seq( 10 ).advanced_by( 0 ), Seq( 10 ) );
  /// ```
  #[ must_use ]
  pub const fn advanced_by( self, n : u64 ) -> Self
  {
    Self( self.0 + n )
  }

  /// How many publications separate this sequence from a later one, or `0`
  /// when `later` is not actually later.
  ///
  /// Saturating rather than signed: the caller that needs the direction has
  /// already compared the two, and every caller that does not wants a count.
  ///
  /// ```
  /// use ring_types::Seq;
  /// assert_eq!( Seq( 4 ).distance_to( Seq( 10 ) ), 6 );
  /// assert_eq!( Seq( 10 ).distance_to( Seq( 4 ) ), 0 );
  /// ```
  #[ must_use ]
  pub const fn distance_to( self, later : Self ) -> u64
  {
    later.0.saturating_sub( self.0 )
  }
}

/// A position within a ring's storage, always in `0..capacity`.
///
/// Derived from a [`Seq`] by `ring_index`, never constructed by counting.
/// Two sequences a full lap apart produce the identical `SlotIndex`, which is
/// the whole reason [`Seq`] exists as a separate type.
///
/// ```
/// use ring_types::SlotIndex;
/// assert_eq!( SlotIndex( 3 ).get(), 3 );
/// ```
#[ derive( Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default ) ]
pub struct SlotIndex( pub usize );

impl SlotIndex
{
  /// The underlying offset.
  ///
  /// ```
  /// use ring_types::SlotIndex;
  /// assert_eq!( SlotIndex( 7 ).get(), 7 );
  /// ```
  #[ must_use ]
  pub const fn get( self ) -> usize
  {
    self.0
  }
}
