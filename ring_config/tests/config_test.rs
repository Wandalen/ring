//! Tests for `ring_config` — the record every ring is built from.
//!
//! Claims the configuration half of
//! `docs/feature/180_ring_config_and_factory.md`. Its acceptance criterion is
//! that `RingConfig` carries capacity, wait kind, overflow policy, producer
//! count and batch size, and that each field is *observable* one at a time —
//! the factory half, which asserts a built ring's behaviour matches each field,
//! is `ring_factory`'s and lands at stage S7.
//!
//! The clamping behaviour below is the part worth testing hardest: a builder
//! that silently corrects an impossible value is only safe if the correction is
//! specified, because a caller cannot tell a clamped value from an honoured one
//! by reading its own code.

use ring_config::RingConfig;
use ring_types::{OverflowPolicy, RingError, WaitKind};

/// Every field the feature names is present and readable.
#[test]
fn every_named_field_is_carried() {
  let cfg = RingConfig::new(1024)
    .unwrap()
    .with_wait(WaitKind::Park)
    .with_overflow(OverflowPolicy::Fail)
    .with_producers(4)
    .with_batch(64);

  assert_eq!(cfg.capacity().get(), 1024);
  assert_eq!(cfg.wait(), WaitKind::Park);
  assert_eq!(cfg.overflow(), OverflowPolicy::Fail);
  assert_eq!(cfg.producers(), 4);
  assert_eq!(cfg.batch(), 64);
}

/// A fresh configuration has the documented defaults, so a ring built without
/// any `with_*` call has a stated shape rather than an accidental one.
#[test]
fn defaults_are_the_documented_ones() {
  let cfg = RingConfig::new(8).unwrap();
  assert_eq!(cfg.wait(), WaitKind::Spin);
  assert_eq!(cfg.overflow(), OverflowPolicy::DropNewest);
  assert_eq!(cfg.producers(), 1);
  assert_eq!(cfg.batch(), 1);
}

/// Capacity validation is the constructor's, inherited from `ring_types`, so an
/// invalid configuration cannot exist to be passed on to a factory.
#[test]
fn capacity_is_validated_at_construction() {
  assert_eq!(RingConfig::new(0).unwrap_err(), RingError::CapacityZero);
  assert_eq!(RingConfig::new(7).unwrap_err(), RingError::CapacityNotPowerOfTwo(7));
  assert!(RingConfig::new(1).is_ok());
  assert!(RingConfig::new(65_536).is_ok());
}

/// Each setter changes exactly its own field and leaves the other four alone —
/// the property that makes a builder chain readable in any order.
#[test]
fn each_setter_is_independent() {
  let base = RingConfig::new(16).unwrap();

  let waited = base.with_wait(WaitKind::Yield);
  assert_eq!(waited.wait(), WaitKind::Yield);
  assert_eq!(waited.overflow(), base.overflow());
  assert_eq!(waited.producers(), base.producers());
  assert_eq!(waited.batch(), base.batch());
  assert_eq!(waited.capacity(), base.capacity());

  let overflowed = base.with_overflow(OverflowPolicy::DropOldest);
  assert_eq!(overflowed.overflow(), OverflowPolicy::DropOldest);
  assert_eq!(overflowed.wait(), base.wait());

  let produced = base.with_producers(3);
  assert_eq!(produced.producers(), 3);
  assert_eq!(produced.batch(), base.batch());

  let batched = base.with_batch(4);
  assert_eq!(batched.batch(), 4);
  assert_eq!(batched.producers(), base.producers());
}

/// Setters commute: the same five values in any order produce the same record.
#[test]
fn setters_commute() {
  let forward = RingConfig::new(32)
    .unwrap()
    .with_wait(WaitKind::None)
    .with_overflow(OverflowPolicy::Fail)
    .with_producers(2)
    .with_batch(8);

  let backward = RingConfig::new(32)
    .unwrap()
    .with_batch(8)
    .with_producers(2)
    .with_overflow(OverflowPolicy::Fail)
    .with_wait(WaitKind::None);

  assert_eq!(forward, backward);
}

