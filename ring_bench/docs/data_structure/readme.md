# Data Structure Doc Definition

### Scope

- **Purpose**: Record the two records this crate defines — the description a run is driven by, and the result a run produces — and the trap each of them was shaped to close.
- **Responsibility**: State each record's fields, its invariants, and what a wrong shape would have cost.
- **In Scope**: `Workload`'s four fields and the two of them called "producers"; `Outcome`'s three counts and why they are not interchangeable.
- **Out of Scope**: `RingConfig` itself, which is `ring_config`'s; `RingStats`, which is `ring_stats`'.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [The Workload Description](001_the_workload_description.md) | Four fields, two of which are the same number under two names, tied so they cannot diverge | 🔄 |
| 002 | [Three Counts That Are Not Interchangeable](002_three_counts_that_are_not_interchangeable.md) | Offered, reported, received — and the judgement that reading the wrong one inverts | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench/docs/data_structure
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### BN[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| BN[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BN9 | `Workload::with_batch` | **latent hazard** | The document says `batch` is tied the same way as capacity; the two setters disagree, and both tests that assert the tie were handed the one fixture where it happens to hold |
| BN10 | the two fields called "producers" | **wrong doc** | The section claims both duplicate representations are closed by making the second unreachable; the second is reachable, and nothing in the family reads it |
| BN11 | `run`'s four `stats.record_*` writes | **latent hazard** | Two of the three counters feeding `RingStats`'s one derived reading are handed the identical `received` expression, so the in-flight count is structurally zero and the assertion that pins it cannot fail |
| BN12 | `RingStats::record_drop` | n/a — inconsistency | `record_drop` buckets every loss by overflow policy, and the mutex baseline — which has no policy and never consults the one it is handed — files its drops under it anyway |
