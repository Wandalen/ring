//! Tests for `ring_slot` — the two slot shapes over one ring.
//!
//! Claims `docs/feature/182_typed_slot_and_bytes_slot.md`. The feature's
//! constraint is that "both use the same claim, gating, and drain — the
//! difference is confined to what a slot contains", so the tests that matter
//! most are the ones exercising both shapes *through the trait*: if anything
//! downstream had to know which shape it held, the constraint would already be
//! broken.
//!
//! The other load-bearing property is that a `BytesSlot` reads back exactly what
//! was written and never the unused tail. The crate carries no `unsafe`, so this
//! is a length-tracking question rather than an initialisation one — but a
//! partially-filled slot leaking its previous contents would be a data leak
//! across ring laps either way, which is why the overwrite cases below are
//! explicit.

use ring_slot::{BytesSlot, Slot, TypedSlot};
use ring_types::RingError;

// ---- TypedSlot ----

/// A fresh typed slot is empty, and reports so through both its own accessor
/// and the trait.
#[test]
fn a_fresh_typed_slot_is_empty() {
    let slot = TypedSlot::<u32>::empty();
    assert!(slot.is_empty());
    assert_eq!(slot.get(), None);
    assert!(TypedSlot::<String>::default().is_empty());
}

/// A typed slot round-trips its value: what is set is what is borrowed and what
/// is taken.
#[test]
fn a_typed_slot_round_trips_its_value() {
    let mut slot = TypedSlot::empty();
    assert_eq!(slot.set(42u32), None);
    assert!(!slot.is_empty());
    assert_eq!(slot.get(), Some(&42));
    assert_eq!(slot.take(), Some(42));
    assert!(slot.is_empty());
    assert_eq!(slot.take(), None, "a second take finds nothing");
}

/// Setting over an occupied slot returns the displaced value rather than
/// dropping it — what lets an evict-oldest policy hand back what it evicted
/// instead of losing it silently.
#[test]
fn setting_over_a_value_returns_the_displaced_one() {
    let mut slot = TypedSlot::empty();
    assert_eq!(slot.set(1u8), None);
    assert_eq!(slot.set(2u8), Some(1));
    assert_eq!(slot.set(3u8), Some(2));
    assert_eq!(slot.get(), Some(&3));
}

/// `TypedSlot::clear` runs the payload's destructor; `BytesSlot::clear` cannot,
/// and the asymmetry is the whole content of `lifecycle/002` SL31. Counting
/// drops is the only way to see it — every API-visible effect of the two
/// `clear`s is identical, which is exactly what makes the difference a trap.
#[test]
fn clearing_a_typed_slot_runs_the_payloads_destructor() {
    use core::sync::atomic::{AtomicUsize, Ordering};

    static DROPS: AtomicUsize = AtomicUsize::new(0);

    struct CountsItsOwnDrop;

    impl Drop for CountsItsOwnDrop {
        fn drop(&mut self) {
            DROPS.fetch_add(1, Ordering::Relaxed);
        }
    }

    let mut slot = TypedSlot::empty();
    slot.set(CountsItsOwnDrop);
    assert_eq!(DROPS.load(Ordering::Relaxed), 0, "publishing drops nothing");

    slot.clear();
    assert_eq!(DROPS.load(Ordering::Relaxed), 1, "clearing runs the destructor");
    assert!(slot.is_empty());

    slot.clear();
    assert_eq!(DROPS.load(Ordering::Relaxed), 1, "and a second clear has nothing left to run");
}

/// The other half of that asymmetry, pinned structurally rather than by
/// observation: a `BytesSlot`'s payload is inline in the slot, so there is no
/// allocation for `clear` to release and no destructor for it to run. `clear`
/// sets a length and the array is untouched — the residue `lifecycle/002` SL32
/// describes. Nothing safe can read it back, which is the point of the
/// hand-written `Debug` and `PartialEq`, so its lifetime is established from the
/// type's layout and from the crate's own field census instead. See
/// `pitfall/002` SL43.
#[test]
fn a_byte_slots_payload_is_inline_so_clear_has_no_allocation_to_release() {
    use core::mem::size_of;

    assert_eq!(
        size_of::<BytesSlot<16>>(),
        16 + size_of::<usize>(),
        "sixteen bytes of payload and a length, with no pointer to anything else",
    );
    assert_eq!(
        size_of::<BytesSlot<64>>() - size_of::<BytesSlot<16>>(),
        48,
        "the slot grows byte for byte with N, so the payload is stored in it, not behind it",
    );

    let mut slot = BytesSlot::<16>::empty();
    slot.write(b"lap 1: secret456").unwrap();
    assert_eq!(slot.len(), 16);

    slot.clear();
    slot.write(b"lap 2").unwrap();
    assert_eq!(slot.read(), b"lap 2", "the shorter lap-2 write reads back as itself");
    assert_eq!(slot.len(), 5, "and lap 1's other eleven bytes are past the length, not gone");
}

