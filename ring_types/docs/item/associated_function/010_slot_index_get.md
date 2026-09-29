# SlotIndex::get

## Representation

Returns the underlying offset. **Two production call sites, both in `ring_store`,
both on the same two lines of the same file** — and the narrowest usage of any
item in the crate.

**It is also the only accessor here that is fully redundant.**
`SlotIndex`'s field is `pub`, so `.0` and `.get()` are interchangeable; the type
offers two spellings of the same read and the family uses each in different
places. Contrast [`Capacity::get`](002_capacity_get.md), whose field is private,
which is why its 29 callers had no alternative. **The difference in call counts —
29 against 2 — tracks the difference in field visibility more closely than any
difference in usefulness.**

## Kind

Associated Function/Method (§ Item Kind Taxonomy : Associated Item Kinds #1)

## Definition

`ring_types/src/id.rs:110`

```rust
#[ must_use ]
pub const fn get( self ) -> usize
```

Body is `self.0` (`id.rs:112`).

The whole `impl SlotIndex` block is this one function (`id.rs:101`–`:114`),
making it the smallest of the crate's eight implementations
(→ [`../implementation/006_impl_slot_index.md`](../implementation/006_impl_slot_index.md)).

## File Usage

| File | Line(s) | Context |
|------|---------|---------|
| `ring_types/src/id.rs` | 96, 103, 105-108, 109-110, 112 | Doc example on the struct itself (96); doc summary "The underlying offset." (103); doc example (105-108); `#[ must_use ]` and **the definition (109-110)**; the body (112) |

Test-only references: `ring_types` — `SlotIndex` appears 7 times in
`tests/types_test.rs`, of which one is the accessor: `types_test.rs:77`,
`assert_eq!( SlotIndex( 3 ).get(), 3 );` inside `slot_index_is_its_own_type`.
**A single in-crate assertion, and it is not really about this function** — the
test's subject is that `SlotIndex` and `Seq` do not unify, and `.get()` is
incidental to showing it.

## Crate Usage

| Crate | Via File | Purpose |
|-------|----------|---------|
| `ring_types` | `src/id.rs` | Defining crate |
| `ring_store` | `src/lib.rs` | Slot addressing — `&self.slots[ index.get() ]` (`:206`) and `&mut self.slots[ index.get() ]` (`:217`) |

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -rn 'index\.get()' ring_*/src | command grep -v '^ring_types/' \
  | command grep -v ':[0-9]*: *//' | command sed -E 's/^([^:]+):[0-9]+:/\1:/'
```

Live output:

```
ring_store/src/lib.rs:    &self.slots[ index.get() ]
ring_store/src/lib.rs:    &mut self.slots[ index.get() ]
```

returns exactly two lines, eleven apart, in one file.

**One crate, and it is the right one.** `ring_store` owns the `Vec< S >` that a
`SlotIndex` indexes; every other crate that holds a `SlotIndex` passes it along
rather than dereferencing it. The two sites are the immutable and mutable halves
of one accessor pair — `Buffer::get( index : SlotIndex )` and
`Buffer::get_mut( index : SlotIndex )` — which is why they are eleven lines apart
and identical otherwise.

**`Buffer::get`'s body is `&self.slots[ index.get() ]`**, so the method named
`get` calls the method named `get` on its own argument. The crate also exposes a
second pair, `at( seq : Seq )` and `at_mut( seq : Seq )` (`:207`, `:213`), which
folds through `ring_index` first — so `ring_store` offers both "I already have
the slot" and "I have a sequence" entry points, and only the first one reaches
this item.

**The type is used far more widely than this function is.** `SlotIndex` itself
appears in `ring_batch`, `ring_store` and `ring_index`
(→ [`../struct/003_slot_index.md`](../struct/003_slot_index.md)); only
`ring_store` ever unwraps one, because only `ring_store` has something to index.
That is the fold's design working: `ring_index` produces the value,
`ring_store` consumes it, and nothing in between needs to see the `usize`.

## Caller Tree

- *No caller within `ring_types`*
- *External: `ring_store::Buffer::get`* (`ring_store/src/lib.rs:206`, fn at `:204`)
- *External: `ring_store::Buffer::get_mut`* (`ring_store/src/lib.rs:217`, fn at `:215`)

## Callee Tree

- *(none)* — a field read on a `Copy` newtype

**Two callers and a public field means this item's deletion would be a
five-minute change**, which is not an argument for deleting it: the accessor is
what makes `SlotIndex` substitutable for a future private-field version, and the
type most likely to acquire a private field is this one, since a bounds-checked
`SlotIndex` is the natural next step from a validated `Capacity`
(→ [`../../pattern/002`](../../pattern/002_a_newtype_that_makes_a_check_unnecessary.md)).
Keeping the accessor costs three lines and keeps that option open.
