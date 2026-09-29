# Non-Functional Requirement Doc Definition

### Scope

- **Purpose**: State what the crate must cost and must not do, and separately audit what actually enforces each of those statements.
- **Responsibility**: The constraints, their measurement commands and current readings; and per constraint, the guard named for it, the thing that runs that guard, and how often.
- **In Scope**: Dependency direction; per-call cost; allocation; the absence of implicit invocation; the enforcement route behind each of C1–C5 and Q1–Q4.
- **Out of Scope**: Why a hot-path check would be the wrong trade (→ [`pitfall/`](../pitfall/readme.md)); what the checks do (→ [`api/`](../api/readme.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Absent Unless Called](001_absent_unless_called.md) | The five constraints and four open questions, each with a command that measures it | 🔄 |
| 002 | [The Constraints With No Number](002_the_constraints_with_no_number.md) | What executes each of those commands, and which constraints have no number and no route to one | 🔄 |

**The split is the claim against its enforcement, and they were measured
separately because they disagree.** `001` names a guard beside every constraint;
`002` asks, for each one, what runs it and when. Two of the five are re-measured by
something that runs unprompted, two by hand, and one by nothing at all — and the
one `001` flags as having *no* mechanical guard turns out to be the one a manual
stage in this same crate has been grepping for since the crate was built.

Keeping them in one file would let the guard column read as the enforcement.
Naming a command is not running it, and the difference is only visible when the
two are written down apart from each other.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_debug/docs/non_functional_requirement
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### DB[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| DB[0-9]+ ' readme.md )"
```

Live output:

```
instances:                2
finding headings inside:  4
rows in the table below:  4
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| DB45 | the cost model | **misleading doc** | The stated per-call cost counts the two acquire loads issued by the three entry points a family caller cannot reach, and none of the loads issued by the one it can — which goes through a backend-dispatching enum and reads each cursor twice, at two different orderings, in a crate this one does not name. |
| DB46 | the bash fence | n/a — unenforced | The block holding the guards for the two constraints the requirement calls mechanically checkable is fenced `bash`, and the corpus recipe checker executes `sh` fences only — so the crate's own enforcement commands are the one recipe in its docs that nothing has ever run. |
| DB47 | Q4's "no mechanical guard" | **misleading doc** | The requirement records one open question as having nothing that greps for it, while stage M3 of the manual plan in the same crate runs exactly that grep and its Run Record carries a dated passing result. |
| DB48 | the two claims with no number | n/a — doc gap | Both deferred figures were sent to a benchmark feature that does not name this crate, from a crate that no manifest in the family depends on, so neither claim has a route to a number and nothing is scheduled to give it one. |
