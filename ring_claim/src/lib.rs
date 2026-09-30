//! Sequence-range claiming without waiting.
//!
//! Tier 5 of the ring family's 33 crates — the concurrency write-path implementation.
//! Depends on `ring_types`, `ring_cursor`, `ring_gating`.
//!
//! `ring_seqno` was scaffolded into this crate's manifest before the
//! implementation existed and is not among them: every piece of sequence
//! arithmetic claiming needs is either `ring_types::Seq`'s own (`advanced_by`,
//! and the `Ord` that makes range containment a comparison rather than a
//! subtraction) or already inside `ring_gating::GatingSet::headroom`, which
//! reaches `ring_seqno` on this crate's behalf. Declaring it here as well would
//! be a dependency carried for the shape of the forest rather than for a call.
//!
//! Claiming is the first half of
//! `docs/feature/170_claim_publish_available_commit_handshake.md`: a producer
//! takes exclusive ownership of a range of sequences, writes into the slots
//! they index, and only then publishes. `ring_publish` is the second half.
//!
//! Multi-producer exclusivity is
//! `docs/feature/172_multi_producer_claim.md`'s requirement, and it is the
//! reason this crate exists rather than being two lines inside `ring_publish`:
//! no two producers may ever be granted the same sequence.
//!
//! ## Why claiming never waits
//!
//! Every function here returns immediately — `Ok` with a range, or `Err` saying
//! why not. That is not an incidental design choice; it is what lets the same
//! primitive serve a spinning producer, a parking producer, and the tick path
//! that must not block at all. A claim that waited internally would force the
//! wait strategy into this crate and make `WaitKind::None` unimplementable
//! above it.
//!
//! The caller that wants to wait composes: `ring_wait::for_space`, then
//! [`Claimer::claim`].
//!
//! ## Why a claim is `#[must_use]` and carries no destructor
//!
//! A [`Claim`] is a promise the producer made to itself: these sequences are
//! mine and I will publish them. Dropping one without publishing strands the
//! range — the producer cursor has already advanced past it, so those slots are
//! never written and never reclaimed, and every consumer stalls at the gap
//! forever.
//!
//! There is deliberately no `Drop` impl that "releases" the claim. Releasing is
//! not possible: another producer may already have claimed the range beyond it,
//! so rewinding the cursor would hand out sequences twice, which is the one
//! thing `docs/feature/172` forbids outright. The type is `#[must_use]` so the
//! compiler objects to the common accident, and the invariant is stated here
//! for the uncommon one.
//!
//! ## Why the CAS loop is not `fetch_add`
//!
//! A `fetch_add` claim is shorter and wrong. It advances the cursor
//! unconditionally, so the gating check has to happen *before* it — and between
//! that check and the add, another producer can take the space the check just
//! found. The result is a producer holding a range that overlaps a slot a
//! consumer is still reading, which no later check can undo because the range
//! is already granted.
//!
//! The compare-exchange loop re-reads the gate inside the retry, so the
//! decision to grant and the granting itself are one atomic step.

#![deny(missing_docs)]

use ring_cursor::{GATING, PaddedCursor, SeqCell};
use ring_gating::GatingSet;
use ring_types::{RingError, Seq};

/// The ordering a successful claim publishes the new cursor value at.
///
/// `AcqRel` rather than `Release`: the success case is both a release of the
/// cursor advance to other producers and an acquire of whatever the producer
/// whose value we replaced had done. A bare `Release` would let this producer's
/// slot writes be reordered before it observed the previous producer's claim.
const CLAIM_SUCCESS: core::sync::atomic::Ordering = core::sync::atomic::Ordering::AcqRel;

/// A contiguous range of sequences granted to exactly one producer.
///
/// Half-open: `start..end`, so an empty claim and a one-slot claim are not the
/// same value, and `end` is directly the sequence the producer cursor now sits
/// at.
///
/// ```
/// use ring_claim::Claim;
/// use ring_types::Seq;
///
/// let claim = Claim::new(Seq(4), 3);
/// assert_eq!(claim.start(), Seq(4));
/// assert_eq!(claim.end(), Seq(7));
/// assert_eq!(claim.len(), 3);
/// assert_eq!(claim.sequences().collect::<Vec<_>>(), vec![Seq(4), Seq(5), Seq(6)]);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use = "a claimed range that is never published strands its slots and stalls every consumer"]
pub struct Claim {
    start: Seq,
    len: usize,
}

