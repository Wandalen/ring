# Type: `Backend`

### Scope

- **Purpose**: Describe the one public type this crate introduces — a name for the choice made at construction — and account for the fact that its variant count is a build property.
- **Responsibility**: Variants, derives, the reporting method, and the two consequences of the count being conditional.
- **In Scope**: `Backend`, `Ring::backend()`.
- **Out of Scope**: The private `Storage` enum it names (→ [`data_structure/001`](../data_structure/001_three_way_storage_enum.md)); how the choice is made (→ [`lifecycle/001`](../lifecycle/001_construction_and_backend_selection.md)).

### Definition

```rust
pub enum Backend
{
  Spsc,
  Mpsc,
  #[ cfg( feature = "crossbeam" ) ]
  Crossbeam,
}
```

Three variants with the feature on, **two without**. `Ring::backend()` is
`const` and reports which one is underneath.

#### Why a public name for a private choice

The `Storage` enum is private and stays that way — exposing it would put
`ring_spsc::Ring` and `ArrayQueue` in this crate's public API and make every
backend's version a breaking change here.

`Backend` is the projection of that choice with nothing attached: a caller can
*learn* which backend is running without being handed it. That matters for
exactly two audiences, and it was added for the first:

- **Tests**, which must parameterize over the build's actual backends. The
  reached-test iterates a `Vec< Backend >` and asserts each ring reports the
  variant it was asked for. Without a public name, the same test would have to
  be written three times behind `cfg`, which is the outcome this crate's
  build-flag-not-rewrite requirement exists to avoid.
- **Benchmarks** (`ring_bench`), which must label a result with
  what produced it. A benchmark that reports a number without the backend it
  measured is not a measurement.

### Validation

1. **A `match` on `Backend` in a downstream crate is not portable between
   builds.** A three-arm match fails to compile without the feature; a two-arm
   match fails with it. Callers should match with a `_` arm or not match at all.
   This crate's own tests take the second route — they build the list once and
   iterate it.

2. **No compile-time check can assert the variant roster from outside.** Adding
   a fourth variant would not break any external test. This is the same shape as
   `ring_types::RingError`'s hand-maintained roster, which
   `#[ non_exhaustive ]` puts beyond external checking and which G1's 100%
   coverage threshold catches instead. Here the equivalent detector is
   `every_backend_the_build_offers_is_actually_exercised`, which pins the count
   per configuration — three with the feature, two without — so a variant added
   and forgotten fails a test rather than passing silently.

### Derives

`Debug, Clone, Copy, PartialEq, Eq, Hash`.

Five of the six are load-bearing: `Copy` because the value is passed by value
into a construction helper, `Clone` as `Copy`'s prerequisite, `PartialEq`/`Eq`
because tests and doc examples assert on it, and `Debug` because
`every_public_type_is_debuggable` requires the whole public surface to be
printable — a `Debug` that stops compiling under one feature configuration is a
real regression, and that test is what catches it.

**`Hash` is used by nothing.** Measured:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -rn 'HashMap\|HashSet\|\.hash(' ring_core/src ring_core/tests || true
```

Live output:

```
```

It is kept rather than removed on one concrete expectation, not a general
"someone might": `ring_bench` must key results by backend, and a map keyed on
`Backend` is the obvious shape for that. If `ring_bench` lands without using
it, the derive should go — an unused derive is a claim about the type that
nothing checks. Recorded here so that decision is made rather than inherited.

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_three_way_storage_enum.md](../data_structure/001_three_way_storage_enum.md) | The private enum this names |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_construction_and_backend_selection.md](../lifecycle/001_construction_and_backend_selection.md) | Where the value reported here is decided |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_feature_gated_code_reads_as_uncovered.md](../pitfall/002_feature_gated_code_reads_as_uncovered.md) | The measurement consequence of the same conditional-arity property |

### Tests

| File | Relationship |
|------|--------------|
| `tests/core_test.rs` | `every_backend_the_build_offers_is_actually_exercised` — consequence 2's detector |
| `tests/core_test.rs` | `the_producer_count_selects_between_spsc_and_mpsc`, `the_crossbeam_backend_ignores_the_producer_count` — that `backend()` reports the truth |
| `tests/core_test.rs` | `every_public_type_is_debuggable` — the derives |

### CO50 — `Backend` Is a Bare Discriminant With No Payload and No Callers

`Backend` has three unit variants and holds nothing. That is the right shape for
its job — it answers "which one" and nothing else — and it means the type's
entire value is in being read.

It is read twice, both in `ring_factory/tests/factory_test.rs`
(→ [`../item/002`](../item/002_the_backend_discrimination_surface.md), CO31).
A data-free type whose only purpose is to be compared, and which nothing
compares outside a test, is the clearest single statement of what
`decisions/001` is about.
