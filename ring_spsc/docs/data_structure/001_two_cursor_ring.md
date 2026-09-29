# Data Structure: Two-Cursor Ring Without Per-Slot State

### Scope

- **Purpose**: Give the SPSC ring its field-level shape, and account for the per-slot publication state it does *not* carry — the structural saving that follows from having one producer.
- **Responsibility**: The fields, their ownership, their cache-line placement, and the operations over them with costs.
- **In Scope**: The storage, the two cursors, the padding contract, the derived quantities.
- **Out of Scope**: The slot's internal byte layout, which is [`ring_slot`](../../../ring_slot/readme.md)'s; the procedures over this structure (→ [`algorithm/`](../algorithm/readme.md)); the multi-producer variant's stamps (→ [`ring_mpsc`](../../../ring_mpsc/docs/data_structure/001_sequence_stamped_ring.md)).

### Abstract

Three fields: a fixed slot array and two cursors. **The sibling MPSC ring
needs a fourth — a per-slot sequence stamp — and this one does not**, which is
the clearest structural statement of what single-producer buys.

A stamp exists in `ring_mpsc` to answer "is slot *i* published?" for a slot
whose publication may have been overtaken: producer 2 can publish sequence 7
while producer 1 still holds sequence 6, so the producer cursor's value does
not imply that everything below it is readable. Here nothing can overtake
anything, so the producer cursor alone answers the question for every slot at
once (→ [Slot State Without Holes](../lifecycle/003_slot_state_without_holes.md)).

The saving is not only memory. It removes a stamp store from the publish path,
a stamp load from the drain path, and the entire question of what value a
stamp is initialized to — which is where the sibling crate's construction
instance records a real initialization hazard.

### Structure

| Field | Type | Written by | Read by | Placement |
|-------|------|-----------|---------|-----------|
| `slots` | `[ T; CAPACITY ]`, allocated once | Producer | Consumer | Contiguous; one allocation at construction |
| `producer_cursor` | `PaddedCursor` (`ring_cursor`) | Producer only | Both | Its own 64-byte cache line |
| `consumer_cursor` | `PaddedCursor` (`ring_cursor`) | Consumer only | Both | Its own 64-byte cache line |

**Each cursor has exactly one writer.** That is the property the whole crate
rests on: a plain load of your own cursor is sound, and only the *other*
thread's cursor needs an acquire. In `ring_mpsc` the producer cursor has many
writers and every touch of it is a read-modify-write.

#### Cursor cache-line separation is a contract, not an optimization

The two cursors are written by different threads on every operation. Placed in
one cache line they share it, and each write invalidates the other core's copy
— the producer's publish store forces the consumer to re-fetch a line whose
only relevant content it already had. Throughput collapses under a workload
that has no logical contention at all.

The false-sharing risk this creates is real, and
[`ring_align`](../../../ring_align/readme.md) exists
to supply the padding. This crate's own reached-test asserts it mechanically:
`align_of::<PaddedCursor>() == 64`, `size_of::<PaddedCursor>() == 64`, and two
`PaddedCursor` values in one struct at least 64 bytes apart. That is a
structural assertion, not a benchmark — the padding either holds or it does
not.

**Nothing here is per-slot.** No stamp, no flag, no generation counter. The
slot array carries payload and nothing else, which is what lets
[`ring_slot`](../../../ring_slot/readme.md)'s typed and bytes translators write
straight into it.

#### Derived, not stored

| Quantity | Computed as | Cost |
|----------|-------------|------|
| Slot index for sequence *s* | `s & ( CAPACITY - 1 )` | One mask — power-of-two `Capacity` is what makes it a mask, not a division |
| Occupancy | `producer - consumer` | One subtraction |
| Free capacity | `CAPACITY - ( producer - consumer )` | One subtraction, one compare (→ [Free Capacity](../type/002_free_capacity.md)) |
| Available to drain | `producer - consumer` | Same as occupancy — the two coincide here, and do not in `ring_mpsc` |

**The last row is the structural payoff restated.** In the multi-producer ring
"how full is it" and "how much can I read" are different questions with
different answers; here they are one subtraction.

### Operations

| Operation | Touches | Cost | Synchronization |
|-----------|---------|------|-----------------|
| `push` | `producer_cursor` (r/w), `consumer_cursor` (r), one slot (w) | Load, acquire load, compare, memcpy, release store | One release store |
| `drain` | `consumer_cursor` (r/w), `producer_cursor` (r), N slots (r) | Acquire load, subtract, N reads, release store | One release store per batch |
| `free_capacity` | `producer_cursor` (r), `consumer_cursor` (r) | Acquire load, subtract | None |
| `is_empty` | Both cursors (r) | Acquire load, compare | None |

**No operation performs a read-modify-write.** Not one. That is
[No Lock in the Path](../invariant/002_no_lock_in_the_path.md) stated at the
structure level, and it is the concrete difference the benchmark is expected
to measure.

**Settled at this grain.** `slots` is a
[`ring_store::Buffer< UnsafeCell< S > >`](../../../ring_store/readme.md) — a
`Box< [ UnsafeCell< S > ] >` of `Default`-constructed slots, allocated once —
and `CAPACITY` is a runtime `Capacity` field carried by the `CursorPair` rather
than a const parameter.

