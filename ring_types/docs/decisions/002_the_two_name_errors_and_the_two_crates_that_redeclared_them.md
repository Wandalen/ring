# Decision: The Two Name Errors, and the Two Crates That Redeclared Them

### Scope

- **Purpose**: Record what happened to `RingError::NameTaken` and `NameUnknown` in the workspace that actually exists — not that nothing constructs them, which [`pitfall/002`](../pitfall/002_two_name_errors_nothing_constructs.md) already states, but that two other crates declared the same failure under their own names instead.
- **Responsibility**: State the three declarations, the manifest edge that survived the decision, and what each disposal option costs the crates that would have to change.
- **In Scope**: The two variants, `ring_factory::BuildError::NameTaken`, `ring_registry::RegistryError::NameTaken`, and `ring_registry`'s `[dependencies]` entry.
- **Out of Scope**: Why the registry declined the shared error — see [`integration/002`](../integration/002_the_registry_that_declined_the_shared_error.md); the "one error type" claim this is downstream of — see [`001`](001_one_error_type_is_a_rule_the_family_does_not_keep.md).

### Three Declarations of One Failure

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r 'NameTaken' ring_types/src/error.rs ring_factory/src/lib.rs ring_registry/src/lib.rs \
  | grep -vE ': *(//|///|//!)' | sed 's/:  */:/'
```

Live output:

```
ring_types/src/error.rs:NameTaken,
ring_types/src/error.rs:| Self::NameTaken
ring_types/src/error.rs:| Self::NameTaken
ring_types/src/error.rs:Self::NameTaken => write!( f, "a ring is already registered under this name" ),
ring_factory/src/lib.rs:Err( ( RegistryError::NameTaken { .. }, _refused ) ) => Err( BuildError::NameTaken ),
ring_factory/src/lib.rs:NameTaken,
ring_factory/src/lib.rs:Self::NameTaken => write!( f, "a ring is already registered under this name" ),
ring_registry/src/lib.rs:NameTaken
ring_registry/src/lib.rs:Self::NameTaken { name } => write!( f, "a ring is already registered as {name:?}" ),
ring_registry/src/lib.rs:Err( ( RegistryError::NameTaken { name }, ring ) )
```

**The family spells "a ring is already registered under this name" three times,
in three enums, and constructs it in two of them.** The one it does not construct
is the shared one declared at tier 0 for exactly this purpose.

### What Each Crate Did and Why

| Crate | Variant | Constructed | Reason it exists |
|-------|---------|-------------|------------------|
| `ring_types` | `RingError::NameTaken` | **never** | Declared at tier 0 so the registry would not need its own error |
| `ring_types` | `RingError::NameUnknown` | **never** | Same, for the lookup half |
| `ring_registry` | `RegistryError::NameTaken` | yes | `RingError` is `Copy`, so it cannot carry the colliding name (→ [`non_functional_requirement/001`](../non_functional_requirement/001_errors_and_positions_do_not_allocate.md)) |
| `ring_factory` | `BuildError::NameTaken` | yes | `ring_factory` does not depend on `ring_registry`'s error and needed the case in its own return type |

`ring_factory`'s is the interesting one. It is not the registry working around
`Copy`; it is a third crate meeting the same failure and reaching for neither
existing declaration — one because it is unconstructed and undocumented, the
other because naming it would put `ring_registry` in a Contract-bound signature.

### The Edge That Outlived the Decision

```sh
cd "$(git rev-parse --show-toplevel)"
grep -c 'ring_types' ring_registry/Cargo.toml
grep -cE '\b(Capacity|RingError|Seq|SlotIndex|OverflowPolicy|WaitKind|ring_types)\b' ring_registry/src/lib.rs
# The command above can legitimately exit nonzero -- a count of zero, a diff
# that differs -- and the sweep that keeps this output current skips any block
# that exits nonzero, so without this the block freezes instead of refreshing.
:
```

Live output:

```
3
0
```

`ring_registry` declares the dependency and names nothing from it. The edge is
in the manifest graph, in `cargo tree`, and in every build plan that touches the
registry; it is in no line of code.

### The Options

| Option | Edit | Cost |
|--------|------|------|
| Delete both variants | 2 lines here | A semver event on a `#[ non_exhaustive ]` enum — which is exactly the case `#[ non_exhaustive ]` makes cheap, so the cost is close to nothing (→ [`001`](001_one_error_type_is_a_rule_the_family_does_not_keep.md), TY5) |
| Keep and document | ~4 lines of doc comment | The two variants stay in the classifier, the `Display` match, and every reader's mental model of the error surface |
| Make them reachable | `pub use` in `ring_factory` (P4), then fold the registry in | Requires dropping `Copy` or dropping the name from the message — the trade `RegistryError` was created to avoid |
| Delete the manifest edge | 1 line in `ring_registry/Cargo.toml` | None found — but it is another crate's manifest, so it is a proposal, not a fix |

**`#[ non_exhaustive ]` changes the answer P3 recorded.** P3 weighed deletion
against "a semver event on a `#[ non_exhaustive ]` enum" as though the attribute
were a cost. It is the opposite: the attribute exists so that consumers cannot
match exhaustively, which is what makes removing an unconstructed variant a
non-breaking change for every crate that already compiles. And nothing in the
workspace matches on a `RingError` value at all, exhaustively or otherwise
(→ [`001`](001_one_error_type_is_a_rule_the_family_does_not_keep.md), TY5), so
removing a variant nothing constructs cannot break a match that does not exist.

### TY6 — The Shared Variant Was Redeclared Twice Rather Than Used Once

`RingError::NameTaken` is constructed nowhere in the workspace, while
`ring_factory::BuildError::NameTaken` and `ring_registry::RegistryError::NameTaken`
are each constructed and each returned from public API. Tier 0 declared the
variant so the two crates above it would not need their own; both wrote their
own anyway, and the reasons differ per crate.

### TY7 — `NameUnknown` Exists Only in Its Own `Display` Arm

Across the whole workspace, the only references to `RingError::NameUnknown`
outside this crate's own test suite are its declaration, its arm in
`is_configuration`'s inverse, and the `Display` arm that renders it. No lookup
path in `ring_registry` — the crate whose lookup it names — mentions it.

### TY8 — `ring_registry` Carries a Dependency It Names Nowhere

`ring_registry/Cargo.toml` lists `ring_types`; `ring_registry/src/lib.rs` names
no item from it. It is the only one of the 31 declared dependents of which that
is true, and the reason is this decision: the registry declined the shared error
and kept the edge that existed to carry it.

### Status

**Open**, and narrower than P3 recorded it. The `#[ non_exhaustive ]` argument
against deletion does not hold, and the choice is between deleting two variants
and documenting why they are reserved. The manifest edge is a separate,
independently actionable line in a crate this one does not own.
