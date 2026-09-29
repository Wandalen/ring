# API Doc Definition

### Scope

- **Purpose**: Document the two caller-facing surfaces, and record which of their shape questions are cheap to leave open because this crate sits behind the family's five-crate export boundary.
- **Responsibility**: Name each surface's operations, costs, error cases, and compatibility position.
- **In Scope**: The producer end; the consumer end; the candidate shapes each could take.
- **Out of Scope**: The procedures behind them (→ [`algorithm/`](../algorithm/readme.md)); the handle types a consumer actually holds, which are `ring_handle`'s.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Producer Surface](001_producer_surface.md) | Three candidate shapes, and why `free_capacity()` is a binding guarantee here and a hint in `ring_mpsc` | 🔄 |
| 002 | [Consumer Surface](002_consumer_surface.md) | Why the batch shape is forced rather than chosen, and the one drain shape that is both zero-copy and sound | 🔄 |


### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_spsc/docs/api
printf 'instances:                '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### SP[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| SP[0-9]+ ' readme.md
# instances:                2
# finding headings inside:  4
# rows in the table below:  4
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SP5 | `Producer` | n/a — observation | Duplicating or sharing the producer would break the single-producer invariant, and the negatives are pinned by compile-fail doc tests. |
| SP6 | the producer surface | n/a — observation | `claim`, `push_with` and `try_push` are three entry points onto the same claim-write-publish sequence, differing only in who owns the record. |
| SP7 | `Batch` | n/a — observation | `a_batch_commits_exactly_its_own_length` means a caller that reads three of ten records still releases all ten. |
| SP8 | `Batch` | n/a — coverage | `get_and_iter_agree_at_every_offset` guards two access paths onto the same slots against drifting apart. |
