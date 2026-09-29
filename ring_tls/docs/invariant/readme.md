# Invariant Doc Definition

### Scope

- **Purpose**: State the discipline that makes lock-free appends sound, so it is enforced as a contract rather than assumed as a convention.
- **Responsibility**: Document `ring_tls`'s own invariants and what enforces each.
- **In Scope**: Properties of the write epoch — who may append, when a reader may look, and what the append path may not call.
- **Out of Scope**: What the appended bytes mean (→ consumers' own encodings); the merge-side ordering contract (→ [`ring_mpsc/docs/invariant/`](../../../ring_mpsc/docs/invariant/readme.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Single-Writer Append](001_single_writer_append.md) | One thread per buffer per epoch; no reader until every writer quiesces | 🔄 |
| 002 | [Zero Allocations in Steady State](002_zero_allocations_in_steady_state.md) | Exactly 0 allocator calls per cycle after warm-up — an allocator call is a smuggled-in lock | 🔄 |




### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_tls/docs/invariant
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
| TL28 | the single-writer rule | n/a — observation | Every mutating method takes `&mut self`, so the borrow checker enforces the invariant this instance argues for by convention. |
| TL29 | the epoch discipline | **wrong doc** | The soundness argument turns on a per-buffer epoch counter; no epoch exists in the crate or anywhere in the family. |
| TL30 | the allocation count | n/a — observation | Zero allocations in steady state holds for `Vec::with_capacity` plus a refusing `push`, by a different mechanism than the one specified. |
| TL31 | the test coverage | n/a — coverage | This crate's own reached-test asserts zero allocations "by a counting allocator", and `ring_tls` carries no `#[ global_allocator ]` to do the asserting — four sibling crates do. |