/// Clearing empties an occupied slot and is idempotent on an empty one, so a
/// drain need not check first.
#[test]
fn clearing_a_typed_slot_is_idempotent() {
    let mut slot = TypedSlot::empty();
    slot.set(5u16);
    slot.clear();
    assert!(slot.is_empty());
    slot.clear();
    assert!(slot.is_empty());
    assert_eq!(slot.get(), None);
}

/// A slot holding a non-`Copy` payload works the same way — the shape is about
/// occupancy, not about what the payload can do.
#[test]
fn a_typed_slot_holds_non_copy_payloads() {
    let mut slot = TypedSlot::empty();
    assert_eq!(slot.set(String::from("first")), None);
    assert_eq!(slot.set(String::from("second")), Some(String::from("first")));
    assert_eq!(slot.take(), Some(String::from("second")));
    assert!(slot.is_empty());
}

/// A slot holding a value that is *itself* empty-looking is still occupied —
/// occupancy is the slot's own state, not a property read off the payload.
#[test]
fn a_slot_holding_a_default_value_is_still_occupied() {
    let mut slot = TypedSlot::empty();
    slot.set(0u32);
    assert!(!slot.is_empty(), "a slot holding zero is not an empty slot");

    let mut text = TypedSlot::empty();
    text.set(String::new());
    assert!(!text.is_empty(), "a slot holding an empty string is not an empty slot");
}

// ---- BytesSlot ----

/// A fresh bytes slot is empty with its capacity available and nothing to read.
#[test]
fn a_fresh_bytes_slot_is_empty() {
    let slot = BytesSlot::<16>::empty();
    assert!(slot.is_empty());
    assert_eq!(slot.len(), 0);
    assert_eq!(slot.capacity(), 16);
    assert_eq!(slot.read(), b"");
    assert!(BytesSlot::<8>::default().is_empty());
}

/// Capacity is the const parameter, and is fixed for the slot's life — a ring's
/// slots are allocated once, so a growable slot would defeat the allocation
/// behaviour the ring was chosen for.
#[test]
fn capacity_is_the_const_parameter() {
    assert_eq!(BytesSlot::<1>::empty().capacity(), 1);
    assert_eq!(BytesSlot::<8>::empty().capacity(), 8);
    assert_eq!(BytesSlot::<4096>::empty().capacity(), 4096);

    let mut slot = BytesSlot::<8>::empty();
    slot.write(b"abc").unwrap();
    assert_eq!(slot.capacity(), 8, "writing does not change capacity");
}

/// A bytes slot round-trips its payload and reports the written length.
#[test]
fn a_bytes_slot_round_trips_its_payload() {
    let mut slot = BytesSlot::<16>::empty();
    slot.write(b"hello").unwrap();
    assert_eq!(slot.read(), b"hello");
    assert_eq!(slot.len(), 5);
    assert!(!slot.is_empty());
}

/// A read returns the written bytes and only those — never the unused tail.
/// Asserted across every length from empty to full, because an off-by-one in
/// the length would leak exactly one stale byte.
#[test]
fn a_read_never_returns_the_unused_tail() {
    const CAP: usize = 8;
    let payload = b"abcdefgh";

    for len in 0..=CAP {
        let mut slot = BytesSlot::<CAP>::empty();
        slot.write(&payload[..len]).unwrap();
        assert_eq!(slot.read(), &payload[..len], "length {len}");
        assert_eq!(slot.read().len(), len);
    }
}

/// A write filling the slot exactly is accepted — the boundary is inclusive.
#[test]
fn a_write_of_exactly_capacity_is_accepted() {
    let mut slot = BytesSlot::<4>::empty();
    assert!(slot.write(b"abcd").is_ok());
    assert_eq!(slot.read(), b"abcd");
    assert_eq!(slot.len(), slot.capacity());
    assert!(!slot.is_empty());
}

/// A write one byte over capacity is refused, and says how much was asked for
/// against how much there was.
#[test]
fn an_oversized_write_is_refused_with_both_numbers() {
    let mut slot = BytesSlot::<4>::empty();
    assert_eq!(slot.write(b"abcde"), Err(RingError::BatchTooLarge { requested: 5, capacity: 4 }));
}

