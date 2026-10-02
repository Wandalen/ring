//! Wait strategies for space and data availability.
//!
//! Part of the ring family's concurrency write path.
//!
//! `ring_types::WaitKind` holds the four discriminants; this crate holds the
//! four handlers. That split is why `ring_types` can say "no ring logic" and
//! mean it. A `WaitKind` is a configuration value that travels through a
//! `RingConfig` and into a struct field without dragging a thread parking
//! implementation behind it.
//!
//! ## What a wait strategy is
//!
//! One question, asked repeatedly until it answers yes or the caller gives up:
//! *is the thing I am waiting for available?* [`wait_until`] is that loop, and
//! the [`WaitKind`] only decides what happens **between** two askings.
//!
//! The caller supplies everything else as a closure: which cursor, which
//! threshold, whether this is a producer waiting for space or a consumer
//! waiting for data.
//!
//! This crate's own description names both sides, and [`for_space`] and
//! [`for_data`] supply them. Each is one line over [`wait_until`], and that is
//! the point. They are two names for the two questions a caller asks, over
//! exactly one loop, so the retry budget, the pause behaviour, and the give-up
//! condition cannot drift apart between producer and consumer. Two full
//! implementations would be the same loop written twice, which is how a family
//! acquires two wait strategies that disagree under load and agree in tests.
//!
//! ## Why `WaitKind::None` is the variant that matters
//!
//! The wait-kind feature requires all four, but names `None` as the one the
//! tick path cannot do without. A tick has a deadline; a strategy that might
//! block has already missed it. [`wait_until`] under `None` evaluates the
//! predicate exactly once and returns. That is no wait at all, rather than a
//! wait with a very short timeout, and
//! [`ring_types::WaitKind::is_non_blocking`] is true for exactly that variant.
//!
//! ## Why every wait is bounded
//!
//! [`wait_until`] takes a `spins` budget and gives up. An unbounded wait on a
//! ring whose producer has died is a hung thread with no diagnostic, and the
//! `Park` strategy makes it a hung thread that never even burns CPU to show it.
//! Returning [`RingError::Empty`] hands the caller a decision it can act on.

#![deny(missing_docs)]

use ring_cursor::CursorPair;
use ring_types::{RingError, WaitKind};

/// The default number of attempts [`wait_until`] makes before giving up.
///
/// Deliberately a plain count rather than a duration. A duration would make
/// the same call take a different number of samples on different hardware,
/// which turns a reproducible test into a flaky one, and this family's whole
/// output is a measured verdict.
///
/// ```
/// assert_eq!( ring_wait::DEFAULT_SPINS, 1024 );
/// ```
pub const DEFAULT_SPINS: usize = 1024;

/// How many times a [`WaitKind`] re-reads before it is worth escalating to a
/// more expensive strategy.
///
/// [`WaitKind::Spin`] burns a core, which is right for a wait measured in
/// nanoseconds and wrong for one measured in milliseconds. A caller that knows
/// its own latency budget picks the strategy; this is the hint for one that
/// does not.
///
/// ```
/// use ring_types::WaitKind;
/// use ring_wait::escalation_hint;
///
/// assert_eq!( escalation_hint( WaitKind::Spin ), Some( WaitKind::Yield ) );
/// assert_eq!( escalation_hint( WaitKind::Yield ), Some( WaitKind::Park ) );
/// assert_eq!( escalation_hint( WaitKind::Park ), None, "nothing cheaper to escalate to" );
/// assert_eq!( escalation_hint( WaitKind::None ), None, "None never waits, so never escalates" );
/// ```
#[must_use]
pub const fn escalation_hint(kind: WaitKind) -> Option<WaitKind> {
  match kind {
    WaitKind::Spin => Some(WaitKind::Yield),
    WaitKind::Yield => Some(WaitKind::Park),
    WaitKind::Park | WaitKind::None => None,
  }
}

/// Do whatever this strategy does between two readings of a cursor.
///
/// The whole behavioural difference between the four variants is in this one
/// function; [`wait_until`] is the same loop for all of them.
///
/// `attempt` is the zero-based index of the pause about to happen, so a
/// strategy can behave differently early and late. [`WaitKind::Spin`] uses it
/// to emit a CPU pause hint rather than a bare busy loop.
///
/// Returns whether the caller should try again at all: `false` for
/// [`WaitKind::None`], which is the non-blocking variant and must not loop.
///
/// ```
/// use ring_types::WaitKind;
/// use ring_wait::pause;
///
/// assert!( pause( WaitKind::Spin, 0 ), "spin says keep going" );
/// assert!( !pause( WaitKind::None, 0 ), "None says stop after the first look" );
/// ```
pub fn pause(kind: WaitKind, attempt: usize) -> bool {
  match kind {
    WaitKind::Spin => {
      // A pause hint rather than an empty loop body. It tells the CPU this is
      // a spin-wait, which cuts the memory-order-violation penalty on leaving
      // the loop and stops the core from starving its hyperthread sibling.
      for _ in 0..=(attempt % 8) {
        core::hint::spin_loop();
      }
      true
    }
    WaitKind::Yield => {
      std::thread::yield_now();
      true
    }
    WaitKind::Park => {
      // Sleeping rather than `thread::park` on purpose. Parking requires the
      // publisher to hold the waiter's handle and unpark it, which is a
      // registration relationship this crate deliberately does not have.
      // `ring_handle` owns who-knows-whom. A short sleep is the same
      // cost profile (idle rather than spinning) without inventing that
      // relationship here, and the sleep length is what a real unpark would
      // make unnecessary.
      std::thread::sleep(std::time::Duration::from_micros(50));
      true
    }
    WaitKind::None => false,
  }
}

