# Integration: Two Dependencies, Two Dependents, and a Roster

### Scope

- **Purpose**: Give this crate's whole position in the family graph, including the one edge that is forbidden by test and the crate name that is a string literal in somebody else's assertion.
- **Responsibility**: Name every in-edge and out-edge, show the guard that pins the in-edge set, and record what the guard can and cannot see.
- **In Scope**: The dependency graph around `ring_wait`.
- **Out of Scope**: The roster check's own fragility as an absorbed constraint — see [`workaround/002`](../workaround/002_a_crate_name_as_a_load_bearing_string.md).

### The Whole Graph

```sh
cd "$(git rev-parse --show-toplevel)"
grep -rl "ring_wait" */Cargo.toml
```

Live output:

```
ring_barrier/Cargo.toml
ring_shutdown/Cargo.toml
ring_wait/Cargo.toml
```

| Direction | Crate | Why |
|-----------|-------|-----|
| **out** | `ring_types` | `WaitKind`, `RingError` |
| **out** | `ring_cursor` | `CursorPair`, for `for_space` and `for_data` only |
| **in** | `ring_barrier` | `Barrier::wait_for`, one call at `:272` |
| **in** | `ring_shutdown` | `wait_for_close` at `:429`, `for_space_or_close` at `:464` |

Four edges. Two out, two in, and the manifest scan finds exactly three files —
those two dependents plus this crate's own `Cargo.toml`, which names itself.

`ring_barrier`'s edge is the thinner of the two: one call, whose return value it
discards. `ring_shutdown`'s is two calls in two functions, one of which needs a
closure that mutates ([`api/002`](../api/002_the_predicate_is_the_parameter.md)).

### The In-Edge Set Is Asserted, By Another Crate

```rust
// ring_poll/src/lib.rs:80
pub const PARKING_CRATES : [ &str; 3 ] = [ "ring_barrier", "ring_shutdown", "ring_wait" ];
```

`ring_poll`'s reached-test reads every `ring_*/Cargo.toml`
on disk, collects the crates whose manifest text contains `ring_wait`, sorts
them, and asserts the result equals `PARKING_CRATES`
(`ring_poll/tests/poll_test.rs:76-114`). It then asserts separately that
none of `ring_poll`, `ring_handle`, `ring_core` is among them.

The comment on that test names the failure it exists for:

> Adding `ring_wait` to `ring_handle` — the exact mistake feature 183 exists to
> prevent, and one `ring_handle`'s own green suite would not notice — fails
> here.

So this crate's in-edge set is not merely documented, it is **pinned**: adding a
fourth dependent, or removing one of the two, fails a test in a crate that does
not depend on this one at all. That is unusual and it is deliberate — the
property being protected is *"the tick path cannot reach a parking operation"*,
which is a property of the graph, and a graph property has no single crate to
live in.

### WT13 — This Crate Is What the Tick Path Must Not Contain

`ring_handle` closes the half of the gap the roster cannot see. Its manifest is
clean, so `ring_poll`'s scan says nothing about a `std::thread::sleep` written
inline — `std` is not a manifest entry. So `ring_handle` scans its own source
for seven forbidden names
(`ring_handle/tests/handle_test.rs:493-494`):

```rust
const FORBIDDEN : [ &str; 7 ] =
[ "thread::sleep", "yield_now", "::park", "park(", "Condvar", "Duration", "Waker" ];
```

Pointed at `ring_wait`, that list is a description of the crate:

```sh
cd "$(git rev-parse --show-toplevel)"
code=$( sed 's|//.*||' ring_wait/src/lib.rs )
for name in 'thread::sleep' 'yield_now' '::park' 'park(' 'Condvar' 'Duration' 'Waker'; do
  printf '%s  %s\n' "$( printf '%s' "$code" | grep -cF -- "$name" )" "$name"
done
```

Live output:

```
1  thread::sleep
1  yield_now
0  ::park
0  park(
0  Condvar
1  Duration
0  Waker
```

