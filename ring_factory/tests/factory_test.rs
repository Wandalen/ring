//! `ring_factory` — the door, and what a config actually determines once through it.
//!
//! `docs/feature/180_ring_config_and_factory.md` asks that a built ring's
//! "observable behaviour matches every field" of the config it was built from.
//! The sharpest test in this file is the one that measures how many fields that
//! is — `only_two_of_five_config_fields_are_observable_through_the_factory`.
//! The answer is two, and the other three are unobservable for three different
//! reasons.
//!
//! | Concern | Where |
//! |---|---|
//! | Feature 180's criterion, honestly scoped | `only_two_of_five_config_fields_are_observable_through_the_factory` and the two field tests around it |
//! | `docs/type/002` V1 — which variant reaches which path | `the_unnamed_path_can_never_return_name_taken`, `both_paths_relay_the_same_refusal` |
//! | `docs/type/002` V2 — a refusal leaves the registered ring untouched | `a_refusal_drops_nothing_that_was_already_registered` |
//! | `docs/invariant/001` — configuration fully determines the ring | `two_rings_from_one_config_behave_identically` |
//! | `docs/decisions` Pendings 3 and 4 — the Contract's re-exports | `the_contract_surface_is_reachable_without_naming_a_non_contract_crate` |

// Ordinary tests, compiled out under `--cfg loom`. That cfg swaps
// `ring_atomic`'s atomics for loom's instrumented ones across the whole
// family, and those panic the moment they are touched outside a
// `loom::model` closure — so without this gate a family-wide loom run dies
// here instead of reaching the models in `ring_spsc`, `ring_mpsc`,
// `ring_publish` and `ring_testkit`.
#![cfg(not(loom))]

use core::sync::atomic::{AtomicUsize, Ordering};

use ring_core::{Backend, Ring};
use ring_factory::{BuildError, Factory, Registry, RingConfig};
use ring_handle::Split;
use ring_types::{OverflowPolicy, RingError, WaitKind};

// ---------------------------------------------------------------------------
// Feature 180's criterion, and its honest scope
// ---------------------------------------------------------------------------

/// The criterion's real extent: **two of five fields**, and each of the other
/// three is silent for its own reason.
///
/// This is the test that would have to change if the family closed any of the
/// three gaps, which is what makes it worth writing as one assertion per field
/// rather than as prose in a doc.
///
/// | Field | Observable through `build`'s output? | Why |
/// |---|---|---|
/// | `capacity` | **Yes** | `free_capacity` reports it |
/// | `overflow` | **Yes** | a full ring either refuses or discards |
/// | `producers` | No | it selects a backend, and `Split` reaches no backend |
/// | `wait` | No | nothing in the closure honours it |
/// | `batch` | No | `ring_core` was implemented without reading it |
#[test]
fn only_two_of_five_config_fields_are_observable_through_the_factory() {
  // Four slots. `observable_profile` offers eight records, and the window in
  // which that discriminates anything is narrower on **both** sides —
  // measured, not reasoned (→ `tests/manual/readme.md` F2):
  //
  //    4 slots   Fail=(4,4,4)   DropNewest=(4,8,4)   <- differ; the only useful row
  //    8 slots   Fail=(8,8,8)   DropNewest=(8,8,8)   <- exactly saturates, no overflow
  //   16 slots   Fail=(16,8,8)  DropNewest=(16,8,8)  <- never fills, no overflow
  //
  // At 8 every one of the six profiles is `( 8, 8, 8 )`, so all four assertions
  // below pass whatever the factory does. At 16 the capacity assertion works
  // and the overflow one goes vacuous instead. Written at eight first, and the
  // positive assertion failed, which is the only reason the vacuous negatives
  // were noticed at all. The helper's `capacity < 8` guard excludes both cases;
  // it was written for the saturation one and covers the never-fills one by
  // accident, which is worth knowing before anyone loosens it.
  let base = RingConfig::new(4).expect("a power of two");

  // capacity — observable.
  let mut small = Factory
    .build::<u32>(RingConfig::new(4).expect("a power of two"))
    .expect("a ring");
  let mut large = Factory
    .build::<u32>(RingConfig::new(16).expect("a power of two"))
    .expect("a ring");
  assert_eq!(free_capacity(&mut small), 4);
  assert_eq!(free_capacity(&mut large), 16);

  // overflow — observable, and measured in its own test below.
  assert_ne!(
    accepted_when_overfilled(base.with_overflow(OverflowPolicy::Fail)),
    accepted_when_overfilled(base.with_overflow(OverflowPolicy::DropNewest)),
  );

  // producers — NOT observable. Two configs differing only in producer count
  // give two rings that are indistinguishable through everything `Split` offers.
  let single = base.with_producers(1);
  let multi = base.with_producers(4);
  assert_ne!(single.producers(), multi.producers(), "the configs do differ");
  assert_eq!(
    observable_profile(single),
    observable_profile(multi),
    "producer count reached the observable surface after all — this test is now the wrong shape",
  );

  // wait — NOT observable. Same shape, and additionally nothing anywhere reads it.
  assert_eq!(
    observable_profile(base.with_wait(WaitKind::Spin)),
    observable_profile(base.with_wait(WaitKind::Park)),
  );

  // batch — NOT observable.
  assert_eq!(observable_profile(base.with_batch(1)), observable_profile(base.with_batch(8)),);
}

