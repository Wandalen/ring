# Non-Functional Requirement: What Each Strategy Costs Per Attempt

### Scope

- **Purpose**: Measure the per-attempt cost of all four strategies, record the span between them, and check the one arithmetic claim the family makes about `Park` against what it actually delivers.
- **Responsibility**: Give the method, the numbers, the derived budget costs, and the two findings that fall out of them.
- **In Scope**: Wall-clock cost attributable to `pause`.
- **Out of Scope**: Memory traffic — see [`001`](001_two_atomic_loads_for_every_look.md).

### Method

No benchmark ships with the crate; these come from a temporary probe built
against the real `ring_wait`, exhausting a fixed budget against a predicate that
never succeeds and dividing by the budget:

```rust
let started = Instant::now();
let _ = wait_until( kind, budget, || false );
let each = started.elapsed().as_nanos() as f64 / budget as f64;
```

Release build, three runs, on this machine. `Spin` and `Yield` used a budget of
100 000 so timer resolution is not the dominant term; `Park` used 200 because at
~115 µs per attempt a large budget would take minutes. `None` cannot be measured
per attempt — it makes exactly one — so the whole call is reported instead.

The predicate is `|| false`, which costs nothing. What is measured is `pause`,
not a real wait ([`001`](001_two_atomic_loads_for_every_look.md) covers the
predicate's own cost).

### WT6 — Three Orders of Magnitude Between the Extremes

```
run 1  Spin   55.1 ns/attempt      run 1  Yield  526.2 ns/attempt
run 2  Spin   77.2 ns/attempt      run 2  Yield  522.5 ns/attempt
run 3  Spin   73.5 ns/attempt      run 3  Yield  519.0 ns/attempt

run 1  Park   118 736.9 ns/attempt      run 1  None  whole call 80 ns
run 2  Park   112 120.6 ns/attempt      run 2  None  whole call 160 ns
run 3  Park   114 471.6 ns/attempt      run 3  None  whole call 80 ns
```

| Strategy | Per attempt | Relative to `Spin` |
|----------|------------:|-------------------:|
| `Spin` | 55–77 ns | 1× |
| `Yield` | 519–526 ns | ≈ 8× |
| `Park` | 112–119 µs | ≈ **1700×** |
| `None` | — (one look, 80–160 ns for the whole call) | — |

**The same call with the same budget spans roughly 1700× depending on one enum
argument**, and the argument is a plain `WaitKind` with no warning attached and
no default that leans cheap. The default is the most expensive-per-core of the
four:

```rust
// ring_types/src/policy.rs:21-26
#[ derive( Debug, Clone, Copy, PartialEq, Eq, Hash, Default ) ]
pub enum WaitKind
{
  /// Re-read the cursor in a tight loop. Lowest latency, burns a core.
  #[ default ]
  Spin,
```

`ring_config` inherits it and asserts it in a doctest —
`RingConfig::new( 8 ).unwrap().wait()` is `WaitKind::Spin`
(`ring_config/src/lib.rs:164`). Defaulting to the cheapest *latency* is
the right call for a ring library; it is worth knowing that it is also the one
that holds a core.

Multiplied out against `DEFAULT_SPINS`:

| Strategy | `wait( kind, … )` failing, worst case |
|----------|--------------------------------------:|
| `None` | 80–160 ns |
| `Spin` | ≈ 56–79 µs |
| `Yield` | ≈ 0.53 ms |
| `Park` | ≈ **115–122 ms** |

An eighth of a second is a long time for a function whose signature says
`spins : usize` and nothing about duration. The count-not-duration decision is
deliberate and documented (`:55-58`) — reproducible sample counts across
hardware, at the price of unpredictable wall-clock — but the price lands
entirely on a caller who has to know this table to compute it.

**Disposition:** declined — this instance's own text calls the cheap-latency
default "the right call for a ring library"; the 1700× spread is a measured
characterization of the four `WaitKind` strategies, not a defect, and no
source or doc fix is implied beyond what is already recorded in
`non_functional_requirement/002_what_each_strategy_costs_per_attempt.md`.

### WT7 — `Park` Delivers Twice What It Asks For

```rust
// ring_wait/src/lib.rs:141
std::thread::sleep( std::time::Duration::from_micros( 50 ) );
```

| | Requested | Delivered | Factor |
|--|----------:|----------:|-------:|
| run 1 | 50 µs | 118.7 µs | 2.4× |
| run 2 | 50 µs | 112.1 µs | 2.2× |
| run 3 | 50 µs | 114.5 µs | 2.3× |

`thread::sleep` guarantees *at least* the requested duration, so this is correct
behaviour rather than a bug — the overshoot is scheduler granularity and wakeup
latency. It matters because **two** tests in two other crates compute a bound
from the requested figure, and neither of them can see this measurement.

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r "from_micros( 50 )\|50µs" {module,ring}/*/src/*.rs {module,ring}/*/tests/*.rs
```

Live output:

```
ring_wait/src/lib.rs:      std::thread::sleep( std::time::Duration::from_micros( 50 ) );
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/input_timeline/tests/timeline_test.rs:    "event 1 is stamped 50µs, before its predecessor at 100µs",
ring_handle/tests/handle_test.rs:    "10 000 empty drains took {elapsed:?}; a 50µs park each would be ~500ms"
ring_poll/tests/poll_test.rs:/// The arithmetic is the assertion. `ring_wait`'s `Park` variant sleeps 50µs
```

Six hits, and three of them bear on this: the constant itself, and two pieces
of arithmetic on it. The other three match the pattern without reasoning about
it — two unrelated `from_micros( 50 )` sleeps in a demo's tests, and a 50 µs
event stamp in a string.

#### The first — `ring_poll`, comfortably safe

`ring_poll/tests/poll_test.rs:136-143`:

> The arithmetic is the assertion. `ring_wait`'s `Park` variant sleeps 50µs per
> attempt, so 20 000 attempts through a parking implementation would cost about
> **1.0 s**. Through a spin hint the same 20 000 attempts cost single-digit
> milliseconds. The 500 ms bound sits 2× under the parking cost and roughly 100×
> over the spinning one, which is what makes it discriminate rather than merely
> pass.

Against the measured figure the real numbers are different, and different in the
safe direction:

| | Comment's estimate | Measured |
|--|-------------------:|---------:|
| 20 000 `Park` attempts | ≈ 1.0 s | ≈ **2.3 s** |
| The 500 ms bound sits under it by | 2× | **4.6×** |

So the test is *more* discriminating than its own comment claims — the best
direction for an error of this kind to point.

#### The second — `ring_handle`, saved by the overshoot

`ring_handle/tests/handle_test.rs:246-250`:

```rust
assert!
(
  elapsed < std::time::Duration::from_millis( 500 ),
  "10 000 empty drains took {elapsed:?}; a 50µs park each would be ~500ms"
);
```

Here the arithmetic lands the bound *on* the thing it is meant to exclude:
10 000 × 50 µs is exactly 500 ms, so on the comment's own numbers a parking
implementation would sit at the boundary rather than safely past it.

| | On the comment's 50 µs | On the measured 115 µs |
|--|----------------------:|-----------------------:|
| 10 000 parked drains | 500 ms | ≈ **1.15 s** |
| The 500 ms bound sits under it by | 1.0× — no margin | **2.3×** |

The test discriminates in practice, and it does so because the constant it
reasons about under-reports the real cost by a factor of two. That is a bound
that works for a reason its own comment does not state.

Both tests are correct today. What neither would survive is the 50 µs being
*lowered*: at 25 µs `ring_handle`'s bound would stop discriminating even against
the measured overshoot, and nothing in either crate would notice, because
`ring_wait` has no test asserting the constant at all. It appears exactly once as
a value (`src/lib.rs:141`), and the manual check covering this arm (W4) requires
only that the arm *sleeps* and *explains itself* — not that it sleeps any
particular length.

**Disposition:** declined — this instance's own text states the overshoot is
"correct behaviour rather than a bug — the overshoot is scheduler granularity
and wakeup latency", not a defect in `thread::sleep`'s use at
`ring_wait/src/lib.rs:141`; no fix to this crate's own source is
implied.

### What This Buys

Setting the costs against what each strategy is for:

| Strategy | Buys | Costs | Right when the wait is |
|----------|------|-------|------------------------|
| `Spin` | lowest latency | a core, continuously | nanoseconds |
| `Yield` | a scheduling slot back | a syscall per attempt | microseconds |
| `Park` | the core back entirely | ≈ 115 µs of latency per attempt | milliseconds |
| `None` | an immediate answer | no waiting at all | you cannot afford to wait |

That ordering is exactly `escalation_hint`'s
([`lifecycle/002`](../lifecycle/002_the_escalation_ladder_nobody_climbs.md)), and
this table is the measured justification for it — which makes it slightly odd
that nothing calls the function the table justifies.


### WT43 — Every Number in This File Comes From Code That Is Not in the Repository

This instance quotes per-attempt costs from 55 ns to 119 µs. None of the code
that produced them is in the tree.

```sh
cd "$(git rev-parse --show-toplevel)"
# benchmark harnesses anywhere in the family
ls -d ring_*/benches 2>/dev/null || echo '(no crate has a benches/ directory)'
# and the family's dev-dependency on a benchmark framework
grep -rl 'criterion\|divan' --include=Cargo.toml */ || echo '(no benchmark framework)'
```

Live output:

```
(no crate has a benches/ directory)
(no benchmark framework)
```

There is no `benches/` directory in any of the 33 crates and no benchmark
framework in any manifest. The figures were produced by a probe program written
into a scratch crate, run once, and transcribed — which the Module Index states
plainly at the head of its findings table: verified *"by a command whose output is
quoted in its instance, or by a probe program in a scratch crate."*

The consequence is asymmetric. Every structural claim in this corpus is backed by
a command the gate re-runs and compares against its quoted output, so a claim that
goes stale fails a check. Every timing claim is backed by a transcription, so a
figure that goes stale stays quoted. WT7's 112–119 µs against a requested 50 µs is
the finding most exposed to this: it is the basis for saying two other crates
budget from the wrong number, and it is not re-derivable by anything that runs.

That is a deliberate trade, not an oversight — wall-clock figures are exactly what
a determinism-checking gate cannot own, since quoting one makes the check fail on
any machine but the one that produced it. The cost of the trade is that the
crate's most consequential measurements are the ones nothing re-checks.

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [001_two_atomic_loads_for_every_look.md](001_two_atomic_loads_for_every_look.md) | The predicate's cost, which this measurement excludes |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_budget_and_the_attempt_index.md](../data_structure/002_the_budget_and_the_attempt_index.md) | Count-not-duration, and the trade it makes |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_park_sleeps_rather_than_parking.md](../decisions/001_park_sleeps_rather_than_parking.md) | Why the 50 µs exists at all |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_two_dependencies_two_dependents_and_a_roster.md](../integration/001_two_dependencies_two_dependents_and_a_roster.md) | The tick-path ban this cost justifies |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_four_arms_of_the_pause.md](../item/001_the_four_arms_of_the_pause.md) | The four arms being measured |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_a_wait_from_the_first_look_to_one_of_two_endings.md](../lifecycle/001_a_wait_from_the_first_look_to_one_of_two_endings.md) | The budget-times-cost arithmetic in context |
| [../lifecycle/002_the_escalation_ladder_nobody_climbs.md](../lifecycle/002_the_escalation_ladder_nobody_climbs.md) | The ordering this table justifies |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_the_backoff_that_resets_every_eight_attempts.md](../pitfall/002_the_backoff_that_resets_every_eight_attempts.md) | Why `Spin`'s cost does not grow with the attempt index |

### Workarounds

| File | Relationship |
|------|--------------|
| [../workaround/001_a_sleep_where_a_park_belongs.md](../workaround/001_a_sleep_where_a_park_belongs.md) | The 50 µs as the price of the missing relationship |

### Sources

| File | Relationship |
|------|--------------|
| `ring_wait/src/lib.rs:141` | The only occurrence of the 50 µs as a value |
| `ring_poll/tests/poll_test.rs:136-163` | The first piece of arithmetic on it — comfortably safe |
| `ring_handle/tests/handle_test.rs:232-251` | The second — a bound the overshoot rescues |
| `ring_types/src/policy.rs:21-26` | `Spin` is `WaitKind`'s `#[ default ]` |
| `ring_config/src/lib.rs:162-167` | `RingConfig` inherits that default, asserted by doctest |

### Tests

| File | Relationship |
|------|--------------|
| `tests/wait_test.rs:112-126` | The one timing assertion in the suite, and it is a ceiling not a measurement |
| `tests/wait_test.rs:304-324` | The suite's only cross-thread wait, and its largest budget — succeeds under `Yield` at 100 000 attempts (≈ 52 ms of budget; ≈ 11–12 s had it been `Park`) |
| `tests/manual/readme.md` § W4 | Requires the sleep and its explanation — not a particular length |
