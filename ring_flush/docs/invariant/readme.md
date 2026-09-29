# Invariant Doc Definition

### Scope

- **Purpose**: State the two restrictions that make a flush policy worth having — that it fires only at its trigger, and that the publication point is designed rather than inherited from call-site order.
- **Responsibility**: Fix each restriction, its enforcement mechanism, and what breaks when it is violated.
- **In Scope**: Trigger exclusivity; publication-point ownership.
- **Out of Scope**: The measurable acceptance criteria (→ [`non_functional_requirement/`](../non_functional_requirement/readme.md)); how a policy is evaluated (→ [`algorithm/`](../algorithm/readme.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [A Policy Fires Only at Its Trigger](001_a_policy_fires_only_at_its_trigger.md) | The negative half of the acceptance criterion — "and at no other point" — and why it is the half nothing observes | 🔄 |
| 002 | [The Publication Point Is Designed, Not Inherited](002_publication_point_is_designed_not_inherited.md) | The flush policy's "If Missing" clause turned into a standing restriction: order must not become a property of which code was written first | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_flush/docs/invariant
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### FL[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| FL[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| FL21 | the V2 deletion | n/a — drift | The reasoning deleting V2 rests on a fault injection run against a suite of twenty-three tests, and the suite now has thirty-six |
| FL22 | the ownership half-close | n/a — coverage | Seven of this crate's instances record the ownership half-close and the two rows about that hole are not among them |
| FL23 | the P5 gate | n/a — unenforced | The gate directory grew from six gates to twenty-two and the one gate this invariant specified is still absent; no gate names `ring_tls` |
| FL24 | C5 | **misleading doc** | C5 is written as a hazard and the method it describes is public, documented, and exercised fourteen times in its own crate's tests |