/// `setters_commute` above proves order-independence only where neither clamp
/// fires — `docs/invariant/002` (RC24) names that as the easy half, since the
/// one setter with a cross-field read (`with_batch`, against `capacity`) is
/// exercised only on the path where that read has no visible effect. This is
/// the interesting half: every one of the 4! = 24 orderings of the four
/// setters, with `with_producers` and `with_batch` both given values that
/// actually clamp, must still land on the same record.
#[test]
fn setters_commute_when_both_clamps_fire() {
  #[derive(Clone, Copy)]
  enum Step {
    Wait,
    Overflow,
    Producers,
    Batch,
  }

  fn apply(cfg: RingConfig, step: Step) -> RingConfig {
    match step {
      Step::Wait => cfg.with_wait(WaitKind::Park),
      Step::Overflow => cfg.with_overflow(OverflowPolicy::DropOldest),
      // Clamps 0 up to 1.
      Step::Producers => cfg.with_producers(0),
      // Clamps 999 down to the capacity.
      Step::Batch => cfg.with_batch(999),
    }
  }

  let capacity = 8;
  let steps = [Step::Wait, Step::Overflow, Step::Producers, Step::Batch];
  let first = RingConfig::new(capacity)
    .unwrap()
    .with_wait(WaitKind::Park)
    .with_overflow(OverflowPolicy::DropOldest)
    .with_producers(0)
    .with_batch(999);

  let mut orders_checked = 0;
  for a in 0..4 {
    for b in 0..4 {
      if b == a {
        continue;
      }
      for c in 0..4 {
        if c == a || c == b {
          continue;
        }
        for d in 0..4 {
          if d == a || d == b || d == c {
            continue;
          }
          let order = [steps[a], steps[b], steps[c], steps[d]];
          let mut cfg = RingConfig::new(capacity).unwrap();
          for step in order {
            cfg = apply(cfg, step);
          }
          assert_eq!(cfg, first, "order {a}{b}{c}{d} disagrees with the first order");
          orders_checked += 1;
        }
      }
    }
  }

  assert_eq!(orders_checked, 24, "sanity: 4! permutations");
  assert_eq!(first.producers(), 1, "with_producers( 0 ) must still clamp to 1");
  assert_eq!(first.batch(), capacity, "with_batch( 999 ) must still clamp to the capacity");
}

/// A zero producer count is clamped to one, because a ring nothing can publish
/// into has no use and failing here would put a `?` in the middle of a chain.
#[test]
fn zero_producers_clamps_to_one() {
  assert_eq!(RingConfig::new(8).unwrap().with_producers(0).producers(), 1);
  assert!(!RingConfig::new(8).unwrap().with_producers(0).is_multi_producer());
}

/// A batch is clamped into `1..=capacity` at both ends: zero would publish
/// nothing, and larger than the ring can never be served however much draining
/// happens.
#[test]
fn batch_clamps_into_one_through_capacity() {
  let cfg = RingConfig::new(16).unwrap();
  assert_eq!(cfg.with_batch(0).batch(), 1);
  assert_eq!(cfg.with_batch(1).batch(), 1);
  assert_eq!(cfg.with_batch(16).batch(), 16);
  assert_eq!(cfg.with_batch(17).batch(), 16);
  assert_eq!(cfg.with_batch(usize::MAX).batch(), 16);

  // The clamp follows the capacity, not a constant.
  assert_eq!(RingConfig::new(2).unwrap().with_batch(999).batch(), 2);
}

/// `with_batch` and `with_producers` are infallible and silent — the only way
/// a caller learns a clamp fired is exactly this: keep the value asked for and
/// compare it to what came back.
#[test]
fn a_caller_can_detect_a_clamp_by_comparing_what_they_asked_for() {
  let requested_batch = 999;
  let cfg = RingConfig::new(16).unwrap().with_batch(requested_batch);
  assert_ne!(cfg.batch(), requested_batch, "the clamp fired");
  assert_eq!(cfg.batch(), 16, "and comparing is the only way to learn it");

  let requested_producers = 0;
  let cfg = RingConfig::new(16).unwrap().with_producers(requested_producers);
  assert_ne!(cfg.producers(), requested_producers, "this clamp fired too");
  assert_eq!(cfg.producers(), 1);
}

/// The one derived reading a factory branches on: a single-producer ring must
/// not pay for the contended claim it does not need.
#[test]
fn multi_producer_is_derived_from_the_count() {
  let cfg = RingConfig::new(8).unwrap();
  assert!(!cfg.is_multi_producer());
  assert!(!cfg.with_producers(1).is_multi_producer());
  assert!(cfg.with_producers(2).is_multi_producer());
  assert!(cfg.with_producers(64).is_multi_producer());
}

/// Tick-safety is exactly non-blocking waiting — the constraint feature 183
/// puts on what a system may reach.
#[test]
fn tick_safety_is_exactly_non_blocking_waiting() {
  let cfg = RingConfig::new(8).unwrap();
  for kind in WaitKind::ALL {
    assert_eq!(
      cfg.with_wait(kind).is_tick_safe(),
      kind.is_non_blocking(),
      "{kind:?} tick-safety must follow its blocking behaviour"
    );
  }
  assert!(cfg.with_wait(WaitKind::None).is_tick_safe());
  assert!(!cfg.with_wait(WaitKind::Spin).is_tick_safe());
}

/// The record is `Copy` and compares by value, so a config can be stored,
/// passed and compared without a clone or a lifetime.
#[test]
fn the_record_is_copy_and_compares_by_value() {
  let a = RingConfig::new(8).unwrap().with_batch(4);
  let b = a;
  assert_eq!(a, b);
  assert_ne!(a, RingConfig::new(8).unwrap().with_batch(2));
  assert_ne!(a, RingConfig::new(16).unwrap().with_batch(4));
}
