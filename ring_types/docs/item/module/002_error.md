# error

## Representation

Private module declaration bringing `src/error.rs` into the crate root. Holds
five items: the [`RingError`](../enum/002_ring_error.md) enum, its inherent
[`impl`](../implementation/002_impl_ring_error.md), the two
[`Display`](../implementation/003_impl_display_for_ring_error.md) and
[`Error`](../implementation/004_impl_error_for_ring_error.md) trait impls, and
the [`use core::fmt;`](../use_declaration/006_use_core_fmt.md) declaration.

184 lines, the second-longest of the four modules — nine documented variants and a
nine-arm `match` account for most of it.

**It is the crate's only sink and never a source.** `capacity` imports from it;
it imports from no sibling. Its one external dependency is `core::fmt`, and its
module doc explains the absence of the others: no `error_tools`, no `thiserror`,
no `anyhow`, deliberately, so that tier 0 compiles in isolation
(→ [`../../invariant/002`](../../invariant/002_tier_zero_depends_on_nothing.md)).

## Kind

Module (§ Item Kind Taxonomy : Stable Item Kinds #1)

## Definition

`ring_types/src/lib.rs:37`

```rust
mod error;
```

## File Usage

| File | Line(s) | Context |
|------|---------|---------|
| `ring_types/src/lib.rs` | 21, 37, 42 | Module responsibility table (21, doc); **declaration (37)**; re-export path `pub use error::RingError;` (42) |

Test-only references: none directly — `tests/types_test.rs` reaches `RingError`
through the crate-root re-export.

## Crate Usage

| Crate | Via File | Purpose |
|-------|----------|---------|
| `ring_types` | `src/lib.rs` | Declares the module so `RingError` and its four implementations exist and can be re-exported |

**The module that would change if the house convention were applied.** The
workspace's `error_tools` rule says a crate using error handling should use that
crate exclusively. This module hand-writes `Display` instead, to keep
`[dependencies]` empty. A gate asserting the empty table would have to encode an
exemption for exactly this file — which is why no such gate exists yet
(→ [`../../decisions/readme.md`](../../decisions/readme.md), P5).
