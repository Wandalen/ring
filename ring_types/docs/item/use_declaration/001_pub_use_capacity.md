# pub use capacity::Capacity

## Representation

Re-exports [`Capacity`](../struct/001_capacity.md) from the private
[`capacity`](../module/001_capacity.md) module to the crate root. This line is
the only path by which the type is reachable from outside `ring_types` — the
module is `mod`, not `pub mod`, so `ring_types::capacity::Capacity` does not
resolve and `ring_types::Capacity` is the sole spelling.

Fifteen sibling crates' `src/` and nineteen `tests/` reach the type through this
line. It is load-bearing in a way its neighbour at line 36 is not: renaming the
module costs nothing outside the crate, renaming the re-exported type breaks
fifteen compilations.

## Kind

Use Declaration (§ Item Kind Taxonomy : Stable Item Kinds #3)

## Definition

`ring_types/src/lib.rs:41`

```rust
pub use capacity::Capacity;
```

Single-name form rather than a brace list, because `capacity` exports one type.
Its three siblings at lines 42-44 split the same way: one name for `error`, two
each for `id` and `policy`, matching what each module actually declares. No glob
(`pub use capacity::*;`) anywhere in the crate — every exported name is written
out, which is what makes the six-line export surface auditable by reading rather
than by expanding.

## File Usage

| File | Line(s) | Context |
|------|---------|---------|
| `ring_types/src/lib.rs` | 41 | **The declaration.** Its only occurrence — a `pub use` is referenced by resolution, not by name |

Test-only references: `ring_types` — `tests/types_test.rs` imports
`ring_types::Capacity`, which resolves through this line, but never names the
declaration itself. The same is true of all nineteen consumer test suites.

## Crate Usage

| Crate | Via File | Purpose |
|-------|----------|---------|
| `ring_types` | `src/lib.rs` | Declares the re-export |
| *(fifteen consumers)* | `src/lib.rs` each | Resolve `ring_types::Capacity` through it — the full list is on [`../struct/001_capacity.md`](../struct/001_capacity.md) |

**The Crate Usage table for a re-export is a resolution table, not a reference
table**, and that is a distinction worth keeping: no consumer's source contains a
string that grep could match to this line. Its usage is exactly the type's usage,
which is why this row delegates rather than duplicating fifteen rows that would
drift independently.
