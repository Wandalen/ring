//! The read path allocates nothing. This file asserts that rather than probing it.
//!
//! ## Why this file exists
//!
//! A requirements document once recorded, as a finding, that every
//! barrier-consulting read here cost one heap allocation. `available`,
//! `available_up_to` and `commit_available` each charged one per call, on every
//! poll of an idle ring. A second document named the site, a `Vec` two crates
//! down whose only job was to change a slice's element type. Both findings were
//! declined, on the ground that fixing them would invalidate quoted evidence in
//! sibling crates nobody in those passes owned.
//!
//! A later commit removed it anyway, from the other direction, and every number
//! in those documents became false at once. Nothing detected that, because the
//! measurement was a scratch binary pasted into a document, run once, and
//! deleted by its own last line.
//!
//! This file is the standing form of that probe. It measures the same six call
//! shapes that finding's table lists, in the same order.
//!
//! ## Why the two empty-barrier rows are kept
//!
//! They read zero before the fix and zero after. `frontier()` returns `None`
//! without reaching the fold when there are no dependencies, so an empty
//! barrier never allocated even when every other row did. That is how the cost
//! stayed invisible. They are kept and labelled so the next reader sees that
//! two of the six rows can distinguish nothing.
//!
//! ## Why one `#[ test ]`, and why a control arm
//!
//! Both for the reasons `ring_cursor/tests/allocation_test.rs` states at
//! length. The counter is process-global, and a silently broken counter
//! reports zero for everything, which is the answer this file is looking for.

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
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use ring_barrier::Barrier;
use ring_consume::Consumer;
use ring_cursor::PaddedCursor;
use ring_types::Seq;

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
fn no_read_of_the_available_range_allocates() {
  // Everything is built before the first measurement, so no construction cost
  // is charged to a call below.
  let published = [PaddedCursor::new(Seq(4096))];
  let position = PaddedCursor::default();
  let consumer = Consumer::new(&position, Barrier::over(&published));

  let idle_position = PaddedCursor::default();
  let idle = Consumer::new(&idle_position, Barrier::over(&[]));

  // The control arm comes first. If this reads zero, the counter is not
  // working and every assertion below is vacuous.
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

  // The finding's rows, in the order its table lists them. `position()` never
  // consulted the barrier and so never allocated; the three that follow did.
  let (calls, bytes, _) = measure(|| {
    for _ in 0..1000 {
      core::hint::black_box(consumer.position());
    }
  });
  assert_eq!((calls, bytes), (0, 0), "position() ×1000");

  let (calls, bytes, _) = measure(|| {
    for _ in 0..1000 {
      core::hint::black_box(consumer.available());
    }
  });
  assert_eq!((calls, bytes), (0, 0), "available() ×1000");

  let (calls, bytes, _) = measure(|| {
    for _ in 0..1000 {
      core::hint::black_box(consumer.available_up_to(8));
    }
  });
  assert_eq!((calls, bytes), (0, 0), "available_up_to( 8 ) ×1000");

  // The first call commits the whole 4096-slot run; the remaining 999 find an
  // empty run and commit nothing. That asymmetry is the point. All 1000
  // consult the barrier *before* learning the answer is zero, so all 1000 used
  // to allocate, including the 999 that did no work.
  let (calls, bytes, _) = measure(|| {
    for _ in 0..1000 {
      core::hint::black_box(consumer.available_up_to(4).end());
      core::hint::black_box(consumer.commit_available());
    }
  });
  assert_eq!((calls, bytes), (0, 0), "commit_available() ×1000");

  // The two rows that read zero before the fix as well, and therefore cannot
  // tell the two states apart. Kept because reading zero both times is the
  // finding.
  let (calls, bytes, run) = measure(|| idle.available());
  assert!(run.is_empty(), "no dependencies, nothing available");
  assert_eq!(
    (calls, bytes),
    (0, 0),
    "available() on an empty barrier — zero before the fix too, and therefore no evidence"
  );

  let (calls, bytes, committed) = measure(|| idle.commit_available());
  assert_eq!(committed, Seq::ZERO, "nothing available, nothing committed");
  assert_eq!(
    (calls, bytes),
    (0, 0),
    "commit_available() on an empty barrier — zero before the fix too, and therefore no evidence"
  );
}
