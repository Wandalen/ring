# Decision: A `Result` Rather Than a `bool`

### Scope

- **Purpose**: Record why `check` reports *why* it refused, and establish what the decision has actually bought so far.
- **Responsibility**: State the decision and its reasoning, then follow it to the one production retry loop in the family and report what that loop does instead.
- **In Scope**: `check`'s return type, and the two predicates it exists to feed.
- **Out of Scope**: The order of `check`'s two tests — see [`algorithm/002`](../algorithm/002_check_orders_its_two_refusals.md).

### The Decision

> `GatingSet::check` distinguishes them, and the distinction is the whole reason
> it returns a `Result` rather than a `bool`. `Full` is back-pressure: the caller
> should retry, because a consumer will move. `BatchTooLarge` is a configuration
> error: a claim wider than the ring can never fit no matter who moves, and a
> retry loop that could not tell them apart would spin forever on the second.
> `RingError::is_configuration` is the caller's test.
>
> — `ring_gating/src/lib.rs:31-38`

The argument is sound and the failure it describes is real. A `bool` gate makes
these two situations identical:

| Situation | `admits` | Correct response |
|-----------|:--------:|------------------|
| Ring full, claim of 4 on a capacity of 8 | `false` | Retry — a consumer will move |
| Claim of 9 on a capacity of 8 | `false` | Stop — nothing will ever make this fit |

A loop written against `admits` alone cannot tell them apart, so it either spins
forever on the second or gives up prematurely on the first. `check` exists to
remove that choice, and `admits` is kept beside it for callers that have already
validated the width.

### G8 — The One Loop Over the Gate Does Not Use It

`check` has **no caller outside this crate's own tests**:

```sh
cd "$(git rev-parse --show-toplevel)"
for f in ring_*/src/*.rs ring_*/tests/*.rs; do
  grep -vE "^[[:space:]]*(///|//!)" "$f" | grep '\.check(' | sed "s|^|$f:|"
done
# every hit is ring_gating/tests/gating_test.rs
```

Live output:

```
ring_gating/tests/gating_test.rs:  assert_eq!( set.check( producer, 1 ), Err( RingError::Full ) );
ring_gating/tests/gating_test.rs:    if set.check( producer, batch ).is_ok()
ring_gating/tests/gating_test.rs:  let err = set.check( Seq::ZERO, 5 ).unwrap_err();
ring_gating/tests/gating_test.rs:  assert_eq!( set.check( Seq( 3 ), 2 ), Err( RingError::Full ) );
ring_gating/tests/gating_test.rs:  assert_eq!( set.check( Seq( 4 ), 4 ), Err( RingError::Full ) );
ring_gating/tests/gating_test.rs:  assert!( set.check( Seq( 4 ), 5 ).unwrap_err().is_configuration() );
ring_gating/tests/gating_test.rs:          set.check( Seq( producer ), count ).is_ok(),
ring_gating/tests/gating_test.rs:  assert!( full.check( Seq( 4 ), 0 ).is_ok() );
```

The caller the decision was written for is `ring_claim::Claimer::claim`, and it
is a real retry loop that really does need the distinction. It gets it by
reproducing both of `check`'s branches by hand:

```rust
// ring_claim/src/lib.rs:421-448
pub fn claim( &self, count : usize ) -> Result< Claim, RingError >
{
  if count > self.consumers.capacity().get()
  {
    return Err( RingError::BatchTooLarge
    {
      requested : count,
      capacity : self.consumers.capacity().get(),
    } );
  }

  let mut current = self.claimed();
  while count <= self.consumers.headroom( current )
  {
    …
  }

  Err( RingError::Full )
}
```

Line for line, that is `check`: the capacity test first, `BatchTooLarge` on
failure, then the headroom test, then `Full`. Both refusals, in the same order,
constructed from the same fields.

### Why the Duplication Is Not Simply a Miss

`claim` cannot call `check` and then act on the result, because the state it
checked can change between the check and the compare-exchange. The crate says so
directly:

> the gating check has to happen *before* it — and between that check and the
> add, another producer can take the space the check just found
>
> — `ring_claim/src/lib.rs:55-58`

