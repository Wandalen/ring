# Data Structure: Three Words, Whatever the Slot Costs

### Scope

**Purpose:** Record the struct's exact layout, that it is three machine words
regardless of what a slot costs, and that the one test standing in for feature
168's "holds no cursor and no ordering state" pins that number and openly names
what it cannot catch.

**Responsibility:** The buffer's own storage — what the struct contains, what it
costs, and how the absence of anything else is checked.

**In Scope:** `ring_store/src/lib.rs:58-63`;
`ring_store/tests/buffer_test.rs:8-24, 56-68`; the measured `size_of`.

**Out of Scope:** Why `capacity` is stored at all is
[`decisions/002`](../decisions/002_a_length_kept_to_be_checked_against_itself.md).
The bound on `S` is
[`data_structure/002`](002_the_default_bound_is_a_bound_on_the_type.md).

---

## The Struct

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A5 -F '#[ derive( Debug ) ]' ring_store/src/lib.rs
```

Live output:

```
#[ derive( Debug ) ]
pub struct Buffer< S >
{
  slots : Box< [ S ] >,
  capacity : Capacity,
}
```

Two fields. A fat pointer to a heap slice, and a validated `usize`.

---

### BF18 — Three Words, Constant Across a Five-Hundred-Fold Change in Slot Size

```
--- (1) what a Buffer is made of ---
  size_of Buffer< TypedSlot< u32 > >  = 24
  size_of Buffer< BytesSlot< 4096 > > = 24
  size_of Box< [ TypedSlot< u32 > ] > = 16
  size_of Capacity                    = 8
  the same three words whatever the slot costs
```

Pointer, length, capacity — 24 bytes on a 64-bit target, and the same 24 whether
a slot is four bytes or four kilobytes. Everything that scales lives behind the
pointer.

**Finding.** The type is a handle, and its size is independent of both capacity
and slot shape. That matters more than it looks: `ring_mpsc` and `ring_spsc` each
embed a `Buffer< S >` by value inside a `Ring< S >`, alongside a `Box< [
AtomicSeq ] >` of stamps and a `GatingSet`. A storage type whose inline footprint
grew with either parameter would push those neighbours across cache lines
differently for every instantiation, and the two rings' layouts would stop being
comparable across the benchmark candidates
`ring_bench` exists to compare.

Held constant, the buffer contributes a fixed 24 bytes to every ring in the
family, and the only thing that varies between candidates is the protocol state
around it — which is precisely the separation that lets several rings with
different protocols share one storage primitive.

**Disposition:** declined — this instance measures and endorses the current
layout as the property `ring_bench`'s comparable candidates depend on; it
names no defect in `ring_store/src/lib.rs`'s `Buffer< S >` struct and
proposes no change to it, only records why holding the size constant matters.

---

### BF19 — The Absence Is Pinned by a `size_of` Assertion That Names Its Own Holes

The reached-test contains a clause a passing test cannot establish, and
the suite says so in its module comment rather than pretending otherwise:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A16 -F '//! Three of those four clauses are ordinary. The fourth — *holds no cursor and' ring_store/tests/buffer_test.rs
```

Live output:

```
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
```

The assertion itself computes the expected size from the two permitted fields
rather than hard-coding 24:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A8 -F '  // The stand-in for "holds no cursor and no ordering state": a boxed slice' ring_store/tests/buffer_test.rs
```

Live output:

```
  // The stand-in for "holds no cursor and no ordering state": a boxed slice
  // (pointer + length) plus a Capacity. Anything else in the struct grows it.
  let expected = size_of::< Box< [ TypedSlot< u32 > ] > >() + size_of::< Capacity >();
  assert_eq!
  (
    size_of::< Buffer< TypedSlot< u32 > > >(),
    expected,
    "Buffer grew a field — a cursor or an ordering flag would land here"
  );
```

**Finding.** This is the right way to assert an absence and it is worth recording
as an example rather than a defect: the expected value is derived from the
permitted fields, so the test tracks a `Capacity` that changed width instead of
failing spuriously, and it still fails the moment a third field appears. The
module comment names both stand-ins, states exactly what each one misses — a
zero-sized ordering field, a cursor consulted only under contention — and admits
the real guarantee is that the struct is short enough to read.

One thing weakens it in practice. The assertion is instantiated at exactly one
slot type, `TypedSlot< u32 >`, so a field whose size depended on `S` — a
`PhantomData< S >` is zero-sized, but a `[ S; 1 ]` sentinel or an `Option< S >`
watermark would not be — would be caught for `u32` and could still shift the
answer for `BytesSlot< 4096 >`. The constancy measured in BF18 is real, is the
property the two rings' comparable layouts actually depend on, and is asserted
nowhere: adding a second `assert_eq!` at `BytesSlot< 4096 >` would cost one line
and close the only hole in this stand-in that the module comment does not
already name.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`data_structure/002`](002_the_default_bound_is_a_bound_on_the_type.md) | The requirement on `S` that the layout does not show |
| [`decisions/002`](../decisions/002_a_length_kept_to_be_checked_against_itself.md) | Why the third word is there |
| [`invariant/001`](../invariant/001_capacity_equals_length_always.md) | The relation between the two fields |
| [`type/001`](../type/001_one_parameter_three_impl_blocks_one_derive.md) | `Buffer< S >` as a type rather than as a layout |
| [`algorithm/002`](../algorithm/002_one_allocation_n_defaults.md) | What fills the space behind the pointer |

### Sources

| Fact | Where |
|------|-------|
| The struct | `ring_store/src/lib.rs:58-63` |
| The suite's own account of the absence clause | `ring_store/tests/buffer_test.rs:8-24` |
| The `size_of` assertion | `ring_store/tests/buffer_test.rs:59-67` |
| 24 bytes at both slot sizes | Release probe, quoted above |
| The reached-test | `ring_store/tests/buffer_test.rs:3-6` |

### Tests

| Test | Covers |
|------|--------|
| `a_buffer_is_exactly_its_slots_and_its_capacity` | The layout, at one slot type |
| `storage_survives_being_addressed_out_of_order` | The second stand-in for the absence clause |
| `exactly_n_slots_are_allocated_for_capacity_n` | That the space behind the pointer is exactly `capacity` slots |
| `tests/manual/readme.md` — M1, M2 | The human reading the module comment names as the real guarantee |
| *(to create)* | The same `size_of` assertion at a second slot type, pinning independence from `S` |
