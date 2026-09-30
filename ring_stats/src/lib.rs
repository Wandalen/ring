//! Ring counters.
//!
//! Tier 1 of the ring family's 33 crates — the concurrency write-path implementation.
//! Depends on `ring_types`.
//!
//! `docs/feature/185_ring_stats.md` asks for counters cheap enough to leave on
//! permanently, because they are the only thing that distinguishes a ring under
//! mild pressure from one quietly discarding traffic. Without them, a drop is
//! invisible until something downstream fails to reconcile, and by then there is
//! no record of whether the ring dropped anything at all. That claim holds only
//! per call, at roughly ten nanoseconds; the aggregate ceiling across threads —
//! bound by one `fetch_add` on one cache line, unmoved by adding cores or
//! spreading load across counters — is a separate property this crate does not
//! state, bound, or provide a lever to raise.
//!
//! The counters are relaxed atomics: a stats read is a diagnostic, never a
//! synchronisation point, so ordering them would buy nothing. The fence-cost
//! half of that argument was never measured here, and measures within noise
//! on this workspace's two target platforms — the layout's cache-line
//! contention is the real, measured cost in this struct, at 2.6x-3.3x, not
//! the ordering. That choice is why [`RingStats`] is shared by reference
//! across threads without a lock.
//!
//! Drops are counted **per policy**, not in one bucket. A ring that dropped a
//! hundred newest items and one that evicted a hundred oldest ones are in
//! completely different trouble, and a single `dropped` counter cannot tell
//! them apart.
//!
//! **Nothing on a live ring's write path moves any of these counters, and that
//! is worth knowing before reading one.** The only production line in the
//! workspace that does is `stats.record_drop( policy, 1 )` inside
//! `ring_overflow::resolve` — and no `src/` anywhere imports `resolve`.
//! `ring_core`, the crate that owns a ring and handles the full-ring case,
//! calls `would_resolve`, the counter-free half, and does not declare
//! `ring_stats` at all. So outside `ring_bench`, which writes a whole run's
//! totals in four calls of its own, every reader here returns a structural zero
//! rather than a measured one. The methods work and are tested; what does not
//! exist is the edge that would let a ring report through them.

#![no_std]
#![deny(missing_docs)]

// Core-only, and now says so. Rationale at `ring_overflow/src/lib.rs`'s own
// attribute — the property is transitive, so it is asserted in all three of
// `ring_types`, this crate, and `ring_overflow` or in none of them.

use core::sync::atomic::{AtomicU64, Ordering};

use ring_types::OverflowPolicy;

/// One ring's counters.
///
/// Every method takes `&self`, so a single instance is shared by every producer
/// and consumer of its ring.
///
/// ```
/// use ring_stats::RingStats;
/// use ring_types::OverflowPolicy;
///
/// let stats = RingStats::new();
/// stats.record_claim(4);
/// stats.record_publish(4);
/// stats.record_drop(OverflowPolicy::DropNewest, 1);
///
/// assert_eq!(stats.claimed(), 4);
/// assert_eq!(stats.published(), 4);
/// assert_eq!(stats.dropped(OverflowPolicy::DropNewest), 1);
/// assert_eq!(stats.dropped_total(), 1);
/// ```
///
/// **`{:?}` is seven independent loads, not a snapshot.** `Debug` is derived and
/// `AtomicU64`'s own `Debug` is a relaxed load, so formatting a shared set reads
/// seven values at seven moments and prints them as one struct literal — which
/// is exactly how a consistent reading would look. Bumping all seven in lockstep
/// from a single writer, over ninety-nine percent of renderings showed a spread
/// the set never held, the widest running to 19,831 on counters that were never
/// more than one apart. `Clone`, `Copy` and `PartialEq` are refused here for
/// precisely that reason and `Debug` is derived anyway, because a log line, an
/// assertion message and a debugger watch all need something.
/// [`RingStats::snapshot`] is what to reach for when the numbers have to agree
/// with each other.
#[derive(Debug, Default)]
pub struct RingStats {
    claimed: AtomicU64,
    published: AtomicU64,
    consumed: AtomicU64,
    dropped_newest: AtomicU64,
    dropped_oldest: AtomicU64,
    failed: AtomicU64,
    wait_nanos: AtomicU64,
}

