//! Tests for `ring_types`, the ring family's shared vocabulary.
//!
//! Claims three features, or the parts of them this crate owns:
//!
//! - `docs/feature/167_sequence_slot_index_and_power_of_two_capacity.md`: the
//!   never-wrapping `Seq`, the derived `SlotIndex`, and the power-of-two
//!   `Capacity`. The mapping between them is `ring_index`'s half; the types and
//!   the capacity constraint are this crate's.
//! - `docs/feature/173_wait_kind_and_strategies.md`: the four `WaitKind`
//!   discriminants. The handlers are `ring_wait`'s half.
//! - `docs/feature/174_overflow_policy_enum_and_handlers.md`: the three
//!   `OverflowPolicy` discriminants, and specifically the *absence* of an
//!   overwrite-unread variant. The handlers are `ring_overflow`'s half.

use ring_types::{Capacity, OverflowPolicy, RingError, Seq, SlotIndex, WaitKind};

// ---- Seq ------------------------------------------------------------------

/// A `Seq` orders by its underlying count, which is what lets a gate compare
/// two positions a lap apart without folding either into `0..capacity`.
#[test]
fn seq_orders_by_count() {
  assert!(Seq(0) < Seq(1));
  assert!(Seq(1_000) > Seq(999));
  assert_eq!(Seq(7), Seq(7));
  assert_eq!(Seq::ZERO, Seq(0));
  assert_eq!(Seq::default(), Seq::ZERO);
}

/// `next` and `advanced_by` agree, and `advanced_by( 0 )` is the identity.
#[test]
fn seq_advances() {
  assert_eq!(Seq(41).next(), Seq(42));
  assert_eq!(Seq(10).advanced_by(5), Seq(15));
  assert_eq!(Seq(10).advanced_by(0), Seq(10));
  assert_eq!(Seq(3).next(), Seq(3).advanced_by(1));
}

/// The sequence-to-slot-index feature's central claim is that the sequence never
/// wraps. Asserted at the one place it could be. A `u64` counting publications
/// does not reach its ceiling in any reachable workload, and the type
/// deliberately offers no wrapping op.
#[test]
fn seq_does_not_wrap_within_any_reachable_workload() {
  // At one publication per nanosecond, far beyond any real ring, a u64
  // exhausts after this many years. The assertion is on the arithmetic, not on
  // a run. It is what licenses plain `<` comparison everywhere in the family.
  let per_second: u64 = 1_000_000_000;
  let seconds_per_year: u64 = 31_557_600;
  let years = u64::MAX / per_second / seconds_per_year;
  assert!(
    years > 500,
    "u64 sequence exhausts in {years} years, too few to assume monotonic"
  );
}

/// `distance_to` counts forward and saturates backward rather than wrapping,
/// so a caller that got the direction wrong sees zero, never a huge number.
#[test]
fn seq_distance_saturates_backward() {
  assert_eq!(Seq(4).distance_to(Seq(10)), 6);
  assert_eq!(Seq(10).distance_to(Seq(10)), 0);
  assert_eq!(Seq(10).distance_to(Seq(4)), 0);
  assert_eq!(Seq::ZERO.distance_to(Seq(u64::MAX)), u64::MAX);
}

// ---- SlotIndex ------------------------------------------------------------

/// `SlotIndex` is a distinct type from `Seq`, which is what stops a folded
/// position being compared against an unfolded one by accident.
#[test]
fn slot_index_is_its_own_type() {
  assert_eq!(SlotIndex(3).get(), 3);
  assert_eq!(SlotIndex::default(), SlotIndex(0));
  assert!(SlotIndex(1) < SlotIndex(2));
}

// ---- Capacity -------------------------------------------------------------

/// Every power of two from 1 to 1024 is accepted and yields the mask the
/// index derivation needs.
#[test]
fn capacity_accepts_powers_of_two() {
  let mut slots = 1usize;
  while slots <= 1024 {
    let cap = Capacity::new(slots).expect("power of two must be accepted");
    assert_eq!(cap.get(), slots);
    assert_eq!(cap.mask(), slots - 1);
    // Masking is modulo, the mask property the whole feature rests on.
    assert_eq!((slots * 3 + 1) & cap.mask(), (slots * 3 + 1) % slots);
    slots *= 2;
  }
}

/// Zero and every non-power-of-two below 32 are rejected, each with the error
/// naming which of the two rules it broke.
#[test]
fn capacity_rejects_zero_and_non_powers_of_two() {
  assert_eq!(Capacity::new(0).unwrap_err(), RingError::CapacityZero);

  for slots in 1..32usize {
    let result = Capacity::new(slots);
    if slots.is_power_of_two() {
      assert!(result.is_ok(), "{slots} is a power of two and must be accepted");
    } else {
      assert_eq!(result.unwrap_err(), RingError::CapacityNotPowerOfTwo(slots));
    }
  }
}

