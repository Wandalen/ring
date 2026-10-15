//! As close a test of "the append path allocates nothing" as this crate can
//! get.
//!
//! # Why this is a proxy and not the measurement
//!
//! The requirement as specified wants a counting global allocator. That is
//! not available here, and the obstacle is structural rather than incidental:
//! `GlobalAlloc` is an unsafe trait, the workspace sets `unsafe-code = "deny"`,
//! and gate G6 confines the opt-out to the crates declared in
//! `bench_harness/gate/declared/ring/unsafe_allowlist.txt`, each of which must
//! justify it in its own `docs/workaround/readme.md`.
//!
//! **G6 scans `src/` only, so an `#![allow(unsafe_code)]` in this file would
//! pass the gate.** It would also defeat the gate's stated purpose of keeping
//! the escape hatch enumerated rather than letting it spread crate by crate,
//! and a measurement bought that way is worth less than the gate it cost. Not
//! done.
//!
//! So what is asserted here is the strongest *safe* proxy: the staging buffer's
//! capacity does not change across `N` appends. A `Vec` that did not grow did
//! not reallocate, and growth is the realistic defect. A buffer that silently
//! expands instead of refusing turns a bounded stage into an unbounded one and
//! puts an amortised allocation on the hot path.
//!
//! | Defect | Caught here |
//! |---|---|
//! | The staging buffer grows instead of refusing | **Yes**, capacity changes |
//! | `append` allocates per record (boxing, a temporary `Vec`) | **No**, needs the real allocator |
//! | An unobserved driver allocates for its absent log | Partly, via the absence of growth, not directly |
//!
//! This is a proxy rather than a discharge of the requirement. The real
//! measurement belongs in `ring_testkit`, which is the one place a counting
//! allocator could be justified once for every crate that needs one. G6 now
//! forces that choice rather than merely preferring it.
//!
//! # What this file *does* discharge outright
//!
//! The append path's **behavioural** cost claims, which need no allocator:
//! nothing reaches the ring, no log entry appears, and a non-firing drive
//! changes nothing. Those are exact, not proxies.

// Ordinary tests, compiled out under `--cfg loom`. That cfg swaps
// `ring_atomic`'s atomics for loom's instrumented ones across the whole
// family, and those panic the moment they are touched outside a
// `loom::model` closure. Without this gate a family-wide loom run dies
// here instead of reaching the models in `ring_spsc`, `ring_mpsc`,
// `ring_publish` and `ring_testkit`.
#![cfg(not(loom))]

use ring_config::RingConfig;
use ring_core::Ring;
use ring_flush::{FlushOutcome, FlushPolicy, Flusher};
use ring_tls::TlsBuffer;
use ring_types::OverflowPolicy;

fn ring(slots: usize) -> Ring<u32> {
  let config = RingConfig::new(slots).unwrap().with_overflow(OverflowPolicy::Fail);
  Ring::new(&config).unwrap()
}

/// The staging buffer does not grow across `N` appends, for any policy.
///
/// The proxy for the allocation requirement. Capacity is read before and
/// after; a `Vec` that did not reallocate reports the same number.
#[test]
fn appending_never_grows_the_staging_buffer() {
  const N: u32 = 512;

  // `OnBatch( N )` rather than anything larger, because validation caps the batch size at the
  // buffer's capacity, so `OnBatch( 600 )` against a 512-record buffer is
  // rejected at binding. That is correct, and it is `an_unusable_batch_size_is_refused_at_binding`'s
  // job to say so. The batch still never fires here, because nothing drives.
  for policy in [FlushPolicy::OnBarrier, FlushPolicy::OnFull, FlushPolicy::OnBatch(N as usize)] {
    let mut r = ring(1024);
    let mut ends = r.ends();
    let (producer, _consumer) = ends.split();

    let buffer = TlsBuffer::with_capacity(N as usize);
    let before = buffer.capacity();
    let mut flusher = Flusher::new(buffer, producer, policy).expect("n is within capacity");

    for i in 0..N {
      flusher.append(i).expect("within capacity");
    }

    assert_eq!(flusher.staged(), N as usize, "{policy:?}: an append was lost");
    assert_eq!(
      flusher.buffer_capacity(),
      before,
      "{policy:?}: the staging buffer reallocated during {N} appends"
    );
  }
}

/// A full staging buffer refuses rather than growing.
///
/// The other half of the proxy, and the one that would fail first if the
/// refusal were ever removed. Capacity staying constant is only meaningful if
/// the buffer is driven to its limit.
#[test]
fn a_full_staging_buffer_refuses_and_keeps_its_capacity() {
  let mut r = ring(1024);
  let mut ends = r.ends();
  let (producer, _consumer) = ends.split();

  let buffer = TlsBuffer::with_capacity(8);
  let before = buffer.capacity();
  let mut flusher = Flusher::new(buffer, producer, FlushPolicy::OnBarrier).unwrap();

  for i in 0..8u32 {
    flusher.append(i).expect("within capacity");
  }
  assert!(flusher.append(8).is_err(), "a full staging buffer accepted a record");

  assert_eq!(flusher.staged(), 8);
  assert_eq!(flusher.buffer_capacity(), before, "the buffer grew instead of refusing");
}

/// A thousand non-firing drives change nothing observable.
///
/// Exact, not a proxy. `OnBarrier` consumers drive far more often than they
/// announce, so the cost of *declining* is paid at the caller's cadence and is
/// worth pinning even without an allocator.
#[test]
fn a_drive_that_does_not_fire_changes_nothing() {
  let mut r = ring(1024);
  let mut ends = r.ends();
  let (producer, consumer) = ends.split();

  let buffer = TlsBuffer::with_capacity(64);
  let before = buffer.capacity();
  let mut flusher = Flusher::new(buffer, producer, FlushPolicy::OnBarrier).unwrap().with_log();
  flusher.append(1).unwrap();

  for _ in 0..1_000 {
    assert_eq!(flusher.drive(), FlushOutcome::NotTriggered, "OnBarrier fired unannounced");
  }

  assert_eq!(flusher.staged(), 1);
  assert_eq!(flusher.buffer_capacity(), before);
  assert_eq!(consumer.len(), 0, "a non-firing drive published");
  assert!(flusher.log().unwrap().is_empty(), "a non-firing drive was recorded");
}

/// A driver given no log never acquires one, however much it flushes.
///
/// The claim that an unobserved driver pays nothing for the instrument it was
/// not given, as an assertion rather than as prose. Exact, because `log()`
/// returning `None` is not a proxy for anything.
#[test]
fn an_unobserved_driver_never_acquires_a_log() {
  let mut r = ring(1024);
  let mut ends = r.ends();
  let (producer, mut consumer) = ends.split();

  let buffer = TlsBuffer::with_capacity(64);
  let mut flusher = Flusher::new(buffer, producer, FlushPolicy::OnBatch(4)).unwrap();

  for round in 0..16u32 {
    for i in 0..4u32 {
      flusher.append(round * 4 + i).unwrap();
    }
    assert_eq!(flusher.drive(), FlushOutcome::Flushed { count: 4 });
    assert!(flusher.log().is_none(), "a driver acquired a log it was never given");
  }

  let mut landed = Vec::new();
  assert_eq!(consumer.try_recv_batch(&mut landed), 64);
  assert_eq!(landed.len(), 64, "sixty-four records were staged and flushed");
}
