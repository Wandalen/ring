# Algorithm: Two Wrappers Over a Predicate They Fix

### Scope

- **Purpose**: Show what `for_space` and `for_data` add to `wait_until`, what they take away, and why what they take away is the reason neither has a production caller.
- **Responsibility**: State both bodies in full, name the single asymmetry between them, and record the consequence for anyone who needs a second exit condition.
- **In Scope**: `for_space`, `for_data`, and the closures they supply on the caller's behalf.
- **Out of Scope**: The loop itself — see [`algorithm/001`](001_one_loop_and_the_two_ways_out.md). The consumer that had to rewrite them — see [`integration/002`](../integration/002_the_wrapper_that_had_to_be_rewritten.md).

### Both Bodies

```rust
// ring_wait/src/lib.rs:239-242
pub fn for_space( pair : &CursorPair, kind : WaitKind, spins : usize ) -> Result< usize, RingError >
{
  wait_until( kind, spins, || pair.may_claim() ).map_err( | _ | RingError::Full )
}

// ring_wait/src/lib.rs:265-269
pub fn for_data( pair : &CursorPair, count : u64, kind : WaitKind, spins : usize )
-> Result< usize, RingError >
{
  wait_until( kind, spins, || pair.pending() >= count )
}
```

One line each. The module documentation states the reason at `:24-30`: two names
for the two questions a caller actually asks, over exactly one loop, so the
retry budget, the pause behaviour, and the give-up condition cannot drift apart
between producer and consumer.

### The One Difference

| | `for_space` | `for_data` |
|--|-------------|------------|
| Predicate | `pair.may_claim()` | `pair.pending() >= count` |
| Extra parameter | none | `count : u64` |
| Failure | `RingError::Full` | `RingError::Empty` |
| `map_err` | yes | **no** |
| Asks about | the producer's room | the consumer's backlog |

The `map_err` is the whole of the asymmetry, and W2 exists to keep it visible:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -vE "^[[:space:]]*//" ring_wait/src/lib.rs \
  | grep -E "RingError::[A-Za-z]+"
```

Live output:

```
    Err(RingError::Empty)
    wait_until(kind, spins, || pair.may_claim()).map_err(|_| RingError::Full)
```

Exactly two hits — `Empty` as `wait_until`'s own give-up value, `Full` in
`for_space`'s `map_err`. `for_data` does not appear, because it wants
`wait_until`'s error unchanged. A symmetrical `map_err( |_| RingError::Empty )`
there would be a line that says nothing and one more place to get it wrong.

Why the translation matters is argued at `:221-224` and asserted at
`tests/wait_test.rs:255-268`: a producer handed `Empty` reads it as *"nothing to
do"* and stops producing, when what it was told is *"back-pressure, retry"*. The
two errors are the same event seen from opposite sides of the ring.

### WT3 — Fixing the Predicate Is What Makes Them Unreachable

The convenience these two functions provide is that the caller does not write
the closure. That is also the entire cost: **the closure is not a parameter, so
it cannot be extended.** A caller who needs the same wait plus one more reason
to stop cannot wrap `for_space` — there is nowhere to put the second condition —
and must go back to `wait_until`.

That is not hypothetical. `ring_shutdown::for_space_or_close` is exactly
`for_space` with a second exit, and its documentation says so at
`ring_shutdown/src/lib.rs:583`. It is built on `wait_until`:

```rust
// ring_shutdown/src/lib.rs:613-621
let outcome = ring_wait::wait_until( kind, spins, ||
{
  if shutdown.is_closed()
  {
    closed = true;
    return true;
  }
  pair.may_claim()
} );
```

The `pair.may_claim()` on the last line is `for_space`'s closure, re-typed. The
family's one production consumer of "wait for space" reached past the function
named for that question, and copied its body instead.

The census is the finding:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r --include=*.rs "ring_wait::" . | grep -vE ':\s*(///|//!|//)' | grep -v '^ring_wait/'
```

Live output:

```
ring_shutdown/src/lib.rs:  ring_wait::wait_until( kind, spins, || shutdown.is_closed() )
ring_shutdown/src/lib.rs:  let outcome = ring_wait::wait_until( kind, spins, ||
ring_barrier/src/lib.rs:    ring_wait::wait_until( kind, spins, || self.admits( from, count ) )?;
```

