# Pattern: The Fixture Returns A Verdict Instead Of Making One

### Scope

- **Purpose**: Name the second design choice the crate is built on — a fixture that hands back a value for the caller to judge, rather than asserting on the caller's behalf — and check what the choice is worth given who currently uses it.
- **Responsibility**: The pattern, the four affordances it requires, which of them are exercised, and the coordinate the returned verdict cannot carry.
- **In Scope**: `Outcome` as a returned value, `Outcome::audit` and `audit_received` as `Result`-returning checks, `Anomaly`'s `Display` and `Error` impls as the reporting surface.
- **Out of Scope**: What the audit checks (→ [`../invariant/001`](../invariant/001_every_minted_record_is_somewhere.md)); how the value is produced (→ [`001`](001_the_script_is_data_and_the_run_is_an_interpreter.md)).

### The pattern

A fixture can do one of two things when a run is over. It can assert — call
`assert_eq!` on what it observed and panic if the observation is wrong — or it
can return what it observed and let the caller decide. This crate does the
second, without exception: **zero** `assert`, `unwrap`, `expect` or `panic!` in
`src/lib.rs` outside doc comments.

Four affordances are needed to make the returning form usable, and the crate
provides all four:

| # | Affordance | Realized as |
|---|---|---|
| R1 | The observation is a value the caller can inspect field by field | `pub struct Outcome` — 10 public fields, no accessors needed |
| R2 | The common judgement is packaged, so every caller does not re-derive it | `Outcome::audit` and `audit_received`, both `-> Result< (), Anomaly >` |
| R3 | A failed judgement explains itself in text | `impl Display for Anomaly` — four arms, each naming the numbers involved |
| R4 | A failed judgement composes with the caller's own error handling | `impl core::error::Error for Anomaly {}` |

R2 is what keeps the pattern from degenerating. A fixture that returns raw data
and nothing else pushes the whole judgement onto every caller, who will each
write it slightly differently. Packaging the judgement as a `Result` gives the
caller the choice — inspect the fields, or take the verdict — without taking the
choice away.

### Why the returning form was the right call here

| # | Reason | What asserting would have cost |
|---|---|---|
| S1 | A refusal is data, not a failure | `refused_full : 3` is the *expected* result of pushing six records into four slots; an asserting fixture would need to be told which refusals are wanted |
| S2 | Two runs must be comparable | `assert_eq!( first, again )` is the caller's line, and it needs two values to compare — an asserting fixture produces none |
| S3 | A failure inside the fixture is indistinguishable from a failure in the code under test | A panic raised four crates below the caller's `#[ test ]` is a diagnosis problem (→ [`../non_functional_requirement/002`](../non_functional_requirement/002_what_a_fixture_owes_its_consumers.md) F1) |
| S4 | The loom half has no `Outcome` at all | `audit_received` takes a slice and a number precisely so a model closure with no accounting can still use R2 |

S2 is the load-bearing one. [`../non_functional_requirement/001`](../non_functional_requirement/001_two_runs_compare_equal.md)'s
entire requirement is that two runs compare equal, and a fixture that asserted
internally could not state that requirement, let alone test it.

### What the verdict cannot say

`Anomaly`'s four `Display` arms name records and totals:

| Variant | Message shape |
|---|---|
| `Unaccounted` | *"{minted} records were minted but only {placed} are accounted for"* |
| `Unminted` | *"record {value} came out of a run that minted only {minted}"* |
| `Overdelivered` | *"{out} records left a ring that accepted only {accepted}"* |
| `OutOfOrder` | *"record {then} came out after {previous}"* |

None names a step. Neither does `Outcome`, whose ten fields are all counts, two
lists and a flag. So a failing audit tells a caller *what* is wrong about the
result and nothing about *where in the script* it went wrong → TK40.

### Evidence

