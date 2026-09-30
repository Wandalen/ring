# Pattern: The Predicate, the Pause, and the Budget

### Scope

- **Purpose**: Name the three-part shape this crate exists to provide, find every instance of it in the family, and record which parts each instance keeps.
- **Responsibility**: State the pattern, show the four instances side by side, and say what the three copies pay for not being the original.
- **In Scope**: Bounded-retry-with-a-pause loops across the `ring_*` family.
- **Out of Scope**: The dependency ban that causes the copies — see [`integration/001`](../integration/001_two_dependencies_two_dependents_and_a_roster.md).

### The Shape

Three parts, and each answers a different question:

| Part | Question | In `wait_until` |
|------|----------|-----------------|
| **predicate** | is it ready yet? | `ready : F where F : FnMut() -> bool` |
| **pause** | what do I do between two askings? | `pause( kind, attempt )` |
| **budget** | when do I stop asking? | `for attempt in 0..spins.max( 1 )` |

Drop any one and the shape becomes something else. Without a predicate it is a
sleep. Without a pause it is a busy loop. Without a budget it is a hang waiting
to happen — which is exactly the crate's stated stake (`:41-46`).

The value of naming it is that the three parts are *independently* substitutable,
and the family substitutes each of them somewhere.

### Four Instances

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r "spin_loop" */src/*.rs
```

Live output:

```
ring_poll/src/lib.rs:                    core::hint::spin_loop();
ring_poll/src/lib.rs:            core::hint::spin_loop();
ring_poll/src/lib.rs:            core::hint::spin_loop();
ring_publish/src/lib.rs://! [`Publisher::publish`] loops on a `spin_loop` hint with no wait strategy and
ring_publish/src/lib.rs:            core::hint::spin_loop();
ring_wait/src/lib.rs:                core::hint::spin_loop();
```

| # | Site | Predicate | Pause | Budget |
|---|------|-----------|-------|--------|
| 1 | `ring_wait:183-193` | caller's closure | `pause`, four strategies | `spins`, caller's |
| 2 | `ring_poll:288-316` | `producer.try_push( held )` | `spin_loop()`, fixed | `Budget` |
| 3 | `ring_poll:367-383` | `producer.try_push_batch( records )` | `spin_loop()`, fixed | `Budget` |
| 4 | `ring_poll:413-426` | `consumer.try_recv()` | `spin_loop()`, fixed | `Budget` |

Instance 1 is the only one where all three parts are parameters. In the other
three, two of the three are welded in:

```rust
// ring_poll/src/lib.rs:413-426
let mut attempt = 0;
while attempt < budget.attempts()
{
  if let Some( record ) = consumer.try_recv()
  {
    return Some( record );
  }
  attempt += 1;
  if attempt < budget.attempts()
  {
    core::hint::spin_loop();
  }
}
None
```

Fourteen lines against `wait_until`'s thirteen, doing strictly less. The
difference is not incompetence — `ring_poll` may not depend on this crate at all,
and its own comment at `:309-310` says why the fixed pause is the only pause it
is allowed:

> A pause hint, and nothing more. Yielding here would be the parking this crate
> exists to keep off the tick path — see `docs/invariant/001`.

### What the Copies Pay

| | `wait_until` | `ring_poll`'s three |
|--|--------------|---------------------|
| Strategies available | 4 | 1 |
| Predicate | any `FnMut() -> bool` | one concrete call each |
| Budget clamped where | loop header, invisibly | `Budget::new`, inspectably |
| Zero-budget behaviour | one look | one look — same rule, [WT17](../algorithm/001_one_loop_and_the_two_ways_out.md) |
| Pause emitted on the final attempt | **yes** | **no** — guarded by `attempt < budget.attempts()` |
| Lines | 13 | 14, 15, 14 |

### WT24 — The Pause After the Last Look

The fifth row is a real behavioural difference and it favours the copies.
`wait_until` calls `pause` on every iteration including the final one, so a
failing wait pauses once for a look that will never happen:

```rust
for attempt in 0..spins.max( 1 )
{
  if ready() { return Ok( attempt ); }
  if !pause( kind, attempt ) { break; }   // runs on the last iteration too
}
```

`ring_poll` guards its hint and skips it: `if attempt < budget.attempts()`.

Measured, with a never-true predicate under `Park`:

| Budget | Looks | Elapsed | Pauses implied |
|-------:|------:|--------:|---------------:|
| 1 | 1 | 133.8 µs | **1** |
| 2 | 2 | 209.8 µs | 2 |
| 3 | 3 | 320.6 µs | 3 |

A budget of 1 sleeps once. There is no second look for that sleep to prepare
for, so it is pure loss — and it is the *whole* cost of that call, not a
rounding error on it.

The waste stays constant as the budget grows: one extra pause out of 1024 is
0.1% of a 122 ms `Park` wait, and invisible under `Spin`. So the impact is
negligible everywhere except the smallest budgets, where it is 100%. `wait( kind,
… )` at `DEFAULT_SPINS` will never notice; `for_space( pair, Park, 1 )` pays
115 µs to learn something it already knew after the first look.

Nothing tests for it in either crate. The fix is one comparison, and the reason
to record it rather than change it is that `pause`'s contract — *"do whatever
this strategy does between two readings"* (`:93`) — already says the final call
is outside its own remit.

**Disposition:** declined — this instance's own text states the reason to
record rather than fix it: `pause`'s contract ("do whatever this strategy
does between two readings", `src/lib.rs:93`) already places the final call
outside its own remit, so the extra pause is accepted behaviour, not a
defect to correct in a corpus disposition pass.


### The Fourth Part Nobody Has

`ring_publish` is the family's near-miss: predicate and pause, no budget.

```rust
// ring_publish/src/lib.rs:202-209
loop
{
  if let Ok( end ) = self.try_publish( start, len )
  {
    return end;
  }
  core::hint::spin_loop();
}
```

Its documentation argues the omission rather than overlooking it — and does so
under a `# Panics` heading, which is where a reader looking for the budget would
never think to check (`:185-188`):

> Never. The loop exits when the predecessor publishes, which it is committed to
> doing; a caller that publishes a range it never claimed deadlocks here instead,
> which is a caller bug this crate cannot detect — `try_publish` is the variant
> for a caller that wants to decide for itself.

That is the pattern's boundary condition, stated by the one crate outside it: a
budget is only meaningful when the thing being waited for might not arrive. For a
predecessor that has already committed, a budget would turn a guaranteed success
into a spurious failure ([`invariant/001`](../invariant/001_every_repetition_is_a_counted_for.md)).

The clause after the semicolon is the honest part: the loop *can* hang, just not
for the reason a budget would help with. A caller publishing a range it never
claimed waits forever, and `try_publish` — not a budget — is the escape offered.

### Why It Is Not Shared Code

The obvious consolidation — one generic bounded-retry helper, used by all four —
is blocked by a rule that has nothing to do with the shape:

| Consumer | Could use a shared helper | Blocked by |
|----------|:-------------------------:|-----------|
| `ring_wait` | it *is* the helper | — |
| `ring_poll` | no | the tick-path dependency ban |
| `ring_publish` | yes, technically | its own argument against a budget |

The ban is on the *crate*, because a manifest edge is the only thing a build can
check ([`integration/001`](../integration/001_two_dependencies_two_dependents_and_a_roster.md) § WT23).
A shared helper generic over the pause would have solved it — `ring_poll` could
instantiate it with `spin_loop` and never see a `WaitKind` — but that helper
would have to live in a crate below both, and the family has no such crate: this
one *is* the low crate, and it is the one carrying the forbidden names.

So four copies of a shape, three of them written out by hand, is the settled
answer rather than a backlog item. What it costs is that the zero-budget rule is
now written twice ([WT17](../algorithm/001_one_loop_and_the_two_ways_out.md)) and
the final-pause difference above went unnoticed in one of the two.


### WT44 — The Same Budget, Typed in One Crate and Bare in the Other

The three-part shape has four instances (WT12). The budget parameter is a
newtype in `ring_poll` and a bare `usize` here.

```sh
cd "$(git rev-parse --show-toplevel)"
grep 'spins : usize' ring_wait/src/lib.rs
grep 'pub struct Budget\|budget : Budget' ring_poll/src/lib.rs | head -4
```

Live output:

```
pub fn wait_until< F >( kind : WaitKind, spins : usize, mut ready : F ) -> Result< usize, RingError >
pub fn for_space( pair : &CursorPair, kind : WaitKind, spins : usize ) -> Result< usize, RingError >
pub fn for_data( pair : &CursorPair, count : u64, kind : WaitKind, spins : usize )
pub struct Budget( usize );
  budget : Budget,
  budget : Budget,
  budget : Budget,
```

`ring_wait` takes `spins : usize` on four of its six functions. `ring_poll` wraps
the same quantity in a `Budget` newtype with `once()`, `new()`, and a clamp in the
constructor.

The newtype buys `ring_poll` two things this crate does without. A `Budget` cannot
be swapped with the other `usize` in scope by argument-order mistake, and the
clamp-to-one lives in the constructor, so it is applied once at the boundary
rather than at every loop header. WT17 records the other half of that: this
crate's `spins.max( 1 )` is the same rule, written at the point of use because
there is no constructor to put it in.

Both choices are reasonable at their own scale — a newtype for a crate whose
budget is a tick-path safety parameter, a plain count for a crate whose budget is
one argument among four. What the pair shows is that the shape WT12 counts four
copies of is not four copies of one thing: two of the four disagree about whether
the budget is a domain type, and the disagreement is invisible because the crates
may not depend on each other.

### Patterns

| File | Relationship |
|------|--------------|
| [002_discriminants_here_handlers_there.md](002_discriminants_here_handlers_there.md) | The other pattern this crate is an instance of |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_one_loop_and_the_two_ways_out.md](../algorithm/001_one_loop_and_the_two_ways_out.md) | Instance 1, read line by line, with WT17 |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_predicate_is_the_parameter.md](../api/002_the_predicate_is_the_parameter.md) | The part that is a parameter here and welded in the copies |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_two_dependencies_two_dependents_and_a_roster.md](../integration/001_two_dependencies_two_dependents_and_a_roster.md) | WT12 and WT23 — why the copies exist |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_every_repetition_is_a_counted_for.md](../invariant/001_every_repetition_is_a_counted_for.md) | The budget part, as a property |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_four_arms_of_the_pause.md](../item/001_the_four_arms_of_the_pause.md) | The pause part, in its four forms |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_what_each_strategy_costs_per_attempt.md](../non_functional_requirement/002_what_each_strategy_costs_per_attempt.md) | What the wasted final pause costs, per strategy |

### Sources

| File | Relationship |
|------|--------------|
| `ring_poll/src/lib.rs:288-316,367-383,413-426` | The three open-coded instances |
| `ring_poll/src/lib.rs:84-113` | `Budget`, the part they substitute for `spins` |
| `ring_publish/src/lib.rs:183-210` | The near-miss, and its argument for having no budget |

### Tests

| File | Relationship |
|------|--------------|
| `tests/wait_test.rs:223-237` | The budget part, honoured exactly |
| `tests/wait_test.rs:193-204` | The predicate part — asking stops the moment it answers |
| `tests/wait_test.rs:69-84` | The pause part, across all four strategies |
| `tests/manual/readme.md` § W5 | The spin hint is inside a bounded `for`, not a bare one |
