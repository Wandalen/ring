//! Publisher stop, drain, and waiter join.
//!
//! Part of the ring family's concurrency write path.
//!
//! The family's shutdown feature asks for three operations: close, drain-all,
//! and reset. None of them is hard alone. The hard part is that **drain-all
//! only terminates because close came first.** A drain loop against an open
//! ring with a live producer never ends; the same loop after a close ends as
//! soon as the in-flight publishes land.
//!
//! So this crate enforces the ordering with *types*. [`Shutdown::close`]
//! returns a [`Stopped`] token, and `drain_all` and `discard_all` are methods
//! on that token rather than free functions. There is no way to spell a drain
//! that did not follow a close. [`Stopped::reopen`] consumes the token it is
//! called on, so a drain on *that* token cannot follow its reopen. A token from
//! an earlier `close` is a separate value and survives the reopen, so a drain
//! after a reopen still compiles through it.
//!
//! One rule is still convention. Nothing forces a producer to consult the flag.
//! [`Shutdown::guard`] is the mitigation. A [`Guarded`] producer checks before
//! every push and cannot skip the check. A caller holding a raw `ring_core::Producer`,
//! though, publishes into a closed ring without complaint. That is the one
//! guarantee this crate makes by convention rather than by construction, and it is
//! why `drain_all` terminates *eventually* rather than *immediately*. See
//! `docs/decisions/001_stopped_tokens_and_into_inner_wait_for_a_real_caller.md`.
//!
//! Acceptance is binary and lives in a test. The feature is Reached when a
//! closed ring refuses a guarded push, a drain after close recovers every
//! record that was published, and a reset ring accepts a full capacity again
//! and delivers it in order. It also requires `tests/shutdown_test.rs` to cite
//! the feature textually, which is the only crate→feature edge the family
//! records.

#![deny(missing_docs)]

use core::sync::atomic::{AtomicBool, Ordering};

use ring_core::{Consumer, Producer};
use ring_cursor::CursorPair;
use ring_types::{RingError, WaitKind};

/// The close flag, and the only piece of state this crate owns.
///
/// One `AtomicBool` shared by reference. `ring_core` deliberately has no such
/// flag. A handle carrying its own copy of liveness is the failure this crate
/// exists to prevent, so there is exactly one, here.
///
/// ```
/// use ring_shutdown::Shutdown;
///
/// let shutdown = Shutdown::new();
/// assert!( !shutdown.is_closed() );
///
/// let stopped = shutdown.close();
/// assert!( shutdown.is_closed() );
///
/// stopped.reopen();
/// assert!( !shutdown.is_closed() );
/// ```
#[derive(Debug)]
pub struct Shutdown {
  closed: AtomicBool,
}

impl Shutdown {
  /// A new, open shutdown.
  #[must_use]
  pub const fn new() -> Self {
    Self {
      closed: AtomicBool::new(false),
    }
  }

  /// Whether the ring has been closed to further publication.
  ///
  /// `Acquire`, so a reader that sees `true` also sees everything the closing
  /// thread wrote before closing.
  #[must_use]
  pub fn is_closed(&self) -> bool {
    self.closed.load(Ordering::Acquire)
  }

