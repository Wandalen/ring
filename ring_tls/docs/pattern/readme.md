# Pattern Doc Definition

### Scope

- **Purpose**: State the two usage rules this crate's mechanism requires but cannot enforce — one about ordering setup correctly, one about how the crate composes into a full write path.
- **Responsibility**: Give each a problem, a solution, its applicability, and its consequences, so that a consumer following them is not reverse-engineering them from the invariants.
- **In Scope**: The register-before-append ordering rule; the staging-then-merge composition with the ring crates.
- **Out of Scope**: The mechanism the rules protect (→ [`algorithm/`](../algorithm/readme.md), [`invariant/`](../invariant/readme.md)); the traps the crate's vocabulary invites, which are traps rather than rules (→ [`pitfall/`](../pitfall/readme.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Register Before First Append](001_register_before_first_append.md) | The ordering rule that closes the silent-loss window a lazily-initialized thread-local opens | 🔄 |
| 002 | [Staging Then Merge](002_staging_then_merge.md) | How this crate composes with a ring into the two-stage write path — the composition this crate's staging exists to feed — and why it is not a ring competitor | 🔄 |




### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_tls/docs/pattern
printf 'instances:                '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### TL[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| TL[0-9]+ ' readme.md
# instances:                2
# finding headings inside:  2
# rows in the table below:  2
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TL43 | the ordering rule | **misleading doc** | "Register before first append" closes a silent-loss window that cannot open when construction is the only entry point. |
| TL44 | the staging pattern | n/a — observation | Stage locally, merge under one atomic — true of the byte region and true of `flush_into`, with the merge in a different crate. |
