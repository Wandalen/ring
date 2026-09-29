# Lifecycle: A Wait From the First Look to One of Two Endings

### Scope

- **Purpose**: Follow one `wait_until` call through time — what is fixed before it starts, what alternates while it runs, how long each ending takes to arrive, and what the caller does next.
- **Responsibility**: Give the state sequence, the wall-clock arithmetic for a full budget under each strategy, and the three real continuations.
- **In Scope**: The lifetime of a single call.
- **Out of Scope**: The exits as control flow — see [`algorithm/001`](../algorithm/001_one_loop_and_the_two_ways_out.md).

### Fixed Before the First Look

Three things are decided by the caller and none of them changes afterwards:

| Fixed | By | Changeable mid-wait |
|-------|-----|:-------------------:|
| the strategy | `kind : WaitKind` | no — `kind` is read once per attempt and never compared |
| the budget | `spins : usize`, clamped to ≥ 1 | no |
| the question | the `ready` closure | no — though a `FnMut` may change *its own* answer |

The third row is the only one with any give in it, and the give belongs to the
caller, not to this crate
([`api/002`](../api/002_the_predicate_is_the_parameter.md)).

### The Sequence

```
        ┌─────────────────────────────────────────┐
        │  attempt = 0                            │
        └─────────────────┬───────────────────────┘
                          ▼
                    ┌───────────┐   true    ┌──────────────────┐
                    │  ready()  ├──────────►│  Ok( attempt )   │
                    └─────┬─────┘           └──────────────────┘
                          │ false
                          ▼
                 ┌──────────────────┐  false  ┌──────────────────┐
                 │ pause( kind, n ) ├────────►│  Err( Empty )    │
                 └────────┬─────────┘         └──────────────────┘
                          │ true                       ▲
                          ▼                            │
              ┌───────────────────────┐  exhausted     │
              │ attempt += 1, in range├────────────────┘
              └───────────┬───────────┘
                          │ still in range
                          └──────────────► back to ready()
```

Two arrows reach `Err( Empty )` and they are the same value from the same source
line, which is why the crate reads as having two endings rather than the three
exits it has ([`algorithm/001`](../algorithm/001_one_loop_and_the_two_ways_out.md)).

Note what the diagram has *no* box for: a timeout, a deadline check, a strategy
change, and a memory fence. None exists in the loop.

### The First Look Is Unconditional

Every path begins at `ready()`, and no argument the caller can supply skips it:

| Budget | Strategy | Looks |
|-------:|----------|------:|
| 0 | any | 1 — the clamp |
| 1 | any | 1 |
| 1024 | `None` | 1 — the `false` |
| 1024 | `Spin` / `Yield` / `Park` | up to 1024 |

The two ways of getting to exactly one look are unrelated — `spins.max( 1 )` at
`:183` and `WaitKind::None => false` at `:144` — and both are asserted
(`tests/wait_test.rs:206-221` and `:128-144`). A crate whose whole job is
waiting is careful that the degenerate case still *asks*, because an `Err` about
a ring nobody read is worse than a slow answer.

### How Long the Second Ending Takes to Arrive

The budget is a count, so the elapsed time of a full exhaustion is the count
times the per-attempt cost — a number that varies by strategy across more than
three orders of magnitude:

| Strategy | Measured per attempt | `DEFAULT_SPINS` (1024) exhausted in |
|----------|---------------------:|------------------------------------:|
| `None` | — | 80–160 ns for the whole call — one look, never 1024 |
| `Spin` | 55–77 ns | ≈ 56–79 µs |
| `Yield` | 519–526 ns | ≈ 0.53 ms |
| `Park` | 112–119 µs | ≈ **115–122 ms** |

That last row is the lifecycle fact worth carrying: a caller who passes
`WaitKind::Park` and the default budget has asked for a call that may not return
for an eighth of a second. Nothing in the signature says so — `spins` is a count
and the documentation is explicit that this is deliberate (`:55-58`):