  /// Stop accepting publications, and get the token that permits draining.
  ///
  /// Idempotent. Closing an already-closed shutdown is not an error. It
  /// returns an equal but distinct token, constructed fresh on every call
  /// rather than cached from the first close. That matters because more than
  /// one path often leads to teardown (the normal end of a run, and a panic
  /// unwinding through a guard), and neither path should have to know whether
  /// it is first.
  ///
  /// ```
  /// use ring_shutdown::Shutdown;
  ///
  /// let shutdown = Shutdown::new();
  /// let _ = shutdown.close();
  /// let _ = shutdown.close();         // no complaint
  /// assert!( shutdown.is_closed() );
  /// ```
  #[must_use = "a Stopped is the only route to a drain; bind it, or bind `_` to close and nothing else"]
  pub fn close(&self) -> Stopped<'_> {
    self.closed.store(true, Ordering::Release);
    Stopped { shutdown: self }
  }

  /// Whether a publish may proceed.
  ///
  /// The check [`Guarded`] performs, exposed for a caller who holds a raw
  /// producer and wants to perform it explicitly.
  ///
  /// # Errors
  ///
  /// [`RingError::Closed`] once closed. It is deliberately not transient.
  /// Retrying cannot clear it; only [`Stopped::reopen`] can.
  ///
  /// ```
  /// use ring_shutdown::Shutdown;
  /// use ring_types::RingError;
  ///
  /// let shutdown = Shutdown::new();
  /// assert_eq!( shutdown.admit(), Ok( () ) );
  ///
  /// let _ = shutdown.close();
  /// assert_eq!( shutdown.admit(), Err( RingError::Closed ) );
  /// assert!( !RingError::Closed.is_transient() );
  /// ```
  pub fn admit(&self) -> Result<(), RingError> {
    if self.is_closed() { Err(RingError::Closed) } else { Ok(()) }
  }

  /// Wrap a producer so that every push consults this flag first.
  ///
  /// The wrapper is the difference between a rule and a guarantee. A caller
  /// holding a [`Guarded`] cannot publish into a closed ring, because the only
  /// push it has performs the check.
  pub const fn guard<'a, T>(&'a self, producer: Producer<'a, T>) -> Guarded<'a, T> {
    Guarded {
      producer,
      shutdown: self,
    }
  }
}

impl Default for Shutdown {
  fn default() -> Self {
    Self::new()
  }
}

/// Proof that a ring is closed, and the only route to a drain.
///
/// It holds a reference to the [`Shutdown`] it came from, so it cannot outlive it.
/// [`Stopped::reopen`] takes `self` by value: it consumes *the token it is
/// called on*, and a drain written against that specific token no longer
/// compiles afterward. A `Stopped` obtained from an earlier [`Shutdown::close`]
/// call is a distinct value and outlives this one's reopen.
#[derive(Debug)]
pub struct Stopped<'a> {
  shutdown: &'a Shutdown,
}

