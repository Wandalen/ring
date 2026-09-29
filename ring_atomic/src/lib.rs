//! Atomic sequence cells with explicit memory orderings.
//!
//! Tier 2 of the ring family's 33 crates — the concurrency write-path implementation.
//! Depends on `ring_types`.
//!
//! Every cursor in the family is a sequence in a shared cell, and every read or
//! write of one is an ordering decision. Spreading those decisions across the
//! crates that happen to need a cursor means the family's memory model is
//! whatever the sum of those call sites turns out to be. Concentrating them here
//! makes the model one thing that can be read in one place.
//!
//! The orderings are therefore **named, not defaulted**: [`SeqCell`]'s methods
//! take an explicit [`Ordering`], and this crate never picks one on a caller's
//! behalf. A helper that quietly chose `SeqCst` would make every operation
//! correct and every benchmark meaningless, which for a workstream whose whole
//! output is a measured verdict is the worse failure. That governs the cells
//! callers hold; [`CountingSeq`]'s own bookkeeping counters are `Relaxed`
//! throughout, decided once here rather than per call, because they exist
//! only to be totalled afterwards and no ordering closes the read/read tear
//! between them (see [`CountingSeq::counts`]).
//!
//! ## Why a trait rather than a struct
//!
//! Two acceptance criteria in
//! `ring/bench_harness/docs/acceptance/001_feature_reached_tests.md` are
//! *negative* claims about atomic traffic — feature 175's "accumulates N items
//! with zero atomic operations" and feature 177's "a claim of 64 slots issues
//! one fence, not 64". Neither can be asserted against a bare `AtomicU64`,
//! because the count is not observable from outside. [`SeqCell`] exists so the
//! crates that perform those operations can be written once and run against
//! either [`AtomicSeq`] in production or [`CountingSeq`] in a test that needs
//! the count.
//!
//! No `unsafe`: `AtomicU64` is a safe abstraction, so nothing here needs to opt
//! out of the workspace-wide `unsafe-code = "deny"`. This crate used to hold an
//! entry in `ring/bench_harness/gate/declared/ring/unsafe_allowlist.txt` saying
//! otherwise;
//! [decision 123](../../../docs/decision/123_ring_shared_slot_storage_unsafe_sited.md)
//! removed it after checking all four listed crates and finding not one of them
//! exercised the permission.
//!
//! ## The `loom` seam
//!
//! Under `--cfg loom` the two atomics below come from `loom` instead of `core`.
//! This is the family's only such switch, and it is here for the same reason
//! the orderings are: this crate is the one place in 33 crates where a
//! *sequence* atomic is created. Every cursor, gating set, claim and barrier
//! reaches its atomic through [`AtomicSeq`], so the `loom` switch here reaches
//! all of them — the counting instrument reaches only the callers generic
//! enough to accept [`CountingSeq`] in its place — and no other crate needs to
//! know the seam exists.
//!
//! It costs one thing, stated at each site: loom's atomics carry model state
//! and have no `const` constructor, so [`AtomicSeq::new`] and
//! [`CountingSeq::new`] — and, downstream, `ring_cursor`'s two — are `const`
//! only in an ordinary build.
//!
//! `ring_publish/tests/handshake_test.rs` is what uses it, and is run with
//! `RUSTFLAGS="--cfg loom" cargo test -p ring_publish --test handshake_test`.

#![deny(missing_docs)]

use core::sync::atomic::Ordering;
#[cfg(not(loom))]
use core::sync::atomic::{AtomicU64, AtomicUsize};

#[cfg(loom)]
use loom::sync::atomic::{AtomicU64, AtomicUsize};
use ring_types::Seq;