> Deliberately a plain count rather than a duration. A duration would make the
> same call take a different number of samples on different hardware, which
> turns a reproducible test into a flaky one — and this family's whole output is
> a measured verdict.

The trade is real and it is stated as a trade: reproducible sample counts,
unpredictable wall-clock. A caller that needs a latency bound has to compute one
from the table above, and nothing in the crate helps it
([`non_functional_requirement/002`](../non_functional_requirement/002_what_each_strategy_costs_per_attempt.md)).

### What Happens After Each Ending

The three production call sites, in full:

| Consumer | On `Ok` | On `Err` |
|----------|---------|----------|
| `ring_shutdown::wait_for_close` | returns `Ok( n )` unchanged | returns `Err( Empty )` unchanged |
| `ring_shutdown::for_space_or_close` | inspects a captured `bool` → `Wake::Closed` or `Wake::Ready` | rewrites to `Err( Full )` |
| `ring_barrier::Barrier::wait_for` | `?` and continues to read the frontier | `?` — propagates to its own caller |

None of the three retries. Every one of them ends the wait at the first ending it
gets, which is what makes the budget meaningful: if a caller looped on `Err`, the
bound this crate provides would be a bound on nothing
([`invariant/001`](../invariant/001_every_repetition_is_a_counted_for.md) § the
second escape).

The one thing no consumer does is escalate. `escalation_hint` is right there,
it is the obvious response to *"the budget ran out"*, and its call count across
the whole workspace is zero
([`lifecycle/002`](002_the_escalation_ladder_nobody_climbs.md)).

### Why There Is No Second Wait Inside a Wait

A caller wanting *spin briefly, then park* has to write it:

```rust
// what escalation would look like, if anything did it
let mut kind = WaitKind::Spin;
loop
{
  match wait_until( kind, 64, &mut ready )
  {
    Ok( n ) => break Ok( n ),
    Err( e ) => match escalation_hint( kind )
    {
      Some( next ) => kind = next,
      None => break Err( e ),
    },
  }
}
```

That is a caller-side `loop` with no budget of its own — the third escape from
[`invariant/001`](../invariant/001_every_repetition_is_a_counted_for.md), written
out. The crate declines to provide it, and the decline is consistent: a
composite wait's bound would have to be a bound over strategies as well as
attempts, and `wait_until`'s signature has room for exactly one of each.


### WT39 — One Strategy of Four Is Ever Run Against a Real Publisher

The crate has one test that puts a waiter and a publisher on different threads.
It picks one `WaitKind` and the other three never appear in that shape.

```sh
cd "$(git rev-parse --show-toplevel)"
grep -A22 'fn a_blocking_wait_succeeds_when_another_thread_publishes' \
  ring_wait/tests/wait_test.rs | grep -E 'WaitKind|spawn|budget|for_data|assert'
# every other test's threading
grep -c 'thread::scope\|thread::spawn' ring_wait/tests/wait_test.rs
```

Live output:

```
  // The shape the strategies exist for. Not a timing assertion: the waiter's
  // budget is large enough that only a genuinely broken loop fails it.
    scope.spawn( ||
    assert!(
      for_data( &pair, 1, WaitKind::Yield, 100_000 ).is_ok(),
1
```

The test spawns a publisher, sleeps 5 ms in it, and waits with
`for_data( &pair, 1, WaitKind::Yield, 100_000 )`. It is the file's only use of
`thread::scope`. Every other test in the file evaluates predicates that are
already true or already false when the call begins.

`Yield` is the sensible pick — `Spin` would hold a core for the 5 ms and `Park`
would sleep through it in 50 µs steps — and the test's own comment says it is
deliberately not a timing assertion: the 100 000 budget exists so that only a
genuinely broken loop can fail it.

