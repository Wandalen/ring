//! Non-blocking progress helpers.
//!
//! One of the 33 crates of the ring family, which implements the concurrency write-path.
//! Depends on `ring_core`.
//!
//! # What this crate is for
//!
//! The feature for try-only operations on the tick path asks that the
//! operations reachable from inside a system be exclusively the fallible,
//! non-parking ones, and that the restriction be *"enforced by what is
//! exposed, not by a rule in a document"*.
//!
//! Half of that is already true without this crate. `ring_core::Producer` and
//! `ring_core::Consumer` have no parking operation on them at all. The parking
//! lives one crate over, in `ring_wait`, so there are no blocking calls to hide.
//! Two questions remain:
//!
//! 1. **What stops a tick-path crate from adding `ring_wait` to its manifest?**
//!    Nothing in the language. The enforcement available is a dependency-graph
//!    property, and this crate asserts it. See [`PARKING_CRATES`].
//! 2. **Why would anyone reach for the parking version in the first place?**
//!    Because try-only is tedious to write by hand. Every helper below exists
//!    to make the non-parking spelling the short one, on the theory that a
//!    restriction survives exactly as long as obeying it stays cheap.
//!
//! # The limitation to know first
//!
//! *Non-parking is necessary and not sufficient.* [`Budget::new`] lets a caller
//! ask for a million attempts; that never deadlocks and will still blow a frame
//! budget. The default is [`Budget::once`] for that reason.

#![deny(missing_docs)]

use ring_core::{Consumer, Producer};

/// The family crates that declare `ring_wait` as a direct dependency.
///
/// Crates that reach a parking operation only transitively are not listed;
/// the second table below covers the two that reach it through an intermediate
/// crate.
///
/// This roster is part of the public API rather than a constant inside a test,
/// so adding a crate to it is a visible API change rather than a quiet edit to
/// an assertion. Every name here is a crate that depends on `ring_wait`,
/// directly or by re-export, and none of them is on the tick path:
///
/// | Crate | Why it may park | Where it runs |
/// |---|---|---|
/// | `ring_wait` | The parking lives here | Outside the tick |
/// | `ring_barrier` | Waits for a cohort to arrive | Outside the tick |
/// | `ring_shutdown` | Waits for a close, or for space before one | Outside the tick |
///
/// Two more crates reach a parking operation without appearing above, because
/// they declare an intermediate rather than `ring_wait` itself:
///
/// | Crate | Reaches `ring_wait` through |
/// |---|---|
/// | `ring_consume` | `ring_barrier` |
/// | `ring_testkit` | `ring_shutdown` |
///
/// The roster is hand-maintained, which is the same weakness `ring_types`'
/// variant list carries. To keep it honest,
/// `tests/poll_test.rs::the_tick_path_cannot_reach_a_parking_operation` reads
/// the manifests off disk and compares them against this array, so a crate that
/// gains a **direct** `ring_wait` dependency without being listed here fails the
/// suite. That scan is a string search over `Cargo.toml` files, so it is blind
/// to the two rows above by construction. A crate that starts reaching
/// `ring_wait` through a new intermediate changes nothing the scan can see.
/// `the_transitive_reach_is_wider_than_the_manifest_scan_can_see` pins the two
/// known cases so that the blindness is a measured quantity rather than an
/// unexamined one. Neither crate is on the tick path, which is why the roster
/// is still the right thing for this array to hold.
///
/// ```
/// assert!( ring_poll::PARKING_CRATES.contains( &"ring_wait" ) );
/// assert!( !ring_poll::PARKING_CRATES.contains( &"ring_handle" ) );
/// ```
pub const PARKING_CRATES: [&str; 3] = ["ring_barrier", "ring_shutdown", "ring_wait"];

// ── Budget ────────────────────────────────────────────────────────────────

