//! C2 of `docs/non_functional_requirement/002_the_decision_costs_nothing_on_the_append_path.md`
//! — as close to "the append path allocates nothing" as this crate can get.
//!
//! # Why this is a proxy and not the measurement
//!
//! C2 as specified wants a counting global allocator. That is not available
//! here, and the obstacle is structural rather than incidental: `GlobalAlloc`
//! is an unsafe trait, the workspace sets `unsafe-code = "deny"`, and gate G6
//! confines the opt-out to three declared crates — `ring_spsc`, `ring_mpsc`,
//! `ring_core` — each justifying it in its own
//! `docs/workaround/readme.md`.
//!
//! **G6 scans `src/` only, so an `#![allow(unsafe_code)]` in this file would
//! pass the gate.** It would also defeat the gate's stated purpose — keeping
//! the escape hatch enumerated rather than letting it spread crate by crate —
//! and a measurement bought that way is worth less than the gate it cost. Not
//! done.
//!
//! So what is asserted here is the strongest *safe* proxy: the staging buffer's
//! capacity does not change across `N` appends. A `Vec` that did not grow did
//! not reallocate, and growth is the realistic defect — a buffer that silently
//! expands instead of refusing turns a bounded stage into an unbounded one and
//! puts an amortised allocation on the hot path.
//!
//! | Defect | Caught here |
//! |---|---|
//! | The staging buffer grows instead of refusing | **Yes** — capacity changes |
//! | `append` allocates per record (boxing, a temporary `Vec`) | **No** — needs the real allocator |
//! | An unobserved driver allocates for its absent log | Partly — via the absence of growth, not directly |
//!
//! Recorded as a proxy in the instance rather than as a discharge of C2. The
//! real measurement belongs in `ring_testkit` (feature 188), which is the one
//! place a counting allocator could be justified once for every crate that
//! needs one — option three of the instance's own table, and now forced by G6
//! rather than merely preferred.
//!
//! # What this file *does* discharge outright
//!
//! The append path's **behavioural** cost claims, which need no allocator:
//! nothing reaches the ring, no log entry appears, and a non-firing drive
//! changes nothing. Those are exact, not proxies.

// Ordinary tests, compiled out under `--cfg loom`. That cfg swaps
// `ring_atomic`'s atomics for loom's instrumented ones across the whole
// family, and those panic the moment they are touched outside a
// `loom::model` closure — so without this gate a family-wide loom run dies
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
/// The proxy for C2. Capacity is read before and after; a `Vec` that did not
/// reallocate reports the same number.
#[test]
fn appending_never_grows_the_staging_buffer() {
    const N: u32 = 512;

    // `OnBatch( N )` rather than anything larger: N2 caps the batch size at the
    // buffer's capacity, so `OnBatch( 600 )` against a 512-record buffer is
    // rejected at binding — correctly, and it is `an_unusable_batch_size_is_refused_at_binding`'s
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
/// The other half of the proxy, and the one that would actually fail first if
/// the refusal were ever removed: capacity staying constant is only meaningful
/// if the buffer is genuinely driven to its limit.
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
/// [`docs/decisions/readme.md`]'s Pending 2 as an assertion rather than as
/// prose: the claim that dissolved the pending decision is that an unobserved
/// driver pays nothing for the instrument it was not given. Exact — `log()`
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
