# Algorithm Doc Definition

### Scope

- **Purpose**: Record the two procedures this crate performs — driving one workload description through six different write paths, and choosing a winner from the results without letting a discarding path win.
- **Responsibility**: State each procedure's steps, its complexity, and the properties it must preserve.
- **In Scope**: The per-candidate run loop and where the clock sits within it; the eligibility filter and its ordering relative to the comparison.
- **Out of Scope**: The candidates' own internal algorithms, which belong to their crates; the report's formatting, which is [`api/002`](../api/002_the_report_surface.md).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [One Workload Through Six Runners](001_one_workload_through_six_runners.md) | The shared shape every candidate is driven by, and the three phases only one of which is timed | 🔄 |
| 002 | [The Eligibility Filter Runs Before the Comparison](002_the_eligibility_filter_runs_before_the_comparison.md) | Filter on losslessness, then minimise on time — never the reverse, and never both at once | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench/docs/algorithm
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### BN[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| BN[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BN1 | `Comparison::conserved` | n/a — coverage | `commit_batch` increments its own counter only on a successful push, so `reported` and `received` are the same number by construction and two of the suite's `conserved()` assertions cannot fail for the one candidate everything else is ranked against |
| BN2 | `Workload::records_of` | n/a — coverage | The crate's one guarantee about what a record contains is bypassed by two of the six runners, which build the range inline rather than calling it, and asserted by no test either way |
| BN3 | `Comparison::fastest`'s eligibility filter | n/a — observation | The whole filter is `received == offered`, so a candidate is rankable only on a workload where nothing was refused, and the crate can therefore rank the uncontended case only |
| BN4 | `Comparison::fastest`'s eligibility filter | n/a — coverage | Every fixture in the crate is all-eligible or all-ineligible — 256 into 4096, 1024 into 4096, 256 into 16 — and both `fastest()` doctests repeat the same split, so the filter has never dropped one candidate while keeping another |
