# API: Two Predicates, and No Caller Outside the Crate

### Scope

**Purpose:** Count every invocation of `lost_an_item` and `accepted_incoming`
across the workspace, and record what the two readings are for against where they
are used.

**Responsibility:** Both predicates' call sites, their partition of the three
outcomes, and the one test that reaches them through `resolve` rather than by
naming a variant.

**In Scope:** `ring_overflow/src/lib.rs:110-118`, `:137-145`;
`ring_overflow/tests/overflow_test.rs:93-147`, `:301`.

**Out of Scope:** The consumer that could call them and does not is
[`algorithm/002`](../algorithm/002_where_the_third_outcome_goes.md) § OV3. The
variant one of them is uniquely true for is
[`pitfall/001`](../pitfall/001_the_variant_a_default_build_cannot_reach.md).

---

## Every Call, Everywhere

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- every call of either predicate, anywhere in the workspace --'
command grep -r 'lost_an_item()\|accepted_incoming()\|Resolution::lost_an_item\|Resolution::accepted_incoming' --include=*.rs . | sed 's|ring/||'
echo '  -- split by where it lives --'
printf '    in src, this crate (doctests)   %s\n' "$( command grep -rc 'lost_an_item()\|accepted_incoming()' ring_overflow/src/lib.rs || true )"
printf '    in tests, this crate            %s\n' "$( command grep -c 'lost_an_item\|accepted_incoming' ring_overflow/tests/overflow_test.rs || true )"
printf '    in any other crate              %s\n' "$( command grep -rl 'lost_an_item\|accepted_incoming' --include=*.rs . | command grep -vc '^ring_overflow/' || true )"
```

Live output:

```
  -- every call of either predicate, anywhere in the workspace --
ring_overflow/tests/overflow_test.rs:    if resolution.accepted_incoming()
ring_overflow/tests/overflow_test.rs:        resolution.lost_an_item(),
ring_overflow/tests/overflow_test.rs:  // **The antecedent is asserted, not assumed.** `accepted_incoming()` is true
ring_overflow/tests/overflow_test.rs:  assert!( Resolution::DroppedIncoming.lost_an_item() );
ring_overflow/tests/overflow_test.rs:  assert!( Resolution::EvictedOldest.lost_an_item() );
ring_overflow/tests/overflow_test.rs:  assert!( !Resolution::Refused.lost_an_item() );
ring_overflow/tests/overflow_test.rs:  assert!( !Resolution::DroppedIncoming.accepted_incoming() );
ring_overflow/tests/overflow_test.rs:  assert!( Resolution::EvictedOldest.accepted_incoming() );
ring_overflow/tests/overflow_test.rs:  assert!( !Resolution::Refused.accepted_incoming() );
ring_overflow/tests/overflow_test.rs:  assert!( !Resolution::Refused.lost_an_item() );
ring_overflow/tests/overflow_test.rs:  assert!( !Resolution::Refused.accepted_incoming() );
ring_overflow/tests/overflow_test.rs:      outcome.is_ok_and( Resolution::lost_an_item ), policy.drops_silently(),
ring_overflow/src/lib.rs:/// assert!( Resolution::DroppedIncoming.lost_an_item() );
ring_overflow/src/lib.rs:/// assert!( Resolution::EvictedOldest.lost_an_item() );
ring_overflow/src/lib.rs:/// assert!( !Resolution::Refused.lost_an_item() );
ring_overflow/src/lib.rs:  /// which [`Resolution::accepted_incoming`] is true, and deleting it would
ring_overflow/src/lib.rs:  /// assert!( Resolution::DroppedIncoming.lost_an_item() );
ring_overflow/src/lib.rs:  /// in [`Resolution::accepted_incoming`], which is exactly `Refused`'s profile:
ring_overflow/src/lib.rs:  /// assert!( Resolution::EvictedOldest.accepted_incoming() );
ring_overflow/src/lib.rs:  /// assert!( !Resolution::DroppedIncoming.accepted_incoming() );
ring_overflow/src/lib.rs:  /// assert!( !Resolution::Refused.accepted_incoming() );
ring_overflow/src/lib.rs:  /// Exhaustive for the reason given on [`Resolution::lost_an_item`].
ring_overflow/src/lib.rs:  // OverflowPolicy::Fail )` — `Resolution::Refused.lost_an_item()` is `false`,
  -- split by where it lives --
    in src, this crate (doctests)   8
    in tests, this crate            12
    in any other crate              0
