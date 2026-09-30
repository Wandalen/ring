//! Runtime invariant checks over a live ring.
//!
//! One of the ring family's 33 crates — the concurrency write-path implementation.
//!
//! # Why this crate exists
//!
//! The family's cursor arithmetic assumes its own invariants and does not check
//! them. That assumption is correct and the arithmetic is right to make it — but
//! when it is violated, the readings do not merely fail to report the problem,
//! they report the healthiest state they can express.
//!
//! A ring whose consumer cursor has run ahead of its producer reports
//! `free_slots = capacity`, `pending = 0`, and `may_claim = true`: the exact
//! readings of a new, empty ring, including the one a producer acts on. Measured,
//! not inferred — see `docs/pitfall/001_saturating_arithmetic_reports_health.md`.
//!
//! Nothing here runs unless called. These are checks for a test, a debug build,
//! or an investigation, deliberately absent from the claim path.
//!
//! # What it checks
//!
//! | | Needs | Catches |
//! |---|---|---|
//! | [`check`] | One observation | A consumer ahead of its producer; a producer more than a lap ahead |
//! | [`Watch`] | Two or more observations | Either of the above, plus a cursor that moved backwards |
//! | [`check_ends`] | A split live ring | Two public readings of one ring disagreeing about it |
//!
//! # Example
//!
//! ```
//! use core::sync::atomic::Ordering;
//!
//! use ring_atomic::SeqCell;
//! use ring_cursor::CursorPair;
//! use ring_debug::{Violation, check};
//! use ring_types::{Capacity, Seq};
//!
//! let pair = CursorPair::new(Capacity::new(8).unwrap());
//! pair.producer().store(Seq(3), Ordering::Release);
//! assert!(check(&pair).is_ok());
//!
//! // The family's own arithmetic calls this state empty and healthy.
//! pair.consumer().store(Seq(9), Ordering::Release);
//! assert_eq!(pair.free_slots(), 8);
//! assert!(pair.may_claim());
//!
//! // This crate does not.
//! assert_eq!(
//!     check(&pair),
//!     Err(Violation::ConsumerAheadOfProducer { producer: Seq(3), consumer: Seq(9) })
//! );
//! ```

#![deny(missing_docs)]

use core::fmt;
use core::sync::atomic::Ordering;

use ring_atomic::SeqCell;
use ring_core::{Consumer, Producer};
use ring_cursor::CursorPair;
use ring_types::{Capacity, Seq};

/// The ordering every check reads at.
///
/// `Acquire`, matching the gating reads in `ring_cursor` this crate is checking
/// the results of. A weaker ordering would let a check observe a cursor pair
/// that no thread ever held, and report a violation of an invariant that was
/// never violated — a false positive in a diagnostic is worse than no
/// diagnostic, because it sends an investigation somewhere there is nothing to
/// find.
const OBSERVE: Ordering = Ordering::Acquire;

/// Both cursors of a pair, read **producer first**.
///
/// Every check in this crate reads the two cursors, and the order is not
/// neutral. The cursors are monotonic and the two loads cannot be atomic
/// together, so on a live ring one of them is always the older reading — and
/// which one decides which false positive is reachable. An old producer against
/// a fresh consumer fabricates **D1**; the other order fabricates D2.
///
/// Producer-first is the deliberate choice, and it is the worse-looking one: D1
/// is the violation [`Violation::ConsumerAheadOfProducer`] exists to surface
/// precisely because nothing else shows it, so a spurious D1 is the false
/// positive a reader has no second source to refute. It is kept because the
/// alternative is not better, only quieter — a spurious D2 is refuted by
/// glancing at `pending`, which means the reachable false positive would be the
/// one most likely to be dismissed, in the one direction where dismissal is
/// cheap. A diagnostic whose noise is easy to ignore trains its reader to ignore
/// it.
///
/// The window is a couple of instructions wide and `check` is documented as
/// wanting a quiescent ring, so neither ordering is a defect. What was a defect
/// was making this choice three times, at three call sites, without making it
/// once — so it is made here, and the three sites call this.
fn observe_pair(pair: &CursorPair) -> (Seq, Seq) {
    let producer = pair.producer().load(OBSERVE);
    let consumer = pair.consumer().load(OBSERVE);
    (producer, consumer)
}

/// Which of a pair's two cursors a violation is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cursor {
    /// The cursor a producer advances when it publishes.
    Producer,
    /// The cursor a consumer advances when it reads.
    Consumer,
}

impl fmt::Display for Cursor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Producer => f.write_str("producer"),
            Self::Consumer => f.write_str("consumer"),
        }
    }
}

