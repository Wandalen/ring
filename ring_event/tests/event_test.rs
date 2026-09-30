//! One publish path and one drain path, over two unrelated slot shapes.
//!
//! Claims `docs/feature/182_typed_slot_and_bytes_slot.md`, whose reached-test
//! reads: "`TypedSlot<T>` and `BytesSlot` both round-trip through the identical
//! claim/publish/drain path; a `T` written and read back is byte-identical, and
//! a byte payload of arbitrary length up to slot size is byte-identical."
//!
//! `ring_slot/tests/slot_test.rs` already claims the byte-identity halves
//! against each shape directly. What it cannot claim, because it holds no such
//! thing, is *identical path* — two shapes each round-tripping through their
//! own code is the reading that criterion is written to exclude.
//!
//! So the load-bearing tests here are the ones that never name a shape:
//! `round_trip` and `land_and_read` below are generic, monomorphised once per
//! shape from a single body, and driven over real `ring_store` storage. If a
//! shape ever needed a special case, one of them would stop compiling — which
//! is a stronger signal than an assertion failing, because it cannot be
//! satisfied by adjusting a number.

use ring_event::{Fill, Peek, drain_from, publish_into, recycle};
use ring_slot::{BytesSlot, Slot, TypedSlot};
use ring_store::Buffer;
use ring_types::{Capacity, RingError, Seq};

/// The whole point: one body, no shape named, used by both shapes below.
fn round_trip<S, P>(slot: &mut S, payload: P) -> Option<S::Out<'_>>
where
    S: Peek,
    P: Fill<S>,
{
    publish_into(slot, payload).expect("the payload fits");
    drain_from(slot)
}

/// The same, but through real storage addressed by a sequence — the shape the
/// ring itself uses.
fn land_and_read<S, P>(buffer: &mut Buffer<S>, seq: Seq, payload: P) -> bool
where
    S: Peek + Slot + Default,
    P: Fill<S>,
{
    publish_into(buffer.at_mut(seq), payload).is_ok() && drain_from(buffer.at(seq)).is_some()
}

fn cap(n: usize) -> Capacity {
    Capacity::new(n).expect("test capacities are powers of two")
}

// --------------------------------------------------- the identical-path claim

#[test]
fn one_generic_body_round_trips_a_typed_slot() {
    let mut slot = TypedSlot::empty();
    assert_eq!(round_trip(&mut slot, 42u32), Some(&42));
}

#[test]
fn the_same_generic_body_round_trips_a_bytes_slot() {
    let mut slot = BytesSlot::<16>::empty();
    assert_eq!(round_trip(&mut slot, &b"payload"[..]), Some(&b"payload"[..]));
}

#[test]
fn both_shapes_land_in_storage_through_the_same_two_calls() {
    let mut typed: Buffer<TypedSlot<u64>> = Buffer::new(cap(8));
    let mut bytes: Buffer<BytesSlot<32>> = Buffer::new(cap(8));

    for raw in 0u64..24 {
        assert!(land_and_read(&mut typed, Seq(raw), raw));
        assert!(land_and_read(&mut bytes, Seq(raw), &raw.to_le_bytes()[..]));
    }
}

#[test]
fn a_typed_payload_survives_storage_byte_identically() {
    let mut buffer: Buffer<TypedSlot<[u8; 8]>> = Buffer::new(cap(4));
    let original = [1u8, 2, 3, 250, 251, 252, 0, 255];

    publish_into(buffer.at_mut(Seq(9)), original).unwrap();

    assert_eq!(drain_from(buffer.at(Seq(9))), Some(&original));
}

#[test]
fn a_byte_payload_of_every_length_up_to_slot_size_survives_storage() {
    let mut buffer: Buffer<BytesSlot<8>> = Buffer::new(cap(4));
    let source = [9u8, 8, 7, 6, 5, 4, 3, 2];

    for length in 0..=8 {
        let payload = &source[..length];
        publish_into(buffer.at_mut(Seq(0)), payload).unwrap();

        assert_eq!(
            drain_from(buffer.at(Seq(0))),
            if length == 0 { None } else { Some(payload) },
            "length {length} did not survive"
        );
    }
}

// ---------------------------------------------------------- the read half's None

#[test]
fn an_unpublished_typed_slot_reads_as_nothing() {
    let slot = TypedSlot::<u32>::empty();
    assert_eq!(drain_from(&slot), None);
}

#[test]
fn an_unpublished_bytes_slot_reads_as_nothing_rather_than_as_an_empty_payload() {
    // The distinction feature 170's handshake rests on: "nobody has published
    // here" and "somebody published nothing" are different states, and a drain
    // must be able to tell them apart.
    let slot = BytesSlot::<8>::empty();
    assert_eq!(drain_from(&slot), None);
}

#[test]
fn a_zero_length_publish_is_indistinguishable_from_unpublished_and_says_so() {
    // The honest limit of the previous test: a `BytesSlot` records length, and a
    // zero-length write leaves the same length an untouched slot has. This is
    // documented here rather than hidden, because a caller who needs the two
    // apart must carry the distinction elsewhere — in the handshake, not the slot.
    let mut slot = BytesSlot::<8>::empty();
    publish_into(&mut slot, &[][..]).unwrap();

    assert_eq!(drain_from(&slot), None, "a zero-length payload reads as nothing");
}

// ------------------------------------------------------------------- refusals

