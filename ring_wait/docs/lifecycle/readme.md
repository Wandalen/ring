# lifecycle

Two lifecycles, and the second one never runs. The first is a single
`wait_until` call from its unconditional first look to one of two endings, with
the wall-clock arithmetic that turns a count into a duration. The second is the
progression a *strategy* is supposed to make when the first keeps failing —
fully specified, three tests, four doctest assertions, and zero callers anywhere
in the workspace.

Putting them adjacent is the point: the second exists to be reached from the
first's `Err`, and the first's `Err` does not carry enough information to reach
it.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [A Wait From the First Look to One of Two Endings](001_a_wait_from_the_first_look_to_one_of_two_endings.md) | What is fixed before the call, the state sequence, the unconditional first look, exhaustion time per strategy, and the three real continuations |
| 002 | [The Escalation Ladder Nobody Climbs](002_the_escalation_ladder_nobody_climbs.md) | `escalation_hint`'s three arms, its two differently-meaning `None`s, the cycle-freedom test, zero callers, and the missing input that explains it |

### The Two Lifecycles Side by Side

| | One wait (001) | One strategy (002) |
|--|----------------|--------------------|
| Governed by | `wait_until`, `:179-195` | `escalation_hint`, `:83-91` |
| States | look / pause / ended | `Spin` → `Yield` → `Park` |
| Longest path | `spins` attempts | 2 transitions |
| Terminates because | the range runs out | the ladder has a top |
| Termination asserted by | `tests/wait_test.rs:223-237` | `tests/wait_test.rs:344-359` |
| Production callers | 3 | **0** |

### Exhaustion Time — Regenerate

The budget is a count, so wall-clock exhaustion is count × per-attempt cost.
Per-attempt figures come from
[`non_functional_requirement/002`](../non_functional_requirement/002_what_each_strategy_costs_per_attempt.md);
the multiplication is here:

```sh
cd "$(git rev-parse --show-toplevel)"

# the budget being multiplied
grep "DEFAULT_SPINS : usize" ring_wait/src/lib.rs

# the sleep the Park row is dominated by
grep "from_micros" ring_wait/src/lib.rs
```

Live output:

```
pub const DEFAULT_SPINS : usize = 1024;
      std::thread::sleep( std::time::Duration::from_micros( 50 ) );
```

| Strategy | Per attempt | × 1024 |
|----------|------------:|-------:|
| `None` | — | 80–160 ns for the whole call — one look, never 1024 |
| `Spin` | 55–77 ns | ≈ 56–79 µs |
| `Yield` | 519–526 ns | ≈ 0.53 ms |
| `Park` | 112–119 µs | ≈ 115–122 ms |

### Regenerate the Caller Census

```sh
cd "$(git rev-parse --show-toplevel)"

# every call into this crate — three lines, all `wait_until`
grep -r --include=*.rs "ring_wait::" . \
  | grep -vE ':\s*(///|//!|//)' | grep -v '^ring_wait/'

# escalation_hint outside this crate — expected: one line, and it is prose
grep -r --include=*.rs "escalation_hint" . | grep -v "^ring_wait/"

# the crate's only `while`, and it is in a test
grep -r "while" ring_wait/src/ ring_wait/tests/*.rs
```

Live output:

```
ring_shutdown/src/lib.rs:  ring_wait::wait_until( kind, spins, || shutdown.is_closed() )
ring_shutdown/src/lib.rs:  let outcome = ring_wait::wait_until( kind, spins, ||
ring_barrier/src/lib.rs:    ring_wait::wait_until( kind, spins, || self.admits( from, count ) )?;
ring_types/src/policy.rs:  /// ones are `ring_wait`'s `escalation_hint` and `pause` in production source
ring_wait/tests/wait_test.rs:    while let Some( next ) = escalation_hint( kind )
```

| | Count |
|--|------:|
| Production call sites | 3 |
| Of those, calls that retry on `Err` | 0 |
| Of those, calls that escalate | 0 |
| `escalation_hint` references outside this crate | 1 |
| Of those, ones that are a call rather than prose | 0 |
| `while` loops in `src/` | 0 |
| `while` loops in `tests/` | 1 |

The single outside reference is a `ring_types` doc comment naming
`escalation_hint` as an example of a wildcard-free `match`
(`ring_types/src/policy.rs:45`). Nothing calls the function.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| WT39 | `ring_wait` | n/a — coverage | The file's one cross-thread test waits with `Yield` at a budget of 100 000, roughly 100× `DEFAULT_SPINS`; `Spin` and `Park` — the default variant and the expensive one — are never observed against a predicate that changes underneath them |
| WT40 | `ring_wait` | n/a — observation | A wait leaves the loop three ways — ready, `pause` refusing to continue, and range exhaustion — and the last two share one `Err( RingError::Empty )`, so "nothing right now" and "nothing after 1024 attempts" reach the caller as one variant |
| WT41 | `ring_wait` | n/a — observation | The ladder ends at `Park`, so "cheapest CPU" and "most expensive attempt" are the same rung; and `escalation_hint` folds `Park` and `None` into one arm, making the four-variant enum a three-rung ladder whose fourth variant is reachable only by choosing it outright |
| WT42 | `ring_wait` | n/a — coverage | Three of the file's 24 tests are dedicated to `escalation_hint`, which no crate calls, while `wait_until` — the one item another crate's `src/` reaches — has no test named for it; verification effort tracks how easy an item is to test, not what depends on it |
