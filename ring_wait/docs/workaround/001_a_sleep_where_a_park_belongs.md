# Workaround: A Sleep Where a Park Belongs

### Scope

- **Purpose**: Record that `WaitKind::Park` sleeps rather than parks, name the missing capability that forces it, and state the two costs and the condition that deletes it.
- **Responsibility**: Quote the substitution, show what it is standing in for, follow the requested figure to the two places that compute against it, and give the deletion test.
- **In Scope**: `pause`'s `Park` arm, `:132-143`.
- **Out of Scope**: What the arm costs to run — see [`non_functional_requirement/002`](../non_functional_requirement/002_what_each_strategy_costs_per_attempt.md).

### The Substitution

```rust
// ring_wait/src/lib.rs:141
std::thread::sleep( std::time::Duration::from_micros( 50 ) );
```

The variant is called `Park`. `std::thread::park` is not called anywhere in this
crate, or anywhere in the family:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r --include=*.rs "thread::park\|::unpark" .
```

Live output:

```
ring_wait/src/lib.rs:      // Sleeping rather than `thread::park` on purpose. Parking requires the
```

**One hit in the whole family**, and it is the comment below explaining the
absence — `ring_wait/src/lib.rs:134`. No crate calls `park`, none calls
`unpark`, and no thread handle is stored anywhere to call it on.

The arm is 12 lines, of which 7 are the comment explaining the substitution
(`:134-140`):

> Sleeping rather than `thread::park` on purpose. Parking requires the publisher
> to hold the waiter's handle and unpark it, which is a registration
> relationship this crate deliberately does not have — `ring_handle` owns
> who-knows-whom. A short sleep is the same cost profile (idle rather than
> spinning) without inventing that relationship here, and the sleep length is
> what a real unpark would make unnecessary.

### The Missing Capability

`thread::park` is not a delay — it is half of a rendezvous. The waiter parks; the
publisher calls `unpark` on the waiter's `Thread` handle. For that to happen the
publisher must *have* the handle, which means someone must have registered the
waiter with the publisher.

| | A real park | This sleep |
|--|-------------|------------|
| Needs the waiter's handle | yes | no |
| Needs a registry | yes | no |
| Wakes on publication | yes | no — wakes on the clock |
| Latency after publication | ~0 | 0–50 µs, uniform |
| Wasted wakeups when nothing published | 0 | one every 50 µs |

The registration relationship is a real thing in this family and it has an owner:
`ring_handle` is the crate that knows who holds what. This crate is a leaf that
takes a `&CursorPair` and a closure
([`data_structure/001`](../data_structure/001_a_crate_with_no_type_of_its_own.md));
it has no way to learn who the publisher is, and acquiring one would mean either
depending on `ring_handle` or inventing a second registry beside it.

So the workaround is not laziness. It is the cheapest thing that preserves the
variant's *cost profile* — idle rather than spinning, which is the property
callers actually select `Park` for — without adding a dependency edge that the
family's own layering forbids
([`integration/001`](../integration/001_two_dependencies_two_dependents_and_a_roster.md)).

### Cost One: The Figure Is Not What Is Delivered

`from_micros( 50 )` is a floor, not a duration.
[`non_functional_requirement/002`](../non_functional_requirement/002_what_each_strategy_costs_per_attempt.md)
measures the arm at **112–119 µs** per attempt across three release runs —
**2.2–2.4×** the requested figure, because a sleep pays a scheduler round trip
the requested duration does not include.

That would be unremarkable if nothing computed against the requested figure. Two
things do:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r "50µs\|50 µs" */tests/*.rs
```

Live output:

```
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/input_timeline/tests/timeline_test.rs:    "event 1 is stamped 50µs, before its predecessor at 100µs",
ring_handle/tests/handle_test.rs:    "10 000 empty drains took {elapsed:?}; a 50µs park each would be ~500ms"
ring_poll/tests/poll_test.rs:/// The arithmetic is the assertion. `ring_wait`'s `Park` variant sleeps 50µs
```

