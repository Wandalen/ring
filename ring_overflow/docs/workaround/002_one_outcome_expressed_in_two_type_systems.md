# Workaround: One Outcome Expressed in Two Type Systems

### Scope

**Purpose:** Record the two strategies that exist for reading an outcome across the
`Result`/`Resolution` boundary — the suite's composition and the consumer's
avoidance — and what each costs.

**Responsibility:** `is_ok_and` as a bridge, `ring_core`'s route around the bridge,
and what neither recovers.

**In Scope:** `ring_overflow/src/lib.rs:204`, `:235`;
`ring_overflow/tests/overflow_test.rs:293-303`;
`ring_core/src/lib.rs:408-416`.

**Out of Scope:** Why the split exists is
[`decisions/002`](../decisions/002_fail_returns_an_error_not_a_resolution.md). Why
there are two functions at all is
[`workaround/001`](001_the_recorder_forecloses_const.md).

---

## Two Ways Across, One Way Around

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the one outcome, in two type systems --'
sed -n '/^    OverflowPolicy::Fail => Err( RingError::Full ),$/p;/^    OverflowPolicy::Fail => Resolution::Refused,$/p' ring_overflow/src/lib.rs
echo '  -- how the suite bridges them --'
sed -n '/^    let outcome = resolve( policy, &stats );$/p;/^      outcome\.is_err(), policy\.reports_failure(),$/p;/^      outcome\.is_ok_and( Resolution::lost_an_item ), policy\.drops_silently(),$/p' ring_overflow/tests/overflow_test.rs
echo '  -- and how the one consumer avoids the bridge entirely --'
command grep -m1 -A8 -F '    match refused' ring_core/src/lib.rs
```

Live output:

```
  -- the one outcome, in two type systems --
    OverflowPolicy::Fail => Err( RingError::Full ),
    OverflowPolicy::Fail => Resolution::Refused,
  -- how the suite bridges them --
    let outcome = resolve( policy, &stats );
      outcome.is_err(), policy.reports_failure(),
      outcome.is_ok_and( Resolution::lost_an_item ), policy.drops_silently(),
  -- and how the one consumer avoids the bridge entirely --
    match refused
    {
      Ok( () ) => Ok( () ),
      Err( record ) => match would_resolve( self.overflow )
      {
        Resolution::DroppedIncoming => Ok( () ),
        Resolution::EvictedOldest | Resolution::Refused => Err( record ),
      },
    }
```

---

### OV39 — The Suite Bridges With `is_ok_and`, Which Collapses Two Distinct Cases Into One `false`

`policy_self_description_agrees_with_the_handler` is the only code reading a
`resolve` outcome as a whole. It does so with two `Result` methods:
`outcome.is_err()` against `policy.reports_failure()`, and
`outcome.is_ok_and( Resolution::lost_an_item )` against `policy.drops_silently()`.

`is_ok_and` is the bridge: it reaches into the `Ok` side and applies a `Resolution`
predicate, yielding `false` for the `Err` side without ever naming it.

**Finding.** That is the only shape available, and it is lossy in a specific way.
`is_ok_and(...)` returns `false` for two different reasons — either the outcome was
`Ok` and the predicate said no, or the outcome was `Err` and the predicate never
ran. The assertion passes for `Fail` because `drops_silently()` is also `false`,
so both sides agree by arriving at the same answer from opposite directions.

The consequence is stated in
[`api/002`](../api/002_two_predicates_and_no_caller.md) § OV8:
`Resolution::Refused` is never constructed by the crate's most valuable test. What
belongs here is that this is not an oversight in the test — it is what the bridge
does. Any caller composing `Result` methods with `Resolution` predicates gets the
same collapse, and recovering the distinction requires abandoning the combinators
and matching both levels by hand.

---

### OV40 — The Consumer Sidesteps the Boundary by Not Crossing It

`ring_core` never holds a `Result< Resolution, RingError >`. It calls
`would_resolve`, which returns a bare `Resolution`, matches one variant by name
and the other two together in a second arm — mapping the result onto its own
`Result< (), T >` where `T` is the record the caller still holds.

So the crate's one production consumer solves the two-type-system problem by
choosing the half that has only one type system.

**Finding.** This is the cheapest available answer and it discards the accounting.
Taking `would_resolve` means no counter is written
([`integration/002`](../integration/002_the_stats_edge_and_the_function_nobody_imports.md)),
which is a real loss traded for a simpler match — and nothing in either crate
records that the trade was made or that it was a trade.

It also relocates the error decision. `resolve` would have handed back
`RingError::Full`, a family error naming the condition. `ring_core` instead returns
`Err( record )` — the payload itself, so the caller gets its item back rather than a
diagnosis. That is arguably better for this consumer and it means
`RingError::Full`, the error half of `resolve`'s whole signature, has no production
construction site anywhere in the workspace via this path.

Both halves of the pair are therefore reduced: `resolve` is unused, and the error
variant it exists to return is unreached.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`workaround/001`](001_the_recorder_forecloses_const.md) | Why two functions exist to choose between |
| [`decisions/002`](../decisions/002_fail_returns_an_error_not_a_resolution.md) | The split being worked around |
| [`api/002`](../api/002_two_predicates_and_no_caller.md) | The variant the bridge never constructs |
| [`algorithm/002`](../algorithm/002_where_the_third_outcome_goes.md) | The consumer's match, in full |

### Sources

| Fact | Where |
|------|-------|
| The two `Fail` arms | `ring_overflow/src/lib.rs:204`, `:235` |
| The `is_ok_and` bridge | `ring_overflow/tests/overflow_test.rs:301` |
| The `is_err` comparison beside it | `ring_overflow/tests/overflow_test.rs:296` |
| The consumer's match | `ring_core/src/lib.rs:408-416` |

### Tests

| Test | Covers |
|------|--------|
| `policy_self_description_agrees_with_the_handler` | The bridge itself, over all three policies |
| `fail_hands_the_decision_back_as_an_error` | Both spellings of the third outcome |
| `resolve_agrees_with_would_resolve` | The two arms where no bridge is needed |
| *(to create)* | Nothing distinguishes `is_ok_and`'s two routes to `false` |
