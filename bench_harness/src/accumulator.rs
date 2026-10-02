//! The two semantics a write sequence may be folded under before comparison.

/// One recorded write against a table cell.
///
/// A write carries no timestamp and no producer id. Under [`Accumulator::Set`]
/// its position in the sequence decides the outcome. Under
/// [`Accumulator::Delta`] its position does not matter. That asymmetry is the
/// whole subject of this module. Because the record itself is order-agnostic,
/// the same slice can be folded both ways.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Write {
  /// Which cell of the table this write lands on.
  pub cell: usize,
  /// The value written, or added, depending on the semantics folding it.
  pub value: u8,
}

impl Write {
  /// A write of `value` against `cell`.
  ///
  /// ```
  /// use bench_harness::Write;
  ///
  /// let w = Write::new( 3, 7 );
  /// assert_eq!( w.cell, 3 );
  /// assert_eq!( w.value, 7 );
  /// ```
  #[must_use]
  pub const fn new(cell: usize, value: u8) -> Self {
    Self { cell, value }
  }
}

/// How a sequence of [`Write`]s folds into the table an oracle compares.
///
/// `docs/decision/050_deferred_mutation_accumulator_scope.md` rules on this. It
/// scopes last-write-wins to structural changes and idempotent overwrites, and
/// requires accumulation-style writes to be summed instead. The distinction is
/// not a matter of preference. For an accumulation, a deterministic overwrite
/// loses an update exactly as surely as an arbitrary one does.
///
/// ```
/// use bench_harness::{ Accumulator, Write };
///
/// let writes = [ Write::new( 0, 3 ), Write::new( 0, 4 ) ];
/// assert_eq!( Accumulator::Set.fold( 1, &writes ), vec![ 4 ] );
/// assert_eq!( Accumulator::Delta.fold( 1, &writes ), vec![ 7 ] );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Accumulator {
  /// Last write wins. Order-dependent, and safe only where the correctness of
  /// a write does not depend on the value it replaces.
  #[default]
  Set,
  /// Writes are summed. Order-independent, because wrapping addition over the
  /// cell is commutative and associative.
  ///
  /// A negative delta is carried as its two's-complement byte, so a `-100`
  /// travels as `156` and still sums correctly in any order.
  Delta,
}

impl Accumulator {
  /// Every variant, in discriminant order.
  ///
  /// The length constrains this array against its own initialiser, never
  /// against the enum. A third variant leaves `ALL` at two and passes the
  /// assertion below. What refuses a third variant is the wildcard-free `match`
  /// in `is_order_independent` just under this, and the one in `fold` below it.
  /// Both live in this file, so growth is a compile error rather than a test
  /// failure. The assertions can reach decay in the other direction, where a
  /// variant is replaced by a duplicate of its sibling. The membership check in
  /// `the_order_independence_flag_matches_the_fold` (`tests/oracle_test.rs`)
  /// covers that case and records the measurement that put it there.
  ///
  /// ```
  /// use bench_harness::Accumulator;
  /// assert_eq!( Accumulator::ALL.len(), 2 );
  /// ```
  pub const ALL: [Self; 2] = [Self::Set, Self::Delta];

  /// Whether folding under this semantics is independent of write order.
  ///
  /// ```
  /// use bench_harness::Accumulator;
  ///
  /// assert!( !Accumulator::Set.is_order_independent() );
  /// assert!( Accumulator::Delta.is_order_independent() );
  /// ```
  // Fix(accumulator_order_independence_not_exhaustive): was `matches!( self, Self::Delta )`,
  // which classifies silently instead of refusing to compile. A third variant would
  // compile straight into `false` with no signal. `fold`'s own exhaustive match just
  // below, by contrast, forces a decision. `Accumulator::ALL` and the T12 test in
  // `tests/oracle_test.rs` only cover variants a developer remembers to add to `ALL`, so
  // they are not a substitute for this. Same shape as `DispatchStrategy`'s
  // `matches!`-based coordination predicates silently misclassifying a new strategy.
  #[must_use]
  pub const fn is_order_independent(self) -> bool {
    match self {
      Self::Set => false,
      Self::Delta => true,
    }
  }

  /// Fold `writes` into a `cells`-long table under this semantics.
  ///
  /// Cells no write touches read zero. A write naming a cell at or beyond
  /// `cells` is discarded instead of panicking. A table is a fixed extent, and
  /// an out-of-range write is a property of the workload, not an error the
  /// oracle should raise.
  ///
  /// ```
  /// use bench_harness::{ Accumulator, Write };
  ///
  /// let writes = [ Write::new( 1, 5 ), Write::new( 9, 1 ) ];
  /// assert_eq!( Accumulator::Delta.fold( 2, &writes ), vec![ 0, 5 ] );
  /// ```
  #[must_use]
  pub fn fold(self, cells: usize, writes: &[Write]) -> Vec<u8> {
    let mut table = vec![0_u8; cells];

    for write in writes {
      let Some(cell) = table.get_mut(write.cell) else { continue };

      match self {
        Self::Set => *cell = write.value,
        Self::Delta => *cell = cell.wrapping_add(write.value),
      }
    }

    table
  }
}
