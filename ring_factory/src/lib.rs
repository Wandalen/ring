//! Ring construction from configuration.
//!
//! One of the ring family's 33 crates, which implement the concurrency write-path.
//!
//! One of the five crates on the family's export Contract, and the only one
//! that is a *verb*. `ring_handle` and `ring_tls` are things a consumer holds,
//! `ring_types` is vocabulary, `ring_flush` is a decision they make. This crate
//! is the door. Under decision/121 § 4, features 171 (SPSC), 172 (MPSC) and
//! 181 (registry) are reached *through* it rather than by importing
//! `ring_spsc`, `ring_mpsc` or `ring_registry` directly.
//!
//! ```
//! use ring_factory::{ Factory, RingConfig };
//!
//! let cfg = RingConfig::new( 8 ).unwrap();
//! let mut split = Factory.build::< u32 >( cfg ).unwrap();
//!
//! let mut ends = split.ends();
//! let ( mut producer, mut consumer ) = ends.split();
//! producer.try_push( 7 ).unwrap();
//! assert_eq!( consumer.try_recv(), Some( 7 ) );
//! ```
//!
//! # `build` returns the owner, not the pair
//!
//! The pre-implementation API specified
//! `build( cfg ) -> Result< HandlePair< S >, BuildError >`, and **that signature
//! cannot be written.** `ring_handle::Split::ends` borrows `&mut self` and
//! `Ends::split` borrows `&'a mut self`, so both handles borrow from a value
//! somebody has to be holding. Returning them needs a self-referential struct,
//! which `ring_handle`'s own doc comment names as the reason its two-step shape
//! exists.
//!
//! `docs/data_structure/002` foresaw this as shape D2 and scored its cost
//! **severe**, because the only two owners it could see were bad ones: the
//! factory holds the ring (making it stateful, against `invariant/001`) or the
//! caller supplies storage (a second argument, against the one-argument API).
//! `ring_handle` shipped a third that neither candidate anticipated. `Split< T >`
//! owns the ring and is an ordinary value, so **the owner is the return value.**
//! The factory allocates it and gives it away. One argument in, one owned value
//! out, and the caller splits when they are ready.
//!
//! # What it refuses
//!
//! Two things, and only one of them is this crate's own.
//!
//! `RingConfig` has already validated `capacity` and clamped `batch` and
//! `producers`, so the record arriving here is legal. But `ring_config` accepts
//! `OverflowPolicy::DropOldest` without complaint and `ring_core::Ring::new`
//! refuses it, because eviction contradicts the exactly-once delivery both
//! in-house backends guarantee. This crate relays that refusal as
//! [`BuildError::Unsupported`] and never re-decides it. Duplicating the
//! backend's policy would silently go wrong the moment a fourth backend
//! disagrees.
//!
//! [`BuildError::NameTaken`] is the one refusal this crate owns outright.
//!
//! # The refusal that carries a route
//!
//! `DropOldest` is unserviceable only *by the in-house backends*.
//! `crossbeam_queue::ArrayQueue::force_push` does exactly what the policy asks,
//! which is why `Factory::build_crossbeam` accepts the config
//! [`Factory::build`] refuses.
//!
//! `build_crossbeam` is named rather than linked, because the route exists only
//! when the `crossbeam` feature is on. In a default build this section
//! describes a door that was not compiled, and a link to it is a rustdoc error
//! rather than a reference. The route is real and conditional. The plain name
//! says both, where a link would claim only the first.
//!
//! The two are deliberately separate functions rather than one that inspects
//! the policy and routes. Routing would make the same `RingConfig` produce a
//! different backend depending on whether a cargo feature was enabled, and
//! `docs/invariant/001` is the rule that a config alone determines the ring.
//! Feature 187 is a build-time opt-in, so it gets a door of its own and
//! `docs/invariant/001` stays true of `build` without a proviso.
//!
//! # Two fields that cannot be honoured to the precision the record expresses
//!
//! `wait` names a strategy in `ring_wait`, whose public API turned out to be
//! free functions taking a `WaitKind` per call rather than a waiter to
//! construct. So there is nothing here to build from the field, and whoever
//! blocks passes the kind themselves. `producers` chooses between two backends
//! and its value above 1 never reaches the ring, because `ring_mpsc`'s claim
//! path is a compare-exchange rather than something told a count.
//!
//! **The acceptance criterion therefore grades the corrected value, not the
//! requested one.** Feature 180 asks that observable behaviour match every
//! field. Two of the five are clamped before `build` is reachable, and the
//! request is stored nowhere. The sharpest case changes the *backend*: a
//! manifest field left empty clamps to one producer and selects SPSC.
//! → `docs/pitfall/001`.

