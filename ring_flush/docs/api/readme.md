# API Doc Definition

### Scope

- **Purpose**: Fix the two surfaces this crate exports — the policy a consumer configures, and the driver a consumer must call.
- **Responsibility**: State each surface's operations, error behaviour, and compatibility guarantees.
- **In Scope**: Policy construction and inspection; the drive/tick entry point.
- **Out of Scope**: When a consumer should call the driver, which is the consumer's schedule; the primitives beneath (→ [`ring_tls`](../../../ring_tls/docs/api/002_consolidator_read_surface.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [The Policy Surface](001_the_policy_surface.md) | What a consumer configures, and why it is a value rather than a trait | 🔄 |
| 002 | [The Driver Surface](002_the_driver_surface.md) | The call a consumer must make and this crate cannot make for itself — the API shape forced by [`pattern/002`](../pattern/002_driven_not_self_firing.md) | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_flush/docs/api
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### FL[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| FL[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| FL5 | Compatibility Guarantee 3 | **wrong doc** | Guarantee 3 promises what the export surface confines, and `ring_bench` breached it and recorded the breach in its own manifest rather than against the guarantee |
| FL6 | Guarantee 1 | n/a — inconsistency | Guarantee 1 rules the `#[non_exhaustive]` question closed, `decisions/readme.md` records it open as P3, and `integration/002`'s X1 states it as an unresolved disjunction |
| FL7 | the no-Result claim | **misleading doc** | The claim that no operation on this surface returns `Result` is written seventy lines below the constructor row that returns one |
| FL8 | the deferral row | n/a — drift | The row deferring to an open decision points at P5, which closed by measurement and now has two tests pinning its answer |
