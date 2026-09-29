# Algorithm Doc Definition

### Scope

- **Purpose**: Specify the procedures this crate runs on a region — step by step, with their branches and allocations named — so the cost profile is reviewable independent of any one buffer layout.
- **Responsibility**: Document `ring_tls`'s own algorithms, and mark which of their details a future benchmark verdict still owns.
- **In Scope**: The append and reset procedures over one region owned by one writer.
- **Out of Scope**: What the appended bytes mean, which stays entirely each consumer's own encoding; the merge-side procedures many regions are drained by (→ [`ring_mpsc`'s Batch Drain by Cursor Swap](../../../ring_mpsc/docs/algorithm/002_batch_drain_by_cursor_swap.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Tagged-Record Bump Append](001_tagged_record_bump_append.md) | Tag byte, operand memcpy, pointer bump — one branch, no allocator call, no destructor | 🔄 |




### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_tls/docs/algorithm
printf 'instances:                '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### TL[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| TL[0-9]+ ' readme.md
# instances:                2
# finding headings inside:  6
# rows in the table below:  6
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TL1 | the append procedure | **wrong doc** | Every step of the append procedure specified here begins at a function `src/lib.rs` does not declare. |
| TL2 | the growth path | n/a — observation | Both the specified and the built procedure make growth unreachable rather than handling it, by different means. |
| TL3 | the flush cost | n/a — observation | The one atomic a flush costs is inside `ring_batch::claim`; this crate's own source has none. |
| TL4 | `Flush::next` | n/a — duplication | `Flush` derives each sequence with a hand-rolled `u64` counter while `BatchClaim::sequences()` returns exactly that range. |
| TL5 | `Flush::next` | **latent hazard** | `Flush` terminates when the drain runs out, never checking `claim.end()`, so a claim and a drain that disagree yield sequences nobody owns. |
| TL6 | `flush_into` | **measured cost** | `claim( cursor, 0, order )` issues a `fetch_add( 0 )`, so a poll loop over an idle buffer pays one atomic per iteration. |
