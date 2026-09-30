//! Comparative write-path measurements — mutex, ring, and thread-local staging.
//!
//! One of the ring family's 33 crates — the concurrency write-path implementation. This is the
//! last of them, and the only one whose output is a *number* rather than a
//! type: feature 186 asks for one workload run against every candidate write
//! path, and says plainly that "the comparison is the deliverable".
//!
//! ```
//! use ring_bench::{Comparison, Workload};
//! use ring_factory::RingConfig;
//!
//! let workload =
//!     Workload::new(RingConfig::new(1024).unwrap()).with_records_per_producer(256).unwrap();
//!
//! let comparison = Comparison::run(workload);
//! assert!(comparison.fastest().is_some(), "1024 slots hold 256 records");
//! ```
//!
//! # What is measured, and what is not
//!
//! The **write** path. Producers publish; the drain happens afterwards, outside
//! the timed region, and is where every count that this crate treats as truth
//! comes from. A benchmark of a queue that also times the read side measures
//! two things and reports one number.
//!
//! Nothing here asserts a *ranking*. [`Comparison::fastest`] exists, and this
//! crate's own test suite never asserts which candidate it returns — an
//! assertion about wall-clock time is a flaky test on a shared machine, and a
//! flaky test in a benchmark harness discredits the measurement it exists to
//! protect. What the suite asserts instead is record accounting: offered,
//! reported, received, dropped. Those are deterministic, and they are what make
//! a timing comparison mean anything at all.
//!
//! # A path that drops records is not eligible to be fastest
//!
//! This is the crate's one load-bearing rule. Under a capacity smaller than the
//! offered load every candidate refuses records, and the candidate that refuses
//! them *fastest* has the lowest elapsed time. Ranking by time alone therefore
//! ranks the worst path first, and does so silently — the number looks fine.
//! [`Comparison::fastest`] filters on [`Outcome::is_lossless`] before comparing,
//! and returns `None` when no candidate kept everything.
//!
//! # What a push *reported* is not what the ring *kept*
//!
//! `OverflowPolicy::default()` is `DropNewest`, so a bare `RingConfig::new( n )`
//! configures a ring on which `try_push` returns `Ok` for a record it discarded.
//! This harness counted those `Ok`s as accepted records in its first working
//! version, and [`Candidate::ContractRing`] duly reported **256 successes into
//! a 16-slot ring** — a lossless-looking run that kept 6% of the workload, and
//! a fast one, because discarding is the cheapest thing a queue can do.
//!
//! So [`Outcome::received`] is drained from the ring and is the only number
//! treated as truth. [`Outcome::reported`] keeps what the API claimed, and the
//! gap between them is published as [`Outcome::silently_discarded`] — not
//! hidden, because it is the measurement that separates a path with
//! back-pressure from one without. [`Outcome::conserved`] is `false` exactly
//! when that gap is open, and a `false` is a fact about the policy rather than
//! a failure of the run.
//!
//! `ring_shutdown/docs/pitfall/002` names this trap ("Ok does not mean kept
//! under `DropNewest`") and this crate is where it was paid for: the trap does
//! not corrupt a record, it corrupts a *verdict*, and the verdict is what
//! this crate exists to produce. See `docs/pitfall/003`.
//!
//! # The Contract door imposes a producer ceiling the data structures do not
//!
//! [`Candidate::ContractRing`] and `Candidate::OffTheShelf` both reach a
//! multi-producer data structure — `ring_mpsc`'s ring and crossbeam's
//! `ArrayQueue` — and both are capped at **one** producer here. The cap is not
//! theirs.
//!
//! The second is named rather than linked because it exists only under the
//! `crossbeam` feature, and this paragraph does not: a link from ungated prose
//! to a gated item resolves under `--all-features` and dangles in a default
//! build, which is the configuration most readers of these docs are in. `ring_factory::build` returns a `ring_handle::Split`, whose `Ends`
//! yields exactly one producer and offers no way to ask for a second;
//! `ring_core::Producer::try_clone` is the operation that would, and
//! `ring_handle` deliberately does not re-expose it.
//!
//! So feature 186's "under the same producer counts" is not satisfiable through
//! one door. [`Candidate::DirectMpsc`] exists to make the gap measurable rather
//! than merely stated: it is the same ring as `ContractRing` in a multi-producer
//! configuration, reached two levels lower, and it has no ceiling. See
//! `docs/pitfall/001`.
//!
//! # The counters are recorded outside the timed region
//!
//! Feature 185's [`RingStats`] turns a result into a diagnosis rather than a
//! bare number, and incrementing an atomic once per record would change the
//! thing being measured. Every counter here is therefore written **once, from
//! totals, after the clock stops**. The cost is that a run reports no
//! intra-run distribution; the benefit is that the number it reports is of the
//! write path and not of the instrumentation. See `docs/pitfall/002`.

#![deny(missing_docs)]

use core::fmt;
use core::ops::Range;
use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use ring_factory::{BuildError, Factory, RingConfig};
use ring_flush::{ConfigError, FlushOutcome, FlushPolicy, Flusher};
use ring_slot::TypedSlot;
use ring_stats::RingStats;
use ring_tls::TlsBuffer;
use ring_types::RingError;

/// The record every candidate moves.
///
/// One fixed payload for the whole comparison, because a candidate measured
/// against a different payload is not in the comparison. Eight bytes: large
/// enough to carry an identity, small enough that the measurement is of the
/// publication protocol rather than of `memcpy`.
pub type Record = u64;

/// How a repeated write to the same accumulator cell resolves against an
/// earlier one.
///
/// `docs/decision/050_deferred_mutation_accumulator_scope.md` names both:
/// **`Set`** is last-write-wins, safe for structural changes and idempotent
/// overwrites, and unsafe the moment two producers land in the same cell
/// within one measured window — the second write erases the first rather than
/// combining with it. **`Delta`** sums every write into the cell instead, so
/// the result is the same regardless of which producer's write physically
/// lands last — safe for any destination more than one producer writes into,
/// because summation is commutative and overwriting is not.
///
/// `ring_bench` measured only `Set` before this axis existed — every
/// candidate's write to a slot was a plain overwrite, unnamed as such because
/// there was nothing to name it against.
/// `docs/decision/121_workstream_008_contract_gaps_ruled.md` ruling 3 requires
/// both semantics measured against every candidate; see [`Workload::with_semantics`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccumulatorSemantics {
    /// Last write wins. What every candidate measured before this axis existed.
    Set,
    /// Every write sums into the cell, regardless of arrival order.
    Delta,
}

