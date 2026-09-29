# impl Display for RingError

## Representation

The hand-written `core::fmt::Display` implementation, holding the crate's single
associated function that is not `const` and, more importantly, **the one
`match` among six that selects among actions rather than classifying a
variant**:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -r '\bmatch\b' ring_types/src/ | command grep -v ': *//'
```

Live output:

```
ring_types/src/error.rs:    match self
ring_types/src/error.rs:    match self
ring_types/src/error.rs:    match self
ring_types/src/policy.rs:    match self
ring_types/src/policy.rs:    match self
ring_types/src/policy.rs:    match self
```

Six lines, not one. The other five are `is_configuration`, `is_transient`
(`error.rs`) and `is_non_blocking`, `reports_failure`, `drops_silently`
(`policy.rs`) — every classification predicate the crate has, each a total
function to `bool`. This block is the only one that maps every variant to a
distinct string rather than to a yes/no answer
(→ [`../../pattern/001`](../../pattern/001_discriminants_here_handlers_elsewhere.md)).

**None of the five classifiers is a `matches!` any more.** Each one was, until
its own `Fix(..._classification_not_exhaustive)` converted it to the same
exhaustive shape this block already had —
`Fix(ring_error_classification_not_exhaustive)` for `is_configuration` and
`is_transient` here in `error.rs`,
`Fix(wait_kind_is_non_blocking_classification_not_exhaustive)` and
`Fix(overflow_policy_classification_not_exhaustive)` for `is_non_blocking`,
`reports_failure` and `drops_silently` in `policy.rs`. This block was never
the exception on *keyword* grounds to begin with; it is the exception on
*behavioural* grounds, and the conversions only removed a syntactic tell that
happened to line up with the real distinction for a while.

**This block is what actually closes the `RingError` variant set at compile
time.** The `match` at line 166 is wildcard-free and lives inside the defining
crate, where `#[ non_exhaustive ]` does not apply, so adding a variant to the
enum makes *this file* fail to compile. That is a mechanism the crate's own tests
cannot provide — a `match` written in `tests/` is a separate crate and needs a
wildcard arm, which absorbs a new variant silently
(→ [`../../non_functional_requirement/002`](../../non_functional_requirement/002_the_enum_sets_are_closed_and_asserted.md), T3a).

The recorded incident fits exactly. When `PolicyUnsupported` was added while
implementing `ring_core`, the compiler forced an arm *here*; what got skipped was
the hand-maintained roster in `tests/types_test.rs`, and gate G1's coverage
threshold caught that instead, reporting `ring_types/src/error.rs 16/17`.

## Kind

Implementation (§ Item Kind Taxonomy : Stable Item Kinds #12)

## Definition

`ring_types/src/error.rs:162`

```rust
impl fmt::Display for RingError
{
  fn fmt( &self, f : &mut fmt::Formatter< '_ > ) -> fmt::Result   // :164
}
```

Named via [`use core::fmt;`](../use_declaration/006_use_core_fmt.md) rather than
fully qualified — the opposite convention to its neighbour twenty-two lines
below (→ [`004_impl_error_for_ring_error.md`](004_impl_error_for_ring_error.md)).
The import pays for itself here because `fmt` appears three times in three lines:
the trait path, the `Formatter` parameter, and the return type.

## File Usage

| File | Line(s) | Context |
|------|---------|---------|
| `ring_types/src/error.rs` | 162, 164, 166-180, 182 | **Block header (162)**; [`fmt`'s signature (164)](../associated_function/006_display_fmt_for_ring_error.md); **the wildcard-free nine-arm `match` (166-180)**; closing brace (182) |

Twenty-one lines for nine messages — the price of not taking a `thiserror`
dependency, paid once so that tier 0's `[dependencies]` table stays empty
(→ [`../../invariant/002`](../../invariant/002_tier_zero_depends_on_nothing.md)).

Test-only references: `ring_types` — `tests/types_test.rs`'s
`every_error_displays_distinctly` formats a hand-written roster of all nine
variants and asserts the messages are pairwise distinct. That roster is the one
`PolicyUnsupported` was missing from.

## Crate Usage

| Crate | Via File | Purpose |
|-------|----------|---------|
| `ring_types` | `src/error.rs` | Declares the impl |
| *(nineteen consumers)* | `src/lib.rs` each | Format a `RingError` into a message — through `{}`, through `.to_string()`, or through `Box< dyn Error >`, none of which names this block |

**A `Display` impl has no call sites in the grep sense** — it is reached by
formatting machinery, so no consumer's source contains a string that matches it.
Its usage is exactly the set of crates that ever print an error, which is a
superset of the crates that construct one and is not mechanically recoverable
from the source.
