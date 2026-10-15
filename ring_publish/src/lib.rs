//! Publication of claimed slots to consumers.
//!
//! Part of the ring family's concurrency write path.
//!
//! This crate does not depend on `ring_seqno`, though it was in the manifest
//! before the implementation existed. Publishing is a cursor
//! advance and a contiguity test; it computes no distances, no free slots and
//! no minimum. The same over-declaration was found and removed in `ring_claim`.
//!
//! This is the second half of the claim, publish, available and commit
//! handshake. Its reached-test runs the whole handshake under `loom` and lives
//! in this crate's `tests/handshake_test.rs`, because publication is the point
//! at which claim, publish, available and commit become observable together.
//!
//! ## The published cursor is not the claimed cursor
//!
//! `ring_claim` advances a cursor when a producer *takes* a range;
//! this crate advances a different one when the producer has *finished writing*
//! it. Between the two, the slot is claimed and unwritten. The feature's
//! central requirement is that a consumer never sees it.
//!
//! Conflating the two cursors is not a subtle bug. It publishes uninitialised
//! memory, it passes every single-threaded test (where the write completes
//! before anything can read), and it fails only under load.
//!
//! ## Lifecycle: a sequence from claim to commit
//!
//! | State | Holds when | Entered by | Cursor moved | Owes |
//! |---|---|---|---|---|
//! | Unclaimed | `seq >= claimed` | | | |
//! | Claimed, unwritten | `published <= seq < claimed` | `ring_claim`'s `Claimer::claim` | claimed | a write |
//! | Claimed, written | `published <= seq < claimed` | the producer's own write into the slot | none | a publish of exactly the claimed range, see [`Publisher::publish`]'s `# Panics` |
//! | Published | `position <= seq < published` | [`Publisher::publish`] | published | a commit, which frees the slot |
//! | Committed | `seq < position` | `ring_consume`'s `Consumer::commit` | the consumer's position | |
//!
//! The write moves no cursor, so nothing outside the producer can tell the
//! second state from the third, and nothing needs to. The published cursor is
//! the one boundary between written and possibly unwritten, and its `Release`
//! orders the write before it. [`Publisher::is_published`] answers `false` for
//! the first three states alike, which is the guarantee a consumer relies on.
//!
//! ## Why publication is refused rather than reordered
//!
//! [`Publisher::try_publish`] advances only when the published cursor is
//! *exactly* at the claim's start. A producer whose predecessor has not
//! finished is told so and must try again.
//!
//! The alternative is to publish to the highest contiguous point, or to track
//! per-slot availability. Either would let producer B's publication proceed
//! while producer A is still writing. That works, and it is what a
//! high-contention multi-producer ring eventually needs. It is deliberately not
//! here, because it is `ring_mpsc`'s problem, and putting it in this crate
//! would make the crate untestable without a second producer. `ring_mpsc`
//! tracks availability with a per-slot sequence stamp.
//!
//! ## Why a plain spin, and not a `WaitKind`
//!
//! [`Publisher::publish`] loops on a `spin_loop` hint with no wait strategy and
//! no budget, which everywhere else in this family would be a bug. Here it is
//! correct, and the difference is what the loop waits *for*.
//!
//! Waiting for space is unbounded. It depends on a consumer that may be slow,
//! stalled, or gone, so it needs a strategy and a give-up. Waiting for your
//! predecessor to publish is bounded by that producer finishing a slot write it
//! has already started and cannot abandon. That producer is not blocked on
//! anything itself. A `WaitKind` here would offer a `Park` that can only ever hurt, and
//! a budget whose exhaustion has no correct handling.
//!
//! `Yield` is not offered either. The expected wait is one slot write, and a
//! yield adds a scheduler round trip to it. Whether that holds is a revisit
//! trigger in `docs/decisions/001_publish_takes_a_bare_start_and_len.md`.

#![deny(missing_docs)]