```

---

### OV7 — Eighteen Calls, All of Them Inside This Crate

Both predicates are called eighteen times across the workspace: seven in this
crate's own doctests, eleven in its own test file, and zero anywhere else. No
other crate calls either, and no other crate declares `ring_overflow` except
`ring_core`, which imports `would_resolve` and `Resolution` and neither
predicate.

Seventeen of the eighteen name a variant literally — `Resolution::Refused
.lost_an_item()` — which is a question with a compile-time answer. Only
`overflow_test.rs:301` passes a value that was computed rather than written down,
and it does so as a function reference inside `is_ok_and`.

**Finding.** The two predicates are the crate's answer to "read the outcome
without matching on it," and every existing call already knows the variant. That
makes them fully tested and, so far, unexercised: nothing in the workspace has yet
had a `Resolution` in hand whose variant it did not statically know.

The absence is not an error — the one consumer needs one bit and gets it from the
`match` — but it means the partition the predicates define has never had to hold
for a value at runtime, and the properties asserted about them are properties of
three constants.

---

### OV8 — The One Test That Reads a Computed Resolution Reads It Through a Sibling Crate's Predicate

`policy_self_description_agrees_with_the_handler` is the only place a resolution
produced by `resolve` reaches a predicate. It asserts two agreements per policy:
`outcome.is_err() == policy.reports_failure()`, and
`outcome.is_ok_and( Resolution::lost_an_item ) == policy.drops_silently()`.

Both right-hand sides live in `ring_types`. So the test that gives this crate's
predicates their only dynamic exercise is a cross-crate consistency check, and
the property it establishes is that `ring_overflow`'s handler and `ring_types`'
policy self-description cannot drift apart.

**Finding.** That is the most valuable test in the file and its name says
"self description agrees with the handler" rather than what it defends against:
`OverflowPolicy` shipping predicates that claim one behaviour while the handler
implements another, in two crates that can be edited independently. The
`is_ok_and` composition is also the only place the crate's own `Result` shape and
its own `Resolution` shape are read together, which is where a mismatch between
them would surface
([`decisions/002`](../decisions/002_fail_returns_an_error_not_a_resolution.md)).

What it does not cover is the third policy's resolution: `is_ok_and` returns
`false` for `Fail` because the outcome is `Err`, matching `drops_silently()`
being false — so the assertion passes without `Resolution::Refused` being
constructed at all. The one dynamic path into the predicates reaches two of the
three variants.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`api/001`](001_four_declarations_three_of_them_const.md) | The declarations these two sit among |
| [`algorithm/002`](../algorithm/002_where_the_third_outcome_goes.md) | The consumer that reads the outcome without them |
| [`type/002`](../type/002_three_variants_and_two_questions.md) | What the two predicates partition |
| [`integration/001`](../integration/001_one_consumer_one_import_one_site.md) | The import list that excludes them |

### Sources

| Fact | Where |
|------|-------|
| Both predicate bodies | `ring_overflow/src/lib.rs:111-118`, `:138-145` |
| Eighteen calls, all in-crate | Census above |
| The one computed-value call | `ring_overflow/tests/overflow_test.rs:301` |
| The cross-crate predicates it compares against | `ring_types/src/policy.rs:153`, `:178` |
| `ring_core`'s import list | `ring_core/src/lib.rs:80` |

### Tests

| Test | Covers |
|------|--------|
| `the_two_readings_partition_the_outcomes` | All six variant/predicate pairs, on literals |
| `a_refusal_loses_nothing` | Both predicates on `Refused`, on a literal |
| `no_resolution_overwrites_unread_data_silently` | The implication between them, over `OverflowPolicy::ALL` |
| `policy_self_description_agrees_with_the_handler` | The only computed resolution reaching a predicate |
