# Pattern: The Script Is Data And The Run Is An Interpreter

### Scope

- **Purpose**: Name the design choice the crate is built on — a test scenario encoded as an inspectable value rather than a closure — and account for what it buys and what it costs.
- **Responsibility**: The pattern, its three participants, the alternative it was chosen over, and the two places the encoding pays a price for its own shape.
- **In Scope**: `Step` as data, `Script` as a builder over it, `Script::run` as the single interpreter.
- **Out of Scope**: The step loop as a procedure (→ [`../algorithm/001`](../algorithm/001_from_a_step_to_an_outcome.md)); what the resulting value must satisfy (→ [`../invariant/001`](../invariant/001_every_minted_record_is_somewhere.md)).

### The pattern

| Participant | Role | Realized as |
|---|---|---|
| Instruction | One operation, with its multiplicity, and nothing else | `pub enum Step` — 10 variants, 3 carrying a `usize` |
| Program | An ordered sequence of instructions, plus the one parameter the sequence cannot carry | `pub struct Script` — a `Vec< Step >` and a `stage_limit` |
| Interpreter | The single place that gives instructions meaning | `Script::run` — one `match` over `*step`, 7 arms, no wildcard |

The alternative is a closure: a fixture that takes `impl FnMut( &mut Ring< u32 > )`
and calls it. That is shorter to write and strictly more expressive, and the
crate does not do it.

### What the encoding buys

| # | Property | Why the closure form cannot have it |
|---|---|---|
| P1 | A scenario is a value: `Debug`, `Clone`, `PartialEq`, `Eq` | A closure has no equality and prints as an opaque address |
| P2 | A scenario can be inspected before it runs — `steps()`, `stage_limit()` | A closure's body is not available to its caller |
| P3 | The same scenario runs against two rings with no risk of shared capture | `run` takes `&self`; a `FnMut` closure carries state between calls by design |
| P4 | The interpreter is exhaustive over instructions | A closure can do anything, so nothing can be checked about what it does |

P3 is the one the crate's central requirement rests on. `Script::run` is
`&self`, so `script.run( &mut a )` and `script.run( &mut b )` cannot influence
each other through the script — which is exactly what
[`../non_functional_requirement/001`](../non_functional_requirement/001_two_runs_compare_equal.md)
asserts and what a `FnMut` would quietly break.

P4 is why `match *step` has no wildcard arm. Adding a variant to `Step` breaks
the build at one site, which is the point of encoding instructions as a closed
enum rather than as strings or as a trait object.

### What the encoding costs

Two costs, both paid in `src/lib.rs` and both visible in the counts:

| # | Cost | Size |
|---|---|---|
| Q1 | Multiplicity is encoded twice — once as a variant, once as a payload | 3 of 10 variants are the `n = 1` case of another 3 → TK37 |
| Q2 | The interpreter reports nothing about which instructions did anything | `run` returns `Outcome`, never `Result`; no field is a step index → TK38 |

Q1 is a shape problem inside the enum. Q2 is a shape problem at its boundary:
the pattern makes the *input* fully inspectable and leaves the *execution*
entirely opaque.

### Where the interpreter merges

Seven arms cover ten variants, so three arms take two variants each:

| Arm | Covers | How multiplicity is recovered |
|---|---|---|
| 1 | `Push`, `PushMany( _ )` | `let count = if let Step::PushMany( n ) = *step { n } else { 1 };` |
| 2 | `Recv`, `RecvMany( _ )` | `let count = if let Step::RecvMany( n ) = *step { n } else { 1 };` |
| 3 | `Stage`, `StageMany( _ )` | `let count = if let Step::StageMany( n ) = *step { n } else { 1 };` |
| 4–7 | `Flush`, `Close`, `Reopen`, `DrainAll` | no multiplicity to recover |

The three merge arms each re-destructure a value the arm pattern has already
matched. That is the direct consequence of Q1, and it is the same three lines
written three times.

### Evidence