/// How many times a tick-path helper may try before giving up.
///
/// A budget bounds *attempts*, never time. Between attempts the helpers emit a
/// CPU pause hint and nothing else. They never yield, sleep or park, so a budget
/// of any size returns, and a budget of the wrong size returns late.
///
/// # An attempt is not the same size everywhere
///
/// All three budget-taking helpers spend the same *count*, and one attempt does
/// not buy the same amount of work in each:
///
/// | Helper | One attempt is |
/// |---|---|
/// | [`push_within`] | one `try_push`, one record |
/// | [`recv_within`] | one `try_recv`, one record |
/// | [`push_batch_within`] | one whole `try_push_batch`, up to the ring's free space |
///
/// So [`Budget::once`], the conservative default, buys one ring operation from
/// the first two and, from the third, a batch this budget does not bound. The
/// ceiling on a batch is the ring's free capacity and the iterator's length,
/// never the budget. [`Tick::drain`] makes the matching argument in the other
/// direction. A drain limit bounds *successes* where a budget bounds *retries*,
/// and the batch helper is the case where the two units were never separated.
///
/// ```
/// use ring_poll::Budget;
///
/// assert_eq!( Budget::once().attempts(), 1 );
/// assert_eq!( Budget::new( 4 ).attempts(), 4 );
/// assert_eq!( Budget::new( 0 ).attempts(), 1, "zero attempts is not a budget" );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Budget(usize);

impl Budget {
  /// A budget of exactly one attempt, so a helper tries once and takes the answer.
  ///
  /// This is the tick-path default, and the only budget that cannot cost more
  /// than one ring operation.
  #[must_use]
  pub const fn once() -> Self {
    Self(1)
  }

  /// A budget of `attempts` attempts, clamped upward to one.
  ///
  /// Zero is clamped rather than rejected. A helper that "tries zero times" is a
  /// no-op with a misleading name, not a bounded retry. Its `Err`, returned
  /// without ever touching the ring, would read as back-pressure when it is
  /// nothing of the sort.
  #[must_use]
  pub const fn new(attempts: usize) -> Self {
    if attempts == 0 { Self(1) } else { Self(attempts) }
  }

  /// How many attempts this budget permits, always at least one.
  #[must_use]
  pub const fn attempts(self) -> usize {
    self.0
  }
}

impl Default for Budget {
  /// [`Budget::once`], the tick-path default.
  fn default() -> Self {
    Self::once()
  }
}

// ── Progress ──────────────────────────────────────────────────────────────

/// Whether a step moved any records, and how many.
///
/// A tick that made no progress is worth reporting up. It is the signal a
/// scheduler needs to decide whether spinning the same systems again is worth
/// anything.
///
/// ```
/// use ring_poll::Progress;
///
/// assert!( !Progress::of( 0 ).is_made() );
/// assert_eq!( Progress::of( 2 ).then( Progress::of( 3 ) ).count(), 5 );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Progress {
  /// Records moved, and this many of them.
  Made(usize),
  /// Nothing moved.
  None,
}

impl Progress {
  /// [`Progress::Made`] for a non-zero count, [`Progress::None`] for zero.
  ///
  /// Constructing through this rather than by hand keeps anyone from spelling
  /// `Made( 0 )`, a value that claims progress and carries none.
  #[must_use]
  pub const fn of(count: usize) -> Self {
    if count == 0 { Self::None } else { Self::Made(count) }
  }

  /// Whether anything moved.
  // Fix(progress_is_made_classification_not_exhaustive): was `matches!( self,
  //   Self::Made( _ ) )`, so a third `Progress` variant would silently read
  //   `false`, meaning "nothing moved", with nothing forcing a second look. `count`
  //   just below already carries the exhaustive shape.
  // Root cause: `matches!` over a single named variant answers every variant
  //   it was not told about with the same default, compiling cleanly however
  //   many variants `Progress` gains.
  // Pitfall: a scheduler gating "is spinning the same systems again worth
  //   anything?" on this predicate would treat new-kind progress as no
  //   progress at all.
  #[must_use]
  pub const fn is_made(self) -> bool {
    match self {
      Self::Made(_) => true,
      Self::None => false,
    }
  }