// ── Workload ──────────────────────────────────────────────────────────────

/// Why a workload description was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkloadError {
    /// Zero producers — nothing would write, and every candidate would tie at
    /// zero.
    ZeroProducers,
    /// Zero records per producer — the producers would start and stop.
    ZeroRecords,
    /// Zero batch — staging would never reach a flush, so the staged candidates
    /// would publish nothing and the unstaged ones would be unaffected. A
    /// comparison in which one arm is inert is not a comparison.
    ZeroBatch,
    /// Zero accumulator cells — every producer's records would decode a
    /// destination via `producer % cells`, which panics on division by zero.
    ZeroCells,
}

impl fmt::Display for WorkloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroProducers => f.write_str("a workload needs at least one producer"),
            Self::ZeroRecords => f.write_str("a workload needs at least one record per producer"),
            Self::ZeroBatch => f.write_str("a workload needs a batch size of at least one"),
            Self::ZeroCells => f.write_str("a workload needs at least one accumulator cell"),
        }
    }
}

impl core::error::Error for WorkloadError {}

/// One workload, run identically against every candidate.
///
/// Built from a [`RingConfig`], which is the family's own configuration record
/// and reaches this crate through `ring_factory`'s re-export — so a workload is
/// described in the same vocabulary a consumer would use to build the ring.
///
/// # The producer count lives here and the config follows it
///
/// `RingConfig` carries its own `producers` field, and it selects the *backend*
/// rather than the thread count. Two independent numbers both called
/// "producers" is a trap, so [`with_producers`](Self::with_producers) sets both
/// and there is no way to set them apart:
///
/// ```
/// use ring_bench::Workload;
/// use ring_factory::RingConfig;
///
/// let workload = Workload::new(RingConfig::new(64).unwrap()).with_producers(4).unwrap();
///
/// assert_eq!(workload.producers(), 4);
/// assert_eq!(workload.config().producers(), 4);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Workload {
    config: RingConfig,
    producers: usize,
    records_per_producer: usize,
    cells: usize,
    semantics: AccumulatorSemantics,
}

impl Workload {
    /// A single producer publishing 1024 records in batches of 32, into one
    /// accumulator cell under `Set` semantics.
    ///
    /// The defaults are deliberately modest: this crate's own suite runs them,
    /// and a suite that takes a second per case stops being run. One cell is
    /// the smallest destination that makes `Set` and `Delta` observably
    /// different — see [`with_cells`](Self::with_cells) and
    /// [`with_semantics`](Self::with_semantics) to widen either.
    #[must_use]
    pub const fn new(config: RingConfig) -> Self {
        Self {
            config,
            producers: 1,
            records_per_producer: 1024,
            cells: 1,
            semantics: AccumulatorSemantics::Set,
        }
    }

    /// Set how many threads publish concurrently, and the backend that matches.
    ///
    /// # Errors
    ///
    /// [`WorkloadError::ZeroProducers`] for zero.
    pub fn with_producers(mut self, producers: usize) -> Result<Self, WorkloadError> {
        if producers == 0 {
            return Err(WorkloadError::ZeroProducers);
        }

        self.producers = producers;
        self.config = self.config.with_producers(producers);
        Ok(self)
    }

    /// Set how many records each producer publishes.
    ///
    /// # Errors
    ///
    /// [`WorkloadError::ZeroRecords`] for zero.
    pub fn with_records_per_producer(mut self, records: usize) -> Result<Self, WorkloadError> {
        if records == 0 {
            return Err(WorkloadError::ZeroRecords);
        }

        self.records_per_producer = records;
        Ok(self)
    }

    /// Set how many records accumulate before a publication.
    ///
    /// Four of six candidates honour this: the mutex queue takes its lock once
    /// per batch, `ContractRing` and `OffTheShelf` publish a batch at a time,
    /// and the staged candidate binds it as its [`FlushPolicy::OnBatch`]
    /// trigger. `DirectSpsc` and `DirectMpsc` push one record per call and do
    /// not read this field at all — they receive it through [`Workload::config`]
    /// but the backends they construct from read only its capacity (→ BN28).
    ///
    /// # Errors
    ///
    /// [`WorkloadError::ZeroBatch`] for zero.
    pub fn with_batch(mut self, batch: usize) -> Result<Self, WorkloadError> {
        if batch == 0 {
            return Err(WorkloadError::ZeroBatch);
        }

        self.config = self.config.with_batch(batch);
        Ok(self)
    }

    /// Set how many destination cells drained records fold into.
    ///
    /// `producer % cells` picks the cell — so `cells == producers` gives every
    /// producer its own, collision-free destination, and `cells < producers`
    /// forces two or more producers to land in the same cell within one run.
    /// The second shape is the one [`AccumulatorSemantics::Set`] gets wrong and
    /// [`AccumulatorSemantics::Delta`] gets right, per `docs/decision/050`.
    ///
    /// # Errors
    ///
    /// [`WorkloadError::ZeroCells`] for zero.
    pub fn with_cells(mut self, cells: usize) -> Result<Self, WorkloadError> {
        if cells == 0 {
            return Err(WorkloadError::ZeroCells);
        }

        self.cells = cells;
        Ok(self)
    }

    /// Set how a repeated write to the same accumulator cell resolves.
    ///
    /// `Set` is what every candidate measured before this axis existed, and
    /// remains the default from [`Workload::new`]. See
    /// [`AccumulatorSemantics`]'s own docs for what each variant means.
    #[must_use]
    pub const fn with_semantics(mut self, semantics: AccumulatorSemantics) -> Self {
        self.semantics = semantics;
        self
    }