impl<'a> Stopped<'a> {
  /// The shutdown this token proves closed.
  ///
  /// # What this hands out
  ///
  /// `Shutdown::close` takes `&self`, so this accessor gives more than read
  /// access to a flag. It gives the capability to mint another `Stopped`.
  /// `Stopped` derives only `Debug` so it cannot be duplicated, yet
  /// `stopped.shutdown().close()` duplicates it anyway, from a *shared* borrow
  /// that leaves the first token alive.
  ///
  /// The token therefore proves the ring was closed at some point, not that it
  /// is closed now and not that this is the only proof outstanding. What makes
  /// a token prove a *current* fact is scarcity, and a `&self` constructor
  /// cannot supply scarcity. See
  /// `docs/decisions/001_stopped_tokens_and_into_inner_wait_for_a_real_caller.md`.
  #[must_use]
  pub const fn shutdown(&self) -> &'a Shutdown {
    self.shutdown
  }

  /// Move every remaining record out of the ring and into `out`.
  ///
  /// Returns how many were recovered. Loops until a batch comes back empty,
  /// which terminates because publication has stopped. The crate docs describe
  /// the one case where it does not.
  ///
  /// ```
  /// use ring_config::RingConfig;
  /// use ring_core::Ring;
  /// use ring_shutdown::Shutdown;
  ///
  /// let mut ring : Ring< u32 > = Ring::new( &RingConfig::new( 8 ).unwrap() ).unwrap();
  /// let mut ends = ring.ends();
  /// let ( mut producer, mut consumer ) = ends.split();
  ///
  /// producer.try_push_batch( &mut [ 1, 2, 3 ].into_iter() );
  ///
  /// let shutdown = Shutdown::new();
  /// let stopped = shutdown.close();
  ///
  /// let mut recovered = Vec::new();
  /// assert_eq!( stopped.drain_all( &mut consumer, &mut recovered ), 3 );
  /// assert_eq!( recovered, [ 1, 2, 3 ] );
  /// ```
  pub fn drain_all<T: Send>(&self, consumer: &mut Consumer<'_, T>, out: &mut Vec<T>) -> usize {
    // A `while` rather than a `loop` with an inner `return`. The choice is for
    // coverage measurement, not style. `llvm-cov` opens a region on a bare
    // `loop` line and never attributes a hit to it, so the line reads as
    // uncovered however hard the tests drain. Measured at 80/81 with `loop`
    // and 81/81 with this, for the identical suite, `cargo tarpaulin --engine
    // llvm`. Recorded in `tests/manual/readme.md` D2.
    let mut total = 0;
    let mut taken = consumer.try_recv_batch(out);
    while taken > 0 {
      total += taken;
      taken = consumer.try_recv_batch(out);
    }
    total
  }

  /// Drain like [`Stopped::drain_all`], but give up after `budget` batches.
  ///
  /// [`Stopped::drain_all`] terminates because publication has stopped, and
  /// that holds for every *guarded* producer. A caller holding a raw
  /// `ring_core::Producer` can publish into a closed ring. Refusing that
  /// publish is the one guarantee this crate makes by convention rather than by
  /// construction, and against such a producer the unbounded loop does not end.
  /// This situation is the expensive one, because a hang is indistinguishable
  /// from a slow drain forever. A budget turns the hang into a return value.
  ///
  /// Records already moved stay in `out` whichever way this returns, so a
  /// caller who gets `Err` recovers the count as the growth of `out`. The
  /// budget bounds the work and never discards what the work achieved.
  ///
  /// # Errors
  ///
  /// [`RingError::Empty`] when `budget` consecutive batches all came back
  /// non-empty. This is `ring_wait`'s budget-exhausted report, which [`wait_for_close`]
  /// already makes for the same reason on the other half of this crate.
  ///
  /// ```
  /// use ring_config::RingConfig;
  /// use ring_core::Ring;
  /// use ring_shutdown::Shutdown;
  ///
  /// let mut ring : Ring< u32 > = Ring::new( &RingConfig::new( 8 ).unwrap() ).unwrap();
  /// let mut ends = ring.ends();
  /// let ( mut producer, mut consumer ) = ends.split();
  ///
  /// producer.try_push_batch( &mut [ 1, 2, 3 ].into_iter() );
  ///
  /// let shutdown = Shutdown::new();
  /// let stopped = shutdown.close();
  ///
  /// let mut recovered = Vec::new();
  /// assert_eq!( stopped.drain_all_bounded( &mut consumer, &mut recovered, 4 ), Ok( 3 ) );
  /// assert_eq!( recovered, [ 1, 2, 3 ] );
  /// ```
  pub fn drain_all_bounded<T: Send>(
    &self,
    consumer: &mut Consumer<'_, T>,
    out: &mut Vec<T>,
    budget: usize,
  ) -> Result<usize, RingError> {
    let mut total = 0;
    // `budget.max( 1 )` matches `ring_wait::wait_until`'s reading of its own
    // `spins`: a budget of zero means one attempt, not none, so a caller who
    // computes the number cannot accidentally ask for no work at all.
    for _ in 0..budget.max(1) {
      let taken = consumer.try_recv_batch(out);
      if taken == 0 {
        return Ok(total);
      }
      total += taken;
    }
    Err(RingError::Empty)
  }

  /// Drop every remaining record, returning how many were dropped.
  ///
  /// The teardown counterpart of [`Stopped::drain_all`], for a caller who
  /// needs the ring empty rather than the records. It drops each record as it
  /// takes it, so a `T` with a `Drop` impl still runs it.
  pub fn discard_all<T: Send>(&self, consumer: &mut Consumer<'_, T>) -> usize {
    let mut total = 0;
    while let Some(record) = consumer.try_recv() {
      drop(record);
      total += 1;
    }
    total
  }

  /// Accept publications again, consuming the token.
  ///
  /// Taking `self` by value is the point. A drain is only sound while the ring
  /// is closed, so the proof that it is closed must not survive reopening.
  pub fn reopen(self) {
    self.shutdown.closed.store(false, Ordering::Release);
  }
}

/// Why a guarded push was refused, carrying the record back.
///
/// Two arms rather than a `RingError`, because the record must come back
/// intact in both. That is `ring_core`'s own refusal contract, extended by one
/// case. A producer acts on the distinction. [`Refusal::Full`] clears when the
/// consumer drains; [`Refusal::Closed`] never does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal<T> {
  /// The ring had no free slot. Retry after the consumer drains.
  Full(T),
  /// The ring is closed. Retrying cannot help.
  Closed(T),
}

