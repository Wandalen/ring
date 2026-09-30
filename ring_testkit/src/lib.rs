//! Determinism-test fixtures driving scripted claim and drain sequences.
//!
//! One of the ring family's 33 crates — the concurrency write-path implementation.
//!
//! # What it is
//!
//! A [`Script`] is a list of [`Step`]s. Running it against a ring produces an
//! [`Outcome`], and the same script against an equivalent ring produces an
//! equal `Outcome` every time. That is the whole fixture: a concurrency test
//! that can be re-run and compared rather than watched.
//!
//! ```
//! use ring_config::RingConfig;
//! use ring_core::Ring;
//! use ring_testkit::{Script, Step};
//!
//! let script = Script::new(4) // staging capacity — unused: no Stage/StageMany/Flush step below
//!     .then(Step::PushMany(3))
//!     .then(Step::RecvMany(3));
//!
//! let config = RingConfig::new(8).unwrap(); // ring capacity — the one that governs this run
//! let mut first: Ring<u32> = Ring::new(&config).unwrap();
//! let mut again: Ring<u32> = Ring::new(&config).unwrap();
//!
//! assert_eq!(script.run(&mut first), script.run(&mut again));
//! assert_eq!(script.run(&mut first).received, [0, 1, 2]);
//! ```
//!
//! # The measurement it exists for
//!
//! `ring_core`'s `try_push` reports `Ok` on a full ring under
//! `OverflowPolicy::DropNewest` — the record is discarded inside the call and
//! the caller is told it succeeded. Run one script over two rings differing
//! only in that policy, capacity 4, eight records pushed and everything
//! drained:
//!
//! ```text
//! Fail:       accepted=4  refused_full=4  received=[0,1,2,3]  vanished=0
//! DropNewest: accepted=8  refused_full=0  received=[0,1,2,3]  vanished=4
//! ```
//!
//! **Neither half of that is sufficient on its own, and the half that fails is
//! the one you would reach for.** The delivered records are *identical* — a
//! fixture that recorded only what came out would call the two rings
//! equivalent, because both delivered exactly `[0,1,2,3]`. The counts do
//! separate them, but they separate them into `accepted=4` and `accepted=8`,
//! which reads as "one ring took twice as much work" rather than as "one ring
//! destroyed half of it".
//!
//! [`Outcome::vanished`] is the two together: accepted, minus delivered, minus
//! still held. Four records went in, were reported accepted, and are nowhere.
//! → `docs/pitfall/001_neither_the_count_nor_the_list_alone.md`.
//!
//! # What it does not do
//!
//! It does not contain a model checker. `loom` is the family's model checker
//! and it is already a `cfg(loom)` dev-dependency of `ring_atomic`,
//! `ring_publish`, `ring_spsc` and `ring_mpsc`. What this crate contributes to
//! that half of feature 188 is [`leak`], which is the one line every loom test
//! over a borrow-based ring has to write, and [`audit_received`], which is the
//! assertion worth making inside `loom::model`.
//! → `docs/integration/001_the_three_edges_and_the_one_that_is_missing.md`.
//!
//! # Under `--cfg loom`
//!
//! This crate declares no `cfg` of its own, and that is a decision rather than
//! an omission. `ring_atomic` owns the family's only `--cfg loom` switch and
//! states that "no other crate needs to know the seam exists"; a `cfg`
//! attribute here would be a fifth place to keep that switch in sync. The
//! consequence is inherited all the same: the seam arrives through `ring_core`
//! → `ring_mpsc`/`ring_spsc` → `ring_atomic`, four crates down, and under the
//! cfg loom's atomics panic when touched outside a `loom::model` — so
//! [`Script::run`] panics on a ring built anywhere else, from a crate that
//! mentions neither loom nor the cfg.
//! → `docs/non_functional_requirement/002_what_a_fixture_owes_its_consumers.md`
//!   TK36.
//!
//! The doc examples in this crate are the visible edge of that. Three of the
//! five construct a `ring_core::Ring` outside a model, so under the cfg they
//! would panic — and a doc example cannot carry `#![ cfg( not( loom ) ) ]`,
//! which is how the 24 test files in the family facing the same problem opt
//! out.
//! Nothing checks it either: the one stage that sets the cfg runs clippy, and
//! clippy does not run doctests.
//! → `docs/decisions/002_the_model_lives_in_tests.md` TK16.