| # | Claim | Test |
|---|---|---|
| R-E1 | Every `Anomaly` variant renders a message naming its numbers | `every_anomaly_says_what_broke` |
| R-E2 | An `Anomaly` can be boxed as a `dyn Error` and still print | `every_anomaly_says_what_broke`, its last three lines |
| R-E3 | An outcome that adds up is still checked for its records | `an_outcome_that_adds_up_is_still_checked_for_its_records` |
| R-E4 | An outcome that lost a record fails the audit | `an_outcome_that_lost_a_record_fails_the_audit` |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit
printf 'assert/panic in src, non-doc:  %s\n' "$( command grep -vE '^\s*//' src/lib.rs | command grep -cE 'assert|unwrap\(|expect\(|panic!' || true )"
printf 'Outcome public fields:         %s\n' "$( awk '/^pub struct Outcome/{f=1} f && /^}$/{exit} f' src/lib.rs | command grep -cE '^  pub ' || true )"
printf 'functions returning Result:    %s\n' "$( command grep -cE '^ *pub fn .*-> Result<' src/lib.rs || true )"
printf 'Display arms:                  %s\n' "$( awk '/impl core::fmt::Display for Anomaly/{f=1} f && /^}$/{exit} f' src/lib.rs | command grep -c 'write!' || true )"
printf 'the Error impl, in full:       %s\n' "$( command grep -m1 'impl core::error::Error' src/lib.rs )"
printf 'Display messages naming a step: %s\n' "$( awk '/impl core::fmt::Display for Anomaly/{f=1} f && /^}$/{exit} f' src/lib.rs | command grep -c 'step' || true )"
printf 'Outcome fields naming a step:  %s\n' "$( awk '/^pub struct Outcome/{f=1} f && /^}$/{exit} f' src/lib.rs | command grep -E '^  pub ' | command grep -cE 'step|index' || true )"
printf 'tests that box an Anomaly:     %s\n' "$( command grep -c 'dyn core::error::Error' tests/testkit_test.rs || true )"
printf 'tests returning Result:        %s\n' "$( cat tests/*.rs | command grep -cE 'fn [a-z_]+\( *\) *-> *Result' || true )"
printf 'to_string call sites in tests: %s\n' "$( cat tests/*.rs | command grep -c 'to_string()' || true )"
printf 'R-E rows above:                %s\n' "$( command grep -c '^| R-E[0-9]' docs/pattern/002_the_fixture_returns_a_verdict_instead_of_making_one.md )"
printf 'distinct tests they name:      %s\n' "$( awk -F'|' '/^\| R-E[0-9]/ { gsub( /[^a-z_]/, "", $4 ); print $4 }' docs/pattern/002_the_fixture_returns_a_verdict_instead_of_making_one.md | sort -u | while read -r n ; do command grep -q "fn $n" tests/testkit_test.rs && echo x ; done | wc -l )"
```

Live output:

```
assert/panic in src, non-doc:  0
Outcome public fields:         10
functions returning Result:    3
Display arms:                  4
the Error impl, in full:       impl core::error::Error for Anomaly {}
Display messages naming a step: 0
Outcome fields naming a step:  0
tests that box an Anomaly:     1
tests returning Result:        0
to_string call sites in tests: 5
R-E rows above:                4
distinct tests they name:      3
```

### Patterns

| File | Relationship |
|------|--------------|
| [001_the_script_is_data_and_the_run_is_an_interpreter.md](001_the_script_is_data_and_the_run_is_an_interpreter.md) | How the value this file is about gets produced |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_outcome_and_anomaly.md](../type/001_outcome_and_anomaly.md) | `Outcome`'s ten fields and `Anomaly`'s four variants |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_every_minted_record_is_somewhere.md](../invariant/001_every_minted_record_is_somewhere.md) | The judgement R2 packages |
| [../invariant/002_the_shape_a_delivered_list_must_have.md](../invariant/002_the_shape_a_delivered_list_must_have.md) | S4's free function, and why it takes a slice |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_two_runs_compare_equal.md](../non_functional_requirement/001_two_runs_compare_equal.md) | The requirement S2 makes statable |
| [../non_functional_requirement/002_what_a_fixture_owes_its_consumers.md](../non_functional_requirement/002_what_a_fixture_owes_its_consumers.md) | S3 as an obligation, and the fact that nothing states it |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | R1–R4 and every count above |

### Tests

| File | Relationship |
|------|--------------|
| `tests/testkit_test.rs` | R-E1 through R-E4 |

### TK39 — the affordance that makes the verdict composable has no caller in either direction

R4 is `impl core::error::Error for Anomaly {}` — an empty impl whose only purpose
is to let an `Anomaly` participate in ordinary error handling. The crate's own
test says so explicitly, in a comment: *"The trait is implemented, so a caller
can `?` an anomaly out of a test."*

Using `?` on a `Result< (), Anomaly >` requires the enclosing function to return
a `Result`. Across both test files, the number of test functions that return a
`Result` is **zero**. Every one of the 36 tests returns `()` and reaches its
verdict through `assert_eq!` or `assert!`.

So the affordance is implemented, and the single test that touches it
(`every_anomaly_says_what_broke`) exercises it by boxing an `Anomaly` into a
`Box< dyn core::error::Error >` and checking the string is non-empty — proving
the impl compiles and prints, which is not the same as proving anyone can use it
for what it is for. Outside the crate the count is the same for a different
reason: no crate depends on `ring_testkit` at all
(→ [`../api/002`](../api/002_the_surface_no_crate_has_taken.md) TK7).

This is not an argument for removing the impl. A testkit's error type
implementing `Error` is the correct default, it costs one line, and the first
consumer that wants `?` will need it already present. It is an argument for
knowing what R-E2 establishes: that the trait is implemented, not that the
pattern it enables has ever been walked end to end.

### TK40 — a failing verdict names the record and never the step

The pattern's whole value is that the caller judges. When the judgement comes
back negative, what the caller gets is an `Anomaly` whose `Display` names
numbers — *"record 5 came out after 7"* — and an `Outcome` whose ten fields are
counts, two lists and a flag.

Neither carries a step. Zero `Display` messages mention one, and zero of the
ten fields is a step index. A script is an ordered sequence of up to ten kinds
of instruction, and a failed audit says nothing about which of them produced the
record it is complaining about.

For the crate's own suite this costs little: its scripts are three to five steps
long and a reader reconstructs the sequence by eye. It costs more at the two
places the fixture is supposed to be worth something. A consumer debugging their
own ring implementation gets a record number and a script they must replay
mentally. And a *long* script — which the pattern explicitly supports, since
`then` composes without bound — produces exactly the same one-line message as a
one-step script.

The fix is not obviously worth its cost. Threading a step index into `Outcome`
would put a field on the type whose determinism is the crate's central
requirement, and threading it into `Anomaly` would add a payload field to
variants callers already match on — which `#[ non_exhaustive ]` does not make
additive, since it reserves room for new variants, not for new fields on
existing ones
(→ [`../item/002`](../item/002_what_the_crate_does_not_declare.md) TK27). Both
are real changes with real costs.

What is missing is that the tradeoff is nowhere on record. The crate documents
what the verdict *says* in three places and never observes what it cannot say,
so a reader meets the limitation for the first time while debugging — which is
the worst moment to discover a diagnostic's shape.
