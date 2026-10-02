//! Full-ring overflow policies.
//!
//! Part of the ring family's concurrency write path.
//!
//! The overflow-policy feature splits into an enum and a set of handlers. The
//! enum lives in `ring_types` and the handlers live here. This crate is the
//! handlers. Given a full ring and a policy, they decide what happens and record it.
//!
//! A publish that reports success did not always keep the item. Under
//! `DropNewest`, `ring_core`'s `try_push` returns `Ok(())` on a full ring and
//! discards the incoming item. `EvictedOldest` accepts the incoming item by
//! dropping the oldest unread one. [`Resolution`] below names what happened to
//! the data in each case instead of leaving it to a boolean.

#![no_std]
#![deny(missing_docs)]

// `no_std` here is an assertion, not a convenience. This crate, `ring_stats`,
// and `ring_types` between them import `core::fmt` and `core::sync::atomic` and
// nothing else across seven source files. They were already core-only, and by
// accident. A `use std::` added to any of them compiled, passed every test, and
// ended the embeddability of all three with nothing anywhere to notice. The
// attribute turns that property from a coincidence somebody has to keep
// noticing into a build error. It is on all three because a `no_std` crate
// depending on a `std` one is a `std` crate, so the property is only worth
// anything transitively.

use ring_stats::RingStats;
use ring_types::{OverflowPolicy, RingError};

/// What a full-ring publish did.
///
/// Three outcomes, one per policy, and each names what happened to the data
/// rather than merely whether the call succeeded.
///
/// ```
/// use ring_overflow::Resolution;
/// assert!( Resolution::DroppedIncoming.lost_an_item() );
/// assert!( Resolution::EvictedOldest.lost_an_item() );
/// assert!( !Resolution::Refused.lost_an_item() );
/// ```
// **No `Default`, deliberately, and the omission needs saying.** A policy
// meeting a full ring causes every `Resolution`, so a default would be a value
// nothing produced, an outcome asserted about a publish that never happened.
// `OverflowPolicy` derives `Default` with `#[ default ]` on `DropNewest` and is
// the right shape for it, because a configuration has to have a value before
// anything happens to it. The two declarations sit eight lines apart in two
// crates and differ by one token, so a good-faith normalisation adding
// `Default` here compiles and passes the whole suite. `ALL` below enforces the
// *count*. This comment is the only thing standing between the derive list and
// the reader who would otherwise "fix" it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Resolution {
  /// The incoming item was discarded; the ring's contents are unchanged.
  DroppedIncoming,
  /// The oldest unread item was discarded to make room; the incoming item was
  /// accepted.
  ///
  /// **No shipping configuration produces this.** It requires
  /// [`OverflowPolicy::DropOldest`], which the default build rejects at
  /// construction with `RingError::PolicyUnsupported`, and which the `crossbeam`
  /// build handles with `force_push` and an early return before reaching the
  /// resolution site. So one of these three exported variants is unconstructible
  /// in every configuration that ships. It is kept rather than removed because
  /// it is the only value for which [`Resolution::accepted_incoming`] is true,
  /// and deleting it would collapse that predicate to a constant `false`.
  EvictedOldest,
  /// Nothing was published and nothing discarded; the caller got the error.
  Refused,
}

impl Resolution {
  /// Every variant, for exhaustive iteration.
  ///
  /// Mirrors `OverflowPolicy::ALL`, and exists for the same reason. A fourth
  /// *policy* already fails to compile against the two `match` blocks below and
  /// the length assertion in `ring_types`. Until this constant, a fourth
  /// *resolution* compiled against everything except those same matches. A
  /// `match` catches a new variant only where one is written, and nothing
  /// published the count a test could pin.
  ///
  /// ```
  /// use ring_overflow::Resolution;
  /// assert_eq!( Resolution::ALL.len(), 3 );
  /// assert!( Resolution::ALL.contains( &Resolution::Refused ) );
  /// ```
  pub const ALL: [Self; 3] = [Self::DroppedIncoming, Self::EvictedOldest, Self::Refused];

  /// Whether an item was lost without the caller being told.
  ///
  /// ```
  /// use ring_overflow::Resolution;
  /// assert!( Resolution::DroppedIncoming.lost_an_item() );
  /// ```
  /// Written as an exhaustive `match` rather than the shorter
  /// `matches!( self, DroppedIncoming | EvictedOldest )`. A `matches!` over
  /// positive patterns answers `false` for anything it was not told about. A
  /// fourth variant, say an `Overwrite`, would read `false` here and `false` in
  /// [`Resolution::accepted_incoming`], which is `Refused`'s profile. A
  /// predicate that never mentioned it would silently classify it as losing
  /// nothing and accepting nothing. Spelled this way it stops compiling
  /// instead, which is the whole point of a closed enum.
  #[must_use]
  pub const fn lost_an_item(self) -> bool {
    match self {
      Self::DroppedIncoming | Self::EvictedOldest => true,
      Self::Refused => false,
    }
  }

