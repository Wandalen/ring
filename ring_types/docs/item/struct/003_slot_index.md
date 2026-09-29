# SlotIndex

## Representation

A position within a ring's storage, always in `0..capacity` — a public `usize`
newtype, structurally identical to [`Seq`](002_seq.md) but semantically its
opposite: this one wraps, and wrapping is correct. Two sequences a full lap apart
produce the identical `SlotIndex`, which is the entire reason the two are
separate types rather than one.

**The `0..capacity` claim in its doc is not enforced by this type.** `SlotIndex`
has a public field and derives `Default`, so `SlotIndex( usize::MAX )` is
constructible and legal. The bound comes from the only intended producer —
[`Capacity::mask`](../associated_function/003_capacity_mask.md) applied by
`ring_index` — and it holds because the mask is total, not because this type
checks anything. The doc says "never constructed by counting"; nothing makes that
true.

It is the crate's least-used item. Three sibling crates name it in `src/` and the
same three in `tests/`:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -rln '\bSlotIndex\b' ring_*/src --include='*.rs' | command grep -v ring_types/ | sort
```

Live output:

```
ring_batch/src/lib.rs
ring_store/src/lib.rs
ring_index/src/lib.rs
```

returns `ring_batch`, `ring_store`, `ring_index` and nothing else — against
eighteen for `Seq`.

## Kind

Struct (§ Item Kind Taxonomy : Stable Item Kinds #6)

## Definition

`ring_types/src/id.rs:99`

```rust
#[ derive( Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default ) ]
pub struct SlotIndex( pub usize );
```

The derive list is byte-identical to `Seq`'s. The types differ only in their
payload (`usize` vs `u64`) and in what the family is entitled to assume about
them — none of which is expressible in a derive.

## File Usage

| File | Line(s) | Context |
|------|---------|---------|
| `ring_types/src/id.rs` | 90-92, 95-96, 99, 101, 106-107 | Type doc (90-92); doc example (95-96); **definition (99)**; inherent `impl` header (101); `get`'s doc example (106-107) |
| `ring_types/src/lib.rs` | 18, 43 | Module responsibility table (18, doc); **re-export (43)** |

Test-only references: `ring_types` (7 in `tests/types_test.rs`), plus the same
three consumer crates listed below — the only item in this crate whose `src/` and
`tests/` consumer sets are identical.

## Crate Usage

| Crate | Via File | Purpose |
|-------|----------|---------|
| `ring_types` | `src/id.rs`, `src/lib.rs` | Defining crate |
| `ring_batch` | `src/lib.rs` | Walks the slots of a claimed run |
| `ring_store` | `src/lib.rs` | Addresses the backing storage |
| `ring_index` | `src/lib.rs` | **Produces it** — the sole intended constructor, `of( seq, capacity )` at `src/lib.rs:42` |

**Three consumers, and one of them is the producer.** `ring_index` exists to make
this type from a `Seq`; `ring_store` exists to dereference it; `ring_batch`
walks a run of them. Nothing else in the family touches a slot, because nothing
else needs to — the other fifteen `Seq` consumers work in sequence space and stop
before the fold.

The exception is worth naming: `ring_mpsc` folds a `Seq` into a raw `usize` at
`src/lib.rs:506` without going through this type or through `ring_index`. It is
correct — the `Capacity` invariant travels with the value — but it means the
family has one slot computation this type does not cover
(→ [`../../pattern/002`](../../pattern/002_a_newtype_that_makes_a_check_unnecessary.md)).
