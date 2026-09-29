# RingError::is_transient

## Representation

Answers whether retrying the identical call could succeed with nothing else
changing — true for exactly two of nine variants. **Zero production callers in
the family.**

**Was the shortest body in the crate** — a one-line `matches!` — until
`Fix(ring_error_classification_not_exhaustive)` (`ring_types/src/error.rs:136`)
expanded it to an exhaustive `match` naming all nine variants, the same
conversion applied to its sibling [`is_configuration`](004_ring_error_is_configuration.md)
alongside it: a `matches!` over `Full | Empty` answered `false` for every
variant it was not told about, so a new transient condition would have compiled
clean while silently telling a caller to give up rather than retry.

The doc comment states its own membership rule precisely enough to check,
unchanged by the fix: "true for exactly the two conditions a peer's progress
clears." `Full` clears when a consumer drains; `Empty` clears when a producer
publishes. No other variant has a peer whose ordinary operation resolves it —
`Closed` least of all, which is the gap
[`../../algorithm/002`](../../algorithm/002_classifying_an_error_into_configuration_or_traffic.md)
argues.

Paired with [`is_configuration`](004_ring_error_is_configuration.md) in the same
inherent `impl` block; the two are the block's entire contents.

## Kind

Associated Function/Method (§ Item Kind Taxonomy : Associated Item Kinds #1)

## Definition

`ring_types/src/error.rs:146`

```rust
#[ must_use ]
pub const fn is_transient( self ) -> bool
```

Body spans lines 147-159:

```rust
match self
{
  Self::Full | Self::Empty => true,
  Self::CapacityZero
  | Self::CapacityNotPowerOfTwo( _ )
  | Self::Closed
  | Self::NameTaken
  | Self::NameUnknown
  | Self::BatchTooLarge { .. }
  | Self::PolicyUnsupported => false,
}
```

Both `true`-arm variants are unit variants, so unlike its sibling's `true` arm
this one needs no payload wildcards — the `false` arm's `CapacityNotPowerOfTwo`
and `BatchTooLarge` still do. **The exhaustive match costs one branch more than
the old `matches!` at runtime and nothing at compile time** — `const fn` plus a
`Copy` receiver means the cost is still entirely the call, and there usually is
not one.

## File Usage

| File | Line(s) | Context |
|------|---------|---------|
| `ring_types/src/error.rs` | 132-134, 145-146, 148-158 | Doc example asserting `Full`, `Empty` true and `Closed` false (132-134); `#[ must_use ]` and **the definition (145-146)**; the exhaustive `match` set (148-158) |

Test-only references: `ring_types` — 6 in `tests/types_test.rs`, in
`only_full_and_empty_are_transient`, which walks all nine variants. Plus 6 across
4 consumer suites.

## Crate Usage

| Crate | Via File | Purpose | Sites |
|-------|----------|---------|-------|
| `ring_types` | `src/error.rs` | Defining crate | — |
| `ring_shutdown` | `tests/`, `src/lib.rs` (doc) | Asserts `Closed` is **not** transient — the whole point of shutdown being terminal (`:126` doc) | 2 tests, 1 doc |
| `ring_batch` | `tests/` | A full ring on a batch publish is transient; a too-large batch is not | 2 |
| `ring_core` | `tests/` | Backend-produced `Full`/`Empty` classification | 1 |
| `ring_tls` | `tests/` | `fn the_refusal_is_transient_because_a_flush_clears_it()` | 1 |

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -r '\.is_transient(' ring_*/src | command grep -v '^ring_types/'
```

Live output:

```
ring_shutdown/src/lib.rs:  /// assert!( !RingError::Closed.is_transient() );
```

returns **one line, a `///` doc example** in `ring_shutdown` (`:128`). Filtering
doc comments leaves nothing.

**`ring_tls` is the interesting consumer and it does not call the function.** Its
test at `tls_test.rs:161` is named `the_refusal_is_transient_because_a_flush_clears_it`
— the predicate's rule restated in a function name, asserting the property by
construction rather than by calling the predicate that decides it. It is the only
place in the family where a bare `is_transient` appears outside a call:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -r 'is_transient' ring_*/tests | command grep -v '^ring_types/' \
  | command grep -v '\.is_transient('
```

Live output:

```
ring_tls/tests/tls_test.rs:fn the_refusal_is_transient_because_a_flush_clears_it()
```

returns that one line. **A rule copied into a name cannot drift with the rule.**
If `is_transient`'s membership changed, `ring_tls`'s test name would become a
lie and nothing would fail.

## Caller Tree

- *No caller within `ring_types`*
- *External production callers: none*
- *External test callers: 6 sites across `ring_shutdown`, `ring_batch`, `ring_core`, `ring_tls`*

## Callee Tree

- *(none)* — an exhaustive `match`, not a macro or a call, since
  `Fix(ring_error_classification_not_exhaustive)`

**Six test call sites against zero production ones, and a nine-arm body.** The
same shape as [`is_configuration`](004_ring_error_is_configuration.md) with a
smaller `true` set and half the tests, and the same fragility: nothing outside
this crate would fail to compile without it
(→ [`../../api/002`](../../api/002_the_five_classifier_predicates.md)).