impl Claim {
    /// A claim of `len` sequences beginning at `start`.
    ///
    /// Public because `ring_publish` and the test suites of both crates need to
    /// construct one directly; a producer obtains real claims from
    /// [`Claimer::claim`], which is the only path that establishes exclusivity.
    ///
    /// ```
    /// use ring_claim::Claim;
    /// use ring_types::Seq;
    /// assert_eq!(Claim::new(Seq::ZERO, 0).len(), 0);
    /// ```
    // No `#[ must_use ]` here: `Claim` itself already carries one *with a
    // message*, and a bare attribute on the constructor would only shadow it
    // with a less informative warning.
    pub const fn new(start: Seq, len: usize) -> Self {
        Self { start, len }
    }

    /// The first sequence in the range.
    ///
    /// ```
    /// use ring_claim::Claim;
    /// use ring_types::Seq;
    /// assert_eq!(Claim::new(Seq(9), 2).start(), Seq(9));
    /// ```
    #[must_use]
    pub const fn start(self) -> Seq {
        self.start
    }

    /// One past the last sequence in the range.
    ///
    /// ```
    /// use ring_claim::Claim;
    /// use ring_types::Seq;
    /// assert_eq!(Claim::new(Seq(9), 2).end(), Seq(11));
    /// ```
    #[must_use]
    pub const fn end(self) -> Seq {
        self.start.advanced_by(self.len as u64)
    }

    /// How many sequences the range covers.
    ///
    /// ```
    /// use ring_claim::Claim;
    /// use ring_types::Seq;
    /// assert_eq!(Claim::new(Seq::ZERO, 5).len(), 5);
    /// ```
    #[must_use]
    pub const fn len(self) -> usize {
        self.len
    }

    /// Whether the range covers nothing.
    ///
    /// ```
    /// use ring_claim::Claim;
    /// use ring_types::Seq;
    /// assert!(Claim::new(Seq(3), 0).is_empty());
    /// ```
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.len == 0
    }

    /// Whether `seq` falls inside the range.
    ///
    /// ```
    /// use ring_claim::Claim;
    /// use ring_types::Seq;
    ///
    /// let claim = Claim::new(Seq(4), 2);
    /// assert!(claim.contains(Seq(4)));
    /// assert!(claim.contains(Seq(5)));
    /// assert!(!claim.contains(Seq(6)), "half-open");
    /// assert!(!claim.contains(Seq(3)));
    /// ```
    #[must_use]
    pub const fn contains(self, seq: Seq) -> bool {
        // Compared as raw `u64` rather than through `Seq`'s operators: `PartialOrd`
        // is not callable in a `const fn`, and reaching through the newtype for two
        // comparisons is the entire cost of having this one at compile time.
        seq.0 >= self.start.0 && seq.0 < self.end().0
    }

    /// Every sequence in the range, in order.
    ///
    /// ```
    /// use ring_claim::Claim;
    /// use ring_types::Seq;
    ///
    /// let seen: Vec<u64> = Claim::new(Seq(2), 3).sequences().map(|s| s.0).collect();
    /// assert_eq!(seen, vec![2, 3, 4]);
    /// ```
    pub fn sequences(self) -> impl Iterator<Item = Seq> {
        (self.start.0..self.end().0).map(Seq)
    }

    /// Whether this range shares any sequence with `other`.
    ///
    /// The property `docs/feature/172_multi_producer_claim.md` forbids across
    /// producers, exposed so a test can assert it directly rather than
    /// reconstructing the comparison at every call site.
    ///
    /// ```
    /// use ring_claim::Claim;
    /// use ring_types::Seq;
    ///
    /// let first = Claim::new(Seq(0), 4);
    /// assert!(!first.overlaps(Claim::new(Seq(4), 4)), "adjacent, not overlapping");
    /// assert!(first.overlaps(Claim::new(Seq(3), 4)));
    /// assert!(!first.overlaps(Claim::new(Seq(0), 0)), "an empty claim covers nothing");
    /// ```
    #[must_use]
    pub const fn overlaps(self, other: Self) -> bool {
        // Raw `u64` comparisons for the same reason `contains` uses them, and with
        // more at stake: this predicate is the one the exclusivity tests assert
        // with, so having it answerable at compile time is worth reaching through
        // the newtype for.
        !self.is_empty()
            && !other.is_empty()
            && self.start.0 < other.end().0
            && other.start.0 < self.end().0
    }
}

/// The producer cursor and the gate it must respect, together.
///
/// Holds both because granting a claim requires reading the gate and advancing
/// the cursor as one step — see the module documentation on why the two cannot
/// be separated without handing out overlapping ranges.
///
/// ```
/// use ring_claim::Claimer;
/// use ring_gating::GatingSet;
/// use ring_types::{Capacity, Seq};
///
/// let consumers = GatingSet::new(Capacity::new(4).unwrap(), 1);
/// let claimer = Claimer::new(&consumers);
///
/// let claim = claimer.claim(3).expect("an empty ring has room");
/// assert_eq!(claim.start(), Seq::ZERO);
/// assert_eq!(claimer.claimed(), Seq(3));
/// ```
#[derive(Debug)]
pub struct Claimer<'a> {
    cursor: PaddedCursor,
    consumers: &'a GatingSet,
}