| Forbidden name | In `ring_wait`'s **code** | In the file at all |
|----------------|--------------------------:|-------------------:|
| `thread::sleep` | 1 | 1 |
| `yield_now` | 1 | 1 |
| `Duration` | 1 | 1 |
| `::park` | 0 | **1** |
| `park(` | 0 | 0 |
| `Condvar` | 0 | 0 |
| `Waker` | 0 | 0 |

Three of the seven are in the code, and a fourth is in the comment W4 requires
to exist ([`decisions/001`](../decisions/001_park_sleeps_rather_than_parking.md) § WT20).
`ring_handle`'s guard strips comments before scanning, and its doc comment
records why: the first version scanned raw text for `park` and failed on the
module documentation explaining that no parking operation may be reachable.

The symmetry is the point. `ring_handle` is defined partly by the absence of
these names; `ring_wait` is the crate that has them. The two tests — one on the
graph, one on the text — are the same rule enforced from opposite ends.

### WT12 — Four Bounded Retry Loops, Only One of Them a Strategy

The ban has a cost, and it is duplication:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r "spin_loop" */src/*.rs
```

Live output:

```
ring_poll/src/lib.rs:          core::hint::spin_loop();
ring_poll/src/lib.rs:      core::hint::spin_loop();
ring_poll/src/lib.rs:      core::hint::spin_loop();
ring_publish/src/lib.rs://! [`Publisher::publish`] loops on a `spin_loop` hint with no wait strategy and
ring_publish/src/lib.rs:      core::hint::spin_loop();
ring_wait/src/lib.rs:        core::hint::spin_loop();
```

| Site | Crate | Bounded by | May depend on `ring_wait` |
|------|-------|-----------|:-------------------------:|
| `ring_wait:123` | `ring_wait` | `spins` | — |
| `ring_poll:311` | `ring_poll` | `Budget` | **no** — its own test forbids it |
| `ring_poll:380` | `ring_poll` | `Budget` | no |
| `ring_poll:423` | `ring_poll` | `Budget` | no |
| `ring_publish:197` | `ring_publish` | **nothing** — a bare `loop` | not banned, argued against |

Three of `ring_poll`'s helpers open-code the shape this crate exists to provide
— a counted retry with a pause hint between attempts — and its own comment at
`:309-310` says why the hint is all it may emit:

> A pause hint, and nothing more. Yielding here would be the parking this crate
> exists to keep off the tick path — see `docs/invariant/001`.

The observation worth stating plainly: **the ban is on the crate, and the crate
is the only granularity that can be enforced at build time.** The family does
have a per-variant tick-safety predicate —
`WaitKind::is_non_blocking` (`ring_types/src/policy.rs`), surfaced
as `RingConfig::is_tick_safe` (`ring_config/src/lib.rs`) — but it
classifies a *value* at runtime, so nothing stops a caller who has `ring_wait` in
scope from passing `Park`. A manifest that does not name the crate needs no one
to remember a check. `ring_poll` pays for that with three copies of a five-line
loop.

### WT23 — The Barred Crate Performs the Barred Operation

The predicate and the duplication point in opposite directions, and it is worth
naming which one is odd.

`is_non_blocking` is true for `None` **only** — `Spin` is classified alongside
`Park`:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A7 -F '  pub const fn is_non_blocking( self ) -> bool' ring_types/src/policy.rs
echo '  -- every file outside ring_wait naming either predicate, and how often --'
# No -n, and `command grep` rather than the shell's ugrep shim, which the second
# call used bare and which returns matches in completion order rather than a
# stable one.  The claim here is which crates reach for the predicate, never
# which line they reach for it on.  The earlier form printed `grep -rn` output
# spanning crates this one does not own, so an edit anywhere above a match --
# in ring_types, in ring_config, anywhere -- made this block stale while every
# line of it remained true.
command grep -rc --include='*.rs' -E 'is_tick_safe|is_non_blocking' */ \
  | command grep -v ':0$' | command grep -v '^ring_wait/' | LC_ALL=C sort