/// Ordering and equality on `Capacity` follow the slot count, so a config
/// comparison does not need to unwrap first.
#[test]
fn capacity_compares_by_slot_count() {
  let small = Capacity::new(4).unwrap();
  let large = Capacity::new(64).unwrap();
  assert!(small < large);
  assert_eq!(small, Capacity::new(4).unwrap());
}

// ---- WaitKind -------------------------------------------------------------

/// The wait-kind feature requires exactly four strategies. `ALL` is asserted
/// to have length four *and* to contain each named variant, so adding a fifth
/// without updating the feature fails here.
#[test]
fn wait_kind_has_exactly_four_variants() {
  assert_eq!(WaitKind::ALL.len(), 4);
  for expected in [WaitKind::Spin, WaitKind::Yield, WaitKind::Park, WaitKind::None] {
    assert!(WaitKind::ALL.contains(&expected), "{expected:?} missing from ALL");
  }
  // Exhaustive match, so a new variant breaks compilation here rather than
  // silently passing the length check via a replaced entry.
  for kind in WaitKind::ALL {
    match kind {
      WaitKind::Spin | WaitKind::Yield | WaitKind::Park | WaitKind::None => {}
    }
  }
}

/// The wait-kind feature singles out the non-blocking strategy as the one the
/// tick path needs. Exactly one variant qualifies.
#[test]
fn exactly_one_wait_kind_is_non_blocking() {
  let non_blocking: Vec<_> = WaitKind::ALL.iter().filter(|w| w.is_non_blocking()).collect();
  assert_eq!(non_blocking, vec![&WaitKind::None]);
}

/// The default is the lowest-latency strategy, so a ring built without an
/// explicit choice takes the fast path rather than parking.
#[test]
fn wait_kind_defaults_to_spin() {
  assert_eq!(WaitKind::default(), WaitKind::Spin);
}

// ---- OverflowPolicy -------------------------------------------------------

/// The overflow-policy feature's key negative claim is that no variant
/// overwrites unread data. Asserted by enumerating the whole set and matching
/// exhaustively, the only way to state an absence in Rust.
///
/// The per-variant `contains` loop closes a gap. The exhaustive `match` below
/// catches the enum *growing* past this array; until this loop, nothing caught
/// the array *decaying* while the enum stood still. Both prior checks pass for
/// any three-element array, because the length is 3 whatever it holds and the
/// `match` is exhaustive over the enum rather than over the roster.
///
/// Measured before adding it, by rewriting `ALL` two ways and running
/// `ring_types`, `ring_stats` and `ring_overflow` against each. Dropping
/// `DropOldest` failed one `ring_types` doctest, because `OverflowPolicy::ALL`'s own
/// doctest asserts `contains( &DropOldest )`. Dropping `DropNewest` passed this
/// crate **entirely**: 19/19 unit tests and 20/20 doctests green, with the
/// then-default policy gone from the roster. Both were caught downstream, by
/// `ring_stats` (4 failures) and `ring_overflow` (5), which is the wrong place
/// for it. `ALL` is declared here, so it is validated here.
#[test]
fn overflow_policy_has_no_overwrite_variant() {
  assert_eq!(OverflowPolicy::ALL.len(), 3);
  for expected in [OverflowPolicy::DropNewest, OverflowPolicy::DropOldest, OverflowPolicy::Fail] {
    assert!(OverflowPolicy::ALL.contains(&expected), "{expected:?} missing from ALL");
  }
  for policy in OverflowPolicy::ALL {
    match policy {
      // Each arm is a policy that either drops a *nameable* item or refuses.
      // No arm overwrites an unread one; a variant that did would have to be
      // added here, which is where a reviewer would see it.
      OverflowPolicy::DropNewest | OverflowPolicy::DropOldest | OverflowPolicy::Fail => {}
    }
  }
}

/// The three policies partition into "loses an item silently" and "tells the
/// caller", with no variant in both or neither.
#[test]
fn overflow_policies_partition_by_reporting() {
  for policy in OverflowPolicy::ALL {
    assert_ne!(
      policy.reports_failure(),
      policy.drops_silently(),
      "{policy:?} must be exactly one of reporting or silently dropping"
    );
  }
  assert_eq!(OverflowPolicy::ALL.iter().filter(|p| p.reports_failure()).count(), 1);
  assert_eq!(OverflowPolicy::ALL.iter().filter(|p| p.drops_silently()).count(), 2);
}