    /// The ring configuration every candidate is built from. `Copy`.
    #[must_use]
    pub const fn config(&self) -> RingConfig {
        self.config
    }

    /// How many threads publish concurrently.
    #[must_use]
    pub const fn producers(&self) -> usize {
        self.producers
    }

    /// How many records each producer publishes.
    #[must_use]
    pub const fn records_per_producer(&self) -> usize {
        self.records_per_producer
    }

    /// How many destination cells drained records fold into.
    #[must_use]
    pub const fn cells(&self) -> usize {
        self.cells
    }

    /// How a repeated write to the same accumulator cell resolves.
    #[must_use]
    pub const fn semantics(&self) -> AccumulatorSemantics {
        self.semantics
    }

    /// How many records accumulate before a publication.
    ///
    /// Read through the config, not from a field of this type's own — so the
    /// value returned here is the one [`RingConfig::with_batch`] accepted after
    /// clamping to the capacity, and is the same number every candidate actually
    /// publishes with.
    ///
    /// Fix(BN8, BN9): this used to return an unclamped `Workload` field, so a
    /// workload built with `capacity 16` and `with_batch( 32 )` reported a batch
    /// of 32 while its own config carried 16 — a pair `RingConfig` refuses to
    /// hold, printed in [`Comparison::report`]'s header line and driven into
    /// every candidate's publish loop.
    ///
    /// Root cause: two fields held the same quantity and only one of them went
    /// through validation.
    /// Pitfall: when a validating type already stores a value, storing it a
    /// second time outside that type makes the unvalidated copy the one every
    /// caller reads.
    #[must_use]
    pub const fn batch(&self) -> usize {
        self.config.batch()
    }

    /// How many slots the ring under test holds.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.config.capacity().get()
    }

    /// How many records the whole workload offers.
    #[must_use]
    pub const fn offered(&self) -> usize {
        self.producers * self.records_per_producer
    }

    /// The records one producer publishes.
    ///
    /// Disjoint per producer, so a drained record identifies its writer. Nothing
    /// in this crate asserts on the values yet — they exist so that a future
    /// ordering question can be asked of a recorded run rather than of a new one.
    #[must_use]
    pub fn records_of(&self, producer: usize) -> Range<Record> {
        let start = (producer * self.records_per_producer) as Record;
        start..start + self.records_per_producer as Record
    }
}

// ── Candidate ─────────────────────────────────────────────────────────────

/// One write path under comparison.
///
/// Feature 186 names four — a mutex-guarded queue, an off-the-shelf concurrent
/// queue, the in-house ring, and thread-local staging over that ring. "The
/// in-house ring" is three variants here rather than one, because the three
/// doors onto it do not admit the same producer counts and the feature also
/// requires the candidates be run "under the same producer counts". A single
/// variant would have had to pick one door and quietly drop the requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Candidate {
    /// `Mutex< VecDeque< Record > >` — the baseline every other candidate has to
    /// beat to justify its existence. Bounded by the workload's capacity so that
    /// it competes under the same back-pressure as the rings rather than
    /// absorbing the whole load.
    MutexQueue,
    /// The in-house ring reached the way the workstream Contract says to reach
    /// it: `ring_factory::build`, returning a `ring_handle::Split`.
    ContractRing,
    /// Thread-local staging over the in-house ring — `ring_tls::TlsBuffer`
    /// accumulating, `ring_flush::Flusher` publishing on a batch policy.
    TlsOverRing,
    /// `ring_spsc` driven directly, two levels below the Contract. Prices the
    /// dispatch `ring_core` and `ring_handle` add: same ring, same single
    /// producer, no wrapper.
    DirectSpsc,
    /// `ring_mpsc` driven directly. The only in-house candidate with no producer
    /// ceiling, and the reason the comparison can be run at more than one
    /// producer at all.
    DirectMpsc,
    /// The off-the-shelf concurrent queue, through `ring_factory`'s second door.
    #[cfg(feature = "crossbeam")]
    OffTheShelf,
}

impl Candidate {
    /// Every candidate, in a fixed order.
    ///
    /// The order is load-bearing, not cosmetic: [`Comparison::fastest`] breaks a
    /// tie by taking the first minimum, so the candidate declared first wins a tie
    /// (BN50). `the_candidate_list_matches_a_copy_written_outside_the_declaration`
    /// pins this list's contents and order against a copy written in the suite, so
    /// a reorder of either `cfg` arm fails rather than silently changing a verdict.
    #[cfg(feature = "crossbeam")]
    pub const ALL: &'static [Self] = &[
        Self::MutexQueue,
        Self::ContractRing,
        Self::TlsOverRing,
        Self::DirectSpsc,
        Self::DirectMpsc,
        Self::OffTheShelf,
    ];
    /// Every candidate, in a fixed order. The order decides a tie in
    /// [`Comparison::fastest`] — see the crossbeam-gated copy above (BN50).
    #[cfg(not(feature = "crossbeam"))]
    pub const ALL: &'static [Self] = &[
        Self::MutexQueue,
        Self::ContractRing,
        Self::TlsOverRing,
        Self::DirectSpsc,
        Self::DirectMpsc,
    ];

    /// A stable name for reports.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::MutexQueue => "mutex_queue",
            Self::ContractRing => "contract_ring",
            Self::TlsOverRing => "tls_over_ring",
            Self::DirectSpsc => "direct_spsc",
            Self::DirectMpsc => "direct_mpsc",
            #[cfg(feature = "crossbeam")]
            Self::OffTheShelf => "off_the_shelf",
        }
    }

    /// The most producers this candidate can be driven by, if it is bounded.
    ///
    /// `None` means unbounded. `Some( 1 )` means the candidate can only be run
    /// single-producer — and of the bounded ones, the door imposes the ceiling
    /// rather than the data structure behind it for most of them: two of three
    /// by default, three of four with the `crossbeam` feature on (the table
    /// below is the six-candidate build):
    ///
    /// | Candidate | Ceiling | Imposed by |
    /// |---|---|---|
    /// | [`MutexQueue`](Self::MutexQueue) | none | — |
    /// | [`ContractRing`](Self::ContractRing) | 1 | `ring_handle::Ends::split`, which yields one non-clonable producer |
    /// | [`TlsOverRing`](Self::TlsOverRing) | 1 | `ring_flush::Flusher` owns its producer, and staging is per-thread by definition |
    /// | [`DirectSpsc`](Self::DirectSpsc) | 1 | the backend — SPSC is single-producer, and this is the only honest 1 in the table |
    /// | [`DirectMpsc`](Self::DirectMpsc) | none | — |
    /// | `OffTheShelf` | 1 | `ring_handle` again; `ArrayQueue` itself is multi-producer |
    #[must_use]
    pub const fn producer_ceiling(self) -> Option<usize> {
        match self {
            Self::MutexQueue | Self::DirectMpsc => None,
            Self::ContractRing | Self::TlsOverRing | Self::DirectSpsc => Some(1),
            #[cfg(feature = "crossbeam")]
            Self::OffTheShelf => Some(1),
        }
    }

    /// Whether this candidate can be driven by a workload's producer count.
    #[must_use]
    pub const fn admits(self, producers: usize) -> bool {
        match self.producer_ceiling() {
            Some(ceiling) => producers <= ceiling,
            None => true,
        }
    }
}

