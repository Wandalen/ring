# OverflowPolicy::ALL

## Representation

Every `OverflowPolicy` variant, in discriminant order, as a `[ Self; 3 ]`. **The
crate's one roster with a production consumer.** `ring_stats::dropped_total` sums
drop counts by iterating this array, so the constant is load-bearing at runtime
in a way [`WaitKind::ALL`](002_wait_kind_all.md) is not.

**It was also, for a time, the roster with no assertion pinning its contents —
and this document's own account of that gap was wrong in both directions.** The
original text simulated every candidate array against the assertions it had
enumerated:

```text
[DropNewest,DropOldest,Fail]  len==3:true  assert_ne:true  rf==1:true  ds==2:true  → correct
[Fail,Fail,Fail]              len==3:true  assert_ne:true  rf==1:FALSE ds==2:FALSE → CAUGHT
[DropNewest,DropNewest,Fail]  len==3:true  assert_ne:true  rf==1:true  ds==2:true  → SURVIVES
[DropOldest,DropOldest,Fail]  len==3:true  assert_ne:true  rf==1:true  ds==2:true  → SURVIVES
```

and concluded that "the nineteen test references that exist across two suites all
*sweep* it, so none of them can detect a corrupted entry."

**Running the four arms refutes both halves.** The simulation enumerated four unit
assertions and no doctests, so row 3 is wrong: the `contains` doc example on this
very constant asserts `contains( &DropOldest )` and fails on
`[DropNewest,DropNewest,Fail]`. And nine of the nineteen consumer references do
detect a corrupted entry — the ones whose assertions require the policies to
*differ from one another*, which is exactly what a duplicate destroys
(→ [`../../non_functional_requirement/002`](../../non_functional_requirement/002_the_enum_sets_are_closed_and_asserted.md),
§ T6, where every arm and every failing test name is tabulated).

| Arm | Before the fix | After |
|-----|----------------|-------|
| `[Fail,Fail,Fail]` | caught — both predicate counts move | caught |
| `[DropNewest,DropNewest,Fail]` | caught — by this constant's own doctest | caught by name |
| `[DropOldest,DropOldest,Fail]` | **survived** — 19/19 unit and 20/20 doctests green, with the `#[ default ]` policy gone from the roster | caught by name |

Only the third arm was ever the real gap, and the original analysis did not name
it. That is the finding: a documented gap is a hypothesis about a suite, and this
suite answered differently than the document predicted.

## Kind

