# API Doc Definition

### Scope

- **Purpose**: Fix the operation surface this crate presents to its two callers, with the added weight that `ring_tls` is one of the family's five externally-visible crates — so this surface is a public contract rather than an internal seam.
- **Responsibility**: Document the writer's append surface and the consolidator's read surface, their error behaviour, and what external stability means for a crate whose mechanism is still ungated.
- **In Scope**: Append, seal, drain, and reset as caller-facing operations.
- **Out of Scope**: The append procedure's steps (→ [`algorithm/`](../algorithm/readme.md)); the region layout, undecided (→ [`../readme.md`](../readme.md)); who triggers consolidation, which is `ring_flush`'s (→ [Consolidation Cycle](../lifecycle/002_consolidation_cycle.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Writer Append Surface](001_writer_append_surface.md) | The per-thread side — no atomics, no failure mode except capacity, and an in-place-construction requirement the signature must not defeat | 🔄 |
| 002 | [Consolidator Read Surface](002_consolidator_read_surface.md) | The cross-thread side — the three operations `ring_flush` sequences, and why they are exposed separately rather than as one call | 🔄 |




### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_tls/docs/api
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
| TL7 | the writer surface | **wrong doc** | Neither `append` nor `append_with` is declared; the exported writer surface is `push`, `is_full` and `len`. |
| TL8 | the append signature | n/a — drift | The instance still presents three candidates for a question `src/lib.rs` settled, and `decisions/readme.md` still calls it the crate's one open trade-off. |
| TL9 | the read surface | **wrong doc** | The `seal`/`drain`/`reset` triple specified here was built as the single fused `flush_into`; only `drain` exists, added afterwards for `ring_flush`. |
| TL10 | `ring_flush` | n/a — inconsistency | `ring_flush` still narrates its steps as "seal, drain and reset" and files an algorithm instance under that name, for a sequence this crate cannot perform. |
