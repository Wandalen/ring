# OverflowPolicy::drops_silently

## Representation

True for two of three variants: `DropNewest` and `DropOldest`. **Zero production
callers**, and — over the enum as it exists — logically identical to
`!reports_failure()`.

The complement argument is made in full at
[`012_overflow_policy_reports_failure.md`](012_overflow_policy_reports_failure.md)
and not repeated. What belongs here is the half of the pair that carries the
warning rather than the reassurance.

**`reports_failure` names a behaviour a caller wants; this one names a behaviour
a caller must not overlook.** A publish under `DropNewest` or `DropOldest`
returns `Ok`, and an item is gone. Nothing in the return type says so, and the
family's own feature documentation is emphatic that no variant overwrites *unread*
data — a guarantee this predicate does not contradict and does not restate:
`DropOldest` discards an item the ring holds and no consumer has taken, which is
precisely "loses an item without telling the caller."

**So the crate has a variant whose danger is documented only in a predicate
nobody calls.** `OverflowPolicy::DropOldest` is also the variant `ring_core`
refuses outright — `RingError::PolicyUnsupported` exists because evicting an
unread record contradicts an exactly-once backend's guarantee
(→ [`../enum/002_ring_error.md`](../enum/002_ring_error.md)).

## Kind

Associated Function/Method (§ Item Kind Taxonomy : Associated Item Kinds #1)

## Definition

`ring_types/src/policy.rs:178`

```rust
#[ must_use ]
pub const fn drops_silently( self ) -> bool
```

Body spans lines 179-185 (`policy.rs`):

```rust
match self
{
  Self::DropNewest | Self::DropOldest => true,
  Self::Fail => false,
}
```

Until `Fix(overflow_policy_classification_not_exhaustive)` (`policy.rs:170`)
this was `matches!( self, Self::DropNewest | Self::DropOldest )` — two arms,
and at the time the crate's **last** associated function by both file order
and catalog ID. The exhaustive form now names `Fail` explicitly in its own
`false` arm rather than leaving it to the macro's implicit default.

**Its doc example is the only one of the five predicates' that asserts a
positive and a negative case rather than a count:**

```rust
assert!( OverflowPolicy::DropOldest.drops_silently() );
assert!( !OverflowPolicy::Fail.drops_silently() );
```

`is_non_blocking` and `reports_failure` both use
`ALL.iter().filter( … ).count()`, which survives a variant being added.
This one does not — a fourth silently-dropping variant leaves both assertions
true and the example still passing. The count form is the better pattern and
this item did not get it
(→ [`../associated_constant/003_overflow_policy_all.md`](../associated_constant/003_overflow_policy_all.md)).

The gap is covered anyway, by
`overflow_policies_partition_by_reporting`'s third assertion — `count() == 2` —
which is in the test file rather than the doc comment. **The property is
asserted; the documentation just isn't the thing asserting it.**

## File Usage

| File | Line(s) | Context |
|------|---------|---------|
| `ring_types/src/policy.rs` | 162-163, 165-169, 177-178, 179-185 | Doc summary — "loses an item without telling the caller — true for both drop variants" (162-163); doc example asserting `DropOldest` true and `Fail` false (165-169); `#[ must_use ]` and **the definition (177-178)**; the body (179-185) |

Test-only references: `ring_types` — `tests/types_test.rs:220`,
`overflow_policies_partition_by_reporting`, twice: once inside the `assert_ne!`
against `reports_failure`, once in the `count() == 2` assertion.

## Crate Usage

| Crate | Via File | Purpose |
|-------|----------|---------|
| `ring_types` | `src/policy.rs` | Defining crate |
| `ring_overflow` | `tests/overflow_test.rs` | Asserts the handler actually lost an item when the predicate says it would (`:301`) |

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -rn '\.drops_silently(' ring_*/src | command grep -v '^ring_types/' || true
```

Live output:

```
```

is empty — no source reference of any kind outside this crate.

**`ring_overflow`'s assertion is the mirror of its `reports_failure` one, and the
sharper of the two:**

```rust
assert_eq!(
  outcome.is_ok_and( Resolution::lost_an_item ), policy.drops_silently(),
  "{policy:?} disagrees with its own drops_silently()"
);
```

`is_ok_and( Resolution::lost_an_item )` is the whole point. Checking `is_ok()`
alone would pass for a policy that silently dropped nothing, or dropped and
reported. The handler is required to have *both* returned success *and* lost an
item, exactly when this predicate says it would
(→ [`../../pattern/001`](../../pattern/001_discriminants_here_handlers_elsewhere.md)).

## Caller Tree

- *No caller within `ring_types`*
- *External production callers: none*
- *External test callers: `ring_overflow/tests/overflow_test.rs:301`*

## Callee Tree

- *(none)* — `matches!` expands in place

**The last of thirteen, and the one whose name does the most work.** Four of the
thirteen associated functions have no production caller and all four are
classifiers (→ [`readme.md`](readme.md)). Of those four, this is the only one
naming a hazard rather than a category — and the only one whose external test
asserts a *loss* occurred rather than a value returned. Its cost is three lines
and a doc example; its value is that `drops_silently` is a harder name to ignore
in a config file than `DropNewest`.
