# Algorithm Doc Definition

### Scope

- **Purpose**: Specify the two procedures a claim-and-publish ring runs — one per side of the producer/consumer boundary — so a proposed mechanism is measured against stated steps rather than against an implementation.
- **Responsibility**: Document `ring_mpsc`'s own procedures, their step sequences, their cost discipline, and which of their details remain open.
- **In Scope**: The producer's slot acquisition and the consumer's batch drain, including the published-watermark detection the drain turns on.
- **Out of Scope**: The fields these procedures operate over (→ [`data_structure/`](../data_structure/readme.md)); the memory orderings both deliberately omit at each step (→ [Publication Ordering](../invariant/002_publication_ordering.md)); when a drain is triggered, which is the consumer's decision and not this crate's (→ [The Spinning Consumer Owns a Core](../pitfall/001_spinning_consumer_owns_a_core.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Claim-Then-Publish Slot Acquisition](001_claim_then_publish.md) | One compare-exchange buys a privately-owned slot, so the ~500 ns payload write never serializes against another producer | 🔄 |
| 002 | [Batch Drain by Single Cursor Swap](002_batch_drain_by_cursor_swap.md) | The consumer claims a whole published run in one atomic and walks it in sequence order — stopping at the first unpublished slot, not at the claim cursor | 🔄 |
| 003 | [Batched Claim-and-Publish](003_batched_claim_and_publish.md) | One gate check and one exchange buy k contiguous slots, amortising the contended step over the group; the drop publishes every sequence of the grant | 🔄 |


### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_mpsc/docs/algorithm
printf 'instances:                '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### MP[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| MP[0-9]+ ' readme.md
# instances:                3
# finding headings inside:  5
# rows in the table below:  5
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| MP1 | the claim step | n/a — observation | This crate owns the stamp and the slot; the cursor arithmetic and headroom check belong to `ring_claim` and `ring_cursor`. |
| MP2 | `Reserved` | **latent hazard** | Dropping a `Reserved` guard without calling `set` publishes whatever the slot already held, and one test pins that behaviour as intended. |
| MP3 | `drain` | n/a — observation | A published sequence behind an unpublished one is not drained, which makes one slow producer stall every later record. |
| MP4 | `drain_up_to` | n/a — observation | A `max` exceeding capacity is clamped, so a caller cannot walk into the next lap by asking for too much. |
| MP53 | `claim_batch` / `ReservedBatch` | n/a — observation | The batched grant is a write group, not a reservation: a held guard parks the consumer behind its whole range, and the drop publishes every sequence of the grant whether written or not. |