/// A shared cell holding one sequence.
///
/// Implemented by [`AtomicSeq`] for production and [`CountingSeq`] for tests
/// that assert how much atomic traffic an operation actually generated. Every
/// method takes the [`Ordering`] explicitly — see the module documentation for
/// why none is defaulted.
///
/// ```
/// use core::sync::atomic::Ordering;
/// use ring_atomic::{ AtomicSeq, SeqCell };
/// use ring_types::Seq;
///
/// let cell = AtomicSeq::new( Seq( 4 ) );
/// assert_eq!( cell.load( Ordering::Acquire ), Seq( 4 ) );
/// assert_eq!( cell.fetch_add( 3, Ordering::AcqRel ), Seq( 4 ) );
/// assert_eq!( cell.load( Ordering::Acquire ), Seq( 7 ) );
/// ```
/// # Why `Sync`
///
/// Every cell in the family is shared between a producer and a consumer, so
/// `Sync` is the actual requirement — it is stated here rather than left to be
/// satisfied by accident.
///
/// Fix(AT7, AT45, AT46): the trait used to have no supertrait. Both
/// implementors are `Sync` by auto-derivation from the atomics inside them, so
/// nothing had ever failed; but `&dyn SeqCell` was **not** `Sync`, making the
/// object form unusable for the crate's only stated purpose without writing
/// `&( dyn SeqCell + Sync )` at every site, and the three generic `C : SeqCell`
/// bounds in `ring_batch` and `ring_tls` depended on a property none of them
/// asked for. With the supertrait, a `!Sync` implementor fails at its own
/// `impl` — naming this trait — instead of at some later use site naming a
/// `Cell< u64 >`.
///
/// Root cause: a requirement satisfied by composition rather than declared.
/// Pitfall: object safety is not object *usability* — check what the `dyn` form
/// auto-implements, not only that it compiles.
pub trait SeqCell: Sync {
  /// Read the current sequence.
  fn load(&self, order: Ordering) -> Seq;

  /// Overwrite the sequence.
  fn store(&self, value: Seq, order: Ordering);

  /// Advance by `n` and return the sequence as it was *before* the advance —
  /// which is the first sequence the caller now owns.
  ///
  /// # Monotonicity
  ///
  /// **Not guaranteed by this method.** `n` is a `u64` because the cell is a
  /// `u64`, and the addition wraps: `fetch_add( u64::MAX )` moves the cursor
  /// *back* by one and returns a value indistinguishable from a legitimate
  /// claim of a huge range. `AtomicSeq::new` likewise accepts any `Seq`, so a
  /// cursor seeded from persistence or a fixture can start arbitrarily close to
  /// the top. Callers own the monotonicity that `ring_types` and `ring_consume`
  /// each describe as something "every gate in the family relies on".
  ///
  /// Fix(AT21, AT41, AT42): the word *monotonic* did not appear anywhere in the
  /// crate that owns every operation those two statements are about, and no
  /// test touched the boundary. `fetch_add_wraps_at_the_top_of_u64` now records
  /// what happens there. No runtime check was added — at a billion advances per
  /// second the horizon is 585 years, and the cost of a branch on every claim
  /// is not worth paying for it — but silence was not the alternative.
  ///
  /// Root cause: the reverse direction comes free with `u64` addition and
  /// nothing in the signature or the contract rules it out.
  /// Pitfall: an unreachable-by-counting state is still reachable in one call
  /// by anyone who seeds the cursor.
  #[must_use = "the returned sequence is the claim — dropping it claims a range nobody will use"]
  fn fetch_add(&self, n: u64, order: Ordering) -> Seq;

  /// Advance from `current` to `new` only if the cell still reads `current`.
  ///
  /// # Errors
  ///
  /// The sequence actually found, when it was not `current` — the multi-producer
  /// claim's retry input.
  fn compare_exchange(&self, current: Seq, new: Seq, success: Ordering, failure: Ordering) -> Result<Seq, Seq>;
}

