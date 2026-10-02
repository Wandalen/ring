//! Publication of claimed slots to consumers.
//!
//! Tier 5 of the ring family's 33 crates, which together implement the concurrency write-path.
//! Depends on `ring_types`, `ring_cursor`.
//!
//! `ring_seqno` was added to this crate's manifest before the implementation
//! existed and is not among them. Publishing is a cursor
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
//! ## Why publication is refused rather than reordered
//!
//! [`Publisher::try_publish`] advances only when the published cursor is
//! *exactly* at the claim's start. A producer whose predecessor has not
//! finished is told so and must try again.
//!
//! The alternative is to publish to the highest contiguous point, or to track
//! per-slot availability in a bitmap. Either would let producer B's publication
//! proceed while producer A is still writing. That works, and it is what a
//! high-contention multi-producer ring eventually needs. It is deliberately not
//! here, because it is `ring_mpsc`'s problem, and putting it in this crate
//! would make the crate untestable without a second producer.
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
  /// The current published position, when it is not `start`. That means some
  /// earlier claim has not been published yet. Deliberately not a `RingError`,
  /// because this is `compare_exchange`'s "try again" rather than a failure.
  /// The returned value is what to try against next.
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
  /// `ring_claim::Claim` has no destructor, so an abandoned claim is a
  /// `#[ must_use ]` warning and nothing more, and `let _ = …` silences even
  /// that. Of the two deadlocks this is the reachable one, and the only
  /// defence against it is that every producer publishes what it claims.
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
