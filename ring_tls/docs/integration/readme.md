# Integration Doc Definition

### Scope

- **Purpose**: Document the two boundaries this crate sits between — its unusually thin dependency list beneath, and the export boundary it sits *on* rather than behind, since `ring_tls` is one of the family's five externally-visible crates.
- **Responsibility**: Name each integration point, what crosses it, the capabilities the thin dependency list leaves unowned, and what adoption would cost a consumer.
- **In Scope**: The two declared dependencies and the family crates conspicuously absent from them; the external export boundary; the consumer-adoption seam.
- **Out of Scope**: The mechanism (→ [`algorithm/`](../algorithm/readme.md), [`data_structure/`](../data_structure/readme.md)); the composition this crate participates in (→ [Staging Then Merge](../pattern/002_staging_then_merge.md)); whether adoption happens, which is a future benchmark's verdict.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Family Dependency Seam](001_family_dependency_seam.md) | Two dependencies, four conspicuous absences, and the export boundary this crate sits on rather than behind | 🔄 |
| 002 | [Prospective Consumer Adoption](002_prospective_consumer_adoption.md) | What adopting the staging half costs a consumer, and why the crate's justification is weaker with none having adopted it yet | 🔄 |




### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_tls/docs/integration
printf 'instances:                '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### TL[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| TL[0-9]+ ' readme.md
# instances:                2
# finding headings inside:  4
# rows in the table below:  4
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TL24 | the dependency seam | n/a — drift | The instance describes two dependencies; the manifest declares three, plus three dev-dependencies. |
| TL25 | the absent dependencies | n/a — observation | `ring_claim`, `ring_publish`, `ring_consume` and `ring_gating` are still not dependencies — not by the design this instance argues from, but because `ring_batch::claim` subsumes the one step this crate takes. |
| TL26 | the consumer set | n/a — drift | The instance projects adoption by a specific named consumer; the three crates that actually declare the dependency are `ring_flush`, `ring_bench` and `ring_testkit`. |
| TL27 | the reach picture | n/a — observation | Every crate naming `ring_tls` in code also declares it, and one crate names it only in a comment saying it was removed. |