/// The production sequence cell: one `AtomicU64`, no bookkeeping.
///
/// ```
/// use core::sync::atomic::Ordering;
/// use ring_atomic::{ AtomicSeq, SeqCell };
/// use ring_types::Seq;
///
/// let cell = AtomicSeq::default();
/// assert_eq!( cell.load( Ordering::Relaxed ), Seq::ZERO );
/// cell.store( Seq( 9 ), Ordering::Release );
/// assert_eq!( cell.load( Ordering::Acquire ), Seq( 9 ) );
/// ```
#[derive(Debug)]
pub struct AtomicSeq(AtomicU64);

impl Default for AtomicSeq {
  /// A cell at [`Seq::ZERO`].
  ///
  /// Written out rather than derived so that it does not depend on whichever
  /// `AtomicU64` is in scope having its own `Default` — one of the two comes
  /// from `loom` and its trait impls are its own business, not something this
  /// crate should be pinned to.
  fn default() -> Self {
    Self::new(Seq::ZERO)
  }
}

impl AtomicSeq {
  /// A cell holding `value`.
  ///
  /// `const` in an ordinary build. Not under `--cfg loom`, whose atomics carry
  /// per-execution model state and have no `const` constructor — see the module
  /// documentation on the seam.
  ///
  /// ```
  /// use ring_atomic::AtomicSeq;
  /// use ring_types::Seq;
  /// let _ = AtomicSeq::new( Seq( 1 ) );
  /// ```
  #[cfg(not(loom))]
  #[must_use]
  pub const fn new(value: Seq) -> Self {
    Self(AtomicU64::new(value.0))
  }

  /// A cell holding `value` — the `--cfg loom` build, where it is not `const`.
  #[cfg(loom)]
  #[must_use]
  pub fn new(value: Seq) -> Self {
    Self(AtomicU64::new(value.0))
  }
}

impl SeqCell for AtomicSeq {
  fn load(&self, order: Ordering) -> Seq {
    Seq(self.0.load(order))
  }

  fn store(&self, value: Seq, order: Ordering) {
    self.0.store(value.0, order);
  }

  fn fetch_add(&self, n: u64, order: Ordering) -> Seq {
    Seq(self.0.fetch_add(n, order))
  }

  fn compare_exchange(&self, current: Seq, new: Seq, success: Ordering, failure: Ordering) -> Result<Seq, Seq> {
    self
      .0
      .compare_exchange(current.0, new.0, success, failure)
      .map(Seq)
      .map_err(Seq)
  }
}

/// How many atomic operations of each kind a [`CountingSeq`] has served.
///
/// `total` is the number that matters for the two negative acceptance criteria:
/// zero for a thread-local accumulation, one for a 64-slot batch claim.
///
/// ```
/// use ring_atomic::OpCounts;
/// assert_eq!( OpCounts::default().total, 0 );
/// ```
///
/// # Not a Coherent Observation
///
/// [`CountingSeq::counts`] fills this struct with four independent `Relaxed`
/// loads, so under concurrency the fields can come from four different instants
/// — see that method for the measured skew. Two consequences follow, and the
/// second is the one that bites:
///
/// 1. A relationship between two fields may be one no instant ever satisfied.
/// 2. **The tearing is not detectable from the value.** `total` is the sum of
///    the same four readings, so `loads + stores + fetch_adds +
///    compare_exchanges == total` holds for every torn struct as strongly as
///    for a clean one. The consistency check a suspicious caller would reach
///    for certifies the torn readings as sound.
///
/// Fix(AT43): the internal relation was the only cross-check available and it
/// is derived from the torn reads rather than independently of them, so nothing
/// was returned that could reveal the problem. This note is that signal;
/// `total_is_derived_from_the_same_four_reads` pins the fact that the relation
/// is arithmetic rather than evidentiary.
///
/// Root cause: a derived field computed from the same reads it would have to
/// audit cannot audit them.
/// Pitfall: an invariant that holds by construction proves the construction,
/// not the data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OpCounts {
  /// Reads served.
  pub loads: usize,
  /// Writes served.
  pub stores: usize,
  /// Advances served — the batch claim's own operation.
  pub fetch_adds: usize,
  /// Compare-exchanges served, successful or not — the contended claim's.
  pub compare_exchanges: usize,
  /// Every operation above, summed.
  pub total: usize,
}