#![deny(missing_docs)]

use ring_core::{Consumer, Ends, Producer, Ring};
use ring_shutdown::{Refusal, Shutdown};
use ring_tls::TlsBuffer;

/// One operation in a [`Script`].
///
/// Every variant that creates a record *mints* it: records are consecutive
/// `u32`s from `0`, so a script needs no input data and two runs of it mint the
/// same values. That is what makes an [`Outcome`] comparable between runs
/// rather than merely similar.
///
/// # The `Many` counts carry no ceiling
///
/// `PushMany`, `RecvMany` and `StageMany` hold a bare `usize`, and
/// [`Script::run`] applies no `min`, clamp or assertion to it. The termination
/// argument in `docs/algorithm/` calls each step "bounded by a constant in the
/// step itself"; that constant is whichever number the caller wrote, so
/// `PushMany( usize::MAX )` mints for as long as the process lives. Only
/// `RecvMany` stops early, and only because an empty ring ends its loop — the
/// other two run their count out against a ring that refuses every record.
/// → `docs/data_structure/001_the_script_as_a_flat_step_list.md` TK10.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Mint one record and offer it to the ring.
    Push,
    /// Mint `n` records and offer each to the ring, in mint order.
    PushMany(usize),
    /// Take one record from the ring, if there is one.
    Recv,
    /// Take up to `n` records from the ring.
    RecvMany(usize),
    /// Mint one record into the thread-local staging buffer.
    Stage,
    /// Mint `n` records into the staging buffer, stopping at nothing — a full
    /// buffer refuses each remaining record individually.
    StageMany(usize),
    /// Offer everything staged to the ring, one record at a time, emptying the
    /// buffer whether or not the ring takes them.
    Flush,
    /// Close the ring to further publication.
    Close,
    /// Open it again.
    ///
    /// **This closes first.** `Stopped::reopen` consumes the token that proves
    /// the ring closed, and `Shutdown::close` is the only thing that produces
    /// one — so reopening an *open* ring closes and reopens it. Invisible in a
    /// single-threaded script; a refusal window in a concurrent one.
    /// → `docs/pitfall/002_reopening_closes_first.md`.
    Reopen,
    /// Close, then take every record still in the ring.
    DrainAll,
}

/// What running a [`Script`] produced.
///
/// Every field is a count or a list, and none of them is a timing — two runs of
/// the same script on equivalent rings compare equal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// How many records the script created.
    pub minted: u32,
    /// How many offers the ring answered with `Ok`.
    ///
    /// **Not the same as how many it stored.** See [`Outcome::vanished`].
    pub accepted: usize,
    /// Offers refused because the ring was full.
    pub refused_full: usize,
    /// Offers refused because the ring was closed.
    pub refused_closed: usize,
    /// Records refused by a full staging buffer, which never reached the ring.
    pub refused_staging: usize,
    /// The records that came back out, in the order they came out.
    pub received: Vec<u32>,
    /// Records the ring accepted, in the order they were actually offered to
    /// it — not the order they were minted in.
    ///
    /// The two orders coincide for a `Push`-only script, which is why every
    /// prior test never needed this field to distinguish them. They diverge
    /// the moment a `Push`/`PushMany` lands between a `Stage`/`StageMany` and
    /// the `Flush` that empties it: the staged record keeps the lower value it
    /// was minted with, but does not reach the ring until `Flush` runs, after
    /// the interceding push already landed. [`Outcome::audit`] checks
    /// `received` against this list rather than against raw ascending mint
    /// values, which is the reading a mixed script needs.
    pub published: Vec<u32>,
    /// Records still in the ring when the script ended.
    pub in_ring_at_end: usize,
    /// Records still in the staging buffer when the script ended.
    pub staged_at_end: usize,
    /// Whether the ring was closed when the script ended.
    pub closed_at_end: bool,
}

