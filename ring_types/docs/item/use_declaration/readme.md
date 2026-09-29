# Use Declaration Items

### Scope

- **Purpose**: Catalog the crate's six `use` declarations — four re-exports that are the public surface, and two imports that are the entire dependency graph.
- **Responsibility**: Give each declaration's line, what it names, and what it costs.
- **In Scope**: The four `pub use` lines in `lib.rs`; `use crate::RingError;` in `capacity.rs`; `use core::fmt;` in `error.rs`.
- **Out of Scope**: The `mod` declarations these re-export from (→ [`../module/`](../module/)).

### Overview Table

| ID | Name | Kind | Defined In | Status |
|----|------|------|-----------|--------|
| 001 | [pub use capacity::Capacity](001_pub_use_capacity.md) | Use Declaration (#3) | `src/lib.rs:41` | 🔄 |
| 002 | [pub use error::RingError](002_pub_use_ring_error.md) | Use Declaration (#3) | `src/lib.rs:42` | 🔄 |
| 003 | [pub use id::{ Seq, SlotIndex }](003_pub_use_seq_slot_index.md) | Use Declaration (#3) | `src/lib.rs:43` | 🔄 |
| 004 | [pub use policy::{ OverflowPolicy, WaitKind }](004_pub_use_policy_enums.md) | Use Declaration (#3) | `src/lib.rs:44` | 🔄 |
| 005 | [use crate::RingError](005_use_crate_ring_error.md) | Use Declaration (#3) | `src/capacity.rs:8` | 🔄 |
| 006 | [use core::fmt](006_use_core_fmt.md) | Use Declaration (#3) | `src/error.rs:13` | 🔄 |

### Six lines, and they are the whole architecture

```sh
cd "$(git rev-parse --show-toplevel)"
command grep '^use \|^pub use ' ring_types/src/*.rs
```

Live output:

```
ring_types/src/capacity.rs:use crate::RingError;
ring_types/src/error.rs:use core::fmt;
ring_types/src/lib.rs:pub use capacity::Capacity;
ring_types/src/lib.rs:pub use error::RingError;
ring_types/src/lib.rs:pub use id::{ Seq, SlotIndex };
ring_types/src/lib.rs:pub use policy::{ OverflowPolicy, WaitKind };
```

returns exactly these six. Four are `pub use` in `lib.rs` and constitute the
crate's entire export surface — the five-crate Contract's tier-0 vocabulary,
reachable by the other 32 crates and by nothing else
(→ [`../../api/001`](../../api/001_the_vocabulary_surface.md)).

The remaining two are the crate's complete import graph:

| Line | Imports | Why |
|------|---------|-----|
| `capacity.rs:8` | `crate::RingError` | `Capacity::new` is the crate's only fallible operation |
| `error.rs:13` | `core::fmt` | The hand-written `Display` impl needs `Formatter` and `Result` |

**Neither reaches outside `core`.** That is what
[`../../invariant/002`](../../invariant/002_tier_zero_depends_on_nothing.md)
states and what makes the family's dependency forest acyclic by construction:
tier 0 has nothing above it to cycle through.

### A `pub use` is not free of consequence

The four re-exports flatten four private modules into one namespace, which is why
a consumer writes `ring_types::Capacity` and never learns that `capacity.rs`
exists. The cost is that the four modules now share a name space and cannot
declare colliding public items — `Capacity` and `RingError` could not both be
called `Error`. With six exported names across four modules that has not bitten,
and the constraint is worth stating because it is the reason a fifth module
cannot be added carelessly.

**The re-exports are also where a rename becomes a semver event.** Changing
`mod capacity` to `mod slots` costs nothing outside the crate; changing
`pub use capacity::Capacity` to export a differently-named type breaks nineteen
crates. The two sit on adjacent lines and only one of them is load-bearing.