So the headroom read must be **the loop condition**, re-evaluated on every failed
exchange. `check( producer, count )` returns a verdict about a moment, and by the
time the exchange runs that moment is gone. `while count <= headroom( current )`
is the same test in the only place it can safely live.

That accounts for the second branch. It does not account for the first:

| Branch | Must be inline? | Why |
|--------|:---------------:|-----|
| `Full` | ✅ | Must be the loop condition, re-read per iteration |
| `BatchTooLarge` | ❌ | Compares `count` against a capacity fixed at construction — cannot change |

`claim` could call `check` once for the width test before entering the loop, or
`ring_gating` could expose the width test on its own. Neither exists, so the
`BatchTooLarge` construction — including its two field names — is written twice
in two crates with nothing tying them together.

### What the Decision Has Bought

| Consumer | Uses the distinction? |
|----------|-----------------------|
| `ring_claim::claim` | Reproduces it rather than calling it |
| Any other production code | — no caller |
| Tests, across 7 crates | 27 assertions on `is_configuration` / `is_transient` |

Both predicates are called **zero** times from production code family-wide:

```sh
cd "$(git rev-parse --show-toplevel)"
for f in */src/*.rs ; do
  grep -vE "^[[:space:]]*(///|//!)" "$f" | grep -E 'is_configuration|is_transient' | sed "s|^|$f:|"
done
```

Live output:

```
ring_types/src/error.rs:  //   second look. `is_transient` just below carried the identical shape.
ring_types/src/error.rs:  pub const fn is_configuration( self ) -> bool
ring_types/src/error.rs:  // Fix(ring_error_classification_not_exhaustive): same shape as `is_configuration`
ring_types/src/error.rs:  // Root cause: see `is_configuration` above — an exhaustive match was available
ring_types/src/error.rs:  pub const fn is_transient( self ) -> bool
```

Two hits, both in `ring_types/src/error.rs`: the two definitions, and nothing
else.

**Correction (2026-09-28):** now five hits, still all in `ring_types/src/error.rs`
and still nothing outside that one file. A later `ring_types` bug fix
(`Fix(ring_error_classification_not_exhaustive)`) replaced both predicates'
non-exhaustive `matches!` bodies and added `Fix`/`Root cause`/`Pitfall` source
comments that name `is_configuration` by name while explaining why
`is_transient` needed the identical treatment — three comment lines join the
two definitions the recipe already found. The finding this recipe supports —
zero production callers of either predicate — is unaffected; the new lines
are commentary about the fix, not a caller.

So the decision is currently **defended by tests and unexercised by callers**.
That is not the same as wrong. The reasoning holds, the failure mode it prevents
is real, and the day a second backend or a policy layer wraps the gate, `check`
is what it should call. What is worth recording is that the family has exactly
one production loop over the gate — `ring_claim::claim` — it is exactly the
caller this method was designed for, and it does not use it.

The family's other loop, `ring_poll::push_within`, retries a *refusal* rather
than an exchange, and it is equally uninvolved: `try_push` hands the record back
instead of a `RingError`, so there is no verdict for it to classify either. That
is [`lifecycle/002`](../lifecycle/002_the_producer_walking_a_lap_against_a_stall.md)
§ G18, and it is why G7's zero is a consequence of two signatures rather than an
oversight by two callers.

**Nothing checks that the two copies of the refusal logic stay in step.** None of
the seven manual checks compares them, and no test asserts that `claim` and
`check` refuse the same inputs the same way — the compiler catches a field
rename and nothing else, per the table below. That absence belongs to the same
class as [`workaround/001`](../workaround/001_the_manual_check_that_names_a_foreign_method.md):
a property that matters, is not expressible to the compiler, and is checked by
hand or not at all. Here it is not at all.

### What Would Make the Duplication a Defect

| Change | Effect |
|--------|--------|
| `RingError::BatchTooLarge` gains or renames a field | Both copies must be updated; the compiler catches it |
| `check` gains a third refusal | Silent divergence — `claim` keeps refusing only two ways |
| The capacity becomes mutable | The width test stops being loop-invariant, and `claim`'s early return goes stale |

