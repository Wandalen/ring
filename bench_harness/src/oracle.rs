//! Deciding whether two completed table states agree, and where they stop.

use crate::accumulator::{Accumulator, Write};

/// What the oracle found when it compared two tables.
///
/// The diverging case carries the offset rather than a bare `false`, because
/// *where* two write paths stop agreeing identifies which one is wrong. A
/// boolean would make every candidate fail the same way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Parity {
  /// Every byte agrees, and both tables are the same length.
  Identical,
  /// The tables first disagree at this byte offset.
  ///
  /// When one table is a prefix of the other, the offset is the length of the
  /// shorter one, which is the first position at which only one table has a byte.
  Diverges {
    /// Byte offset of the first disagreement.
    offset: usize,
  },
}

impl Parity {
  /// Whether the tables agreed.
  ///
  /// ```
  /// use bench_harness::Parity;
  ///
  /// assert!( Parity::Identical.is_identical() );
  /// assert!( !Parity::Diverges { offset : 0 }.is_identical() );
  /// ```
  // Fix(parity_is_identical_classification_not_exhaustive): was `matches!( self,
  //   Self::Identical )`, so a third `Parity` variant would silently read
  //   `false`, meaning "not identical", with nothing in this function forcing
  //   a second look. `offset` just below already has the exhaustive shape.
  // Root cause: `matches!` over a single named variant is exhaustive over
  //   nothing. It answers every variant it was not told about with the same
  //   default, and compiles cleanly however many variants `Parity` gains. Same
  //   shape as this crate's own `Accumulator::is_order_independent` fix just
  //   above.
  // Pitfall: a caller gating "did the tables agree?" on this predicate would
  //   treat a new kind of disagreement as agreement. The error is silent, and
  //   it hides a real divergence from the oracle's own caller.
  #[must_use]
  pub const fn is_identical(self) -> bool {
    match self {
      Self::Identical => true,
      Self::Diverges { .. } => false,
    }
  }

  /// The offset of the first disagreement, or `None` when the tables agreed.
  ///
  /// ```
  /// use bench_harness::Parity;
  ///
  /// assert_eq!( Parity::Identical.offset(), None );
  /// assert_eq!( Parity::Diverges { offset : 4 }.offset(), Some( 4 ) );
  /// ```
  #[must_use]
  pub const fn offset(self) -> Option<usize> {
    match self {
      Self::Identical => None,
      Self::Diverges { offset } => Some(offset),
    }
  }
}

/// The byte-parity oracle, bound to the semantics it compares under.
///
/// The smoke test this oracle serves is *"all patterns produce byte-identical
/// final tables"*, and that claim is only meaningful once the fold rule is named.
/// Two write paths that disagree under [`Accumulator::Set`] may agree under
/// [`Accumulator::Delta`], and vice versa. So the semantics is a constructor
/// argument, not a per-call one, and an oracle never compares in the
/// abstract.
///
/// ```
/// use bench_harness::{ Accumulator, ByteParity, Parity };
///
/// let oracle = ByteParity::new( Accumulator::Set );
/// assert_eq!( oracle.compare( &[ 1, 2 ], &[ 1, 2 ] ), Parity::Identical );
/// assert_eq!( oracle.compare( &[ 1, 2 ], &[ 1, 9 ] ), Parity::Diverges { offset : 1 } );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ByteParity {
  semantics: Accumulator,
}

impl ByteParity {
  /// An oracle comparing under `semantics`.
  #[must_use]
  pub const fn new(semantics: Accumulator) -> Self {
    Self { semantics }
  }

  /// The semantics this oracle folds write sequences under.
  ///
  /// ```
  /// use bench_harness::{ Accumulator, ByteParity };
  ///
  /// assert_eq!( ByteParity::new( Accumulator::Delta ).semantics(), Accumulator::Delta );
  /// ```
  #[must_use]
  pub const fn semantics(self) -> Accumulator {
    self.semantics
  }

  /// Compare two completed tables byte for byte.
  ///
  /// Two empty tables are [`Parity::Identical`] rather than an error. Nothing
  /// written to nothing is a legitimate outcome, and a grader that treats it as
  /// a failure cannot grade the zero-item workload.
  ///
  /// ```
  /// use bench_harness::{ Accumulator, ByteParity, Parity };
  ///
  /// let oracle = ByteParity::new( Accumulator::Set );
  /// assert_eq!( oracle.compare( &[], &[] ), Parity::Identical );
  /// assert_eq!( oracle.compare( &[ 1 ], &[ 1, 2 ] ), Parity::Diverges { offset : 1 } );
  /// ```
  #[must_use]
  pub fn compare(self, left: &[u8], right: &[u8]) -> Parity {
    let shared = left.len().min(right.len());

    for offset in 0..shared {
      if left[offset] != right[offset] {
        return Parity::Diverges { offset };
      }
    }

    if left.len() == right.len() {
      Parity::Identical
    } else {
      Parity::Diverges { offset: shared }
    }
  }

  /// Fold two write sequences under this oracle's semantics, then compare.
  ///
  /// This is the call where the semantics changes the result. Two sequences,
  /// one a reordering of the other, agree under [`Accumulator::Delta`] and
  /// generally disagree under [`Accumulator::Set`].
  ///
  /// ```
  /// use bench_harness::{ Accumulator, ByteParity, Write };
  ///
  /// let forward = [ Write::new( 0, 3 ), Write::new( 0, 4 ) ];
  /// let reverse = [ Write::new( 0, 4 ), Write::new( 0, 3 ) ];
  ///
  /// assert!( ByteParity::new( Accumulator::Delta ).compare_writes( 1, &forward, &reverse ).is_identical() );
  /// assert!( !ByteParity::new( Accumulator::Set ).compare_writes( 1, &forward, &reverse ).is_identical() );
  /// ```
  #[must_use]
  pub fn compare_writes(self, cells: usize, left: &[Write], right: &[Write]) -> Parity {
    self.compare(&self.semantics.fold(cells, left), &self.semantics.fold(cells, right))
  }
}
