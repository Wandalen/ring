# Algorithm: Two Mappings Over Three Policies, Differing by One Line

### Scope

**Purpose:** Record the whole of the crate's computation — two `match` blocks over
the same three-variant domain — and the single statement that is the entire
difference between them.

**Responsibility:** `resolve`'s and `would_resolve`'s bodies, the arm-by-arm
correspondence between them, and where the two disagree.

**In Scope:** `ring_overflow/src/lib.rs:192-206`, `:229-237`.

**Out of Scope:** Why `Fail` produces an `Err` rather than an `Ok` is
[`decisions/002`](../decisions/002_fail_returns_an_error_not_a_resolution.md).
The counter that `:199` moves is
[`integration/002`](../integration/002_the_stats_edge_and_the_function_nobody_imports.md).

---

## The Whole Computation

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
# the third alternative is `pub const` and not `pub const fn` on purpose:
# `pub const ALL` is a declaration too, and the narrower form cannot see it
echo '  -- the whole public surface --'
command grep 'pub enum\|pub fn\|pub const' ring_overflow/src/lib.rs
echo '  -- the two mappings, side by side --'
command grep -m1 -B2 -A3 -F '    OverflowPolicy::DropNewest => Ok( Resolution::DroppedIncoming ),' ring_overflow/src/lib.rs
command grep -m1 -A7 -F 'pub const fn would_resolve( policy : OverflowPolicy ) -> Resolution' ring_overflow/src/lib.rs | tail -n 6
echo '  -- and the one line that separates them --'
command grep 'record_drop' ring_overflow/src/lib.rs
```

Live output:

```
  -- the whole public surface --
pub enum Resolution
  pub const ALL : [ Self; 3 ] = [ Self::DroppedIncoming, Self::EvictedOldest, Self::Refused ];
  pub const fn lost_an_item( self ) -> bool
  pub const fn accepted_incoming( self ) -> bool
pub fn resolve( policy : OverflowPolicy, stats : &RingStats ) -> Result< Resolution, RingError >
pub const fn would_resolve( policy : OverflowPolicy ) -> Resolution
  -- the two mappings, side by side --
  match policy
  {
    OverflowPolicy::DropNewest => Ok( Resolution::DroppedIncoming ),
    OverflowPolicy::DropOldest => Ok( Resolution::EvictedOldest ),
    OverflowPolicy::Fail => Err( RingError::Full ),
  }
  match policy
  {
    OverflowPolicy::DropNewest => Resolution::DroppedIncoming,
    OverflowPolicy::DropOldest => Resolution::EvictedOldest,
    OverflowPolicy::Fail => Resolution::Refused,
  }
  -- and the one line that separates them --
  stats.record_drop( policy, 1 );
```

---

### OV1 — The Crate Is Two Exhaustive Matches and One Statement

There is no loop anywhere in `ring_overflow`, no arithmetic, no comparison
beyond pattern matching itself, and no value read back after being written. The
whole of the production computation is two `match` blocks over `OverflowPolicy`,
each with three arms, each arm a constant.

Both are exhaustive by construction rather than by convention: neither has a `_`
arm, so a fourth `OverflowPolicy` variant is two compile errors naming both
sites. That is the same enforcement `ring_stats` gets for its three drop
counters and does not get for its other four fields
([`ring_stats` § ST51](../../../ring_stats/docs/workaround/002_seven_counters_enumerated_four_times_by_hand.md)).

**Finding.** `resolve` is `would_resolve` with one statement in front of it and a
different return type behind it. Line `:199` — `stats.record_drop( policy, 1 )` —
is the entire behavioural difference, and it executes before the `match`, so it
runs on every policy including the one that goes on to return an error.

That ordering is deliberate and stated: "Exactly one counter is incremented per
call, whichever branch is taken — so a stats read accounts for every full-ring
event, not only the lossy ones." The crate is one branch of computation and one
of accounting, and the accounting is unconditional.

---

### OV2 — The Two Mappings Agree on Two Arms and Diverge on the Third

Arm for arm, the two bodies are the same function. `DropNewest` gives
`DroppedIncoming` in both; `DropOldest` gives `EvictedOldest` in both. The third
arm is where they part: `would_resolve` returns `Resolution::Refused`, and
`resolve` returns `Err( RingError::Full )`.

So `resolve`'s success type is `Resolution`, but only two of that type's three
variants can appear in it. `Resolution::Refused` is reachable through
`would_resolve` and unreachable through `resolve`, in a crate where `resolve` is
the function that does the work.

**Finding.** The suite tests the agreement on exactly the two arms where it
holds — `resolve_agrees_with_would_resolve` asserts `DropNewest` and `DropOldest`
and stops — and tests the divergence separately in
`fail_hands_the_decision_back_as_an_error`. The two tests together are complete
over the domain, and nothing states that they are two halves of one property, so
the pure form is described as "a faithful preview" of a function it agrees with
on two thirds of its input.

The gap is closable in the assertion rather than in the code: for every policy,
`resolve( p, &stats ).unwrap_or( Resolution::Refused ) == would_resolve( p )`
holds across all three, which is the property "the pure form previews the real
one" actually means here. The suite instead asserts it twice, over subsets, in
two tests whose names do not say they partition anything.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`algorithm/002`](002_where_the_third_outcome_goes.md) | What the `Result` does to the third arm at a call site |
| [`decisions/002`](../decisions/002_fail_returns_an_error_not_a_resolution.md) | Why the third arm diverges at all |
| [`item/001`](../item/001_the_two_free_functions_one_statement_apart.md) | The two functions as declarations, with their attributes |
| [`invariant/001`](../invariant/001_the_mapping_is_total_and_injective.md) | The properties both mappings hold |

### Sources

| Fact | Where |
|------|-------|
| The whole public surface | `ring_overflow/src/lib.rs:56`, `:94`, `:111`, `:138`, `:192`, `:229` |
| `resolve`'s three arms | `ring_overflow/src/lib.rs:200-205` |
| `would_resolve`'s three arms | `ring_overflow/src/lib.rs:231-236` |
| The one statement between them | `ring_overflow/src/lib.rs:199` |
| The unconditional-accounting rationale | `ring_overflow/src/lib.rs:152-154` |

### Tests

| Test | Covers |
|------|--------|
| `each_policy_maps_to_its_own_resolution` | `would_resolve` over all three policies |
| `resolve_agrees_with_would_resolve` | The two arms where the mappings agree |
| `fail_hands_the_decision_back_as_an_error` | The third arm, where they diverge |
| `exactly_one_counter_moves_per_call` | That `:199` runs on every policy |
