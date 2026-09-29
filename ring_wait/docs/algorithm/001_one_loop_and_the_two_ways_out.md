# Algorithm: One Loop and the Two Ways Out

### Scope

- **Purpose**: Trace `wait_until`'s thirteen-line body — the only repetition in the crate — and account for each of its three exits, including the one that is not written down.
- **Responsibility**: State what the loop reads, what it returns, what it clamps, and why the give-up is at the loop header rather than in the body.
- **In Scope**: `wait_until`, and the `wait` wrapper that supplies its budget.
- **Out of Scope**: What happens *inside* the pause — see [`item/001`](../item/001_the_four_arms_of_the_pause.md).

### The Whole Body

```rust
// ring_wait/src/lib.rs:179-195
pub fn wait_until< F >( kind : WaitKind, spins : usize, mut ready : F ) -> Result< usize, RingError >
where
  F : FnMut() -> bool,
{
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
```

Thirteen lines including braces. Every other function in the crate is either one
line over this one or a `match` with no repetition in it at all.

### Three Exits, Two Values

| # | Exit | Line | Returns | Reached when |
|---|------|------|---------|--------------|
| 1 | `return Ok( attempt )` | `:187` | the zero-based index of the look that succeeded | `ready()` answered true |
| 2 | `break` → `Err( Empty )` | `:191`, `:194` | `RingError::Empty` | `pause` answered false — only `WaitKind::None` does |
| 3 | loop falls through → `Err( Empty )` | `:193`, `:194` | `RingError::Empty` | the range ran out |

Exits 2 and 3 return the same value from the same line, which is why the
function looks like it has two endings and has three. The distinction is not
observable to a caller and is deliberately not made observable: *"gave up
because you told me not to wait"* and *"gave up because the budget ran out"* are
the same instruction to the caller — stop asking, decide something.

`Ok`'s payload is the interesting half. It is **the number of pauses that
happened**, not the number of looks: a predicate ready on the first look returns
`Ok( 0 )` having paused zero times. `tests/wait_test.rs:179-191` states the
arithmetic in its own assertion message — *"zero-based: ready on look 5 is 4
pauses"*.

### The Clamp Nobody Asked For

`spins.max( 1 )` at `:183` is the whole of the crate's input validation, and it
turns a zero budget into a single look rather than an immediate failure:

```
probe: wait_until( Spin, 0, || { looks += 1; false } )
       spins=0 produced 1 look(s)
```

The alternative — `0..spins`, so a zero budget produces zero looks — returns
`Err( Empty )` about a ring **nobody ever read**. That answer is not merely
unhelpful, it is false: the ring may well have had data. `tests/wait_test.rs:206-221`
pins the behaviour and its own comment gives that reason.

The clamp is at the *loop header*, which matters for a reason the next section
makes concrete.

### WT17 — The Same Clamp, Written Twice, in Two Places

`ring_poll::Budget` enforces the identical rule and enforces it somewhere else:

```rust
// ring_poll/src/lib.rs:137-140
pub const fn new( attempts : usize ) -> Self
{
  if attempts == 0 { Self( 1 ) } else { Self( attempts ) }
}
```

| | `ring_wait` | `ring_poll` |
|--|-------------|-------------|
| Rule | zero attempts means one attempt | identical |
| Site | the loop header, `:183` | the constructor, `:139` |
| Observable at | call time only | construction time — `Budget::new( 0 ).attempts() == 1` |
| Doctest asserting it | none | `ring_poll/src/lib.rs:113` |

`ring_poll`'s placement is the better one: a clamped `Budget` can be *inspected*,
so a caller that computed a zero budget by accident can find out before it waits.
`wait_until`'s clamp is invisible — `spins` is consumed, never returned, and the
`Ok( attempt )` a one-look success produces is `0`, which is also what a
hundred-budget first-look success produces.

The two are not shared code and cannot be: `ring_poll` may not depend on this
crate at all ([`integration/001`](../integration/001_two_dependencies_two_dependents_and_a_roster.md)).
So the family carries one rule in two implementations by construction, and the
only thing keeping them in agreement is that both are one line.

### Why the Budget Is in the Header and the Give-Up Is Not

W1 (`tests/manual/readme.md`) checks a structural property rather than a
behavioural one:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -vE "^[[:space:]]*//" ring_wait/src/lib.rs \
  | grep -E "^[[:space:]]*loop[[:space:]]*(\{|$)|while[[:space:]]+true" \
  || echo '(no unbounded loop form in this crate)'
# control: the identical expression over ring_publish, whose `publish` is one
grep -vE "^[[:space:]]*//" ring_publish/src/lib.rs \
  | grep -E "^[[:space:]]*loop[[:space:]]*(\{|$)|while[[:space:]]+true"
```

Live output:

```
(no unbounded loop form in this crate)
    loop
