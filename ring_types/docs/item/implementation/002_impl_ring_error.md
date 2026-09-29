# impl RingError

## Representation

The inherent implementation block on [`RingError`](../enum/002_ring_error.md),
holding its two classifiers:
[`is_configuration`](../associated_function/004_ring_error_is_configuration.md)
and [`is_transient`](../associated_function/005_ring_error_is_transient.md).
Both are `const fn`, both take `self` by value. Both were single `matches!`
expressions until `Fix(ring_error_classification_not_exhaustive)`
(`ring_types/src/error.rs:98`) converted each to an exhaustive `match` naming
all nine variants — a `matches!` over a positive list answered every variant
it was not told about with the same default, so a tenth `RingError` variant
would have compiled clean while silently misclassifying. The nine-variant
classification table below is unchanged by that fix; only the construct
enforcing it against growth is.

**The two classifiers do not partition the nine variants, and nothing says so.**
Four variants are configuration, two are transient, and three — `Closed`,
`NameTaken`, `NameUnknown` — are neither. `Closed` is deliberate: a closed ring
is not a config error and retrying will not help. The other two are neither
because nothing constructs them
(→ [`../../algorithm/002`](../../algorithm/002_classifying_an_error_into_configuration_or_traffic.md),
[`../../pitfall/002`](../../pitfall/002_two_name_errors_nothing_constructs.md)).

A caller who assumes `!is_configuration()` means "transient, retry" is wrong for
three of nine variants, and the type gives them no third predicate to ask.

## Kind

Implementation (§ Item Kind Taxonomy : Stable Item Kinds #12)

## Definition

`ring_types/src/error.rs:83`

```rust
impl RingError
{
  pub const fn is_configuration( self ) -> bool   // :111
  pub const fn is_transient( self ) -> bool       // :146
}
```

Two functions and no constant — unlike the two policy enums, `RingError` carries
no `ALL` array, because `#[ non_exhaustive ]` makes an exhaustive roster of a
type consumers may not match exhaustively a contradiction
(→ [`../enum/readme.md`](../enum/readme.md)).

## File Usage

| File | Line(s) | Context |
|------|---------|---------|
| `ring_types/src/error.rs` | 83, 111, 146, 160 | **Block header (83)**; `is_configuration` (111); `is_transient` (146); closing brace (160) |

The block spans 83-160 — wider than before `Fix(ring_error_classification_not_exhaustive)`
(`:98`), which replaced each `matches!` with an exhaustive `match` plus a
3-field fix comment; the classification logic itself did not grow, the
documentation of why it is now exhaustive did.

Test-only references: `ring_types` — `tests/types_test.rs` covers both across two
tests (`errors_split_configuration_from_traffic`, `only_full_and_empty_are_transient`),
each maintaining a hand-written variant roster that the `#[ non_exhaustive ]`
attribute prevents the compiler from checking
(→ [`../../non_functional_requirement/002`](../../non_functional_requirement/002_the_enum_sets_are_closed_and_asserted.md)).

## Crate Usage

| Crate | Via File | Purpose |
|-------|----------|---------|
| `ring_types` | `src/error.rs` | Declares the block |
| `ring_gating` | `src/lib.rs` | **Doc only** — its module doc names `is_configuration` as "the caller's test" (`:38`) and two doc examples assert on it (`:278, :281`) |
| `ring_shutdown` | `src/lib.rs` | **Doc only** — one doc example asserts `!RingError::Closed.is_transient()` (`:128`) |

**Not one line of production code in the family calls either classifier.**

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -r 'is_configuration\|is_transient' ring_*/src | command grep -v ring_types/
```

Live output:

```
ring_gating/src/lib.rs://! the second. `RingError::is_configuration` is the caller's test.
ring_gating/src/lib.rs:  /// assert!( too_wide.is_configuration(), "never retry this one" );
ring_gating/src/lib.rs:  /// assert!( !RingError::Full.is_configuration(), "but do retry this one" );
ring_shutdown/src/lib.rs:  /// assert!( !RingError::Closed.is_transient() );
```

returns four lines, and all four are doc comments — three `///` examples and one
`//!` module-doc sentence. Filtering them out leaves nothing:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -rn 'is_configuration\|is_transient' ring_*/src \
  | command grep -v ring_types/ | command grep -v ':[0-9]*: *//' || true
```

Live output:

```
```

**Nine consumer test suites do call them**, though: `ring_batch`, `ring_claim`,
`ring_core`, `ring_event`, `ring_gating`, `ring_publish`, `ring_shutdown`,
`ring_tls`, `ring_wait`. So the classifiers are a *testing* vocabulary rather
than a runtime one — the family asserts that an error is transient far more often
than it branches on whether it is.

That is not obviously wrong, and it is worth naming rather than hiding behind a
coverage number: G1 reports these two functions as covered, and the coverage is
real, but it comes from assertions about errors rather than from any code that
handles one differently because of the answer.