  /// How many records moved, zero for [`Progress::None`].
  #[must_use]
  pub const fn count(self) -> usize {
    match self {
      Self::Made(count) => count,
      Self::None => 0,
    }
  }

  /// The two steps of one tick, combined.
  // Fix(ring_poll_progress_then_overflow): `then` added the two counts with plain
  // `+`, so `Progress::of( usize::MAX ).then( Progress::of( 1 ) )` panicked in a
  // debug build and silently returned `Progress::None` in release. That is the
  // exact false "no progress" reading this type exists to rule out.
  // Root cause: `Progress::of` is a public, unconstrained constructor over any
  // `usize`, so nothing this crate controls bounds `then`'s two operands. Only
  // `Tick`'s own private, ring-traffic-driven counter had been checked against
  // overflow. That counter's "impractical to reach" reasoning does not transfer
  // to a public constructor one direct call away from the same class of bug.
  // Pitfall: a public constructor wrapping a raw integer puts arithmetic on its
  // output outside any bound the defining crate controls. Audit every site that
  // combines two of its values, not only the crate's own internal accumulators.
  #[must_use]
  pub const fn then(self, other: Self) -> Self {
    Self::of(self.count().saturating_add(other.count()))
  }
}

// ── The helpers ───────────────────────────────────────────────────────────

/// Publish one record, retrying within `budget`, never parking.
///
/// Returns the record on failure so the caller can decide whether to retry next
/// tick, drop it, or route it elsewhere. That is why the signature is
/// `Result< (), T >` rather than `Result< (), Error >`. A tick that loses the
/// record has no third option.
///
/// # Not counted by a `Tick`
///
/// [`Tick::push`] calls this and adds the result to its own `moved`; calling
/// this directly does not. [`Tick::budget`] makes holding both convenient,
/// since it hands out exactly the argument this function wants. A caller who
/// does gets a `Progress` short by every record that went through this
/// function. The records all move, in order, exactly once; the number a
/// scheduler reads is the number that took the counted path.
///
/// **Under `OverflowPolicy::DropNewest` this returns `Ok` on the first attempt
/// even when the ring is full**, having discarded the record. The policy does
/// that, not this function.
///
/// ```
/// use ring_config::RingConfig;
/// use ring_core::Ring;
/// use ring_poll::{ push_within, Budget };
///
/// let mut ring : Ring< u32 > = Ring::new( &RingConfig::new( 4 ).unwrap() ).unwrap();
/// let mut ends = ring.ends();
/// let ( mut producer, _consumer ) = ends.split();
///
/// assert!( push_within( &mut producer, 7, Budget::once() ).is_ok() );
/// ```
pub fn push_within<T: Send>(producer: &mut Producer<'_, T>, record: T, budget: Budget) -> Result<(), T> {
  let mut held = record;
  let mut attempt = 0;
  // `while` rather than `loop` + `break` on purpose, here and at the two other
  // budget-bounded loops below. `ring_shutdown` measured the difference.
  // `cargo llvm-cov` attributes a `loop`'s exit edge to a region no test can
  // enter, reporting 72/73 lines where the `while` form reports 73/73 on an
  // identical suite. Rewriting this as a `loop` reads as pure simplification
  // and costs a coverage line that will show up days later, attributed to
  // whatever else moved. To check, make the change and re-run
  // `cargo llvm-cov -p ring_poll`.
  while attempt < budget.attempts() {
    match producer.try_push(held) {
      Ok(()) => return Ok(()),
      Err(returned) => {
        held = returned;
        attempt += 1;
        if attempt < budget.attempts() {
          // A pause hint, and nothing more. Yielding here would be the parking
          // this crate exists to keep off the tick path.
          core::hint::spin_loop();
        }
      }
    }
  }
  Err(held)
}

