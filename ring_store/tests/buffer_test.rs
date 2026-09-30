//! Storage that holds slots and nothing else.
//!
//! Claims `docs/feature/168_ring_buffer_storage.md`, whose reached-test reads:
//! "A `Buffer<T>` of capacity `N` allocates exactly `N` slots once, exposes
//! indexed get/set, holds no cursor and no ordering state; two distinct slot
//! indices never alias."
//!
//! Three of those four clauses are ordinary. The fourth — *holds no cursor and
//! no ordering state* — is a claim about what the type does **not** contain,
//! and a test cannot assert an absence by exercising it. Two things stand in
//! for it here, and neither is the real thing:
//!
//! - `a_buffer_is_exactly_its_slots_and_its_capacity` pins `size_of` against
//!   the two fields the type is allowed to have. A cursor added later would
//!   grow the struct and fail this.
//! - `storage_survives_being_addressed_out_of_order` writes through sequences
//!   in a deliberately scrambled order and reads them back correct. A buffer
//!   that had quietly acquired ordering state would have an opinion about the
//!   order; this one demonstrably has none.
//!
//! The honest limit: a zero-sized ordering field would pass the first, and a
//! cursor consulted only under contention would pass the second. What actually
//! guarantees the absence is that the struct is 30 lines and legible. These
//! tests catch the drift, not the original sin.

use ring_index::of;
use ring_slot::{BytesSlot, Slot, TypedSlot};
use ring_store::Buffer;
use ring_types::{Capacity, Seq, SlotIndex};

fn cap(n: usize) -> Capacity {
    Capacity::new(n).expect("test capacities are powers of two")
}

#[test]
fn exactly_n_slots_are_allocated_for_capacity_n() {
    for n in [1usize, 2, 4, 8, 16, 1024] {
        let buffer: Buffer<TypedSlot<u32>> = Buffer::new(cap(n));
        assert_eq!(buffer.len(), n, "capacity {n} must allocate exactly {n} slots");
        assert_eq!(buffer.capacity().get(), n);
        assert_eq!(buffer.iter().count(), n, "and iteration must see exactly those");
    }
}

#[test]
fn every_slot_starts_empty() {
    let buffer: Buffer<TypedSlot<u32>> = Buffer::new(cap(8));
    assert!(buffer.all_empty());
    assert_eq!(buffer.iter().filter(|s| s.is_empty()).count(), 8);
}

#[test]
fn a_buffer_is_exactly_its_slots_and_its_capacity() {
    // The stand-in for "holds no cursor and no ordering state": a boxed slice
    // (pointer + length) plus a Capacity. Anything else in the struct grows it.
    let expected = size_of::<Box<[TypedSlot<u32>]>>() + size_of::<Capacity>();
    assert_eq!(
        size_of::<Buffer<TypedSlot<u32>>>(),
        expected,
        "Buffer grew a field — a cursor or an ordering flag would land here"
    );
}

#[test]
fn indexed_get_and_set_round_trip() {
    let mut buffer: Buffer<TypedSlot<u32>> = Buffer::new(cap(4));

    buffer.get_mut(SlotIndex(0)).set(10);
    buffer.get_mut(SlotIndex(3)).set(13);

    assert_eq!(buffer.get(SlotIndex(0)).get(), Some(&10));
    assert_eq!(buffer.get(SlotIndex(3)).get(), Some(&13));
    assert_eq!(buffer.get(SlotIndex(1)).get(), None);
    assert_eq!(buffer.get(SlotIndex(2)).get(), None);
}

#[test]
fn two_distinct_slot_indices_never_alias() {
    // The feature's own words. Written into every slot a value only that slot
    // could hold, then read every slot back: any aliasing pair shows up as a
    // duplicate or as a value in the wrong place.
    const N: usize = 64;
    let mut buffer: Buffer<TypedSlot<usize>> = Buffer::new(cap(N));

    for i in 0..N {
        buffer.get_mut(SlotIndex(i)).set(i * 7 + 1);
    }

    for i in 0..N {
        assert_eq!(
            buffer.get(SlotIndex(i)).get(),
            Some(&(i * 7 + 1)),
            "slot {i} does not hold its own value — indices {i} and some other alias"
        );
    }
}

#[test]
fn distinct_indices_have_distinct_addresses() {
    // The stronger form of the same claim: not merely "the values differ" but
    // "the storage differs". A `len` that over-reported while the slots were
    // shared would pass the value test and fail this one.
    let buffer: Buffer<TypedSlot<u8>> = Buffer::new(cap(16));
    let mut seen: Vec<usize> = Vec::new();

    for i in 0..16 {
        let address = std::ptr::from_ref(buffer.get(SlotIndex(i))) as usize;
        assert!(!seen.contains(&address), "slot {i} shares an address with an earlier slot");
        seen.push(address);
    }
}

#[test]
fn a_sequence_addresses_the_slot_ring_index_says_it_does() {
    // `at` must be the fold and nothing else — a second implementation of the
    // fold living here is exactly what `ring_index` exists to prevent.
    let capacity = cap(8);
    let mut buffer: Buffer<TypedSlot<u64>> = Buffer::new(capacity);

    for raw in 0u64..40 {
        let seq = Seq(raw);
        buffer.at_mut(seq).set(raw);

        let expected = of(seq, capacity);
        assert_eq!(
            buffer.get(expected).get(),
            Some(&raw),
            "sequence {raw} did not land in the slot ring_index folds it to"
        );
    }
}

