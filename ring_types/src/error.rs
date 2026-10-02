//! The one error type the ring path returns.
//!
//! There is one enum rather than one per crate because a consumer uses only the
//! five exported crates and never names the 28 internal crates. Per-crate
//! error types would have to be converted into a shared one at that boundary
//! anyway. This is that shared one, declared once at tier 0. `ring_bench`,
//! `ring_factory`, `ring_flush` and `ring_registry` are off the ring path and
//! declare their own.
//!
//! No `error_tools`, `thiserror` or `anyhow`. `ring_types` deliberately has no
//! dependencies at all, so that the family's tier 0 compiles in isolation.

use core::fmt;

/// Every way a ring operation can fail.
///
/// `Copy` and allocation-free, because an error on the tick path must not allocate.
///
/// ```
/// use ring_types::RingError;
/// let e = RingError::Full;
/// assert_eq!( e, RingError::Full );
/// assert!( !e.is_configuration() );
/// ```
/// # Adding a Variant
///
/// `#[ non_exhaustive ]` reserves the right to add variants, and this is the
/// only declaration in the family that carries it. It does **not** reserve the
/// right to add a variant that breaks a derive. `ring_bench::RunError` wraps
/// this type and derives `Copy`, so a variant carrying a `String`, such as a
/// path, a message or a name, would fail to compile three crates away, in a
/// crate this one has never heard of.
///
/// Fix(copy_derive_pins_other_crates_errors): the coupling used to be recorded
/// on neither side. Keep new variants `Copy` (carry a numeric or `Copy`
/// payload, or none) unless the same commit also changes `ring_bench`.
///
/// Root cause: `#[ non_exhaustive ]` announces that variants may be added and
/// says nothing about which traits must keep holding when they are.
/// Pitfall: a derive on a wrapper is a constraint on the wrapped type's future,
/// and nothing in either declaration points at the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RingError {
  /// A capacity of zero was requested. A ring with no slots can never accept
  /// a publish, so this is rejected at construction rather than at first use.
  CapacityZero,
  /// A capacity that is not a power of two was requested; the offending value
  /// is carried so the caller can report it.
  CapacityNotPowerOfTwo(usize),
  /// The ring has no free slot and its [`crate::OverflowPolicy`] is
  /// [`crate::OverflowPolicy::Fail`].
  Full,
  /// The ring has no unread item and the caller asked not to wait.
  Empty,
  /// The ring has been closed; no further publish will be accepted.
  Closed,
  /// A ring is already registered under this name.
  NameTaken,
  /// No ring is registered under this name.
  NameUnknown,
  /// A batch of the requested length cannot be served. The ring's whole
  /// capacity is smaller than the request, so no amount of draining helps.
  BatchTooLarge {
    /// Slots asked for.
    requested: usize,
    /// Slots the ring has in total.
    capacity: usize,
  },
  /// The configured [`crate::OverflowPolicy`] is one this backend cannot
  /// honour.
  ///
  /// The concrete case is [`crate::OverflowPolicy::DropOldest`] on a backend
  /// guaranteeing exactly-once delivery. Evicting an unread record to make room
  /// contradicts the guarantee, so the ring refuses to be built rather than
  /// silently degrading to `DropNewest`. A caller who never learns their policy
  /// was not applied is worse off than one whose construction failed.
  PolicyUnsupported,
}

impl RingError {
  /// Whether this error is a mistake in how the ring was configured, rather
  /// than a condition arising from traffic.
  ///
  /// The distinction is worth a method because the two call for opposite
  /// responses: a configuration error is a bug to fix, a traffic error is a
  /// state to handle.
  ///
  /// ```
  /// use ring_types::RingError;
  /// assert!( RingError::CapacityZero.is_configuration() );
  /// assert!( RingError::CapacityNotPowerOfTwo( 3 ).is_configuration() );
  /// assert!( !RingError::Full.is_configuration() );
  /// ```
  // Fix(ring_error_classification_not_exhaustive): was `matches!` naming only the
  //   four configuration variants, so a tenth `RingError` variant would silently
  //   read `false`, meaning "a traffic condition to handle", with nothing forcing a
  //   second look. `is_transient` just below carried the identical shape.
  // Root cause: `#[ non_exhaustive ]` on this very enum documents that variants get
  //   added (see "# Adding a Variant" above); `matches!` over a positive list is the
  //   one construct that does not care, since a variant it was not told about
  //   silently falls through the pattern to `false` rather than failing to compile.
  // Pitfall: `RingError` growing is not hypothetical for this type in particular.
  //   The enum's own doc section exists because it already happened once.
  //   A predicate whose default answer flips the caller's response (retry vs. fix)
  //   is the wrong place to let that growth go unnoticed.
  #[must_use]
  pub const fn is_configuration(self) -> bool {
    match self {
      Self::CapacityZero | Self::CapacityNotPowerOfTwo(_) | Self::BatchTooLarge { .. } | Self::PolicyUnsupported => true,
      Self::Full | Self::Empty | Self::Closed | Self::NameTaken | Self::NameUnknown => false,
    }
  }

  /// Whether retrying the same operation later could succeed without anything
  /// else changing. True for the two conditions a peer's progress clears.
  ///
  /// ```
  /// use ring_types::RingError;
  /// assert!( RingError::Full.is_transient() );
  /// assert!( RingError::Empty.is_transient() );
  /// assert!( !RingError::Closed.is_transient() );
  /// ```
  // Fix(ring_error_classification_not_exhaustive): same shape as `is_configuration`
  //   above. `matches!( self, Self::Full | Self::Empty )` answered `false` for
  //   every variant it did not name, so a new transient condition would silently
  //   tell a caller to give up rather than retry.
  // Root cause: see `is_configuration` above. An exhaustive match was available
  //   and not used.
  // Pitfall: the two predicates disagreeing about whether growth is a compile
  //   error would itself be a defect; both are fixed together so neither is the
  //   one a future variant slips through.
  #[must_use]
  pub const fn is_transient(self) -> bool {
    match self {
      Self::Full | Self::Empty => true,
      Self::CapacityZero
      | Self::CapacityNotPowerOfTwo(_)
      | Self::Closed
      | Self::NameTaken
      | Self::NameUnknown
      | Self::BatchTooLarge { .. }
      | Self::PolicyUnsupported => false,
    }
  }
}

impl fmt::Display for RingError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::CapacityZero => write!(f, "ring capacity must be at least 1"),
      Self::CapacityNotPowerOfTwo(n) => write!(f, "ring capacity {n} is not a power of two"),
      Self::Full => write!(f, "ring is full"),
      Self::Empty => write!(f, "ring is empty"),
      Self::Closed => write!(f, "ring is closed"),
      Self::NameTaken => write!(f, "a ring is already registered under this name"),
      Self::NameUnknown => write!(f, "no ring is registered under this name"),
      Self::BatchTooLarge { requested, capacity } => {
        write!(f, "batch of {requested} exceeds ring capacity {capacity}")
      }
      Self::PolicyUnsupported => write!(f, "this backend cannot honour the configured overflow policy"),
    }
  }
}

impl core::error::Error for RingError {}
