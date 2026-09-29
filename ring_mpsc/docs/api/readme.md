# API Doc Definition

### Scope

- **Purpose**: Fix the operation surface this crate presents to its two asymmetric callers, so that the producer's and the consumer's contracts are stated separately rather than as one "ring API" that hides how differently the two sides are constrained.
- **Responsibility**: Document each side's operations, their error behaviour, and the compatibility guarantees a caller may rely on while the family's export surface is still settling.
- **In Scope**: The producer-side claim/write/publish surface and the consumer-side drain surface, as operations with contracts.
- **Out of Scope**: The procedures behind them (→ [`algorithm/`](../algorithm/readme.md)); the fields they act on (→ [`data_structure/`](../data_structure/readme.md)); which crate re-exports them to the outside world (→ [Family Dependency Seam](../integration/001_family_dependency_seam.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Producer Publish Surface](001_producer_publish_surface.md) | The many-caller side — lock-free, no failure mode except the one capacity forces, and no ordering guarantee between producers | 🔄 |
| 002 | [Consumer Drain Surface](002_consumer_drain_surface.md) | The single-caller side — batch-shaped by design, and the only surface that can return nothing without that being an error | 🔄 |


### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_mpsc/docs/api
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
| MP5 | `Producer` | n/a — observation | `Producer` is `Send + Sync + Copy`, so duplication needs no method and no reference count. |
| MP6 | `Producer` | n/a — coverage | Copying a producer yields another view of the same cursor, which is the property that makes `Copy` sound. |
| MP7 | `Batch` | n/a — observation | The consumer cursor advances when the batch drops, not when it is created, so slots stay reserved while records are read. |
| MP8 | `is_empty` | **latent hazard** | `Consumer::is_empty` means "nothing drainable now"; `Batch::is_empty` means "this batch took nothing" — and the names are identical. |
