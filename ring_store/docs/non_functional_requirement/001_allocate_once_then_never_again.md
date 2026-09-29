# Non-Functional Requirement: Allocate Once, Then Never Again

### Scope

**Purpose:** Record the allocate-once property measured end to end — one
allocation per buffer at every capacity and both slot shapes, sized exactly
`capacity * size_of::< S >()` with no overhead, and zero allocations across ten
thousand operations — and record that the guarantee stops at the slot boundary.

**Responsibility:** The allocation NFR: what is measured, what it covers, and
where it hands off.

**In Scope:** `ring_store/src/lib.rs:19-22, 87-93`.

**Out of Scope:** The three-step construction is
[`algorithm/002`](../algorithm/002_one_allocation_n_defaults.md). The buffer's
own footprint is
[`data_structure/001`](../data_structure/001_three_words_whatever_the_slot_costs.md).

---

### BF42 — One Allocation, Exactly Sized, and Nothing After

The crate states the requirement in its module comment:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A3 -F '//! What is left is small enough to state completely: `capacity` slots allocated' ring_store/src/lib.rs
```

Live output:

```
//! What is left is small enough to state completely: `capacity` slots allocated
//! once, addressed by [`ring_types::SlotIndex`], with the fold from a sequence
//! delegated to `ring_index`. No `unsafe` — a `Box<[S]>` of `Default` slots is
//! allocated in one go, which is what "exactly N slots once" asks for.
```

Measured against a counting global allocator, at three capacities and both slot
shapes:

```
--- (1) one allocation, exact bytes, at every capacity ---
  TypedSlot<u32>  capacity     1: 1 alloc,        8 bytes, expected        8 = 1 * 8
  TypedSlot<u32>  capacity    16: 1 alloc,      128 bytes, expected      128 = 16 * 8
  TypedSlot<u32>  capacity  1024: 1 alloc,     8192 bytes, expected     8192 = 1024 * 8
  BytesSlot<4096> capacity   256: 1 alloc,  1050624 bytes, expected  1050624 = 256 * 4104
```

And across a working life:

```
--- (2) zero allocations across the working life ---
  10,000 at_mut + at, one all_empty, one iter, one clear: 0 allocations
```

**Finding.** Every part of the claim holds and each part is separately worth
having. *One* allocation — not two, so the `Vec::with_capacity` and the
`into_boxed_slice` do not each pay. *Exactly sized* — the byte count is
`capacity * size_of::< S >()` at every measurement, including a 4104-byte slot
where any per-element padding or bookkeeping would be immediately visible in a
million-byte total. *Nothing after* — ten thousand sequence-addressed writes and
reads, a full sweep, a full iteration and a `clear` add zero.

None of this is asserted anywhere. `exactly_n_slots_are_allocated_for_capacity_n`
compares `len()` to `capacity().get()` at six capacities, which catches a wrong
`resize_with` argument and says nothing about allocation count
([`invariant/001`](../invariant/001_capacity_equals_length_always.md) BF22). The
suite's own module comment names this as the clause it cannot assert directly.
So the strongest property this crate has is also the one with no test, and the
measurement above is the first record of it.

---

### BF43 — The Guarantee Stops at the Slot Boundary, and `BytesSlot` Is Why the Second Shape Exists

The same probe, one line further:

```
  one String payload:                                     1 allocations
```

**Finding.** Allocate-once is a property of the *slot array*, not of the traffic
through it. A `Buffer< TypedSlot< String > >` allocates once for its slots and
then once more for every payload pushed into it, and the buffer neither knows nor
can prevent that — the payload is constructed by the caller and moved in through
a borrow.

That boundary is exactly why the family has a second slot shape.
`BytesSlot< N >` holds its payload inline in a `[ u8; N ]`, so a
`Buffer< BytesSlot< N > >` keeps the allocate-once property end to end: one
allocation at construction, none for any payload, ever. That is the whole
argument for paying `N` bytes per slot whether or not a payload uses them, and
the argument is not stated in this crate, where the allocation claim lives.

Nor is it stated where the shapes are defined — `ring_slot` records that
`BytesSlot` has one library consumer and that the bench meant to justify two
shapes never instantiates it
([`integration/002`](../../../ring_slot/docs/integration/002_where_the_second_shape_stops.md)
SL3, SL4). So the family carries a second slot shape whose reason for existing is
a non-functional property of a *different* crate, measured here for the first
time and asserted nowhere.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`non_functional_requirement/002`](002_bounded_work_per_operation.md) | The other NFR, on time rather than memory |
| [`algorithm/002`](../algorithm/002_one_allocation_n_defaults.md) | The three steps that produce the single allocation |
| [`invariant/001`](../invariant/001_capacity_equals_length_always.md) | The clause the suite does assert, and what it misses |
| [`data_structure/001`](../data_structure/001_three_words_whatever_the_slot_costs.md) | The three words the array hangs off |
| `ring_slot` — [`integration/002`](../../../ring_slot/docs/integration/002_where_the_second_shape_stops.md) | Where the second shape stops being used |

### Sources

| Fact | Where |
|------|-------|
| The stated requirement | `ring_store/src/lib.rs:19-22` |
| The construction | `ring_store/src/lib.rs:87-93` |
| Allocation counts and byte totals | Release probe under a counting global allocator, quoted above |
| Zero allocations across the working life | Same probe |
| A `String` payload's own allocation | Same probe |

### Tests

| Test | Covers |
|------|--------|
| `exactly_n_slots_are_allocated_for_capacity_n` | The slot count, not the allocation count |
| `clear_empties_every_slot_and_keeps_the_allocation` | That `clear` does not reallocate — by address, at one capacity |
| *(to create)* | A counting allocator asserting one allocation at construction and zero thereafter |
| *(to create)* | The same allocator over `Buffer< BytesSlot< N > >`, asserting zero for a payload write |
