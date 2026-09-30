# Item: The Loop, the Wrapper, and the Two Questions

### Scope

- **Purpose**: Read the four waiting functions — `wait_until`, `wait`, `for_space`, `for_data` — as a body-size ladder, and record what each one adds over the one below it.
- **Responsibility**: Give each body, name what it fixes, and follow the attempt count through the three production call sites.
- **In Scope**: `ring_wait/src/lib.rs:179-269`.
- **Out of Scope**: The dispatch these four share — see [`item/001`](001_the_four_arms_of_the_pause.md).

### Four Bodies, Thirteen Lines, One Loop

| Item | Declared | Body | Body lines | Fixes |
|------|----------|------|-----------:|-------|
| `wait_until` | `:179` | `:183-195` | 13 | nothing |
| `wait` | `:210` | `:214-215` | 2 | the budget |
| `for_space` | `:239` | `:241-242` | 2 | the predicate **and** the error |
| `for_data` | `:265` | `:268-269` | 2 | the predicate |

The bottom three are two lines only because the closing brace gets its own —
each is a single expression:

```rust
// ring_wait/src/lib.rs:214
wait_until( kind, DEFAULT_SPINS, ready )

// ring_wait/src/lib.rs:241
wait_until( kind, spins, || pair.may_claim() ).map_err( | _ | RingError::Full )

// ring_wait/src/lib.rs:268
wait_until( kind, spins, || pair.pending() >= count )
```

Three of the four public waiting functions are a single expression forwarding to
the fourth. That is not a criticism — a named `for_data` reads better at a call
site than an inline closure would — but it does mean the crate's whole executable
substance is thirteen lines, and the other three exist to spend the reader's
attention on a name instead of a predicate.

### What Each Wrapper Costs Its Caller

The ladder is not free. Each rung fixes a parameter, and a caller who wanted that
parameter has to climb back down:

| Wrapper | Fixed | A caller wanting it back must |
|---------|-------|-------------------------------|
| `wait` | `spins = 1024` | call `wait_until` |
| `for_space` | `pair.may_claim()`, `Err → Full` | call `wait_until` and re-type both |
| `for_data` | `pair.pending() >= count` | call `wait_until` and re-type it |

`ring_shutdown::for_space_or_close` is exactly that descent, performed for real:
it needed a second exit condition, so it re-typed `may_claim()` and the
`Err → Full` mapping over `wait_until` rather than wrapping `for_space`
([`integration/002`](../integration/002_the_wrapper_that_had_to_be_rewritten.md)).

### The Two Questions Have Opposite Failures

`for_space` is the only item in the crate that changes the error, and its own
doc comment is where the reason is written down (`:221-224`):

> [`RingError::Full`] when the budget runs out with the ring still full — note
> the error, which is the one difference from [`for_data`]. The two questions
> have the same shape and opposite failures, and a producer handed `Empty` would
> read it as "nothing to do" rather than "back-pressure".

| | `for_space` | `for_data` |
|--|-------------|------------|
| Asks | is there room? | are there ≥ `count` items? |
| Predicate | `pair.may_claim()` | `pair.pending() >= count` |
| Budget exhausted | `RingError::Full` | `RingError::Empty` |
| Caller | a producer | a consumer |
| Extra parameter | — | `count : u64` |

The `map_err` is one line and it is the crate's only defence against a real
misreading ([`pitfall/001`](../pitfall/001_reading_empty_as_nothing_to_do.md)).
Both are the *same* condition from the loop's point of view — the budget ran out
— and the two names differ only in who is asking.

### The Degenerate Corner

`for_data`'s predicate is `pair.pending() >= count`, and at `count = 0` that is a
tautology:

```rust
for_data( &pair, 0, WaitKind::None, 1 )   // Ok( 0 ), on an empty ring
```

`pending()` returns a `u64`, so `>= 0` holds unconditionally, the first look
succeeds, and the caller is told data is available when none is. Nothing in the
signature prevents it: `count` is a bare `u64` with no non-zero wrapper, and
`for_space` has no equivalent corner because it takes no count at all
([`type/001`](../type/001_a_usize_budget_and_a_u64_count.md)).

