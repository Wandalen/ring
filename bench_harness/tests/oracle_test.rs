//! Byte-parity comparison, covering Test Matrix rows T04–T09, T11 and T12.
//!
//! Two of these rows matter most. T06 and T07 use the same two write
//! sequences, reordered the same way, and differ only in the semantics folding
//! them. `Delta` agrees and `Set` does not. That pairing shows the semantics
//! changes the result. An oracle that ignored its accumulator would pass one of
//! the two and fail the other, whichever way it was hard-coded.
//!
//! The remaining rows pin the reporting contract. The oracle returns *where*
//! two tables stop agreeing, so T05 and T09 assert an offset rather than a
//! boolean; a `Diverges` that always reported offset 0 would satisfy a
//! mismatch-only assertion and tell a candidate nothing about which write went
//! wrong.

use bench_harness::{Accumulator, ByteParity, Parity, Write};

/// T04: two byte-identical tables compare as parity.
#[test]
fn two_identical_tables_report_parity() {
  let oracle = ByteParity::new(Accumulator::Set);
  let table = [9_u8, 8, 7, 6];

  assert_eq!(oracle.compare(&table, &table), Parity::Identical);
  assert_eq!(oracle.compare(&table, &[9, 8, 7, 6]), Parity::Identical);
}

/// T05: a single differing byte is reported at its own offset.
///
/// Every offset in the table is exercised, not just one. A comparison that
/// returned a fixed offset, or that stopped at the first byte regardless,
/// would pass a single-position check and fail here.
#[test]
fn one_differing_byte_is_reported_at_its_offset() {
  let oracle = ByteParity::new(Accumulator::Set);
  let base = [1_u8, 2, 3, 4, 5];

  for offset in 0..base.len() {
    let mut corrupted = base;
    corrupted[offset] = corrupted[offset].wrapping_add(1);

    assert_eq!(
      oracle.compare(&base, &corrupted),
      Parity::Diverges { offset },
      "a byte corrupted at {offset} was not reported there",
    );
  }
}

/// T06: under `Delta`, the same writes in any order fold to the same table.
///
/// Order-independence is the property `docs/decision/050` buys by summing
/// rather than overwriting, and it is the reason an accumulator-carrying
/// component can be written by two systems in one tick without losing an
/// update.
#[test]
fn delta_folds_the_same_whatever_the_order() {
  let oracle = ByteParity::new(Accumulator::Delta);

  let forward = [Write::new(0, 10), Write::new(1, 20), Write::new(0, 5)];
  let reversed = [Write::new(0, 5), Write::new(1, 20), Write::new(0, 10)];

  assert_eq!(oracle.compare_writes(2, &forward, &reversed), Parity::Identical);

  // And the fold is the sum, not merely something stable. 10 + 5 lands in
  // cell 0. A fold that consistently kept the first write would also be
  // order-independent and would be wrong.
  assert_eq!(Accumulator::Delta.fold(2, &forward), vec![15, 20]);
}

/// T07: under `Set`, reordering changes the outcome unless the last write agrees.
///
/// The test asserts the "unless" as its own case rather than describing it in
/// prose. Reordering writes that do not touch the final value of any cell is
/// invisible to last-write-wins, and an oracle reporting a mismatch there
/// would be wrong in the opposite direction.
#[test]
fn set_depends_on_order_except_when_the_last_write_agrees() {
  let oracle = ByteParity::new(Accumulator::Set);

  let forward = [Write::new(0, 10), Write::new(0, 5)];
  let reversed = [Write::new(0, 5), Write::new(0, 10)];

  assert_eq!(
    oracle.compare_writes(1, &forward, &reversed),
    Parity::Diverges { offset: 0 },
    "last-write-wins did not notice the final write changing",
  );

  // Same reordering, but the write that lands last is the same in both. The
  // earlier writes differ in order and cannot be observed.
  let one = [Write::new(0, 1), Write::new(1, 2), Write::new(0, 7)];
  let other = [Write::new(1, 2), Write::new(0, 1), Write::new(0, 7)];

  assert_eq!(oracle.compare_writes(2, &one, &other), Parity::Identical);
}

/// T08: two empty tables are parity, not an error.
///
/// The zero-item workload is a legitimate configuration, and a grader that
/// raised on it could not grade the empty case at all.
#[test]
fn two_empty_tables_report_parity_under_either_semantics() {
  for semantics in Accumulator::ALL {
    let oracle = ByteParity::new(semantics);

    assert_eq!(
      oracle.compare(&[], &[]),
      Parity::Identical,
      "{semantics:?} rejected two empty tables"
    );
    assert_eq!(
      oracle.compare_writes(0, &[], &[]),
      Parity::Identical,
      "{semantics:?} rejected an empty fold"
    );
  }
}

