# Pattern Doc Definition

### Scope

- **Purpose**: Name the two practices this crate is built from — moving a decision from call sites into a value, and separating deciding from firing.
- **Responsibility**: State each pattern's problem, solution, applicability, and consequences.
- **In Scope**: Policy-as-value; driven-not-self-firing.
- **Out of Scope**: The specific policies (→ [`type/`](../type/readme.md)); the sequencing (→ [`algorithm/`](../algorithm/readme.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Policy as a Value, Not a Call Site](001_policy_as_a_value.md) | The practice whose absence the flush policy's "If Missing" clause describes, stated positively | 🔄 |
| 002 | [Driven, Not Self-Firing](002_driven_not_self_firing.md) | The structural consequence of this crate's dependency list — it can decide, and cannot act unprompted | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_flush/docs/pattern
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### FL[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| FL[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| FL37 | the comparison table | **misleading doc** | "Exactly one, by construction" holds on the single-producer backend and is a convention on the other two, where `try_clone` hands out a second producer |
| FL38 | the applicability boundary | n/a — doc gap | The table's sharpest boundary — do not make a correctness requirement configurable — is crossed by the enum it was written for, since `OnBarrier` is an obligation and the other two are trades |
| FL39 | the closure concession | **wrong doc** | The concession that `ring_barrier` is in the transitive closure is false: eighteen crates are reachable and it is not among them, nor named anywhere in this crate |
| FL40 | R5 | n/a — diagnostics | R5 calls the forgotten-driver failure indistinguishable from an idle buffer, and `staged()` is a public accessor returning exactly the number that distinguishes them |
