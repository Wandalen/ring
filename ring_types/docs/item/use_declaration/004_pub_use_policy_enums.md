# pub use policy::{ OverflowPolicy, WaitKind }

## Representation

Re-exports both configuration enums from the private
[`policy`](../module/004_policy.md) module. This is the line that makes the
discriminant/handler split operable: the discriminants become
`ring_types::WaitKind` and `ring_types::OverflowPolicy`, and `ring_wait` and
`ring_overflow` import them from here to write the handlers that act on them
(→ [`../../pattern/001`](../../pattern/001_discriminants_here_handlers_elsewhere.md)).

**Adding a variant to either enum is a breaking change that this line
propagates.** Neither is `#[ non_exhaustive ]`, so every consumer's exhaustive
`match` fails to compile — deliberately. Seven crates resolve `WaitKind` through
this line and ten resolve `OverflowPolicy`, which puts a lower bound on the blast
radius of a new wait strategy or overflow policy.

## Kind

Use Declaration (§ Item Kind Taxonomy : Stable Item Kinds #3)

## Definition

`ring_types/src/lib.rs:44`

```rust
pub use policy::{ OverflowPolicy, WaitKind };
```

Alphabetical, which reverses the declaration order in `policy.rs` — `WaitKind` is
at line 22 and `OverflowPolicy` at 105. The sibling at line 43 orders by
declaration instead. The inconsistency is harmless and worth noting only because
someone will eventually normalize one of them and should know the other exists.

## File Usage

| File | Line(s) | Context |
|------|---------|---------|
| `ring_types/src/lib.rs` | 44 | **The declaration.** Its only occurrence |

Test-only references: 6 consumer test suites for `WaitKind` and 12 for
`OverflowPolicy`, plus `ring_types`' own, all resolving through this line without
naming it.

## Crate Usage

| Crate | Via File | Purpose |
|-------|----------|---------|
| `ring_types` | `src/lib.rs` | Declares the re-export |
| *(seven consumers)* | `src/lib.rs` each | Resolve `ring_types::WaitKind` — full list on [`../enum/001_wait_kind.md`](../enum/001_wait_kind.md) |
| *(ten consumers)* | `src/lib.rs` each | Resolve `ring_types::OverflowPolicy` — full list on [`../enum/003_overflow_policy.md`](../enum/003_overflow_policy.md) |

**The two name sets overlap in exactly three crates** — `ring_config`,
`ring_factory` and `ring_shutdown` — so this line serves fourteen distinct
consumers, not seventeen:

```sh
cd "$(git rev-parse --show-toplevel)"
cd ring
comm -12 \
  <(command grep -rln '\bWaitKind\b'       ring_*/src --include='*.rs' | sed 's|/src.*||' | sort -u) \
  <(command grep -rln '\bOverflowPolicy\b' ring_*/src --include='*.rs' | sed 's|/src.*||' | sort -u) \
  | command grep -v '^ring_types$'
```

Live output:

```
ring_config
ring_factory
ring_shutdown
```

The `grep -v` matters: without it the intersection is four, because the defining
crate names both in `lib.rs`'s own module table and re-export.

**The three that name both are the three that carry configuration rather than act
on it** — a builder, a config struct, and the shutdown path that has to honour
whatever both were set to. Neither handler crate appears: `ring_wait` names
`WaitKind` and never `OverflowPolicy`, `ring_overflow` the reverse. That is the
discriminant/handler split showing up as a set intersection, and it is a
stronger statement of the rule than the module doc's prose, because it is
falsifiable by one command.
