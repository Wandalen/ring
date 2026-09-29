# OverflowPolicy::reports_failure

## Representation

True for exactly one of three variants: `OverflowPolicy::Fail`. **Zero production
callers**, and the exact logical complement of its sibling
[`drops_silently`](013_overflow_policy_drops_silently.md).

**The complement relation is the fact worth recording.** Over three variants,
`reports_failure` is true for `{ Fail }` and `drops_silently` for `{ DropNewest,
DropOldest }`. The sets are disjoint and their union is the whole enum, so
`drops_silently() == !reports_failure()` for every value that exists. Two
predicates, one bit of information.

Contrast the `RingError` pair, which looks like the same design and is not:
[`is_configuration`](004_ring_error_is_configuration.md) and
[`is_transient`](005_ring_error_is_transient.md) cover six of nine variants and
leave three answering `false` to both
(→ [`../../algorithm/002`](../../algorithm/002_classifying_an_error_into_configuration_or_traffic.md)).
**Here the partition is total; there it is not, and neither type says which it
is.** The only place the difference is written down is a test name.

## Kind

Associated Function/Method (§ Item Kind Taxonomy : Associated Item Kinds #1)

## Definition

`ring_types/src/policy.rs:153`

```rust
#[ must_use ]
pub const fn reports_failure( self ) -> bool
```

Body spans lines 154-160 (`policy.rs`):

```rust
match self
{
  Self::Fail => true,
  Self::DropNewest | Self::DropOldest => false,
}
```

Until `Fix(overflow_policy_classification_not_exhaustive)` (`policy.rs:140`)
this was `matches!( self, Self::Fail )`. `OverflowPolicy` is **not**
`#[ non_exhaustive ]` — the attribute appears once in the crate, on `RingError`
(`error.rs:43`) — so adding a variant already breaks every downstream
exhaustive `match` at compile time; `matches!`'s implicit `false` arm was the
one place in this function that did not participate, classifying a new
variant as "does not report failure" and compiling silently regardless.

**Previously that gap was closed by a test rather than the compiler.** A fourth
variant would have made `reports_failure` and `drops_silently` both return
`false` for it, breaking the complement relation — and
`overflow_policies_partition_by_reporting` asserts exactly that relation, so
the test would have failed where the compiler did not. The fix adds the
compiler back as a second, earlier line of defence: the test still asserts the
complement relation and is unchanged by the fix, but a fourth variant now also
fails to compile here before that test ever gets a chance to run.

## File Usage

| File | Line(s) | Context |
|------|---------|---------|
| `ring_types/src/policy.rs` | 101-102, 133-134, 136-139, 152-153, 155-159 | Doc example on the enum asserting `Fail` true and `DropNewest` false (101-102); the doc summary — "returns an error rather than silently dropping something" (133-134); doc example counting the matches across `ALL` (136-139); `#[ must_use ]` and **the definition (152-153)**; the exhaustive `match` (155-159) |

Test-only references: `ring_types` — `tests/types_test.rs:220`,
`overflow_policies_partition_by_reporting`, which is the only place the
complement relation is stated anywhere:

```rust
for policy in OverflowPolicy::ALL
{
  assert_ne!
  (
    policy.reports_failure(),
    policy.drops_silently(),
    "{policy:?} must be exactly one of reporting or silently dropping"
  );
}
assert_eq!( OverflowPolicy::ALL.iter().filter( | p | p.reports_failure() ).count(), 1 );
assert_eq!( OverflowPolicy::ALL.iter().filter( | p | p.drops_silently() ).count(), 2 );
```

**`assert_ne!` between two predicates is an unusual and precise assertion.** It
does not check either membership; it checks that they disagree, for every
variant, which is the property that makes one of the two removable and both of
them safe to keep.

## Crate Usage

| Crate | Via File | Purpose |
|-------|----------|---------|
| `ring_types` | `src/policy.rs` | Defining crate |
| `ring_overflow` | `tests/overflow_test.rs` | Asserts the **handler's actual outcome** agrees with the predicate (`:296`) |

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -rn '\.reports_failure(' ring_*/src | command grep -v '^ring_types/' || true
```

Live output:

```
```

is empty — **not one reference in any sibling crate's source, not even a doc
comment**, which is stricter than the `RingError` classifiers managed (they have
three doc-comment mentions).

**`ring_overflow`'s single test is the strongest defence any predicate in this
crate has**, and it is stronger than `is_non_blocking`'s manual check:

```rust
assert_eq!(
  outcome.is_err(), policy.reports_failure(),
  "{policy:?} disagrees with its own reports_failure()"
);
```

It does not assert what the predicate returns. It asserts that the handler's
behaviour and the predicate's claim agree — so a drift in either direction fails,
in the crate that owns the behaviour, which is exactly what decision 121 § 5's
discriminant/handler split is for
(→ [`../../pattern/001`](../../pattern/001_discriminants_here_handlers_elsewhere.md)).

## Caller Tree

- *No caller within `ring_types`*
- *External production callers: none*
- *External test callers: `ring_overflow/tests/overflow_test.rs:296`*

## Callee Tree

- *(none)* — an exhaustive `match`, not a macro or a call, since
  `Fix(overflow_policy_classification_not_exhaustive)`

**One external test call site, and it is worth more than the eleven
[`is_configuration`](004_ring_error_is_configuration.md) has.** Those eleven
assert the predicate against a hand-written expectation; this one asserts it
against a running implementation. A predicate checked against a literal is
checked against whoever wrote the literal; a predicate checked against behaviour
is checked against the system.