#[test]
fn a_byte_payload_longer_than_the_slot_is_refused_and_changes_nothing() {
    let mut slot = BytesSlot::<4>::empty();
    publish_into(&mut slot, &b"abcd"[..]).unwrap();

    // Not `Full`: a payload that does not fit is a configuration error, and no
    // amount of draining shrinks it. The same distinction `ring_batch` draws
    // between `BatchTooLarge` and `Full`, applied one level down.
    let refused = publish_into(&mut slot, &b"abcde"[..]);
    assert_eq!(refused, Err(RingError::BatchTooLarge { requested: 5, capacity: 4 }));
    assert!(refused.unwrap_err().is_configuration(), "a caller must not retry this");

    assert_eq!(drain_from(&slot), Some(&b"abcd"[..]), "the refused write left the old one");
}

#[test]
fn a_typed_publish_cannot_fail() {
    // Not a tautology worth skipping: it pins that the shared signature's Result
    // is the byte shape's need, and that the typed shape pays no runtime check
    // for it.
    let mut slot = TypedSlot::empty();
    for i in 0..100u32 {
        assert_eq!(publish_into(&mut slot, i), Ok(()));
    }
    assert_eq!(drain_from(&slot), Some(&99));
}

// ------------------------------------------------------------------- recycling

/// `drain_from` reads and does not empty — for both shapes, identically. Named
/// after the trap rather than after the mechanism: a caller who reads "drain"
/// as "remove" gets a ring every slot of which reports occupied after a full
/// pass, with no compile error and no runtime error to say so. That is why the
/// third call exists and why the function's own documentation opens by denying
/// its name. See `lifecycle/001` SL29.
#[test]
fn draining_reads_the_slot_without_emptying_it() {
    let mut typed = TypedSlot::empty();
    let mut bytes = BytesSlot::<8>::empty();

    publish_into(&mut typed, 7u32).unwrap();
    publish_into(&mut bytes, &b"payload"[..]).unwrap();

    assert_eq!(drain_from(&typed), Some(&7));
    assert_eq!(drain_from(&bytes), Some(&b"payload"[..]));

    assert!(!typed.is_empty(), "the typed slot is still occupied after being drained");
    assert!(!bytes.is_empty(), "and so is the byte slot");

    // Draining again returns the same payload, which is the observable form of
    // the same fact: nothing was consumed.
    assert_eq!(drain_from(&typed), Some(&7));
    assert_eq!(drain_from(&bytes), Some(&b"payload"[..]));

    recycle(&mut typed);
    recycle(&mut bytes);

    assert!(typed.is_empty(), "the third call is the one that empties it");
    assert!(bytes.is_empty(), "for both shapes, through the same generic body");
}

#[test]
fn recycling_empties_either_shape_through_the_same_call() {
    let mut typed = TypedSlot::empty();
    let mut bytes = BytesSlot::<8>::empty();

    publish_into(&mut typed, 1u8).unwrap();
    publish_into(&mut bytes, &b"x"[..]).unwrap();

    recycle(&mut typed);
    recycle(&mut bytes);

    assert_eq!(drain_from(&typed), None);
    assert_eq!(drain_from(&bytes), None);

    // EV12: pin recycle's "identical path" claim at the value level too, not
    // only through `drain_from`/`is_empty` — a recycled byte slot must compare
    // and print exactly as a fresh one, through the shape's own `PartialEq`/
    // `Debug`, the same surface a caller would actually inspect.
    assert_eq!(bytes, BytesSlot::<8>::empty(), "a recycled byte slot equals a fresh one");
}

#[test]
fn recycling_a_slot_that_was_never_published_is_harmless() {
    let mut slot = TypedSlot::<u8>::empty();
    recycle(&mut slot);
    recycle(&mut slot);
    assert_eq!(drain_from(&slot), None);
}

#[test]
fn a_recycled_storage_slot_stops_returning_the_previous_lap() {
    // The property `ring_shutdown`'s reset depends on: a slot cleared during
    // recycling must not hand the next lap the previous world's payload.
    let mut buffer: Buffer<TypedSlot<u32>> = Buffer::new(cap(2));

    publish_into(buffer.at_mut(Seq(0)), 7u32).unwrap();
    assert_eq!(drain_from(buffer.at(Seq(0))), Some(&7));

    recycle(buffer.at_mut(Seq(0)));
    assert_eq!(drain_from(buffer.at(Seq(2))), None, "sequence 2 folds onto the cleared slot");
}

// ---------------------------------------------------------------- trait shapes

#[test]
fn fill_is_implemented_on_the_payload_so_a_new_payload_needs_no_slot_change() {
    // Demonstrated by using the trait directly rather than through the free
    // function: this is what a caller adding a payload kind would write.
    let mut slot = TypedSlot::empty();
    Fill::fill(3u8, &mut slot).unwrap();
    assert_eq!(slot.get(), Some(&3));

    let mut bytes = BytesSlot::<4>::empty();
    Fill::fill(&b"ab"[..], &mut bytes).unwrap();
    assert_eq!(bytes.read(), b"ab");
}

#[test]
fn peek_is_implemented_on_the_slot_so_the_reader_needs_no_payload_type() {
    let mut typed = TypedSlot::empty();
    typed.set(5u16);
    assert_eq!(Peek::peek(&typed), Some(&5));

    let mut bytes = BytesSlot::<4>::empty();
    bytes.write(b"cd").unwrap();
    assert_eq!(Peek::peek(&bytes), Some(&b"cd"[..]));
}

#[test]
fn peek_agrees_with_the_slots_own_emptiness() {
    // Two independent readings of one state; a divergence between them would let
    // a drain read a slot the handshake considers empty.
    let mut typed = TypedSlot::<u8>::empty();
    let mut bytes = BytesSlot::<4>::empty();

    assert_eq!(typed.peek().is_none(), typed.is_empty());
    assert_eq!(bytes.peek().is_none(), bytes.is_empty());

    typed.set(1);
    bytes.write(b"z").unwrap();

    assert_eq!(typed.peek().is_none(), typed.is_empty());
    assert_eq!(bytes.peek().is_none(), bytes.is_empty());
}