/// An invariant this crate found broken.
///
/// Every variant carries the numbers it was derived from rather than a message,
/// so a caller can assert on the state rather than on prose, and a report can be
/// formatted at the site that knows how the report will be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Violation {
    /// D1 — the consumer has read past what the producer published.
    ///
    /// **The dangerous one.** The family's arithmetic saturates here, so this
    /// state reads as an empty, healthy ring and `may_claim` returns `true`.
    ConsumerAheadOfProducer {
        /// Where the producer had published to.
        producer: Seq,
        /// Where the consumer claimed to have read to — past `producer`.
        consumer: Seq,
    },

    /// D2 — the producer is more than a full lap ahead of the consumer.
    ///
    /// Unread slots have been overwritten. Less dangerous than
    /// [`Self::ConsumerAheadOfProducer`] only because it leaves evidence: `pending`
    /// exceeds capacity, which no valid state can.
    ProducerLappedConsumer {
        /// Where the producer has published to.
        producer: Seq,
        /// Where the consumer has read to.
        consumer: Seq,
        /// The ring size the two are positions in.
        capacity: usize,
    },

    /// D3 — a cursor holds a smaller sequence than it did at a previous
    /// observation.
    ///
    /// Only reachable through [`Watch`]; a single observation cannot see it,
    /// because every individual pair of sequences is a valid pair of sequences.
    CursorWentBackwards {
        /// Which cursor moved.
        cursor: Cursor,
        /// What it read at the previous observation.
        was: Seq,
        /// What it reads now.
        now: Seq,
    },

    /// Two public readings of one live ring do not add up to its capacity.
    ///
    /// [`Consumer::len`] and [`Producer::free_capacity`] are computed
    /// independently; on a quiescent ring their sum is the capacity. A
    /// disagreement means at least one of them is wrong about the ring they both
    /// describe, and a caller holding only one of them cannot tell.
    ReadingsDisagree {
        /// What the consumer says is waiting to be read.
        pending: usize,
        /// What the producer says is free to publish into.
        free: usize,
        /// What the ring says it holds.
        capacity: usize,
    },
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConsumerAheadOfProducer { producer, consumer } => write!(
                f,
                "consumer at {} is ahead of producer at {} — the ring reads as empty and permits a claim",
                consumer.0, producer.0
            ),
            // The distance is the crate's only arithmetic outside a check, and it was
            // written `saturating_sub` — in the crate whose founding measurement is
            // that saturating arithmetic reports health it cannot support. Reached
            // through `check_seqs` the guard has already run and it cannot underflow;
            // reached through a hand-built variant (the fields are public and the enum
            // is deliberately not `#[ non_exhaustive ]`) it absorbed a contradiction
            // into "is 0 ahead of", which reads as a measurement. `checked_sub` makes
            // the contradictory case say so instead of rendering a number.
            Self::ProducerLappedConsumer { producer, consumer, capacity } => {
                match producer.0.checked_sub(consumer.0) {
                    Some(ahead) => write!(
                        f,
                        "producer at {} is {ahead} ahead of consumer at {}, past a capacity of {capacity}",
                        producer.0, consumer.0
                    ),
                    None => write!(
                        f,
                        "producer at {} is behind consumer at {} — a lap report its own cursors contradict",
                        producer.0, consumer.0
                    ),
                }
            },
            Self::CursorWentBackwards { cursor, was, now } => {
                write!(f, "{cursor} cursor went backwards, from {} to {}", was.0, now.0)
            },
            Self::ReadingsDisagree { pending, free, capacity } => {
                write!(f, "pending {pending} plus free {free} is not the capacity {capacity}")
            },
        }
    }
}

impl core::error::Error for Violation {}

/// Check a cursor pair against [`Violation::ConsumerAheadOfProducer`] and
/// [`Violation::ProducerLappedConsumer`].
///
/// Reads both cursors once, `Acquire`, and compares them directly rather than
/// through `ring_seqno` — whose saturating arithmetic is what makes the first of
/// the two invisible.
///
/// **Reads only.** Nothing here stores, and the two loads are the same loads a
/// gating check performs, so calling this against a live ring perturbs it no
/// more than an ordinary claim would.
///
/// # Errors
///
/// The first invariant found broken. D1 is reported in preference to D2 when a
/// pair somehow breaks both, because D1 is the one that reads as healthy and is
/// therefore the one a reader has no other way to learn about.
///
/// ```
/// use ring_cursor::CursorPair;
/// use ring_debug::check;
/// use ring_types::Capacity;
///
/// assert!(check(&CursorPair::new(Capacity::new(4).unwrap())).is_ok());
/// ```
pub fn check(pair: &CursorPair) -> Result<(), Violation> {
    let (producer, consumer) = observe_pair(pair);
    check_seqs(producer, consumer, pair.capacity())
}