  /// Whether the incoming item made it into the ring.
  ///
  /// True only for [`Resolution::EvictedOldest`], the one policy that makes
  /// room rather than refusing or discarding the arrival.
  ///
  /// **Narrower than the name suggests.** A publish that finds room and
  /// succeeds normally never constructs a `Resolution` at all, so this
  /// predicate never sees that case. The question it answers is "did the item
  /// get in *by evicting something*", not "did the item get in".
  ///
  /// ```
  /// use ring_overflow::Resolution;
  /// assert!( Resolution::EvictedOldest.accepted_incoming() );
  /// assert!( !Resolution::DroppedIncoming.accepted_incoming() );
  /// assert!( !Resolution::Refused.accepted_incoming() );
  /// ```
  /// Exhaustive for the reason given on [`Resolution::lost_an_item`].
  #[must_use]
  pub const fn accepted_incoming(self) -> bool {
    match self {
      Self::EvictedOldest => true,
      Self::DroppedIncoming | Self::Refused => false,
    }
  }
}

/// Apply `policy` to a publish that found the ring full, recording the outcome
/// in `stats`.
///
/// Returns the [`Resolution`] on the two policies that keep going, and
/// [`RingError::Full`] on the one that hands the decision back. Each call
/// increments exactly one counter, whichever branch it takes, so a stats read
/// accounts for every full-ring event, not only the lossy ones.
///
/// **This guarantee is about calls to `resolve`, not about a build.** No
/// production caller in this workspace currently calls it. The crate's own sole
/// consumer takes [`would_resolve`] instead, which touches no counter, so a
/// `RingStats` read in that build reflects nothing about full-ring events
/// regardless of how many occurred. The promise above holds fully. It is a
/// contract on the function, not a claim about which build exercises it.
///
/// ```
/// use ring_overflow::{ resolve, Resolution };
/// use ring_stats::RingStats;
/// use ring_types::{ OverflowPolicy, RingError };
///
/// let stats = RingStats::new();
///
/// assert_eq!( resolve( OverflowPolicy::DropNewest, &stats ), Ok( Resolution::DroppedIncoming ) );
/// assert_eq!( resolve( OverflowPolicy::DropOldest, &stats ), Ok( Resolution::EvictedOldest ) );
/// assert_eq!( resolve( OverflowPolicy::Fail, &stats ), Err( RingError::Full ) );
///
/// assert_eq!( stats.dropped( OverflowPolicy::DropNewest ), 1 );
/// assert_eq!( stats.dropped( OverflowPolicy::DropOldest ), 1 );
/// assert_eq!( stats.dropped( OverflowPolicy::Fail ), 1 );
/// ```
///
/// # Errors
///
/// [`RingError::Full`] under [`OverflowPolicy::Fail`], which is the policy's
/// entire purpose rather than a failure of this function.
///
/// **This error path is not effect-free.** `resolve` increments the counter
/// first, so `Err` returns with shared state already mutated. That goes against
/// the usual reading that an `Err` means nothing happened, and it is intended,
/// because a refusal is a full-ring event and stats exist to count those. It
/// follows that **`resolve` is not idempotent and a retry double-counts**. A
/// caller that backs off and calls again on the same full-ring event adds a
/// second increment to `dropped( Fail )` for one arrival. Retry loops should
/// call [`would_resolve`] to decide and `resolve` once to record.
pub fn resolve(policy: OverflowPolicy, stats: &RingStats) -> Result<Resolution, RingError> {
  // Counts an event, not a loss. This runs on every policy, including `Fail`,
  // whose own share is retrievable separately via `stats.dropped(
  // OverflowPolicy::Fail )`. `Resolution::Refused.lost_an_item()` is `false`,
  // so this call and that predicate answer different questions about the
  // same arrival.
  stats.record_drop(policy, 1);
  match policy {
    OverflowPolicy::DropNewest => Ok(Resolution::DroppedIncoming),
    OverflowPolicy::DropOldest => Ok(Resolution::EvictedOldest),
    OverflowPolicy::Fail => Err(RingError::Full),
  }
}

/// The resolution a policy produces, without touching any counters.
///
/// The pure half of [`resolve`], for callers deciding what a policy *would* do
/// rather than handling a real full-ring event. Examples are a factory
/// validating a configuration and a test tabulating the mapping.
///
/// **Also the right half for a caller that holds no `&RingStats` at all.**
/// The pattern divides by purity, not purpose. A caller handling a genuine
/// full-ring event still belongs here if it has nothing to record into, since
/// [`resolve`] cannot be called without one. That is why this crate's own sole
/// consumer takes this half for a real full-ring event rather than a
/// hypothetical one.
///
/// ```
/// use ring_overflow::{ would_resolve, Resolution };
/// use ring_types::OverflowPolicy;
///
/// assert_eq!( would_resolve( OverflowPolicy::Fail ), Resolution::Refused );
/// assert_eq!( would_resolve( OverflowPolicy::DropOldest ), Resolution::EvictedOldest );
/// ```
#[must_use]
pub const fn would_resolve(policy: OverflowPolicy) -> Resolution {
  match policy {
    OverflowPolicy::DropNewest => Resolution::DroppedIncoming,
    OverflowPolicy::DropOldest => Resolution::EvictedOldest,
    OverflowPolicy::Fail => Resolution::Refused,
  }
}