impl Outcome {
    /// Records the ring accepted and neither returned nor still holds.
    ///
    /// Zero under `OverflowPolicy::Fail`, where a full ring refuses. Non-zero
    /// under `DropNewest`, where a full ring answers `Ok` and discards.
    ///
    /// This is the reading that says *records were destroyed*. `accepted` alone
    /// says the opposite — `DropNewest` accepts strictly more — and `received`
    /// alone says nothing at all, because both policies deliver the same records
    /// in the same order. Measured, and the measurement contradicted the
    /// prediction that produced this method: `tests/manual/readme.md` M1.
    ///
    /// # `0` Is Two Different Answers
    ///
    /// The subtraction saturates, and delivered-plus-held can exceed `accepted`
    /// two established ways: one script run twice against one ring, and an
    /// `Outcome` built by hand. Both clamp to `0` — the healthy reading — for a
    /// state no single run can produce. [`Outcome::audit`] is what separates
    /// them: it answers [`Anomaly::Overdelivered`] for exactly that case, so
    /// `vanished() == 0` means "nothing was destroyed" only for an `Outcome`
    /// that audits clean.
    /// → `docs/data_structure/002_nine_counters_and_a_number_that_is_two_things.md`
    ///   TK11.
    #[must_use]
    pub fn vanished(&self) -> usize {
        self.accepted.saturating_sub(self.received.len() + self.in_ring_at_end)
    }

    /// Check every property a run should hold, and name the first that does not —
    /// first in the fixed pass order the checks run in, not first by position in
    /// the record list.
    ///
    /// # What it does not check
    ///
    /// [`Outcome::vanished`]. A destroyed record is still *placed*: `accepted`
    /// counts the offer the ring answered `Ok` to, whether or not it kept what
    /// was offered, so a `DropNewest` run that discarded half its input audits
    /// clean. That is deliberate — vanishing is what `DropNewest` is for, and an
    /// audit that failed on it could not be run across both policies. It also
    /// means the packaged verdict is not the whole reading: a caller who takes
    /// `audit()` and never calls `vanished()` is told nothing about the records
    /// the ring destroyed.
    /// → `docs/pitfall/001_neither_the_count_nor_the_list_alone.md` TK41.
    ///
    /// # Errors
    ///
    /// [`Anomaly::Unaccounted`] if a minted record is in none of the four places
    /// a record can be, [`Anomaly::Overdelivered`] if more records left the ring
    /// than it ever accepted; otherwise [`Anomaly::Unminted`] for a delivered
    /// record this run never minted, or [`Anomaly::OutOfOrder`] for one that did
    /// not follow [`Outcome::published`]'s own order.
    // Fix(audit_assumed_mint_order_is_push_order): was
    //   `audit_received( &self.received, self.minted )`, which reports
    //   `Anomaly::OutOfOrder` whenever `received` fails to ascend by raw mint
    //   value — correct only when push order equals mint order. A script that
    //   lets a `Push`/`PushMany` land between a `Stage`/`StageMany` and the
    //   `Flush` that empties it breaks that equality on purpose: the staged
    //   record keeps its lower mint value but reaches the ring only when
    //   `Flush` runs, after the interceding push already landed. A perfectly
    //   correct FIFO delivery then came back reporting an anomaly for a run
    //   that did nothing wrong.
    // Root cause: `audit_received` was written for, and only ever exercised
    //   by, scripts where minting and publishing happen in the same
    //   statement. Every existing test either pushed directly or staged then
    //   flushed with no push in between, so mint order and push order always
    //   coincided by construction and the two were never distinguished in the
    //   data `Outcome` carried.
    // Pitfall: do not restore a direct call to
    //   `audit_received( &self.received, self.minted )` here — it silently
    //   reintroduces the false `OutOfOrder` the moment a script interleaves a
    //   `Push`/`PushMany` with a pending `Stage`/`Flush`. `audit_received`
    //   itself is unchanged and stays correct for its own documented callers
    //   (a `Push`-only script, the loom model's direct `try_push`), where mint
    //   order and push order are the same thing by construction — this fix
    //   only changes what `Outcome::audit` checks `received` against.
    pub fn audit(&self) -> Result<(), Anomaly> {
        let placed = self.accepted
            + self.refused_full
            + self.refused_closed
            + self.refused_staging
            + self.staged_at_end;
        if placed != self.minted as usize {
            return Err(Anomaly::Unaccounted { minted: self.minted as usize, placed });
        }

        let out = self.received.len() + self.in_ring_at_end;
        if out > self.accepted {
            return Err(Anomaly::Overdelivered { accepted: self.accepted, out });
        }

        audit_delivery_order(&self.received, self.minted, &self.published)
    }
}