// The struct is `COUNTERS` counters and nothing else, checked at compile time.
// Every field is an `AtomicU64`, so the type's size is exactly the count times
// one counter's. This is the line an eighth field fails on, and failing here is
// the point: before it, a counter added to the struct but left out of `reset`'s
// array was cleared by nothing, read plausibly, stayed monotone, and failed no
// test in the suite.
const _: () = assert!(
    core::mem::size_of::<RingStats>() == RingStats::COUNTERS * core::mem::size_of::<AtomicU64>()
);

/// Every counter of one [`RingStats`], read once and returned together.
///
/// The value type `RingStats` cannot be. `AtomicU64` forecloses `Clone`, `Copy`
/// and `PartialEq` on the live set — a consumer that wants to hold a reading,
/// print it, diff it against an earlier one or send it somewhere needs this
/// instead.
///
/// **This is not an atomic snapshot, and no such thing is available here.** The
/// numbers still come from [`RingStats::COUNTERS`] `Relaxed` loads taken at that
/// many moments, so a set under traffic can return a combination the ring never
/// held. What the type does guarantee is that the reading is *internally*
/// consistent: `dropped_total` is the sum of the three drop fields **of this
/// value**, and `in_flight` is **this value's own** `claimed - published`.
/// Reading those through [`RingStats::dropped_total`] and
/// [`RingStats::in_flight`] instead re-loads the counters, so the total a caller
/// prints need not be the sum of the breakdown printed beside it — driven from
/// one writer that kept the three drop counters within one of each other, three
/// to five percent of separately-read breakdowns showed a spread the ring never
/// had, the widest running to 3,325.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct StatsCounts {
    /// Slots claimed.
    pub claimed: u64,
    /// Slots published.
    pub published: u64,
    /// Items consumed.
    pub consumed: u64,
    /// Items dropped under [`OverflowPolicy::DropNewest`].
    pub dropped_newest: u64,
    /// Items dropped under [`OverflowPolicy::DropOldest`].
    pub dropped_oldest: u64,
    /// Publishes refused under [`OverflowPolicy::Fail`] — counted, though nothing
    /// was lost.
    pub failed: u64,
    /// The three drop counters above, summed from those same three reads.
    pub dropped_total: u64,
    /// This value's own `claimed - published`, floored at zero.
    pub in_flight: u64,
    /// Nanoseconds spent waiting.
    pub wait_nanos: u64,
}

impl StatsCounts {
    /// In flight, with the caller bug kept apart from a balanced ring.
    ///
    /// `None` means `published` exceeds `claimed` — which the counters permit,
    /// which no correct caller produces, and which `in_flight` floors to zero, the
    /// same value a healthy ring gives. `checked_sub` is one word different from
    /// `saturating_sub` and is the only reading in the crate that tells the two
    /// apart.
    ///
    /// ```
    /// use ring_stats::RingStats;
    ///
    /// let s = RingStats::new();
    /// s.record_claim(2);
    /// s.record_publish(5);
    ///
    /// assert_eq!(s.snapshot().in_flight, 0);
    /// assert_eq!(s.snapshot().checked_in_flight(), None);
    /// ```
    #[must_use]
    pub const fn checked_in_flight(&self) -> Option<u64> {
        self.claimed.checked_sub(self.published)
    }
}

impl RingStats {
    /// How many counters a `RingStats` holds.
    ///
    /// Rust cannot iterate a struct's fields, so the seven are written out by hand
    /// more than once: as struct fields, as `new`'s initialisers, and as
    /// `counters`' array. The compiler already insists on the first two — a
    /// missing field is a hard error. This constant is what makes it insist on the
    /// third: adding an eighth field breaks the size assertion above, raising the
    /// constant to match breaks the array's declared length, and that error names
    /// the array by line. `ring_types` publishes `OverflowPolicy::ALL` and asserts
    /// its length for the same reason; this is that convention applied to the one
    /// hand-written set in this crate that the compiler was not already checking.
    ///
    /// ```
    /// use ring_stats::RingStats;
    /// assert_eq!(RingStats::COUNTERS, 7);
    /// ```
    pub const COUNTERS: usize = 7;