// ── Accumulator ───────────────────────────────────────────────────────────

/// The destination every candidate's drained records fold into.
///
/// One cell per [`Workload::cells`], built once per run by replaying every
/// received record through the workload's active [`AccumulatorSemantics`].
/// Comparing two tables built from the same workload is
/// `docs/spike/023_smoke_ring_write_path.md`'s pass criterion: byte-identical
/// final tables regardless of which candidate produced them or what order its
/// records happened to arrive in — a property `Delta` semantics guarantees by
/// construction (sum is commutative) and `Set` semantics does not (last write
/// wins, and "last" depends on scheduling).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccumulatorTable {
    cells: Vec<i64>,
}

impl AccumulatorTable {
    fn new(cells: usize) -> Self {
        Self { cells: vec![0; cells] }
    }

    fn apply(&mut self, semantics: AccumulatorSemantics, cell: usize, delta: i64) {
        match semantics {
            AccumulatorSemantics::Set => self.cells[cell] = delta,
            AccumulatorSemantics::Delta => self.cells[cell] += delta,
        }
    }

    /// The accumulated value in each cell, in cell order.
    #[must_use]
    pub fn cells(&self) -> &[i64] {
        &self.cells
    }
}

/// Decode a drained record into the cell it targets and the delta it carries.
///
/// [`Workload::records_of`] hands out disjoint per-producer ranges, so the
/// producer that wrote a record is recoverable from the record's own value —
/// no side channel is needed. The sign alternates by producer index,
/// mirroring `docs/decision/050`'s fee/payout pair: an odd-indexed producer
/// subtracts, an even-indexed one adds, so a `cells() < producers()` workload
/// has a correct total that only `Delta` semantics reconstructs.
///
/// # Panics
///
/// If the decoded producer index falls outside `0 .. workload.producers()`. A
/// genuine drained value always decodes inside that range, because
/// `records_of` never hands out anything else — a value that decodes outside
/// it did not come from a real write. This is this crate's measured stand-in
/// for a torn read (`docs/hard_problem/129_gating_so_the_producer_never_laps.md`):
/// every table built by [`run`] passes every drained record through this
/// check, so a torn or corrupted value is caught here rather than silently
/// folded into a total that merely looks plausible.
fn destination_of(workload: &Workload, record: Record) -> (usize, i64) {
    let producer = (record / workload.records_per_producer() as Record) as usize;
    assert!(
        producer < workload.producers(),
        "record {record} decodes to producer {producer}, outside 0..{} — torn or corrupted write",
        workload.producers(),
    );

    let cell = producer % workload.cells();
    let delta = if producer.is_multiple_of(2) { 1 } else { -1 };
    (cell, delta)
}

// ── Outcome ───────────────────────────────────────────────────────────────

/// What one candidate did with one workload.
///
/// Three counts, and the distinction between the last two is the crate's most
/// expensive lesson: [`offered`](Self::offered) is what the workload asked for,
/// [`reported`](Self::reported) is what the write API said it took, and
/// [`received`](Self::received) is what the drain actually produced. Under the
/// default `DropNewest` policy the second can exceed the third by two orders of
/// magnitude, so every derived judgement here — losslessness, drops, fastest —
/// is computed from `received`.
#[derive(Debug)]
pub struct Outcome {
    candidate: Candidate,
    producers: usize,
    offered: usize,
    reported: usize,
    received: usize,
    write_nanos: u128,
    stats: RingStats,
    table: AccumulatorTable,
}

impl Outcome {
    /// Which write path produced this.
    #[must_use]
    pub const fn candidate(&self) -> Candidate {
        self.candidate
    }

    /// How many threads published.
    #[must_use]
    pub const fn producers(&self) -> usize {
        self.producers
    }

    /// How many records the workload offered.
    #[must_use]
    pub const fn offered(&self) -> usize {
        self.offered
    }

    /// How many the write API said it took.
    ///
    /// **Not evidence a record was kept.** Under `OverflowPolicy::DropNewest` a
    /// full ring returns `Ok` for a record it discarded, so this is an upper
    /// bound on what landed and nothing more. Published because the gap against
    /// [`received`](Self::received) is itself a measurement — see
    /// [`silently_discarded`](Self::silently_discarded).
    #[must_use]
    pub const fn reported(&self) -> usize {
        self.reported
    }

    /// How many came back out of the drain.
    ///
    /// The only count treated as truth. Every judgement this crate makes about a
    /// candidate is computed from it.
    #[must_use]
    pub const fn received(&self) -> usize {
        self.received
    }