Only the first is caught by the build. The third cannot happen while
[`data_structure/001`](../data_structure/001_the_set_that_cannot_grow.md) holds —
no `&mut self` method exists — which is one more thing that invariant is quietly
load-bearing for.

### GT17 — The Distinction Has No Production Consumer

```
is_configuration / is_transient
  callers in : 27
  callers in   :  0
  definitions                : ring_types/src/error.rs:111, :146
```

The whole point of returning a `Result` rather than a `bool` is that a caller
can tell a configuration error from back-pressure. Measuring who does that gives
one number for tests and another for shipping code.

**Finding.** The two predicates the whole `Result`-over-`bool` decision exists to feed are called 27 times from tests and **zero** times from production code in all 33 crates

---

### GT18 — The Detail Is on the Wrong Side

```
BatchTooLarge { requested, capacity }   <- stop; both numbers carried
Full                                     <- retry; no payload
```

A caller that must give up is told exactly what it asked for and what was
available. A caller that should back off and try again is told nothing it could
size a backoff with.

**Finding.** `BatchTooLarge { requested, capacity }` carries both numbers; `Full` carries none. The variant a caller must give up on has full detail, and the variant a caller must retry — where knowing how much room exists would inform a backoff — has nothing

---

### GT62 — The Third Rung Has No Caller

```
callers of GatingSet::check outside ring_gating/tests/ : 0
ring_claim/src/lib.rs:423:    if count > self.consumers.capacity().get()
                      :425:      return Err( RingError::BatchTooLarge
                      :488:    while let granted @ 1.. = max.min( .. headroom( current ) )
                      :498:    Err( RingError::Full )
```

The decision to return a `Result` produced a third rung — `check` — that says
*why not*. The one crate that most needs both refusals writes them out itself.

**Finding.** It has no caller outside this crate's tests. `ring_claim::claim` reproduces both of its branches by hand — the `BatchTooLarge { requested, capacity }` refusal at `src/lib.rs:423` and the `Full` at `:498` — and only one of the two needs to be inline: the headroom re-read must sit inside the CAS loop, the capacity test need not

---



### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_check_orders_its_two_refusals.md](../algorithm/002_check_orders_its_two_refusals.md) | The order the duplicate must also preserve |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_reading_that_returns_a_position.md](../api/002_the_reading_that_returns_a_position.md) | Which refusals a caller may retry on |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_set_that_cannot_grow.md](../data_structure/001_the_set_that_cannot_grow.md) | Why the width test is loop-invariant |

### Decisions

| File | Relationship |
|------|--------------|
| [001_capacity_for_an_empty_set.md](001_capacity_for_an_empty_set.md) | The crate's other stated decision |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_three_dependencies_and_two_dependents.md](../integration/001_three_dependencies_and_two_dependents.md) | `ring_claim` as a dependent, and what it borrows |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_the_producer_walking_a_lap_against_a_stall.md](../lifecycle/002_the_producer_walking_a_lap_against_a_stall.md) | G18 — the other loop, and why it cannot use the distinction either |

### Workarounds

| File | Relationship |
|------|--------------|
| [../workaround/001_the_manual_check_that_names_a_foreign_method.md](../workaround/001_the_manual_check_that_names_a_foreign_method.md) | The same class of unenforceable property, checked by hand there and not at all here |

### Sources

| File | Relationship |
|------|--------------|
| `ring_gating/src/lib.rs:31-38` | The decision, stated |
| `ring_gating/src/lib.rs:247-294` | `check`'s doc and body |
| `ring_claim/src/lib.rs:421-447` | The retry loop that reproduces it |
| `ring_claim/src/lib.rs:55-58` | Why the headroom read must be the loop condition |
| `ring_types/src/error.rs:110-159` | `is_configuration` and `is_transient` |

### Tests

| File | Relationship |
|------|--------------|
| `tests/gating_test.rs:222-229` | `BatchTooLarge`, asserted to be a configuration error |
| `tests/gating_test.rs:232-238` | `Full`, asserted not to be |
| `ring_claim/tests/claim_test.rs:180` | The same assertion against the reproduced copy |
| `ring_claim/tests/claim_test.rs:191` | And its `Full` counterpart |
