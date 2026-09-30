# Decisions: The Fourth Variant That Is Not There

### Scope

**Purpose:** Record the choice to give `Resolution` exactly three variants, all of
them consequences of a ring already known to be full, and what that choice buys.

**Responsibility:** The precondition `resolve` is documented under, the absence of
any variant meaning *the publish succeeded normally*, and the `const`-ness that
absence makes possible.

**In Scope:** `ring_overflow/src/lib.rs:57-76`, `:148-149`, `:137-145`, `:229`.

**Out of Scope:** The third variant's two representations are
[`decisions/002`](002_fail_returns_an_error_not_a_resolution.md). The layout of
the enum is
[`data_structure/001`](../data_structure/001_one_byte_and_two_hundred_fifty_three_spare_niches.md).

---

## What the Type Is Defined Over

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the precondition the whole type is defined under --'
command grep -m1 -A1 -F '/// Apply `policy` to a publish that found the ring full, recording the outcome' ring_overflow/src/lib.rs
echo '  -- what a resolution is a function of --'
command grep -m1 -F 'pub const fn would_resolve( policy : OverflowPolicy ) -> Resolution' ring_overflow/src/lib.rs
echo '  -- the three variants, none of which names acceptance --'
command grep '^  DroppedIncoming,\|^  EvictedOldest,\|^  Refused,' ring_overflow/src/lib.rs
echo '  -- the only notion of acceptance, a reading over the three --'
command grep -m1 -A6 -F '  /// assert!( !Resolution::Refused.accepted_incoming() );' ring_overflow/src/lib.rs | tail -n 5
```

Live output:

```
  -- the precondition the whole type is defined under --
/// Apply `policy` to a publish that found the ring full, recording the outcome
/// in `stats`.
  -- what a resolution is a function of --
pub const fn would_resolve( policy : OverflowPolicy ) -> Resolution
  -- the three variants, none of which names acceptance --
  DroppedIncoming,
  EvictedOldest,
  Refused,
  -- the only notion of acceptance, a reading over the three --
  /// Exhaustive for the reason given on [`Resolution::lost_an_item`].
  #[ must_use ]
  pub const fn accepted_incoming( self ) -> bool
  {
    match self
```

---

### OV13 — The Type Models Only What Happens After the Ring Is Already Full

`Resolution` has no variant meaning *there was room and the item went in*. All
three of its variants are consequences of a publish that already failed to find a
slot, and `resolve`'s first documentation line states that precondition
explicitly: "Apply `policy` to a publish that found the ring full."

The happy path is therefore outside this type entirely. A caller that publishes
successfully never constructs a `Resolution`, never calls either function, and
never reaches this crate.

**Finding.** That is what makes `would_resolve` a `const fn` of one argument.
Because the type is scoped to a state the caller has already established, a
resolution depends on the policy and on nothing else — no capacity, no cursor, no
occupancy, no clock. A fourth variant for the non-full case would have made the
outcome a function of ring state, which would have cost the `const`, the purity,
and the injectivity the suite rests on
([`invariant/001`](../invariant/001_the_mapping_is_total_and_injective.md)).

The decision is sound and unrecorded as a decision. It reads in the source as a
one-line precondition on one function's doc comment, not as the reason the type
has the shape it has — so the argument for keeping the happy path out is
available to anyone who reads `resolve`'s first sentence carefully, and to nobody
else.

---

### OV14 — Acceptance Exists, as a Reading Over Three Variants Rather Than a Fourth

The crate does have a notion of the incoming item being accepted:
`accepted_incoming()` is true for exactly `EvictedOldest`, the one outcome where
the publish that triggered the overflow ultimately went in — at the cost of the
oldest unread item.

So "accepted" is modelled as a predicate over the three-variant partition, not as
a variant beside them. This is the right shape for the same reason above: the
acceptance in question is acceptance *given* a full ring, which is a property of
which policy ran, not a separate outcome.

**Finding.** The consequence is that the crate's word "accepted" means something
narrower than a caller will assume from the name. `accepted_incoming()` returns
false for a publish that succeeded normally — because such a publish never
produces a `Resolution` at all — so the predicate is not "did the item get in"
but "did the item get in *by evicting something*."

The doc comment on `accepted_incoming` does not draw that distinction, and the
name does not carry it. The one variant it is true for is also the one no
production build can reach
([`pitfall/001`](../pitfall/001_the_variant_a_default_build_cannot_reach.md)), so
the predicate is, in every shipping configuration, a function that returns false.

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
command grep -A3 -F 'Narrower than the name suggests' ring_overflow/src/lib.rs
```

Live output:

```
    /// **Narrower than the name suggests.** A publish that finds room and
    /// succeeds normally never constructs a `Resolution` at all, so this
    /// predicate never sees that case — what it actually answers is "did the
    /// item get in *by evicting something*", not "did the item get in".
```

**Disposition:** applied — `accepted_incoming`'s doc comment in
`ring_overflow/src/lib.rs` now states that a normal successful publish
never constructs a `Resolution` at all, so the predicate answers "did the
item get in *by evicting something*", not "did the item get in" — the
distinction this instance found missing. Now prints: `**Narrower than the
name suggests.**`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`decisions/002`](002_fail_returns_an_error_not_a_resolution.md) | The third variant, and where it goes instead |
| [`invariant/001`](../invariant/001_the_mapping_is_total_and_injective.md) | The property this scoping makes provable |
| [`pitfall/001`](../pitfall/001_the_variant_a_default_build_cannot_reach.md) | Why `accepted_incoming` is always false in production |
| [`api/002`](../api/002_two_predicates_and_no_caller.md) | Where the predicate is called |

### Sources

| Fact | Where |
|------|-------|
| The full-ring precondition | `ring_overflow/src/lib.rs:148-149` |
| The three variants | `ring_overflow/src/lib.rs:59`, `:73`, `:75` |
| `accepted_incoming`'s body | `ring_overflow/src/lib.rs:137-145` |
| `would_resolve`'s single argument, `const` | `ring_overflow/src/lib.rs:229` |

### Tests

| Test | Covers |
|------|--------|
| `resolution_has_exactly_three_variants_and_no_overwrite` | The variant count, structurally |
| `the_two_readings_partition_the_outcomes` | That acceptance is a reading, not a variant |
| `would_resolve_touches_no_counters` | The purity the scoping buys |
| *(to create)* | Nothing asserts the precondition — a caller may call `resolve` on a ring with room and be counted a drop |