impl<'a> Claimer<'a> {
    /// A claimer starting at sequence zero, gated by `consumers`.
    ///
    /// ```
    /// use ring_claim::Claimer;
    /// use ring_gating::GatingSet;
    /// use ring_types::{Capacity, Seq};
    ///
    /// let consumers = GatingSet::new(Capacity::new(8).unwrap(), 1);
    /// assert_eq!(Claimer::new(&consumers).claimed(), Seq::ZERO);
    /// ```
    #[must_use]
    pub fn new(consumers: &'a GatingSet) -> Self {
        Self { cursor: PaddedCursor::default(), consumers }
    }

    /// The producer cursor, for `ring_publish` to read and for a gating set on
    /// the other side of the ring to be built against.
    ///
    /// # This is a convention, not encapsulation
    ///
    /// `PaddedCursor` implements the public `SeqCell` trait, and every one of
    /// its methods takes `&self` — so a `&PaddedCursor` is enough to `store` the
    /// cursor backwards or `fetch_add` it past the gate. The second is exactly
    /// the design this crate's module documentation rejects, reachable from
    /// outside in one line of safe code, and
    /// `writing_through_the_cursor_accessor_defeats_the_gate` in
    /// `tests/claim_test.rs` demonstrates both failure modes it produces:
    /// a grant past a gate that had just returned `Full`, and two live `Claim`s
    /// that `Claim::overlaps` reports as covering the same sequences.
    ///
    /// Monotonicity is therefore a property of the *methods* — `claim`,
    /// `claim_up_to`, `claimed` — and not of the cursor itself. Read it, hand it
    /// to `ring_publish`, take its address for a layout assertion; do not write
    /// through it. The crate's manual `§ C2` check greps this crate's own source
    /// for `store` and `fetch_add` and passes, which is correct and says nothing
    /// about callers.
    ///
    /// ```
    /// use core::sync::atomic::Ordering;
    ///
    /// use ring_claim::Claimer;
    /// use ring_cursor::SeqCell;
    /// use ring_gating::GatingSet;
    /// use ring_types::{Capacity, Seq};
    ///
    /// let consumers = GatingSet::new(Capacity::new(8).unwrap(), 1);
    /// let claimer = Claimer::new(&consumers);
    /// assert_eq!(claimer.cursor().load(Ordering::Acquire), Seq::ZERO);
    /// ```
    #[must_use]
    pub const fn cursor(&self) -> &PaddedCursor {
        &self.cursor
    }