    /// How many never landed, by any mechanism.
    ///
    /// Refused and silently discarded records both count here — from the
    /// workload's point of view they are the same event, and the difference
    /// between them is what [`silently_discarded`](Self::silently_discarded)
    /// reports.
    /// # Panics
    ///
    /// If `received` exceeds `offered` — the first half of this crate's ordering
    /// invariant. The panic is unconditional rather than a debug-build overflow
    /// check, so it fires in release too.
    ///
    /// Fix(BN22): the invariant's Violation Consequences argued a violation would
    /// be loud because "release panics on integer underflow". That is true only of
    /// a project setting `overflow-checks = true`, which nothing in this workspace
    /// does; in release the subtraction wrapped to `18446744073709551360` — the
    /// value the same paragraph offered as the counterfactual.
    ///
    /// Root cause: a claim about a build profile read as a claim about the
    /// language, and no test runs in release to contradict it.
    /// Pitfall: if loudness is load-bearing for an unguarded subtraction, write
    /// the guard — do not inherit it from a profile setting nobody set.
    #[must_use]
    pub const fn dropped(&self) -> usize {
        assert!(self.received <= self.offered, "received exceeded offered");
        self.offered - self.received
    }

    /// How many the path accepted and then did not have.
    ///
    /// Zero under `OverflowPolicy::Fail`, where a refusal is returned to the
    /// caller. Nonzero under `DropNewest`, and that is the policy behaving as
    /// specified rather than a defect — the defect would be a harness that
    /// counted these as delivered.
    /// # Panics
    ///
    /// If `received` exceeds `reported` — the second half of the ordering
    /// invariant. Unconditional, for the reason given on
    /// [`dropped`](Self::dropped) (BN22).
    #[must_use]
    pub const fn silently_discarded(&self) -> usize {
        assert!(self.received <= self.reported, "received exceeded reported");
        self.reported - self.received
    }

    /// Wall-clock nanoseconds spent in the write phase.
    ///
    /// The drain is not in this figure, and neither is construction. **Never
    /// assert on it** — see this crate's module documentation.
    #[must_use]
    pub const fn write_nanos(&self) -> u128 {
        self.write_nanos
    }

    /// Feature 185's counters for this run, written once from totals after the
    /// clock stopped.
    #[must_use]
    pub const fn stats(&self) -> &RingStats {
        &self.stats
    }

    /// The accumulator table this run produced.
    ///
    /// Built once, after the drain, by replaying every received record through
    /// [`Workload::semantics`] — see [`AccumulatorTable`]'s own docs for what
    /// "byte-identical" means when comparing two of these across candidates.
    #[must_use]
    pub const fn table(&self) -> &AccumulatorTable {
        &self.table
    }

    /// Whether the drain produced everything the workload offered.
    ///
    /// The eligibility test for [`Comparison::fastest`], and computed from
    /// [`received`](Self::received) rather than [`reported`](Self::reported) —
    /// which is the whole point, since a `DropNewest` ring reports success for
    /// every record it is given no matter how small it is.
    #[must_use]
    pub const fn is_lossless(&self) -> bool {
        self.received == self.offered
    }

    /// Whether the write API's success count matched what came back out.
    ///
    /// Not a correctness assertion — under `DropNewest` this is `false` by
    /// design, and the run is still valid. It is the flag that says *how* the
    /// candidate's `dropped` figure was arrived at: `true` means the path handed
    /// its refusals back to the caller, `false` means it absorbed them.
    #[must_use]
    pub const fn conserved(&self) -> bool {
        self.reported == self.received
    }
}

// ── Errors ────────────────────────────────────────────────────────────────

/// Why a candidate could not be run.
///
/// Every variant names the candidate. Fix(BN25): the three that relay another
/// crate's refusal — `ring_factory`'s, `ring_core`'s and `ring_flush`'s — used
/// to be tuple variants carrying only the inner error, so `Display` forwarded
/// to a message with no candidate on it. Those are exactly the cases a reader
/// most needs the name for, because the same `RingConfig` is offered to all
/// five candidates and only one of them refused it.
///
/// Root cause: relaying an error verbatim discards the context the relay had
/// and the source did not.
/// Pitfall: a report keyed on a name is only keyed on it for the lines that
/// carry one — check the refusal path, not just the results table.
///
/// # What the Derive List Costs Somebody Else
///
/// Three variants wrap another crate's error, so every trait below is a
/// conjunction across four crates. `Copy` is the load-bearing one: nothing here
/// needs it — `RunError` is returned by value, collected into
/// `Comparison::refusals` and read back through `&[ RunError ]` — but it is
/// written, so `BuildError`, `ring_types::RingError` and
/// `ring_flush::ConfigError` are all pinned `Copy` by this line.
///
/// `RingError` is `#[ non_exhaustive ]` — an explicit reservation that variants
/// may be added. A variant carrying a `String` would be an unremarkable
/// addition there and
/// would break this derive. Fix(BN26): `both_halves_of_the_copy_coupling_are_named`
/// pins the conjunction so the break lands on a named test rather than on a
/// derive expansion, and `ring_types::RingError`'s own docs now name this crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunError {
    /// The candidate cannot be driven by this many producers.
    ProducerCeiling {
        /// Which candidate refused.
        candidate: Candidate,
        /// How many producers the workload asked for.
        requested: usize,
        /// How many it admits.
        ceiling: usize,
    },
    /// `ring_factory` refused the configuration.
    Build {
        /// Which candidate was being built.
        candidate: Candidate,
        /// What `ring_factory` said.
        error: BuildError,
    },
    /// `ring_core` refused the configuration, on a path that does not go through
    /// the factory.
    Ring {
        /// Which candidate was being built.
        candidate: Candidate,
        /// What `ring_core` said.
        error: RingError,
    },
    /// `ring_flush` refused the flush policy.
    Flush {
        /// Which candidate was being built.
        candidate: Candidate,
        /// What `ring_flush` said.
        error: ConfigError,
    },
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProducerCeiling { candidate, requested, ceiling } => {
                write!(
                    f,
                    "{} admits {ceiling} producer(s), asked for {requested}",
                    candidate.name()
                )
            },
            Self::Build { candidate, error } => write!(f, "{}: {error}", candidate.name()),
            Self::Ring { candidate, error } => write!(f, "{}: {error}", candidate.name()),
            Self::Flush { candidate, error } => write!(f, "{}: {error}", candidate.name()),
        }
    }
}

