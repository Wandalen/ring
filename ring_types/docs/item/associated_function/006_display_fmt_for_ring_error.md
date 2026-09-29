# Display::fmt for RingError

## Representation

The crate's only trait method, its only function taking `&self`, its only
non-`const fn`, and its longest body — nine match arms rendering each variant to
a distinct human-readable line.

**Every one of those four "onlys" is imposed by the trait rather than chosen.**
`core::fmt::Display::fmt`'s signature is fixed: `&self`, a `&mut Formatter`, a
`fmt::Result`. It cannot be `const` because `Formatter` writing is not, and it
cannot take `self` by value because the trait says otherwise. So the item that
looks least like the rest of the crate is the one item the crate did not get to
shape (→ [`../implementation/003_impl_display_for_ring_error.md`](../implementation/003_impl_display_for_ring_error.md)).

## Kind

Associated Function/Method (§ Item Kind Taxonomy : Associated Item Kinds #1)

## Definition

`ring_types/src/error.rs:164`

```rust
fn fmt( &self, f : &mut fmt::Formatter< '_ > ) -> fmt::Result
```

No `pub` — trait methods inherit the trait's visibility. No `#[ must_use ]` —
`fmt::Result` is a `Result` and carries it.

Body is a nine-arm `match self` (`:166`–`:180`), each arm a `write!`:

| Line | Variant | Message |
|------|---------|---------|
| 168 | `CapacityZero` | `ring capacity must be at least 1` |
| 169 | `CapacityNotPowerOfTwo( n )` | `ring capacity {n} is not a power of two` |
| 170 | `Full` | `ring is full` |
| 171 | `Empty` | `ring is empty` |
| 172 | `Closed` | `ring is closed` |
| 173 | `NameTaken` | `a ring is already registered under this name` |
| 174 | `NameUnknown` | `no ring is registered under this name` |
| 177 | `BatchTooLarge { requested, capacity }` | `batch of {requested} exceeds ring capacity {capacity}` |
| 179 | `PolicyUnsupported` | `this backend cannot honour the configured overflow policy` |

**Two arms interpolate their payload and seven do not**, which is the same split
[`is_configuration`](004_ring_error_is_configuration.md) deliberately ignores
with its wildcards. Here the payload is the message's whole value: `ring capacity
is not a power of two` without the offending number would send the caller back to
their own logs.

**The match has no wildcard arm**, so it is exhaustive over the nine variants.
`#[ non_exhaustive ]` does not weaken that — the attribute constrains *other*
crates' matches, not the defining crate's. Until each classifier's own
`Fix(..._classification_not_exhaustive)` landed, this was **the one place in
the workspace where adding a variant was a compile error rather than a silent
`false`** — [`is_configuration`](004_ring_error_is_configuration.md),
[`is_transient`](005_ring_error_is_transient.md),
[`is_non_blocking`](011_wait_kind_is_non_blocking.md), `reports_failure` and
`drops_silently` were all a `matches!` over a positive list, so a variant they
did not name reached them through the macro's implicit `false` arm.
`Fix(ring_error_classification_not_exhaustive)`,
`Fix(wait_kind_is_non_blocking_classification_not_exhaustive)` and
`Fix(overflow_policy_classification_not_exhaustive)` converted all five to the
same exhaustive shape this `match` already had, so the crate now has six such
places, not one — this item is no longer the exception, it is the pattern the
other five were fixed to match.

## File Usage

| File | Line(s) | Context |
|------|---------|---------|
| `ring_types/src/error.rs` | 13, 162, 164, 166, 168-174, 177, 179 | `use core::fmt;` (13); the `impl` header (162); **the definition (164)**; `match self` (166); the seven unit arms (168-174); the two payload arms (177, 179) |

Test-only references: `ring_types` — 4 in `tests/types_test.rs`, all reaching the
method through `to_string()` rather than by name (`:302`, `:310`, `:311` and the
`batch` assertion that follows).

## Crate Usage

| Crate | Via File | Purpose |
|-------|----------|---------|
| `ring_types` | `src/error.rs` | Defining crate |

**No consumer crate formats a `RingError`, in production or in tests.**

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -rn 'to_string()' ring_*/src | command grep -v '^ring_types/' \
  | command grep -v ':[0-9]*: *//' || true
```

Live output:

```
```

is empty. The `to_string()` calls that do exist in sibling test suites —
`ring_bench` ×9, `ring_registry` ×6, `ring_testkit` ×4 and six others — are on
names, labels and report rows, not on this error. **The rendering exists for a
consumer behind the export Contract, and the family is not it**, which is the
same finding the five predicates produce
(→ [`../../api/002`](../../api/002_the_five_classifier_predicates.md)).

`to_string()` itself is an `alloc` facility, not a `core` one. The method being
documented is pure `core::fmt`, so it survives the crate's `no_std`
compatibility; every *use* of it recorded above does not
(→ [`../../non_functional_requirement/003`](../../non_functional_requirement/003_the_crate_compiles_without_std.md)).

## Caller Tree

- *No caller by name anywhere in the workspace* — the method is reached through formatting machinery
- *External: `core::fmt::Display::to_string` / `ToString::to_string`* — via `alloc`, from `ring_types/tests/types_test.rs:302, 310, 311`
- *External: any `{}` interpolation of a `RingError`* — none exists in the family

## Callee Tree

- *External: `core::write!`* ×9 (`error.rs:168-174,177,179`) — the only macro
  invocation this function makes; `BatchTooLarge`'s arm is the one braced
  body, so its `write!` sits at :177 rather than on the match-arm line.
  `Fix(ring_error_classification_not_exhaustive)`,
  `Fix(wait_kind_is_non_blocking_classification_not_exhaustive)` and
  `Fix(overflow_policy_classification_not_exhaustive)` between them converted
  every one of the crate's five classifiers to an exhaustive `match`, so
  `matches!` now appears nowhere in `src/` — including `policy.rs`'s
  `is_non_blocking`, the last of the five to convert.
- *External: `core::fmt::Formatter::write_fmt`* — what each `write!` expands to

**This is the one item in the crate whose coverage is load-bearing.** Nine arms,
each executed only if a test constructs that variant, and
`every_error_displays_distinctly` maintains the roster by hand.
`RingError` is `#[ non_exhaustive ]`, so that test — living in a separate crate —
needs a wildcard and cannot be made to fail the build when a variant is added.

**It has already happened once.** `PolicyUnsupported` was added while
implementing `ring_core`, the roster was not updated, and gate G1 reported
`ring_types/src/error.rs 16/17` on the next run. Detection took one gate run
instead of a compiler error — slower, and not silent, which is the property that
matters (→ [`../../invariant/003`](../../invariant/003_every_error_renders_distinctly.md)).
