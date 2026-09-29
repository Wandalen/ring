# Enum Items

### Scope

- **Purpose**: Catalog the three enums `ring_types` defines — two closed policy sets and one open error set.
- **Responsibility**: Give each one's variants, derives, exhaustiveness, and measured usage.
- **In Scope**: `WaitKind`, `RingError`, `OverflowPolicy`.
- **Out of Scope**: The handlers that dispatch on the two policy enums, owned by `ring_wait` and `ring_overflow` (→ [`../../pattern/001`](../../pattern/001_discriminants_here_handlers_elsewhere.md)); `RingError`'s validation rules, which are [`../../type/002`](../../type/002_ring_error.md)'s.

### Overview Table

| ID | Name | Kind | Defined In | Status |
|----|------|------|-----------|--------|
| 001 | [WaitKind](001_wait_kind.md) | Enum (#7) | `src/policy.rs:22` | 🔄 |
| 002 | [RingError](002_ring_error.md) | Enum (#7) | `src/error.rs:44` | 🔄 |
| 003 | [OverflowPolicy](003_overflow_policy.md) | Enum (#7) | `src/policy.rs:105` | 🔄 |

### One of the three is open, and it is the one with nine variants

| | Variants | `#[ non_exhaustive ]` | `Default` | `ALL` array |
|--|----------|----------------------|-----------|-------------|
| `WaitKind` | 4 | no | `Spin` | yes, `[Self; 4]` |
| `OverflowPolicy` | 3 | no | `DropNewest` | yes, `[Self; 3]` |
| `RingError` | 9 | **yes** | no | **no** |

**The split is deliberate and it is the reason the three are documented
differently everywhere else in this tree.** The two policy enums are closed sets
a consumer is entitled to match exhaustively — adding a variant is a breaking
change, which is the point, since a new wait strategy nobody handles is worse
than a compile error. `RingError` is open because a new failure mode is not a
design change and a consumer that matches on it should be written to tolerate
one.

The consequence for testing is asymmetric. A closed enum's completeness can be
asserted from outside the crate with a wildcard-free `match`; an open one's
cannot, because a foreign `match` on a `#[ non_exhaustive ]` enum requires a
wildcard arm that will silently absorb the new variant. So `RingError`'s
closure is watched by gate G1's line-coverage threshold instead — and that
mechanism has already caught a real omission
(→ [`../../non_functional_requirement/002`](../../non_functional_requirement/002_the_enum_sets_are_closed_and_asserted.md)).

### The `ALL` arrays enforce nothing on their own

Both policy enums carry a `pub const ALL` array, and the array by itself
enforces nothing: a five-variant enum with `ALL : [ Self; 4 ]` compiles cleanly,
because the array is written by hand and the compiler has no reason to relate
its length to the variant count. Both doc comments now say this directly rather
than overclaiming what the length assertion catches (`policy.rs:41-52`, `:118-125`).

What actually closes the set is the wildcard-free `match` in
`tests/types_test.rs`, and both `WaitKind` and `OverflowPolicy` now carry the
full three-mechanism pair — a length assert, a per-variant `contains` loop, and
the exhaustive `match`. `overflow_policy_has_no_overwrite_variant` gained the
`contains` loop that used to be missing, so a *duplicated* entry no longer
passes every test in the workspace
(→ [`../../non_functional_requirement/002`](../../non_functional_requirement/002_the_enum_sets_are_closed_and_asserted.md) T6, closed).

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'instances:        '; ls ring_types/docs/item/enum/[0-9][0-9][0-9]_*.md | wc -l
printf 'rows in table:    '; command grep -c '^| [0-9][0-9][0-9] |' ring_types/docs/item/enum/readme.md
printf 'pub enum in src:  '; command grep -h '^pub enum ' ring_types/src/*.rs | wc -l
```

Live output:

```
instances:        3
rows in table:    3
pub enum in src:  3
```

All three counts agree: three `.md` instances in this directory, three rows in
the Overview Table above, three `pub enum` declarations in the crate's `src/`.
A fourth enum landing in source without a matching instance and table row would
move only the third count, which is exactly the drift this recipe exists to
catch.