impl<T> Refusal<T> {
  /// The record that was not published.
  ///
  /// The attribute guards against a real loss. This is the only method on the
  /// crate whose return value is a *payload* rather than a fact. The other
  /// eight `must_use` items answer a question, and discarding one of those
  /// wastes a computation. Discarding this one destroys the only copy of a
  /// record that has already left the ring and the caller's hands. One
  /// statement causes the exact loss [`Refusal`] exists to prevent.
  #[must_use = "this is the record itself, not a copy — dropping it loses it"]
  pub fn into_record(self) -> T {
    match self {
      Self::Full(record) | Self::Closed(record) => record,
    }
  }

  /// Whether the refusal was a close rather than back-pressure.
  // Fix(refusal_is_closed_classification_not_exhaustive): was `matches!( self,
  //   Self::Closed( _ ) )`, so a third `Refusal` variant would silently read
  //   `false`, grouped with back-pressure, with nothing forcing a second
  //   look. `into_record` and `reason` just below already carry the
  //   exhaustive shape.
  // Root cause: `matches!` over a single named variant answers every variant
  //   it was not told about with the same default, compiling cleanly however
  //   many variants `Refusal` gains.
  // Pitfall: a producer deciding "retry, or give up?" on this predicate would
  //   retry against a refusal that can never clear, the exact distinction
  //   [`Refusal`]'s own doc says this type exists to preserve.
  #[must_use]
  pub const fn is_closed(&self) -> bool {
    match self {
      Self::Closed(_) => true,
      Self::Full(_) => false,
    }
  }

  /// The equivalent [`RingError`], for a caller reporting rather than retrying.
  #[must_use]
  pub const fn reason(&self) -> RingError {
    match self {
      Self::Full(_) => RingError::Full,
      Self::Closed(_) => RingError::Closed,
    }
  }
}

/// A producer that consults a [`Shutdown`] before every push.
///
/// ```
/// use ring_config::RingConfig;
/// use ring_core::Ring;
/// use ring_shutdown::Shutdown;
///
/// let mut ring : Ring< u32 > = Ring::new( &RingConfig::new( 4 ).unwrap() ).unwrap();
/// let mut ends = ring.ends();
/// let ( producer, _consumer ) = ends.split();
///
/// let shutdown = Shutdown::new();
/// let mut guarded = shutdown.guard( producer );
///
/// assert!( guarded.try_push( 7 ).is_ok() );
///
/// let _ = shutdown.close();
/// assert!( guarded.try_push( 8 ).unwrap_err().is_closed() );
/// ```
#[derive(Debug)]
pub struct Guarded<'a, T> {
  producer: Producer<'a, T>,
  shutdown: &'a Shutdown,
}

impl<'a, T: Send> Guarded<'a, T> {
  /// Publish one record, or hand it back with the reason.
  ///
  /// # Errors
  ///
  /// [`Refusal::Closed`] when the shutdown is closed, [`Refusal::Full`] when
  /// the ring has no room. The record comes back in both cases.
  ///
  /// **`Full` is unreachable only under `OverflowPolicy::DropNewest`.** That
  /// policy's contract is to discard the record and report success, so
  /// under the default a full push returns `Ok` and the record is gone. The
  /// guard does not change that and could not. Refusing where the policy
  /// says discard would be a different policy. `Full` is reachable under
  /// `Fail`, and also under `DropOldest` on a build without the `crossbeam`
  /// feature (`ring_core`'s own default). Eviction is a `crossbeam`-gated
  /// code path; without it, `DropOldest` refuses instead of evicting.
  pub fn try_push(&mut self, record: T) -> Result<(), Refusal<T>> {
    if self.shutdown.is_closed() {
      return Err(Refusal::Closed(record));
    }
    self.producer.try_push(record).map_err(Refusal::Full)
  }