Three call sites, all `wait_until`. `for_space` and `for_data` are called from
`ring_wait/tests/wait_test.rs` and nowhere else — six and eight times
respectively, plus one doctest each.

**This is a design signal, not an oversight.** A wait predicate in real code is
almost never a single question: it is *this, or shutdown*, or *this, or the
deadline*, or *this, and the barrier admits it*. The two named wrappers serve
the case where it is exactly one question, which is the case that has not yet
occurred outside a test.

### WT9 — A `count` of Zero Is a Wait That Never Waits

`for_data`'s predicate is `pair.pending() >= count`. At `count == 0` that is a
tautology over a `u64`, so the function returns `Ok( 0 )` on a ring that has
never been published to:

```
probe: for_data( &CursorPair::new( Capacity::new( 4 ) ), 0, WaitKind::None, 1 )
       = Ok(0)
```

No doctest mentions it: the three in `for_data`'s own block use counts of 1, 3,
and 4 (`src/lib.rs:259-263`). One test does —
`a_zero_count_is_already_satisfied_and_spends_no_attempt`
(`tests/wait_test.rs:383-400`). The behaviour is arguably right —
*"wait until at least nothing is available"* is trivially satisfied — but it is
right by arithmetic rather than by decision, and the failure mode is a
`count` computed as `wanted - already_have` that underflows to zero and turns a
wait into a no-op. `for_space` has no equivalent because it takes no quantity.

### What `wait` Adds, and to Whom

The third wrapper is `wait` (`:210-215`), which is `wait_until` with
`DEFAULT_SPINS` substituted for `spins`. It fixes the *budget* rather than the
predicate, and it fares no better: its only caller in the entire family is
`tests/wait_test.rs:243`.

| Wrapper | Fixes | Callers outside this crate |
|---------|-------|---------------------------:|
| `wait` | the budget | 0 |
| `for_space` | the predicate | 0 |
| `for_data` | the predicate, given a count | 0 |
| `wait_until` | nothing | **3** |

Every wrapper in the crate fixes one parameter, and every production caller
needed that parameter.

### Algorithms

| File | Relationship |
|------|--------------|
| [001_one_loop_and_the_two_ways_out.md](001_one_loop_and_the_two_ways_out.md) | The loop all three wrappers delegate to |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_seven_items_and_the_one_with_a_caller.md](../api/001_seven_items_and_the_one_with_a_caller.md) | WT1 — the surface-wide caller census |
| [../api/002_the_predicate_is_the_parameter.md](../api/002_the_predicate_is_the_parameter.md) | What a caller gives up by taking the wrapper |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_the_wrapper_that_had_to_be_rewritten.md](../integration/002_the_wrapper_that_had_to_be_rewritten.md) | `for_space_or_close`, in full |

### Items

| File | Relationship |
|------|--------------|
| [../item/002_the_loop_the_wrapper_and_the_two_questions.md](../item/002_the_loop_the_wrapper_and_the_two_questions.md) | Per-function contracts and coverage |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_two_atomic_loads_for_every_look.md](../non_functional_requirement/001_two_atomic_loads_for_every_look.md) | What each evaluation of these predicates costs |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_reading_empty_as_nothing_to_do.md](../pitfall/001_reading_empty_as_nothing_to_do.md) | The error the `map_err` exists to prevent |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_a_usize_budget_and_a_u64_count.md](../type/001_a_usize_budget_and_a_u64_count.md) | Why `count` is `u64` and `spins` is `usize` |

### Sources

| File | Relationship |
|------|--------------|
| `ring_cursor/src/lib.rs:417-420` | `may_claim`, `for_space`'s predicate |
| `ring_cursor/src/lib.rs:387-390` | `pending`, `for_data`'s |
| `ring_shutdown/src/lib.rs:581-631` | `for_space_or_close`, built on the loop rather than on the wrapper |

### Tests

| File | Relationship |
|------|--------------|
| `tests/wait_test.rs:255-268` | The two failures are different errors |
| `tests/wait_test.rs:270-278` | Neither failure is a configuration error |
| `tests/wait_test.rs:280-291` | `for_data` counts pending, not capacity |
| `tests/wait_test.rs:293-302` | `for_space` tracks the consumer as well as the producer |
| `tests/wait_test.rs:383-400` | WT9's corner — a `count` of zero is satisfied before any pause |
| `tests/manual/readme.md` § W2 | Exactly two `RingError::` mentions, and which |
