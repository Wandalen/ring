# Invariant: Capacity Equals Length, Always

### Scope

**Purpose:** Record that `capacity().get() == len()` holds for every buffer that
can be constructed, that nothing after `new` can break it because nothing writes
either value, and that the reached-test names an API — indexed `set` —
that this type does not have.

**Responsibility:** The relation between the two fields, and the exact wording of
the claim this crate is answering.

**In Scope:** `ring_store/src/lib.rs:88-93, 146-171`;
`ring_store/tests/buffer_test.rs:1-6, 36-46`.

**Out of Scope:** Why both readings exist is
[`decisions/002`](../decisions/002_a_length_kept_to_be_checked_against_itself.md).
The non-aliasing clause is
[`invariant/002`](002_two_distinct_indices_never_alias.md).

---

### BF22 — The Invariant Holds by Construction and Nothing Can Subsequently Break It

`new` sizes the slice from the capacity and stores both:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A5 -F '  pub fn new( capacity : Capacity ) -> Self' ring_store/src/lib.rs
```

Live output:

```
  pub fn new( capacity : Capacity ) -> Self
  {
    let mut slots = Vec::with_capacity( capacity.get() );
    slots.resize_with( capacity.get(), S::default );
    Self { slots : slots.into_boxed_slice(), capacity }
  }
```

`resize_with` sets the length to exactly `capacity.get()`, and
`into_boxed_slice` preserves it. Measured at capacity 1024 and asserted across
six capacities by the suite:

```
--- (6) capacity stored, and capacity derivable ---
  capacity().get() = 1024, len() = 1024, equal = true
```

**Finding.** The invariant is established in one line and cannot subsequently be
violated, because no function writes either value after construction. There is no
`resize`, no `push`, no `truncate`, no second constructor, and the fields are
private — so the only code that could break the relation is the three lines
above.

That makes this an unusually strong invariant and an unusually weak test target.
`exactly_n_slots_are_allocated_for_capacity_n` compares the two readings at six
capacities, which is exactly right for catching a wrong `resize_with` argument,
and there is no state space beyond that to explore: a buffer that survives `new`
with the relation intact keeps it for its whole life by construction rather than
by discipline. The property worth asserting alongside it is the one BF12
measures and no test covers — that `new` reached that state in a single
allocation.

---

### BF23 — The Reached-Test Names an API the Type Does Not Have

The suite quotes the claim it is answering:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A3 -F '//! Claims `docs/feature/168_ring_buffer_storage.md`, whose reached-test reads:' ring_store/tests/buffer_test.rs
```

Live output:

```
//! Claims `docs/feature/168_ring_buffer_storage.md`, whose reached-test reads:
//! "A `Buffer<T>` of capacity `N` allocates exactly `N` slots once, exposes
//! indexed get/set, holds no cursor and no ordering state; two distinct slot
//! indices never alias."
```

Four clauses. The second is *exposes indexed get/set*, and `Buffer` has no `set`:

```sh
cd "$(git rev-parse --show-toplevel)"
grep 'pub fn set\|pub fn get' ring_store/src/lib.rs
```

Live output:

```
  pub fn get( &self, index : SlotIndex ) -> &S
  pub fn get_mut( &mut self, index : SlotIndex ) -> &mut S
```

`get` and `get_mut`. Writing a payload is the slot's operation, not the buffer's
— `TypedSlot::set` and `BytesSlot::write` live in `ring_slot`, and the buffer
reaches them only by handing back a `&mut S`.

**Finding.** The reached-test's wording predates the split between storage and slot,
and the crate answered the intent rather than the letter: indexed access in both
directions, with the write expressed as an exclusive borrow rather than as a
`set` taking a payload. That is the better design — a `Buffer::set( index, value
)` would have to know what a payload is, which is exactly the coupling that lets
one buffer type serve both slot shapes.

The residue is that the reached-test, which is the contract this crate is
verified against, describes a `set` nobody will find. The suite's own comment
enumerates *four* clauses and calls three of them "ordinary" without noting that
one of the three is discharged by a differently-named pair. Anyone auditing the
reached-test's claim against the crate meets the mismatch and has to re-derive the reasoning;
one clause reworded to *indexed shared and exclusive access* would retire it.

**Disposition:** declined — the reworded clause belongs in the external
feature document that `buffer_test.rs`'s module comment quotes verbatim as its
reached-test, which lives outside this corpus's own scope, so the fix is
recorded here rather than applied.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`invariant/002`](002_two_distinct_indices_never_alias.md) | The fourth clause of the same reached-test |
| [`decisions/002`](../decisions/002_a_length_kept_to_be_checked_against_itself.md) | Why the two readings are kept separate |
| [`algorithm/002`](../algorithm/002_one_allocation_n_defaults.md) | The single allocation that establishes this invariant |
| [`item/001`](../item/001_the_slot_as_the_buffer_sees_it.md) | Why the write is the slot's operation |
| [`api/002`](../api/002_six_ways_to_reach_a_slot.md) | The accessor pairs that answer the clause |

### Sources

| Fact | Where |
|------|-------|
| `new`, the only writer of either value | `ring_store/src/lib.rs:88-93` |
| The two readings | `ring_store/src/lib.rs:146-171` |
| The reached-test | `ring_store/tests/buffer_test.rs:3-6` |
| No `set` on the buffer | `ring_store/src/lib.rs` — no occurrence |
| The equality at capacity 1024 | Release probe, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `exactly_n_slots_are_allocated_for_capacity_n` | `len()` against `capacity()` at six capacities |
| `a_buffer_is_never_empty_because_a_capacity_is_never_zero` | The same equality at the smallest capacity |
| `indexed_get_and_set_round_trip` | The clause's "get/set", through `get_mut` and the slot's own `set` |
| *(to create)* | A counting allocator asserting the invariant is reached in one allocation |
