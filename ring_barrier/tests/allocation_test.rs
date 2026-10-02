//! Asserts, rather than argues, that every barrier operation allocates nothing.
//!
//! ## Why this file exists
//!
//! The allocation requirement used to open with a table of allocation counts
//! measured by hand, in a scratch binary, against a `ring_cursor::slowest` that
//! collected cursor positions into a `Vec` before folding them. Every row above
//! zero in that table came from that one `collect()`, one crate down. When it
//! was removed the table became false in every row at once, and nothing
//! detected it. The numbers lived only in prose, and prose does not run.
//!
//! This file replaces the scratch binary. The table it now backs is the same
//! table, measured the same way, by something that fails when the answer
//! changes.
//!
//! ## What is measured here that `ring_cursor`'s own file does not cover
//!
//! `ring_cursor/tests/allocation_test.rs` pins the fold. This one pins the
//! five call shapes *this* crate exposes over it, including the two that
//! multiply. `wait_for` reads the frontier once per spin, so a budget spent
//! against an unsatisfied predicate used to charge one allocation per
//! attempt. Ten thousand spins were ten thousand allocations, on the path
//! whose entire purpose is to wait cheaply. That multiplier is the reason this
//! crate needs its own measurement rather than inheriting `ring_cursor`'s.
//!
//! ## Why one `#[ test ]`, and why a control arm
//!
//! Both for the reasons `ring_cursor/tests/allocation_test.rs` states at
//! length: the counter is process-global, and a silently broken counter
//! reports zero for everything, which is the answer this file is looking
//! for.

// Ordinary tests, compiled out under `--cfg loom`. That cfg swaps
// `ring_atomic`'s atomics for loom's instrumented ones across the whole
// family, and those panic the moment they are touched outside a
// model closure that instruments them.
#![cfg(not(loom))]
// A counting allocator cannot be written in safe Rust, because `GlobalAlloc` is
// an unsafe trait by construction. The workspace denies `unsafe_code`, and the
// two crates that override it do so in `src/lib.rs`. This file is not one of
// them. The allowance here is test-only, covers one wrapper that forwards to
// `std::alloc::System` unchanged, and adds no unsafe code to any shipped crate.
#![allow(unsafe_code)]

use core::alloc::{GlobalAlloc, Layout};
use core::sync::atomic::{AtomicUsize, Ordering};

use ring_barrier::Barrier;
use ring_cursor::{PaddedCursor, SeqCell};
use ring_types::{RingError, Seq, WaitKind};

/// Allocation calls seen since the process started.
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

/// Bytes requested since the process started.
static BYTES: AtomicUsize = AtomicUsize::new(0);

/// `std::alloc::System`, plus a tally.
///
/// `alloc_zeroed` and `realloc` are left at their trait defaults, both of
/// which route through `alloc`, so a single counter here sees every path a
/// `Vec` can take to the allocator.
struct Counting;

// SAFETY: every method forwards its arguments unchanged to
// `std::alloc::System`, which upholds the trait's contract; the two
// `fetch_add` calls touch only this file's own statics and never the
// allocation itself.
unsafe impl GlobalAlloc for Counting {
  unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
    ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
    BYTES.fetch_add(layout.size(), Ordering::Relaxed);
    // SAFETY: `layout` is passed through untouched, so the caller's own
    // guarantee that it is non-zero-sized and well-formed still holds.
    unsafe { std::alloc::System.alloc(layout) }
  }

  unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
    // SAFETY: `ptr` and `layout` are passed through untouched, so the
    // caller's own guarantee that they describe a live allocation from this
    // allocator still holds.
    unsafe { std::alloc::System.dealloc(ptr, layout) }
  }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// Allocations and bytes charged while `body` ran.
fn measure<R>(body: impl FnOnce() -> R) -> (usize, usize, R) {
  let calls_before = ALLOCATIONS.load(Ordering::Relaxed);
  let bytes_before = BYTES.load(Ordering::Relaxed);
  let value = body();
  let calls = ALLOCATIONS.load(Ordering::Relaxed) - calls_before;
  let bytes = BYTES.load(Ordering::Relaxed) - bytes_before;
  (calls, bytes, value)
}