    /// The gating set this claimer respects.
    ///
    /// ```
    /// use ring_claim::Claimer;
    /// use ring_gating::GatingSet;
    /// use ring_types::Capacity;
    ///
    /// let consumers = GatingSet::new(Capacity::new(8).unwrap(), 2);
    /// assert_eq!(Claimer::new(&consumers).consumers().len(), 2);
    /// ```
    #[must_use]
    pub const fn consumers(&self) -> &'a GatingSet {
        self.consumers
    }

    /// How far claiming has advanced — one past the last sequence handed out.
    ///
    /// Note this is *claimed*, not published: a slot counted here may still be
    /// mid-write. `ring_publish` tracks the published frontier separately, and
    /// conflating the two is exactly what
    /// `docs/feature/170_claim_publish_available_commit_handshake.md` forbids.
    ///
    /// ```
    /// use ring_claim::Claimer;
    /// use ring_gating::GatingSet;
    /// use ring_types::{Capacity, Seq};
    ///
    /// let consumers = GatingSet::new(Capacity::new(8).unwrap(), 1);
    /// let claimer = Claimer::new(&consumers);
    /// let _claim = claimer.claim(2).unwrap();
    /// assert_eq!(claimer.claimed(), Seq(2));
    /// ```
    #[must_use]
    pub fn claimed(&self) -> Seq {
        self.cursor.load(GATING)
    }

    /// How many slots could be claimed right now.
    ///
    /// A hint only. Between this reading and a [`claim`] another producer may
    /// take the space — which is why `claim` re-checks rather than trusting a
    /// prior `headroom`.
    ///
    /// Costs one load plus one more per registered consumer — cheap for a
    /// diagnostic, worth avoiding in a hot loop with many consumers.
    ///
    /// [`claim`]: Self::claim
    ///
    /// ```
    /// use ring_claim::Claimer;
    /// use ring_gating::GatingSet;
    ///
    /// let consumers = GatingSet::new(ring_types::Capacity::new(4).unwrap(), 1);
    /// let claimer = Claimer::new(&consumers);
    /// assert_eq!(claimer.headroom(), 4);
    ///
    /// let _claim = claimer.claim(3).unwrap();
    /// assert_eq!(claimer.headroom(), 1);
    /// ```
    #[must_use]
    pub fn headroom(&self) -> usize {
        self.consumers.headroom(self.claimed())
    }

    /// Claim exactly `count` contiguous sequences, or fail.
    ///
    /// Never waits and never claims fewer than asked — see
    /// [`claim_up_to`] for the partial variant.
    ///
    /// [`claim_up_to`]: Self::claim_up_to
    ///
    /// # Errors
    ///
    /// [`RingError::BatchTooLarge`] when `count` exceeds the ring's capacity: a
    /// configuration error no consumer's progress can fix, so a retry loop must
    /// stop. [`RingError::Full`] when the space is not available *right now* —
    /// back-pressure, so a retry loop should keep going.
    ///
    /// A `count` of zero always succeeds, even on a full ring — there is
    /// nothing for back-pressure to block. [`claim_up_to`] treats a zero grant
    /// as `Full` instead; the two functions disagree here on purpose.
    ///
    /// ```
    /// use core::sync::atomic::Ordering;
    ///
    /// use ring_claim::Claimer;
    /// use ring_cursor::SeqCell;
    /// use ring_gating::GatingSet;
    /// use ring_types::{Capacity, RingError, Seq};
    ///
    /// let consumers = GatingSet::new(Capacity::new(4).unwrap(), 1);
    /// let claimer = Claimer::new(&consumers);
    ///
    /// assert_eq!(claimer.claim(4).unwrap().start(), Seq::ZERO);
    /// assert_eq!(claimer.claim(1), Err(RingError::Full));
    ///
    /// consumers.cursor(0).unwrap().store(Seq(2), Ordering::Release);
    /// assert_eq!(claimer.claim(2).unwrap().start(), Seq(4));
    /// ```
    pub fn claim(&self, count: usize) -> Result<Claim, RingError> {
        if count > self.consumers.capacity().get() {
            return Err(RingError::BatchTooLarge {
                requested: count,
                capacity: self.consumers.capacity().get(),
            });
        }

        // The gate is the loop condition, and is therefore re-read on every
        // iteration: on a failed exchange another producer moved the cursor, so
        // the headroom computed against the old value is stale and granting on it
        // would overlap that producer's range.
        let mut current = self.claimed();
        while count <= self.consumers.headroom(current) {
            let next = current.advanced_by(count as u64);
            match self.cursor.compare_exchange(current, next, CLAIM_SUCCESS, GATING) {
                Ok(_) => return Ok(Claim::new(current, count)),
                Err(actual) => current = actual,
            }
        }

        Err(RingError::Full)
    }

    /// Claim as many of `max` sequences as are available, down to one.
    ///
    /// For a batching producer that would rather write four items now than wait
    /// for room for eight.
    ///
    /// # Errors
    ///
    /// [`RingError::Full`] when not even one slot is free. Never
    /// `BatchTooLarge` — a `max` wider than the ring is not an error here, it is
    /// simply more than will be granted.
    ///
    /// A `max` of zero is also `Full`, since there is no partial success at
    /// zero to report — this differs from [`claim`], which treats a `count`
    /// of zero as always satisfiable.
    ///
    /// [`claim`]: Self::claim
    ///
    /// ```
    /// use core::sync::atomic::Ordering;
    ///
    /// use ring_claim::Claimer;
    /// use ring_cursor::SeqCell;
    /// use ring_gating::GatingSet;
    /// use ring_types::{Capacity, RingError, Seq};
    ///
    /// let consumers = GatingSet::new(Capacity::new(4).unwrap(), 1);
    /// let claimer = Claimer::new(&consumers);
    ///
    /// let claim = claimer.claim_up_to(100).unwrap();
    /// assert_eq!(claim.len(), 4, "capped at what the ring holds");
    /// assert_eq!(claimer.claim_up_to(1), Err(RingError::Full));
    /// ```
    pub fn claim_up_to(&self, max: usize) -> Result<Claim, RingError> {
        // `granted @ 1..` binds the grant and gates on it in one expression, which
        // is what keeps the headroom re-read in the loop condition rather than
        // duplicated between a pre-loop computation and the retry arm. A grant of
        // zero — no room, or a `max` of zero — exits to the `Full` below.
        let mut current = self.claimed();
        while let granted @ 1.. = max.min(self.consumers.headroom(current)) {
            let next = current.advanced_by(granted as u64);
            match self.cursor.compare_exchange(current, next, CLAIM_SUCCESS, GATING) {
                Ok(_) => return Ok(Claim::new(current, granted)),
                Err(actual) => current = actual,
            }
        }

        Err(RingError::Full)
    }
}