```

Live output:

```
  pub const fn is_non_blocking( self ) -> bool
  {
    match self
    {
      Self::None => true,
      Self::Spin | Self::Yield | Self::Park => false,
    }
  }
  -- every file outside ring_wait naming either predicate, and how often --
ring_config/src/lib.rs:4
ring_config/tests/config_test.rs:4
ring_types/src/policy.rs:5
ring_types/tests/types_test.rs:2
```

Two crates, four files. `ring_types` declares the predicate and asserts it;
`ring_config` surfaces it and asserts the surfacing. Nothing else in the family
consults either one.

Yet `ring_poll` — the crate the ban exists to protect — emits
`core::hint::spin_loop()` on its own tick path three times over
(`:311`, `:380`, `:423`), inside bounded retry loops. That is the same operation
`pause`'s `Spin` arm performs, differing only in multiplicity: `ring_poll` emits
one hint per attempt, `pause` emits one to eight
([`pitfall/002`](../pitfall/002_the_backoff_that_resets_every_eight_attempts.md)).

| | `ring_poll`'s own loops | `WaitKind::Spin` |
|--|------------------------|------------------|
| Operation | `core::hint::spin_loop()` | `core::hint::spin_loop()` |
| Hints per attempt | 1 | 1–8 |
| Bounded by | `Budget` | `spins` |
| `is_tick_safe` would say | — (not a `WaitKind`) | **false** |

So the family's classifier would reject a `WaitKind` describing exactly what
`ring_poll` already does. The two are not in contradiction — `is_non_blocking`
asks *"does this return after one look?"*, not *"does this sleep?"*, and `Spin`
genuinely does loop.

But the documentation does not say that. It says the other thing
(`ring_types/src/policy.rs:60-61`):

> Whether this strategy can be used on the tick path — true for exactly
> [`WaitKind::None`].

Read literally, that sentence says a bounded `Spin` cannot be used on the tick
path, and `ring_poll` — the crate the tick path belongs to — spins on it three
times. The predicate's *body* is `matches!( self, Self::None )`, which is an
honest name for `is_non_blocking` and an overreach for "can be used on the tick
path". The two readings only coincide if a caller is assumed to pass an unbounded
budget, and the whole crate exists to make that impossible.

`ring_publish` is the fourth site and a different argument again: it is not
banned from depending on this crate, and declines anyway
(`ring_publish/src/lib.rs:42-53`), because what it waits for is a
predecessor committed to finishing, so a budget would be wrong and a `Park`
"can only ever hurt".

### What This Crate Does Not Depend On

| Not a dependency | Although |
|------------------|----------|
| `ring_seqno` | its `pending` and `may_claim` are what the two wrappers ultimately call — through `CursorPair` |
| `ring_handle` | it owns the registration a real park would need ([`decisions/001`](../decisions/001_park_sleeps_rather_than_parking.md)) |
| `ring_stats` | `record_wait` takes nanoseconds this crate never measures |
| `ring_core` | nothing here knows what a ring is; it knows what a cursor pair is |

Two declared dependencies puts this crate in the family's lower third — two
crates declare none and ten declare one — but the real count on production paths
is smaller still. `ring_cursor`, one of the two, is named only by `for_space`
and `for_data`, and neither has a caller outside this crate's tests
([`api/001`](../api/001_seven_items_and_the_one_with_a_caller.md) § WT1). Every
line of this crate that anything in the family actually executes reaches
`ring_types` and nothing else.


### WT32 — The Composition `ring_claim` Prescribes Has Never Been Written

`ring_claim` deliberately does not wait. Its module documentation says so, and
tells the reader what to do instead.

```sh
cd "$(git rev-parse --show-toplevel)"
grep -B2 -A1 'The caller that wants to wait composes' ring_claim/src/lib.rs
# does ring_claim depend on the crate it names?
grep -c 'ring_wait' ring_claim/Cargo.toml || echo '0 — not a dependency'
# and has anyone written the composition it prescribes?
grep -r 'for_space(' --include=*.rs . --exclude-dir=docs \
  | grep -v '^ring_wait/' || echo '(nowhere in the family)'
