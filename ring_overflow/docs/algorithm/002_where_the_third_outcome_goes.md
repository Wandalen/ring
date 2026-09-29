# Algorithm: Where the Third Outcome Goes

### Scope

**Purpose:** Follow a `Resolution` from `would_resolve` to the one production
site that consumes it, and record what that site does with a three-variant type.

**Responsibility:** The single call site in `ring_core::Producer::try_push`, the
second arm it collapses two variants into, and which variants can actually arrive.

**In Scope:** `ring_core/src/lib.rs:80`, `:165`, `:408-416`;
`ring_core/Cargo.toml:9`.

**Out of Scope:** That the two predicates are never called on the way is
[`api/002`](../api/002_two_predicates_and_no_caller.md). Which variants a default
build can produce at all is
[`pitfall/001`](../pitfall/001_the_variant_a_default_build_cannot_reach.md).

---

## One Import, One Call Site, One Named Variant

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- every production use of this crate, outside its own source --'
command grep -r 'use ring_overflow' --include=*.rs */src/ | command grep -v '^ring_overflow/' | sed 's|ring/||'
echo '  -- the one call site, and what it does with three variants --'
command grep -m1 -A8 -F '    match refused' ring_core/src/lib.rs
echo '  -- the policy that would produce the third variant --'
command grep -m1 -F '//! | `OverflowPolicy::DropOldest` | **rejected at construction** | rejected | supported |' ring_core/src/lib.rs
echo '  -- and whether the path that accepts it is compiled by default --'
command grep '^default = ' ring_core/Cargo.toml
echo '  -- what the variant declaration now says about that --'
command grep -m1 -A6 -F '  /// **No shipping configuration produces this.**' ring_overflow/src/lib.rs
```

Live output:

```
  -- every production use of this crate, outside its own source --
ring_core/src/lib.rs:use ring_overflow::{ would_resolve, Resolution };
  -- the one call site, and what it does with three variants --
    match refused
    {
      Ok( () ) => Ok( () ),
      Err( record ) => match would_resolve( self.overflow )
      {
        Resolution::DroppedIncoming => Ok( () ),
        Resolution::EvictedOldest | Resolution::Refused => Err( record ),
      },
    }
  -- the policy that would produce the third variant --
//! | `OverflowPolicy::DropOldest` | **rejected at construction** | rejected | supported |
  -- and whether the path that accepts it is compiled by default --
default = []
  -- what the variant declaration now says about that --
  /// **No shipping configuration produces this.** It requires
  /// [`OverflowPolicy::DropOldest`], which the default build rejects at
  /// construction with `RingError::PolicyUnsupported`, and which the `crossbeam`
  /// build handles with `force_push` and an early return before reaching the
  /// resolution site. So one of these three exported variants is unconstructible
  /// in every configuration that ships — `ring_core:337` records that for its
  /// own `match`, and until now this crate, which owns the type, recorded it