/// T09: differing lengths diverge at the first offset only one table has.
///
/// Asserted in both directions. A comparison that iterated the left table
/// would report parity when the left is the shorter prefix, which is the
/// failure this row exists to catch.
#[test]
fn tables_of_different_lengths_diverge_at_the_first_excess_offset() {
  for semantics in Accumulator::ALL {
    let oracle = ByteParity::new(semantics);
    let short = [1_u8, 2, 3];
    let long = [1_u8, 2, 3, 4, 5];

    assert_eq!(
      oracle.compare(&short, &long),
      Parity::Diverges { offset: 3 },
      "{semantics:?} missed the excess when the left table was shorter",
    );
    assert_eq!(
      oracle.compare(&long, &short),
      Parity::Diverges { offset: 3 },
      "{semantics:?} missed the excess when the right table was shorter",
    );

    // An earlier disagreement outranks the length difference. The offset
    // reported is the first one, not the shorter length.
    assert_eq!(oracle.compare(&[9_u8], &long), Parity::Diverges { offset: 0 });
  }
}

/// T11: a result reports its own outcome, and the oracle reports its rule.
///
/// Callers branch on these accessors rather than on the enum shape, so they
/// are the crate's reporting API. `offset` returning `None` on
/// agreement is what lets a caller handle the match and the mismatch as one
/// expression instead of two.
#[test]
fn a_result_reports_agreement_and_where_it_stopped() {
  let oracle = ByteParity::new(Accumulator::Set);

  let agreed = oracle.compare(&[1_u8, 2], &[1, 2]);
  assert!(agreed.is_identical());
  assert_eq!(agreed.offset(), None, "an agreeing comparison still reported an offset");

  let diverged = oracle.compare(&[1_u8, 2], &[1, 9]);
  assert!(!diverged.is_identical());
  assert_eq!(
    diverged.offset(),
    Some(1),
    "the reported offset was not where the tables differ"
  );

  // And the oracle reports the rule it folded under, so a recorded result can
  // name the semantics that produced it rather than leaving it to be inferred.
  for semantics in Accumulator::ALL {
    assert_eq!(ByteParity::new(semantics).semantics(), semantics);
  }
}

/// T12: the order-independence flag agrees with what folding actually does.
///
/// Asserted against the behaviour rather than against the constant. The flag
/// is a claim *about* the fold, so the defect worth catching is the two
/// disagreeing. Restating `Delta => true` would pass whatever `fold` had been
/// changed to do.
///
/// The test uses one witness reordering, which falsifies the flag in either
/// direction but proves it in neither. A semantics that agreed here and
/// diverged on some other reordering would still be mislabelled. The witness
/// writes one cell twice with different values, because that is the case
/// last-write-wins can observe.
///
/// The membership assertion is this test's own precondition, not a separate
/// concern. "The flag matches the fold for *every* accumulator" is a claim
/// about the roster as much as about the fold, and it stops being true the
/// moment `ALL` stops being every accumulator. The loop below cannot notice
/// that on its own. Each iteration checks one variant against itself, so a
/// roster holding the same variant twice is self-consistent and passes.
///
/// Measured before adding it, by rewriting `ALL` both ways and running the
/// crate. `[Set, Set]` and `[Delta, Delta]` each scored 14/14 unit tests and
/// 14/14 doctests, green, with one of the two semantics swept by none of the
/// four `ALL` loops. Nothing downstream covers it either, because
/// `Accumulator::ALL` has no reference outside this crate. This is the same
/// gap as T6 in `ring_types`'
/// `docs/non_functional_requirement/002_the_enum_sets_are_closed_and_asserted.md`,
/// and worse. There, a doctest caught one arm and two sibling crates caught
/// both. The exhaustive `match`es in `is_order_independent` and `fold` already
/// refuse enum *growth* at compile time. Decay is the half that had nothing.
#[test]
fn the_order_independence_flag_matches_the_fold() {
  let forward = [Write::new(0, 10), Write::new(1, 20), Write::new(0, 5)];
  let reversed = [Write::new(0, 5), Write::new(1, 20), Write::new(0, 10)];

  for expected in [Accumulator::Set, Accumulator::Delta] {
    assert!(Accumulator::ALL.contains(&expected), "{expected:?} missing from ALL");
  }

  for semantics in Accumulator::ALL {
    let reordering_was_invisible = ByteParity::new(semantics)
      .compare_writes(2, &forward, &reversed)
      .is_identical();

    assert_eq!(
      semantics.is_order_independent(),
      reordering_was_invisible,
      "{semantics:?} claims an order-independence its own fold does not honour",
    );
  }
}