| # | Claim | Test |
|---|---|---|
| P-E1 | A script reports the steps and limit it was built from | `a_script_reports_what_it_was_built_from` |
| P-E2 | The singular and `Many`-of-one forms are the same instruction | `the_single_record_steps_match_a_many_of_one` |
| P-E3 | One script against two rings gives equal outcomes | `one_script_run_twice_produces_equal_outcomes` |
| P-E4 | A script with no steps is a run, not an error | `an_empty_script_produces_an_empty_outcome` |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit
printf 'Step variants:               %s\n' "$( awk '/^pub enum Step/{f=1} f && /^  [A-Z]/{n++} f && /^}$/{exit} END{print n+0}' src/lib.rs )"
printf 'of those, carrying a usize:  %s\n' "$( awk '/^pub enum Step/{f=1} f && /^}$/{exit} f' src/lib.rs | command grep -cE '^  [A-Z][a-zA-Z]+\( usize \),' || true )"
printf 'arms in the interpreter:     %s\n' "$( awk '/pub fn run\( &self/{f=1} f && /^  }$/{exit} f' src/lib.rs | command grep -cE '^ *Step::.*=>' || true )"
printf 'wildcard arms:               %s\n' "$( awk '/pub fn run\( &self/{f=1} f && /^  }$/{exit} f' src/lib.rs | command grep -cE '^ *_ *=>' || true )"
printf 'copies of the collapse line: %s\n' "$( awk '/pub fn run\( &self/{f=1} f && /^  }$/{exit} f' src/lib.rs | command grep -c 'if let Step::' || true )"
printf 'the collapse line, verbatim: %s\n' "$( awk '/pub fn run\( &self/{f=1} f && /if let Step::/{ sub( /^ */, "" ); print; exit }' src/lib.rs )"
printf 'run takes self how:          %s\n' "$( command grep -m1 -oE 'pub fn run\( &self[^)]*\) -> [A-Za-z]+' src/lib.rs )"
printf 'then takes self how:         %s\n' "$( command grep -m1 -oE 'pub fn then\( [a-z ]*self[^)]*\) -> [A-Za-z]+' src/lib.rs )"
printf 'what Script and Step derive: %s\n' "$( command grep -B1 -E '^pub (enum Step|struct Script)' src/lib.rs | command grep -oE 'derive\( [^)]*\)' | tr '\n' ' ' )"
printf 'Result in the run signature: %s\n' "$( command grep -m1 -c 'pub fn run.*Result' src/lib.rs || true )"
printf 'Outcome fields:              %s\n' "$( awk '/^pub struct Outcome/{f=1} f && /^}$/{exit} f' src/lib.rs | command grep -cE '^  pub ' || true )"
printf 'of those naming a step:      %s\n' "$( awk '/^pub struct Outcome/{f=1} f && /^}$/{exit} f' src/lib.rs | command grep -E '^  pub ' | command grep -cE 'step|index' || true )"
printf 'P-E rows above:              %s\n' "$( command grep -c '^| P-E[0-9]' docs/pattern/001_the_script_is_data_and_the_run_is_an_interpreter.md )"
printf 'of those, tests that exist:  %s\n' "$( awk -F'|' '/^\| P-E[0-9]/ { gsub( /[^a-z_]/, "", $4 ); print $4 }' docs/pattern/001_the_script_is_data_and_the_run_is_an_interpreter.md | while read -r n ; do command grep -q "fn $n" tests/testkit_test.rs && echo x ; done | wc -l )"
```

Live output:

```
Step variants:               10
of those, carrying a usize:  3
arms in the interpreter:     7
wildcard arms:               0
copies of the collapse line: 3
the collapse line, verbatim: let count = if let Step::PushMany( n ) = *step { n } else { 1 };
run takes self how:          pub fn run( &self, ring : &mut Ring< u32 > ) -> Outcome
then takes self how:         pub fn then( mut self, step : Step ) -> Self
what Script and Step derive: derive( Debug, Clone, Copy, PartialEq, Eq ) derive( Debug, Clone, PartialEq, Eq ) 
Result in the run signature: 0
Outcome fields:              10
of those naming a step:      0
P-E rows above:              4
of those, tests that exist:  4
```

### Patterns

| File | Relationship |
|------|--------------|
| [002_the_fixture_returns_a_verdict_instead_of_making_one.md](002_the_fixture_returns_a_verdict_instead_of_making_one.md) | The other half of the choice — what the interpreter hands back |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_from_a_step_to_an_outcome.md](../algorithm/001_from_a_step_to_an_outcome.md) | The same loop as a procedure rather than as a design choice |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_script_surface.md](../api/001_the_script_surface.md) | What `new`, `then`, `steps`, `stage_limit` and `run` each promise |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_outcome_and_anomaly.md](../type/001_outcome_and_anomaly.md) | The value the interpreter returns |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_two_runs_compare_equal.md](../non_functional_requirement/001_two_runs_compare_equal.md) | The requirement P3 exists to make possible |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | `Step`, `Script`, and the interpreter |

### Tests

| File | Relationship |
|------|--------------|
| `tests/testkit_test.rs` | P-E1 through P-E4 |

### TK37 — three of ten instructions are the one-case of another three

`Step` has ten variants. Three of them — `Push`, `Recv`, `Stage` — are exactly
the `n = 1` case of `PushMany( 1 )`, `RecvMany( 1 )` and `StageMany( 1 )`, and
the crate has a test whose entire job is to assert that they are:
`the_single_record_steps_match_a_many_of_one` builds two three-step scripts, one
of each form, and compares their `Outcome`s for equality.

The redundancy costs three things. Three extra variants on a public enum a
consumer matches. Three arms in the interpreter that merge a pair and then
re-destructure it — `let count = if let Step::PushMany( n ) = *step { n } else
{ 1 };`, written verbatim three times with one name changed. And one test that
exists only to pin an equivalence a seven-variant enum would not have needed to
state.

`Push( usize )` alone would remove all three. The ergonomic argument for keeping
both is real — `Step::Push` reads better than `Step::Push( 1 )` in a script
literal, and a fixture's scripts are read more than they are written. But the
argument is nowhere on record: no doc comment, decision or note in this crate
weighs the two forms, so what is currently visible is three duplicated variants,
three duplicated lines, and a test asserting the duplication is faithful.

The scale is small and it does not grow. What makes it worth a finding is that
this is the crate's central data type, and a consumer reading the enum has no
way to tell whether the pairs are a deliberate ergonomic choice or an unfinished
refactor.

### TK38 — the pattern makes the input inspectable and the execution opaque

`Script::run` returns `Outcome` and never `Result`. There is no failure channel,
which is correct — a refused push is data, not an error, and the whole point of
the accounting is that refusals are counted rather than raised.

The consequence is elsewhere. `Outcome`'s ten fields carry counts and two lists;
none is a step index, a per-step record, or any other trace of which
instructions did anything. So several distinct scripts produce byte-identical
outcomes:

- `Flush` with an empty staging buffer, and no `Flush` step at all
- `RecvMany( 100 )` on an empty ring, `RecvMany( 0 )`, and no `Recv` step — the
  arm breaks on the first empty read
- `Reopen` on an already-open ring, and no `Reopen` step
  (→ [`../lifecycle/001`](../lifecycle/001_the_states_a_script_moves_through.md))

This is a real asymmetry in the pattern, not a bug. P2 gives a caller full
inspection of the program *before* it runs — `steps()` returns the exact `&[
Step ]` — and the interpreter returns a summary that cannot be attributed back
to any of them. A scenario is data going in and an aggregate coming out.

It matters most where the fixture is most useful. When an audit fails, the
`Anomaly` names records by value and the `Outcome` names totals; neither names a
step, so localizing the failure means re-reading the script and reasoning
forward by hand
(→ [`002`](002_the_fixture_returns_a_verdict_instead_of_making_one.md) TK40).
The pattern chose an inspectable input over an inspectable execution, and the
choice is unrecorded.