/// Ask `ready` until it answers true, pausing per `kind` between askings, for
/// at most `spins` attempts.
///
/// `ready` is evaluated at least once under every strategy including
/// [`WaitKind::None`]. The non-blocking variant returns *whatever is
/// available*, which requires looking.
///
/// # Errors
///
/// [`RingError::Empty`] when the budget is exhausted without `ready` answering
/// true. The caller decides what that means: a consumer treats it as "nothing
/// yet", a producer as "no room yet", and neither is this crate's business.
///
/// ```
/// use ring_types::WaitKind;
/// use ring_wait::wait_until;
///
/// let mut looks = 0;
/// let outcome = wait_until( WaitKind::Spin, 16, ||
/// {
///   looks += 1;
///   looks == 3
/// } );
///
/// assert!( outcome.is_ok() );
/// assert_eq!( looks, 3, "stopped as soon as it was ready" );
/// ```
///
/// # Panics
///
/// Never. The budget is a `usize` count and the loop is bounded by it.
pub fn wait_until<F>(kind: WaitKind, spins: usize, mut ready: F) -> Result<usize, RingError>
where
  F: FnMut() -> bool,
{
  for attempt in 0..spins.max(1) {
    if ready() {
      return Ok(attempt);
    }
    if !pause(kind, attempt) {
      break;
    }
  }
  Err(RingError::Empty)
}

/// [`wait_until`] with the [`DEFAULT_SPINS`] budget.
///
/// # Errors
///
/// [`RingError::Empty`], as [`wait_until`].
///
/// ```
/// use ring_types::WaitKind;
/// use ring_wait::wait;
///
/// assert_eq!( wait( WaitKind::None, || true ), Ok( 0 ) );
/// assert!( wait( WaitKind::None, || false ).is_err() );
/// ```
pub fn wait<F>(kind: WaitKind, ready: F) -> Result<usize, RingError>
where
  F: FnMut() -> bool,
{
  wait_until(kind, DEFAULT_SPINS, ready)
}

/// Wait until the ring has room for a producer to claim.
///
/// # Errors
///
/// [`RingError::Full`] when the budget runs out with the ring still full.
/// Note the error, which is the one difference from [`for_data`]. The two
/// questions have the same shape and opposite failures, and a producer handed
/// `Empty` would read it as "nothing to do" rather than "back-pressure".
///
/// ```
/// use core::sync::atomic::Ordering;
/// use ring_cursor::SeqCell;
/// use ring_cursor::CursorPair;
/// use ring_types::{ Capacity, Seq, WaitKind };
/// use ring_wait::for_space;
///
/// let pair = CursorPair::new( Capacity::new( 4 ).unwrap() );
/// assert!( for_space( &pair, WaitKind::None, 1 ).is_ok(), "an empty ring has room" );
///
/// pair.producer().store( Seq( 4 ), Ordering::Release );
/// assert!( for_space( &pair, WaitKind::None, 1 ).is_err(), "a full one does not" );
/// ```
pub fn for_space(pair: &CursorPair, kind: WaitKind, spins: usize) -> Result<usize, RingError> {
  wait_until(kind, spins, || pair.may_claim()).map_err(|_| RingError::Full)
}

/// Wait until the ring holds at least `count` unread items.
///
/// # Errors
///
/// [`RingError::Empty`] when the budget runs out with fewer than `count`
/// pending.
///
/// ```
/// use core::sync::atomic::Ordering;
/// use ring_cursor::SeqCell;
/// use ring_cursor::CursorPair;
/// use ring_types::{ Capacity, Seq, WaitKind };
/// use ring_wait::for_data;
///
/// let pair = CursorPair::new( Capacity::new( 8 ).unwrap() );
/// assert!( for_data( &pair, 1, WaitKind::None, 1 ).is_err(), "nothing published yet" );
///
/// pair.producer().store( Seq( 3 ), Ordering::Release );
/// assert!( for_data( &pair, 3, WaitKind::None, 1 ).is_ok() );
/// assert!( for_data( &pair, 4, WaitKind::None, 1 ).is_err(), "three is not four" );
/// ```
pub fn for_data(pair: &CursorPair, count: u64, kind: WaitKind, spins: usize) -> Result<usize, RingError> {
  wait_until(kind, spins, || pair.pending() >= count)
}
