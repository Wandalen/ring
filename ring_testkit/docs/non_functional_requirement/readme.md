# Non-Functional Requirement Doc Definition

### Scope

- **Purpose**: State the properties this crate must have to be a fixture at all — determinism, so its verdict means something, and the obligations it takes on by being depended upon in a consumer's test rather than in their program.
- **Responsibility**: The requirements, their enforcement, their measurement, and an honest account of what each measurement is and is not worth.
- **In Scope**: `Script::run`'s determinism; panic behavior, cfg neutrality, import cost and matching stability as consumer-visible properties.
- **Out of Scope**: What the signatures guarantee (→ [`api/`](../api/readme.md)); the laws a run must satisfy (→ [`invariant/`](../invariant/readme.md)), which are about a run's correctness rather than the fixture's fitness.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Two Runs Compare Equal](001_two_runs_compare_equal.md) | The determinism requirement, and the exact limit of what ten equal runs establish | 🔄 |
| 002 | [What A Fixture Owes Its Consumers](002_what_a_fixture_owes_its_consumers.md) | Four obligations that follow from being a testkit, and which two of them are met | 🔄 |

**The split is by whose property it is.** `001`'s subject is a property of the
fixture's own output: run the same script twice and the two `Outcome`s compare
equal. It can be stated, enforced and measured without reference to anyone
outside the crate. `002`'s four are properties of the fixture *as a dependency*
— they are unobservable from inside and become real only at a consumer.

That difference decides which is currently checkable. `001`'s requirement has
three automated tests behind it. `002`'s has two satisfied obligations, two
unmet ones, and no consumer to notice any of them
(→ [`../api/002`](../api/002_the_surface_no_crate_has_taken.md)).

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit/docs/non_functional_requirement
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### TK[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| TK[0-9]+ ' readme.md )"
printf 'C- and F- rows, all tables: %s\n' "$( command grep -hcE '^\| (C[0-9]|F[0-9]) \| ' [0-9][0-9][0-9]_*.md | paste -sd+ | bc )"
printf 'obligations met:          %s\n' "$( command grep -cE '^\| F[0-9] \| .* \| \*\*met' 002_what_a_fixture_owes_its_consumers.md || true )"
printf 'obligations unmet:        %s\n' "$( command grep -cE '^\| F[0-9] \| .* \| \*\*unmet\*\*' 002_what_a_fixture_owes_its_consumers.md || true )"
```

Live output:

```
instances:                2
finding headings inside:  4
rows in the table below:  4
C- and F- rows, all tables: 11
obligations met:          2
obligations unmet:        2
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TK33 | C2's ten-run script | n/a — coverage | C1 is measured on a five-step script over five `Step` variants where two refusal counters carry non-zero values; C2's ten repeats use a different, three-variant script with zero `Step::Push` occurrences, an eight-slot ring and a matching staging limit, so every refusal counter is structurally zero in the run that is actually repeated. |
| TK34 | E4's evidence form | n/a — unenforced | The section names E4 as *"the one that was not assumed"*, and E1–E3 are `#[ test ]` functions while E4 is `tests/manual/readme.md` M2 — a human procedure with one recorded pass and zero automated form in either test file; the manual plan independently calls M2 the stage it would be weakest without, and neither document arranges for it to run again. |
| TK35 | the crate's panic behavior | n/a — doc gap | `src/lib.rs` has zero `unwrap`, `expect` or `panic!` outside doc comments, which is the property that makes a fixture's verdict readable at all — and no `# Panics` section, module sentence, requirement row or test states it, so a consumer learns it by reading 741 lines and has no guarantee it survives the next change. |
| TK36 | the loom seam's reach | **latent hazard** | `ring_atomic` owns the family's only `--cfg loom` switch and states that *"no other crate needs to know the seam exists"*; `ring_testkit` reaches it through four crates, declares zero `cfg` attributes, and its own loom test records that the cursors *"panic if touched with no model running"* — so a consumer building under the cfg and calling `Script::run` outside a model panics from four crates down, and zero files in `src/` mention it. |
