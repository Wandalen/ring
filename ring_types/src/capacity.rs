//! A ring's slot count, validated to a power of two at construction.
//!
//! `docs/feature/167_sequence_slot_index_and_power_of_two_capacity.md` requires
//! the constraint so that sequence→slot conversion is a bitmask rather than a
//! division. Enforcing it in a type means the mask is always valid wherever a
//! `Capacity` is in scope, so `ring_index` needs no runtime check of its own.

use crate::RingError;

/// A validated power-of-two slot count.
///
/// The only constructor rejects zero and non-powers of two, so `mask()` below
/// is total: every `Capacity` that exists has a usable mask.
///
/// ```
/// use ring_types::Capacity;
/// let cap = Capacity::new( 8 ).unwrap();
/// assert_eq!( cap.get(), 8 );
/// assert_eq!( cap.mask(), 7 );
/// assert!( Capacity::new( 7 ).is_err() );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Capacity(usize);

impl Capacity {
  /// Validate `slots` as a ring capacity.
  ///
  /// # Errors
  ///
  /// [`RingError::CapacityZero`] when `slots` is `0`, and
  /// [`RingError::CapacityNotPowerOfTwo`] when it has more than one bit set.
  ///
  /// ```
  /// use ring_types::{ Capacity, RingError };
  /// assert_eq!( Capacity::new( 0 ).unwrap_err(), RingError::CapacityZero );
  /// assert_eq!( Capacity::new( 6 ).unwrap_err(), RingError::CapacityNotPowerOfTwo( 6 ) );
  /// assert!( Capacity::new( 1 ).is_ok() );
  /// ```
  pub const fn new(slots: usize) -> Result<Self, RingError> {
    if slots == 0 {
      return Err(RingError::CapacityZero);
    }
    if !slots.is_power_of_two() {
      return Err(RingError::CapacityNotPowerOfTwo(slots));
    }
    Ok(Self(slots))
  }

  /// The slot count.
  ///
  /// ```
  /// use ring_types::Capacity;
  /// assert_eq!( Capacity::new( 16 ).unwrap().get(), 16 );
  /// ```
  #[must_use]
  pub const fn get(self) -> usize {
    self.0
  }

  /// The bitmask that folds a sequence into a slot index, always
  /// `capacity - 1`. It is always valid because the constructor rejected every
  /// value for which it would not be.
  ///
  /// ```
  /// use ring_types::Capacity;
  /// assert_eq!( Capacity::new( 1 ).unwrap().mask(), 0 );
  /// assert_eq!( Capacity::new( 1024 ).unwrap().mask(), 1023 );
  /// ```
  #[must_use]
  pub const fn mask(self) -> usize {
    self.0 - 1
  }
}
