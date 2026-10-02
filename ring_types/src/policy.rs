//! The two configuration enums a ring carries: what a consumer does when the
//! ring is empty, and what a producer does when it is full.
//!
//! Only the discriminants live here. The handlers that act on them are
//! `ring_wait` and `ring_overflow` respectively. That split is the reason this
//! crate's own description ends "no ring logic".

/// What a consumer does when it asks for items and the ring has none.
///
/// The wait-kind feature requires all four, and requires [`WaitKind::None`]
/// specifically, because the tick path cannot afford any of the other three.
///
/// ```
/// use ring_types::WaitKind;
/// assert_eq!( WaitKind::default(), WaitKind::Spin );
/// assert!( WaitKind::None.is_non_blocking() );
/// assert!( !WaitKind::Park.is_non_blocking() );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum WaitKind {
  /// Re-read the cursor in a tight loop. Lowest latency, burns a core.
  #[default]
  Spin,
  /// Yield to the scheduler between reads. Cheaper than spinning under
  /// oversubscription, higher and less predictable latency.
  Yield,
  /// Idle between reads; no publisher wakes it. Cheapest, highest latency.
  Park,
  /// Return immediately with whatever is available, possibly nothing. The only
  /// variant reachable from inside a tick.
  None,
}

impl WaitKind {
  /// Every variant, in discriminant order.
  ///
  /// The length constrains this array against its own initialiser, never
  /// against the enum. A fifth variant compiles clean in this file, leaves
  /// `ALL` at four, and passes the assertion below; this was measured, not assumed.
  /// What refuses a fifth variant is a wildcard-free `match`, and the nearest
  /// ones are `ring_wait`'s `escalation_hint` and `pause` in production source
  /// plus `wait_kind_has_exactly_four_variants` in `tests/types_test.rs`. What
  /// the assertions here do carry is this array's own shape: length,
  /// membership, and no dropped or duplicated entry. That is the half a runtime
  /// check can reach. The distinction is stated here because someone reading
  /// this declaration alone would otherwise not see it.
  ///
  /// ```
  /// use ring_types::WaitKind;
  /// assert_eq!( WaitKind::ALL.len(), 4 );
  /// ```
  pub const ALL: [Self; 4] = [Self::Spin, Self::Yield, Self::Park, Self::None];

  /// Whether this strategy can be used on the tick path, which holds only for
  /// [`WaitKind::None`].
  ///
  /// ```
  /// use ring_types::WaitKind;
  /// assert_eq!( WaitKind::ALL.iter().filter( | w | w.is_non_blocking() ).count(), 1 );
  /// ```
  // Fix(wait_kind_is_non_blocking_classification_not_exhaustive): was
  //   `matches!( self, Self::None )`, so a fifth `WaitKind` variant would
  //   silently read `false`, meaning it blocks like the other three, with nothing
  //   forcing a second look, even though this predicate's own doc names the
  //   one variant the tick path "cannot afford any of the other three"
  //   besides. `OverflowPolicy::reports_failure`/`drops_silently` just below
  //   already carry the exhaustive shape.
  // Root cause: `matches!` over a single named variant answers every variant
  //   it was not told about with the same default, compiling cleanly however
  //   many variants `WaitKind` gains.
  // Pitfall: a caller gating "is it safe to call this from inside a tick?" on
  //   this predicate would silently admit a new blocking strategy onto the
  //   one path that cannot tolerate blocking at all.
  #[must_use]
  pub const fn is_non_blocking(self) -> bool {
    match self {
      Self::None => true,
      Self::Spin | Self::Yield | Self::Park => false,
    }
  }
}

/// What a producer does when the ring has no free slot.
///
/// The overflow-policy feature is explicit that there is **no variant that
/// overwrites unread data silently**. `DropOldest` does discard an unread item,
/// and `ring_overflow` reports that as a loss. A publish that reports success
/// has not always kept the item. Under `DropNewest`, `ring_core`'s `try_push`
/// returns `Ok(())` on a full ring and discards the incoming item.
///
/// ```
/// use ring_types::OverflowPolicy;
/// assert_eq!( OverflowPolicy::ALL.len(), 3 );
/// assert!( OverflowPolicy::Fail.reports_failure() );
/// assert!( !OverflowPolicy::DropNewest.reports_failure() );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum OverflowPolicy {
  /// Discard the item being published; the ring's contents are untouched.
  #[default]
  DropNewest,
  /// Discard the oldest unread item to make room for the new one.
  DropOldest,
  /// Publish nothing and return an error, handing the decision to the caller.
  Fail,
}

impl OverflowPolicy {
  /// Every variant, in discriminant order.
  ///
  /// Same split as [`WaitKind::ALL`]. The wildcard-free `match` in
  /// `overflow_policy_has_no_overwrite_variant` refuses a fourth variant, and
  /// the `contains` loop beside it pins this array's own shape. Until that loop
  /// existed the doctest below was the crate's only check on the roster's
  /// contents, and it covers one entry of three. A measured run that dropped
  /// `DropNewest` instead left the whole crate green, which is what put the loop there.
  ///
  /// ```
  /// use ring_types::OverflowPolicy;
  /// assert!( OverflowPolicy::ALL.contains( &OverflowPolicy::DropOldest ) );
  /// ```
  pub const ALL: [Self; 3] = [Self::DropNewest, Self::DropOldest, Self::Fail];

  /// Whether a full-ring publish under this policy returns an error rather
  /// than silently dropping something.
  ///
  /// ```
  /// use ring_types::OverflowPolicy;
  /// assert_eq!( OverflowPolicy::ALL.iter().filter( | p | p.reports_failure() ).count(), 1 );
  /// ```
  // Fix(overflow_policy_classification_not_exhaustive): was `matches!( self,
  //   Self::Fail )`, so a fourth policy would silently read `false`, meaning "does not
  //   report failure", with nothing in this function forcing a second look.
  //   `drops_silently` just below carried the identical shape.
  // Root cause: `matches!` over a single named variant is exhaustive over
  //   nothing; it answers every variant it was not told about with the same
  //   default, compiling cleanly however many variants `OverflowPolicy` gains.
  // Pitfall: a caller gating "did this publish actually fail?" on this
  //   predicate would treat a new failing policy as having succeeded. The
  //   fail-open direction is backwards for an overflow policy, whose
  //   entire contract (see the enum's own doc) is that a caller must be able to
  //   trust what it reports.
  #[must_use]
  pub const fn reports_failure(self) -> bool {
    match self {
      Self::Fail => true,
      Self::DropNewest | Self::DropOldest => false,
    }
  }

  /// Whether a full-ring publish under this policy loses an item without
  /// telling the caller, which is true for both drop variants.
  ///
  /// ```
  /// use ring_types::OverflowPolicy;
  /// assert!( OverflowPolicy::DropOldest.drops_silently() );
  /// assert!( !OverflowPolicy::Fail.drops_silently() );
  /// ```
  // Fix(overflow_policy_classification_not_exhaustive): same shape as
  //   `reports_failure` above. `matches!( self, Self::DropNewest |
  //   Self::DropOldest )` answered `false` for every variant it did not name.
  // Root cause: see `reports_failure` above.
  // Pitfall: a new silently-dropping policy misclassified here would lose data
  //   with no warning raised on its behalf. That is the one behavior this
  //   predicate exists to flag.
  #[must_use]
  pub const fn drops_silently(self) -> bool {
    match self {
      Self::DropNewest | Self::DropOldest => true,
      Self::Fail => false,
    }
  }
}