use ring_cursor::{GATING, PaddedCursor, SeqCell};
use ring_types::Seq;

/// The ordering a publication is made visible at.
///
/// `Release`, paired with the consumer's `Acquire` read of the same cursor.
/// That pairing is the entire happens-before edge between a producer's slot
/// writes and a consumer's reads of them. Weakening it to `Relaxed` produces a
/// ring that works on x86, where the hardware supplies the ordering the code
/// failed to ask for, and races on aarch64.
const PUBLISH: core::sync::atomic::Ordering = core::sync::atomic::Ordering::Release;

/// The frontier a consumer may read up to.
///
/// Separate from `ring_claim::Claimer`'s cursor on purpose. The module
/// documentation explains why conflating them publishes unwritten slots.
///
/// ```
/// use ring_publish::Publisher;
/// use ring_types::Seq;
///
/// let publisher = Publisher::new();
/// assert_eq!( publisher.published(), Seq::ZERO );
///
/// assert_eq!( publisher.try_publish( Seq::ZERO, 3 ), Ok( Seq( 3 ) ) );
/// assert_eq!( publisher.published(), Seq( 3 ) );
/// ```
#[derive(Debug, Default)]
pub struct Publisher {
  cursor: PaddedCursor,
}

impl Publisher {
  /// A publisher with nothing published.
  ///
  /// ```
  /// use ring_publish::Publisher;
  /// use ring_types::Seq;
  /// assert_eq!( Publisher::new().published(), Seq::ZERO );
  /// ```
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  /// The published cursor, for a consumer's barrier to be built over.
  ///
  /// # Pitfall: the handed-out cursor can be written
  ///
  /// **Trap.** Treating the returned reference as read-only.
  ///
  /// **Failure.** [`PaddedCursor`] implements [`SeqCell`], so `store` and
  /// `fetch_add` compile on it. A `store` moves the frontier anywhere, including
  /// backwards, which un-publishes slots a consumer may already be reading. This
  /// crate moves the cursor only through [`Self::try_publish`], from the exact
  /// current frontier, and cannot stop a holder of this reference from doing
  /// otherwise.
  ///
  /// **Mitigation.** Only read it, through [`SeqCell::load`]. This crate has no
  /// read-only cursor type to hand out instead.
  ///
  /// ```
  /// use core::sync::atomic::Ordering;
  /// use ring_cursor::SeqCell;
  /// use ring_publish::Publisher;
  /// use ring_types::Seq;
  ///
  /// let publisher = Publisher::new();
  /// assert_eq!( publisher.cursor().load( Ordering::Acquire ), Seq::ZERO );
  /// ```
  #[must_use]
  pub const fn cursor(&self) -> &PaddedCursor {
    &self.cursor
  }

  /// How far publication has reached, as one past the last readable sequence.
  ///
  /// ```
  /// use ring_publish::Publisher;
  /// use ring_types::Seq;
  ///
  /// let publisher = Publisher::new();
  /// let _ = publisher.try_publish( Seq::ZERO, 2 );
  /// assert_eq!( publisher.published(), Seq( 2 ) );
  /// ```
  #[must_use]
  pub fn published(&self) -> Seq {
    self.cursor.load(GATING)
  }

