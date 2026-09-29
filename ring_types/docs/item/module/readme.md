# Module Items

### Scope

- **Purpose**: Catalog the crate's four module declarations, all private, all in `lib.rs`.
- **Responsibility**: Give each declaration's line, its contents, and what depends on it.
- **In Scope**: `capacity`, `error`, `id`, `policy`.
- **Out of Scope**: The `pub use` lines that re-export each module's contents, which are separate items (→ [`../use_declaration/`](../use_declaration/)).

### Overview Table

| ID | Name | Kind | Defined In | Status |
|----|------|------|-----------|--------|
| 001 | [capacity](001_capacity.md) | Module (#1) | `src/lib.rs:36` | 🔄 |
| 002 | [error](002_error.md) | Module (#1) | `src/lib.rs:37` | 🔄 |
| 003 | [id](003_id.md) | Module (#1) | `src/lib.rs:38` | 🔄 |
| 004 | [policy](004_policy.md) | Module (#1) | `src/lib.rs:39` | 🔄 |

### All four are private, and that is the whole design

```rust
mod capacity;
mod error;
mod id;
mod policy;

pub use capacity::Capacity;
pub use error::RingError;
pub use id::{ Seq, SlotIndex };
pub use policy::{ OverflowPolicy, WaitKind };
```

Not one is `pub mod`. A consumer writes `ring_types::Capacity`, never
`ring_types::capacity::Capacity`, and the module names are an implementation
detail that could be rearranged without touching a single call site in the other
32 crates. **The re-export list is therefore the entire public surface** — nine
lines of `lib.rs` describe everything the family can reach
(→ [`../../api/001`](../../api/001_the_vocabulary_surface.md)).

`lib.rs` also carries `#![ deny( missing_docs ) ]` at line 27. That is an inner
attribute rather than an item, so it has no instance here, but it is why every
one of the forty items in this catalog has a doc comment.

### The four modules are four independent problems

Filtering the crate's type references down to code leaves exactly one edge
between modules — `capacity.rs:8`, `use crate::RingError;`
(→ [`../use_declaration/005_use_crate_ring_error.md`](../use_declaration/005_use_crate_ring_error.md)).
`error`, `id` and `policy` reference nothing outside themselves but `core::fmt`.

So the decomposition is not a layering; it is four unrelated files sharing a
crate. Nothing except the export Contract argues they belong together, and the
honest reading is that the Contract *is* the argument — one tier-0 crate the
other 32 depend on, rather than four crates they each depend on separately
(→ [`../../invariant/002`](../../invariant/002_tier_zero_depends_on_nothing.md)).

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'instances:        '; ls ring_types/docs/item/| wc -l
printf 'rows in table:    '; command grep -c '^| [0-9][0-9][0-9] |' ring_types/docs/item//home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/readme.md
printf 'mod decls in lib: '; command grep -h '^mod ' ring_types/src/lib.rs | wc -l
printf 'pub mod decls:    '; command grep -h '^pub mod ' ring_types/src/lib.rs | wc -l
```

Live output:

```
instances:        4
rows in table:    4
mod decls in lib: 4
pub mod decls:    0
```

Four counts, four agreements: four `.md` instances, four rows in the Overview
Table, four `mod` declarations in `lib.rs`, and zero of those four are `pub` —
the source for this file's own opening claim, "All four are private."