    /// Every counter, in declaration order.
    ///
    /// The array's length is [`RingStats::COUNTERS`], so it cannot silently fall
    /// out of step with the struct — see that constant for what happens when an
    /// eighth field arrives.
    const fn counters(&self) -> [&AtomicU64; Self::COUNTERS] {
        [
            &self.claimed,
            &self.published,
            &self.consumed,
            &self.dropped_newest,
            &self.dropped_oldest,
            &self.failed,
            &self.wait_nanos,
        ]
    }

    /// A fresh set of counters, all zero.
    ///
    /// ```
    /// use ring_stats::RingStats;
    /// assert_eq!(RingStats::new().claimed(), 0);
    /// ```
    #[must_use]
    pub const fn new() -> Self {
        Self {
            claimed: AtomicU64::new(0),
            published: AtomicU64::new(0),
            consumed: AtomicU64::new(0),
            dropped_newest: AtomicU64::new(0),
            dropped_oldest: AtomicU64::new(0),
            failed: AtomicU64::new(0),
            wait_nanos: AtomicU64::new(0),
        }
    }

    /// Record `n` slots claimed.
    ///
    /// ```
    /// use ring_stats::RingStats;
    /// let s = RingStats::new();
    /// s.record_claim(3);
    /// assert_eq!(s.claimed(), 3);
    /// ```
    pub fn record_claim(&self, n: u64) {
        self.claimed.fetch_add(n, Ordering::Relaxed);
    }

    /// Record `n` slots published.
    ///
    /// ```
    /// use ring_stats::RingStats;
    /// let s = RingStats::new();
    /// s.record_publish(2);
    /// assert_eq!(s.published(), 2);
    /// ```
    pub fn record_publish(&self, n: u64) {
        self.published.fetch_add(n, Ordering::Relaxed);
    }

    /// Record `n` items consumed.
    ///
    /// ```
    /// use ring_stats::RingStats;
    /// let s = RingStats::new();
    /// s.record_consume(5);
    /// assert_eq!(s.consumed(), 5);
    /// ```
    pub fn record_consume(&self, n: u64) {
        self.consumed.fetch_add(n, Ordering::Relaxed);
    }

    /// Record `n` items lost, under the policy that lost them.
    ///
    /// [`OverflowPolicy::Fail`] is counted too even though it loses nothing —
    /// the caller was handed the decision, and how often that happened is the
    /// pressure signal for a `Fail` ring.
    ///
    /// ```
    /// use ring_stats::RingStats;
    /// use ring_types::OverflowPolicy;
    ///
    /// let s = RingStats::new();
    /// s.record_drop(OverflowPolicy::DropOldest, 2);
    /// s.record_drop(OverflowPolicy::Fail, 1);
    /// assert_eq!(s.dropped(OverflowPolicy::DropOldest), 2);
    /// assert_eq!(s.dropped(OverflowPolicy::Fail), 1);
    /// assert_eq!(s.dropped_total(), 3);
    /// ```
    pub fn record_drop(&self, policy: OverflowPolicy, n: u64) {
        let counter = match policy {
            OverflowPolicy::DropNewest => &self.dropped_newest,
            OverflowPolicy::DropOldest => &self.dropped_oldest,
            OverflowPolicy::Fail => &self.failed,
        };
        counter.fetch_add(n, Ordering::Relaxed);
    }

    /// Record nanoseconds spent waiting for space or data.
    ///
    /// **No crate calls this.** `wait_nanos` is the fourth of the four counters
    /// `docs/feature/185_ring_stats.md` asks for, and `ring_wait` — the crate that
    /// spins, yields and sleeps — declares `ring_types` and `ring_cursor` in its
    /// manifest, not `ring_stats`. The edge that would let the waiting crate
    /// report its waiting does not exist, so [`RingStats::wait_nanos`] reads zero
    /// in every configuration this workspace can be built in. Zero is also the
    /// legitimate reading for "nothing waited", and nothing distinguishes the two.
    ///
    /// ```
    /// use ring_stats::RingStats;
    /// let s = RingStats::new();
    /// s.record_wait(1_500);
    /// assert_eq!(s.wait_nanos(), 1_500);
    /// ```
    pub fn record_wait(&self, nanos: u64) {
        self.wait_nanos.fetch_add(nanos, Ordering::Relaxed);
    }