Neither was a close call in the end. `MaybeUninit` was unnecessary because the
family's slot types are already `Default` and already model emptiness
(`TypedSlot` is an `Option`, `BytesSlot` a buffer plus a length), so there is
no uninitialized state to track — and adding one would have put per-slot state
back into the very structure this instance exists to say has none. A const
generic `CAPACITY` would have made every downstream type generic over it for a
mask that costs one instruction either way, and would have made
`Ring::with_config` — which reads a capacity from a runtime `RingConfig` —
impossible to write.

The `UnsafeCell` is on **each slot**, not around the whole `Buffer`. It wrapped
the buffer originally, on the reasoning that `Buffer::new` required
`S : Slot` and `UnsafeCell< S >` is not a `Slot`. That arrangement was unsound:
reaching a slot through `( *cell.get() ).at_mut( seq )` materialises
`&mut Buffer< S >` — an exclusive claim over the *entire* allocation — so the
producer writing one slot and the consumer reading a different one aliased the
whole buffer, even though the cursor invariant guarantees the two slots are
disjoint. Miri's data-race detector reports it as a retag conflict on
`Buffer< S >` itself rather than on any slot, which is the tell: the two threads
never touched the same record.

The fix was to bound `Buffer::new` on `Default` alone — allocation never needed
anything `Slot` offers — which makes `Buffer< UnsafeCell< S > >` constructible.
A per-slot cell claims exactly the slot being touched, matching what the cursors
actually guarantee. `ring_store` still knows nothing of this crate: it stores
whatever element type it is given and carries no `unsafe`, so every opt-out
stays here by design.

The two cursors are a `CursorPair` rather than two loose `PaddedCursor` fields
— the pair already carries the capacity, already guarantees the cache-line
separation this instance requires, and already has a test for it.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_uncontended_claim_and_publish.md](../algorithm/001_uncontended_claim_and_publish.md) | The producer-side operation over these fields |
| [../algorithm/002_single_consumer_drain.md](../algorithm/002_single_consumer_drain.md) | The consumer-side operation |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_consumer_surface.md](../api/002_consumer_surface.md) | The borrowed-batch question is about pointers into `slots` |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_family_dependency_seam.md](../integration/001_family_dependency_seam.md) | `ring_store`, `ring_cursor` and `ring_align` supply the three fields |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_exactly_one_producer_one_consumer.md](../invariant/001_exactly_one_producer_one_consumer.md) | The single-writer-per-cursor property the Structure table records |
| [../invariant/002_no_lock_in_the_path.md](../invariant/002_no_lock_in_the_path.md) | The Operations table's synchronization column |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/003_slot_state_without_holes.md](../lifecycle/003_slot_state_without_holes.md) | Why no per-slot field is needed |
| [../lifecycle/004_ring_occupancy.md](../lifecycle/004_ring_occupancy.md) | The states the derived occupancy expresses |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_producer_cursor.md](../type/001_producer_cursor.md) | The single-writer cursor |
| [../type/002_free_capacity.md](../type/002_free_capacity.md) | The derived quantity, and why it binds here |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_align/readme.md`](../../../ring_align/readme.md) | Supplies the padding that keeps the two cursors off one cache line |
| [`ring_store/readme.md`](../../../ring_store/readme.md) | The storage this structure composes — holds no cursor and no ordering state of its own |

### Tests

| File | Relationship |
|------|--------------|
| `tests/spsc_test.rs` | `one_producer_and_one_consumer_exchange_one_hundred_thousand_items` drives every operation in the table |
| `tests/spsc_test.rs` | `a_new_ring_is_empty_and_fully_free` asserts `Ring::on_distinct_lines()` — feature 169's assertion, checked at this crate's composition rather than only at [`ring_cursor`](../../../ring_cursor/readme.md). The method is re-exposed on `Ring` precisely so a test at this level can make it |

### SP9 — The Cursors Come From `ring_cursor` as a Pair, Not as Two Fields

```sh
cd "$(git rev-parse --show-toplevel)"/ring_spsc
printf 'CursorPair uses:       '; grep -c 'CursorPair' src/lib.rs
printf 'PaddedCursor uses:     '; grep -c 'PaddedCursor' src/lib.rs
printf 'the separation test:   '; grep -oE 'fn on_distinct[a-z_]*|pub fn on_distinct_lines' src/lib.rs | head -1
```

Live output:

```
CursorPair uses:       7
PaddedCursor uses:     0
the separation test:   pub fn on_distinct_lines
```

`ring_mpsc` declares two `PaddedCursor` fields directly; this crate takes one
`CursorPair` and gets the separation as part of the type. The observable
difference is `on_distinct_lines()` — a method here, a test-only concern there.

**The layout guarantee therefore lives one crate down**, which is where it is
checkable once for every consumer rather than per ring.

### SP10 — The Slot Buffer Is the Same Shape as the Sibling's

```sh
cd "$(git rev-parse --show-toplevel)"
for c in ring_spsc ring_mpsc; do
  printf '%-11s ' "$c"; grep 'slots : ' $c/src/lib.rs
done
```

Live output:

```
ring_spsc     slots : Buffer< UnsafeCell< S > >,
      slots : Buffer::new( capacity ),
ring_mpsc     slots : Buffer< UnsafeCell< S > >,
      slots : Buffer::new( capacity ),
```

Identical field type in two crates. The per-slot cell rather than a cell around
the whole `Buffer` is the sound form — the outer form materialises
`&mut Buffer< S >` per write and aliases the entire allocation — and the
reasoning is recorded in `ring_mpsc`'s `data_structure/001` rather than here.

**Recorded as duplication rather than as reuse** because it is genuinely the
same decision made in two files, and nothing links them: a fix to one does not
reach the other.
