# Data Structure: The Budget and the Attempt Index

### Scope

- **Purpose**: Account for the two integers this crate threads through every call — the budget the caller supplies and the attempt index the loop produces — and show what each is used for and what it is not.
- **Responsibility**: State where each originates, where it ends, who reads it, and what a caller can do with the one it gets back.
- **In Scope**: `spins`, `attempt`, and `DEFAULT_SPINS`.
- **Out of Scope**: Their widths as a type question — see [`type/001`](../type/001_a_usize_budget_and_a_u64_count.md).

### The Two Integers

| | `spins` | `attempt` |
|--|---------|-----------|
| Direction | in, from the caller | out, to the caller — and sideways, to `pause` |
| Declared at | `:179` (`wait_until`), `:239`, `:265` | `:183`, the `for` induction variable |
| Read by | the loop header only | `pause` (`:189`), and returned at `:187` |
| Clamped | `spins.max( 1 )` at `:183` | never |
| Default | `DEFAULT_SPINS = 1024` (`:63`) | starts at 0 |
| Meaning | how many looks are permitted | how many pauses have happened |

Nothing else in the crate is a number. There is no accumulated time, no
deadline, no attempt-so-far counter kept across calls, and no statistic — a
caller that wants any of those has `Stats::record_wait`
(`ring_stats/src/lib.rs:312`), which lives in a different crate and takes
nanoseconds this one never measures.

### A Count, Deliberately, Rather Than a Duration

`DEFAULT_SPINS`' own documentation states the reason at `:55-58`:

> Deliberately a plain count rather than a duration. A duration would make the
> same call take a different number of samples on different hardware, which
> turns a reproducible test into a flaky one — and this family's whole output is
> a measured verdict.

The trade is exact and worth stating in both directions:

| | A count (chosen) | A duration (rejected) |
|--|------------------|----------------------|
| Same call on two machines | same number of samples | same wall-clock, different sample count |
| Reproducible in a test | yes | no |
| Bounded in time | **no** | yes |
| Cost of `DEFAULT_SPINS` under `Spin` | ~52 ns/attempt → ~53 µs | fixed by construction |
| Cost of `DEFAULT_SPINS` under `Park` | ~120 µs/attempt → ~123 ms | fixed by construction |

The bottom two rows are the price. One budget spans a 2 300× range in wall-clock
depending on which strategy carries it, so `DEFAULT_SPINS` is a meaningful
default for `Spin` and an eighth of a second for `Park`
([`non_functional_requirement/002`](../non_functional_requirement/002_what_each_strategy_costs_per_attempt.md)).
A caller who picks the strategy from a config and leaves the budget alone gets
whichever of those it happens to land on, and nothing warns.

`ring_poll` reaches the same conclusion in its own words at
`ring_poll/src/lib.rs:86` — *"A budget bounds attempts, never time"* —
and then has to solve the wall-clock problem anyway, by refusing to admit any
strategy but a spin hint.

### The Attempt Index Goes Two Places

```rust
// ring_wait/src/lib.rs:183-192
for attempt in 0..spins.max( 1 )
{
  if ready()
  {
    return Ok( attempt );      // out, to the caller
  }
  if !pause( kind, attempt )   // sideways, to the strategy
  {
    break;
  }
}
```

**Out.** `Ok( attempt )` is the crate's only return channel for information
other than success or failure, and two of the three production call sites throw
it away ([`item/002`](../item/002_the_loop_the_wrapper_and_the_two_questions.md) § WT4).

**Sideways.** `pause` receives it so a strategy can behave differently early and
late (`:98-100`). Exactly one of the four arms reads it
([`item/001`](../item/001_the_four_arms_of_the_pause.md) § WT21), and what that
arm does with it is a sawtooth rather than a ramp
([`pitfall/002`](../pitfall/002_the_backoff_that_resets_every_eight_attempts.md) § WT5).

### What a Caller Can Learn From the Number It Gets Back

`Ok( n )` means *n pauses happened before the predicate answered true*. That is
strictly less information than it looks like:

| Question | Answerable from `Ok( n )`? |
|----------|---------------------------|
| How long did this take? | **no** — the strategy is not in the return value |
| Was the budget nearly exhausted? | only if the caller still has `spins` |
| Did the ring have data on the first look? | yes — `n == 0` |
| Should I escalate the strategy? | not directly; `escalation_hint` ignores `n` |

The last row is the gap that keeps `escalation_hint` unused. The natural caller
of an escalation hint is one that just spent its whole budget and wants a
cheaper strategy for the retry — but a caller in that position received
`Err( Empty )`, which carries **no** attempt count at all. The number survives
only on the success path, where nobody needs it
([`lifecycle/002`](../lifecycle/002_the_escalation_ladder_nobody_climbs.md)).

### `DEFAULT_SPINS` and the Doctest That Is Its Only Test

```rust
// ring_wait/src/lib.rs:60-63
/// ```
/// assert_eq!( ring_wait::DEFAULT_SPINS, 1024 );
/// ```
pub const DEFAULT_SPINS : usize = 1024;
```

A constant cannot carry a `#[ test ]`, so this doctest and
`tests/wait_test.rs:239-251` are the two places the value 1024 is asserted. The
test asserts it twice over — once as a literal, and once behaviourally, by
counting that `wait` makes exactly `DEFAULT_SPINS` looks. The behavioural half
is the one that would catch `wait` being changed to use some other budget while
the constant stayed put.

