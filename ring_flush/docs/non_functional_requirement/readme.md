# Non-Functional Requirement Doc Definition

### Scope

- **Purpose**: Carry the flush policy's binary Reached condition and the cost ceiling that makes the policy usable on the append path.
- **Responsibility**: State each requirement, its measurement method, and its acceptance threshold.
- **In Scope**: The three-trigger criterion and its flush log; the per-append cost of deciding.
- **Out of Scope**: The restrictions themselves (→ [`invariant/`](../invariant/readme.md)); the batch-size-vs-tail-latency trade being tuned.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Three Triggers, Proven by a Recorded Flush Log](001_three_triggers_proven_by_a_flush_log.md) | The acceptance criterion, and the observability mechanism it forces into existence | 🔄 |
| 002 | [The Decision Costs Nothing on the Append Path](002_the_decision_costs_nothing_on_the_append_path.md) | The unstated requirement — a policy consulted per append must not undo the append path's zero-atomic property | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_flush/docs/non_functional_requirement
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### FL[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| FL[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| FL33 | the citation rule | **misleading doc** | The gate said to record a crate-to-feature edge searches every family crate's tests with no file-type filter, so prose satisfies it and the crate is not part of the tuple |
| FL34 | M4 | n/a — unenforced | M4's threshold cannot be missed: the crate's only `FlushEntry` literal derives the entry from the outcome, so the agreement it measures is guaranteed by construction |
| FL35 | the cycle-check block | n/a — coverage | The block introduced by "confirm it rather than taking it on trust" is fenced so the gate never runs it, and describes a scratch workspace nothing creates |
| FL36 | the unsafe allowlist | n/a — drift | This instance and the proxy test it produced both cite a four-name unsafe allowlist that was later replaced with three, precisely because the four were inert |
