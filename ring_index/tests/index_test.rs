//! Tests for `ring_index` — the sequence-to-slot fold.
//!
//! Claims the folding half of
//! `docs/feature/167_sequence_slot_index_and_power_of_two_capacity.md`. Its
//! acceptance criterion, filed at
//! `ring/bench_harness/docs/acceptance/001_feature_reached_tests.md`, is exact:
//! `Index::of( seq, cap )` equals `seq % cap` for every `seq` in `0..4*cap` and
//! every power-of-two `cap` in `2..=1024`, computed by mask not division, and a
//! non-power-of-two capacity is rejected at construction.
//!
//! All four clauses are asserted below.

use ring_index::{aliases, of, run};
use ring_types::{Capacity, Seq, SlotIndex};

fn cap(n: usize) -> Capacity {
    Capacity::new(n).expect("test capacities are powers of two")
}

/// The acceptance criterion, verbatim: mask equals modulo across four full laps
/// of every power-of-two capacity from 2 to 1024.
#[test]
fn mask_equals_modulo_over_four_laps_of_every_capacity() {
    let mut slots = 2usize;
    while slots <= 1024 {
        let c = cap(slots);
        for seq in 0..(4 * slots) as u64 {
            assert_eq!(
                of(Seq(seq), c),
                SlotIndex(seq as usize % slots),
                "capacity {slots}, seq {seq}"
            );
        }
        slots *= 2;
    }
}

/// The derivation is a mask, not a division. Asserted structurally: the result
/// equals the masked value for every capacity, which only holds when the
/// capacity is a power of two — so a division-based implementation that
/// happened to agree on modulo would still have to satisfy this.
#[test]
fn derivation_is_a_mask() {
    let mut slots = 2usize;
    while slots <= 1024 {
        let c = cap(slots);
        for seq in [0u64, 1, 7, 63, 1023, 1_000_000, u32::MAX as u64] {
            assert_eq!(of(Seq(seq), c), SlotIndex(seq as usize & c.mask()));
        }
        slots *= 2;
    }
}

/// A non-power-of-two capacity never reaches this crate, because it cannot be
/// constructed. The rejection is upstream, which is why `of` has no error path.
#[test]
fn non_power_of_two_capacity_is_rejected_upstream() {
    for slots in [0usize, 3, 5, 6, 7, 9, 100, 1000] {
        assert!(Capacity::new(slots).is_err(), "{slots} must not be constructible");
    }
    for slots in [1usize, 2, 4, 8, 1024] {
        assert!(Capacity::new(slots).is_ok(), "{slots} must be constructible");
    }
}

/// Capacity 1 is legal and degenerate: every sequence maps to slot 0.
#[test]
fn capacity_one_maps_everything_to_slot_zero() {
    let c = cap(1);
    for seq in 0..10u64 {
        assert_eq!(of(Seq(seq), c), SlotIndex(0));
    }
}

/// Two sequences alias exactly when they are a whole number of laps apart —
/// the property the gating machinery exists to keep unobservable.
#[test]
fn aliasing_is_exactly_whole_laps() {
    let c = cap(4);
    for a in 0..4u64 {
        for b in 0..40u64 {
            let whole_laps = a.abs_diff(b) % 4 == 0;
            assert_eq!(aliases(Seq(a), Seq(b), c), whole_laps, "a={a}, b={b}");
        }
    }
}

/// A sequence always aliases itself, whatever the capacity.
#[test]
fn a_sequence_aliases_itself() {
    for slots in [1usize, 2, 8, 1024] {
        let c = cap(slots);
        assert!(aliases(Seq(12_345), Seq(12_345), c));
    }
}

/// A batch's slots are contiguous in sequence space and wrap at most once
/// within one capacity, which is what preserves a staged buffer's order.
#[test]
fn a_run_wraps_at_most_once_within_one_capacity() {
    let c = cap(4);
    assert_eq!(run(Seq(2), 4, c), vec![SlotIndex(2), SlotIndex(3), SlotIndex(0), SlotIndex(1)]);
    assert_eq!(run(Seq(0), 4, c), vec![SlotIndex(0), SlotIndex(1), SlotIndex(2), SlotIndex(3)]);
}

/// A run of the full capacity touches every slot exactly once, from any start.
#[test]
fn a_full_capacity_run_covers_every_slot_once() {
    let slots = 8usize;
    let c = cap(slots);
    for start in 0..(2 * slots) as u64 {
        let mut seen = run(Seq(start), slots, c);
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), slots, "start {start} did not cover every slot exactly once");
    }
}

/// An empty run is empty rather than a one-element run at the start position.
#[test]
fn an_empty_run_is_empty() {
    assert_eq!(run(Seq(5), 0, cap(8)), vec![]);
}

/// A run longer than the capacity repeats slots, which is the caller's problem
/// to prevent by gating — this crate reports the truth rather than clamping.
#[test]
fn an_oversized_run_repeats_slots() {
    let produced = run(Seq(0), 6, cap(4));
    assert_eq!(produced.len(), 6);
    assert_eq!(produced[0], produced[4]);
    assert_eq!(produced[1], produced[5]);
}

/// IX18: `Seq( u64::MAX )` is not a hypothetical input — `ring_mpsc::UNSTAMPED`
/// reserves exactly that value and a caller can hand it to `run` directly.
/// `run` adds before it folds, so this is reachable in one step rather than
/// after `2^64` publications. Pins the debug-build behaviour `run`'s own
/// `# Panics` section now documents, so the boundary is an asserted fact
/// rather than only a fact demonstrated in `docs/pitfall/001`'s probe output.
#[test]
#[should_panic(expected = "attempt to add with overflow")]
fn run_overflows_at_the_top_of_the_sequence_space() {
    let _ = run(Seq(u64::MAX), 2, cap(1024));
}

/// One below the boundary the previous test pins: `run`'s last internal step
/// is `start.0 + ( count - 1 )`, not `start.0 + count`, so a `start` that
/// would overflow under the coarser sum must still succeed here. Pins the
/// `# Panics` section's precise addend against the off-by-one a less careful
/// reading of `0..count` invites.
#[test]
fn run_does_not_overflow_one_below_the_top_of_the_sequence_space() {
    assert_eq!(run(Seq(u64::MAX - 1), 2, cap(1024)), vec![SlotIndex(1022), SlotIndex(1023)]);
}