impl core::error::Error for RunError {}

// ── Running one candidate ─────────────────────────────────────────────────

/// Run one candidate against one workload.
///
/// # Errors
///
/// [`RunError::ProducerCeiling`] when the candidate cannot be driven by the
/// workload's producer count — checked before anything is built, so a refusal
/// costs no allocation. The other three variants relay a dependency's own
/// refusal of the configuration.
pub fn run(candidate: Candidate, workload: &Workload) -> Result<Outcome, RunError> {
    let exceeded = candidate.producer_ceiling().filter(|ceiling| workload.producers() > *ceiling);
    if let Some(ceiling) = exceeded {
        return Err(RunError::ProducerCeiling {
            candidate,
            requested: workload.producers(),
            ceiling,
        });
    }

    let (reported, drained, write_nanos) = match candidate {
        Candidate::MutexQueue => run_mutex_queue(workload),
        Candidate::ContractRing => run_contract_ring(workload)?,
        Candidate::TlsOverRing => run_tls_over_ring(workload)?,
        Candidate::DirectSpsc => run_direct_spsc(workload),
        Candidate::DirectMpsc => run_direct_mpsc(workload),
        #[cfg(feature = "crossbeam")]
        Candidate::OffTheShelf => run_off_the_shelf(workload)?,
    };
    let received = drained.len();

    // One shared fold for every candidate, so the semantics axis is applied
    // uniformly rather than six times — see `AccumulatorTable::apply`. Built
    // from `drained`, the same values `received` is a length of, so a table
    // built here can never disagree with the count above about which records
    // it saw.
    let mut table = AccumulatorTable::new(workload.cells());
    for record in &drained {
        let (cell, delta) = destination_of(workload, *record);
        table.apply(workload.semantics(), cell, delta);
    }

    let offered = workload.offered();
    let stats = RingStats::new();
    // Every counter is the *drained* count, never the reported one. A record that
    // came back out claimed exactly one slot and published it; a record that did
    // not claimed none — a `DropNewest` discard never reaches a slot at all. That
    // mapping is what keeps `in_flight` at zero here, so a nonzero reading would
    // mean what `ring_stats` says it means (a slot taken and abandoned) rather
    // than "the workload offered more than the ring could hold", which is not a
    // leak and is already reported as `dropped`.
    //
    // Fix(BN11): that zero is *structural*, not measured. `in_flight` is
    // `claimed - published` and both are this same expression, so no run of this
    // harness can make it nonzero — the two assertions on it in the suite pin the
    // mapping's consequence and cannot fail on their own. What can fail, and is
    // what actually guards the mapping, is the expression-level check in
    // `a_dropnewest_ring_reports_successes_it_did_not_keep`, which runs the one
    // candidate where `reported` and `received` differ and asserts all three
    // counters against `received`.
    //
    // Root cause: a derived reading whose two inputs are the same expression is a
    // constant, and an assertion on a constant reads exactly like a measurement.
    // Pitfall: to guard a mapping, assert what feeds each counter on a fixture
    // where the candidate expressions disagree — never the derived value alone.
    stats.record_claim(received as u64);
    stats.record_publish(received as u64);
    stats.record_consume(received as u64);
    // Fix(BN53): this feeds `record_drop` from the same `offered - received`
    // subtraction as `Outcome::dropped` (-> BN22), but runs before `Outcome`
    // exists — BN22's guard sits on the two accessor methods that expose the
    // subtraction afterward, and cannot cover a computation that happens
    // earlier on the same values. Unguarded, a violation here would silently
    // wrap to a near-`u64::MAX` stat instead of panicking, exactly the failure
    // mode BN22 named and fixed one call site over.
    //
    // Root cause: the guard was added to the two accessors that expose the
    // subtraction, not to the earlier internal computation that performs the
    // identical subtraction first.
    // Pitfall: guarding a derived accessor does not guard every computation
    // that shares its expression — each call site needs its own assertion.
    assert!(received <= offered, "received exceeded offered");
    stats.record_drop(workload.config().overflow(), (offered - received) as u64);

    Ok(Outcome {
        candidate,
        producers: workload.producers(),
        offered,
        reported,
        received,
        write_nanos,
        stats,
        table,
    })
}

/// One producer's share of the mutex candidate: stage locally, take the lock
/// once per batch.
///
/// Amortising the lock over a batch is what makes the baseline a fair one. A
/// mutex taken per record would lose to anything, and the comparison would
/// establish nothing that was in doubt.
fn commit_batch(
    queue: &Mutex<VecDeque<Record>>,
    staged: &mut Vec<Record>,
    capacity: usize,
) -> usize {
    if staged.is_empty() {
        return 0;
    }

    let mut taken = 0;
    // Fix(mutex_queue_poison_recovery): `commit_batch` runs on every producer
    // thread's every batch flush against the one `queue` Mutex `run_mutex_queue`
    // shares across `workload.producers()` spawned threads for the whole
    // benchmark run — a panic inside any single call's critical section (an
    // allocator failure inside `push_back`, say) poisoned the lock forever
    // after, turning one rare, unrelated panic into every other producer's next
    // `commit_batch` call crashing the whole run. No caller-supplied code runs
    // while `guard` is held — just `VecDeque::len`/`push_back` on a plain
    // `Record` ( `u64` ) — so there is no half-established invariant for
    // poisoning to protect, the same reasoning `ring_trace::Trace::entries_guard`
    // already relies on for its own shared log; recovering the stale-but-valid
    // guard and continuing is strictly better than aborting the whole run.
    // Pitfall: this is benchmark harness code, not a service, but N spawned
    // threads hammering one shared Mutex for the run's whole duration is
    // exactly the shared-lock shape poisoning targets — "it's just a harness"
    // is not a reason to skip the same scrutiny production code gets.
    let mut guard = queue.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    for record in staged.drain(..) {
        if guard.len() < capacity {
            guard.push_back(record);
            taken += 1;
        }
    }

    taken
}

