# impl WaitKind

## Representation

The inherent implementation block on [`WaitKind`](../enum/001_wait_kind.md),
holding the [`ALL`](../associated_constant/002_wait_kind_all.md) roster and the
single classifier
[`is_non_blocking`](../associated_function/011_wait_kind_is_non_blocking.md).

**Two members, and neither is a strategy.** `ALL` enumerates the four
discriminants; `is_non_blocking` is a `matches!` answering whether this variant
is `None`. Nothing here spins, yields or parks — those live in `ring_wait`, per
the discriminant/handler split that makes this block's contents a
list of what the crate is *allowed* to do with a discriminant it owns
(→ [`../../pattern/001`](../../pattern/001_discriminants_here_handlers_elsewhere.md)).

The rule holds today by construction: `policy.rs` contains three `matches!` and
zero `match`. Nothing enforces it — no gate greps this file — so a `pause()`
added here tomorrow would violate the ruling without failing anything.

## Kind

Implementation (§ Item Kind Taxonomy : Stable Item Kinds #12)

## Definition

`ring_types/src/policy.rs:37`

```rust
impl WaitKind
{
  pub const ALL : [ Self; 4 ] = [ Self::Spin, Self::Yield, Self::Park, Self::None ];   // :58
  pub const fn is_non_blocking( self ) -> bool                                          // :81
}
```

## File Usage

| File | Line(s) | Context |
|------|---------|---------|
| `ring_types/src/policy.rs` | 37, 58, 81, 89 | **Block header (37)**; `ALL` (58); `is_non_blocking` (81); closing brace (89) |

Test-only references: `ring_types` — two of the nineteen tests exercise this
block. `wait_kind_has_exactly_four_variants` asserts `ALL.len() == 4`, then loops
a `contains` check per variant, then runs a wildcard-free `match` over `ALL`.
`exactly_one_wait_kind_is_non_blocking` filters `ALL` by the classifier and
asserts the count is 1.

**The `contains` loop was the assertion `OverflowPolicy`'s equivalent test was
missing, until T6 closed the gap.** `overflow_policy_has_no_overwrite_variant`
now carries the identical loop, so the two policy blocks are equally tested
(→ [`../../non_functional_requirement/002`](../../non_functional_requirement/002_the_enum_sets_are_closed_and_asserted.md), T6, closed).

## Crate Usage

| Crate | Via File | Purpose |
|-------|----------|---------|
| `ring_types` | `src/policy.rs` | Declares the block |
| `ring_config` | `src/lib.rs` | **The only production caller** — `self.wait.is_non_blocking()` at `:240` |
| `ring_wait` | `src/lib.rs` | Doc only — its module doc cites `[ring_types::WaitKind::is_non_blocking]` at `:39` as the definition of the tick-path variant |

**The two members have opposite usage profiles**, and only a per-item count shows
it:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -r 'WaitKind::ALL\|is_non_blocking' ring_*/src | command grep -v ring_types/
```

Live output:

```
ring_config/src/lib.rs:    self.wait.is_non_blocking()
ring_wait/src/lib.rs://! [`ring_types::WaitKind::is_non_blocking`] is true for exactly that variant.
```

returns two lines — one real call, one doc citation. Broadening to `tests/` adds
eight more, in `ring_config` and `ring_wait`.

**`WaitKind::ALL` has zero source references outside this crate**, and that is by
design rather than by neglect: its own doc says it exists so a test can assert
the set is exactly these four. Its consumers are `ring_wait/tests/wait_test.rs`
(five references) and `ring_config/tests/config_test.rs` (one), both iterating it
to check a property holds for every variant.

**`is_non_blocking` has exactly one production caller, and that call is itself
documented as a decision.** `ring_config`'s manual test plan records check M4 —
"the body delegates to `WaitKind::is_non_blocking` rather than [a local match]" —
and its 2026-08-28 result reads "Body is `self.wait.is_non_blocking()` —
delegation, not a local match." So the one crate that consults the predicate has
a written check that it keeps consulting it rather than re-deriving the rule
locally. That is the discriminant/handler split being defended from the consumer
side, which is the side no rule covers explicitly.