/// A property of a run that did not hold.
///
/// # Not Exhaustive
///
/// A run has more properties than this enum has variants, and the fourth
/// arrived after the first three: `Overdelivered` came from
/// `docs/data_structure/002` TK11, which found that [`Outcome::vanished`]
/// answers `0` — the healthy reading — for the one state that makes it
/// meaningless. Adding it was a breaking change to any exhaustive `match`,
/// which is the cost `docs/item/002` TK27 predicted while this enum had no
/// room attribute. The attribute is here so the next one is additive.
/// → `docs/item/002_what_the_crate_does_not_declare.md` TK27.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anomaly {
    /// A minted record was neither accepted, nor refused, nor left staged.
    Unaccounted {
        /// How many records the script created.
        minted: usize,
        /// How many were found somewhere.
        placed: usize,
    },
    /// A record came out that was never minted.
    Unminted {
        /// The value that came out.
        value: u32,
        /// How many records existed to come out.
        minted: u32,
    },
    /// More records left the ring than the ring ever accepted.
    ///
    /// Impossible within one run under every policy: a record cannot be
    /// delivered or still held without first having been accepted. Reaching it
    /// means the `Outcome` describes two runs rather than one — a script run
    /// twice against a single ring — or was built by hand, and it is exactly the
    /// state in which [`Outcome::vanished`]'s saturating subtraction answers `0`.
    Overdelivered {
        /// How many offers the ring answered with `Ok`.
        accepted: usize,
        /// How many records came back out, plus how many the ring still held.
        out: usize,
    },
    /// Two records came out in the wrong order, or the same one came out twice.
    ///
    /// One variant for both, because the check is `then > previous` — a
    /// duplicate is the equality case of an ordering failure, and splitting them
    /// would mean two checks where the ring only ever breaks one property.
    OutOfOrder {
        /// The earlier record.
        previous: u32,
        /// The record that did not follow it.
        then: u32,
    },
}

impl core::fmt::Display for Anomaly {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Unaccounted { minted, placed } => {
                write!(f, "{minted} records were minted but only {placed} are accounted for")
            },
            Self::Unminted { value, minted } => {
                write!(f, "record {value} came out of a run that minted only {minted}")
            },
            Self::Overdelivered { accepted, out } => {
                write!(f, "{out} records left a ring that accepted only {accepted}")
            },
            Self::OutOfOrder { previous, then } => {
                write!(f, "record {then} came out after {previous}")
            },
        }
    }
}

impl core::error::Error for Anomaly {}

/// Check `received` against the order records were actually offered to the
/// ring, rather than against raw ascending mint values.
///
/// [`audit_received`] assumes mint order and push order coincide, which holds
/// for a `Push`-only script but not for one where a `Push`/`PushMany` lands
/// between a `Stage`/`StageMany` and the `Flush` that empties it — see
/// [`Outcome::published`]. This walks `received` against `published` instead
/// of against raw values, so a record is judged by where it was actually
/// offered rather than by the number stamped on it at mint time. R2
/// (provenance) is unchanged from [`audit_received`]; only R3 (ascent) is
/// re-based onto `published`.
fn audit_delivery_order(received: &[u32], minted: u32, published: &[u32]) -> Result<(), Anomaly> {
    for &value in received {
        if value >= minted {
            return Err(Anomaly::Unminted { value, minted });
        }
    }

    for pair in received.windows(2) {
        let (previous, then) = (pair[0], pair[1]);
        let earlier = published.iter().position(|&candidate| candidate == previous);
        let later = published.iter().position(|&candidate| candidate == then);

        let advances =
            matches!( ( earlier, later ), ( Some( earlier ), Some( later ) ) if later > earlier );
        if !advances {
            return Err(Anomaly::OutOfOrder { previous, then });
        }
    }

    Ok(())
}

