# WaitKind::ALL

## Representation

Every `WaitKind` variant, in discriminant order, as a `[ Self; 4 ]`.
**Zero references in any sibling crate's source — not one line, not even a doc
comment.** Eight references in two test suites, and that is the whole external
footprint.

**Its doc comment is explicit about what the array's own assertion does *not*
catch:** "a fifth variant compiles clean in this file, leaves `ALL` at four, and
passes the assertion below — measured, not assumed" (`policy.rs:41-43`). What
actually refuses a fifth variant is a wildcard-free `match` — `ring_wait`'s
`escalation_hint` and `pause` in production source, plus
`wait_kind_has_exactly_four_variants` in `tests/types_test.rs` (`policy.rs:44-46`).

**It is now equally defended with the crate's other roster.**
`wait_kind_has_exactly_four_variants` and `overflow_policy_has_no_overwrite_variant`
carry the identical three mechanisms — a length assert, a per-variant `contains`
loop, and a wildcard-free exhaustive `match`
(→ [`003_overflow_policy_all.md`](003_overflow_policy_all.md),
[`../../non_functional_requirement/002`](../../non_functional_requirement/002_the_enum_sets_are_closed_and_asserted.md) T6, closed).

## Kind

Associated Constant (§ Item Kind Taxonomy : Associated Item Kinds #2)

## Definition

`ring_types/src/policy.rs:58`

```rust
pub const ALL : [ Self; 4 ] = [ Self::Spin, Self::Yield, Self::Park, Self::None ];
```

**An array, not a slice and not an iterator**, so `ALL.len()` is a compile-time
constant and the type itself carries the count. Changing the array's contents
without changing `4` is a compile error; changing both together is not, which is
exactly the hole the `contains` assertions fill.

The first item in `impl WaitKind`, ahead of
[`is_non_blocking`](../associated_function/011_wait_kind_is_non_blocking.md)
(→ [`../implementation/007_impl_wait_kind.md`](../implementation/007_impl_wait_kind.md)).

**Nothing in the language keeps this in sync with the enum.** `WaitKind` is not
`#[ non_exhaustive ]`, so adding a variant breaks downstream exhaustive matches —
but this array is not a match, and an added variant leaves it compiling, three of
four correct, and one short.

## File Usage

| File | Line(s) | Context |
|------|---------|---------|
| `ring_types/src/policy.rs` | 39, 41-52, 54-58, 65 | Doc summary, "Every variant, in discriminant order" (39); the stated rationale — what the length assertion does and does not catch (41-52); doc example asserting `ALL.len() == 4` (54-57); **the definition (58)**; used by `is_non_blocking`'s own doc example, `ALL.iter().filter( … ).count() == 1` (65) |

**Line 65 is the only in-crate reference and it is in a doc comment**, which
makes this the one item whose sole internal use is documentation of a different
item.

Test-only references: `ring_types` — `tests/types_test.rs:139`,
`wait_kind_has_exactly_four_variants`, and `:160`,
`exactly_one_wait_kind_is_non_blocking`. Plus 8 across two consumer suites.

## Crate Usage

| Crate | Via File | Purpose | Refs |
|-------|----------|---------|-----:|
| `ring_types` | `src/policy.rs` | Defining crate | — |
| `ring_wait` | `tests/wait_test.rs` | Sweeps every variant through the handler — the other half of decision 121 § 5's split | 7 |
| `ring_config` | `tests/config_test.rs` | Sweeps every variant through `is_tick_safe` | 1 |

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -rn 'WaitKind::ALL' ring_*/src | command grep -v '^ring_types/' || true
```

Live output:

```
```

**is empty — exit 1.** No production code in the family enumerates wait kinds.
That is the correct outcome for a discriminant roster: production code *matches*
on a `WaitKind` it was handed, and only a test needs to visit all of them.

**`ring_wait`'s seven references are the interesting ones**, and one of them is
the strongest assertion made about this constant anywhere — including in the
crate that declares it:

```rust
// ring_wait/tests/wait_test.rs:60-64
assert_eq!(
  WaitKind::ALL,
  [ WaitKind::Spin, WaitKind::Yield, WaitKind::Park, WaitKind::None ],
  "in discriminant order — a reordering changes what a serialized config means"
);
```

**Array equality pins contents *and* order.** `ring_types`' own
`wait_kind_has_exactly_four_variants` uses a length check plus a `contains` loop,
which accepts any permutation; `ring_wait`'s single `assert_eq!` does not. The
message says why order is not cosmetic — a discriminant's numeric value is what a
serialized config carries, so swapping `Yield` and `Park` silently reinterprets
every stored configuration.

**The handler crate defends the discriminant roster better than the discriminant
crate does**, which is the split working in the direction nobody designed for it:
`ring_wait` cares about order because it is the crate that would suffer, and that
motivation does not exist here.

**A sweep-based suite still cannot validate the array it sweeps.** Five of
`ring_wait`'s seven references iterate `ALL` (`:67`, `:83`, `:97`, `:339`,
`:348`); if the array lost `Park`, all five would pass over three variants and
report success. Only the two that *name* the variants catch it —
`wait_test.rs:60-64` and `types_test.rs:144`. **`OverflowPolicy::ALL` now has
one of the two:** its own `overflow_policy_has_no_overwrite_variant` gained the
identical `contains` loop (`types_test.rs:203`), but no consumer crate carries
the order-pinning `assert_eq!` `ring_wait` writes for `WaitKind`
(→ [`003_overflow_policy_all.md`](003_overflow_policy_all.md),
[`../../non_functional_requirement/002`](../../non_functional_requirement/002_the_enum_sets_are_closed_and_asserted.md) T6, closed).

**`ring_wait` also checks the non-blocking property against behaviour rather than
against the predicate:** `exactly_one_discriminant_is_non_blocking` filters on
`!pause( *k, 0 )` — what the handler actually does — not on
[`is_non_blocking`](../associated_function/011_wait_kind_is_non_blocking.md). It
is the same technique `ring_overflow` uses for the policy predicates
(→ [`../associated_function/012_overflow_policy_reports_failure.md`](../associated_function/012_overflow_policy_reports_failure.md)),
and it is what makes the two sides of the split able to disagree.
