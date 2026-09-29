# Api: Six Ways to Reach a Slot

### Scope

**Purpose:** Record that six of the twelve functions are three mutability pairs
addressing slots three ways, that the two `IntoIterator` impls cover borrows and
not ownership, and that with `new` the only constructor a slot can enter a buffer
but never leave one.

**Responsibility:** The accessor surface — by index, by sequence, by iteration —
and the one direction it does not offer.

**In Scope:** `ring_store/src/lib.rs:193-278`.

**Out of Scope:** The census and the `const`/`must_use` splits are
[`api/001`](001_twelve_functions_three_const_seven_must_use.md). What the fold
does is [`algorithm/001`](../algorithm/001_one_mask_no_modulo.md).

---

### BF16 — Three Pairs, One Question Each, and Consumers Use One Pair

The six accessors are the same three questions asked twice, once shared and once
exclusive:

| Pair | Addressed by | Shared | Exclusive |
|------|--------------|--------|-----------|
| by index | `SlotIndex` | `get` | `get_mut` |
| by sequence | `Seq` | `at` | `at_mut` |
| over all | — | `iter` | `iter_mut` |

`at` is `get` composed with the fold; `iter` is the slice's own iterator. So the
surface is one primitive — index a slice — with a convenience above it and a
sweep beside it, doubled by mutability.

**Finding.** The symmetry is complete and the usage is not. The two real
consumers call `at` and `at_mut` and never `get`, `get_mut`, `iter` or `iter_mut`
([`integration/001`](../integration/001_four_dependents_two_that_build_on_it.md)
BF3) — a ring holds sequences, so the sequence-addressed pair is the only one it
wants, and the index-addressed pair exists underneath it as the implementation
plus a public surface for anyone holding a `SlotIndex` directly.

That is a defensible shape rather than a redundancy: `get` is the primitive `at`
is built from, and hiding it would leave `SlotIndex` — a public type in
`ring_types` — with no consumer at all in this crate. What the arrangement costs
is that four of the six accessors are exercised only by this crate's own suite,
so the *ergonomics* of the index and iteration pairs have never met a caller. A
signature that reads awkwardly at a real call site has had no opportunity to
prove it.

---

### BF17 — Slots Can Enter a Buffer and Never Leave It

`IntoIterator` is implemented twice, and both times for a reference:

```sh
cd "$(git rev-parse --show-toplevel)"
grep '^impl' ring_store/src/lib.rs
```

Live output:

```
impl< S : Default > Buffer< S >
impl< S : Slot + Default > Buffer< S >
impl< S > Buffer< S >
impl< 'a, S > IntoIterator for &'a Buffer< S >
impl< 'a, S > IntoIterator for &'a mut Buffer< S >
```

`&'a Buffer< S >` yields `&'a S` and `&'a mut Buffer< S >` yields `&'a mut S`.
There is no `impl IntoIterator for Buffer< S >`, so `for slot in buffer` — by
value — does not compile, and there is no other way out either:

```sh
cd "$(git rev-parse --show-toplevel)"
grep 'pub fn .*-> Self\|pub fn .*-> Vec<\|pub fn .*-> Box<\|impl.*From.*for ' ring_store/src/lib.rs
```

Live output:

```
  pub fn new( capacity : Capacity ) -> Self
```

One function produces a `Buffer` and none decomposes one. The fields are private,
`Buffer` derives only `Debug`, and no `into_inner`, `into_boxed_slice` or
`From< Buffer< S > > for Vec< S >` exists.

**Finding.** A slot enters through `S::default` inside `new` and leaves only when
the whole buffer drops. Every path in and out is a borrow.

For a ring that is exactly right, and the omission is load-bearing rather than
incidental: a `Buffer` whose slots could be moved out would be a `Buffer` whose
storage address could change, and both consumers hold raw pointers into it
through an `UnsafeCell`
([`integration/002`](../integration/002_every_unsafe_block_in_the_family.md)). An
`into_slots` returning the `Box< [ S ] >` would hand a caller the allocation the
ring's `unsafe` blocks are still pointing at.

Nothing records that. The absence reads as a feature not yet needed rather than
as a deliberate closure, and the next person wanting to drain a shut-down ring's
payloads out of storage will find adding one four-line function very easy and its
consequences three crates away. A sentence on the type — *slots are reachable
only by borrow; moving them out would invalidate the pointers `ring_mpsc` and
`ring_spsc` hold* — costs nothing and puts the reason where the change would be
made.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`api/001`](001_twelve_functions_three_const_seven_must_use.md) | The census these six are drawn from |
| [`integration/002`](../integration/002_every_unsafe_block_in_the_family.md) | The raw pointers a by-value exit would invalidate |
| [`lifecycle/001`](../lifecycle/001_allocate_once_borrow_forever_drop_plainly.md) | The allocation's life, which this closure keeps whole |
| [`pattern/002`](../pattern/002_every_write_is_a_borrow.md) | The `&`/`&mut` doubling these pairs are built on |
| [`type/002`](../type/002_the_types_that_cross_the_boundary.md) | `SlotIndex` and `Seq`, the two addressing types |

### Sources

| Fact | Where |
|------|-------|
| The five impl blocks | `ring_store/src/lib.rs:65, 96, 144, 281, 292` |
| The six accessors | `ring_store/src/lib.rs:193-278` |
| `new` as the only producer | `ring_store/src/lib.rs:88` |
| Private fields, `Debug`-only derive | `ring_store/src/lib.rs:58-63` |
| Consumers use one pair | Census in `integration/001` |

### Tests

| Test | Covers |
|------|--------|
| `indexed_get_and_set_round_trip` | The index pair |
| `storage_survives_being_addressed_out_of_order` | The sequence pair, scrambled |
| `iteration_visits_slots_in_index_order` | `iter`, and that order is index order |
| `a_borrowed_buffer_iterates_directly` | Both `IntoIterator` impls, by `&mut` then by `&` |
| *(to create)* | A compile-fail pinning that `for slot in buffer` by value does not compile |
