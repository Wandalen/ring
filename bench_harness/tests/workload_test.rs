//! Determinism of the seeded workload.
//!
//! The first three tests are deliberately not independent. A generator that
//! returns a constant satisfies the same-seed test alone, and the different-seeds
//! test exists to refute exactly that reading. It asserts two seeds disagree, so
//! the same-seed agreement is evidence of reproducibility rather than of a stuck
//! generator. The producer-count test then separates "reproducible at one
//! producer count" from "reproducible across them". This crate needs the
//! second, because a candidate is benched at several producer counts and the
//! comparison is only meaningful if it saw the same items each time.

use bench_harness::{Item, PayloadArchetype, Workload};

/// Every archetype, so a case added to the enum fails a test rather than
/// silently going unmeasured.
const ARCHETYPES: [PayloadArchetype; 2] = [PayloadArchetype::Uniform, PayloadArchetype::Mixed];

/// The family's mixing constant, restated here deliberately rather than read
/// from the crate. A test that imported it could not notice the crate
/// changing it.
const MIXING_CONSTANT: u64 = 0x9E37_79B9_7F4A_7C15;

/// Indices only, for comparing sequences by identity rather than payload.
fn indices(items: &[Item]) -> Vec<u64> {
  items.iter().map(|item| item.index).collect()
}

/// The same seed produces a byte-identical sequence on two separate runs.
///
/// Constructed twice rather than cloned, so the assertion covers the generator
/// rather than the copy.
#[test]
fn the_same_seed_produces_the_same_sequence_on_two_separate_runs() {
  for archetype in ARCHETYPES {
    let first = Workload::new(0xFEED, 4, 256, 64, archetype).items();
    let second = Workload::new(0xFEED, 4, 256, 64, archetype).items();

    assert_eq!(first, second, "{archetype:?} sequence differed between two runs of one seed");
    assert_eq!(first.len(), 256, "{archetype:?} produced the wrong item count");
  }
}

/// One producer and eight produce the same multiset of items.
///
/// Asserts the stronger of the two available readings. The weaker one is that
/// the two producer counts agree with each other. The stronger one is that each
/// partitions the full sequence exactly, with every item owned by exactly one
/// producer, nothing duplicated and nothing dropped. A generator that lost the
/// same item at both counts would satisfy the weaker reading and fail this one.
#[test]
fn every_producer_count_partitions_the_same_multiset() {
  let expected = Workload::new(0xFEED, 1, 256, 64, PayloadArchetype::Uniform).items();

  for producers in [1_usize, 8] {
    let workload = Workload::new(0xFEED, producers, 256, 64, PayloadArchetype::Uniform);

    let mut collected: Vec<Item> = (0..producers).flat_map(|p| workload.items_for(p)).collect();
    collected.sort_by_key(|item| item.index);

    assert_eq!(
      collected, expected,
      "{producers} producer(s) did not reconstruct the full sequence"
    );
  }

  // The partition is a division of labour, not every producer receiving
  // everything. That would also satisfy the reconstruction above once
  // duplicates were sorted away, had the comparison been on a set.
  let dealt = Workload::new(0xFEED, 8, 256, 8, PayloadArchetype::Uniform);
  for producer in 0..8 {
    assert_eq!(
      dealt.items_for(producer).len(),
      32,
      "producer {producer} was not dealt an equal share"
    );
  }
  assert!(
    dealt.items_for(8).is_empty(),
    "a producer beyond the declared count owned items"
  );

  // **Fewer batches than producers leaves producers idle**, and this test
  // asserts that rather than smoothing it over. 256 items in batches of 64 is
  // four batches. Dealt round-robin to eight producers, the last four get
  // nothing and the config is measuring a four-producer workload whatever it
  // says. Hiding this would report an eight-producer number for a
  // four-producer run.
  let starved = Workload::new(0xFEED, 8, 256, 64, PayloadArchetype::Uniform);
  assert_eq!(
    starved.items_for(0).len(),
    64,
    "the first batch did not go to producer 0 whole"
  );
  assert!(
    starved.items_for(4).is_empty(),
    "a producer past the batch count was dealt items"
  );
  assert!(
    starved.items_for(7).is_empty(),
    "a producer past the batch count was dealt items"
  );
}

