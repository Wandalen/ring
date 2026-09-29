# Data Structure Doc Definition

### Scope

- **Purpose**: Describe the two structures this crate defines — the policy value itself, and the flush log its acceptance criterion requires.
- **Responsibility**: Fix each structure's shape, its operations, and its cost.
- **In Scope**: The three-variant policy; the recorded flush log.
- **Out of Scope**: The buffer being flushed (→ [`ring_tls`](../../../ring_tls/docs/readme.md)); the ring receiving the drain.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [The Policy Enum](001_the_policy_enum.md) | Three variants, one of which carries a number, and why that asymmetry is load-bearing | 🔄 |
| 002 | [The Flush Log](002_the_flush_log.md) | A structure that exists only because the acceptance criterion is negative — and the danger of a test-only structure on a hot path | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_flush/docs/data_structure
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### FL[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| FL[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| FL9 | the cross-reference headings | n/a — inconsistency | Eight of this crate's instances carry a `### State Machines` heading and no crate in the family has a `state_machine/` definition; thirty-two have `lifecycle/` |
| FL10 | the width snippet | **wrong doc** | The snippet offered under "verify rather than trust" asserts a literal sixteen bytes and the assertion that shipped asserts two pointer widths |
| FL11 | the compilation-boundary section | n/a — drift | The section opens by retracting its own framing and keeps the bolded requirement that framing produced, so fifty-five lines argue for a boundary three lines say was never built |
| FL12 | the Tests table | n/a — coverage | Twenty-two of this crate's instances use a `| File |` Tests table the citation checker cannot parse and four use the `| Test |` form it reads, so most citations are checked by nothing |