# control: the identical expression for the function callers did reach
grep -r 'wait_until(' --include=*.rs . --exclude-dir=docs | grep -v '^ring_wait/'
```

Live output:

```
//! above it.
//!
//! The caller that wants to wait composes: `ring_wait::for_space`, then
//! [`Claimer::claim`].
0
0 — not a dependency
(nowhere in the family)
ring_shutdown/src/lib.rs:  ring_wait::wait_until( kind, spins, || shutdown.is_closed() )
ring_shutdown/src/lib.rs:  let outcome = ring_wait::wait_until( kind, spins, ||
ring_barrier/src/lib.rs:    ring_wait::wait_until( kind, spins, || self.admits( from, count ) )?;
```

`ring_claim/src/lib.rs:34` reads *"The caller that wants to wait composes:
`ring_wait::for_space`, then [`Claimer::claim`]."* — a two-step recipe naming the
exact function.

`ring_claim` does not depend on `ring_wait`, which is correct: the sentence
addresses the *caller*, not this crate. Pushing the wait outward is the reason
`WaitKind::None` is implementable above `ring_claim` at all, stated two lines
earlier. The prescription is sound and the non-dependency is the design.

The measurement is that `for_space(` has no call site anywhere in the family. The
one crate that wanted this shape — `ring_shutdown` — needed a second exit
condition, found `for_space`'s fixed predicate had nowhere to put it, and rewrote
the body over `wait_until` (WT3). So the recommended composition has one
documented recipe, one crate that tried to use it, and zero instances in the
tree.

That is the sharpest available evidence for what
[`api/002`](../api/002_the_predicate_is_the_parameter.md) argues structurally: a
helper that fixes its own predicate is a helper for callers who want exactly that
predicate, and the first real caller wanted one more condition.

### Integrations

| File | Relationship |
|------|--------------|
| [002_the_wrapper_that_had_to_be_rewritten.md](002_the_wrapper_that_had_to_be_rewritten.md) | The heavier of the two in-edges, in full |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_seven_items_and_the_one_with_a_caller.md](../api/001_seven_items_and_the_one_with_a_caller.md) | The three call sites these two edges carry |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_park_sleeps_rather_than_parking.md](../decisions/001_park_sleeps_rather_than_parking.md) | The arm that makes this crate a parking crate |
| [../decisions/002_the_discriminants_live_in_ring_types.md](../decisions/002_the_discriminants_live_in_ring_types.md) | Why `ring_types` is the other out-edge |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_a_crate_with_no_type_of_its_own.md](../data_structure/001_a_crate_with_no_type_of_its_own.md) | The three types the two out-edges supply |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_what_each_strategy_costs_per_attempt.md](../non_functional_requirement/002_what_each_strategy_costs_per_attempt.md) | The cost that makes the tick-path ban worth enforcing |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_the_predicate_the_pause_and_the_budget.md](../pattern/001_the_predicate_the_pause_and_the_budget.md) | The shape `ring_poll` re-implements three times |

### Workarounds

| File | Relationship |
|------|--------------|
| [../workaround/002_a_crate_name_as_a_load_bearing_string.md](../workaround/002_a_crate_name_as_a_load_bearing_string.md) | WT10 and WT11 — what the roster check can and cannot distinguish |

### Sources

| File | Relationship |
|------|--------------|
| `ring_poll/src/lib.rs:80,309-310` | `PARKING_CRATES`, and the comment on the open-coded pause |
| `ring_poll/tests/poll_test.rs:66-114` | The reached-test that pins this crate's in-edge set |
| `ring_handle/tests/handle_test.rs:462-513` | The seven forbidden names, and why comments are stripped |
| `ring_publish/src/lib.rs:42-53` | The crate that could depend on this one and argues against it |

### Tests

| File | Relationship |
|------|--------------|
| `tests/manual/readme.md` § W6 | Both declared dependencies are actually used |
| `tests/wait_test.rs:112-126` | The non-blocking property the tick-path ban ultimately protects |
