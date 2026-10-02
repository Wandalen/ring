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
//! ## Why one `#[ test ]` and not several
//!
//! The counter is process-global. `cargo nextest` gives each test its own
//! process, but plain `cargo test` runs a file's tests on a thread pool in one
//! process, where two concurrent measurements would each see the other's
//! allocations. Writing the whole measurement as one test makes the file
//! correct under both runners rather than only the one the family happens to
//! use.
//!
//! ## Why the control arm is not optional
//!
//! An allocation counter can be silently broken: miscompiled away, never
//! installed, or counting into a different static. It then reports zero for
//! everything, which is the answer this file is looking for. `a_control_that_must
//! _allocate` below forces a real allocation through the same counter in the
//! same process, so a zero from the measured calls means "nothing allocated"
//! rather than "nothing was watching".

// Ordinary tests, compiled out under `--cfg loom`. That cfg swaps
// `ring_atomic`'s atomics for loom's instrumented ones across the whole
// family, and those panic the moment they are touched outside a
// model closure that instruments them.
#![cfg(not(loom))]
// A counting allocator cannot be written in safe Rust. `GlobalAlloc` is an
// unsafe trait by construction. The workspace denies `unsafe_code`, and the
// two crates that override it do so in `src/lib.rs`
// (`ring_store/docs/integration/002_every_unsafe_block_in_the_family.md`
// measures exactly that set, and this file is not in it). The allowance here
// is test-only, covers one wrapper that forwards to `std::alloc::System`
// unchanged, and adds no unsafe code to any shipped crate.
#![allow(unsafe_code)]

use core::alloc::{GlobalAlloc, Layout};
use core::sync::atomic::{AtomicUsize, Ordering};

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
