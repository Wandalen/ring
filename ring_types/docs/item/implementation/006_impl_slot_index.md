# impl SlotIndex

## Representation

The crate's smallest non-empty implementation block: one associated function,
[`get`](../associated_function/010_slot_index_get.md), returning the wrapped
`usize`.

**And it is redundant.** `SlotIndex`'s field is `pub`, so `idx.0` and `idx.get()`
are the same operation written two ways, and the type offers no reason to prefer
either. The identical situation holds for [`Seq`](../struct/002_seq.md), which
has a public field and no `get` at all — so the two position types disagree about
whether an accessor is worth having, in the same file, twenty lines apart.

Neither choice is wrong. What is worth recording is that they are different:
`Seq`'s callers write `seq.0` (as `ring_index/src/lib.rs:51` and
`ring_mpsc/src/lib.rs:543` both do), `SlotIndex`'s callers may write either, and
nothing in the crate explains the asymmetry.

[`Capacity`](../struct/001_capacity.md)'s `get` is the one that is not redundant
— its field is private, so the accessor is the only way out.

## Kind

Implementation (§ Item Kind Taxonomy : Stable Item Kinds #12)

## Definition

`ring_types/src/id.rs:101`

```rust
impl SlotIndex
{
  pub const fn get( self ) -> usize   // :110
}
```

No associated constant. `SlotIndex` has no `ZERO`, unlike its sibling — slot zero
is not a meaningful starting position the way sequence zero is, since a fresh
ring's first slot is whatever `Seq::ZERO` folds to, which is zero only
incidentally.

## File Usage

| File | Line(s) | Context |
|------|---------|---------|
| `ring_types/src/id.rs` | 101, 110, 114 | **Block header (101)**; `get` (110); closing brace (114) |

Fourteen lines, ten of them documentation and its example.

Test-only references: `ring_types` — one line, `types_test.rs:77`, inside
`slot_index_is_its_own_type`:

```rust
assert_eq!( SlotIndex( 3 ).get(), 3 );
```

The test that names this block is a type-identity test, not a behaviour test, and
that is honest: there is no behaviour here to test beyond the field read.

## Crate Usage

| Crate | Via File | Purpose |
|-------|----------|---------|
| `ring_types` | `src/id.rs` | Declares the block |
| `ring_batch` | `src/lib.rs` | Walks a run of slot indices |
| `ring_store` | `src/lib.rs` | Dereferences a slot |
| `ring_index` | `src/lib.rs` | Produces the values this block reads |

**Three consumers, the narrowest reach of any implementation block in the crate**
— against fifteen for `impl Capacity` and eighteen for `impl Seq`. The whole
family works in sequence space and only three crates ever see a slot
(→ [`../struct/003_slot_index.md`](../struct/003_slot_index.md)).