    /// Slots claimed so far.
    #[must_use]
    pub fn claimed(&self) -> u64 {
        self.claimed.load(Ordering::Relaxed)
    }

    /// Slots published so far.
    #[must_use]
    pub fn published(&self) -> u64 {
        self.published.load(Ordering::Relaxed)
    }

    /// Items consumed so far.
    #[must_use]
    pub fn consumed(&self) -> u64 {
        self.consumed.load(Ordering::Relaxed)
    }

    /// Items lost under one policy — except `OverflowPolicy::Fail`, whose count is
    /// refusals, not losses: the item was handed back to the caller, not dropped.
    #[must_use]
    pub fn dropped(&self, policy: OverflowPolicy) -> u64 {
        let counter = match policy {
            OverflowPolicy::DropNewest => &self.dropped_newest,
            OverflowPolicy::DropOldest => &self.dropped_oldest,
            OverflowPolicy::Fail => &self.failed,
        };
        counter.load(Ordering::Relaxed)
    }

    /// Items lost across every policy — except `OverflowPolicy::Fail`, whose count
    /// is refusals folded in here anyway: the record-and-read pattern gives every
    /// policy the same drop verb, so this sum cannot tell a loss from a refusal.
    ///
    /// ```
    /// use ring_stats::RingStats;
    /// use ring_types::OverflowPolicy;
    /// let s = RingStats::new();
    /// for p in OverflowPolicy::ALL {
    ///     s.record_drop(p, 1);
    /// }
    /// assert_eq!(s.dropped_total(), 3);
    /// ```
    // Fix(ring_stats_dropped_total_overflow): `dropped_total` folded the three
    // per-policy counters with `Iterator::sum`, plain `u64` addition. `record_drop`
    // takes an unbounded `n`, so two calls whose counts summed past `u64::MAX`
    // panicked in a debug build and silently wrapped to a small number — as low as
    // `0` — in release, reporting a ring that lost an enormous amount of work as
    // one that lost nothing.
    // Root cause: the fold assumed its three inputs would never sum past `u64::MAX`,
    // but nothing enforces that — `record_drop` places no bound on `n`, unlike the
    // claim paths elsewhere in this family that gate a count before ever adding it.
    // Pitfall: an unbounded `record_*( n : u64 )` counter makes every later sum of
    // its stored value unbounded too; check the fold, not just the individual
    // `fetch_add`, for a matching bound.
    #[must_use]
    pub fn dropped_total(&self) -> u64 {
        OverflowPolicy::ALL.iter().map(|p| self.dropped(*p)).fold(0, u64::saturating_add)
    }

    /// Nanoseconds spent waiting.
    ///
    /// Structurally zero — see [`RingStats::record_wait`] for why nothing writes
    /// it.
    #[must_use]
    pub fn wait_nanos(&self) -> u64 {
        self.wait_nanos.load(Ordering::Relaxed)
    }

    /// Slots claimed but not yet published — a nonzero reading here at rest means
    /// a producer took a slot and abandoned it, which is a leak of ring capacity.
    ///
    /// ```
    /// use ring_stats::RingStats;
    /// let s = RingStats::new();
    /// s.record_claim(4);
    /// assert_eq!(s.in_flight(), 4);
    /// s.record_publish(4);
    /// assert_eq!(s.in_flight(), 0);
    /// ```
    ///
    /// **Two loads at two moments, and the error has a direction.** `claimed` is
    /// read first and `published` second, and both only ever climb, so a publish
    /// landing between the two is subtracted from a `claimed` that predates it:
    /// the result comes out at or below the truth and never above it.
    /// `saturating_sub` then floors that error at zero — which is also the reading
    /// a healthy ring gives. So a *nonzero* reading is evidence, and a *zero*
    /// reading taken under traffic is evidence of nothing. Held against a
    /// permanent eight-slot leak with one matched producer beside it, about one
    /// reading in a hundred understated the leak and a handful per two million
    /// reported no leak at all.
    ///
    /// Reversing the two loads is not the repair. Measured, it removes every miss
    /// and invents leaks of hundreds of slots on a ring that is fine — three
    /// percent of readings on a healthy ring, ranging as high as 832. Take
    /// [`RingStats::snapshot`] when the reading has to be self-consistent, and
    /// [`StatsCounts::checked_in_flight`] when `published > claimed` has to be
    /// distinguishable from a balanced ring.
    ///
    /// **Zero is where three separate roads end.** A healthy ring reads it
    /// because every claim was published; a leak sampled at the wrong moment
    /// floors to it, per the paragraph above; and `published > claimed` floors to
    /// it too, distinguishable from the other two only through
    /// [`StatsCounts::checked_in_flight`]'s `None`. A nonzero reading here, not a
    /// zero one, is what this method can actually tell a caller.
    #[must_use]
    pub fn in_flight(&self) -> u64 {
        self.claimed().saturating_sub(self.published())
    }