```

---

### OV3 — The Only Consumer Converts the Named Outcome Straight Back to a Boolean

The module comment states the type's purpose plainly: `Resolution` "is the type
that makes the alternative outcomes explicit rather than leaving them to a
boolean." One crate consumes it, at one site, and that site is a two-arm `match`
whose arms are `Ok( () )` and `Err( record )`.

`DroppedIncoming` gets one arm; `EvictedOldest` and `Refused` share the other,
which means the call site cannot distinguish "the ring made room and took your
item" from "the ring refused and here it is back" — and it does not need to,
because the record it returns carries that distinction instead.

**Finding.** The conversion is correct for `try_push`'s signature,
`Result< (), T >`, and it is exactly the boolean the type was introduced to
avoid. What survives from three named outcomes to the caller of `ring_core` is
one bit plus the record. The two predicates that exist to read a `Resolution`
without matching on it — `lost_an_item` and `accepted_incoming` — are not used
here, and a fourth variant now fails at this site alongside the two `match`
blocks inside `ring_overflow`
([`algorithm/001`](001_two_mappings_over_three_policies.md) § OV1).

That last clause said the opposite until `ring_core`'s CO1 was applied: the
second arm read `_ =>`, so a fourth `Resolution` variant compiled silently here
and broke loudly two crates away. The reduction to one bit is unchanged and is
still the finding; what changed is that the reduction is now spelled out by
name, so the enum's third variant cannot be added without this site being
reconsidered.

None of that is a defect in `ring_core`, which needs one bit and has a
signature that carries it. It is a fact about what the crate's central design
argument buys at the only place it is spent.

---

### OV4 — Two of the Three Arms Cannot Be Taken in a Default Build

`ring_core`'s own header table records the position: `OverflowPolicy::DropOldest`
is **rejected at construction** on the default path, and `Ring::new` returns
`RingError::PolicyUnsupported` for it at `:154`. The one constructor that accepts
it, `new_crossbeam`, is behind `#[ cfg( feature = "crossbeam" ) ]`, and
`ring_core`'s manifest sets `default = []`.

So in a default build there is no way to configure `DropOldest`, and therefore no
way for `would_resolve` to return `EvictedOldest`. With the feature on, the
crossbeam arm handles `DropOldest` with `force_push` and returns before reaching
`would_resolve` at all.

**Finding.** Across both build configurations, `would_resolve` is only ever
called with `DropNewest` or `Fail`. Its `DropOldest` arm — and with it the whole
of `Resolution::EvictedOldest`, the single variant for which
`accepted_incoming()` is true — has no production caller in either. The mapping
is total over the policy enum, and the consumer's domain is a two-element subset
of it.

That is a reasonable state for a Tier 1 crate whose consumer is still being
built, and nothing anywhere recorded it. `ring_core` documented that `DropOldest`
cannot arrive at its own `match`; `ring_overflow` documented `EvictedOldest` as
one of three equals.

The declaration now says which configuration produces the variant and what each
build does instead, so the asymmetry is visible from the type rather than only
from the consumer two crates away.

**Disposition:** applied — the doc comment at `Resolution::EvictedOldest` in
`ring_overflow/src/lib.rs`, recorded in full at
[`pitfall/001`](../pitfall/001_the_variant_a_default_build_cannot_reach.md)
§ OV29. This instance's own subject — that `would_resolve`'s domain in production
is a two-element subset of the policy enum — is unchanged and remains true; what
changed is that it is written down where a reader of the type will find it.
Documentation only, no behaviour altered. Now prints: `  /// **No shipping configuration produces this.**`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`algorithm/001`](001_two_mappings_over_three_policies.md) | The mapping this site consumes one third of |
| [`pitfall/001`](../pitfall/001_the_variant_a_default_build_cannot_reach.md) | The same reachability fact, as a hazard rather than a trace |
| [`integration/001`](../integration/001_one_consumer_one_import_one_site.md) | The dependency edge measured |
| [`api/002`](../api/002_two_predicates_and_no_caller.md) | The two readings this site does not use |

### Sources

| Fact | Where |
|------|-------|
| The only production import | `ring_core/src/lib.rs:80` |
| The only call site | `ring_core/src/lib.rs:411-415` |
| `DropOldest` rejected at construction | `ring_core/src/lib.rs:44`, `:165` |
| `new_crossbeam` behind a cfg | `ring_core/src/lib.rs:195-196` |
| The feature is off by default | `ring_core/Cargo.toml:9` |
| The crossbeam arm returning early | `ring_core/src/lib.rs:396-404` |
| `Resolution` justified against a boolean | `ring_overflow/src/lib.rs:15-16` |

### Tests

| Test | Covers |
|------|--------|
| `each_policy_maps_to_its_own_resolution` | All three arms, in this crate's own suite |
| `the_two_readings_partition_the_outcomes` | The predicates the call site does not use |
| *(to create)* | Nothing asserts which policies a consumer can actually configure |