Two things follow. The budget is roughly 100× `DEFAULT_SPINS`, so the one test
that exercises a real blocking wait uses a figure no caller would; and `Spin` and
`Park`, the default variant and the expensive one, are never observed against a
predicate that changes underneath them. What the crate's states look like when
the answer arrives *during* a wait is measured once, for one arm out of four.


### WT40 — Three Endings, Two Spellings

A wait ends in one of three ways, and the type distinguishes two of them.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A12 -F '  for attempt in 0..spins.max( 1 )' ring_wait/src/lib.rs
# what both failure exits return
grep 'Err( RingError::Empty )\|break' ring_wait/src/lib.rs
```

Live output:

```
  for attempt in 0..spins.max( 1 )
  {
    if ready()
    {
      return Ok( attempt );
    }
    if !pause( kind, attempt )
    {
      break;
    }
  }
  Err( RingError::Empty )
}
      break;
  Err( RingError::Empty )
```

The loop leaves by `return Ok( attempt )` when the predicate answers true, by
`break` when `pause` returns false, or by exhausting the range. The second and
third both fall to the same `Err( RingError::Empty )` on the line below.

The distinction they lose is real. A `break` means the strategy refused to
continue — the `None` arm, one look, by design. A range exhaustion means the
strategy tried `spins` times and the answer never came. To the caller these are
"there is nothing right now" and "there has been nothing for 1024 attempts",
which are different facts about the ring, delivered as one variant.

A caller can recover the difference: `WaitKind::None` produces the first and only
the first, and the caller chose the `WaitKind`. So the information is not
destroyed, it is moved from the return value to the argument the caller already
had — consistent with this crate's general position that interpretation belongs
to the caller.

It is worth recording because `RingError::Empty` then carries three meanings
across the family — nothing yet, nothing after a full budget, and (through
`ring_barrier`'s `?`) a barrier that does not admit — while `for_space` exists to
rename exactly one of them (WT47).

### Lifecycles

| File | Relationship |
|------|--------------|
| [002_the_escalation_ladder_nobody_climbs.md](002_the_escalation_ladder_nobody_climbs.md) | The transition this lifecycle never makes |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_one_loop_and_the_two_ways_out.md](../algorithm/001_one_loop_and_the_two_ways_out.md) | The same sequence as control flow, with the third exit named |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_predicate_is_the_parameter.md](../api/002_the_predicate_is_the_parameter.md) | The one input with any give in it |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_budget_and_the_attempt_index.md](../data_structure/002_the_budget_and_the_attempt_index.md) | Count-not-duration, and what it costs |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_the_wrapper_that_had_to_be_rewritten.md](../integration/002_the_wrapper_that_had_to_be_rewritten.md) | The three continuations, read from the consumers' side |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_every_repetition_is_a_counted_for.md](../invariant/001_every_repetition_is_a_counted_for.md) | Why no consumer's retry loop would be bounded |
| [../invariant/002_none_looks_exactly_once.md](../invariant/002_none_looks_exactly_once.md) | The one-look row of the table above |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_four_arms_of_the_pause.md](../item/001_the_four_arms_of_the_pause.md) | What happens in the box marked `pause` |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_what_each_strategy_costs_per_attempt.md](../non_functional_requirement/002_what_each_strategy_costs_per_attempt.md) | Where the per-attempt column was measured |

### Sources

| File | Relationship |
|------|--------------|
| `ring_shutdown/src/lib.rs:555-579,581-631` | Two of the three continuations |
| `ring_barrier/src/lib.rs:285` | The third |

### Tests

| File | Relationship |
|------|--------------|
| `tests/wait_test.rs:179-191` | The ending's payload — a zero-based pause count |
| `tests/wait_test.rs:193-204` | The wait stops the moment the answer arrives |
| `tests/wait_test.rs:206-221` | The first look happens even at a zero budget |
| `tests/wait_test.rs:223-237` | Budgets 1 through 7, each honoured exactly |
| `tests/wait_test.rs:304-324` | A blocking wait that ends in `Ok` because another thread published |
