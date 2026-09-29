# Pitfall: The Backoff That Resets Every Eight Attempts

### Scope

- **Purpose**: Record that `Spin`'s attempt-dependent pause is a repeating sawtooth rather than a backoff, give its exact shape, and note that nothing asserts it.
- **Responsibility**: Show the expression, tabulate what it produces, say what it is not, and identify the assertion gap.
- **In Scope**: `pause`'s `Spin` arm at `:116-126`.
- **Out of Scope**: What one hint costs — see [`non_functional_requirement/002`](../non_functional_requirement/002_what_each_strategy_costs_per_attempt.md).

### The Expression

```rust
// ring_wait/src/lib.rs:121-124
for _ in 0..=( attempt % 8 )
{
  core::hint::spin_loop();
}
```

The signature's justification reads like a backoff (`:98-100`):

> `attempt` is the zero-based index of the pause about to happen, so a strategy
> can behave differently early and late — [`WaitKind::Spin`] uses it to emit a
> CPU pause hint rather than a bare busy loop.

*"Differently early and late"* is what a backoff does. This is not one.

### WT5 — It Is a Period-8 Sawtooth

```
attempt:  0  1  2  3  4  5  6  7 | 8  9 10 11 12 13 14 15 | …
hints:    1  2  3  4  5  6  7  8 | 1  2  3  4  5  6  7  8 | …
```

`attempt % 8` discards everything above the low three bits, so attempt 1000 and
attempt 0 are indistinguishable:

| Attempts | Hints emitted |
|----------|---------------|
| 0–7 | 1, 2, 3, 4, 5, 6, 7, 8 |
| 1000–1007 | 1, 2, 3, 4, 5, 6, 7, 8 |
| 1 000 000–1 000 007 | 1, 2, 3, 4, 5, 6, 7, 8 |

```sh
python3 -c "
n = 1024
print( 'total hints:', sum( ( a % 8 ) + 1 for a in range( n ) ) )
print( 'attempts 1000..1007:', [ ( a % 8 ) + 1 for a in range( 1000, 1008 ) ] )
"
```

Live output:

```
total hints: 4608
attempts 1000..1007: [1, 2, 3, 4, 5, 6, 7, 8]
```

Over `DEFAULT_SPINS`: **4608** `spin_loop()` calls across 1024 attempts, mean
4.5 per attempt, and the same 128 times over.

### What It Is and Is Not

| | This | A real exponential backoff |
|--|------|---------------------------|
| Hints at attempt 7 | 8 | 128 |
| Hints at attempt 1000 | 1 | saturated |
| Grows without bound | no | yes, until clamped |
| Bounded per call | yes — 8, unconditionally | needs an explicit ceiling |
| Total over 1024 attempts | 4608 | ~2¹⁰ at the clamp × attempts |

The bounded-ness is a genuine virtue and it is what
[`invariant/001`](../invariant/001_every_repetition_is_a_counted_for.md) counts
on: the inner loop cannot be given a larger bound by any caller, so the crate's
total repetition count is `spins × 8` and both factors are visible in the source.

What it does not do is escalate. A `Spin` wait at attempt 900 is spending exactly
the same effort as at attempt 4, so the strategy has no way to become gentler as
a wait drags on — that job belongs entirely to `escalation_hint`, and nothing
calls it ([`lifecycle/002`](../lifecycle/002_the_escalation_ladder_nobody_climbs.md)).

Whether that is a defect depends on what the hint is for. `spin_loop()` is not a
delay; it is a hint to the pipeline that this is a spin-wait, and its documented
effects — reducing the memory-order-violation penalty on loop exit and freeing
the hyperthread sibling — do not accumulate with repetition. The crate's own
comment says exactly that (`:118-120`):

> A pause hint rather than an empty loop body: it tells the CPU this is a
> spin-wait, which cuts the memory-order-violation penalty on leaving the loop
> and stops the core from starving its hyperthread sibling.

Under that reading, emitting between 1 and 8 is arbitrary but harmless, and
emitting a growing number would be pointless. The pitfall is not that the shape
is wrong — it is that the parameter's stated purpose ("behave differently early
and late") describes something the code does not do.

### Nothing Asserts the Shape

```sh
cd "$(git rev-parse --show-toplevel)"
grep "% 8" ring_wait/src/lib.rs ring_wait/tests/*.rs
```

Live output:

```
ring_wait/src/lib.rs:      for _ in 0..=( attempt % 8 )
```

**One hit**, in `src/lib.rs`. The nearest test asserts only the arm's return
value across attempt indices:

```rust
// ring_wait/tests/wait_test.rs:366-371
// The attempt index feeds a backoff. What must hold regardless is that spin
// never reports "stop" — only `None` does that.
for attempt in 0..32
{
  assert!( pause( WaitKind::Spin, attempt ), "spin stopped at attempt {attempt}" );
}
```

That `pause` keeps saying `true`, not how many hints it emitted. `pause` returns
a `bool`, so the count is not observable from outside at all.

Note the first word of the comment. The test file calls it *a backoff* too, and
then — correctly — declines to assert anything about backing off. The name has
outlived the intent in two places, and the code is the only one that never
claimed it.

W5 in the manual plan covers the structural half — that the hint is inside a
bounded `for` rather than a bare loop — which is the property
[`invariant/001`](../invariant/001_every_repetition_is_a_counted_for.md) needs.
The modulus itself is unchecked by anything.

That is the right amount of testing for a hint whose effect is unobservable, and
it means the `8` is free to change. If it did, the only thing that would notice
is the arithmetic in this document.

### The One Thing Worth Guarding

`attempt % 8` is safe for every `usize`, including `usize::MAX`, and cannot
panic or overflow — the range `0..=( attempt % 8 )` has at most 8 elements no
matter what. So the parameter that three of four arms ignore
([`item/001`](../item/001_the_four_arms_of_the_pause.md) § WT21) is also
harmless in the fourth.

The failure mode to watch for is the opposite edit: replacing `% 8` with
something unbounded — `1 << attempt.min( 20 )`, say — would turn a constant-cost
arm into one whose cost is a function of the attempt index, and W1's grep would
not fire, because it would still be a counted `for`.


### WT48 — "Early and Late" Means Within the First Eight Attempts

`pause`'s documentation gives the `attempt` parameter a purpose. The arm that
reads it holds that purpose for eight attempts.

```sh
cd "$(git rev-parse --show-toplevel)"
grep 'behave differently early and late' -B2 -A2 ring_wait/src/lib.rs
grep '0..=( attempt % 8 )' ring_wait/src/lib.rs
```

Live output:

```
///
/// `attempt` is the zero-based index of the pause about to happen, so a
/// strategy can behave differently early and late — [`WaitKind::Spin`] uses it
/// to emit a CPU pause hint rather than a bare busy loop.
///
      for _ in 0..=( attempt % 8 )
```

The rustdoc says `attempt` is passed *"so a strategy can behave differently early
and late"*. The `Spin` arm reads it as `0..=( attempt % 8 )`, so the work at
attempt 1023 is the work at attempt 7, and the work at attempt 8 is the work at
attempt 0.

There is no late. Past the eighth attempt the arm has no state that distinguishes
a wait that just started from one about to give up at `DEFAULT_SPINS`, and the
only strategy that reads the parameter cannot express the shape the parameter
exists to allow.

The parameter is still doing something: the ramp inside each period is real, and
`the_spin_pause_varies_with_the_attempt_and_always_continues` asserts the arm
keeps returning true. And the interface is what matters — `attempt` is passed to
every arm, so a future strategy *can* behave differently late without changing a
signature. WT5 records the sawtooth and its naming; this records that the
sentence describing the parameter's purpose describes a capability the shipped
arm declines to use, over 99.2% of a default-budget wait.

### Pitfalls

| File | Relationship |
|------|--------------|
| [001_reading_empty_as_nothing_to_do.md](001_reading_empty_as_nothing_to_do.md) | The other thing that is not what its name suggests |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_every_repetition_is_a_counted_for.md](../invariant/001_every_repetition_is_a_counted_for.md) | The constant bound this arm contributes to `spins × 8` |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_four_arms_of_the_pause.md](../item/001_the_four_arms_of_the_pause.md) | WT21 — the only arm that reads `attempt` |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_the_escalation_ladder_nobody_climbs.md](../lifecycle/002_the_escalation_ladder_nobody_climbs.md) | Where escalation actually lives, and why it never happens |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_what_each_strategy_costs_per_attempt.md](../non_functional_requirement/002_what_each_strategy_costs_per_attempt.md) | `Spin` at 55–77 ns per attempt, mean 4.5 hints |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_the_predicate_the_pause_and_the_budget.md](../pattern/001_the_predicate_the_pause_and_the_budget.md) | `ring_poll`'s copies, which emit exactly one hint and never vary |

### Sources

| File | Relationship |
|------|--------------|
| `ring_poll/src/lib.rs:311,380,423` | The same hint, one per attempt, with no modulus at all |

### Tests

| File | Relationship |
|------|--------------|
| `tests/wait_test.rs:363-372` | The arm keeps returning `true` across attempt indices — the count is not asserted |
| `tests/manual/readme.md` § W5 | The hint is inside a bounded `for` |