/// Publish from `records` in batches, retrying within `budget`, never parking.
///
/// Returns how many records were published. Stops early on an attempt that
/// moves nothing, because a second attempt against an unchanged ring can only
/// find the same answer. The budget buys time for *another thread* to drain,
/// not for this one to try harder.
///
/// Every attempt pulls a record from `records` before it can know whether
/// the ring has room. A refused attempt consumes and drops that record, so a
/// budget above one can destroy records on a full ring. The published count
/// that this function returns does not show the loss. A caller who needs that
/// number can subtract. The iterator is still theirs, so what it yielded less
/// what this returned is what was destroyed. [`Tick::push_batch`] does exactly
/// that and records it as [`Tick::lost`].
///
/// # The budget bounds rounds, not records
///
/// One attempt here is a whole `try_push_batch`, whose inner loop takes no
/// limit from `budget` and ends only when the ring refuses. [`Budget::once`]
/// therefore buys one ring operation from [`push_within`] and up to the ring's
/// free space from this. See [`Budget`]'s own table. To bound records rather
/// than rounds, bound the iterator: `records.by_ref().take( n )`.
///
/// # Not counted by a `Tick`
///
/// As with [`push_within`], a `Tick` counts only what goes through
/// [`Tick::push_batch`].
///
/// ```
/// use ring_config::RingConfig;
/// use ring_core::Ring;
/// use ring_poll::{ push_batch_within, Budget };
///
/// let mut ring : Ring< u32 > = Ring::new( &RingConfig::new( 8 ).unwrap() ).unwrap();
/// let mut ends = ring.ends();
/// let ( mut producer, _consumer ) = ends.split();
///
/// let published = push_batch_within( &mut producer, &mut ( 0..5 ), Budget::once() );
/// assert_eq!( published, 5 );
/// ```
pub fn push_batch_within<T: Send>(
  producer: &mut Producer<'_, T>,
  records: &mut impl Iterator<Item = T>,
  budget: Budget,
) -> usize {
  let mut total = 0;
  let mut attempt = 0;
  while attempt < budget.attempts() {
    let moved = producer.try_push_batch(records);
    total += moved;
    if moved == 0 {
      break;
    }
    attempt += 1;
    if attempt < budget.attempts() {
      core::hint::spin_loop();
    }
  }
  total
}

/// Take one record, retrying within `budget`, never parking.
///
/// # Not counted by a `Tick`
///
/// As with [`push_within`], a `Tick` counts only what goes through
/// [`Tick::recv`].
///
/// ```
/// use ring_config::RingConfig;
/// use ring_core::Ring;
/// use ring_poll::{ recv_within, Budget };
///
/// let mut ring : Ring< u32 > = Ring::new( &RingConfig::new( 4 ).unwrap() ).unwrap();
/// let mut ends = ring.ends();
/// let ( mut producer, mut consumer ) = ends.split();
/// producer.try_push( 9 ).unwrap();
///
/// assert_eq!( recv_within( &mut consumer, Budget::once() ), Some( 9 ) );
/// assert_eq!( recv_within( &mut consumer, Budget::new( 3 ) ), None );
/// ```
pub fn recv_within<T: Send>(consumer: &mut Consumer<'_, T>, budget: Budget) -> Option<T> {
  let mut attempt = 0;
  while attempt < budget.attempts() {
    if let Some(record) = consumer.try_recv() {
      return Some(record);
    }
    attempt += 1;
    if attempt < budget.attempts() {
      core::hint::spin_loop();
    }
  }
  None
}