/// A failed write leaves the previous contents intact — the slot is not
/// half-updated, so a caller that handles the error still has valid data.
#[test]
fn a_failed_write_leaves_the_previous_contents_intact() {
    let mut slot = BytesSlot::<4>::empty();
    slot.write(b"abcd").unwrap();
    assert!(slot.write(b"abcde").is_err());

    assert_eq!(slot.read(), b"abcd");
    assert_eq!(slot.len(), 4);
    assert!(!slot.is_empty());
}

/// A failed write onto an empty slot leaves it empty rather than partially
/// filled.
#[test]
fn a_failed_write_onto_an_empty_slot_leaves_it_empty() {
    let mut slot = BytesSlot::<2>::empty();
    assert!(slot.write(b"too long").is_err());
    assert!(slot.is_empty());
    assert_eq!(slot.read(), b"");
}

/// A shorter write over a longer one truncates the reading — the stale tail must
/// not reappear, which across ring laps would be a data leak between publishes.
#[test]
fn a_shorter_write_does_not_leak_the_longer_one() {
    let mut slot = BytesSlot::<8>::empty();
    slot.write(b"AAAAAAAA").unwrap();
    assert_eq!(slot.read(), b"AAAAAAAA");

    slot.write(b"bb").unwrap();
    assert_eq!(slot.read(), b"bb", "the previous payload's tail must not reappear");
    assert_eq!(slot.len(), 2);
}

/// Clearing a bytes slot empties the reading, and a clear followed by a write
/// starts from nothing.
#[test]
fn clearing_a_bytes_slot_empties_the_reading() {
    let mut slot = BytesSlot::<8>::empty();
    slot.write(b"payload").unwrap();
    slot.clear();

    assert!(slot.is_empty());
    assert_eq!(slot.len(), 0);
    assert_eq!(slot.read(), b"");

    slot.clear();
    assert!(slot.is_empty(), "clearing twice is harmless");

    slot.write(b"z").unwrap();
    assert_eq!(slot.read(), b"z", "a cleared slot writes from nothing");
}

/// A zero-length write empties the slot rather than leaving the old payload —
/// writing nothing means the slot holds nothing.
#[test]
fn a_zero_length_write_empties_the_slot() {
    let mut slot = BytesSlot::<8>::empty();
    slot.write(b"data").unwrap();
    slot.write(b"").unwrap();

    assert!(slot.is_empty());
    assert_eq!(slot.len(), 0);
    assert_eq!(slot.read(), b"");
}

/// A zero-capacity slot accepts only the empty payload — a degenerate case, but
/// one a generic caller can construct, so it must not panic.
#[test]
fn a_zero_capacity_slot_accepts_only_nothing() {
    let mut slot = BytesSlot::<0>::empty();
    assert_eq!(slot.capacity(), 0);
    assert!(slot.is_empty());
    assert!(slot.write(b"").is_ok());
    assert!(slot.is_empty());
    assert_eq!(slot.write(b"x"), Err(RingError::BatchTooLarge { requested: 1, capacity: 0 }));
}

/// `BytesSlot` carries an inherent `is_empty` alongside the trait's, so a caller
/// holding the concrete type need not import `Slot`. The two must never
/// disagree — this is the only test that can catch it, since every other call
/// site resolves to whichever one is in scope.
#[test]
fn the_inherent_and_trait_emptiness_agree() {
    let mut slot = BytesSlot::<4>::empty();
    for payload in [&b""[..], b"a", b"abcd", b""] {
        slot.write(payload).unwrap();
        assert_eq!(
            BytesSlot::is_empty(&slot),
            Slot::is_empty(&slot),
            "payload {payload:?} reads differently through the two paths"
        );
    }

    slot.write(b"ab").unwrap();
    slot.clear();
    assert_eq!(BytesSlot::is_empty(&slot), Slot::is_empty(&slot));
}

/// Two slots holding the same payload compare equal even when their backing
/// arrays differ — which is the whole point, and is what a derived `PartialEq`
/// would get wrong. The first fixture pair is written once each from empty, so
/// their tails coincide and it could not distinguish the two relations; the
/// second is the case that can, and the one this test's name promises. See
/// `pitfall/001` SL41 and SL42.
#[test]
fn slots_compare_by_payload_not_by_tail() {
    let mut written_once = BytesSlot::<8>::empty();
    written_once.write(b"ab").unwrap();

    let mut also_written_once = BytesSlot::<8>::empty();
    also_written_once.write(b"ab").unwrap();

    assert_eq!(written_once, also_written_once);
    assert_eq!(written_once.read(), also_written_once.read());

    // Same payload, different tail: `bytes` is [ 97, 98, 88, 88, 88, 88, 88, 88 ]
    // here against [ 97, 98, 0, 0, 0, 0, 0, 0 ] above. A derived comparison reads
    // all eight and reports these unequal; the hand-written one reads `read()`.
    let mut overwritten = BytesSlot::<8>::empty();
    overwritten.write(b"XXXXXXXX").unwrap();
    overwritten.write(b"ab").unwrap();

    assert_eq!(overwritten.read(), written_once.read(), "indistinguishable through every accessor");
    assert_eq!(overwritten, written_once, "and therefore indistinguishable through `==`");
}