#![deny(missing_docs)]

use core::fmt;

pub use ring_config::RingConfig;
use ring_core::Ring;
use ring_handle::Split;
pub use ring_registry::Registry;
use ring_registry::RegistryError;
use ring_types::RingError;

/// A ring that is not yet built, and the only supported way to build one.
///
/// **It has no fields**, which is `docs/invariant/001` made structural. Every
/// input to a build arrives in the `RingConfig` argument, so a field would be a
/// second input the invariant forbids. Write it as a literal, `Factory.build(
/// cfg )`, since there is deliberately no `Default` impl to conjure one from.
///
/// It is a type rather than a free function because it is one of five names on
/// the family's export Contract, and a consumer needs a noun to import and to
/// name in their own documentation.
#[derive(Debug, Clone)]
pub struct Factory;

impl Factory {
  /// Build a ring from a configuration.
  ///
  /// Returns the [`Split`] that owns it. Call `ends()` then `split()` on that
  /// value to get the producer and consumer, which borrow from it. See the
  /// crate-level example.
  ///
  /// # Errors
  ///
  /// [`BuildError::Unsupported`] when the configured overflow policy is one no
  /// in-house backend can honour. Today that is exactly
  /// `OverflowPolicy::DropOldest`. The refusal is `ring_core`'s, and this
  /// function relays it rather than re-deciding it, so a future backend that
  /// accepts it needs no change here.
  ///
  /// This path never returns [`BuildError::NameTaken`], because that error
  /// needs a name and this path has none.
  ///
  /// ```
  /// use ring_factory::{ BuildError, Factory, RingConfig };
  /// use ring_types::{ OverflowPolicy, RingError };
  ///
  /// let evicting = RingConfig::new( 8 ).unwrap().with_overflow( OverflowPolicy::DropOldest );
  /// assert_eq!
  /// (
  ///   Factory.build::< u8 >( evicting ).unwrap_err(),
  ///   BuildError::Unsupported( RingError::PolicyUnsupported ),
  /// );
  /// ```
  pub fn build<S: Send>(&self, cfg: RingConfig) -> Result<Split<S>, BuildError> {
    let ring = Ring::new(&cfg).map_err(BuildError::Unsupported)?;
    Ok(Split::new(ring))
  }

