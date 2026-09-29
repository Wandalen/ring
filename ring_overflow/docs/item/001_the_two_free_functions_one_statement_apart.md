# Item: The Two Free Functions, One Statement Apart

### Scope

**Purpose:** Record `resolve` and `would_resolve` as individual items — their whole
bodies, and the exact textual difference between them.

**Responsibility:** Both function declarations, both bodies, and what the diff
between them shows about the pair's design.

**In Scope:** `ring_overflow/src/lib.rs:192-206`, `:229-237`.

**Out of Scope:** Why one is `const` and the other is not is
[`workaround/001`](../workaround/001_the_recorder_forecloses_const.md). The type
and its predicates are [`item/002`](002_the_enum_and_its_two_readings.md).

---

## Both Bodies, and the Difference

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the two free functions, whole --'
command grep -m1 -A9 -F 'pub fn resolve( policy : OverflowPolicy, stats : &RingStats ) -> Result< Resolution, RingError >' ring_overflow/src/lib.rs
echo '  ---'
command grep -m1 -A8 -F 'pub const fn would_resolve( policy : OverflowPolicy ) -> Resolution' ring_overflow/src/lib.rs
echo '  -- what separates them, line by line --'
diff <( command grep -m1 -A9 -F 'pub fn resolve( policy : OverflowPolicy, stats : &RingStats ) -> Result< Resolution, RingError >' ring_overflow/src/lib.rs | tail -n 9 ) <( command grep -m1 -A8 -F 'pub const fn would_resolve( policy : OverflowPolicy ) -> Resolution' ring_overflow/src/lib.rs | tail -n 8 ) || true
```

Live output:

```
  -- the two free functions, whole --
pub fn resolve( policy : OverflowPolicy, stats : &RingStats ) -> Result< Resolution, RingError >
{
  // Counts an event, not a loss: this runs on every policy, including `Fail`,
  // whose own share is retrievable separately via `stats.dropped(
  // OverflowPolicy::Fail )` — `Resolution::Refused.lost_an_item()` is `false`,
  // so this call and that predicate answer different questions about the
  // same arrival.
  stats.record_drop( policy, 1 );
  match policy
  {
  ---
pub const fn would_resolve( policy : OverflowPolicy ) -> Resolution
{
  match policy
  {
    OverflowPolicy::DropNewest => Resolution::DroppedIncoming,
    OverflowPolicy::DropOldest => Resolution::EvictedOldest,
    OverflowPolicy::Fail => Resolution::Refused,
  }
}
  -- what separates them, line by line --
2,7d1
<   // Counts an event, not a loss: this runs on every policy, including `Fail`,
<   // whose own share is retrievable separately via `stats.dropped(
<   // OverflowPolicy::Fail )` — `Resolution::Refused.lost_an_item()` is `false`,
<   // so this call and that predicate answer different questions about the
<   // same arrival.
<   stats.record_drop( policy, 1 );
9a4,8
>     OverflowPolicy::DropNewest => Resolution::DroppedIncoming,
>     OverflowPolicy::DropOldest => Resolution::EvictedOldest,
>     OverflowPolicy::Fail => Resolution::Refused,
>   }
> }
```

---

### OV25 — The Whole Difference Is One Added Statement and Three Rewrapped Arms

`resolve` is nine lines and `would_resolve` is eight. The `diff` reports exactly
two hunks: one deletion — `stats.record_drop( policy, 1 )` — and one substitution
covering all three match arms, where two gain an `Ok(...)` wrapper and the third
changes both its wrapper and its value.

Everything else is identical: same scrutinee, same arm order, same variant names,
same brace style.

**Finding.** The pair is therefore a textbook pure/effectful split with one
irregularity, and the `diff` isolates it precisely. Two of the three arms differ
only by `Ok(...)`, which is a mechanical wrapping. The third differs in substance:
`Err( RingError::Full )` against `Resolution::Refused`
([`decisions/002`](../decisions/002_fail_returns_an_error_not_a_resolution.md)).

So the phrase "the pure half of `resolve`" holds exactly to the extent that
wrapping in `Ok` is not a semantic change — true for two arms, false for the
third. A reader checking whether the two implementations agree can do it by
inspection for `DropNewest` and `DropOldest` and must reason about type systems for
`Fail`, which is also the only arm the agreement test skips.

---

### OV26 — Two Bodies Encoding One Mapping, With No Mechanism Keeping Them Equal

The mapping from policy to outcome is written twice, in adjacent functions, by
hand. Nothing derives one from the other: `resolve` does not call `would_resolve`
and wrap the result, and `would_resolve` does not call `resolve` with a throwaway
`RingStats`.

Either would have collapsed the duplication. The first is the obvious one — 
`resolve` could be `stats.record_drop( policy, 1 ); match would_resolve( policy )
{ Refused => Err( RingError::Full ), other => Ok( other ) }` — and it is not what
the crate does.

**Finding.** The duplication is deliberate and load-bearing in one direction:
delegating `would_resolve` to `resolve` is impossible, because `resolve` takes a
`&RingStats` and touches an atomic, which would cost `would_resolve` its `const`
([`workaround/001`](../workaround/001_the_recorder_forecloses_const.md)). The
reverse delegation has no such obstacle and is simply not done.

What keeps the two in step is `resolve_agrees_with_would_resolve`, which covers two
of the three arms. The third is covered separately by
`fail_hands_the_decision_back_as_an_error`, which asserts each half's `Fail`
behaviour independently rather than asserting a relationship between them — because
there is no relationship to assert once one side returns `Err`. So the mapping is
written twice, and the property that both copies say the same thing is verified for
two arms out of three and is not statable for the third.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`item/002`](002_the_enum_and_its_two_readings.md) | The type these two produce |
| [`algorithm/001`](../algorithm/001_two_mappings_over_three_policies.md) | The mapping both encode |
| [`workaround/001`](../workaround/001_the_recorder_forecloses_const.md) | Why one direction of delegation is closed |
| [`decisions/002`](../decisions/002_fail_returns_an_error_not_a_resolution.md) | The arm where the two genuinely differ |

### Sources

| Fact | Where |
|------|-------|
| `resolve`, whole | `ring_overflow/src/lib.rs:192-206` |
| `would_resolve`, whole | `ring_overflow/src/lib.rs:229-237` |
| The two-hunk diff | Census above |

### Tests

| Test | Covers |
|------|--------|
| `resolve_agrees_with_would_resolve` | Two of the three arms, as a relationship |
| `fail_hands_the_decision_back_as_an_error` | The third arm, as two independent facts |
| `would_resolve_touches_no_counters` | That the deleted statement is the only effect |
| `exactly_one_counter_moves_per_call` | That the added statement runs on every arm |
