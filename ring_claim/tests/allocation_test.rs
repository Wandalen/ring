//! Claiming allocates nothing, asserted here instead of probed once.
//!
//! ## Why this file exists
//!
//! A requirements document once recorded, as a finding, that this crate's
//! headline no-allocation property was false. Every `headroom()` and every
//! `claim()` charged one heap allocation, two crates down, inside a `GatingSet`
//! gating read. The measurement behind that finding was a scratch binary
//! written into the document, run once by hand, and deleted by its own last
//! line. It was right, and it left nothing behind.
//!
//! When a later commit removed the allocation from `ring_cursor::slowest`, the
//! finding's numbers became false in the other direction, and again nothing
//! detected it, because a document is not a test. This file is that probe made
//! permanent: the same call shapes and the same counter, run by the suite.
//!
//! ## What this file measures that `ring_cursor`'s own file does not cover
//!
//! `ring_cursor/tests/allocation_test.rs` pins the fold. This one pins the
//! calls that finding was about, `headroom` and `claim`, including the two
//! shapes that distinguish this crate: the compare-exchange **success** path
//! and the **refusal** path. Each evaluates the gate once and was therefore one
//! allocation.
//!
//! It also keeps the ungated row that hid the bug for as long as it went
//! unnoticed. A `GatingSet` with no consumers has an empty cursor slice, an
//! empty `collect()` yields `Vec::new()`, and `Vec::new()` never reaches the
//! allocator. So every test using an ungated claimer measured zero and was
//! telling the truth about a case no real ring is in. The row stays on
//! purpose, labelled, so the next reader sees why a green measurement was
//! not evidence.
//!
//! ## Why a per-thread counter, and why the control arms
//!
//! Both for the reasons `ring_cursor/tests/allocation_test.rs` states at
//! length. Other threads allocate while this test runs, and a silently broken
//! counter reports zero for everything, which is the answer this file is
//! looking for.

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
use core::cell::Cell;
use core::sync::atomic::{AtomicBool, Ordering};

use ring_claim::Claimer;
use ring_gating::GatingSet;
use ring_types::{Capacity, RingError, Seq};

thread_local! {
  /// Allocation calls this thread has made since it started.
  static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };

  /// Bytes this thread has requested since it started.
  static BYTES: Cell<usize> = const { Cell::new(0) };
}

/// `std::alloc::System`, plus a tally.
///
/// This impl leaves `alloc_zeroed` and `realloc` at their trait defaults, both
/// of which route through `alloc`, so a single counter here sees every path a
/// `Vec` can take to the allocator.
struct Counting;

