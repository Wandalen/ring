# Data Structure: The Ring Is a Computation, Not a Layout

### Scope

**Purpose:** Record that no data structure in the family is circular — the
storage is a flat boxed slice and a number — and that the ring topology exists
only as this crate's fold, recomputed at every access and stored nowhere.

**Responsibility:** Where the ring's circularity lives, given that it is not in
any struct's layout.

**In Scope:** `ring_store/src/lib.rs:59-63`; every field of type
`SlotIndex` in `/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/` — of which there are none.

**Out of Scope:** the one collection this crate builds is
[`data_structure/002`](002_the_one_collection_the_crate_builds.md). The fold's
cost is
[`non_functional_requirement/001`](../non_functional_requirement/001_what_the_fold_costs.md).

---

## The Storage, Whole

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A4 -F 'pub struct Buffer< S >' ring_store/src/lib.rs
echo '  -- and the methods that read it --'
command grep 'pub fn get\b\|pub fn get_mut\|pub fn at\b\|pub fn at_mut\|pub const fn capacity' ring_store/src/lib.rs
```

Live output:

```
pub struct Buffer< S >
{
  slots : Box< [ S ] >,
  capacity : Capacity,
}
  -- and the methods that read it --
  pub const fn capacity( &self ) -> Capacity
  pub fn get( &self, index : SlotIndex ) -> &S
  pub fn get_mut( &mut self, index : SlotIndex ) -> &mut S
  pub fn at( &self, seq : Seq ) -> &S
  pub fn at_mut( &mut self, seq : Seq ) -> &mut S
```

Two fields. Neither of them is a head, a tail, a wrap flag, or a link.

---

### IX49 — Nothing in the Layout Is Circular

**Finding.** A ring buffer implemented the classical way stores its own topology:
a head index, a tail index, sometimes a full/empty discriminator, and the wrap is
a branch on those values. `Buffer` stores none of that. It holds a flat
`Box< [ S ] >` — the same layout a `Vec` of fixed length has — plus the capacity
that sized it.

The circularity is entirely in `of`. Every access recomputes it:
`Buffer::at( seq )` folds through `ring_index` and subscripts, so the ring
topology is produced fresh at each read and each write and never written down.
Between two calls to `at`, there is no object anywhere in the process that knows
the array is a ring.

Three consequences follow, and each is visible elsewhere in this corpus.

- **The topology cannot go stale**, because there is nothing to go stale. A
  classical head/tail pair can disagree with the data; a recomputed fold cannot.
  This is why the crate's tests are enumerations over arithmetic rather than
  scenarios over a structure
  ([`pattern/002`](../pattern/002_stateless_arithmetic_over_borrowed_types.md)
  IX43).
- **The topology can be duplicated**, because it lives in a function rather than
  in a field. A second `& mask` elsewhere is a second ring over the same array,
  and the compiler has no way to notice
  ([`pattern/001`](../pattern/001_one_owner_for_one_arithmetic_fact.md) IX42).
- **The capacity must travel with the storage**, because the fold takes it as an
  argument rather than reading it from the array. `Buffer` holds both in one
  struct. `ring_mpsc`'s `stamps` field does not, which is the whole of
  [`pitfall/002`](../pitfall/002_the_second_fold_nobody_noticed.md).

---

### IX50 — `SlotIndex` Is Never Stored Anywhere in the Family

The output type of this crate's central function is never a field:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -rE '^\s*[a-z_]+ *: *(SlotIndex|Option< SlotIndex|Vec< SlotIndex)' --include=*.rs */ || echo '  (none — SlotIndex is never a struct field)'
```

Live output:

```
  (none — SlotIndex is never a struct field)
```

The three types' sizes and one run's heap cost below come from a standalone
`rustc -O` probe, not from the recipe above — the grep only shows that no field
of the type exists, it does not measure that type's size:

```
  size_of Seq            :   8
  size_of SlotIndex      :   8
  size_of Capacity       :   8
  size_of Vec<SlotIndex> :  24  (ptr + len + cap, on the stack)
  run-of-1: len 1 capacity 1  heap bytes 8
  run-of-1024: len 1024 capacity 1024  heap bytes 8192
  exactly sized in both cases: true
```

**Finding.** Zero fields of type `SlotIndex`, `Option< SlotIndex >`, or
`Vec< SlotIndex >` exist in any of the 33 crates. Every `SlotIndex` that has ever
existed at runtime was a temporary: produced by `of`, handed to a subscript,
discarded before the next statement.

That is the structural counterpart to IX49. Sequences are stored — in cursors, in
stamps, in `AtomicSeq` fields all over the family — because a sequence is state.
Slots are not stored, because a slot is a *view* of a sequence through a
capacity, and both of those are already held. Recomputing is cheaper than
caching for a value that costs one `and` instruction
([`non_functional_requirement/001`](../non_functional_requirement/001_what_the_fold_costs.md)
IX21).

It also explains why `SlotIndex`'s `pub` field has never caused a problem despite
enforcing nothing
([`type/002`](../type/002_the_sentence_the_public_field_contradicts.md) IX39). A
forged `SlotIndex` would have to be forged at the point of use, in the same
expression as the subscript, because there is nowhere to put one and no window in
which one survives. The type's safety comes from its lifetime, not from its
field.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`data_structure/002`](002_the_one_collection_the_crate_builds.md) | The one place a `SlotIndex` does outlive its expression, and what that costs |
| [`pattern/001`](../pattern/001_one_owner_for_one_arithmetic_fact.md) | The duplication a computed topology permits |
| [`pitfall/002`](../pitfall/002_the_second_fold_nobody_noticed.md) | What happens when capacity and storage stop travelling together |
| [`type/002`](../type/002_the_sentence_the_public_field_contradicts.md) | Why the `pub` field is harmless in practice |

### Sources

| Fact | Where |
|------|-------|
| `Buffer`'s two fields | `ring_store/src/lib.rs:59-63` |
| The methods that recompute rather than read | `ring_store/src/lib.rs:204, 215, 246, 253` |
| Zero `SlotIndex` fields family-wide | Census above |
| Type sizes and exact `Vec` sizing | Standalone `rustc -O` probe, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `mask_equals_modulo_over_four_laps_of_every_capacity` | The topology, asserted as arithmetic because there is no structure to assert it against |
| `a_full_capacity_run_covers_every_slot_once` | That the computed ring is a bijection over one lap |
| *(to create)* | A census assertion that no crate stores a `SlotIndex` in a field |
