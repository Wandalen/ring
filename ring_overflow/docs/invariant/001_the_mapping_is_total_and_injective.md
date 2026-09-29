# Invariant: The Mapping Is Total and Injective

### Scope

**Purpose:** Record the crate's central invariant — every policy has exactly one
resolution and no two policies share one — and how much of it the suite actually
pins.

**Responsibility:** Totality, injectivity, the set the tests iterate, and the
asymmetry between the two enums' published constants.

**In Scope:** `ring_overflow/tests/overflow_test.rs:21-43`;
`ring_types/src/policy.rs:131`.

**Out of Scope:** The two mappings that must both satisfy it are
[`algorithm/001`](../algorithm/001_two_mappings_over_three_policies.md). The
missing inverse function is [`api/001`](../api/001_four_declarations_three_of_them_const.md) § OV6.

---

## Both Halves of the Property

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- totality: every policy has an image --'
command grep -m1 -A2 -F '  assert_eq!( would_resolve( OverflowPolicy::DropNewest ), Resolution::DroppedIncoming );' ring_overflow/tests/overflow_test.rs
echo '  -- injectivity: no two collide, over the published set --'
command grep -m1 -B3 -A3 -F '    assert!( !seen.contains( &resolution ), "{policy:?} collided with an earlier policy" );' ring_overflow/tests/overflow_test.rs
echo '  -- the set it iterates --'
command grep 'ALL : \[ Self; 3 \]' ring_types/src/policy.rs
echo '  -- how the target guards its own count: an exhaustive match in a test --'
command grep -m1 -A5 -F '    match resolution' ring_overflow/tests/overflow_test.rs
echo '  -- and how the two readings classify one --'
command grep -m1 -A7 -F '  pub const fn lost_an_item( self ) -> bool' ring_overflow/src/lib.rs
command grep -m1 -A7 -F '  pub const fn accepted_incoming( self ) -> bool' ring_overflow/src/lib.rs
```

Live output:

```
  -- totality: every policy has an image --
  assert_eq!( would_resolve( OverflowPolicy::DropNewest ), Resolution::DroppedIncoming );
  assert_eq!( would_resolve( OverflowPolicy::DropOldest ), Resolution::EvictedOldest );
  assert_eq!( would_resolve( OverflowPolicy::Fail ), Resolution::Refused );
  -- injectivity: no two collide, over the published set --
  for policy in OverflowPolicy::ALL
  {
    let resolution = would_resolve( policy );
    assert!( !seen.contains( &resolution ), "{policy:?} collided with an earlier policy" );
    seen.push( resolution );
  }
  assert_eq!( seen.len(), 3 );
  -- the set it iterates --
  pub const ALL : [ Self; 3 ] = [ Self::DropNewest, Self::DropOldest, Self::Fail ];
  -- how the target guards its own count: an exhaustive match in a test --
    match resolution
    {
      Resolution::DroppedIncoming => "dropped_incoming",
      Resolution::EvictedOldest => "evicted_oldest",
      Resolution::Refused => "refused",
    }
  -- and how the two readings classify one --
  pub const fn lost_an_item( self ) -> bool
  {
    match self
    {
      Self::DroppedIncoming | Self::EvictedOldest => true,
      Self::Refused => false,
    }
  }
  pub const fn accepted_incoming( self ) -> bool
  {
    match self
    {
      Self::EvictedOldest => true,
      Self::DroppedIncoming | Self::Refused => false,
    }
  }
