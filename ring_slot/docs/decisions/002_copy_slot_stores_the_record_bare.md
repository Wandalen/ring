# `CopySlot` stores the record bare, and a slot published without a write hands back the previous lap's record

Status: Accepted

## Context

`ring_slot::TypedSlot<T>` stores an `Option<T>`. For a record with no spare bit pattern the tag takes a word of its
own: `Option<u64>` is 16 bytes, so every push stores twice what it carries. Neither ring reads the tag.
`ring_spsc` states it on `Ring`: "A slot's state is never stored either. Free, published and drained follow from
comparing a sequence with the two cursors". `ring_mpsc` gates reads on its stamps.

Measured in `docs/benchmarks/001_the_tag_store_against_an_untagged_slot.md`, `ring_spsc` over an untagged slot
drops 0.84 ns per single-threaded push and pop, moves 26% more records in a single-threaded fill and drain at 64 and
1024 slots, and at 16384 slots moves 1004 M records/s against 53 single-threaded and 326 against 26 with two threads.
A tag written and never checked had measured as slow as `TypedSlot` earlier, so the cost is the store, not the
check.

The tag does carry one thing the cursors do not. Both rings publish a claimed slot when its guard drops, written or
not, and the tag is what lets a reader tell "published with a record" from "published with nothing".

The workspace denies `unsafe`, and `ring_slot` has no exemption. `ring_store::Buffer::new` fills every slot with
`Default` before any producer runs.

## Decision

Add `CopySlot<T>`, a third shape beside `TypedSlot` and `BytesSlot`: the record itself, `T: Copy + Default`, no
tag. `set` overwrites, `get` copies out. A fresh slot holds `T::default()`. Through `Slot`, `is_empty` is always
false, because the slot always holds a value, and `clear` writes the default back.

A slot published without a write hands the reader whatever it last held: the previous lap's record, or the default
on the first lap. That is documented as `CopySlot`'s pitfall, not prevented. `ring_spsc` gets a `try_push` over
`CopySlot` that always writes, so the trap is reachable only through a bare `claim` dropped unwritten.

`TypedSlot` stays the default everywhere it is today. `ring_core` and `ring_mpsc` do not change.

## Alternatives considered

- **`MaybeUninit<T>`, for any `T`.** It keeps `TypedSlot`'s reach and drops the tag. Reading a slot nobody wrote
  would then be undefined behaviour, so the rings' guards would have to stop publishing unwritten slots: a protocol
  change in both rings, and an `unsafe` exemption for `ring_slot`. Refused as far more than the measured gain needs,
  since the traffic that showed it is small `Copy` records.
- **Keep the tag and move its check to compile time.** The measured cost is the tag's store, so this saves the
  branch alone, at most about 0.1 ns per record.
- **Make the rings refuse to publish an unwritten `CopySlot`.** A per-slot "written" bit is the tag again, and a
  guard that tracks writes cannot see a write made through the `&mut` it hands out without wrapping it.

## Consequences

- `CopySlot` is the first shape whose occupancy `Slot` cannot report. `is_empty` answers false, and
  `ring_store::Buffer::all_empty` over `CopySlot`s is false from construction on. The rings never ask.
- A reader of a `CopySlot` ring cannot detect a stale duplicate. A producer that claims and drops unwritten delivers
  the previous lap's record a second time. Rust's type system does not prevent it; the pitfall and `try_push` do.
- Records of 64 and 256 bytes measured 30–46% slower untagged. The untagged strides are powers of two there, and 4K
  aliasing is a candidate cause, untested. A caller with wide records should measure before choosing `CopySlot`.
- Two-thread hand-off at 64 and 1024 slots follows where each ring's cursors land, not the slot, on a harness that
  builds rings on the stack. The record shows those cells both ways and does not use them for this decision.
- Revisit when the wide-record regression is explained, when a non-`Copy` record needs the untagged path, or when a
  caller needs to tell an empty publication from a written one on a `CopySlot` ring.

See [`src/lib.rs`](../../src/lib.rs).