fn run_mutex_queue(workload: &Workload) -> (usize, Vec<Record>, u128) {
    let capacity = workload.capacity();
    let queue = Mutex::new(VecDeque::<Record>::with_capacity(capacity));
    let reported = AtomicUsize::new(0);

    let started = Instant::now();
    std::thread::scope(|scope| {
        for index in 0..workload.producers() {
            let queue = &queue;
            let reported = &reported;
            scope.spawn(move || {
                let mut staged = Vec::with_capacity(workload.batch());
                let mut taken = 0;
                for record in workload.records_of(index) {
                    staged.push(record);
                    if staged.len() == workload.batch() {
                        taken += commit_batch(queue, &mut staged, capacity);
                    }
                }
                taken += commit_batch(queue, &mut staged, capacity);
                reported.fetch_add(taken, Ordering::Relaxed);
            });
        }
    });
    let write_nanos = started.elapsed().as_nanos();

    let drained: Vec<Record> =
        queue.into_inner().expect("no producer panics while holding the lock").drain(..).collect();
    (reported.into_inner(), drained, write_nanos)
}

fn run_contract_ring(workload: &Workload) -> Result<(usize, Vec<Record>, u128), RunError> {
    let mut split = Factory
        .build::<Record>(workload.config())
        .map_err(|error| RunError::Build { candidate: Candidate::ContractRing, error })?;
    let mut ends = split.ends();
    let (mut producer, mut consumer) = ends.split();

    let started = Instant::now();
    let mut reported = 0;
    let mut next = 0;
    while next < workload.records_per_producer() {
        let end = usize::min(next + workload.batch(), workload.records_per_producer());
        let mut records = next as Record..end as Record;
        reported += producer.try_push_batch(&mut records);
        next = end;
    }
    let write_nanos = started.elapsed().as_nanos();

    let mut sink = Vec::new();
    let mut drained = Vec::new();
    loop {
        sink.clear();
        let taken = consumer.try_recv_batch(&mut sink);
        if taken == 0 {
            break;
        }
        drained.extend_from_slice(&sink);
    }

    Ok((reported, drained, write_nanos))
}

fn run_tls_over_ring(workload: &Workload) -> Result<(usize, Vec<Record>, u128), RunError> {
    let mut ring: ring_core::Ring<Record> = ring_core::Ring::new(&workload.config())
        .map_err(|error| RunError::Ring { candidate: Candidate::TlsOverRing, error })?;
    let mut ends = ring.ends();
    let (producer, mut consumer) = ends.split();

    let buffer = TlsBuffer::<Record>::with_capacity(workload.batch());
    let mut flusher = Flusher::new(buffer, producer, FlushPolicy::OnBatch(workload.batch()))
        .map_err(|error| RunError::Flush { candidate: Candidate::TlsOverRing, error })?;

    let started = Instant::now();
    let mut reported = 0;
    for record in workload.records_of(0) {
        // An `append` refusal means a previous flush was `Rejected` and its records
        // are still staged. `ring_flush` documents that a rejection must be
        // retried and that it will not retry on the caller's behalf; not retrying
        // here is deliberate, because a harness that retries measures its own retry
        // loop.
        if flusher.append(record).is_err() {
            continue;
        }
        if let FlushOutcome::Flushed { count } = flusher.drive() {
            reported += count;
        }
    }
    if let FlushOutcome::Flushed { count } = flusher.drain_final() {
        reported += count;
    }
    let write_nanos = started.elapsed().as_nanos();

    let mut sink = Vec::new();
    let mut drained = Vec::new();
    loop {
        sink.clear();
        let taken = consumer.try_recv_batch(&mut sink);
        if taken == 0 {
            break;
        }
        drained.extend_from_slice(&sink);
    }

    Ok((reported, drained, write_nanos))
}

fn run_direct_spsc(workload: &Workload) -> (usize, Vec<Record>, u128) {
    let mut ring: ring_spsc::Ring<TypedSlot<Record>> =
        ring_spsc::Ring::with_config(&workload.config());
    let (mut producer, mut consumer) = ring.split();

    let started = Instant::now();
    let mut reported = 0;
    for record in workload.records_of(0) {
        if producer.try_push(record).is_ok() {
            reported += 1;
        }
    }
    let write_nanos = started.elapsed().as_nanos();

    let mut drained = Vec::new();
    loop {
        let batch = consumer.drain();
        if batch.is_empty() {
            break;
        }
        drained.extend(batch.iter().filter_map(TypedSlot::get).copied());
    }

    (reported, drained, write_nanos)
}

fn run_direct_mpsc(workload: &Workload) -> (usize, Vec<Record>, u128) {
    let mut ring: ring_mpsc::Ring<TypedSlot<Record>> =
        ring_mpsc::Ring::with_config(&workload.config());
    let mut ends = ring.ends();
    let (producer, mut consumer) = ends.split();
    let reported = AtomicUsize::new(0);

    let started = Instant::now();
    std::thread::scope(|scope| {
        for index in 0..workload.producers() {
            let reported = &reported;
            scope.spawn(move || {
                let mut taken = 0;
                for record in workload.records_of(index) {
                    if producer.push(record).is_ok() {
                        taken += 1;
                    }
                }
                reported.fetch_add(taken, Ordering::Relaxed);
            });
        }
    });
    let write_nanos = started.elapsed().as_nanos();

    let mut drained = Vec::new();
    loop {
        let batch = consumer.drain();
        if batch.is_empty() {
            break;
        }
        drained.extend(batch.iter().filter_map(TypedSlot::get).copied());
    }

    (reported.into_inner(), drained, write_nanos)
}