#[test]
fn every_barrier_operation_allocates_nothing() {
  // Built before the first measurement, so their own construction is never
  // charged to the operation under test.
  let none: [PaddedCursor; 0] = [];
  let one = [PaddedCursor::default()];
  let three = [
    PaddedCursor::new(Seq(12)),
    PaddedCursor::new(Seq(4)),
    PaddedCursor::new(Seq(9)),
  ];
  one[0].store(Seq(6), Ordering::Release);

  // The control arm comes first. If this reads zero, the counter is not
  // working and every assertion below is vacuous.
  let (control_calls, _, buffer) = measure(|| Vec::<Seq>::with_capacity(3));
  assert!(
    control_calls >= 1,
    "a control that must allocate reported {control_calls} calls — the counting \
     allocator is not installed, so a zero from the measured calls below would \
     mean nothing"
  );
  drop(buffer);

  // `over`, `len` and `is_empty` were already zero before the fold changed.
  // They are measured anyway, because a table with some rows measured and
  // others assumed is how the previous version of this document went stale.
  let (calls, bytes, barrier) = measure(|| Barrier::over(&three));
  assert_eq!((calls, bytes), (0, 0), "Barrier::over");

  let (calls, bytes, _) = measure(|| (barrier.len(), barrier.is_empty()));
  assert_eq!((calls, bytes), (0, 0), "len() + is_empty()");

  // The three rows that used to read 1 allocation of 8 bytes, one per
  // dependency read.
  let (calls, bytes, answer) = measure(|| Barrier::over(&one).frontier());
  assert_eq!(answer, Some(Seq(6)));
  assert_eq!((calls, bytes), (0, 0), "frontier() — 1 dependency");

  let (calls, bytes, answer) = measure(|| barrier.frontier());
  assert_eq!(answer, Some(Seq(4)), "the minimum is in the middle");
  assert_eq!((calls, bytes), (0, 0), "frontier() — 3 dependencies");

  let (calls, bytes, answer) = measure(|| Barrier::over(&none).frontier());
  assert_eq!(answer, None, "no dependencies, no frontier");
  assert_eq!((calls, bytes), (0, 0), "frontier() — 0 dependencies");

  let (calls, bytes, answer) = measure(|| barrier.available(Seq::ZERO));
  assert_eq!(answer, 4);
  assert_eq!((calls, bytes), (0, 0), "available( … )");

  let (calls, bytes, answer) = measure(|| barrier.admits(Seq::ZERO, 4));
  assert!(answer);
  assert_eq!((calls, bytes), (0, 0), "admits( … )");

  // A thousand reads, because the claim is per-call and a single zero is also
  // what a hoisted, cached allocation would report.
  let (calls, bytes, _) = measure(|| {
    for _ in 0..1000 {
      core::hint::black_box(barrier.frontier());
    }
  });
  assert_eq!((calls, bytes), (0, 0), "frontier() ×1000");

  // The two `wait_for` rows. The old table showed them as 2 and 10 000,
  // because `wait_for` reads the frontier once per spin plus once on success.
  let (calls, bytes, answer) = measure(|| Barrier::over(&one).wait_for(Seq::ZERO, 1, WaitKind::None, 1));
  assert_eq!(answer, Ok(Seq(6)), "satisfied on the first look");
  assert_eq!((calls, bytes), (0, 0), "wait_for( …, None, 1 ), satisfied at once");

  // Unsatisfiable, because the three-cursor barrier admits 4, never 5, so the
  // whole budget is spent. This is the row that used to cost 80 kB.
  let (calls, bytes, answer) = measure(|| barrier.wait_for(Seq::ZERO, 5, WaitKind::Spin, 10_000));
  assert_eq!(answer, Err(RingError::Empty), "the budget runs out");
  assert_eq!(
    (calls, bytes),
    (0, 0),
    "wait_for( …, Spin, 10_000 ), budget spent — 10 000 reads, 0 allocations"
  );
}