/// Check a list of received records against the number minted.
///
/// Separate from [`Outcome::audit`] because the list a concurrent test collects
/// does not come from an `Outcome` — under `loom::model` there is no single
/// script and no accounting, only the records a consumer thread saw.
///
/// # Single-producer
///
/// The ordering pass requires each record to be strictly greater than the one
/// before it, and that is a property of one producer minting alone. Two
/// producers minting independently interleave in claim order, so
/// `[ 0, 100, 1 ]` is a correct delivery that this function reports as
/// [`Anomaly::OutOfOrder`] — inside the one function whose reason for existing
/// is the concurrent case. Use [`audit_received_unordered`] for a model with
/// more than one producer; it keeps the two checks a ring can actually fail
/// and drops the one only a single producer guarantees.
/// → `docs/algorithm/002_the_four_passes_of_an_audit.md` TK3.
///
/// # Errors
///
/// [`Anomaly::Unminted`] for a value no producer could have created,
/// [`Anomaly::OutOfOrder`] for a pair that did not ascend — which covers
/// duplicates, since a repeat is the equality case.
///
/// ```
/// use ring_testkit::{Anomaly, audit_received};
///
/// assert_eq!(audit_received(&[0, 1, 2], 3), Ok(()));
/// assert_eq!(audit_received(&[0, 0], 3), Err(Anomaly::OutOfOrder { previous: 0, then: 0 }),);
/// ```
pub fn audit_received(received: &[u32], minted: u32) -> Result<(), Anomaly> {
    for &value in received {
        if value >= minted {
            return Err(Anomaly::Unminted { value, minted });
        }
    }

    for pair in received.windows(2) {
        let (previous, then) = (pair[0], pair[1]);
        if then <= previous {
            return Err(Anomaly::OutOfOrder { previous, then });
        }
    }

    Ok(())
}

/// Check a list of received records without requiring them to ascend.
///
/// The multi-producer form of [`audit_received`]. Two producers minting
/// independently interleave in claim order, so ascent is not a property of a
/// correct delivery — but *nothing unminted* and *nothing twice* still are,
/// and those are the two failures a ring can actually produce. Sorting a copy
/// rather than tracking a set keeps the check allocation-proportional to the
/// list a consumer thread already collected.
///
/// # Errors
///
/// [`Anomaly::Unminted`] for a value no producer could have created, and
/// [`Anomaly::OutOfOrder`] with `previous == then` for a record delivered
/// twice — the same variant [`audit_received`] uses for its equality case,
/// because a duplicate is a duplicate whichever pass finds it.
///
/// ```
/// use ring_testkit::{Anomaly, audit_received, audit_received_unordered};
///
/// // Two producers interleaving: a correct delivery the ordered form rejects.
/// assert_eq!(audit_received_unordered(&[0, 100, 1], 200), Ok(()));
/// assert_eq!(
///     audit_received(&[0, 100, 1], 200),
///     Err(Anomaly::OutOfOrder { previous: 100, then: 1 }),
/// );
///
/// // A duplicate is an anomaly in both.
/// assert_eq!(
///     audit_received_unordered(&[7, 7], 8),
///     Err(Anomaly::OutOfOrder { previous: 7, then: 7 }),
/// );
/// ```
pub fn audit_received_unordered(received: &[u32], minted: u32) -> Result<(), Anomaly> {
    for &value in received {
        if value >= minted {
            return Err(Anomaly::Unminted { value, minted });
        }
    }

    let mut sorted = received.to_vec();
    sorted.sort_unstable();
    for pair in sorted.windows(2) {
        if pair[0] == pair[1] {
            return Err(Anomaly::OutOfOrder { previous: pair[0], then: pair[1] });
        }
    }

    Ok(())
}