  /// Build a ring and register it under a name.
  ///
  /// The registry is passed in, not held. A `Factory` with a registry field would
  /// own every ring it ever built, make two factories non-interchangeable, and
  /// make "which factory built this ring" a question with an answer. Pass it,
  /// and the ring's lifetime is the registry's alone.
  ///
  /// Retrieve the ring with [`Registry::get_mut`], which is re-exported here so
  /// a consumer bound by the export Contract never has to name `ring_registry`.
  ///
  /// # Errors
  ///
  /// [`BuildError::Unsupported`] as for [`build`](Self::build), raised before
  /// any ring exists.
  ///
  /// [`BuildError::NameTaken`] when the name is already registered. **The ring
  /// built for the refused call is dropped before returning** and was never
  /// reachable by the caller, so a refused registration leaks nothing and
  /// exposes nothing. **That guarantee is not free: a full `Split<S>` is
  /// constructed, moved into the registry, moved back out, and dropped before
  /// the much smaller [`BuildError`] reaches the caller.** Building first is a
  /// correctness choice. Registration is the only operation that can decide the
  /// name atomically, and another thread could invalidate a check-then-build
  /// order between the two calls.
  ///
  /// ```
  /// use ring_factory::{ BuildError, Factory, Registry, RingConfig };
  ///
  /// let cfg = RingConfig::new( 8 ).unwrap();
  /// let mut registry = Registry::< u32 >::new();
  ///
  /// Factory.build_named( cfg, "events", &mut registry ).unwrap();
  /// assert!( registry.contains( "events" ) );
  ///
  /// let again = Factory.build_named( cfg, "events", &mut registry );
  /// assert_eq!( again.unwrap_err(), BuildError::NameTaken );
  /// assert_eq!( registry.len(), 1 );
  /// ```
  pub fn build_named<S: Send>(&self, cfg: RingConfig, name: &str, registry: &mut Registry<S>) -> Result<(), BuildError> {
    let split = self.build(cfg)?;

    // The refused `Split` comes back in the error payload and is dropped as
    // this arm ends. That is what makes the "no ring was exposed" guarantee
    // free rather than checked. The caller never held it.
    match registry.register(name, split) {
      Ok(()) => Ok(()),
      // The name is discarded rather than carried into `BuildError`, because the
      // caller passed it in and still has it. Carrying it would also put a `String` in
      // a `Copy` error type for no new information.
      Err((RegistryError::NameTaken { .. }, _refused)) => Err(BuildError::NameTaken),
    }
  }

  /// Build a ring on the crossbeam backend, feature 187's door.
  ///
  /// **Outside the one-door promise, deliberately.** [`build`](Self::build)
  /// selects a backend from the configuration alone; this one selects crossbeam
  /// whatever the configuration says, because feature 187 is a build-time
  /// opt-in for consumers not blocked on the in-house ring being finished.
  /// Folding it into `build` would make one config produce different rings on
  /// different feature flags.
  ///
  /// It accepts `OverflowPolicy::DropOldest`, which `build` refuses, because
  /// `ArrayQueue::force_push` evicts. Eviction is the one capability the
  /// in-house backends lack.
  ///
  /// # Errors
  ///
  /// None currently. The signature matches [`build`](Self::build) so the two
  /// are interchangeable at a call site, which is the point of a swappable
  /// backend. That means **at a call site fixed ahead of time**, never inside a
  /// wrapper that reads `cfg` and picks between them at runtime. Such a wrapper
  /// would reintroduce exactly the second input `docs/invariant/001` forbids,
  /// and nothing in either signature stops anyone from writing it.
  #[cfg(feature = "crossbeam")]
  pub fn build_crossbeam<S: Send>(&self, cfg: RingConfig) -> Result<Split<S>, BuildError> {
    let ring = Ring::new_crossbeam(&cfg).map_err(BuildError::Unsupported)?;
    Ok(Split::new(ring))
  }
}

/// What a build may refuse.
///
/// Two variants, and only one of them is about a name. Most refusals a factory
/// could plausibly make happen elsewhere: `RingConfig::new` rejects a bad
/// capacity, `with_batch` and `with_producers` clamp rather than fail, and
/// nothing anywhere checks the wait strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildError {
  /// A ring is already registered under this name.
  ///
  /// Reachable only from [`Factory::build_named`], and only after the build
  /// half of that call already succeeded. The two variants partition by
  /// phase, so this one means a ring was built and then dropped because
  /// registration refused it, never that the build itself failed.
  NameTaken,
  /// The backend cannot honour the configured overflow policy.
  ///
  /// Carries `ring_core`'s own refusal rather than re-spelling it, so the
  /// backend's vocabulary survives the relay. `RingError` is `ring_types`', so
  /// this payload stays inside the export Contract's five crates. From
  /// `build_named`, this variant means the build phase itself failed and the
  /// registry was never consulted. It is the mirror image of `NameTaken`.
  Unsupported(RingError),
}

impl fmt::Display for BuildError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::NameTaken => write!(f, "a ring is already registered under this name"),
      Self::Unsupported(error) => write!(f, "the backend refused the configuration: {error}"),
    }
  }
}

impl core::error::Error for BuildError {}