/// The comparison, over values rather than over cursors.
///
/// Split out because [`Watch`] needs the same two questions asked of sequences
/// it has already read — reading them a second time would compare two different
/// observations and could report a violation that never existed.
fn check_seqs(producer: Seq, consumer: Seq, capacity: Capacity) -> Result<(), Violation> {
    // The subtraction is the D1 check. Written as two independent blocks — a
    // `consumer.0 > producer.0` guard, then a bare `producer.0 - consumer.0` — the
    // second is sound only because the first precedes it, and nothing but their
    // order holds that in place: both operands are `u64`, so a reordering
    // compiles, and with no `overflow-checks` in any workspace profile a release
    // build wraps to roughly `u64::MAX`, which exceeds every capacity and so
    // returns `ProducerLappedConsumer` for a ring whose actual defect is D1. The
    // wrong diagnosis, delivered confidently, from the crate whose job is the
    // right one.
    //
    // `checked_sub` removes the ordering rather than documenting it: D1 is
    // precisely the case where the subtraction has no answer, so there is no
    // second block to put in the wrong place and no operand order to get wrong.
    let Some(pending) = producer.0.checked_sub(consumer.0) else {
        return Err(Violation::ConsumerAheadOfProducer { producer, consumer });
    };

    if pending > capacity.get() as u64 {
        return Err(Violation::ProducerLappedConsumer {
            producer,
            consumer,
            capacity: capacity.get(),
        });
    }

    Ok(())
}

/// A cursor pair watched across observations, so that a cursor going backwards
/// is visible.
///
/// [`check`] cannot see D3 and no stateless check can: a pair reading `(0, 0)`
/// is either a new ring or a wholly corrupted one, and the reading is identical.
/// A `Watch` keeps the previous observation so the comparison exists.
///
/// ```
/// use core::sync::atomic::Ordering;
///
/// use ring_atomic::SeqCell;
/// use ring_cursor::CursorPair;
/// use ring_debug::{Cursor, Violation, Watch};
/// use ring_types::{Capacity, Seq};
///
/// let pair = CursorPair::new(Capacity::new(8).unwrap());
/// pair.producer().store(Seq(5), Ordering::Release);
/// let mut watch = Watch::new(&pair).expect("a valid pair");
///
/// pair.producer().store(Seq(7), Ordering::Release);
/// assert!(watch.observe(&pair).is_ok(), "forward is fine");
///
/// pair.producer().store(Seq(2), Ordering::Release);
/// assert_eq!(
///     watch.observe(&pair),
///     Err(Violation::CursorWentBackwards { cursor: Cursor::Producer, was: Seq(7), now: Seq(2) })
/// );
/// ```
///
/// # Not `Copy`, deliberately
///
/// The three fields are all `Copy`, so this type could be — and was. It is not,
/// because a `Watch` is *the* record of what was last seen, and a silent copy is
/// a second baseline that diverges from the first. The one left behind reports
/// [`Violation::CursorWentBackwards`] for every later observation of a perfectly
/// healthy ring, because its baseline is stale rather than because anything moved
/// backwards.
///
/// `Clone` is kept: checkpointing a watch before a suspect phase and comparing
/// afterwards is a real use. Requiring the `.clone()` is what makes the fork a
/// decision instead of a typo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Watch {
    producer: Seq,
    consumer: Seq,
    capacity: Capacity,
}

impl Watch {
    /// Start watching, taking the current observation as the baseline.
    ///
    /// The pair is checked as it stands — a `Watch` started against an already
    /// broken pair reports that immediately rather than adopting the broken state
    /// as its baseline and reporting nothing forever after.
    ///
    /// # What this does not guarantee
    ///
    /// The check is on the **baseline**, not on the ring, and it is worth being
    /// precise about how far that reaches — because the guarantee is strong enough
    /// to be relied on and narrow enough to be relied on wrongly.
    ///
    /// - **D3 is unchecked here, necessarily.** A backwards move is a property of
    ///   two readings and this call has one. A `Watch` started *after* a cursor
    ///   reset holds a baseline entirely consistent with itself and entirely wrong
    ///   about the ring's history.
    /// - **It says nothing about the pair passed to [`Self::observe`].** Nothing
    ///   ties a `Watch` to the pair it was built from, so a watch validated against
    ///   one ring and then observed against another carries a construction-time
    ///   guarantee about a ring it is no longer reporting on. The two combine into
    ///   something neither states alone: the strongest guarantee this type offers
    ///   is voided by a call that type-checks.
    ///
    /// # Errors
    ///
    /// Whatever [`check`] finds at this moment.
    pub fn new(pair: &CursorPair) -> Result<Self, Violation> {
        let (producer, consumer) = observe_pair(pair);
        let capacity = pair.capacity();

        check_seqs(producer, consumer, capacity)?;
        Ok(Self { producer, consumer, capacity })
    }

