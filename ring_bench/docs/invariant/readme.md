# Invariant Doc Definition

### Scope

- **Purpose**: Record the two properties that must hold for this crate's output to be a measurement rather than a plausible table — the count ordering every candidate satisfies, and the region the counters must stay outside of.
- **Responsibility**: State each property, the mechanism that maintains it, and how it is checked.
- **In Scope**: `received ≤ reported ≤ offered`; counters written from totals after `elapsed()`.
- **Out of Scope**: Properties of the candidates themselves, which belong to their crates; the ranking rule, which is [`algorithm/002`](../algorithm/002_the_eligibility_filter_runs_before_the_comparison.md)'s.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Received Never Exceeds Reported Never Exceeds Offered](001_received_never_exceeds_reported_never_exceeds_offered.md) | The only count ordering that holds for every candidate under every policy | 🔄 |
| 002 | [The Counters Are Written Outside the Clock](002_the_counters_are_written_outside_the_clock.md) | Four writes per run, never per record, always after `elapsed()` | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench/docs/invariant
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### BN[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| BN[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BN21 | `invariant/001`'s "structurally guaranteed" | **misleading doc** | `reported ≤ offered` is claimed as structurally guaranteed on the grounds that every runner iterates `records_of( i )` exactly once, and two of the six do not iterate it at all |
| BN22 | `Outcome::dropped` and `Outcome::silently_discarded` | **latent hazard** | Violation Consequences rests on integer underflow panicking in release as well as debug; `overflow-checks` is set nowhere in the workspace, so the subtraction wraps instead |
| BN23 | `run_mutex_queue` and `run_direct_mpsc`'s timed regions | **measured cost** | The invariant warns specifically against candidate-dependent cost inside the clock, and `std::thread::scope` is inside the timed region of exactly the two runners the ranking turns on |
| BN24 | `invariant/002`'s Invariant Statement | **misleading doc** | The boundary is presented as one readable region with a single line marked "← the boundary"; it is six lines in six runner functions, and the four `record_*` writes it is drawn against are in a seventh |
| BN53 | `run`'s inline `stats.record_drop` computation | **latent hazard** | BN22 guarded the two `Outcome` accessors that expose `offered - received`; `run` computes the identical subtraction earlier, inline, to feed `record_drop`, and that third call site was not covered by BN22's guard |