#[test]
fn a_full_lap_overwrites_and_a_partial_one_does_not() {
    let capacity = cap(4);
    let mut buffer: Buffer<TypedSlot<u64>> = Buffer::new(capacity);

    for raw in 0u64..4 {
        buffer.at_mut(Seq(raw)).set(raw);
    }
    // Sequence 4 laps onto slot 0.
    buffer.at_mut(Seq(4)).set(400);

    assert_eq!(buffer.get(SlotIndex(0)).get(), Some(&400), "the lap landed");
    assert_eq!(buffer.get(SlotIndex(1)).get(), Some(&1), "and touched nothing else");
    assert_eq!(buffer.get(SlotIndex(2)).get(), Some(&2));
    assert_eq!(buffer.get(SlotIndex(3)).get(), Some(&3));
}

#[test]
fn storage_survives_being_addressed_out_of_order() {
    // The second stand-in for "no ordering state": write through a scrambled
    // sequence order and read back correct. A buffer with an opinion about order
    // would have to be wrong about at least one of these.
    let capacity = cap(16);
    let mut buffer: Buffer<TypedSlot<u64>> = Buffer::new(capacity);
    let scrambled = [9u64, 2, 15, 0, 7, 13, 4, 11, 1, 8, 14, 3, 10, 5, 12, 6];

    for &raw in &scrambled {
        buffer.at_mut(Seq(raw)).set(raw * 100);
    }

    for raw in 0u64..16 {
        assert_eq!(buffer.at(Seq(raw)).get(), Some(&(raw * 100)));
    }
}

#[test]
fn clear_empties_every_slot_and_keeps_the_allocation() {
    let mut buffer: Buffer<TypedSlot<u32>> = Buffer::new(cap(8));
    for i in 0..8 {
        buffer.get_mut(SlotIndex(i)).set(i as u32);
    }
    assert!(!buffer.all_empty());

    let before = std::ptr::from_ref(buffer.get(SlotIndex(0))) as usize;
    buffer.clear();
    let after = std::ptr::from_ref(buffer.get(SlotIndex(0))) as usize;

    assert!(buffer.all_empty(), "clear must empty every slot");
    assert_eq!(buffer.len(), 8, "and keep every slot");
    assert_eq!(before, after, "and not reallocate — that is why clear exists at all");
}

#[test]
fn a_buffer_is_never_empty_because_a_capacity_is_never_zero() {
    let buffer: Buffer<TypedSlot<u8>> = Buffer::new(cap(1));
    assert!(!buffer.is_empty());
    assert_eq!(buffer.len(), buffer.capacity().get());
}

#[test]
fn is_empty_and_all_empty_disagree_on_a_freshly_built_buffer() {
    // The two questions named `is_empty`, on the one buffer where both answers
    // are visible together: `is_empty` asks whether the buffer has zero slots
    // (never true — a `Capacity` cannot be zero) and `all_empty` asks whether
    // every slot holds nothing (true here, since nothing has been written yet).
    // Elsewhere in this suite each is asserted alone; this pins the disagreement
    // itself, on the one buffer, in one place (-> docs/pitfall/001 BF38).
    let buffer: Buffer<TypedSlot<u8>> = Buffer::new(cap(4));
    assert!(!buffer.is_empty(), "is_empty asks about slot count, which is never zero");
    assert!(buffer.all_empty(), "all_empty asks about slot contents, which start empty");
}

#[test]
fn the_same_buffer_type_serves_both_slot_shapes() {
    // Feature 182's "both use the same claim, gating and drain" reaching down to
    // storage: one `Buffer` definition, two unrelated slot shapes, no branch.
    let mut typed: Buffer<TypedSlot<u32>> = Buffer::new(cap(4));
    let mut bytes: Buffer<BytesSlot<8>> = Buffer::new(cap(4));

    typed.at_mut(Seq(5)).set(42);
    bytes.at_mut(Seq(5)).write(b"hello").unwrap();

    assert_eq!(typed.get(SlotIndex(1)).get(), Some(&42));
    assert_eq!(bytes.get(SlotIndex(1)).read(), b"hello");

    assert_eq!(typed.len(), bytes.len(), "capacity means the same thing to both");
}

#[test]
fn iteration_visits_slots_in_index_order() {
    let mut buffer: Buffer<TypedSlot<usize>> = Buffer::new(cap(8));
    for i in 0..8 {
        buffer.get_mut(SlotIndex(i)).set(i);
    }

    let seen: Vec<usize> = buffer.iter().map(|s| *s.get().unwrap()).collect();
    assert_eq!(seen, (0..8).collect::<Vec<_>>());
}

#[test]
fn mutable_iteration_reaches_every_slot() {
    let mut buffer: Buffer<TypedSlot<usize>> = Buffer::new(cap(8));
    for (i, slot) in buffer.iter_mut().enumerate() {
        slot.set(i * 2);
    }

    let seen: Vec<usize> = buffer.iter().map(|s| *s.get().unwrap()).collect();
    assert_eq!(seen, (0..8).map(|i| i * 2).collect::<Vec<_>>());
}

#[test]
fn a_borrowed_buffer_iterates_directly() {
    let mut buffer: Buffer<TypedSlot<u8>> = Buffer::new(cap(4));
    for slot in &mut buffer {
        slot.set(1);
    }
    assert_eq!((&buffer).into_iter().filter(|s| !s.is_empty()).count(), 4);
}

#[test]
#[should_panic(expected = "index out of bounds")]
fn an_index_past_the_capacity_panics_rather_than_wrapping() {
    // Deliberately not an error return. A SlotIndex that came from `ring_index`
    // cannot be out of range, so one that is means two rings' capacities were
    // mixed — a defect, and silently folding it would hide the mixing.
    let buffer: Buffer<TypedSlot<u8>> = Buffer::new(cap(4));
    let _ = buffer.get(SlotIndex(4));
}