/// `producers` is not merely unobservable — it *does* change the ring, and the
/// change is hidden by `ring_handle` on purpose.
///
/// This is the control for the negative assertion above. Without it, "the two
/// rings look the same" is equally consistent with the factory ignoring the
/// field entirely, which would be a bug rather than an encapsulation choice.
#[test]
fn the_backend_does_change_with_producer_count_where_it_can_still_be_seen() {
  let base = RingConfig::new(8).expect("a power of two");

  let single: Ring<u32> = Ring::new(&base.with_producers(1)).expect("a ring");
  let multi: Ring<u32> = Ring::new(&base.with_producers(4)).expect("a ring");

  assert_eq!(single.backend(), Backend::Spsc);
  assert_eq!(multi.backend(), Backend::Mpsc);
}

/// `capacity` reaches the built ring exactly, with no rounding of its own.
#[test]
fn capacity_reaches_the_built_ring_unchanged() {
  for slots in [1_usize, 2, 4, 8, 64, 1024] {
    let cfg = RingConfig::new(slots).expect("a power of two");
    let mut split = Factory.build::<u8>(cfg).expect("a ring");
    assert_eq!(free_capacity(&mut split), slots, "capacity {slots} did not survive the build");
  }
}

/// `overflow` reaches the built ring, and the two supported policies differ in
/// the way `ring_core` documents: `Fail` hands the record back, `DropNewest`
/// reports success and discards it.
#[test]
fn the_overflow_policy_reaches_the_built_ring() {
  let cfg = RingConfig::new(4).expect("a power of two");

  assert_eq!(accepted_when_overfilled(cfg.with_overflow(OverflowPolicy::Fail)), 4);
  assert_eq!(accepted_when_overfilled(cfg.with_overflow(OverflowPolicy::DropNewest)), 8);
}

// ---------------------------------------------------------------------------
// The two refusals
// ---------------------------------------------------------------------------

/// `docs/type/002` V1's first half: `NameTaken` is unreachable from `build`.
///
/// Asserted structurally rather than by exhaustion — `build` returns on exactly
/// two paths, and neither can name a name it was never given.
#[test]
fn the_unnamed_path_can_never_return_name_taken() {
  let cfg = RingConfig::new(8).expect("a power of two");

  for policy in [OverflowPolicy::Fail, OverflowPolicy::DropNewest, OverflowPolicy::DropOldest] {
    match Factory.build::<u32>(cfg.with_overflow(policy)) {
      Ok(_) => (),
      Err(BuildError::Unsupported(_)) => (),
      Err(BuildError::NameTaken) => panic!("the unnamed path produced NameTaken for {policy:?}"),
    }
  }
}

/// V1's second half, and the asymmetry that makes it worth a test: `Unsupported`
/// reaches **both** paths, because it originates one crate down and neither can
/// avoid it.
///
/// A suite that checked only `build_named` would pass while `build` panicked,
/// which is exactly what `docs/type/002`'s test note warns about.
#[test]
fn both_paths_relay_the_same_refusal() {
  let evicting = RingConfig::new(8)
    .expect("a power of two")
    .with_overflow(OverflowPolicy::DropOldest);
  let mut registry = Registry::<u32>::new();

  assert_eq!(
    Factory.build::<u32>(evicting).unwrap_err(),
    BuildError::Unsupported(RingError::PolicyUnsupported),
  );
  assert_eq!(
    Factory.build_named(evicting, "events", &mut registry).unwrap_err(),
    BuildError::Unsupported(RingError::PolicyUnsupported),
  );

  // Raised before a ring exists, so nothing was registered on the way past.
  assert!(registry.is_empty());
}