    /// Take another observation and compare it with the last one.
    ///
    /// On success the observation becomes the new baseline. On failure it does
    /// **not**: a `Watch` that adopted a corrupt reading would report the
    /// corruption once and then treat it as the new normal, which turns a
    /// permanent fault into a single lost message.
    ///
    /// # Errors
    ///
    /// [`Violation::CursorWentBackwards`] for either cursor, checked before D1 and
    /// D2 — a cursor that moved backwards explains any ordering violation that
    /// came with it, and reporting the consequence instead of the cause sends an
    /// investigation to the wrong place.
    pub fn observe(&mut self, pair: &CursorPair) -> Result<(), Violation> {
        let (producer, consumer) = observe_pair(pair);

        if producer.0 < self.producer.0 {
            return Err(Violation::CursorWentBackwards {
                cursor: Cursor::Producer,
                was: self.producer,
                now: producer,
            });
        }

        if consumer.0 < self.consumer.0 {
            return Err(Violation::CursorWentBackwards {
                cursor: Cursor::Consumer,
                was: self.consumer,
                now: consumer,
            });
        }

        check_seqs(producer, consumer, self.capacity)?;

        self.producer = producer;
        self.consumer = consumer;
        Ok(())
    }

    /// The baseline this watch will compare the next observation against.
    ///
    /// Producer first, then consumer. Exposed so a test can assert that a failed
    /// [`Self::observe`] left the baseline alone.
    #[must_use]
    pub fn last(&self) -> (Seq, Seq) {
        (self.producer, self.consumer)
    }
}

/// Check that a split ring's two ends agree about the ring they share.
///
/// [`Consumer::len`] and [`Producer::free_capacity`] are computed
/// independently and describe the same ring; on a quiescent ring their sum is
/// the capacity. This is the one check here that is about a *ring* rather than
/// about cursors, and it is the only one available to a caller holding a
/// `ring_core::Ring` — nothing in the family hands out a `CursorPair`.
///
/// **It cannot detect a consumer ahead of its producer.** Both readings derive
/// from the saturating arithmetic that masks that state, so a D1-corrupted ring
/// satisfies this check exactly as a healthy one does. Measured, and pinned by
/// `check_ends_cannot_see_the_corruption_check_can` — see
/// `docs/integration/001`, whose J4 is precisely "do not read a pass here as
/// evidence against D1".
///
/// **Quiescent means quiescent.** Both readings are separate atomic loads, so
/// on a ring being written concurrently they are two snapshots of two moments
/// and their sum is not required to be anything. Calling this mid-flight
/// produces false positives; that is a precondition on the caller, not a defect
/// to be worked around here.
///
/// The `T : Send` bound is inherited, not chosen: `ring_core` puts it on the
/// ends themselves, so `len` and `free_capacity` are not callable without it.
///
/// # Where the capacity comes from
///
/// **The parameter is here because of a borrow, not because you are meant to
/// choose it.** Neither end carries a capacity accessor, and `Ring::capacity`
/// takes `&self` while `Ring::ends` takes `&mut self` — so once the ends exist,
/// the ring is exclusively borrowed and the number is out of reach. It is not
/// derivable from anything this function is passed.
///
/// It is derivable one statement earlier, and that is the only shape worth
/// copying: read it off the ring **before** splitting, and pass the binding.
///
/// ```rust,ignore
/// let mut ring : Ring< u32 > = Ring::new( &config )?;
/// let capacity = ring.capacity();          // before the &mut borrow
/// let mut ends = ring.ends();
/// let ( producer, consumer ) = ends.split();
/// check_ends( capacity, &producer, &consumer )?;
/// ```
///
/// Re-typing the size as a literal compiles and passes, and stops being true the
/// first time the ring is built with a different one. A wrong capacity is
/// reported as [`Violation::ReadingsDisagree`], which is the right report and
/// the wrong moment to find out.
///
/// # Errors
///
/// [`Violation::ReadingsDisagree`], carrying all three numbers.
pub fn check_ends<T>(
    capacity: Capacity,
    producer: &Producer<'_, T>,
    consumer: &Consumer<'_, T>,
) -> Result<(), Violation>
where
    T: Send,
{
    let pending = consumer.len();
    let free = producer.free_capacity();

    if pending + free == capacity.get() {
        return Ok(());
    }

    Err(Violation::ReadingsDisagree { pending, free, capacity: capacity.get() })
}
