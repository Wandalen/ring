# Pattern Doc Definition

### Scope

- **Purpose**: Record the two shaping choices that make this crate's output auditable — the measurement is a value rather than a print, and a refusal is a row rather than an absence.
- **Responsibility**: State each pattern, the forces that select it, and its consequences.
- **In Scope**: Returning data rather than emitting text; collecting refusals rather than propagating them.
- **Out of Scope**: The ranking rule (→ [`algorithm/002`](../algorithm/002_the_eligibility_filter_runs_before_the_comparison.md)); patterns inside the candidates.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [The Measurement Is a Value](001_the_measurement_is_a_value.md) | Every number is a field with an accessor; nothing is printed by the library | 🔄 |
| 002 | [A Refusal Is a Row](002_a_refusal_is_a_row.md) | A candidate that could not run appears with its reason, never as a shorter table | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench/docs/pattern
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### BN[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| BN[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BN37 | `pattern/001` Consequences, and `examples/comparison.rs` | **wrong doc** | The Consequences section states as a fact about the crate that the price of returning a value is that there is no binary; the binary exists, and no document under `docs/` names it outside a findings table |
| BN38 | `examples/comparison.rs::stability` | **misleading doc** | The one place this crate states a verdict prints a fixed literal about lock-free paths winning with no overlap, and the code above it computes a spread over a single candidate |
| BN39 | `pattern/002` Problem vs Consequences | **wrong doc** | The Problem section opens on one refusal count and the Consequences section closes on a different one, about the same run |
| BN40 | `a_comparison_lists_refusals_rather_than_shortening_the_table` | n/a — coverage | The document diagnoses that `outcomes + refusals == ALL` holds at any split and therefore never contradicted the wrong number, and the test it names as this pattern's own is still that assertion |