/// The refusal is `ring_core`'s and is relayed, not re-decided here.
///
/// If this crate had duplicated the policy check, the two would agree today and
/// silently disagree the moment a backend changed its mind. Asserting they
/// agree *now* is the cheap half; the doc comment on `build` is what states the
/// intent.
#[test]
fn the_policy_refusal_matches_the_backend_that_makes_it() {
  let evicting = RingConfig::new(8)
    .expect("a power of two")
    .with_overflow(OverflowPolicy::DropOldest);

  let from_backend = Ring::<u32>::new(&evicting).unwrap_err();
  let from_factory = Factory.build::<u32>(evicting).unwrap_err();

  assert_eq!(from_factory, BuildError::Unsupported(from_backend));
}

// ---------------------------------------------------------------------------
// Naming, and V2
// ---------------------------------------------------------------------------

/// The registering path's happy case, end to end through the re-exported
/// registry.
#[test]
fn a_named_build_is_retrievable_by_that_name_and_by_no_other() {
  let cfg = RingConfig::new(8).expect("a power of two");
  let mut registry = Registry::<u32>::new();

  Factory.build_named(cfg, "events", &mut registry).expect("a free name");

  assert!(registry.get_mut("events").is_some());
  assert!(registry.get_mut("telemetry").is_none());
  assert_eq!(registry.len(), 1);
}

/// `docs/type/002` V2, restricted to the half that can actually be observed.
///
/// V2 has two clauses and only one of them is testable from here. **"The
/// refused build's own ring is destroyed" is not** — that ring is empty by
/// construction, so no record-drop counter can see it die, and there is no
/// allocation hook in this suite to watch its buffer. It is guaranteed
/// structurally instead: `_refused` is an ordinary binding that goes out of
/// scope at the end of the match arm, and nothing in the path calls
/// `mem::forget`.
///
/// **"The registered ring is untouched" is testable, and is the clause that
/// could plausibly break** — a registry that swapped on collision, or dropped
/// the incumbent before refusing, would fail here and pass every `len()` check.
/// So the registered ring is filled with records that count their own drops,
/// and the count across the refusal is what carries the assertion.
///
/// The name of this test used to be `a_refused_registration_drops_the_ring_it_built`,
/// which named the untestable clause. → `tests/manual/readme.md` F1.
#[test]
fn a_refusal_drops_nothing_that_was_already_registered() {
  static DROPPED: AtomicUsize = AtomicUsize::new(0);

  struct Counted;

  impl Drop for Counted {
    fn drop(&mut self) {
      DROPPED.fetch_add(1, Ordering::Relaxed);
    }
  }

  let cfg = RingConfig::new(8).expect("a power of two");
  let mut registry = Registry::<Counted>::new();

  Factory.build_named(cfg, "events", &mut registry).expect("a free name");

  // Fill the *registered* ring, so any later drop count is attributable.
  {
    let held = registry.get_mut("events").expect("just registered");
    let mut ends = held.ends();
    let (mut producer, _consumer) = ends.split();
    for _ in 0..4 {
      assert!(producer.try_push(Counted).is_ok(), "within capacity");
    }
  }
  assert_eq!(DROPPED.load(Ordering::Relaxed), 0, "nothing should have dropped yet");

  // The refused build constructs a ring and must destroy it before returning.
  assert_eq!(
    Factory.build_named(cfg, "events", &mut registry).unwrap_err(),
    BuildError::NameTaken,
  );

  // Zero here is load-bearing in one direction only. It proves the refusal did
  // not destroy the *registered* ring. It says nothing about the *refused*
  // ring, which was empty and therefore drops no records whether it was
  // destroyed or leaked — measured, not assumed. → `tests/manual/readme.md` F1.
  assert_eq!(
    DROPPED.load(Ordering::Relaxed),
    0,
    "the refusal destroyed the registered ring"
  );
  assert_eq!(registry.len(), 1);

  drop(registry);
  assert_eq!(
    DROPPED.load(Ordering::Relaxed),
    4,
    "the registered ring's records outlived the registry"
  );
}

