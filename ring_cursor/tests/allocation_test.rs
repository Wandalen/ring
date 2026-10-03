//! Asserts, rather than argues, that `ring_cursor::slowest` allocates nothing.
//!
//! ## Why this file exists
//!
//! `slowest` used to read
//! `cursors.iter().map( | c | c.load( GATING ) ).collect::< Vec< _ > >()`
//! and hand the `Vec` to `ring_seqno::slowest`, because `ring_seqno` folds over
//! values and this crate holds cells. That cost one heap allocation on every
//! gate read, inside the CAS retry loop of `ring_claim::claim`. Six crates
//! documented it as a measured fact before it was removed.
//!
//! Removing it was a source change with nothing pinning it. Every downstream
//! document that quoted the number had to be rewritten by hand, and nothing
//! would have caught the same allocation coming back. This file is that
//! missing guard. It measures, rather than reasons about, the one property
//! those documents now assert.
//!
//! ## Why the counter is per thread
//!
//! The measuring thread is not the only one allocating. libtest's main thread
//! does its own bookkeeping while a test runs, and plain `cargo test` runs a
//! file's other tests beside it on a thread pool. A process-global counter
//! charged all of that to whichever measurement was open. On CI it failed
//! `ring_barrier`'s longest measurement about one run in five, always with
//! 4 allocations of 900 bytes. A counter owned by the thread that reads it
//! sees only what the measured body allocated, under either runner.
//!
//! ## Why the control arm is not optional
//!
//! An allocation counter can be silently broken: miscompiled away, never
//! installed, or counting into a different static. It then reports zero for
//! everything, which is the answer this file is looking for. `a_control_that_must
//! _allocate` below forces a real allocation through the same counter on the
//! same thread, so a zero from the measured calls means "nothing allocated"
//! rather than "nothing was watching". A second arm has another thread
//! allocate during a measurement that must still read zero, so the counter
//! cannot drift back to charging the whole process.

// Ordinary tests, compiled out under `--cfg loom`. That cfg swaps
// `ring_atomic`'s atomics for loom's instrumented ones across the whole
// family, and those panic the moment they are touched outside a
// model closure that instruments them.
#![cfg(not(loom))]
// A counting allocator cannot be written in safe Rust. `GlobalAlloc` is an
// unsafe trait by construction. The workspace denies `unsafe_code`, and the two
// crates that override it do so in `src/lib.rs`. This file is not one of them.
// The allowance here is test-only, covers one wrapper that forwards to
// `std::alloc::System` unchanged, and adds no unsafe code to any shipped crate.
#![allow(unsafe_code)]

use core::alloc::{GlobalAlloc, Layout};
use core::cell::Cell;
use core::sync::atomic::{AtomicBool, Ordering};

use ring_cursor::PaddedCursor;
use ring_types::Seq;

thread_local! {
  /// Allocation calls this thread has made since it started.
  static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };

  /// Bytes this thread has requested since it started.
  static BYTES: Cell<usize> = const { Cell::new(0) };
}

/// `std::alloc::System`, plus a tally.
///
/// `alloc_zeroed` and `realloc` are left at their trait defaults, both of
/// which route through `alloc`, so a single counter here sees every path a
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

#[test]
fn the_gating_fold_allocates_nothing_at_every_arity() {
  // Built before the first measurement, so their own construction is never
  // charged to the fold.
  let none: [PaddedCursor; 0] = [];
  let one = [PaddedCursor::new(Seq(7))];
  let three = [
    PaddedCursor::new(Seq(12)),
    PaddedCursor::new(Seq(4)),
    PaddedCursor::new(Seq(9)),
  ];

  // The control arm comes first. If this reads zero the counter is not working and
  // every assertion below is vacuous.
  let (control_calls, control_bytes, buffer) = measure(|| {
    let mut buffer = Vec::<Seq>::with_capacity(3);
    buffer.push(Seq(1));
    buffer
  });
  assert!(
    control_calls >= 1,
    "a control that must allocate reported {control_calls} calls — the counting \
     allocator is not installed, so a zero from the measured calls below would \
     mean nothing"
  );
  assert!(
    control_bytes >= 3 * size_of::<Seq>(),
    "the control allocated {control_bytes} bytes for a 3-element Vec< Seq >, \
     which is less than the elements alone need"
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

  // The empty case was already free before the `Vec` was removed. An empty
  // `collect()` yields `Vec::new()`, which never reaches the allocator. It is
  // measured anyway, because it is the one case that would have stayed green
  // through the whole regression this file guards against.
  let (calls, bytes, answer) = measure(|| ring_cursor::slowest(&none));
  assert_eq!(answer, None, "no cursors, no minimum");
  assert_eq!((calls, bytes), (0, 0), "slowest over an empty slice");

  let (calls, bytes, answer) = measure(|| ring_cursor::slowest(&one));
  assert_eq!(answer, Some(Seq(7)));
  assert_eq!((calls, bytes), (0, 0), "slowest over one cursor");

  let (calls, bytes, answer) = measure(|| ring_cursor::slowest(&three));
  assert_eq!(answer, Some(Seq(4)), "the minimum is in the middle");
  assert_eq!(
    (calls, bytes),
    (0, 0),
    "slowest over three cursors — the arity the removed Vec was sized by"
  );

  // A thousand reads, because the claim the documents make is per-call and a
  // single zero is also what a hoisted, cached allocation would report.
  let (calls, bytes, _) = measure(|| {
    for _ in 0..1000 {
      core::hint::black_box(ring_cursor::slowest(&three));
    }
  });
  assert_eq!(
    (calls, bytes),
    (0, 0),
    "a thousand gate reads — the shape the removed Vec charged a thousand times"
  );
}