#[cfg(feature = "crossbeam")]
fn run_off_the_shelf(workload: &Workload) -> Result<(usize, Vec<Record>, u128), RunError> {
    let mut split = Factory
        .build_crossbeam::<Record>(workload.config())
        .map_err(|error| RunError::Build { candidate: Candidate::OffTheShelf, error })?;
    let mut ends = split.ends();
    let (mut producer, mut consumer) = ends.split();

    let started = Instant::now();
    let mut reported = 0;
    let mut next = 0;
    while next < workload.records_per_producer() {
        let end = usize::min(next + workload.batch(), workload.records_per_producer());
        let mut records = next as Record..end as Record;
        reported += producer.try_push_batch(&mut records);
        next = end;
    }
    let write_nanos = started.elapsed().as_nanos();

    let mut sink = Vec::new();
    let mut drained = Vec::new();
    loop {
        sink.clear();
        let taken = consumer.try_recv_batch(&mut sink);
        if taken == 0 {
            break;
        }
        drained.extend_from_slice(&sink);
    }

    Ok((reported, drained, write_nanos))
}

// ── Comparison ────────────────────────────────────────────────────────────

/// Every candidate, run against one workload.
///
/// The deliverable of feature 186. A candidate the workload's producer count
/// excludes is not silently omitted — it appears in
/// [`refusals`](Self::refusals) with the reason, so a report at four producers
/// says which paths could not be reached rather than showing a shorter table.
#[derive(Debug)]
pub struct Comparison {
    workload: Workload,
    outcomes: Vec<Outcome>,
    refusals: Vec<RunError>,
}

impl Comparison {
    /// Run every candidate in [`Candidate::ALL`], in order.
    #[must_use]
    pub fn run(workload: Workload) -> Self {
        let mut outcomes = Vec::new();
        let mut refusals = Vec::new();

        for candidate in Candidate::ALL {
            match run(*candidate, &workload) {
                Ok(outcome) => outcomes.push(outcome),
                Err(refusal) => refusals.push(refusal),
            }
        }

        Self { workload, outcomes, refusals }
    }

    /// The workload every outcome was produced under.
    #[must_use]
    pub const fn workload(&self) -> &Workload {
        &self.workload
    }

    /// The candidates that ran.
    #[must_use]
    pub fn outcomes(&self) -> &[Outcome] {
        &self.outcomes
    }

    /// The candidates that did not, and why.
    #[must_use]
    pub fn refusals(&self) -> &[RunError] {
        &self.refusals
    }

    /// Whether every candidate that ran handed its refusals back to the caller.
    ///
    /// `false` means at least one path absorbed a record it reported taking —
    /// which is `OverflowPolicy::DropNewest` working as specified, not a defect.
    /// It is worth surfacing at the comparison level because a mixed run, where
    /// some candidates report their drops and others do not, is the one where
    /// reading `reported` instead of `received` would produce a ranking that is
    /// wrong rather than merely imprecise.
    #[must_use]
    pub fn conserved(&self) -> bool {
        self.outcomes.iter().all(Outcome::conserved)
    }

    /// How many records the whole comparison could not account for.
    ///
    /// The sum of every candidate's [`Outcome::silently_discarded`]. Zero under
    /// `OverflowPolicy::Fail`; the size of the trap under `DropNewest`.
    #[must_use]
    pub fn silently_discarded(&self) -> usize {
        self.outcomes.iter().map(Outcome::silently_discarded).sum()
    }

    /// The fastest candidate **among those that lost nothing**.
    ///
    /// `None` when no candidate kept the whole workload — which is the honest
    /// answer under a capacity smaller than the offered load, because the
    /// quickest way to finish a write phase is to refuse every record.
    ///
    /// ```
    /// use ring_bench::{Comparison, Workload};
    /// use ring_factory::RingConfig;
    ///
    /// // 16 slots, 1024 records offered: every path drops, so none is eligible.
    /// let cramped = Workload::new(RingConfig::new(16).unwrap());
    /// assert!(Comparison::run(cramped).fastest().is_none());
    /// ```
    ///
    /// # Ties
    ///
    /// Fix(BN50): a tie goes to whichever candidate appears **first** in
    /// [`Candidate::ALL`], because `min_by_key` returns the first minimum and the
    /// iteration follows that list. `MutexQueue` is declared first, so on a clock
    /// too coarse to separate two paths the control wins by default — the least
    /// interesting outcome the harness can report, and the one it reports on a
    /// tie. Nothing here detects that a tie happened; a caller that needs to know
    /// must compare `write_nanos()` across the eligible outcomes itself.
    ///
    /// Root cause: `Candidate::ALL`'s declaration order silently doubles as this
    /// method's tie-break rule.
    /// Pitfall: `min_by_key` has a documented tie behaviour and inherits its
    /// meaning from the iteration order — reordering the list changes this
    /// verdict with nothing at the call site to say so.
    #[must_use]
    pub fn fastest(&self) -> Option<&Outcome> {
        self.outcomes
            .iter()
            .filter(|outcome| outcome.is_lossless())
            .min_by_key(|outcome| outcome.write_nanos)
    }

    /// A plain-text table of the run.
    #[must_use]
    pub fn report(&self) -> String {
        use fmt::Write as _;

        let mut out = String::new();
        let _ = writeln!(
            out,
            "{} producer(s) x {} records, batch {}, capacity {}, overflow {:?}",
            self.workload.producers(),
            self.workload.records_per_producer(),
            self.workload.batch(),
            self.workload.capacity(),
            self.workload.config().overflow(),
        );
        let _ = writeln!(
            out,
            "{:<16} {:>9} {:>9} {:>9} {:>8} {:>8} {:>12}",
            "candidate", "offered", "reported", "received", "dropped", "silent", "write ns",
        );

        for outcome in &self.outcomes {
            let _ = writeln!(
                out,
                "{:<16} {:>9} {:>9} {:>9} {:>8} {:>8} {:>12}",
                outcome.candidate.name(),
                outcome.offered(),
                outcome.reported(),
                outcome.received(),
                outcome.dropped(),
                outcome.silently_discarded(),
                outcome.write_nanos(),
            );
        }

        for refusal in &self.refusals {
            let _ = writeln!(out, "refused: {refusal}");
        }

        match self.fastest() {
            Some(outcome) => {
                let _ = writeln!(out, "fastest lossless: {}", outcome.candidate.name());
            },
            None => {
                let _ = writeln!(out, "fastest lossless: none — every candidate dropped records");
            },
        }

        out
    }
}