1024 is a power of two and nothing depends on that. Unlike `Capacity`, which
rejects non-powers of two at construction
(`ring_types/src/capacity.rs`), the budget is masked against nothing and
divides nothing; `spins` of 1000 works identically. The `% 8` inside `pause`'s
`Spin` arm is the only modulus in the crate, and it is applied to `attempt`, not
to `spins`.


### WT29 — A `count` Above Capacity Is a Budget That Cannot Be Met

`for_data`'s `count` is a bare `u64` compared against `pending()`. Nothing bounds
it by the ring's capacity, so a caller can ask for more records than the ring can
ever hold and buy a full budget of looks that are guaranteed to fail.

```sh
cd "$(git rev-parse --show-toplevel)"
# the comparison, and what bounds count
grep 'fn for_data' -A6 ring_wait/src/lib.rs | grep -v '^--'
# what the tests do with a count above what is pending
grep 'for_data' ring_wait/tests/wait_test.rs
```

Live output:

```
pub fn for_data( pair : &CursorPair, count : u64, kind : WaitKind, spins : usize )
-> Result< usize, RingError >
{
  wait_until( kind, spins, || pair.pending() >= count )
}
use ring_wait::{ escalation_hint, for_data, for_space, pause, wait, wait_until, DEFAULT_SPINS };
  let outcome = for_data( &pair, 1, WaitKind::None, DEFAULT_SPINS );
  assert_eq!( for_data( &pair, 2, WaitKind::None, DEFAULT_SPINS ), Ok( 0 ) );
  assert_eq!( for_data( &empty, 1, WaitKind::None, 1 ), Err( RingError::Empty ) );
fn for_data_counts_pending_and_not_capacity()
    assert!( for_data( &pair, wanted, WaitKind::None, 1 ).is_ok(), "{wanted} of 3 pending" );
  assert!( for_data( &pair, 4, WaitKind::None, 1 ).is_err(), "4 of 3 pending" );
      for_data( &pair, 1, WaitKind::Yield, 100_000 ).is_ok(),
  // `for_data`'s predicate is `pending() >= count` over a `u64`, so a count of
  // `for_data` cannot fail: the same empty ring that answers `Err( Empty )` for
  assert_eq!( for_data( &empty, 1, WaitKind::None, 1 ), Err( RingError::Empty ), "one of none pending" );
    assert_eq!( for_data( &empty, 0, kind, 4 ), Ok( 0 ), "{kind:?} spent an attempt on a count of zero" );
```

The body is `pair.pending() >= count`, and `count` is checked against nothing
else. A ring of capacity 4 asked for 5 records runs the predicate `spins` times,
sleeps or spins `spins` times, and returns `Empty` — the same answer it would
have given after one look, bought at the full price.

The tests reach the boundary and stop there.
`for_data_counts_pending_and_not_capacity` asserts `4 of 3 pending` fails, at a
budget of **1**. That is the cheapest possible spelling of the failing case and
it is the right test for the question it asks — whether `for_data` counts pending
records rather than capacity. It cannot observe the cost of the same failure at
`DEFAULT_SPINS`, because at a budget of 1 there is no difference between an
unsatisfiable request and an unsatisfied one.

This is the upper counterpart of the zero-count tautology WT9 records at the
other end. Both are inputs the type admits, neither is rejected, and the crate's
position — stated in the module documentation — is that the caller decides what
its own predicate means. That position is coherent. It also means the two
degenerate counts are the caller's to get wrong, with nothing here to notice.

### Data Structures

| File | Relationship |
|------|--------------|
| [001_a_crate_with_no_type_of_its_own.md](001_a_crate_with_no_type_of_its_own.md) | Why these two integers are all the state there is |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_one_loop_and_the_two_ways_out.md](../algorithm/001_one_loop_and_the_two_ways_out.md) | The clamp, and the loop that produces `attempt` |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_four_arms_of_the_pause.md](../item/001_the_four_arms_of_the_pause.md) | WT21 — the one arm that reads `attempt` |
| [../item/002_the_loop_the_wrapper_and_the_two_questions.md](../item/002_the_loop_the_wrapper_and_the_two_questions.md) | WT4 — who discards the returned count |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_the_escalation_ladder_nobody_climbs.md](../lifecycle/002_the_escalation_ladder_nobody_climbs.md) | Why the count reaches nobody who could use it |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_what_each_strategy_costs_per_attempt.md](../non_functional_requirement/002_what_each_strategy_costs_per_attempt.md) | What one budget costs under each of the four strategies |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_the_backoff_that_resets_every_eight_attempts.md](../pitfall/002_the_backoff_that_resets_every_eight_attempts.md) | WT5 — what `attempt` is actually used for |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_a_usize_budget_and_a_u64_count.md](../type/001_a_usize_budget_and_a_u64_count.md) | Why both are `usize` and `count` is not |

### Sources

| File | Relationship |
|------|--------------|
| `ring_poll/src/lib.rs:86,84-113` | `Budget` — the same idea, with an inspectable clamp |
| `ring_stats/src/lib.rs:312` | `record_wait`, which takes the nanoseconds this crate never measures |

### Tests

| File | Relationship |
|------|--------------|
| `tests/wait_test.rs:223-237` | Budgets 1 through 7 each produce exactly that many looks |
| `tests/wait_test.rs:239-251` | `wait` uses `DEFAULT_SPINS`, asserted behaviourally and by value |
| `tests/wait_test.rs:179-191` | The returned attempt index, zero-based |