  /// Publish from `records` until one is refused, returning how many landed.
  ///
  /// Stops at the first refusal of either kind. A closed ring accepts nothing,
  /// so this returns `0` without consuming from the iterator. It checks before
  /// the first read.
  pub fn try_push_batch(&mut self, records: &mut impl Iterator<Item = T>) -> usize {
    if self.shutdown.is_closed() {
      return 0;
    }
    self.producer.try_push_batch(records)
  }

  /// Room in the ring, binding at SPSC and advisory elsewhere, per
  /// `ring_core`'s own split contract. Guarding does not change that.
  #[must_use]
  pub fn free_capacity(&self) -> usize {
    self.producer.free_capacity()
  }

  /// Whether a push would be *impeded* right now, for either reason.
  ///
  /// Unlike `ring_core::Producer::is_full`, this is not `free_capacity() == 0`:
  /// a closed ring is blocked whatever its occupancy.
  ///
  /// **It does not predict a refusal.** Under `OverflowPolicy::DropNewest` a
  /// blocked-by-occupancy push still returns `Ok`, having discarded the
  /// record. Only the close half of this predicate implies a refusal; the
  /// occupancy half inherits `free_capacity`'s advisory contract at MPSC and
  /// crossbeam on top of that.
  #[must_use]
  pub fn is_blocked(&self) -> bool {
    self.shutdown.is_closed() || self.producer.is_full()
  }

  /// The shutdown this producer consults.
  ///
  /// # What this hands out
  ///
  /// The guard has two exits, and comparing them shows the framing is backwards.
  /// [`Guarded::into_inner`] is the documented one. It consumes the guard and
  /// yields **less** than the guard had, a producer with no flag. This one
  /// costs nothing, keeps the guard, and yields **more**. `Shutdown::close`
  /// takes `&self`, so what comes back is more than read access to a flag. It is
  /// the capability to close the ring, mint a [`Stopped`], and from it reach
  /// `drain_all`, `discard_all` and `reopen`, the consumer-side teardown
  /// operations, from a shared reference, in a `const fn`.
  ///
  /// [`Shutdown::guard`]'s promise survives exactly as worded. A `Guarded`
  /// holder still cannot publish into a closed ring. The promise does not say,
  /// but a reader takes away anyway, that holding a `Guarded` is the
  /// constrained position. On this path it is the unconstrained one.
  #[must_use]
  pub const fn shutdown(&self) -> &'a Shutdown {
    self.shutdown
  }

  /// Give up the guarantee and take the raw producer back.
  pub fn into_inner(self) -> Producer<'a, T> {
    self.producer
  }
}

/// What ended a close-aware wait.
///
/// The attribute is on the enum rather than only on the `Result` that carries
/// it, because `?` satisfies `Result`'s own `must_use`. Inside any
/// `Result`-returning function the reflex form is `for_space_or_close( .. )?;`.
/// There the `?` consumes the `Result` and leaves a bare `Wake` in statement
/// position. Dropping that `Wake` merges "room appeared" and "stop", which is
/// what this type was introduced to make unspellable.
#[must_use = "a Wake::Closed means stop, not publish"]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Wake {
  /// The condition the caller was waiting for became true, as of the instant
  /// this was observed. Like [`Wake::Closed`], this is a statement about a
  /// past instant, not a current guarantee. By the time the caller acts on
  /// it, the condition may no longer hold.
  Ready,
  /// The ring closed while waiting. The condition may still be false.
  Closed,
}

impl Wake {
  /// Whether the wait ended because the condition was met.
  // Fix(wake_is_ready_classification_not_exhaustive): was `matches!( self,
  //   Self::Ready )`, so a third `Wake` variant would silently read `false`,
  //   grouped with `Closed`, with nothing forcing a second look.
  // Root cause: `matches!` over a single named variant answers every variant
  //   it was not told about with the same default, compiling cleanly however
  //   many variants `Wake` gains.
  // Pitfall: this type's own doc names the exact failure mode a wrong default
  //   here would reintroduce. It would merge "room appeared" and "stop" back into
  //   one bare bool, which is what `Wake` was introduced to make unspellable.
  #[must_use]
  pub const fn is_ready(self) -> bool {
    match self {
      Self::Ready => true,
      Self::Closed => false,
    }
  }
}

