# `BytesSlot` marks occupancy with a length over a zeroed array, not with `MaybeUninit` or a flag byte

Status: Accepted

## Context

`ring_slot::BytesSlot<N>` carries an opaque payload of at most `N` bytes, for traffic decoded later by whoever knows
the wire format. The slot has to know how many of its bytes are live and when it counts as empty, because
`ring_slot::Slot::is_empty` and `Slot::clear` are what the rest of the family uses.

Two constraints shape the answer. The workspace sets `unsafe-code = "deny"`, and `ring_slot` has no exemption. A ring
allocates its slots once, through `ring_store::Buffer::new` filling every position with `Default`, and reuses them on
every lap.

## Decision

`BytesSlot<N>` is two fields, `bytes: [u8; N]` and `len: usize`. `BytesSlot::empty` zeroes all `N` bytes.
`BytesSlot::write` copies the payload in and sets `len` to its length, zero included. `Slot::clear` sets `len` to zero
and leaves the bytes. `is_empty` is `len == 0`, and `read`, `Debug` and `PartialEq` all look at the first `len` bytes
only.

## Alternatives considered

- **`MaybeUninit<[u8; N]>`.** It skips the zeroing, which is the usual reason to reach for it. Reading a partially
  written buffer then needs an `unsafe` assertion that the region read was written, and the crate would need an
  exemption from the workspace lint.
- **A flag byte per slot beside the length.** It would tell a deliberately published empty payload from a slot nobody
  touched. It was refused as not worth a byte per slot, because the ring already carries that distinction in the
  published-sequence handshake, and `ring_slot::TypedSlot<()>` sends a payload-free signal without the limitation.
  This argument is written on `ring_event::Peek::peek`, not in `ring_slot`.

## Consequences

- `write(b"")` produces the same value as `BytesSlot::empty()`. Both report `is_empty()`, and they compare equal. A
  caller that needs "somebody published nothing" reads it from the published sequence, or uses `TypedSlot<()>`, whose
  `Option` carries the distinction. The test `a_zero_length_write_empties_the_slot` pins the behaviour without naming
  this consequence, and nothing on `BytesSlot` itself mentions it.
- Every `BytesSlot<N>` costs `N` bytes of zeroing at construction, whether or not a byte is ever written. A ring pays
  that once per slot at allocation and never per publish.
- Because only the length moves, `clear` leaves the old payload in memory until a write of at least that length lands
  on the same slot. `Slot::clear` documents how long that residue lasts.
- Revisit when a consumer must tell an empty publication from an untouched slot and cannot read the published
  sequence.
- Revisit when slots stop being allocated once per ring, since the zeroing cost would then recur.

See [`src/lib.rs`](../../src/lib.rs).
