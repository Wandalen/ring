---
name: new-gate
description: Add a stage gate to bench_harness (or any new check — CI job, lint, script) that can demonstrably fail, following the project's non-vacuity standard.
---

# New gate

Read first: `bench_harness/docs/invariant/001_gate_non_vacuity.md` and
`bench_harness/docs/guide/003_what_the_gates_do_not_prove.md`. A gate that cannot tell "not
started" from "finished" measures nothing.

## Design

1. State the property in one sentence, and the cheap proxy the gate will actually measure.
   Write down what a REACHED reading will **not** prove.
2. Pair the assertion with a non-vacuity check: something must be in scope, the tool must have
   run (check its exit code and its own output, not just an empty findings list), and every
   declared input must exist.
3. Decide the scope: one family (`declared/<family>/gates.txt`) or repo-wide
   (`declared/repo_gates.txt`, never both). Do not declare a gate for a family whose plan
   never committed to its property.

## Script

`bench_harness/gate/g<N>_<snake_name>.sh`, next free number (g23; g11 is retired).

```bash
#!/usr/bin/env bash
# G<N> — <what it asserts>, and <what proves it had something to measure>.
#
# <Why this is a gate; the finding that motivated it; its non-vacuity argument.>
set -uo pipefail
GATE=G<N>
source "$( dirname "${BASH_SOURCE[0]}" )/common.sh"
assert_declared_crates_exist

# … iterate with `family_crates` (stage-scoped) or `family_members`;
# run cargo through `cargo_over_workspaces`, never `cd "$REPO"`;
# temp files via mktemp + trap … EXIT.

pass "<evidence with counts>"   # or: fail "<reason naming the crates>"
```

- `set -uo pipefail`, never `-e`. Exactly one `pass` or `fail`; the pass line carries counts
  because it is the proof the gate ran.
- No positional arguments; scope comes from `GATE_FAMILY` / `GATE_CRATES` / `GATE_STAGE`. When
  a stage owns nothing this gate grades, pass with an explicit note.
- Later corrections go in `# Fix(<slug>):` blocks with root cause and pitfall.
- Corpus-style gate: a checker in `gate/corpus/` on `corpus_lib` (exit 0 clean, 1 findings,
  else crash) called through `run_corpus_checker`, plus `corpus_control/g<N>_<defect>/` and at
  least one `g<N>_<…>_clean/` fixture.

## Register and document

- Add the id with a rationale comment to `gates.txt` or `repo_gates.txt`.
- `bench_harness/gate/readme.md` table, `bench_harness/docs/guide/001` and `003` tables, `verb/gate` and
  `verb/readme.md` gate ranges.

## Prove it

1. Plant a violation (a probe file named `-<something>`, gitignored; or a temporary edit) →
   the gate reports NOT REACHED naming it.
2. Remove it → REACHED with counts.
3. Put both runs' output in the PR. That red → green pair is the definition of done.