/// Wait until the ring closes.
///
/// The waiter-join half of the shutdown feature: a thread parked on a ring's
/// progress needs a second reason to give up, or teardown blocks on it forever.
///
/// # Errors
///
/// [`RingError::Empty`] when the spin budget runs out with the ring still
/// open. This is `ring_wait`'s own budget-exhausted error, unchanged.
///
/// ```
/// use ring_shutdown::{ wait_for_close, Shutdown };
/// use ring_types::WaitKind;
///
/// let shutdown = Shutdown::new();
/// assert!( wait_for_close( &shutdown, WaitKind::None, 1 ).is_err() );
///
/// let _ = shutdown.close();
/// assert!( wait_for_close( &shutdown, WaitKind::None, 1 ).is_ok() );
/// ```
pub fn wait_for_close(shutdown: &Shutdown, kind: WaitKind, spins: usize) -> Result<usize, RingError> {
  ring_wait::wait_until(kind, spins, || shutdown.is_closed())
}

/// Wait for room to publish, giving up early if the ring closes.
///
/// This is `ring_wait::for_space` with a second exit. The return value says
/// which exit the wait took, so a producer can tell "room appeared" from
/// "stop". A bare `Ok` would merge those two outcomes.
///
/// # Errors
///
/// [`RingError::Full`] when the budget runs out with the ring neither closed
/// nor drained. This matches `ring_wait::for_space`, whose error a producer
/// reads as back-pressure rather than as "nothing to do".
///
/// ```
/// use ring_cursor::CursorPair;
/// use ring_shutdown::{ for_space_or_close, Shutdown, Wake };
/// use ring_types::{ Capacity, WaitKind };
///
/// let pair = CursorPair::new( Capacity::new( 4 ).unwrap() );
/// let shutdown = Shutdown::new();
///
/// assert_eq!( for_space_or_close( &pair, &shutdown, WaitKind::None, 1 ), Ok( Wake::Ready ) );
/// ```
pub fn for_space_or_close(pair: &CursorPair, shutdown: &Shutdown, kind: WaitKind, spins: usize) -> Result<Wake, RingError> {
  let mut closed = false;
  let outcome = ring_wait::wait_until(kind, spins, || {
    if shutdown.is_closed() {
      closed = true;
      return true;
    }
    pair.may_claim()
  });

  match outcome {
    // The close is reported even when room also appeared: a producer told to
    // stop must stop, and a `Ready` here would send it back to publish.
    Ok(_) if closed => Ok(Wake::Closed),
    Ok(_) => Ok(Wake::Ready),
    Err(_) => Err(RingError::Full),
  }
}

/// The whole teardown in one call: close, empty, and reopen.
///
/// Returns how many records were discarded. This is the operation a test
/// harness or a world recycle wants. Afterwards the ring behaves as a freshly
/// built one of the same capacity, on the same allocation.
///
/// Discarding rather than draining is deliberate. A reset whose records had to
/// go somewhere would need the caller to supply a sink at teardown, which is
/// the moment they least want one. A caller who wants the remainder calls
/// [`Shutdown::close`] and [`Stopped::drain_all`] and then reopens.
///
/// ```
/// use ring_config::RingConfig;
/// use ring_core::Ring;
/// use ring_shutdown::{ reset, Shutdown };
///
/// let mut ring : Ring< u32 > = Ring::new( &RingConfig::new( 4 ).unwrap() ).unwrap();
/// let mut ends = ring.ends();
/// let ( mut producer, mut consumer ) = ends.split();
///
/// producer.try_push_batch( &mut [ 1, 2, 3 ].into_iter() );
///
/// let shutdown = Shutdown::new();
/// assert_eq!( reset( &shutdown, &mut consumer ), 3 );
/// assert!( !shutdown.is_closed(), "reset leaves the ring open" );
/// assert_eq!( consumer.len(), 0 );
/// ```
pub fn reset<T: Send>(shutdown: &Shutdown, consumer: &mut Consumer<'_, T>) -> usize {
  let stopped = shutdown.close();
  let discarded = stopped.discard_all(consumer);
  stopped.reopen();
  discarded
}