```

No match here, and the control below it returns `ring_publish`'s `loop` line, so
the empty result is a measured absence rather than a pattern that cannot fire.
Every repetition in the crate is a `for` over a counted range, so the
budget lives where it cannot be forgotten. The alternative shape — `loop { … if
attempt >= spins { break } … }` — is behaviourally identical and one refactor
away from unbounded, and a test cannot tell the difference between a bounded
loop and an unbounded one except by timing out.

`ring_publish` is the family's counter-example and it is deliberate: its
`publish` is a bare `loop { }` (`ring_publish/src/lib.rs:200-210`), correct
there because what it waits for — a predecessor finishing a slot write it has
already committed to — cannot fail to arrive. Its module documentation argues
the distinction directly (`ring_publish/src/lib.rs:42-53`). Waiting for
*space* depends on a consumer that may be stalled or gone; waiting for a
committed predecessor does not.

### What the Loop Does Not Do

| Not done | Consequence |
|----------|-------------|
| No deadline | The same call takes the same number of samples on every machine, and a different amount of *time* on each ([`data_structure/002`](../data_structure/002_the_budget_and_the_attempt_index.md)) |
| No re-read of `kind` | The strategy is fixed for the whole wait; `escalation_hint` exists for a caller that wants to change it, and nothing calls it ([`lifecycle/002`](../lifecycle/002_the_escalation_ladder_nobody_climbs.md)) |
| No memory ordering of its own | The loads happen inside the caller's closure, at whatever ordering the closure uses ([`non_functional_requirement/001`](../non_functional_requirement/001_two_atomic_loads_for_every_look.md)) |
| No panic | `:176-178` states it: the budget is a `usize` count and the loop is bounded by it |


### WT25 — The Success Value Is Documented Everywhere Except in the Function

`wait_until` returns `Result< usize, RingError >`. The `usize` is load-bearing —
it is the only thing distinguishing "ready on the first look" from "ready after a
thousand" — and the function's rustdoc never says what it counts.

```sh
cd "$(git rev-parse --show-toplevel)"
# every rustdoc section header wait_until declares
command grep -m1 -A52 -F '/// Ask `ready` until it answers true, pausing per `kind` between askings, for' ring_wait/src/lib.rs | grep '^/// # ' || echo '(no sections)'
# what the doctest asserts about the value
command grep -m1 -B3 -A16 -F '/// yet", a producer as "no room yet", and neither is this crate'"'"'s business.' ring_wait/src/lib.rs | grep 'assert'
# does any crate in the family write a `# Returns` section to hang it on?
grep -l '# Returns' ring_*/src/lib.rs || echo '(no crate in the family uses one)'
```

Live output:

```
/// # Errors
/// # Panics
/// # Errors
/// assert!( outcome.is_ok() );
/// assert_eq!( looks, 3, "stopped as soon as it was ready" );
(no crate in the family uses one)
```

The declared sections are `# Errors` and `# Panics`. There is no `# Returns`, and
no crate in the family writes one — so the omission is house style rather than an
oversight in this crate alone, which is precisely why it goes unnoticed here where
it costs something.

The doctest is the other place a reader would look. It asserts `outcome.is_ok()`
and `looks == 3` — the *closure's own counter*, incremented inside the predicate,
never the returned number. A reader running the example learns that the predicate
was asked three times and learns nothing about what came back.

The value is `looks - 1`: the count of pauses that happened, not the count of
looks that did. `tests/wait_test.rs:219` pins it — `assert_eq!( outcome, Ok( 0 ) )`
against `assert_eq!( looks, 1 )` on the same call — and
[`data_structure/002`](../data_structure/002_the_budget_and_the_attempt_index.md)
states it in prose. Both are outside the rustdoc. A caller reading only
`cargo doc` has the type and no interpretation of it, and the off-by-one between
the two plausible readings is exactly the size of the mistake that produces.

### Algorithms

| File | Relationship |
|------|--------------|
| [002_two_wrappers_over_a_predicate_they_fix.md](002_two_wrappers_over_a_predicate_they_fix.md) | The two one-line functions built on this loop |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_seven_items_and_the_one_with_a_caller.md](../api/001_seven_items_and_the_one_with_a_caller.md) | Where this function sits in the surface, and who calls it |
| [../api/002_the_predicate_is_the_parameter.md](../api/002_the_predicate_is_the_parameter.md) | Why `FnMut`, and what the closure is allowed to do |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_budget_and_the_attempt_index.md](../data_structure/002_the_budget_and_the_attempt_index.md) | The two integers this loop threads |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_every_repetition_is_a_counted_for.md](../invariant/001_every_repetition_is_a_counted_for.md) | W1, as a property rather than a reading |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_four_arms_of_the_pause.md](../item/001_the_four_arms_of_the_pause.md) | What `pause` does between two looks |
| [../item/002_the_loop_the_wrapper_and_the_two_questions.md](../item/002_the_loop_the_wrapper_and_the_two_questions.md) | This function's contract and coverage |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_a_wait_from_the_first_look_to_one_of_two_endings.md](../lifecycle/001_a_wait_from_the_first_look_to_one_of_two_endings.md) | The same three exits, as states |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_the_predicate_the_pause_and_the_budget.md](../pattern/001_the_predicate_the_pause_and_the_budget.md) | The three-part shape this loop is the family's only instance of |

### Sources

| File | Relationship |
|------|--------------|
| `ring_poll/src/lib.rs:137-140` | The same clamp, at the constructor |
| `ring_publish/src/lib.rs:42-53,200-210` | The family's one deliberate unbounded loop, with its argument |

### Tests

| File | Relationship |
|------|--------------|
| `tests/wait_test.rs:179-191` | `Ok( attempt )` is a pause count, zero-based |
| `tests/wait_test.rs:193-204` | The loop stops asking the moment it is ready |
| `tests/wait_test.rs:206-221` | A zero budget still looks once |
| `tests/wait_test.rs:223-237` | `for budget in 1..8usize` — budgets 1 through 7, each producing exactly that many looks |
| `tests/manual/readme.md` § W1 | No `loop {` and no `while true` anywhere in the crate |
