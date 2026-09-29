# Algorithm Doc Definition

### Scope

- **Purpose**: Give the two procedures this crate owns — deciding whether to flush, and sequencing the three primitives `ring_tls` exposes when the answer is yes.
- **Responsibility**: State each procedure's steps, its inputs, and where it is driven from.
- **In Scope**: Policy evaluation; seal/drain/reset sequencing.
- **Out of Scope**: The primitives' own mechanics, which are `ring_tls`'s; the ring-side claim, which is `ring_batch`'s.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Evaluating a Policy at an Append](001_evaluating_a_policy_at_an_append.md) | The hot-path half — three comparisons, no allocation, no atomic, and one branch that must stay predictable | 🔄 |
| 002 | [Sequencing Seal, Drain and Reset](002_sequencing_seal_drain_reset.md) | The cold-path half — the cycle `ring_tls` deliberately did not fuse into one call, and why the order is not free | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_flush/docs/algorithm
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### FL[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| FL[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| FL1 | the refusal path | **latent hazard** | The path this procedure takes when the ring refuses is documented as returning the record to the caller, and `ring_tls::push` early-returns before pushing, so the record is dropped |
| FL2 | the ordering criterion | n/a — unenforced | The publication-ordering criterion this sequence is written against lives in a feature the instance does not cite, and the crate that owns it records its own half as unenforced |
| FL3 | O6 | n/a — coverage | O6 is not merely untested in this crate: the tick-path guard it defers to names only three crates, so thirty of the family are unexamined by it |
| FL4 | `run` | n/a — doc gap | `run`'s comment says the shortfall requires violating this crate's contract, and that contract is stated nowhere but inside `run` itself |
