# API Doc Definition

### Scope

- **Purpose**: Record the two surfaces this crate offers — running candidates and rendering the result — and the argument type a Contract-bound consumer could not otherwise name.
- **Responsibility**: State each operation's signature, its failure modes, and what it deliberately does not return.
- **In Scope**: `run`, `Comparison::run`, the accessors on `Outcome` and `Comparison`, and `report`.
- **Out of Scope**: The internal runners, which are private (→ [`algorithm/001`](../algorithm/001_one_workload_through_six_runners.md)); the candidates' own APIs.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [The Run Surface](001_the_run_surface.md) | One fallible function per candidate, one infallible one for all of them, and where the refusals go | 🔄 |
| 002 | [The Report Surface](002_the_report_surface.md) | A `String` a human reads, which states its absences rather than omitting them | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench/docs/api
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### BN[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| BN[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BN5 | the Compatibility Guarantee's accounting claim | n/a — coverage | "Accounted for exactly once" is checked as `outcomes.len() + refusals.len() == ALL.len()`, which holds at any split and is therefore weaker than the sentence it carries |
| BN6 | the unreachable `Flush` variant, and the test named for it | n/a — coverage | `RunError::Flush` is documented as unreachable because the staging buffer's capacity and the flush trigger are the same number, and the test named for that tie compares two numbers neither side reads |
| BN7 | `Comparison::report` | **misleading doc** | `fastest()` filters before it minimises and `report()` does not filter at all, so the eligibility rule the crate's ranking depends on reaches only the report's last line |
| BN8 | `Comparison::report`'s header line | **latent hazard** | The header reads two of its five fields through different paths, so it can print a batch and a capacity that `RingConfig` would refuse to construct together |
