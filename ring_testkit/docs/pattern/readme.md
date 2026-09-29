# Pattern Doc Definition

### Scope

- **Purpose**: Name the two design choices the crate is built on — a scenario encoded as inspectable data, and a fixture that returns a verdict rather than making one — and account for what each buys, what it costs, and which of its affordances anything actually uses.
- **Responsibility**: The patterns, their participants, the alternatives they were chosen over, and the price each shape pays for being that shape.
- **In Scope**: `Step`/`Script`/`run` as instruction-program-interpreter; `Outcome`/`audit`/`Anomaly` as the returned-verdict surface.
- **Out of Scope**: The step loop as a procedure (→ [`algorithm/`](../algorithm/readme.md)); the laws the returned value must satisfy (→ [`invariant/`](../invariant/readme.md)); a workaround forced by a tool rather than chosen (→ [`workaround/`](../workaround/readme.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [The Script Is Data And The Run Is An Interpreter](001_the_script_is_data_and_the_run_is_an_interpreter.md) | Four properties the encoding buys, two costs it pays, and where the interpreter merges | 🔄 |
| 002 | [The Fixture Returns A Verdict Instead Of Making One](002_the_fixture_returns_a_verdict_instead_of_making_one.md) | Four affordances the returning form needs, why it was right here, and the coordinate the verdict cannot carry | 🔄 |

**The split is input and output.** `001` is about what a caller hands the
fixture: an ordered value with ten instruction kinds, inspectable before it
runs, interpreted by one exhaustive `match`. `002` is about what the fixture
hands back: a value with ten fields, a packaged judgement, and an error type
that prints — and no assertion anywhere in `src/`.

They are two halves of one decision and are apart because their costs land in
different places. `001`'s costs are internal: three redundant variants and three
copies of one collapse line. `002`'s land at the caller: an affordance nothing
has walked end to end, and a diagnostic that names records but never steps.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit/docs/pattern
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### TK[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| TK[0-9]+ ' readme.md )"
printf 'properties 001 claims:    %s\n' "$( command grep -cE '^\| P[0-9] \| ' 001_*.md )"
printf 'costs 001 records:        %s\n' "$( command grep -cE '^\| Q[0-9] \| ' 001_*.md )"
printf 'affordances 002 claims:   %s\n' "$( command grep -cE '^\| R[0-9] \| ' 002_*.md )"
printf 'reasons 002 records:      %s\n' "$( command grep -cE '^\| S[0-9] \| ' 002_*.md )"
```

Live output:

```
instances:                2
finding headings inside:  4
rows in the table below:  4
properties 001 claims:    4
costs 001 records:        2
affordances 002 claims:   4
reasons 002 records:      4
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TK37 | the singular/`Many` variant pairs | n/a — duplication | Three of `Step`'s ten variants are the `n = 1` case of another three, costing three public variants, three verbatim copies of `let count = if let Step::XMany( n ) = *step { n } else { 1 };`, and one test whose sole job is to assert the pairs are equivalent — with no doc comment or decision anywhere weighing the ergonomic argument that would justify them. |
| TK38 | what the interpreter reports | n/a — observation | `run` returns `Outcome` and never `Result`, and none of its ten fields is a step index, so a `Flush` over an empty buffer, a `RecvMany( 100 )` on an empty ring and a `Reopen` on an open ring each produce an outcome byte-identical to that step's absence — the pattern makes the input fully inspectable and the execution entirely opaque, and does not say so. |
| TK39 | the `Error` impl's reachability | n/a — unadopted | `impl core::error::Error for Anomaly {}` exists so a caller can `?` an anomaly out of a test, as the crate's own test comment states; zero of the 36 test functions across both files return a `Result`, and no crate outside depends on `ring_testkit`, so the affordance is compiled and printed but never walked end to end. |
| TK40 | what a failing verdict names | n/a — diagnostics | Zero of `Anomaly`'s four `Display` arms and zero of `Outcome`'s ten fields name a step, so a failed audit reports a record number against a script the caller must replay by hand — cheap for the crate's own three-to-five-step scripts, and unchanged for a script of any length the pattern permits. |