  /// Publish `len` sequences starting at `start`, if it is this producer's
  /// turn.
  ///
  /// # Errors
  ///
  /// The current published position, when it is not `start`. Deliberately not a
  /// `RingError`, because this is `compare_exchange`'s "not now" rather than a
  /// failure, and returning the position spares the caller a second load that
  /// could already be stale.
  ///
  /// The caller cannot retry against the returned value. A producer publishes
  /// only the range it claimed, so `start` stays fixed. What the value tells the
  /// caller is which side of the frontier it is on. Below `start`, an earlier
  /// claim is still unpublished, and a retry succeeds once that producer
  /// publishes. Above `start`, the frontier has already passed the range, so it
  /// was published before or never belonged to this caller. The frontier only
  /// moves forward, so no retry succeeds.
  ///
  /// ```
  /// use ring_publish::Publisher;
  /// use ring_types::Seq;
  ///
  /// let publisher = Publisher::new();
  ///
  /// // Producer B finished first, but A's range is still unwritten.
  /// assert_eq!( publisher.try_publish( Seq( 4 ), 4 ), Err( Seq::ZERO ) );
  /// assert_eq!( publisher.published(), Seq::ZERO, "and nothing moved" );
  ///
  /// assert_eq!( publisher.try_publish( Seq::ZERO, 4 ), Ok( Seq( 4 ) ) );
  /// assert_eq!( publisher.try_publish( Seq( 4 ), 4 ), Ok( Seq( 8 ) ), "now it is B's turn" );
  /// ```
  pub fn try_publish(&self, start: Seq, len: usize) -> Result<Seq, Seq> {
    let end = start.advanced_by(len as u64);
    self.cursor.compare_exchange(start, end, PUBLISH, GATING).map(|_| end)
  }

  /// Publish `len` sequences starting at `start`, waiting for this producer's
  /// turn.
  ///
  /// Spins rather than taking a [`ring_types::WaitKind`]. The module
  /// documentation explains why a strategy would be wrong here, not merely
  /// absent.
  ///
  /// ```
  /// use ring_publish::Publisher;
  /// use ring_types::Seq;
  ///
  /// let publisher = Publisher::new();
  /// assert_eq!( publisher.publish( Seq::ZERO, 3 ), Seq( 3 ) );
  /// assert_eq!( publisher.publish( Seq( 3 ), 1 ), Seq( 4 ) );
  /// ```
  ///
  /// # Panics
  ///
  /// Never. It deadlocks instead, and there are two ways in.
  ///
  /// A caller that publishes a range it never claimed waits for a turn that
  /// cannot arrive. That is a caller bug this crate cannot detect, and
  /// `try_publish` is the variant for a caller that wants to decide for itself.
  ///
  /// A caller whose *predecessor* dropped its claim without publishing waits
  /// just as long, and that one is not the waiting caller's bug at all. The
  /// module documentation's termination argument says a predecessor "cannot
  /// abandon" a slot write it has already started. That describes correct
  /// producers, not a property the types enforce.
  /// `ring_claim::Claim` has no destructor, and most ways of abandoning one draw
  /// no warning, as `Claim`'s pitfall lists. Of the two deadlocks this is the
  /// reachable one, and the only defence against it is that every producer
  /// publishes what it claims.
  pub fn publish(&self, start: Seq, len: usize) -> Seq {
    loop {
      if let Ok(end) = self.try_publish(start, len) {
        return end;
      }
      core::hint::spin_loop();
    }
  }

  /// Whether `seq` has been published and is therefore readable.
  ///
  /// The handshake feature is graded on this consumer-facing question. A slot
  /// claimed but not published must answer `false`.
  ///
  /// Each call is one `Acquire` load of the frontier, so a caller asking about
  /// several sequences may get each answer from a different frontier. Read
  /// [`Self::published`] once and compare against it instead. A consumer reads
  /// through `ring_consume`'s `Consumer::available`, which reads the frontier
  /// once for a whole run.
  ///
  /// ```
  /// use ring_publish::Publisher;
  /// use ring_types::Seq;
  ///
  /// let publisher = Publisher::new();
  /// let _ = publisher.try_publish( Seq::ZERO, 2 );
  ///
  /// assert!( publisher.is_published( Seq::ZERO ) );
  /// assert!( publisher.is_published( Seq( 1 ) ) );
  /// assert!( !publisher.is_published( Seq( 2 ) ), "claimed, perhaps, but not published" );
  /// ```
  #[must_use]
  pub fn is_published(&self, seq: Seq) -> bool {
    seq < self.published()
  }
}