/// A refused registration leaves the existing ring's *contents* reachable, not
/// merely its name.
///
/// `len() == 1` says a name is still occupied. This says the ring behind it is
/// the original one, with the original records in it — which is the property
/// `ring_registry`'s own pitfall/001 exists about, checked here from the door.
#[test]
fn a_refused_registration_leaves_the_original_ring_intact() {
  let cfg = RingConfig::new(8).expect("a power of two");
  let mut registry = Registry::<u32>::new();

  Factory.build_named(cfg, "events", &mut registry).expect("a free name");
  {
    let held = registry.get_mut("events").expect("just registered");
    let mut ends = held.ends();
    let (mut producer, _consumer) = ends.split();
    for record in 0..4_u32 {
      producer.try_push(record).expect("within capacity");
    }
  }

  assert!(Factory.build_named(cfg, "events", &mut registry).is_err());

  let held = registry.get_mut("events").expect("still registered");
  let mut ends = held.ends();
  let (_producer, mut consumer) = ends.split();
  let survived: Vec<u32> = core::iter::from_fn(|| consumer.try_recv()).collect();
  assert_eq!(survived, vec![0, 1, 2, 3]);
}

/// Two names hold two rings, and a build under the second name is not refused
/// by the first.
#[test]
fn two_names_hold_two_rings() {
  let cfg = RingConfig::new(8).expect("a power of two");
  let mut registry = Registry::<u32>::new();

  Factory.build_named(cfg, "events", &mut registry).expect("a free name");
  Factory.build_named(cfg, "telemetry", &mut registry).expect("a free name");

  assert_eq!(registry.len(), 2);
  let mut names: Vec<&str> = registry.names().collect();
  names.sort_unstable();
  assert_eq!(names, vec!["events", "telemetry"]);
}

/// A name freed by `remove` can be built into again.
///
/// The refusal is about a *live* name, not about a name ever having been used.
#[test]
fn a_removed_name_can_be_built_into_again() {
  let cfg = RingConfig::new(8).expect("a power of two");
  let mut registry = Registry::<u32>::new();

  Factory.build_named(cfg, "events", &mut registry).expect("a free name");
  assert!(Factory.build_named(cfg, "events", &mut registry).is_err());

  registry.remove("events").expect("registered a moment ago");
  Factory
    .build_named(cfg, "events", &mut registry)
    .expect("the name is free again");
  assert_eq!(registry.len(), 1);
}

// ---------------------------------------------------------------------------
// invariant/001 — configuration fully determines the ring
// ---------------------------------------------------------------------------

/// Two builds from one config are indistinguishable.
///
/// This is the invariant's positive half: nothing outside the config — no
/// factory state, no call order, no ambient default — reached the ring.
///
/// **Only as strong as what `observable_profile` can see.** `Split` exposes
/// no backend discriminant for `wait`, `batch` or `producers`, so this would
/// pass just as cleanly if `build` silently ignored those three fields — it
/// proves determinism in `capacity` and `overflow` only, the two fields this
/// crate's surface can actually observe (`docs/invariant/001` FC22).
#[test]
fn two_rings_from_one_config_behave_identically() {
  let cfg = RingConfig::new(4)
    .expect("a power of two")
    .with_overflow(OverflowPolicy::Fail);

  assert_eq!(observable_profile(cfg), observable_profile(cfg));
}

/// Two `Factory` values are interchangeable — `docs/type/001`'s second
/// validation row, which is the row that would stop holding if the factory ever
/// gained a registry field.
#[test]
fn two_factories_build_identically() {
  let cfg = RingConfig::new(4)
    .expect("a power of two")
    .with_overflow(OverflowPolicy::Fail);

  let one = Factory;
  let other = one.clone();

  let mut from_one = one.build::<u32>(cfg).expect("a ring");
  let mut from_other = other.build::<u32>(cfg).expect("a ring");

  assert_eq!(free_capacity(&mut from_one), free_capacity(&mut from_other));
}

/// The factory owns nothing, so dropping it loses nothing.
///
/// Written as a compile-and-run assertion rather than as prose because it is
/// the property that makes `build`'s return value the ring's sole owner — the
/// ring outlives the factory that made it by an unbounded margin.
#[test]
fn a_ring_outlives_the_factory_that_built_it() {
  let cfg = RingConfig::new(8).expect("a power of two");

  let mut split = {
    let factory = Factory;
    factory.build::<u32>(cfg).expect("a ring")
  };

  let mut ends = split.ends();
  let (mut producer, mut consumer) = ends.split();
  producer.try_push(1).expect("within capacity");
  assert_eq!(consumer.try_recv(), Some(1));
}

// ---------------------------------------------------------------------------
// The Contract surface
// ---------------------------------------------------------------------------