/// Two different seeds produce different sequences.
///
/// This is the anti-faking check for the same-seed test. Without it, a
/// generator returning a constant would pass every determinism assertion in
/// this file.
#[test]
fn two_different_seeds_produce_different_sequences() {
  for archetype in ARCHETYPES {
    let one = Workload::new(1, 4, 256, 64, archetype).items();
    let other = Workload::new(2, 4, 256, 64, archetype).items();

    assert_ne!(one, other, "{archetype:?} produced the same sequence for two different seeds");

    // The indices are positional and must agree; only the payloads may differ.
    // Asserting this separates "a different sequence" from "a differently
    // ordered one", which are not the same claim.
    assert_eq!(
      indices(&one),
      indices(&other),
      "{archetype:?} changed item order with the seed"
    );
  }
}

/// A workload reports the configuration it is actually running, not the
/// one it was asked for.
///
/// The accessors exist so a result can be labelled with what produced it, and
/// a run labelled with the requested config rather than the effective one is a
/// mislabelled measurement. `new` clamps a zero producer count and a zero
/// batch size to one, so the interesting assertion is that the accessors show
/// the clamp instead of echoing the zero back.
#[test]
fn a_workload_reports_its_effective_configuration() {
  let asked = Workload::new(0xABC, 4, 256, 64, PayloadArchetype::Mixed);

  assert_eq!(asked.seed(), 0xABC);
  assert_eq!(asked.producer_count(), 4);
  assert_eq!(asked.item_count(), 256);
  assert_eq!(asked.batch_size(), 64);
  assert_eq!(asked.archetype(), PayloadArchetype::Mixed);

  let clamped = Workload::new(0xABC, 0, 8, 0, PayloadArchetype::Uniform);

  assert_eq!(
    clamped.producer_count(),
    1,
    "a zero producer count was reported back unclamped"
  );
  assert_eq!(clamped.batch_size(), 1, "a zero batch size was reported back unclamped");

  // The clamp is required, not cosmetic. Both clamped fields are divisors in
  // `owner_of`, so a config that kept the zero would divide by it rather than
  // deal anything. Dealing the full sequence is the evidence.
  assert_eq!(clamped.items_for(0).len(), 8, "the clamped config did not deal every item");
}

/// The seed can be recovered by unmixing any item.
///
/// Added after a mutation survey found the first three tests all survive
/// replacing `^` with `|` or `&`. Those three assert the sequence is
/// *reproducible* and *seed-dependent*, and every bitwise operator satisfies
/// both, so nothing pinned the mixing operation itself. The generator behind
/// every recorded benchmark comparison could have been changed silently, and
/// past runs would have become incomparable with no test failing.
///
/// Asserted as XOR's own algebraic property, that unmixing returns the seed,
/// rather than against captured output bytes. Captured bytes would only
/// restate whatever the code happens to do today.
#[test]
fn the_seed_is_recoverable_from_any_item() {
  let seed = 0x0123_4567_89AB_CDEF_u64;
  let workload = Workload::new(seed, 4, 64, 8, PayloadArchetype::Uniform);

  for index in 0..64_u64 {
    let mixed = u64::from_le_bytes(workload.item(index).payload);

    assert_eq!(
      mixed ^ index.wrapping_mul(MIXING_CONSTANT),
      seed,
      "item {index} did not unmix back to the seed",
    );
  }
}

/// `Mixed` carries `1 + index % 8` significant bytes and zero padding
/// beyond them.
///
/// Added after the same survey found `1 + ( index % 8 )` could become
/// `1 * ( index % 8 )` with every test still green. The first three tests
/// compare `Mixed` sequences only against other `Mixed` sequences, so a width
/// schedule that changed consistently was invisible to all of them.
///
/// Pinned against the `Uniform` payload for the same index rather than against
/// captured bytes. The significant prefix must agree with the full record,
/// and everything past the width must be padding.
#[test]
fn the_mixed_archetype_pads_beyond_its_significant_width() {
  let seed = 0x0123_4567_89AB_CDEF_u64;
  let full = Workload::new(seed, 4, 64, 8, PayloadArchetype::Uniform);
  let mixed = Workload::new(seed, 4, 64, 8, PayloadArchetype::Mixed);

  for index in 0..16_u64 {
    let width = 1 + (index % 8) as usize;
    let full_payload = full.item(index).payload;
    let mixed_payload = mixed.item(index).payload;

    assert_eq!(
      &mixed_payload[..width],
      &full_payload[..width],
      "index {index} did not carry {width} significant byte(s)",
    );
    assert!(
      mixed_payload[width..].iter().all(|byte| *byte == 0),
      "index {index} had a non-zero byte past its {width}-byte width",
    );
  }
}
