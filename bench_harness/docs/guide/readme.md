# Guide Doc Definition

### Scope

- **Purpose**: Teach this crate's outcomes to someone who did not build it: what was measured, what it established, how to reproduce every number, and what the machinery does not prove.
- **Responsibility**: Carry the reader from "33 crates exist" to "I can run the verdicts myself and I know what they are worth".
- **In Scope**: The inspection path (every command, in order); the four measured verdicts; the limits of the gates.
- **Out of Scope**: Per-crate design rationale (each crate's own `docs/`); the stage decomposition and its verdict log (the family's own implementation plan); the reached-test definitions (→ [`acceptance/001`](../acceptance/001_feature_reached_tests.md)).

### Why this lives in `bench_harness`

The guide spans all 33 `ring_*` crates, so it is leaf-proximate to none of
them. `bench_harness` is the one crate here that is family-neutral. It depends
on no `ring_*` crate, which is why its gates could run against an empty workspace
on day one. It already owns the reached-tests every verdict is graded against. A
guide to the outcomes belongs with the thing that produced them.

### Reading Order

The three instances are meant to be read in order, and each is useful alone.

| Read this | If you want |
|---|---|
| [001](001_running_the_verdicts_yourself.md) | To see it work. Every command, with the output to expect |
| [002](002_the_four_verdicts.md) | To know what was established, and how strongly |
| [003](003_what_the_gates_do_not_prove.md) | To know where to distrust it |

**003 is the one to read if you only read one.** It records the measurement that
showed 100% line coverage catching none of the family's headline defect, which
is the most transferable thing this effort produced.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Running the verdicts yourself](001_running_the_verdicts_yourself.md) | Six commands, in order, from a clean checkout to the full comparison | 🔄 |
| 002 | [The four verdicts](002_the_four_verdicts.md) | What the comparison established, with the evidence and the confidence for each | 🔄 |
| 003 | [What the gates do not prove](003_what_the_gates_do_not_prove.md) | Coverage is not defect detection, and the measurement that showed it | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
files=$( ls bench_harness/docs/guide/ | grep -v '^readme.md$' )
echo '  -- guide/ holds exactly the three instances the Reading Order above lists --'
printf '%s\n' "$files"
```

Live output:

```
  -- guide/ holds exactly the three instances the Reading Order above lists --
001_running_the_verdicts_yourself.md
002_the_four_verdicts.md
003_what_the_gates_do_not_prove.md
```
