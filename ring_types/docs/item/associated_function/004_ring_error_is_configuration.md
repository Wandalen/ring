# RingError::is_configuration

## Representation

Answers whether the failure is a mistake in how the ring was set up — true for
four of nine variants. **Zero production callers in the family.**

The membership set and the three-variant gap it leaves are argued in
[`../../algorithm/002`](../../algorithm/002_classifying_an_error_into_configuration_or_traffic.md);
this file records the item, its citations, and who touches it.

What is worth stating here is what the item *is* rather than what it decides: a
`const fn` wrapping an exhaustive `match` over all nine variants, taking `self`
by value, `#[ must_use ]`, declared in the inherent `impl RingError` block that
also holds [`is_transient`](005_ring_error_is_transient.md) and nothing else.
The block is two predicates over one enum — no constructor, no accessor, no
conversion
(→ [`../implementation/002_impl_ring_error.md`](../implementation/002_impl_ring_error.md)).

Until `Fix(ring_error_classification_not_exhaustive)`
(`ring_types/src/error.rs:98`) this was a `matches!` naming only the four
configuration variants — a positive list that answered every variant it was
not told about, current or future, with the same `false`. The nine-variant
classification below is unchanged; a tenth `RingError` variant now fails to
compile here instead of silently reading as traffic.

## Kind

Associated Function/Method (§ Item Kind Taxonomy : Associated Item Kinds #1)

## Definition

`ring_types/src/error.rs:111`

```rust
#[ must_use ]
pub const fn is_configuration( self ) -> bool
```

Body spans lines 112-125:

```rust
match self
{
  Self::CapacityZero
  | Self::CapacityNotPowerOfTwo( _ )
  | Self::BatchTooLarge { .. }
  | Self::PolicyUnsupported => true,
  Self::Full
  | Self::Empty
  | Self::Closed
  | Self::NameTaken
  | Self::NameUnknown => false,
}
```

**The two payload-carrying variants are matched with wildcards** — `( _ )` and
`{ .. }` — so the classification never inspects a payload. That is what keeps the
function `const` and total, and it means adding a field to `BatchTooLarge` cannot
change this answer. Every other variant is now named explicitly rather than
falling through a wildcard arm, which is the property `Fix(ring_error_classification_not_exhaustive)`
added: the five variants classified `false` are listed by name, not implied by
"everything else".

## File Usage

| File | Line(s) | Context |
|------|---------|---------|
| `ring_types/src/error.rs` | 23, 88-90, 94-96, 110-111, 113-124 | Doc example on the enum itself asserting `!e.is_configuration()` for `Full` (23); the doc rationale — "the two call for opposite responses" (88-90); doc example asserting two true cases and one false (94-96); `#[ must_use ]` and **the definition (110-111)**; the exhaustive `match` set (113-124) |

Test-only references: `ring_types` — 3 in `tests/types_test.rs`, in
`errors_split_configuration_from_traffic`, which asserts all nine memberships.
Plus 11 across 7 consumer suites, tabulated below.

## Crate Usage

| Crate | Via File | Purpose | Sites |
|-------|----------|---------|-------|
| `ring_types` | `src/error.rs` | Defining crate | — |
| `ring_gating` | `tests/`, `src/lib.rs` (doc) | Asserts `BatchTooLarge` is never worth retrying; module doc names the predicate as "the caller's test" (`:38`) | 3 tests, 3 doc |
| `ring_claim` | `tests/` | Batch-bound rejection is a configuration error | 2 |
| `ring_wait` | `tests/` | Confirms a wait-path failure is *not* configuration | 2 |
| `ring_batch` | `tests/` | Batch-bound rejection | 1 |
| `ring_core` | `tests/` | `PolicyUnsupported` from the crossbeam backend | 1 |
| `ring_event` | `tests/` | Same distinction on the event path | 1 |
| `ring_publish` | `tests/` | Publish-path failure classification | 1 |

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -r '\.is_configuration(' ring_*/src | command grep -v '^ring_types/'
```

Live output:

```
ring_gating/src/lib.rs:    /// assert!(too_wide.is_configuration(), "never retry this one");
ring_gating/src/lib.rs:    /// assert!(!RingError::Full.is_configuration(), "but do retry this one");
```

returns **two lines, both `///` doc examples** in `ring_gating` (`:278`, `:281`).
Filtering doc comments out leaves nothing:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -rn '\.is_configuration(' ring_*/src \
  | command grep -v '^ring_types/' | command grep -v ':[0-9]*: *//' || true
```

Live output:

```
```

is empty, exit 1.

**Eleven test call sites across seven crates, zero production call sites across
thirty-two.** The predicate is a testing vocabulary, not a runtime one — every
crate that handles a `RingError` in anger matches the variant directly. The
argument for keeping it anyway is the export Contract: it is public surface for a
consumer outside the family, and the family is not that consumer
(→ [`../../api/002`](../../api/002_the_five_classifier_predicates.md)).

## Caller Tree

- *No caller within `ring_types`* — the crate never classifies an error it produces; `Capacity::new` returns the variant and stops
- *External production callers: none*
- *External test callers: 11 sites across `ring_gating`, `ring_claim`, `ring_wait`, `ring_batch`, `ring_core`, `ring_event`, `ring_publish`*

## Callee Tree

- *(none)* — an exhaustive `match`, not a macro or a call, since
  `Fix(ring_error_classification_not_exhaustive)`; no function is called

**Nothing to call and nothing that calls it.** Both trees are empty of production
edges, which for a public predicate is the most fragile state an item can be in:
no compile error anywhere outside this crate would follow from deleting it. Eleven
consumer test assertions would fail, which is a real defence and a weaker one than
a caller — a test can be deleted alongside the thing it tests without anything
looking wrong. What protects it beyond that is the export Contract, and a contract
is not a compiler (→ [`../../api/002`](../../api/002_the_five_classifier_predicates.md),
which measures the same absence across all five predicates).