/// Give a ring a `'static` lifetime by leaking it.
///
/// `loom::thread::spawn` takes `'static` closures and loom has no scoped
/// threads, so the two ends of a ring cannot be moved onto loom threads while
/// the ring is a local. Leaking one ring per model execution is the least
/// contrived way to get the lifetime: a loom model runs a deliberately tiny
/// ring, and loom's own per-execution bookkeeping dwarfs it.
///
/// **Only for tests, and only for tiny rings.** Nothing frees this. The
/// crate is a testkit and the leak is the reason it can exist at all, but a
/// production caller reaching for it has misread the name.
///
/// ```
/// use ring_config::RingConfig;
/// use ring_core::Ring;
///
/// let ring: Ring<u32> = Ring::new(&RingConfig::new(2).unwrap()).unwrap();
/// let leaked = ring_testkit::leak(ring);
/// assert_eq!(leaked.capacity().get(), 2);
/// ```
#[must_use = "nothing frees this — dropping the reference leaks the ring with no way to reach it again"]
pub fn leak<T: Send>(ring: Ring<T>) -> &'static mut Ring<T> {
    Box::leak(Box::new(ring))
}

/// The two ends of a ring, both `'static`.
///
/// [`leak`] alone is not enough to reach a loom thread: `Ring::ends` returns an
/// [`Ends`] that `split` then borrows, so a `Producer` from a leaked ring is
/// still bounded by whatever local the `Ends` was bound to. **Two leaks are
/// needed, not one**, and forgetting the second produces a borrow error whose
/// message points at the local rather than at the missing leak.
///
/// **This, not [`leak`], is what every loom model in this crate actually
/// calls.** Each call strands three heap blocks, not one: the boxed `Ring`,
/// the `Storage` it owns, and the boxed `Ends` this function leaks on top —
/// [`leak`]'s own cost sentence ("one ring per model execution") undercounts
/// by describing a function no loom test in this crate invokes.
///
/// Same caveat as [`leak`], twice over: nothing frees either allocation.
///
/// ```
/// use ring_config::RingConfig;
/// use ring_core::Ring;
///
/// let ring: Ring<u32> = Ring::new(&RingConfig::new(2).unwrap()).unwrap();
/// let (mut producer, mut consumer) = ring_testkit::leak_ends(ring);
///
/// // Both ends outlive every scope in this example, which is what a
/// // `loom::thread::spawn` closure requires and a borrowed local cannot give.
/// std::thread::spawn(move || producer.try_push(1)).join().unwrap().unwrap();
/// assert_eq!(std::thread::spawn(move || consumer.try_recv()).join().unwrap(), Some(1));
/// ```
#[must_use = "nothing frees either allocation — dropping the pair leaks both with no way to reach them again"]
pub fn leak_ends<T: Send>(ring: Ring<T>) -> (Producer<'static, T>, Consumer<'static, T>) {
    let ends: &'static mut Ends<'static, T> = Box::leak(Box::new(leak(ring).ends()));
    ends.split()
}

/// A sequence of [`Step`]s, and the staging-buffer size to run them with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Script {
    steps: Vec<Step>,
    stage_limit: usize,
}

impl Script {
    /// An empty script whose staging buffer holds `stage_limit` records.
    ///
    /// The limit is required rather than defaulted: it decides how many
    /// [`Step::Stage`]s are refused, so a hidden default would put a number
    /// nobody chose into the `refused_staging` count.
    #[must_use]
    pub fn new(stage_limit: usize) -> Self {
        Self { steps: Vec::new(), stage_limit }
    }

    /// Append a step.
    #[must_use]
    pub fn then(mut self, step: Step) -> Self {
        self.steps.push(step);
        self
    }

    /// The steps, in order.
    #[must_use]
    pub fn steps(&self) -> &[Step] {
        &self.steps
    }

    /// The staging-buffer size this script runs with.
    #[must_use]
    pub fn stage_limit(&self) -> usize {
        self.stage_limit
    }