/// Take at most `max` records into `out`, stopping at the limit or at the first
/// empty read, whichever comes first.
///
/// The bound is the point. `Consumer::try_recv_batch` takes whatever is there,
/// which on a busy ring is however much a producer happened to publish. A tick
/// has no control over that number. `max` is the caller's own ceiling. Without
/// it the call's cost is finite; with it the cost is predictable.
///
/// # Not counted by a `Tick`
///
/// As with [`push_within`], a `Tick` counts only what goes through
/// [`Tick::drain`].
///
/// ```
/// use ring_config::RingConfig;
/// use ring_core::Ring;
/// use ring_poll::drain_up_to;
///
/// let mut ring : Ring< u32 > = Ring::new( &RingConfig::new( 8 ).unwrap() ).unwrap();
/// let mut ends = ring.ends();
/// let ( mut producer, mut consumer ) = ends.split();
/// producer.try_push_batch( &mut ( 0..5 ) );
///
/// let mut out = Vec::new();
/// assert_eq!( drain_up_to( &mut consumer, &mut out, 3 ), 3 );
/// assert_eq!( consumer.len(), 2, "the limit held" );
/// ```
pub fn drain_up_to<T: Send>(consumer: &mut Consumer<'_, T>, out: &mut Vec<T>, max: usize) -> usize {
  let mut taken = 0;
  while taken < max {
    match consumer.try_recv() {
      Some(record) => {
        out.push(record);
        taken += 1;
      }
      None => break,
    }
  }
  taken
}

// ── Tick ──────────────────────────────────────────────────────────────────

/// One system's turn on the ring: a budget, a running count of what moved, and
/// a running count of what was destroyed getting there.
///
/// Every method delegates to the free function of the same shape and adds only
/// the accounting, so there is one implementation of each operation and one
/// place a retry rule can be got wrong.
///
/// # One frame, then [`reset`]
///
/// A tick is a per-frame object and nothing about the type enforces that. It
/// has no `Drop`, and no method takes `self` by value, so a tick hoisted out of
/// a per-frame loop keeps accumulating and reports [`Progress::Made`] forever.
/// That permanently silences a scheduler that backs off on
/// [`Progress::None`]. Call [`Tick::reset`] at the frame boundary, or build a
/// new tick; the first keeps the budget, the second recomputes it.
///
/// [`reset`]: Tick::reset
///
/// # Deliberately not `Copy`
///
/// `Tick` holds a counter that four `&mut self` methods increment. Under `Copy`,
/// a by-value use such as the ordinary signature `fn run_systems( tick : Tick )`
/// silently duplicates it. The callee's increments land in the copy and the
/// caller reads [`Progress::None`] after real work was done. `Tick` keeps `Clone`
/// because duplicating a tick on purpose is a legitimate thing to want, but the
/// duplication has to be written down.
///
/// # The accounting is bypassable
///
/// [`Tick::budget`] is public and so are the free functions, so
/// `push_within( &mut producer, record, tick.budget() )` spends this tick's
/// budget, publishes the record, and leaves `moved` untouched. `moved` is
/// private with no setter, so nothing can repair the count afterwards. That is
/// the price of keeping the free layer usable without a `Tick` at all; all this
/// type can do about it is say so.
///
/// ```
/// use ring_config::RingConfig;
/// use ring_core::Ring;
/// use ring_poll::{ Budget, Tick };
///
/// let mut ring : Ring< u32 > = Ring::new( &RingConfig::new( 8 ).unwrap() ).unwrap();
/// let mut ends = ring.ends();
/// let ( mut producer, mut consumer ) = ends.split();
///
/// let mut tick = Tick::new( Budget::once() );
/// tick.push( &mut producer, 1 ).unwrap();
/// tick.push( &mut producer, 2 ).unwrap();
/// assert_eq!( tick.recv( &mut consumer ), Some( 1 ) );
/// assert_eq!( tick.progress().count(), 3, "two published and one taken" );
/// ```
#[derive(Debug, Clone)]
pub struct Tick {
  budget: Budget,
  moved: usize,
  lost: usize,
}

impl Tick {
  /// A tick that will spend at most `budget` attempts on each operation.
  #[must_use]
  pub const fn new(budget: Budget) -> Self {
    Self {
      budget,
      moved: 0,
      lost: 0,
    }
  }

