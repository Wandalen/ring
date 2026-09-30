# Data Structure: The Script As A Flat Step List

### Scope

- **Purpose**: Define the shape a script is stored in — a flat, `Copy`, branch-free list plus one scalar — and what that shape buys and forbids.
- **Responsibility**: The layout, the two invariants it makes free, the expressiveness it gives up, and the one field that is not part of the list.
- **In Scope**: `Script`'s `Vec< Step >` and `stage_limit`; `Step`'s representation.
- **Out of Scope**: What each step *does* (→ [`algorithm/001`](../algorithm/001_from_a_step_to_an_outcome.md)); the values a run produces (→ [`002`](002_nine_counters_and_a_number_that_is_two_things.md)).

### Layout

```text
Script
├─ steps       : Vec< Step >   private
└─ stage_limit : usize         private

Step  —  ten variants, three carrying one usize each
         derive( Debug, Clone, Copy, PartialEq, Eq )
```

Two fields, both private, reached by `steps()` and `stage_limit()`. `Step` is
`Copy`, so the list is a flat block of plain data with no owned interior: cloning
a `Script` is one allocation and a memcpy, and `run` takes `&self` and mutates
nothing.

### What the flatness is for

| Property | Why the shape gives it |
|---|---|
| Two runs mint identical records | The list carries no data — records are consecutive `u32`s from `0`, so a script *is* its shape |
| A script can be compared | `Step : PartialEq + Eq` and `Vec : PartialEq` compose without a manual impl |
| A run cannot diverge on its input | There is no input: no closure, no seed, no iterator, nothing a caller could vary between two runs |
| `run` cannot be re-entered | The list is walked once by a `for` over a borrow; no step names another step |

**The list has no control flow.** There is no branch step, no loop step, no
conditional, and no step whose behaviour depends on an earlier step's result. A
script that wants "push until refused" cannot be written; a script that wants
"push eight times" can. That is the trade the determinism claim rests on — the
only thing that can make two runs differ is the ring, which is the variable the
fixture exists to measure.

### The field that is not a step

`stage_limit` sits beside the list rather than inside it. It is the capacity of
the `TlsBuffer` `run` allocates before walking the first step, so it is fixed for
the whole run and could not be a step without making the buffer resizable
mid-script.

It is also the one number in the type with **no relation to the ring**. A
`Script` does not know the capacity of the ring it will be run against, and
nothing checks the two against each other — a script with `stage_limit` of 8 run
against a ring of 2 is legal and produces a run where the staging buffer never
refuses and the ring always does.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit
printf 'Step variants:                   %s\n' "$( awk '/^pub enum Step$/{f=1} f && /^  [A-Z]/{n++} f && /^}$/{exit} END{print n}' src/lib.rs )"
printf 'variants carrying a payload:     %s\n' "$( awk '/^pub enum Step$/{f=1} f && /^  [A-Z][A-Za-z]*\(/{n++} f && /^}$/{exit} END{print n}' src/lib.rs )"
printf 'payload types used:              %s\n' "$( awk '/^pub enum Step$/{f=1} f && /^  [A-Z][A-Za-z]*\(/{ sub( /.*\( /, "" ); sub( / \).*/, "" ); print } f && /^}$/{exit}' src/lib.rs | sort -u | tr '\n' ' ' )"
printf 'Step derives:                    %s\n' "$( awk '/^pub enum Step$/{ print prev } { prev = $0 }' src/lib.rs )"
printf 'bounds checked on a Many count:  %s\n' "$( awk '/pub fn run/{f=1} f' src/lib.rs | command grep -cE 'count\.min\(|assert.*count|if count >' || true )"
printf 'Script::new arguments in use:    %s\n' "$( command grep -rhoE 'Script::new\( [0-9]+ \)' src tests | command grep -oE '[0-9]+' | sort -un | tr '\n' ' ' )"
printf 'Stage or Flush in the crate doc: %s\n' "$( awk '/^\/\/! use ring_testkit/,/^\/\/! ```$/' src/lib.rs | command grep -c 'Step::Stage\|Step::Flush' || true )"
printf 'where the absent ceiling is said: %s\n' "$( command grep -m1 -o '# The `Many` counts carry no ceiling' src/lib.rs )"
printf 'largest Many count in the suite:  %s\n' "$( command grep -rhoE 'Step::(Push|Recv|Stage)Many\( [0-9]+ \)' src tests | command grep -oE '[0-9]+' | sort -un | tail -1 )"
```

Live output:

```
Step variants:                   10
variants carrying a payload:     3
payload types used:              usize 
Step derives:                    #[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
bounds checked on a Many count:  0
Script::new arguments in use:    0 2 3 4 6 7 8 
Stage or Flush in the crate doc: 0
where the absent ceiling is said: # The `Many` counts carry no ceiling
largest Many count in the suite:  1000
```

### Types

| File | Relationship |
|------|--------------|
| [../type/001_outcome_and_anomaly.md](../type/001_outcome_and_anomaly.md) | `Step`'s variants defined one by one |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_from_a_step_to_an_outcome.md](../algorithm/001_from_a_step_to_an_outcome.md) | The walk over this list |

### Data Structures

| File | Relationship |
|------|--------------|
| [002_nine_counters_and_a_number_that_is_two_things.md](002_nine_counters_and_a_number_that_is_two_things.md) | The shape the walk produces |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_two_runs_compare_equal.md](../non_functional_requirement/001_two_runs_compare_equal.md) | The claim the flatness exists to support |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | `Step`, `Script` |

### Tests

| File | Relationship |
|------|--------------|
| `tests/testkit_test.rs` | `a_script_reports_what_it_was_built_from`, `an_empty_script_produces_an_empty_outcome`, `a_zero_slot_staging_buffer_refuses_every_record` |

### TK9 — the front-page example sizes a buffer it never stages into

The crate-level doc example opens:

```text
let script = Script::new( 4 )
  .then( Step::PushMany( 3 ) )
  .then( Step::RecvMany( 3 ) );