    /// Drive `ring` through every step and report what happened.
    ///
    /// The ring is left in whatever state the last step put it in — a caller
    /// comparing two runs must supply two rings, not run twice on one.
    ///
    /// # Backend-blind
    ///
    /// `run` never asks which implementation is under the ring. `Ring::new` and
    /// `Ring::new_crossbeam` produce the same type, and the second accepts
    /// `OverflowPolicy::DropOldest` where the first refuses it — so this
    /// fixture's exclusion of `DropOldest` is not a property of the fixture. It
    /// holds because `new_crossbeam` sits behind `ring_core`'s `crossbeam`
    /// feature, which this crate neither enables nor names in its manifest. A
    /// caller who enables it can hand `run` an evicting ring and get an
    /// `Outcome` shaped like any other.
    /// → `docs/pitfall/001_neither_the_count_nor_the_list_alone.md` TK42.
    #[must_use = "the Outcome is the measurement — `script.run( &mut ring );` as a statement drives the ring and discards everything it observed"]
    #[allow(
        clippy::too_many_lines,
        reason = "seven arms cover ten variants, three collapsed into their Many form; the length comes from the three Many loops and the Flush body, not a one-arm-per-variant shape"
    )]
    pub fn run(&self, ring: &mut Ring<u32>) -> Outcome {
        let mut ends = ring.ends();
        let (producer, mut consumer) = ends.split();

        let shutdown = Shutdown::new();
        let mut guard = shutdown.guard(producer);
        let mut staging: TlsBuffer<u32> = TlsBuffer::with_capacity(self.stage_limit);

        let mut minted: u32 = 0;
        let mut accepted = 0;
        let mut refused_full = 0;
        let mut refused_closed = 0;
        let mut refused_staging = 0;
        let mut received = Vec::new();
        let mut published = Vec::new();

        for step in &self.steps {
            match *step {
                Step::Push | Step::PushMany(_) => {
                    let count = if let Step::PushMany(n) = *step { n } else { 1 };
                    for _ in 0..count {
                        let record = minted;
                        minted += 1;
                        match guard.try_push(record) {
                            Ok(()) => {
                                accepted += 1;
                                published.push(record);
                            },
                            Err(Refusal::Full(_)) => refused_full += 1,
                            Err(Refusal::Closed(_)) => refused_closed += 1,
                        }
                    }
                },

                Step::Recv | Step::RecvMany(_) => {
                    let count = if let Step::RecvMany(n) = *step { n } else { 1 };
                    for _ in 0..count {
                        match consumer.try_recv() {
                            Some(record) => received.push(record),
                            None => break,
                        }
                    }
                },

                Step::Stage | Step::StageMany(_) => {
                    let count = if let Step::StageMany(n) = *step { n } else { 1 };
                    for _ in 0..count {
                        let record = minted;
                        minted += 1;
                        if staging.push(record).is_err() {
                            refused_staging += 1;
                        }
                    }
                },

                Step::Flush => {
                    // `TlsBuffer::flush_into` is the amortised path — one `fetch_add` for
                    // the whole batch — and it cannot be used here: it claims against a
                    // `SeqCell`, and a `ring_core::Ring` exposes no cursor at all. So the
                    // staged records go in one at a time, which is the only join the two
                    // crates have. → `docs/pitfall/003_the_amortised_flush_has_no_ring.md`.
                    //
                    // Collected first because `drain()` borrows `staging` for the loop,
                    // and the guard's push cannot run while that borrow is live.
                    let staged: Vec<u32> = staging.drain().collect();
                    for record in staged {
                        match guard.try_push(record) {
                            Ok(()) => {
                                accepted += 1;
                                published.push(record);
                            },
                            Err(Refusal::Full(_)) => refused_full += 1,
                            Err(Refusal::Closed(_)) => refused_closed += 1,
                        }
                    }
                },

                Step::Close => {
                    let _ = shutdown.close();
                },

                Step::Reopen => shutdown.close().reopen(),

                Step::DrainAll => {
                    let stopped = shutdown.close();
                    stopped.drain_all(&mut consumer, &mut received);
                },
            }
        }

        Outcome {
            minted,
            accepted,
            refused_full,
            refused_closed,
            refused_staging,
            received,
            published,
            in_ring_at_end: consumer.len(),
            staged_at_end: staging.len(),
            closed_at_end: shutdown.is_closed(),
        }
    }
}