/// The default refuses, so an unconfigured ring neither evicts a queued item
/// nor reports success for an incoming one it discarded.
#[test]
fn overflow_policy_defaults_to_fail() {
  assert_eq!(OverflowPolicy::default(), OverflowPolicy::Fail);
  assert!(OverflowPolicy::default().reports_failure());
  assert!(!OverflowPolicy::default().drops_silently());
}

// ---- RingError ------------------------------------------------------------

/// Configuration errors and traffic errors are disjoint, and every variant is
/// classified by one predicate or the other rather than falling through both.
#[test]
fn errors_split_configuration_from_traffic() {
  let configuration = [
    RingError::CapacityZero,
    RingError::CapacityNotPowerOfTwo(6),
    RingError::BatchTooLarge {
      requested: 9,
      capacity: 8,
    },
    // This is configuration rather than traffic. The policy will still be unsupported
    // on the next call, so the caller must change the config, not retry.
    RingError::PolicyUnsupported,
  ];
  let traffic = [RingError::Full, RingError::Empty, RingError::Closed];
  let naming = [RingError::NameTaken, RingError::NameUnknown];

  for e in configuration {
    assert!(e.is_configuration(), "{e:?} should be a configuration error");
    assert!(!e.is_transient(), "{e:?} is not cleared by waiting");
  }
  for e in traffic {
    assert!(!e.is_configuration(), "{e:?} arises from traffic, not configuration");
  }
  for e in naming {
    assert!(!e.is_configuration(), "{e:?} is a lookup failure, not a configuration one");
    assert!(!e.is_transient(), "{e:?} is not cleared by waiting");
  }
}

/// Exactly the two conditions a peer's progress clears are transient; `Closed`
/// is not, because nothing a peer does reopens a ring.
#[test]
fn only_full_and_empty_are_transient() {
  assert!(RingError::Full.is_transient());
  assert!(RingError::Empty.is_transient());
  assert!(!RingError::Closed.is_transient());
  assert!(!RingError::CapacityZero.is_transient());
}

/// Every variant renders a distinct, non-empty message, so a log line
/// identifies which one occurred without the `Debug` form.
///
/// **The roster below is maintained by hand, and nothing here can tell you it
/// is short.** `RingError` is `#[ non_exhaustive ]`, so a `match` written in
/// this file, which is a separate crate, needs a wildcard arm and cannot be made
/// to fail the build when a variant is added. What catches an omission is
/// gate G1's 100% line-coverage threshold. An unrostered variant leaves its
/// `Display` arm unexecuted, and the gate names the file and the fraction.
///
/// That is not hypothetical. `PolicyUnsupported` was added to
/// `ring_types` while implementing `ring_core`, this list was not updated, and
/// G1 reported `ring_types/src/error.rs 16/17` on the next run. Detection took
/// one gate run rather than a compiler error. That is slower, but not silent,
/// which is the property that matters.
#[test]
fn every_error_displays_distinctly() {
  let all = [
    RingError::CapacityZero,
    RingError::CapacityNotPowerOfTwo(6),
    RingError::Full,
    RingError::Empty,
    RingError::Closed,
    RingError::NameTaken,
    RingError::NameUnknown,
    RingError::BatchTooLarge {
      requested: 9,
      capacity: 8,
    },
    RingError::PolicyUnsupported,
  ];

  let mut rendered: Vec<String> = Vec::new();
  for e in all {
    let text = e.to_string();
    assert!(!text.is_empty(), "{e:?} renders empty");
    assert!(!rendered.contains(&text), "{e:?} duplicates an earlier message");
    rendered.push(text);
  }
  assert_eq!(rendered.len(), 9);

  // The two variants carrying data put it in the message.
  assert!(RingError::CapacityNotPowerOfTwo(6).to_string().contains('6'));
  let batch = RingError::BatchTooLarge {
    requested: 9,
    capacity: 8,
  }
  .to_string();
  assert!(batch.contains('9') && batch.contains('8'));
}

/// `RingError` is usable as a `std::error::Error`, so a consumer
/// of the exported crates can box it alongside its own errors.
#[test]
fn error_implements_the_error_trait() {
  fn accepts<E: core::error::Error>(_: E) {}
  accepts(RingError::Full);
}

/// The error is `Copy` and allocation-free, which is what lets it be returned
/// from the tick path.
#[test]
fn error_is_copy() {
  let e = RingError::Full;
  let copied = e;
  assert_eq!(e, copied);
}
