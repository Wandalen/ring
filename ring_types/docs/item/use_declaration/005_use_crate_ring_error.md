# use crate::RingError

## Representation

**The crate's only cross-module code reference.** `capacity.rs` needs
[`RingError`](../enum/002_ring_error.md) because
[`Capacity::new`](../associated_function/001_capacity_new.md) is the only
fallible operation `ring_types` declares — it returns
`Result< Self, RingError >` and constructs two of the enum's nine variants.

Filtering every type reference in `src/` down to code leaves exactly one line
that is neither a definition, an `impl` header, nor a `lib.rs` re-export:

```sh
cd "$(git rev-parse --show-toplevel)"
cd ring_types
for id in Capacity Seq SlotIndex WaitKind OverflowPolicy RingError; do
  command grep -n "\b$id\b" src/*.rs | command grep -v ':[0-9]*: *//' | command sed -E 's/^([^:]+):[0-9]+:/\1:/'
done
```

Live output:

```
src/capacity.rs:pub struct Capacity( usize );
src/capacity.rs:impl Capacity
src/lib.rs:pub use capacity::Capacity;
src/id.rs:pub struct Seq( pub u64 );
src/id.rs:impl Seq
src/lib.rs:pub use id::{ Seq, SlotIndex };
src/id.rs:pub struct SlotIndex( pub usize );
src/id.rs:impl SlotIndex
src/lib.rs:pub use id::{ Seq, SlotIndex };
src/lib.rs:pub use policy::{ OverflowPolicy, WaitKind };
src/policy.rs:pub enum WaitKind
src/policy.rs:impl WaitKind
src/lib.rs:pub use policy::{ OverflowPolicy, WaitKind };
src/policy.rs:pub enum OverflowPolicy
src/policy.rs:impl OverflowPolicy
src/capacity.rs:use crate::RingError;
src/capacity.rs:  pub const fn new( slots : usize ) -> Result< Self, RingError >
src/capacity.rs:      return Err( RingError::CapacityZero );
src/capacity.rs:      return Err( RingError::CapacityNotPowerOfTwo( slots ) );
src/error.rs:pub enum RingError
src/error.rs:impl RingError
src/error.rs:impl fmt::Display for RingError
src/error.rs:impl core::error::Error for RingError {}
src/lib.rs:pub use error::RingError;
```

This one. **So the crate's internal dependency graph is a single edge**, and the
other three modules are mutually invisible
(→ [`../readme.md`](../readme.md) § The internal dependency graph is one edge).

It resolves through the crate root rather than through `error::` — `crate::RingError`
names the binding that
[`pub use error::RingError;`](002_pub_use_ring_error.md) creates at `lib.rs:42`,
not the enum's own path. Writing `use crate::error::RingError;` would work
identically and would not have needed the re-export; the form chosen couples the
two modules through the public surface instead of through the private one.

## Kind

Use Declaration (§ Item Kind Taxonomy : Stable Item Kinds #3)

## Definition

`ring_types/src/capacity.rs:8`

```rust
use crate::RingError;
```

Plain `use`, not `pub use` — importing the name into `capacity`'s scope without
re-exporting it. A `pub use` here would create a second public path
(`ring_types::capacity::RingError`) only if the module itself were public, which
it is not, so the distinction is inert in this crate and would stop being inert
the moment `mod capacity` became `pub mod capacity`.

## File Usage

| File | Line(s) | Context |
|------|---------|---------|
| `ring_types/src/capacity.rs` | 8, 31-32, 35-37, 40, 44, 48 | **The declaration (8)**; `# Errors` doc naming both variants (31-32, doc); doc example (35-37); **`new`'s return type (40)**; **`RingError::CapacityZero` (44)**; **`RingError::CapacityNotPowerOfTwo( slots )` (48)** |

Three of the eight lines are executable: the return type and the two variant
constructions. Lines 44 and 48 are **the only variant constructions anywhere in
`ring_types`** — the other seven variants are built by sibling crates or, in two
cases, by nothing.

Test-only references: none. `tests/types_test.rs` imports `ring_types::RingError`
from the crate root, so it exercises the same type without ever resolving through
this line.

## Crate Usage

| Crate | Via File | Purpose |
|-------|----------|---------|
| `ring_types` | `src/capacity.rs` | Local alias so `new` can declare `Result< Self, RingError >` and construct its two error variants without fully-qualifying `crate::RingError` at each of the three use sites |

**One row, and it can never grow.** A plain `use` inside a private module is
invisible outside the crate, which makes this the narrowest item in the catalog —
and, since it is also the crate's whole internal dependency graph, arguably the
one whose deletion would say the most: without it, `ring_types` would be four
files that never mention each other.