```

---

### OV21 — The Invariant Is a Bijection, and Only One Direction Is Named

`would_resolve` is total: `each_policy_maps_to_its_own_resolution` gives every
policy an image, and the compiler enforces it — the `match` has no `_` arm, so a
fourth policy is a compile error rather than a missing case.

It is also injective: `distinct_policies_give_distinct_resolutions` iterates
`OverflowPolicy::ALL` and asserts no resolution repeats. Three policies, three
distinct resolutions, three variants in the target — so the mapping is a bijection
between the two enums.

**Finding.** The suite establishes a bijection and the crate exposes only one
direction of it. The test's own doc comment says why the property matters — "the
resolution alone identifies what happened" — which is exactly the statement that a
resolution determines its policy, and there is no function that performs that
determination
([`api/001`](../api/001_four_declarations_three_of_them_const.md) § OV6).

The invariant is therefore proved and half-spent. A caller holding a resolution has
a value that mathematically identifies the configuration and no way to recover it,
in a crate that is 237 lines and could add the inverse in four.

---

### OV22 — Both Enums' Variant Counts Are Guarded; Neither Reading Is

Each enum's size is pinned, by a different mechanism. `OverflowPolicy` publishes
`ALL : [ Self; 3 ]`, which the injectivity test iterates and whose length
`ring_types` asserts in its own suite. `Resolution` publishes nothing, but the
`name()` helper inside
`resolution_has_exactly_three_variants_and_no_overwrite` matches it exhaustively —
and since `Resolution` is not `#[ non_exhaustive ]`, a fourth variant makes that
match non-exhaustive and the test file stops compiling. The test's own doc comment
says so: "exhaustively matching three named variants compiles, which it could not
if a fourth existed."

Both tripwires work. What neither covers is the two readings.

**Finding.** `lost_an_item` and `accepted_incoming` were `matches!` over positive
patterns — `Self::DroppedIncoming | Self::EvictedOldest` and `Self::EvictedOldest`.
Any variant not named fell through to `false`. So a fourth resolution added then
produced a compile error in `name()`, and once whoever added it updated that
helper — a one-line edit, obviously correct-looking — the new variant silently
acquired `lost_an_item() == false` and `accepted_incoming() == false`, which is
exactly `Refused`'s profile.

The compile error therefore pointed at the one place that did not matter and
stayed silent about the two that did. Nothing forced a decision about how a new
outcome should read; the default was assigned by pattern-matching omission rather
than by anyone choosing it. An `Overwrite` variant — the case the test is
explicitly named against — would have been classified as losing nothing and
accepting nothing, which is the precise inverse of what it means.

Both readings are now exhaustive `match` expressions, so the omission is no longer
expressible: every variant appears on the left of an arm in both, and a fourth one
stops the crate compiling in three places instead of one test helper. The
tripwire and the readings now fail together, which is what makes the tripwire mean
what its name says.

**Disposition:** applied — `lost_an_item` and `accepted_incoming` in
`ring_overflow/src/lib.rs` rewritten from `matches!` to exhaustive `match`,
with the reason recorded on the first of the pair. `Resolution::ALL` was added in
the same pass and pinned by
`resolution_has_exactly_three_variants_and_no_overwrite`, closing the other half
of this finding: the count is now published as well as matched, so an `ALL` that
drifted from the enum fails rather than handing every iterating test a short list.
The census above shows both bodies. Now prints: `      Self::EvictedOldest => true,`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`algorithm/001`](../algorithm/001_two_mappings_over_three_policies.md) | The two mappings this constrains |
| [`invariant/002`](002_no_resolution_overwrites_unread_data_silently.md) | The safety property layered on top |
| [`api/001`](../api/001_four_declarations_three_of_them_const.md) | The inverse the bijection would justify |
| [`data_structure/001`](../data_structure/001_one_byte_and_two_hundred_fifty_three_spare_niches.md) | The two enums' matching shapes |

### Sources

| Fact | Where |
|------|-------|
| The totality test | `ring_overflow/tests/overflow_test.rs:21-28` |
| The injectivity test | `ring_overflow/tests/overflow_test.rs:30-43` |
| `OverflowPolicy::ALL` | `ring_types/src/policy.rs:131` |
| The exhaustive match guarding `Resolution`'s count | `ring_overflow/tests/overflow_test.rs:60-65` |
| Both predicates' exhaustive matches | Census above |

### Tests

| Test | Covers |
|------|--------|
| `each_policy_maps_to_its_own_resolution` | Totality, on three literals |
| `distinct_policies_give_distinct_resolutions` | Injectivity, over the published set |
| `resolution_has_exactly_three_variants_and_no_overwrite` | The target's variant count, structurally |
| `the_two_readings_partition_the_outcomes` | Every variant's answer at both readings, now that an exhaustive `match` forces each one to be chosen |
