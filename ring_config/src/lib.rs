//! Ring construction parameters.
//!
//! Tier 1 of the ring family's 33 crates — the concurrency write-path implementation.
//! Depends on `ring_types`.
//!
//! `docs/feature/180_ring_config_and_factory.md` asks for everything that varies
//! between rings collected into one value, so that a ring's shape is *data*
//! rather than a choice of constructor. That matters twice over: it keeps the
//! set of legal configurations enumerable instead of "whatever someone wrote a
//! constructor for", and it means the manifest language that eventually
//! describes channels describes exactly this record with no translation layer.
//!
//! This crate holds the record and its validation. `ring_factory` turns one
//! into a ring — the other half of feature 180, kept separate because a
//! configuration must be constructible and inspectable at tier 1, long before
//! anything at tier 11 exists to consume it.

#![deny(missing_docs)]

use ring_types::{Capacity, OverflowPolicy, RingError, WaitKind};

/// Everything that varies between one ring and another.
///
/// Built through [`RingConfig::new`] and narrowed by the `with_*` methods,
/// which return `Self` so a configuration reads as one expression:
///
/// ```
/// use ring_types::{ OverflowPolicy, WaitKind };
/// use ring_config::RingConfig;
///
/// let cfg = RingConfig::new( 1024 ).unwrap()
///   .with_wait( WaitKind::None )
///   .with_overflow( OverflowPolicy::Fail )
///   .with_producers( 4 )
///   .with_batch( 64 );
///
/// assert_eq!( cfg.capacity().get(), 1024 );
/// assert!( cfg.is_multi_producer() );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RingConfig {
  capacity: Capacity,
  wait: WaitKind,
  overflow: OverflowPolicy,
  producers: usize,
  batch: usize,
}

impl RingConfig {
  /// A configuration of `slots` capacity, with every other field at its
  /// default: spin waiting, drop-newest overflow, one producer, batch of one.
  ///
  /// # Errors
  ///
  /// Whatever [`Capacity::new`] rejects — a zero or non-power-of-two capacity.
  ///
  /// ```
  /// use ring_config::RingConfig;
  /// assert!( RingConfig::new( 64 ).is_ok() );
  /// assert!( RingConfig::new( 63 ).is_err() );
  /// ```
  pub fn new(slots: usize) -> Result<Self, RingError> {
    Ok(Self {
      capacity: Capacity::new(slots)?,
      wait: WaitKind::default(),
      overflow: OverflowPolicy::default(),
      producers: 1,
      batch: 1,
    })
  }

  /// Set the wait strategy.
  ///
  /// ```
  /// use ring_types::WaitKind;
  /// use ring_config::RingConfig;
  /// let cfg = RingConfig::new( 8 ).unwrap().with_wait( WaitKind::Park );
  /// assert_eq!( cfg.wait(), WaitKind::Park );
  /// ```
  #[must_use]
  pub const fn with_wait(mut self, wait: WaitKind) -> Self {
    self.wait = wait;
    self
  }

  /// Set the overflow policy.
  ///
  /// ```
  /// use ring_types::OverflowPolicy;
  /// use ring_config::RingConfig;
  /// let cfg = RingConfig::new( 8 ).unwrap().with_overflow( OverflowPolicy::Fail );
  /// assert_eq!( cfg.overflow(), OverflowPolicy::Fail );
  /// ```
  #[must_use]
  pub const fn with_overflow(mut self, overflow: OverflowPolicy) -> Self {
    self.overflow = overflow;
    self
  }

  /// Set the expected producer count.
  ///
  /// A count of `0` is clamped to `1`: a ring nothing can publish into has no
  /// use, and clamping keeps the setter infallible so a builder chain does not
  /// need a `?` in its middle.
  ///
  /// ```
  /// use ring_config::RingConfig;
  /// assert_eq!( RingConfig::new( 8 ).unwrap().with_producers( 0 ).producers(), 1 );
  /// assert_eq!( RingConfig::new( 8 ).unwrap().with_producers( 6 ).producers(), 6 );
  /// ```
  #[must_use]
  pub const fn with_producers(mut self, producers: usize) -> Self {
    self.producers = if producers == 0 { 1 } else { producers };
    self
  }