/// A sequence cell that behaves exactly like [`AtomicSeq`] and counts what it
/// was asked to do.
///
/// Not a mock: the underlying operations are the same real atomics, so a test
/// running against this observes the same values production would — at
/// roughly double the cost per operation, so this is a correctness instrument
/// and not a timing one. Only the bookkeeping is added — which is why an
/// assertion made against it is a statement about the code under test rather
/// than about a substitute for it.
///
/// The four counters are packed against the cell with no padding between
/// them, deliberately: every operation touches its own counter *and* the
/// cell, so separating them onto their own cache lines pays for an extra
/// line acquisition on every call rather than avoiding one, and measures
/// slower past four threads.
///
/// ```
/// use core::sync::atomic::Ordering;
/// use ring_atomic::{ CountingSeq, SeqCell };
/// use ring_types::Seq;
///
/// let cell = CountingSeq::default();
/// assert_eq!( cell.counts().total, 0 );
///
/// let first = cell.fetch_add( 64, Ordering::AcqRel );
/// assert_eq!( first, Seq::ZERO );
///
/// // One operation bought all 64 slots.
/// assert_eq!( cell.counts().fetch_adds, 1 );
/// assert_eq!( cell.counts().total, 1 );
/// ```
#[derive(Debug)]
pub struct CountingSeq {
  cell: AtomicSeq,
  loads: AtomicUsize,
  stores: AtomicUsize,
  fetch_adds: AtomicUsize,
  compare_exchanges: AtomicUsize,
}

impl Default for CountingSeq {
  /// A counting cell at [`Seq::ZERO`], with every count at zero.
  fn default() -> Self {
    Self::new(Seq::ZERO)
  }
}

impl CountingSeq {
  /// A counting cell holding `value`, with every count at zero.
  ///
  /// `const` in an ordinary build, and not under `--cfg loom`, for the reason
  /// given on [`AtomicSeq::new`].
  ///
  /// ```
  /// use ring_atomic::CountingSeq;
  /// use ring_types::Seq;
  /// assert_eq!( CountingSeq::new( Seq( 2 ) ).counts().total, 0 );
  /// ```
  #[cfg(not(loom))]
  #[must_use]
  pub const fn new(value: Seq) -> Self {
    Self {
      cell: AtomicSeq::new(value),
      loads: AtomicUsize::new(0),
      stores: AtomicUsize::new(0),
      fetch_adds: AtomicUsize::new(0),
      compare_exchanges: AtomicUsize::new(0),
    }
  }

  /// A counting cell holding `value` — the `--cfg loom` build, not `const`.
  #[cfg(loom)]
  #[must_use]
  pub fn new(value: Seq) -> Self {
    Self {
      cell: AtomicSeq::new(value),
      loads: AtomicUsize::new(0),
      stores: AtomicUsize::new(0),
      fetch_adds: AtomicUsize::new(0),
      compare_exchanges: AtomicUsize::new(0),
    }
  }

