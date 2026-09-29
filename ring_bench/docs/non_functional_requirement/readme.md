# Non-Functional Requirement Doc Definition

### Scope

- **Purpose**: Record the qualities the comparison itself must have to be worth reading — what it measures, and what it must keep out of the measurement.
- **Responsibility**: State each requirement as a checkable property with a named measurement method.
- **In Scope**: The crate's own stated comparison criterion; the exclusion of harness cost from the timed region.
- **Out of Scope**: Performance targets for the candidates themselves — this crate takes no position on how fast a ring *should* be (→ [`decisions/002`](../decisions/002_no_test_asserts_an_ordering.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [The Comparison Is Reproducible and Same-Conditions](001_the_comparison_is_reproducible_and_same_conditions.md) | The crate's own criterion: one workload, every candidate, identical conditions | 🔄 |
| 002 | [The Harness Is Not in the Measurement](002_the_harness_is_not_in_the_measurement.md) | What the clock covers, what it must not, and the one place this was nearly violated | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench/docs/non_functional_requirement
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### BN[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| BN[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BN33 | `non_functional_requirement/001` R3's measured table | **misleading doc** | R3's two rows report six candidates at one producer and two run / four refused at four; both counts include `OffTheShelf`, which the default build does not compile, and the table does not say so |
| BN34 | `non_functional_requirement/001` R1's shared `Workload` | n/a — observation | R1's "exact — the same value, not equal values" threshold is met by construction, and the value it shares carries two different batch numbers |
| BN35 | `non_functional_requirement/002`'s Residue section | n/a — doc gap | Residue 1 discloses `run_mutex_queue` timing its own `std::thread::scope`, with the right consequence for a reader; a second runner does the identical thing and appears in the section zero times |
| BN36 | `non_functional_requirement/002`'s Status and Residue 2 | n/a — inconsistency | The operative threshold is "zero harness bookkeeping that scales with the record count", and Residue 2 discloses bookkeeping that scales with the record count, in the same document that records the status as MET |