/// `docs/decisions` Pendings 3 and 4, closed and asserted.
///
/// Every type needed to call this crate's two functions and to retrieve what
/// they built is nameable through `ring_factory` or another Contract crate.
/// **This module's own `use` list is the assertion** — it names `ring_factory`
/// and `ring_types`, both on the Contract, and `ring_core` only for the backend
/// control test above.
#[test]
fn the_contract_surface_is_reachable_without_naming_a_non_contract_crate() {
  // `RingConfig` is `ring_config`'s and is re-exported here — Pending 4.
  let cfg: RingConfig = RingConfig::new(8).expect("a power of two");
  // `Registry` is `ring_registry`'s and is re-exported here — Pending 3.
  let mut registry: Registry<u32> = Registry::new();

  Factory.build_named(cfg, "events", &mut registry).expect("a free name");
  assert!(registry.contains("events"));
}

/// `BuildError` renders both variants distinctly, and the relayed one keeps the
/// backend's own words.
#[test]
fn both_refusals_render_distinctly() {
  let taken = BuildError::NameTaken.to_string();
  let unsupported = BuildError::Unsupported(RingError::PolicyUnsupported).to_string();

  assert_ne!(taken, unsupported);
  assert!(taken.contains("already registered"), "{taken}");
  assert!(
    unsupported.contains(&RingError::PolicyUnsupported.to_string()),
    "the relay dropped the backend's own wording: {unsupported}",
  );
}

// ---------------------------------------------------------------------------
// Feature 187's door
// ---------------------------------------------------------------------------

/// The crossbeam backend accepts the policy the in-house ones refuse.
///
/// This is the whole reason the second door exists, and it is the one case
/// where `build` and `build_crossbeam` disagree about the same config.
#[cfg(feature = "crossbeam")]
#[test]
fn the_crossbeam_door_accepts_the_policy_the_in_house_backends_refuse() {
  let evicting = RingConfig::new(8)
    .expect("a power of two")
    .with_overflow(OverflowPolicy::DropOldest);

  assert!(Factory.build::<u32>(evicting).is_err());
  assert!(Factory.build_crossbeam::<u32>(evicting).is_ok());
}

/// The two doors agree on every config the in-house backends do accept.
#[cfg(feature = "crossbeam")]
#[test]
fn the_two_doors_agree_on_capacity() {
  let cfg = RingConfig::new(16).expect("a power of two");

  let mut in_house = Factory.build::<u32>(cfg).expect("a ring");
  let mut crossbeam = Factory.build_crossbeam::<u32>(cfg).expect("a ring");

  assert_eq!(free_capacity(&mut in_house), free_capacity(&mut crossbeam));
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// The free capacity of a fresh ring, read through the handle surface only.
fn free_capacity<T: Send>(split: &mut Split<T>) -> usize {
  let mut ends = split.ends();
  let (producer, _consumer) = ends.split();
  producer.free_capacity()
}

/// How many of eight pushes into a ring the producer reports as accepted.
///
/// Under `Fail` this is the capacity; under `DropNewest` it is every push,
/// because a full ring reports success and discards. That divergence is the
/// only way the overflow field is observable from out here.
fn accepted_when_overfilled(cfg: RingConfig) -> usize {
  observable_profile(cfg).1
}

/// Everything a caller can observe about a built ring, through the handle
/// surface alone: its capacity, how many of eight pushes it accepted, and how
/// many records a drain then recovered.
///
/// Deliberately exhaustive over what `ring_handle` exposes — there is no fourth
/// thing to ask a `Split`. Two configs producing equal profiles are
/// indistinguishable to any consumer of this crate, which is what makes the
/// negative assertions in
/// `only_two_of_five_config_fields_are_observable_through_the_factory`
/// meaningful rather than merely unproven.
fn observable_profile(cfg: RingConfig) -> (usize, usize, usize) {
  let mut split = Factory.build::<u32>(cfg).expect("a ring");
  let mut ends = split.ends();
  let (mut producer, mut consumer) = ends.split();

  let capacity = producer.free_capacity();
  assert!(
    capacity < 8,
    "a profile of a {capacity}-slot ring never overflows, so it discriminates nothing — \
     every comparison against it would pass vacuously",
  );

  let accepted = (0..8_u32).filter(|record| producer.try_push(*record).is_ok()).count();
  let drained = core::iter::from_fn(|| consumer.try_recv()).count();

  (capacity, accepted, drained)
}
