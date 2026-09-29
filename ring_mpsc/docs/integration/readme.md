# Integration Doc Definition

### Scope

- **Purpose**: Document the two boundaries this crate sits between — the seven sibling crates it composes from, and the prospective consumers it was factored out to serve — so that neither is left as an unstated assumption in the mechanism docs.
- **Responsibility**: Name each integration point, what crosses it, what it assumes of the other side, and which of those assumptions are currently unverified.
- **In Scope**: The `ring_*` family seam beneath this crate and the export surface above it; the consumer-adoption seam and what adoption would actually require of a consumer.
- **Out of Scope**: The mechanism itself (→ [`algorithm/`](../algorithm/readme.md), [`data_structure/`](../data_structure/readme.md)); the operation contracts (→ [`api/`](../api/readme.md)); whether adoption happens at all, which is this effort's own open verdict.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Family Dependency Seam](001_family_dependency_seam.md) | The seven crates beneath this one and the five-crate export surface above it — what this crate owns versus merely composes | 🔄 |
| 002 | [Prospective Consumer Adoption](002_prospective_consumer_adoption.md) | What adopting this crate would actually cost a consumer, stated before adoption rather than discovered during it | 🔄 |


### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_mpsc/docs/integration
printf 'instances:                '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### MP[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| MP[0-9]+ ' readme.md
# instances:                2
# finding headings inside:  4
# rows in the table below:  4
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| MP18 | `Cargo.toml` | n/a — observation | Every dependency is a workspace sibling, so nothing outside this repository can impose a version constraint here. |
| MP19 | the seam | n/a — observation | The import count exceeds the dependency count because several siblings contribute more than one name. |
| MP20 | `adoption` | n/a — unadopted | `ring_bench` takes `Ring` alone; every handle type is reached through `ring_core`. |
| MP21 | `adoption` | n/a — observation | The observation surface with no caller is exactly what a monitoring or backpressure adopter would need first. |
