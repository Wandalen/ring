//! Full-ring overflow policies.
//!
//! Tier 1 of the ring family's 33 crates — the concurrency write-path implementation.
//! Depends on `ring_types` and `ring_stats`.
//!
//! `docs/feature/174_overflow_policy_enum_and_handlers.md` splits into an enum
//! and a set of handlers; `docs/decision/121_workstream_008_contract_gaps_ruled.md`
//! § 5 puts the enum in `ring_types` and the handlers here. This crate is the
//! handlers: given a full ring and a policy, decide what happens and record it.
//!
//! The feature's own emphasis is on what is *absent*: there is deliberately no
//! variant that overwrites unread data. That absence is what makes a successful
//! publish meaningful — under every policy here, a publish that reported success
//! kept the item. [`Resolution`] below is the type that makes the alternative
//! outcomes explicit rather than leaving them to a boolean.

#![no_std]
#![deny(missing_docs)]

// `no_std` here is an assertion, not a convenience. This crate, `ring_stats`,
// and `ring_types` between them import `core::fmt` and `core::sync::atomic` and
// nothing else across seven source files — they were already core-only, and were
// so by accident: a `use std::` added to any of them compiled, passed every
// test, and ended the embeddability of all three with nothing anywhere to
// notice. The attribute is what turns that property from a coincidence somebody
// has to keep noticing into a build error. It is on all three because the
// property is only worth anything transitively: a `no_std` crate depending on a
// `std` one is a `std` crate.

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
// **No `Default`, deliberately, and the omission needs saying.** Every
// `Resolution` is caused by a policy meeting a full ring, so a default would be
// a value nothing produced — an outcome asserted about a publish that never
// happened. `OverflowPolicy` derives `Default` with `#[ default ]` on
// `DropNewest` and is the right shape for it: a configuration has to have a
// value before anything happens to it. The two declarations sit eight lines
// apart in two crates and differ by one token, so a good-faith normalisation
// adding `Default` here compiles and passes the whole suite. `ALL` below is what
// makes the *count* enforced; this comment is the only thing standing between
// the derive list and the reader who would otherwise "fix" it.
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
  /// in every configuration that ships — `ring_core:337` records that for its
  /// own `match`, and until now this crate, which owns the type, recorded it
  /// nowhere. It is kept rather than removed because it is the only value for
  /// which [`Resolution::accepted_incoming`] is true, and deleting it would
  /// collapse that predicate to a constant `false`.
  EvictedOldest,
  /// Nothing was published and nothing discarded; the caller got the error.
  Refused,
}

impl Resolution {
  /// Every variant, for exhaustive iteration.
  ///
  /// Mirrors `OverflowPolicy::ALL`, and exists for the same reason: a fourth
  /// *policy* already fails to compile against the two `match` blocks below and
  /// the length assertion in `ring_types`, but until this constant a fourth
  /// *resolution* compiled against everything except those same matches — the
  /// asymmetry being that a `match` catches a new variant only where one is
  /// written, and nothing published the count a test could pin.
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
  /// positive patterns answers `false` for anything it was not told about, so a
  /// fourth variant — an `Overwrite`, say — would read `false` here and `false`
  /// in [`Resolution::accepted_incoming`], which is exactly `Refused`'s profile:
  /// classified as losing nothing and accepting nothing, silently, by a
  /// predicate that never mentioned it. Spelled this way it stops compiling
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
  /// True only for [`Resolution::EvictedOldest`] — the one policy that makes
  /// room rather than refusing or discarding the arrival.
  ///
  /// **Narrower than the name suggests.** A publish that finds room and
  /// succeeds normally never constructs a `Resolution` at all, so this
  /// predicate never sees that case — what it actually answers is "did the
  /// item get in *by evicting something*", not "did the item get in".
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
/// [`RingError::Full`] on the one that hands the decision back. Exactly one
/// counter is incremented per call, whichever branch is taken — so a stats read
/// accounts for every full-ring event, not only the lossy ones.
///
/// **This guarantee is about calls to `resolve`, not about a build.** No
/// production caller in this workspace currently calls it — the crate's own
/// sole consumer takes [`would_resolve`] instead, which touches no counter —
/// so a `RingStats` read in that build reflects nothing about full-ring
/// events regardless of how many occurred. The promise above holds fully; it
/// is a contract on the function, not a claim about which build exercises it.
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
/// **This error path is not effect-free.** The counter is incremented first, so
/// `Err` returns with shared state already mutated — against the usual reading
/// that an `Err` means nothing happened. That is intended: a refusal is a
/// full-ring event and stats exist to count those. What follows from it is that
/// **`resolve` is not idempotent and a retry double-counts**: a caller that
/// backs off and calls again on the same full-ring event adds a second increment
/// to `dropped( Fail )` for one arrival. Retry loops should call
/// [`would_resolve`] to decide and `resolve` once to record.
pub fn resolve(policy: OverflowPolicy, stats: &RingStats) -> Result<Resolution, RingError> {
  // Counts an event, not a loss: this runs on every policy, including `Fail`,
  // whose own share is retrievable separately via `stats.dropped(
  // OverflowPolicy::Fail )` — `Resolution::Refused.lost_an_item()` is `false`,
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
/// The pure half of [`resolve`], for callers deciding what a policy *would* do —
/// a factory validating a configuration, a test tabulating the mapping — rather
/// than handling a real full-ring event.
///
/// **Also the right half for a caller that holds no `&RingStats` at all.**
/// The pattern's real division is purity, not purpose: a caller handling a
/// genuine full-ring event still belongs here if it has nothing to record
/// into, since [`resolve`] cannot be called without one. That is why this
/// crate's own sole consumer takes this half for a real full-ring event
/// rather than a hypothetical one.
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
