# Type Doc Definition

### Scope

- **Purpose**: Define the two enumerations this crate owns — what may be compared, and why a comparison may be refused.
- **Responsibility**: State each type's variants, its derives, and the validation rules that hold by construction.
- **In Scope**: `Candidate` and its ceilings; `RunError` and its four variants, three of which relay a dependency.
- **Out of Scope**: `Workload` and `Outcome`, which are records rather than vocabularies and are documented under `data_structure/`; `WorkloadError`, whose four variants are documented with the record they validate.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Candidate](001_candidate.md) | Six variants for four named paths, each declaring the producer count it admits | 🔄 |
| 002 | [Run Error](002_run_error.md) | One refusal of this crate's own and three relayed, one of which is unreachable by construction | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench/docs/type
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### BN[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| BN[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BN45 | `type/001` §`producer_ceiling`, and the crate's prose cardinals generally | **misleading doc** | One section states plainly that the candidate count is five or six depending on a feature, the next opens on "four variants" and closes on "four-sixths wrong", and twenty bare cardinals across thirteen documents name no build at all |
| BN46 | `Candidate::name`, and §`Validation` | **latent hazard** | §`Validation` says there is nothing to validate beyond uniqueness, and one `name()` string is a load-bearing identifier in an untested file where a mismatch prints a `usize::MAX` spread and `0.0x` |
| BN47 | `every_error_renders`, and §`Rendering` | **misleading doc** | §`Rendering` and the test's own doc comment promise that errors chain; the test boxes the one variant that wraps nothing, and no `Error` impl in the family implements `source` |
| BN48 | §`Derives`, `No #[ non_exhaustive ]` | **wrong doc** | The second reason given for the omission is that `#[ non_exhaustive ]` would cost the ability to construct a variant in a test; on an enum it blocks exhaustive matching rather than construction, and `ring_types::RingError` is constructed at 126 test sites across eleven other crates |
