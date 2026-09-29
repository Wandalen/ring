# capacity

## Representation

Private module declaration bringing `src/capacity.rs` into the crate root. Holds
three items: the [`Capacity`](../struct/001_capacity.md) struct, its inherent
[`impl` block](../implementation/001_impl_capacity.md), and the three associated
functions on it — [`new`](../associated_function/001_capacity_new.md),
[`get`](../associated_function/002_capacity_get.md),
[`mask`](../associated_function/003_capacity_mask.md).

79 lines, the shortest of the four modules. **It is the only one that
imports from a sibling**: `use crate::RingError;` at `capacity.rs:8`, because
`new` is the crate's only fallible operation.

`mod`, not `pub mod` — the contents reach consumers through
[`pub use capacity::Capacity;`](../use_declaration/001_pub_use_capacity.md) at
`lib.rs:41` and by no other path.

## Kind

Module (§ Item Kind Taxonomy : Stable Item Kinds #1)

## Definition

`ring_types/src/lib.rs:36`

```rust
mod capacity;
```

## File Usage

| File | Line(s) | Context |
|------|---------|---------|
| `ring_types/src/lib.rs` | 19, 36, 41 | Module responsibility table (19, doc); **declaration (36)**; re-export path `pub use capacity::Capacity;` (41) |

The module's own file never names it — a module declared with `mod capacity;`
refers to itself as `crate` or by relative path, so `capacity.rs` contains zero
occurrences of the identifier outside the doc comment's prose.

Test-only references: none. `tests/types_test.rs` imports `ring_types::Capacity`
from the crate root, exercising this module transitively through the re-export
rather than path-qualifying `capacity::` — which it could not do in any case,
since the module is private.

## Crate Usage

| Crate | Via File | Purpose |
|-------|----------|---------|
| `ring_types` | `src/lib.rs` | Declares the module so `Capacity` and its three associated functions exist and can be re-exported |

**One row, and it is the defining crate.** A private module is unreachable from
outside the crate by construction, so this table can never grow — which is the
difference between a module instance and a struct instance in this catalog, and
the reason the module files here are short.