Associated Constant (§ Item Kind Taxonomy : Associated Item Kinds #2)

## Definition

`ring_types/src/policy.rs:131`

```rust
pub const ALL : [ Self; 3 ] = [ Self::DropNewest, Self::DropOldest, Self::Fail ];
```

The first item in `impl OverflowPolicy`, ahead of
[`reports_failure`](../associated_function/012_overflow_policy_reports_failure.md)
and [`drops_silently`](../associated_function/013_overflow_policy_drops_silently.md)
(→ [`../implementation/008_impl_overflow_policy.md`](../implementation/008_impl_overflow_policy.md)).

**Its doc example is a single `contains`, which an earlier form of this note
called the weak choice. Measurement inverted that:**

```rust
assert!( OverflowPolicy::ALL.contains( &OverflowPolicy::DropOldest ) );
```

[`WaitKind::ALL`](002_wait_kind_all.md)'s doc example asserts `len() == 4`;
`is_non_blocking`'s asserts a count over the array. **The two rosters are one line
apart in the same file and their doc examples check different things**, with
nothing marking the difference — that observation stands. What does not stand is
the ranking put on it. A length is true of any array of that length and pins
nothing about contents; a count catches only a corruption that moves the count.
This `contains` is the one assertion of the three that fails on a duplicate, and
it was the only check anywhere in the crate that caught the
`[DropNewest,DropNewest,Fail]` arm.

Its weakness is scope, not kind: one variant of three. So the fix was not to
replace it but to generalise it — the same `contains`, over all three variants,
in `overflow_policy_has_no_overwrite_variant` (`tests/types_test.rs:203`), where
the third arm now fails by name too. Membership is the right shape for this
class; a sharper count would not have been.

## File Usage

| File | Line(s) | Context |
|------|---------|---------|
| `ring_types/src/policy.rs` | 100, 122-129, 131, 138 | Doc example on the enum asserting `ALL.len() == 3` (100); doc summary and the single-`contains` example (122-129); **the definition (131)**; used by `reports_failure`'s doc example, `ALL.iter().filter( … ).count() == 1` (138) |

Test-only references: `ring_types` — `tests/types_test.rs:198`,
`overflow_policy_has_no_overwrite_variant` (length, **the per-variant `contains`
loop that closed T6**, and an exhaustive `match`), and `:220`,
`overflow_policies_partition_by_reporting` (the `assert_ne!` plus two counts).
Plus 19 across two consumer suites — 11 in `ring_stats`' `stats_test.rs`, 8 in
`ring_overflow`'s `overflow_test.rs`.

## Crate Usage

| Crate | Via File | Purpose | Refs |
|-------|----------|---------|-----:|
| `ring_types` | `src/policy.rs` | Defining crate | — |
| `ring_stats` | `src/lib.rs`, `tests/` | **`dropped_total` sums over the array** (`:378` — the one production reference); doc example (`:360`); a prose mention of the length convention (`:185`); 13 test references (11 in `stats_test.rs`, 2 in `tests/manual/readme.md`) | 3 src, 13 tests |
| `ring_overflow` | `src/lib.rs`, `tests/overflow_test.rs` | Sweeps every policy through the handler; `src/lib.rs:82` is a doc comment naming this array as the model for its own roster, not a use | 1 src (doc), 8 tests |

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -r 'OverflowPolicy::ALL' ring_*/src | command grep -v '^ring_types/' \
  | command grep -v ':[ \t]*//'
```

Live output:

```
ring_stats/src/lib.rs:        OverflowPolicy::ALL.iter().map(|p| self.dropped(*p)).fold(0, u64::saturating_add)
```

This is one line — **the family's only production use of any `ALL` array**.

**A corrupted roster becomes a wrong number rather than a failure at this line.**
With `[ DropNewest, DropNewest, Fail ]`, `dropped_total` double-counts
`DropNewest` and omits `DropOldest` entirely — the total is wrong and nothing at
`:378` objects, because a sum has no opinion about what it is summing over.

**But the suites around it do object, and that is where the earlier reading went
wrong.** It generalised from the line above to "a test that sweeps `ALL` cannot
validate `ALL`", and every arm refutes that: four of `ring_stats`' eleven
`stats_test.rs` references fail on both duplicate arms, and five of
`ring_overflow`'s eight do. What separates the nine that catch it from the ten
that do not is not that they avoid sweeping — all nineteen sweep — but *what they
assert while sweeping*. `a_drop_lands_under_its_own_policy_only`,
`distinct_policies_give_distinct_resolutions` and
`counts_accumulate_and_stay_separated` require the policies to resolve to
*different* places; a roster holding one variant twice cannot satisfy that. The
ones that inherit the corruption are the ones whose assertions hold of each
element considered alone.

**`ring_overflow`'s eight references are sweeps too**, and they check behaviour
rather than restating the roster: two of its assertions compare the handler's
actual outcome against
[`reports_failure`](../associated_function/012_overflow_policy_reports_failure.md)
and [`drops_silently`](../associated_function/013_overflow_policy_drops_silently.md).
Those catch a predicate that drifts from the implementation, and — because they
are per-element facts — they are among the ones that do *not* catch a lost
variant. Its five that do are the separation and counter-isolation assertions.

**The contrast with `WaitKind::ALL` was the original finding, and it has been
resolved rather than merely restated.** That constant has zero production
references and two independent order-and-contents assertions — one in
`ring_types`, a stronger one in `ring_wait`. This one has a production reference
doing arithmetic and, until T6 closed, no assertion in its own crate pinning its
contents. The defence was inverted relative to the exposure; the `contains` loop
in `overflow_policy_has_no_overwrite_variant` is what un-inverted it, in the crate
that declares the constant rather than two crates downstream.