  /// Clear both counters at the frame boundary, keeping the budget.
  ///
  /// The budget is fixed for a tick's life. There is no setter, and this does
  /// not add one. A scheduler that wants to vary the budget between frames
  /// builds a new tick, which is the same cost as this call plus recomputing
  /// the budget it wanted to change anyway.
  pub const fn reset(&mut self) {
    self.moved = 0;
    self.lost = 0;
  }

  /// The budget each of this tick's operations is held to.
  #[must_use]
  pub const fn budget(&self) -> Budget {
    self.budget
  }

  /// What this tick has moved so far, across every operation on it.
  ///
  /// This counts arrivals and says nothing about cost. A tick that published
  /// four records and destroyed three getting there reports `Made( 4 )`; the
  /// three are in [`Tick::lost`].
  #[must_use]
  pub const fn progress(&self) -> Progress {
    Progress::of(self.moved)
  }

  /// How many records this tick consumed from an iterator and did not publish.
  ///
  /// Only [`Tick::push_batch`] can produce a loss. `try_push_batch` pulls a
  /// record before it can know whether the ring has room, and drops it when the
  /// answer is no. Every other operation either publishes or hands the record
  /// back. A caller reading [`Tick::progress`] alone sees what arrived and not
  /// what it cost.
  #[must_use]
  pub const fn lost(&self) -> usize {
    self.lost
  }

  /// [`push_within`] against this tick's budget, counting a success.
  ///
  /// # Errors
  ///
  /// Returns the record when the budget ran out with the ring still full.
  pub fn push<T: Send>(&mut self, producer: &mut Producer<'_, T>, record: T) -> Result<(), T> {
    let outcome = push_within(producer, record, self.budget);
    if outcome.is_ok() {
      self.moved += 1;
    }
    outcome
  }

  /// [`push_batch_within`] against this tick's budget, counting what moved and
  /// what it cost.
  ///
  /// The free function cannot report the loss. It returns one number, and the
  /// records it consumed and dropped are not in it. This method counts the
  /// iterator on the way past, so `offered - moved` is exactly what the ring
  /// refused after the record had already been taken. That difference lands in
  /// [`Tick::lost`], never in the returned count and never in
  /// [`Tick::progress`].
  pub fn push_batch<T: Send>(&mut self, producer: &mut Producer<'_, T>, records: &mut impl Iterator<Item = T>) -> usize {
    let mut offered = 0;
    let moved = {
      let mut counted = records.by_ref().inspect(|_| offered += 1);
      push_batch_within(producer, &mut counted, self.budget)
    };
    debug_assert!(
      offered >= moved,
      "the batch helper published {moved} of {offered} records it was handed"
    );
    self.moved += moved;
    self.lost += offered - moved;
    moved
  }

  /// [`recv_within`] against this tick's budget, counting a success.
  pub fn recv<T: Send>(&mut self, consumer: &mut Consumer<'_, T>) -> Option<T> {
    let outcome = recv_within(consumer, self.budget);
    if outcome.is_some() {
      self.moved += 1;
    }
    outcome
  }

  /// [`drain_up_to`] with this tick's own ceiling, counting what moved.
  ///
  /// The ceiling is `max` rather than the budget. A budget bounds *retries of a
  /// failed operation*; a drain limit bounds *successes*. Collapsing the two
  /// would make `Budget::once()` mean "take at most one record per tick", which
  /// is not what a single attempt means anywhere else here.
  pub fn drain<T: Send>(&mut self, consumer: &mut Consumer<'_, T>, out: &mut Vec<T>, max: usize) -> usize {
    let moved = drain_up_to(consumer, out, max);
    self.moved += moved;
    moved
  }
}

impl Default for Tick {
  /// A tick on [`Budget::once`].
  fn default() -> Self {
    Self::new(Budget::once())
  }
}
