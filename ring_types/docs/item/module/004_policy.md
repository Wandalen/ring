# policy

## Representation

Private module declaration bringing `src/policy.rs` into the crate root. Holds
eight items: the [`WaitKind`](../enum/001_wait_kind.md) and
[`OverflowPolicy`](../enum/003_overflow_policy.md) enums, their two inherent
impls ([007](../implementation/007_impl_wait_kind.md),
[008](../implementation/008_impl_overflow_policy.md)), the two
[`ALL`](../associated_constant/002_wait_kind_all.md)
[arrays](../associated_constant/003_overflow_policy_all.md), and the three
classifier functions
[`is_non_blocking`](../associated_function/011_wait_kind_is_non_blocking.md),
[`reports_failure`](../associated_function/012_overflow_policy_reports_failure.md),
[`drops_silently`](../associated_function/013_overflow_policy_drops_silently.md).

186 lines (was 125 before `Fix(overflow_policy_classification_not_exhaustive)`
widened `reports_failure` and `drops_silently` with exhaustive match bodies and
3-field fix comments). Imports nothing.

**This is the module the discriminant/handler split is about.** Its own module doc states
the rule in one sentence — "Only the discriminants live here. The handlers that
act on them are `ring_wait` and `ring_overflow` respectively" — and the file
obeys it, though the mechanism that was once checkable by macro count is not any
more. Before the fix above, the file contained three `matches!` and zero `match`;
now it contains one `matches!` (`is_non_blocking`) and two exhaustive `match`
expressions (`reports_failure`, `drops_silently`) — so "zero `match`" is no
longer the fact that proves this file classifies rather than dispatches. What
still proves it: all three functions, macro or `match`, take `self` and return a
`bool` naming which discriminant it is, and none of the three calls into
`ring_wait` or `ring_overflow` or otherwise acts on the classification
(→ [`../../pattern/001`](../../pattern/001_discriminants_here_handlers_elsewhere.md)).

## Kind

Module (§ Item Kind Taxonomy : Stable Item Kinds #1)

## Definition

`ring_types/src/lib.rs:39`

```rust
mod policy;
```

## File Usage

| File | Line(s) | Context |
|------|---------|---------|
| `ring_types/src/lib.rs` | 1, 14, 20, 39, 44 | Crate doc naming the policy enums as part of the remit (1, doc); the rule that a behaviour dispatching on a policy does not belong here (14, doc); module responsibility table (20, doc); **declaration (39)**; re-export path `pub use policy::{ OverflowPolicy, WaitKind };` (44) |

Test-only references: none directly — `tests/types_test.rs` reaches both enums
through the crate-root re-export, in six of its nineteen tests.

## Crate Usage

| Crate | Via File | Purpose |
|-------|----------|---------|
| `ring_types` | `src/lib.rs` | Declares the module so both policy enums and their six associated items exist and can be re-exported |

**Nothing stops this module from growing a handler.** The classify-without-
dispatching property above is a measurement of the current file, not an enforced
constraint — no gate distinguishes a classifying `match` from a dispatching one,
and adding a `pause()` method to `WaitKind` tomorrow would violate the split
without failing anything. The rule is real; the mechanism is a code review.

The other half of the same gap is that the split is not two-crate in practice:
`ring_stats::record_drop` dispatches on `OverflowPolicy` to pick a counter, which
makes three crates that must change when a variant is added, against the two the
ruling names.