The corner has its own test.
`a_zero_count_is_already_satisfied_and_spends_no_attempt`
(`tests/wait_test.rs:383-400`) asserts that the same empty ring answering
`Err( Empty )` for a count of one answers `Ok( 0 )` for a count of none, under
all four strategies. `for_data_counts_pending_and_not_capacity`
(`tests/wait_test.rs:280-291`) is the neighbouring case and stops short of it,
walking `wanted` from **1** to 3 before checking that 4 of 3 pending fails.

Whether this matters depends entirely on whether a caller can reach it, and
today none can — `for_data` has no production caller
([`api/001`](../api/001_seven_items_and_the_one_with_a_caller.md) § WT1). It is a
sharp edge on an unused tool, which is the cheapest possible time to know about
it.

### WT4 — Where the Attempt Count Goes

Every one of the four returns `Result< usize, RingError >`, where the `usize` is
the attempt index at which the predicate finally answered true. Following it
through the three production call sites:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r --include=*.rs "ring_wait::" . \
  | grep -vE ':\s*(///|//!|//)' | grep -v '^ring_wait/'
```

Live output:

```
ring_shutdown/src/lib.rs:  ring_wait::wait_until( kind, spins, || shutdown.is_closed() )
ring_shutdown/src/lib.rs:  let outcome = ring_wait::wait_until( kind, spins, ||
ring_barrier/src/lib.rs:    ring_wait::wait_until( kind, spins, || self.admits( from, count ) )?;
```

| Call site | Keeps the count? | What it returns instead |
|-----------|:----------------:|-------------------------|
| `ring_shutdown::wait_for_close` (`:429`) | **yes** | the count, forwarded unchanged |
| `ring_shutdown::for_space_or_close` (`:464`) | no | `Wake::Ready` or `Wake::Closed` |
| `ring_barrier::Barrier::wait_for` (`:272`) | no | the frontier it went on to read |

**Two of three discard it**, and both do so for the same reason: they had
something more useful to hand back. The count survives only in the one consumer
that had nothing else to say — which is the honest reading of the return type,
not a complaint about it. A wait that reports *how long* it waited is telling the
caller something real; it is simply less interesting than a `Wake`.

The count is never zero-cost to produce, either — it is the loop variable, so it
exists whether anyone reads it or not. What the discarding shows is that the
crate's return type was designed for the consumer it does not have.

### The Whole Public Surface, Once

```sh
cd "$(git rev-parse --show-toplevel)"
grep -E "^pub (const|fn)" ring_wait/src/lib.rs
```

Live output:

```
pub const DEFAULT_SPINS : usize = 1024;
pub const fn escalation_hint( kind : WaitKind ) -> Option< WaitKind >
pub fn pause( kind : WaitKind, attempt : usize ) -> bool
pub fn wait_until< F >( kind : WaitKind, spins : usize, mut ready : F ) -> Result< usize, RingError >
pub fn wait< F >( kind : WaitKind, ready : F ) -> Result< usize, RingError >
pub fn for_space( pair : &CursorPair, kind : WaitKind, spins : usize ) -> Result< usize, RingError >
pub fn for_data( pair : &CursorPair, count : u64, kind : WaitKind, spins : usize )
```

| Item | Kind | Covered by |
|------|------|-----------|
| `DEFAULT_SPINS` | `const` | [`data_structure/002`](../data_structure/002_the_budget_and_the_attempt_index.md) |
| `escalation_hint` | `const fn` | [`lifecycle/002`](../lifecycle/002_the_escalation_ladder_nobody_climbs.md) |
| `pause` | `fn` | [`item/001`](001_the_four_arms_of_the_pause.md) |
| `wait_until` | `fn` | here |
| `wait` | `fn` | here |
| `for_space` | `fn` | here |
| `for_data` | `fn` | here |

Seven items. Four of them are in this instance, and three of those four have no
caller anywhere outside this crate's own tests.


### WT38 — The Two Arms That Can Block Are the Two With No Executable Example

`pause` carries a doctest. It exercises half the enum.

```sh
cd "$(git rev-parse --show-toplevel)"
grep 'pause(WaitKind' ring_wait/src/lib.rs
# which arms reach std
grep -A2 'WaitKind::Yield =>\|WaitKind::Park =>' ring_wait/src/lib.rs | grep 'std::'
```

Live output:

```
/// assert!(pause(WaitKind::Spin, 0), "spin says keep going");
/// assert!(!pause(WaitKind::None, 0), "None says stop after the first look");
            std::thread::yield_now();
```

The doctest asserts `pause( WaitKind::Spin, 0 )` is true and
`pause( WaitKind::None, 0 )` is false — the two arms that stay inside `core`. The
two that call into `std` — `Yield`'s `yield_now`, `Park`'s 50 µs `sleep` — have
no doctest.

The reason is plain: an example that yields or sleeps is an example whose runtime
depends on the scheduler, and `cargo test --doc` would carry that cost on every
run. Choosing the two cheap arms is the right call for a doctest.

It is worth measuring because those two arms are where everything interesting
about this crate is. They are the two that make it un-`no_std`, the two named by
`ring_handle`'s forbidden-substring guard (WT13), the reason `ring_poll` may not
depend on it, and the subject of the crate's single largest decision
([`decisions/001`](../decisions/001_park_sleeps_rather_than_parking.md)). The
executable documentation covers the two arms nobody argues about.

`tests/wait_test.rs` does exercise all four, in
`every_discriminant_has_a_handler_that_runs` — so this is a gap in the examples a
reader runs, not in the suite.

### Items

| File | Relationship |
|------|--------------|
| [001_the_four_arms_of_the_pause.md](001_the_four_arms_of_the_pause.md) | The dispatch all four of these share |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_one_loop_and_the_two_ways_out.md](../algorithm/001_one_loop_and_the_two_ways_out.md) | The thirteen lines, read as an algorithm |
| [../algorithm/002_two_wrappers_over_a_predicate_they_fix.md](../algorithm/002_two_wrappers_over_a_predicate_they_fix.md) | WT3 — why fixing the predicate is what makes them unused |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_seven_items_and_the_one_with_a_caller.md](../api/001_seven_items_and_the_one_with_a_caller.md) | WT1 — which of the seven is actually called |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_the_wrapper_that_had_to_be_rewritten.md](../integration/002_the_wrapper_that_had_to_be_rewritten.md) | The descent back down the ladder, performed for real |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_reading_empty_as_nothing_to_do.md](../pitfall/001_reading_empty_as_nothing_to_do.md) | The `map_err` that is the only defence |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_a_usize_budget_and_a_u64_count.md](../type/001_a_usize_budget_and_a_u64_count.md) | The two integer parameters, and the corner one of them opens |
| [../type/002_one_return_type_and_the_one_must_use.md](../type/002_one_return_type_and_the_one_must_use.md) | `Result< usize, RingError >`, shared by all four |

### Sources

| File | Relationship |
|------|--------------|
| `ring_cursor/src/lib.rs:387-390,417-420` | `pending` and `may_claim`, the two fixed predicates |
| `ring_shutdown/src/lib.rs:555-579,581-631` | The two consumers, one keeping the count and one not |
| `ring_barrier/src/lib.rs:285` | The third call site |

### Tests

| File | Relationship |
|------|--------------|
| `tests/wait_test.rs:179-191` | `a_wait_reports_how_many_attempts_it_took` — the count is the index the predicate succeeded on |
| `tests/wait_test.rs:239-251` | `the_default_budget_is_the_documented_constant` — `wait` uses `DEFAULT_SPINS` |
| `tests/wait_test.rs:255-268` | `space_and_data_fail_with_different_errors` — the `Full`/`Empty` split |
| `tests/wait_test.rs:270-278` | `neither_failure_is_a_configuration_error` — both are retryable back-pressure |
| `tests/wait_test.rs:280-291` | `for_data_counts_pending_and_not_capacity` — `wanted` from 1 to 3, and 4 of 3 fails |
| `tests/wait_test.rs:293-302` | `for_space_tracks_the_consumer_as_well_as_the_producer` — a released slot reopens room |
| `tests/wait_test.rs:383-400` | `a_zero_count_is_already_satisfied_and_spends_no_attempt` — the degenerate corner above, `Ok( 0 )` under all four strategies |