    /// Every counter, read once, returned as one value.
    ///
    /// The reading is internally consistent — its `dropped_total` is the sum of
    /// its own three drop fields, its `in_flight` its own subtraction — but it is
    /// still [`RingStats::COUNTERS`] loads at that many moments, not an atomic
    /// snapshot. See [`StatsCounts`] for what that does and does not buy.
    ///
    /// ```
    /// use ring_stats::RingStats;
    /// use ring_types::OverflowPolicy;
    ///
    /// let s = RingStats::new();
    /// s.record_claim(4);
    /// s.record_publish(1);
    /// s.record_drop(OverflowPolicy::Fail, 2);
    ///
    /// let counts = s.snapshot();
    /// assert_eq!(counts.in_flight, 3);
    /// assert_eq!(counts.dropped_total, 2);
    /// assert_eq!(counts.failed, 2);
    /// assert_eq!(counts.checked_in_flight(), Some(3));
    /// ```
    #[must_use]
    pub fn snapshot(&self) -> StatsCounts {
        let claimed = self.claimed.load(Ordering::Relaxed);
        let published = self.published.load(Ordering::Relaxed);
        let consumed = self.consumed.load(Ordering::Relaxed);
        let dropped_newest = self.dropped_newest.load(Ordering::Relaxed);
        let dropped_oldest = self.dropped_oldest.load(Ordering::Relaxed);
        let failed = self.failed.load(Ordering::Relaxed);
        let wait_nanos = self.wait_nanos.load(Ordering::Relaxed);

        StatsCounts {
            claimed,
            published,
            consumed,
            dropped_newest,
            dropped_oldest,
            failed,
            // Fix(ring_stats_dropped_total_overflow): same class of bug as
            // `dropped_total`'s own fold, one call site over — see the `Fix` comment
            // there for the concrete triggering sequence and root cause.
            dropped_total: dropped_newest.saturating_add(dropped_oldest).saturating_add(failed),
            in_flight: claimed.saturating_sub(published),
            wait_nanos,
        }
    }

    /// Reset every counter to zero.
    ///
    /// Named for `ring_shutdown`'s reset, so a recycled ring would not carry the
    /// previous world's numbers, per
    /// `docs/feature/184_close_reset_and_drain_all.md` — but `ring_shutdown` does
    /// not declare this crate as a dependency, so that call does not exist yet;
    /// today this is exercised only by this crate's own tests and doctest.
    ///
    /// ```
    /// use ring_stats::RingStats;
    /// let s = RingStats::new();
    /// s.record_claim(9);
    /// s.reset();
    /// assert_eq!(s.claimed(), 0);
    /// ```
    ///
    /// **Not atomic, and the partial state is identifiable.** The counters are
    /// stored in declaration order through a shared reference —
    /// [`RingStats::COUNTERS`] separate stores, `claimed` first and `wait_nanos`
    /// last — so a concurrent reader can land between any two of them. A fill runs
    /// that same order, setting `claimed` before `wait_nanos`, so a read caught
    /// mid-fill returns `( set, 0 )`; a read caught mid-reset returns `( 0, set )`,
    /// a combination no fill and no complete state can produce. Against one writer
    /// looping fill-then-reset, that signature appeared between 5 and 101 times per
    /// two million paired reads across six runs — rare enough to escape a casual
    /// test, common enough to happen. The stated purpose above is exactly the case
    /// where a reader may still be sampling: a shutdown in progress, a monitor not
    /// yet told to stop.
    pub fn reset(&self) {
        for counter in self.counters() {
            counter.store(0, Ordering::Relaxed);
        }
    }
}