/// Two slots holding different payloads do not compare equal. Every other
/// `PartialEq` test in this file asserts `assert_eq!` — proving what a
/// hand-written `eq` must agree with a derived one on — and none of them
/// would notice `eq` always answering `true`. This is the one case that does.
#[test]
fn slots_with_different_payloads_are_not_equal() {
    let mut a = BytesSlot::<8>::empty();
    a.write(b"ab").unwrap();

    let mut b = BytesSlot::<8>::empty();
    b.write(b"cd").unwrap();

    assert_ne!(a, b, "different payloads must not compare equal");
}

/// A cleared slot is equal to a fresh one, and prints as one. Both were false
/// while `Debug` and `PartialEq` were derived: the array still held the payload
/// after `clear` moved the length, so a cleared slot compared unequal to a
/// fresh one and printed the bytes it no longer held. See `pitfall/002` SL43
/// and SL44 — the residue is still in memory, it is just no longer reachable
/// through any public API this type has.
#[test]
fn a_cleared_slot_is_indistinguishable_from_a_fresh_one() {
    let mut cleared = BytesSlot::<8>::empty();
    cleared.write(b"secret").unwrap();
    cleared.clear();

    let fresh = BytesSlot::<8>::empty();

    assert!(cleared.is_empty());
    assert_eq!(cleared.read(), fresh.read());
    assert_eq!(cleared, fresh, "a cleared slot is a fresh slot's value");
    assert_eq!(format!("{cleared:?}"), "BytesSlot { payload: [] }");
    assert!(
        !format!("{cleared:?}").contains("115"),
        "no byte of `secret` survives into the printed form"
    );
}

/// `TypedSlot`'s `Default` is written out as `impl< T >`, not derived. The
/// derive would emit `impl< T : Default >` — it defaults every field, including
/// the `Option< T >` whose own default needs nothing from `T` — and that bound
/// would propagate to every `S : Slot + Default` consumer, silently narrowing
/// the ring to payloads that happen to be `Default`. This test does not run
/// anything: `NotDefault` deliberately has no `Default` impl, so the file stops
/// compiling if the hand-written impl is ever replaced by the derive. See
/// `non_functional_requirement/002` SL36.
#[test]
fn a_slot_is_default_for_a_payload_that_is_not() {
    struct NotDefault(#[allow(dead_code)] u32);

    let slot: TypedSlot<NotDefault> = TypedSlot::default();
    assert!(slot.is_empty());
}

// ---- The shared trait ----

/// Both shapes are usable through `Slot` alone, which is the feature's actual
/// constraint: everything downstream is written against the trait and cannot
/// branch on which shape it holds.
#[test]
fn both_shapes_drive_through_the_trait_alone() {
    fn fill_and_drain(slot: &mut dyn Slot) -> (bool, bool) {
        let occupied_before_clear = !slot.is_empty();
        slot.clear();
        (occupied_before_clear, slot.is_empty())
    }

    let mut typed = TypedSlot::empty();
    typed.set(9u32);
    assert_eq!(fill_and_drain(&mut typed), (true, true));

    let mut bytes = BytesSlot::<8>::empty();
    bytes.write(b"payload").unwrap();
    assert_eq!(fill_and_drain(&mut bytes), (true, true));
}

/// The trait's two methods agree for both shapes across the whole occupancy
/// cycle — empty, filled, cleared — so a drain loop written once behaves the
/// same on either.
#[test]
fn the_trait_reports_the_same_cycle_for_both_shapes() {
    fn cycle<S: Slot>(slot: &mut S, fill: impl FnOnce(&mut S)) -> [bool; 3] {
        let empty_at_start = slot.is_empty();
        fill(slot);
        let empty_when_filled = slot.is_empty();
        slot.clear();
        [empty_at_start, empty_when_filled, slot.is_empty()]
    }

    let typed = cycle(&mut TypedSlot::empty(), |s| {
        s.set(1u8);
    });
    let bytes = cycle(&mut BytesSlot::<4>::empty(), |s| s.write(b"ab").unwrap());

    assert_eq!(typed, [true, false, true]);
    assert_eq!(typed, bytes, "the two shapes must be indistinguishable through the trait");
}
