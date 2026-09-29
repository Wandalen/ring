# Integration Doc Definition

### Scope

- **Purpose**: Give this crate's position in the family — the 30 crates that declare it, and the one that measured the same constraint and declined.
- **Responsibility**: State each integration's system description, integration points, error handling, and compatibility requirements.
- **In Scope**: The dependency map with per-export consumer counts; `ring_registry`'s separate error type and what it costs a Contract-bound consumer.
- **Out of Scope**: The `Copy` requirement that forced the split (→ [`../non_functional_requirement/001`](../non_functional_requirement/001_errors_and_positions_do_not_allocate.md)); the acyclicity rule (→ [`../invariant/002`](../invariant/002_tier_zero_depends_on_nothing.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [The Crate Thirty-One of Thirty-Three Depend On](001_the_crate_thirty_one_of_thirty_three_depend_on.md) | The dependency map, per-export consumer counts, and the seven compatibility requirements a change here has to satisfy | 🔄 |
| 002 | [The Registry That Declined the Shared Error](002_the_registry_that_declined_the_shared_error.md) | The one non-declarer whose refusal is a design choice rather than an omission — and the export-Contract gap it leaves open | 🔄 |

**001 is the census and 002 is the exception**, and the pair only works read in
that order: the 30-of-33 figure is unremarkable until the three non-declarers are
named, and one of them is this crate itself.

**002 is where the family's one unresolved integration defect is recorded.**
`ring_factory` re-exports `Registry` but not `RegistryError`, so a consumer
bound by the five-crate export Contract can register a ring and cannot name the
error registration returns. The fix is one `pub use` line in a crate this one
does not own; the finding belongs here because the cause — `RingError` being
`Copy` — is this crate's.


### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_types/docs/integration
printf 'instances:               '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### TY[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| TY[0-9]+ ' readme.md
# instances:               2
# finding headings inside:  5
# rows in the table below:  5
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TY29 | ring family | n/a — unadopted | `ring_registry` is the one declared dependent naming no item from `ring_types` in `src/`, so the manifest graph overstates the code graph by exactly one edge |
| TY30 | ring family | n/a — observation | Construction reach per variant ranges from 8 dependent crates down to 0, and the four with none include both name variants and both capacity variants |
| TY31 | ring family | n/a — observation | 30 crates declare the dependency and the most-used exported name reaches 18, so most dependents adopt a small slice of a small crate |
| TY32 | ring family | n/a — duplication | `RingError::NameTaken`, `BuildError::NameTaken` and `RegistryError::NameTaken` all mean a ring is already registered under this name, and two of the three render the identical message |
| TY33 | `ring_factory` | n/a — doc gap | It converts `RegistryError::NameTaken` into its own `BuildError::NameTaken` at `ring_factory:214`, so the Contract consumer never learns which crate refused |