let config = RingConfig::new( 8 ).unwrap();
```

`Script::new`'s argument is the staging buffer's capacity, and the script
contains **no** `Stage`, `StageMany` or `Flush` step — the grep for them across
that block returns zero. So the `4` allocates a `TlsBuffer` that no step writes
to and that ends the run empty, and it is the first number a reader of this crate
ever sees.

Two lines below it, `RingConfig::new( 8 )` sets the capacity that actually
governs the run. A reader with no prior context has one plausible reading of two
adjacent constructors taking one integer each — that `4` is the ring and `8` is
something else — and nothing in the example says otherwise. The signature is
documented correctly on `Script::new` itself; the example is where the reader
arrives first.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit
command grep -F 'staging capacity — unused: no Stage/StageMany/Flush step below' src/lib.rs
```

Live output:

```
//! let script = Script::new(4) // staging capacity — unused: no Stage/StageMany/Flush step below
```

**Disposition:** applied — the crate-level doc example now carries an inline
comment on `Script::new( 4 )` naming it as the staging capacity and noting no
step below uses it, plus a matching comment on `RingConfig::new( 8 )` naming it
as the capacity that governs the run.
Now prints: `staging capacity — unused: no Stage/StageMany/Flush step below`

### TK10 — "bounded by a constant in the step itself"

`PushMany`, `RecvMany` and `StageMany` each carry an unchecked `usize`, and
`run` binds it with `let count = if let Step::PushMany( n ) = *step { n } else { 1 };`
— no `min`, no assertion, no comparison against the ring or the staging buffer.
The grep for a bound on `count` inside `run` returns zero.

[`algorithm/001`](../algorithm/001_from_a_step_to_an_outcome.md)'s Termination
section states that *"every step is bounded by a constant in the step itself or
by the buffer's own size"*. The constant is real, and it is supplied by the
caller with no ceiling: `Step::PushMany( usize::MAX )` is a legal step that
type-checks, builds, and does not terminate in any useful sense.

`RecvMany` is the exception that shows the shape of the gap — it breaks on the
first empty read, so its count is an upper bound rather than a loop trip count,
and it is the only one of the three that a too-large argument cannot hang. The
other two mint every record they are asked for.

Nothing in the family passed a large one; the suite's arguments were single
digits throughout. The hazard was that the termination argument read as a property
of the type and was a property of the values the suite happened to use.

**Disposition:** applied — as a stated absence and a test that reaches it, and the
ceiling itself is **declined**. `Step` now carries a
`# The `Many` counts carry no ceiling` doc section saying the count is a loop trip
count executed in full, that `Step::PushMany( usize::MAX )` type-checks and does
not finish, and that `RecvMany` is bounded by the ring rather than by its
argument. A clamp was the obvious fix and would have been the wrong one: the whole
purpose of this crate is measuring what a ring does when it is offered more than
it can hold, so a `count.min( capacity )` would silently delete the measurement
`refused_full` exists to take. `a_many_count_far_above_capacity_is_executed_in_full`
pins the behaviour instead — `PushMany( 1000 )` on a capacity-4 ring mints all
1000, the ring accepts 4 and refuses 996, and `audit()` calls that coherent, which
it is. What this does not buy: the termination argument in
[`algorithm/001`](../algorithm/001_from_a_step_to_an_outcome.md) is still written
as a property of the type, and no test can prove a non-terminating step
terminates, so the gap between what that section claims and what the type
guarantees is now documented in two places rather than closed. Now prints:
`largest Many count in the suite:  1000`
