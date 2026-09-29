# API Doc Definition

### Scope

- **Purpose**: Document the two caller-facing surfaces — and, since this crate's guarantees are absences, enumerate what is deliberately missing from each.
- **Responsibility**: Name each surface's operations, its absent operations, its error handling, and the compatibility guarantees an exported crate carries.
- **In Scope**: The publishing end; the draining end.
- **Out of Scope**: The values these surfaces belong to (→ [`type/`](../type/readme.md)); where the consumer should be held (→ [`lifecycle/`](../lifecycle/readme.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Producer Surface](001_producer_surface.md) | Four operations, six absences, and one signature carrying two different contracts depending on backend | 🔄 |
| 002 | [Consumer Surface](002_consumer_surface.md) | Why a cloned consumer breaks nothing and destroys replayability, and why `drain()`'s bound is fixed at call time | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_handle/docs/api
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### HD[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| HD[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| HD5 | the Error Handling table | **wrong doc** | The producer surface's Error Handling table names three error types and none of them exists anywhere in the family's source — the surface's only refusal is the returned record itself |
| HD6 | `Split::new` | n/a — observation | The crate's only constructor takes a `ring_core::Ring`, which a consumer outside `ring_*` cannot name, so the audience the export surface exists for cannot call it and must reach the crate through `ring_factory` |
| HD7 | `try_recv` | n/a — observation | Every refusal on the draining surface is a bare `Option::None` carrying no reason, so empty, closed and drained are one observable state and a caller cannot distinguish them |
| HD8 | `try_recv_batch` | n/a — observation | The only `Vec` in the crate's public surface is the caller-supplied output buffer on the draining side; the publishing side takes an iterator and allocates nothing |