| Where | The arithmetic | Against measured cost |
|-------|----------------|-----------------------|
| `ring_poll/tests/poll_test.rs:138-143` | "20 000 attempts … would cost about **1.0 s** … The 500 ms bound sits 2× under the parking cost" | 20 000 × 115 µs ≈ **2.3 s** — the bound sits **4.6×** under, not 2× |
| `ring_handle/tests/handle_test.rs:249` | "10 000 empty drains … a 50µs park each would be ~500ms", asserted against a 500 ms bound | 10 000 × 115 µs ≈ **1.15 s** |

Both errors are in the safe direction — each test discriminates by a wider
margin than its comment claims, so neither can pass a parking implementation by
accident. But `ring_handle`'s bound is the one worth looking at twice: 10 000 ×
50 µs is *exactly* 500 ms, so on the requested figure the bound lands precisely
on the value it is supposed to exclude. It discriminates only because the real
cost is 2.3× higher than the figure the comment reasons from.

Nothing anywhere asserts the 50 µs value itself. The pattern does match under
one `tests/` directory, and it belongs to a crate that names no `ring_*`
dependency in its manifest:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r "from_micros" */tests/*.rs 
```

Live output:

```
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/demo_live_capture/tests/capture_test.rs:  std::thread::sleep( Duration::from_micros( 50 ) );
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/demo_live_capture/tests/capture_test.rs:  std::thread::sleep( Duration::from_micros( 50 ) );
```

Both lines are one `demo_live_capture` test (relocated to `/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/` by the
demo-crate relocation) sleeping twice to advance a clock — the same duration by coincidence,
reached through no code of this crate's. So
changing `50` to `500` would leave both comments stale, both tests passing, and
nothing in the suite pointing at the line that moved.

### Cost Two: The Variant's Name Promises More Than the Arm Does

A caller reading `WaitKind::Park` in a config file has no way to learn from the
type that no parking happens. The discriminant lives in `ring_types`
([`decisions/002`](../decisions/002_the_discriminants_live_in_ring_types.md)) and
its doc comment there describes the strategy, not this implementation of it —
and this is the only implementation
([`pattern/002`](../pattern/002_discriminants_here_handlers_there.md) § WT22), so
the gap between the two is invisible from the naming crate.

That is a documentation cost rather than a correctness one: nothing behaves
wrongly, and the cost profile the caller selected is the one they get.

The manual plan treats the comment as the load-bearing part and says what happens
if it goes (`tests/manual/readme.md:83-86`):

> The risk is not the sleep; it is the sleep losing its explanation and looking
> like an oversight to the next reader, who fixes it by reaching for
> `thread::park` and produces a thread nobody will ever unpark.

So W4's check is inverted from the usual shape — it requires that `thread::park`
*appears* in `src/lib.rs`, because the only place it appears is the comment
explaining why it is not called:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -c "thread::park" ring_wait/src/lib.rs   # must be non-zero
```

Live output:

```
1
```

### The Deletion Condition

The comment's last clause states it: *"the sleep length is what a real unpark
would make unnecessary."*

Concretely, this arm can be replaced when a publisher can reach a waiting
thread's handle — that is, when `ring_handle` (or whatever owns registration by
then) exposes a way to register a waiter and signal it. At that point:

| Before | After |
|--------|-------|
| `sleep( 50 µs )`, return `true` | `park()`, return `true` |
| Wakes on the clock | Wakes on publication |
| 112–119 µs per attempt | one wakeup per publication |
| No dependency | needs whatever exposes the handle |

**The test that would notice:** none exists today. The check to add alongside the
replacement is a publication-latency measurement — publish from one thread while
another waits under `Park`, assert the wait returns within a bound far below
50 µs. Under the sleep that assertion fails by construction; under a real park it
passes.

The arrangement is already built. `a_blocking_wait_succeeds_when_another_thread_publishes`
(`tests/wait_test.rs:304-324`) spawns a publisher, sleeps 5 ms, stores, and
asserts the waiter saw it — but under `WaitKind::Yield`, not `Park`, and its own
comment declines the timing question outright:

> Not a timing assertion: the waiter's budget is large enough that only a
> genuinely broken loop fails it.

So the new test is that one with the kind changed and a bound added, which is why
it is worth naming here rather than filing as new work.

Until then the arm is correct, cheap, and honest in its comment if not in its
name — and the two tests that reason about it should have their arithmetic
corrected to the measured figure rather than the requested one, which is a
two-line edit in two crates that nothing currently forces.


### WT51 — The Number a Caller Cannot Change Is the One They Cannot Name

The crate has two tunables. One is a documented public constant with its own
doctest; the other is a literal inside a match arm.

```sh
cd "$(git rev-parse --show-toplevel)"
grep 'pub const DEFAULT_SPINS' ring_wait/src/lib.rs
grep 'from_micros' ring_wait/src/lib.rs
# every public constant the crate exports
grep -c '^pub const ' ring_wait/src/lib.rs
```

Live output:

```
pub const DEFAULT_SPINS : usize = 1024;
      std::thread::sleep( std::time::Duration::from_micros( 50 ) );
2
```

`DEFAULT_SPINS` is `pub`, carries eight lines of rationale explaining why a count
beats a duration, and has a doctest asserting its value. The sleep length is
`50` written inline, with no name, and the crate exports exactly one public
constant.

The asymmetry runs the wrong way for a caller. `DEFAULT_SPINS` is a *default* —
every function taking a budget takes it as an argument, so a caller who dislikes
1024 passes their own number and never touches the constant. The 50 µs is not a
default; it is the only value, reachable through no parameter, and a caller who
finds it wrong for their latency target has no argument to pass.

Naming it would not make it tunable — that needs a parameter or a config field,
which is the larger change [`workaround/001`](001_a_sleep_where_a_park_belongs.md)
is about. What naming it would buy is the thing `DEFAULT_SPINS` already has: a
symbol the two other crates that budget from the figure could reference instead
of transcribing it, which is the transcription WT7 measures as 2.3× off.

### Workarounds

| File | Relationship |
|------|--------------|
| [002_a_crate_name_as_a_load_bearing_string.md](002_a_crate_name_as_a_load_bearing_string.md) | The other missing capability — a dependency ban Cargo cannot express |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_the_discriminants_live_in_ring_types.md](../decisions/002_the_discriminants_live_in_ring_types.md) | Why the name and the arm live in different crates |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_two_dependencies_two_dependents_and_a_roster.md](../integration/001_two_dependencies_two_dependents_and_a_roster.md) | The layering that rules out reaching for `ring_handle` |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_four_arms_of_the_pause.md](../item/001_the_four_arms_of_the_pause.md) | The arm in the context of the other three |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_what_each_strategy_costs_per_attempt.md](../non_functional_requirement/002_what_each_strategy_costs_per_attempt.md) | WT7 — the 112–119 µs measurement |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_discriminants_here_handlers_there.md](../pattern/002_discriminants_here_handlers_there.md) | WT22 — one handler, so no second implementation contradicts this one |

### Sources

| File | Relationship |
|------|--------------|
| `ring_poll/tests/poll_test.rs:136-163` | The 500 ms bound and the 1.0 s estimate it reasons from |
| `ring_handle/tests/handle_test.rs:228-251` | 10 000 × 50 µs = exactly the bound |

### Tests

| File | Relationship |
|------|--------------|
| `tests/wait_test.rs:304-324` | The two-thread arrangement a latency bound would extend, currently run under `Yield` |
| `tests/wait_test.rs:159-175` | `Park` loops like the other two blocking kinds — the only thing asserted about it |
| `tests/manual/readme.md` § W4 | `Park` does not park, and the comment saying so must survive |