// SAFETY: every method forwards its arguments unchanged to
// `std::alloc::System`, which upholds the trait's contract. The two counter
// updates touch only this file's own thread-locals and never the allocation
// itself. Those thread-locals are `const`-initialised and have no destructor,
// so reaching them neither allocates nor re-enters this allocator.
unsafe impl GlobalAlloc for Counting {
  unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
    ALLOCATIONS.set(ALLOCATIONS.get() + 1);
    BYTES.set(BYTES.get() + layout.size());
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

/// Allocations and bytes this thread charged while `body` ran.
fn measure<R>(body: impl FnOnce() -> R) -> (usize, usize, R) {
  let calls_before = ALLOCATIONS.get();
  let bytes_before = BYTES.get();
  let value = body();
  let calls = ALLOCATIONS.get() - calls_before;
  let bytes = BYTES.get() - bytes_before;
  (calls, bytes, value)
}

fn cap(slots: usize) -> Capacity {
  Capacity::new(slots).expect("test capacities are powers of two")
}

#[test]
fn no_claim_and_no_gate_read_allocates() {
  // Every set is built before the first measurement. `GatingSet::new` owns its
  // cursors and allocates once, at construction. That allocation is real, is
  // not what this file is about, and must not be charged to a call below.
  let gated = GatingSet::new(cap(1024), 1);
  let claimer = Claimer::new(&gated);

  let tight = GatingSet::new(cap(4), 1);
  let exhausted = Claimer::new(&tight);

  let ungated = GatingSet::new(cap(64), 0);
  let ungated_claimer = Claimer::new(&ungated);

  // The control arm, first: if this reads zero the counter is not working and
  // every assertion below is vacuous.
  let (control_calls, _, buffer) = measure(|| Vec::<Seq>::with_capacity(8));
  assert!(
    control_calls >= 1,
    "a control that must allocate reported {control_calls} calls — the counting \
     allocator is not installed, so a zero from the measured calls below would \
     mean nothing"
  );
  drop(buffer);

  // The second control: an allocation made on another thread is not charged
  // here. libtest's main thread allocates while a test runs, and a counter
  // that charged those allocations to this thread would fail at random.
  let started = AtomicBool::new(false);
  let finished = AtomicBool::new(false);
  std::thread::scope(|scope| {
    scope.spawn(|| {
      while !started.load(Ordering::Acquire) {
        core::hint::spin_loop();
      }
      drop(core::hint::black_box(Vec::<Seq>::with_capacity(8)));
      finished.store(true, Ordering::Release);
    });
    let (calls, bytes, ()) = measure(|| {
      started.store(true, Ordering::Release);
      while !finished.load(Ordering::Acquire) {
        core::hint::spin_loop();
      }
    });
    assert_eq!(
      (calls, bytes),
      (0, 0),
      "another thread allocated during the measurement — the counter charged \
       this thread for allocations it never made"
    );
  });

  // The finding's first row: 1000 gate reads, which measured 1000 allocations.
  let (calls, bytes, _) = measure(|| {
    for _ in 0..1000 {
      core::hint::black_box(claimer.headroom());
    }
  });
  assert_eq!((calls, bytes), (0, 0), "headroom() ×1000");

  // The finding's second row, success path: 1000 claims that all fit, so every
  // call evaluates the gate once and then wins its compare-exchange.
  //
  // Every grant here is deliberately dropped. `Claim` is `#[ must_use ]`
  // because a range claimed and never published strands its slots. That is
  // true, and irrelevant to a measurement that never publishes anything and
  // never reuses the ring afterwards.
  let (calls, bytes, _) = measure(|| {
    for _ in 0..1000 {
      let _granted = claimer.claim(1).expect("1024 slots, 1000 claims of one");
    }
  });
  assert_eq!((calls, bytes), (0, 0), "claim( 1 ) ×1000, all granted");

  // The refusal path, which the finding never separated out and which a
  // contended producer spins on. The call reads the gate, finds it short, and
  // returns without a compare-exchange at all.
  let _fills_the_ring = exhausted.claim(4).expect("a fresh ring of four admits four");
  let (calls, bytes, answer) = measure(|| {
    let mut last = Ok(());
    for _ in 0..1000 {
      last = exhausted.claim(1).map(|_| ());
    }
    last
  });
  assert_eq!(answer, Err(RingError::Full), "the ring stays full");
  assert_eq!((calls, bytes), (0, 0), "claim( 1 ) ×1000, all refused");

  // The partial-grant sibling, whose loop condition reads the gate the same way.
  let (calls, bytes, answer) = measure(|| claimer.claim_up_to(8));
  assert_eq!(answer.map(|c| c.len()), Ok(8));
  assert_eq!((calls, bytes), (0, 0), "claim_up_to( 8 )");

  // The row that hid the bug. It read zero before the fix and reads zero now,
  // so it cannot tell the two states apart. That is the problem.
  let (calls, bytes, _) = measure(|| {
    for _ in 0..1000 {
      let _ = ungated_claimer.claim(1);
    }
  });
  assert_eq!(
    (calls, bytes),
    (0, 0),
    "claim( 1 ) ×1000 on an ungated set — green before the fix too, and therefore no evidence"
  );

  // `Claim::sequences` hands back an iterator; draining it must not materialise
  // the range, or a batching producer pays per claim for the convenience.
  let claim = claimer.claim(16).expect("room for sixteen");
  let (calls, bytes, count) = measure(|| claim.sequences().count());
  assert_eq!(count, 16);
  assert_eq!((calls, bytes), (0, 0), "Claim::sequences() drained");
}