  /// What this cell has been asked to do so far — read as four separate
  /// values, so meaningful only when nothing else is touching the cell.
  ///
  /// ```
  /// use core::sync::atomic::Ordering;
  /// use ring_atomic::{ CountingSeq, SeqCell };
  ///
  /// let cell = CountingSeq::default();
  /// cell.load( Ordering::Relaxed );
  /// assert_eq!( cell.counts().loads, 1 );
  /// ```
  ///
  /// # Not a Snapshot
  ///
  /// This is **four independent `Relaxed` loads**, not one atomic read. Under
  /// concurrent use the four fields can come from four different instants, and
  /// [`OpCounts::total`] is the sum of readings that were never simultaneously
  /// true. Assert on a single field, or take the reading while nothing else
  /// touches the cell — never on a *relationship between two fields*.
  ///
  /// Fix(AT3): a two-million-sample probe against a writer whose own loop keeps
  /// `loads >= stores` true at every instant found 11,575 returned structs
  /// reporting `stores > loads`, the widest by 5,456. Concurrency is not
  /// hypothetical here: every `SeqCell` in the family is shared between a
  /// producer and a consumer, and this crate's own contention test drives four
  /// threads against one cell.
  ///
  /// Root cause: bundling several independent atomic reads into one struct
  /// makes the struct look like a single observation.
  /// Pitfall: a cross-field assertion on this type fails intermittently, and
  /// its author will look for the cause in the code under test.
  #[must_use]
  pub fn counts(&self) -> OpCounts {
    let loads = self.loads.load(Ordering::Relaxed);
    let stores = self.stores.load(Ordering::Relaxed);
    let fetch_adds = self.fetch_adds.load(Ordering::Relaxed);
    let compare_exchanges = self.compare_exchanges.load(Ordering::Relaxed);
    OpCounts {
      loads,
      stores,
      fetch_adds,
      compare_exchanges,
      total: loads + stores + fetch_adds + compare_exchanges,
    }
  }

  /// Return every count to zero, leaving the sequence itself untouched.
  ///
  /// Four independent `Relaxed` stores, not one atomic reset — safe to read
  /// as complete only while nothing else is touching the cell. For a test
  /// that sets up a state through the cell and then wants to count only what
  /// the operation under test does; confirming the sequence survived costs a
  /// count of its own, since the confirming read is itself counted.
  ///
  /// ```
  /// use core::sync::atomic::Ordering;
  /// use ring_atomic::{ CountingSeq, SeqCell };
  /// use ring_types::Seq;
  ///
  /// let cell = CountingSeq::default();
  /// cell.store( Seq( 5 ), Ordering::Release );
  /// cell.reset_counts();
  ///
  /// assert_eq!( cell.counts().total, 0 );
  /// assert_eq!( cell.load( Ordering::Acquire ), Seq( 5 ), "the sequence survives" );
  /// ```
  pub fn reset_counts(&self) {
    for counter in [&self.loads, &self.stores, &self.fetch_adds, &self.compare_exchanges] {
      counter.store(0, Ordering::Relaxed);
    }
  }
}

/// Each method bumps its counter **before** delegating to the cell.
///
/// Fix(AT24): the order is deliberate and was undocumented. A concurrent
/// observer reading the counter and the cell while both are moving can
/// therefore see a counter that has already been incremented for an operation
/// the cell has not yet performed — a million-sample probe found 7,176 such
/// orderings. Bumping after the delegation would only move the window, not
/// close it; closing it needs a lock, which is the cost this type exists to
/// measure rather than pay.
///
/// The consequence is bounded and worth stating plainly: at rest the counts are
/// exact (`counts_are_exact_under_contention` asserts that), and in flight they
/// lead. Never assert a counter against the cell's value while anything is
/// still running.
///
/// Root cause: two pieces of state updated in sequence are observable between
/// the updates.
/// Pitfall: "the counts are exact" is a statement about quiescence, and every
/// test that establishes it is a test taken at rest.
impl SeqCell for CountingSeq {
  fn load(&self, order: Ordering) -> Seq {
    self.loads.fetch_add(1, Ordering::Relaxed);
    self.cell.load(order)
  }

  fn store(&self, value: Seq, order: Ordering) {
    self.stores.fetch_add(1, Ordering::Relaxed);
    self.cell.store(value, order);
  }

  fn fetch_add(&self, n: u64, order: Ordering) -> Seq {
    self.fetch_adds.fetch_add(1, Ordering::Relaxed);
    self.cell.fetch_add(n, order)
  }

  fn compare_exchange(&self, current: Seq, new: Seq, success: Ordering, failure: Ordering) -> Result<Seq, Seq> {
    self.compare_exchanges.fetch_add(1, Ordering::Relaxed);
    self.cell.compare_exchange(current, new, success, failure)
  }
}