  /// Set the batch size, clamped to at least `1` and at most the capacity —
  /// a batch larger than the ring can never be served however much draining
  /// happens, so it is corrected here rather than failing at first publish.
  ///
  /// ```
  /// use ring_config::RingConfig;
  /// let cfg = RingConfig::new( 16 ).unwrap();
  /// assert_eq!( cfg.with_batch( 0 ).batch(), 1 );
  /// assert_eq!( cfg.with_batch( 8 ).batch(), 8 );
  /// assert_eq!( cfg.with_batch( 999 ).batch(), 16 );
  /// ```
  #[must_use]
  pub const fn with_batch(mut self, batch: usize) -> Self {
    let capped = if batch > self.capacity.get() {
      self.capacity.get()
    } else {
      batch
    };
    self.batch = if capped == 0 { 1 } else { capped };
    self
  }

  /// The validated capacity.
  ///
  /// ```
  /// use ring_config::RingConfig;
  /// assert_eq!( RingConfig::new( 32 ).unwrap().capacity().get(), 32 );
  /// ```
  #[must_use]
  pub const fn capacity(&self) -> Capacity {
    self.capacity
  }

  /// The wait strategy.
  ///
  /// ```
  /// use ring_types::WaitKind;
  /// use ring_config::RingConfig;
  /// assert_eq!( RingConfig::new( 8 ).unwrap().wait(), WaitKind::Spin );
  /// ```
  #[must_use]
  pub const fn wait(&self) -> WaitKind {
    self.wait
  }

  /// The overflow policy.
  ///
  /// ```
  /// use ring_types::OverflowPolicy;
  /// use ring_config::RingConfig;
  /// assert_eq!( RingConfig::new( 8 ).unwrap().overflow(), OverflowPolicy::DropNewest );
  /// ```
  #[must_use]
  pub const fn overflow(&self) -> OverflowPolicy {
    self.overflow
  }

  /// The expected producer count, always at least one.
  ///
  /// ```
  /// use ring_config::RingConfig;
  /// assert_eq!( RingConfig::new( 8 ).unwrap().producers(), 1 );
  /// ```
  #[must_use]
  pub const fn producers(&self) -> usize {
    self.producers
  }

  /// The batch size, always between one and the capacity inclusive.
  ///
  /// ```
  /// use ring_config::RingConfig;
  /// assert_eq!( RingConfig::new( 8 ).unwrap().batch(), 1 );
  /// ```
  #[must_use]
  pub const fn batch(&self) -> usize {
    self.batch
  }

  /// Whether this configuration needs the contended multi-producer claim.
  ///
  /// A derived reading, and the one a factory branches on: a single-producer
  /// ring must not pay for a synchronisation it does not need, per
  /// `docs/feature/172_multi_producer_claim.md`.
  ///
  /// ```
  /// use ring_config::RingConfig;
  /// assert!( !RingConfig::new( 8 ).unwrap().is_multi_producer() );
  /// assert!( RingConfig::new( 8 ).unwrap().with_producers( 2 ).is_multi_producer() );
  /// ```
  #[must_use]
  pub const fn is_multi_producer(&self) -> bool {
    self.producers > 1
  }

  /// Whether this configuration is safe to use from inside a tick — true only
  /// when waiting cannot park, per
  /// `docs/feature/183_try_only_operations_on_the_tick_path.md`.
  ///
  /// ```
  /// use ring_types::WaitKind;
  /// use ring_config::RingConfig;
  /// let cfg = RingConfig::new( 8 ).unwrap();
  /// assert!( !cfg.is_tick_safe() );
  /// assert!( cfg.with_wait( WaitKind::None ).is_tick_safe() );
  /// ```
  #[must_use]
  pub const fn is_tick_safe(&self) -> bool {
    self.wait.is_non_blocking()
  }
}
