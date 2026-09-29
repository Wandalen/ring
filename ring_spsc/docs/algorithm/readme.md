# Algorithm Doc Definition

### Scope

- **Purpose**: Document the two procedures this crate performs — the producer's claim-and-publish and the consumer's batch drain — step by step, with the ordering each step requires and the cost it carries.
- **Responsibility**: Name each step, its synchronization, and the atomic operations the single-producer cardinality removes from it.
- **In Scope**: The publish procedure; the drain procedure; their ordering pairs.
- **Out of Scope**: The surfaces these are exposed through (→ [`api/`](../api/readme.md)); the fields they operate on (→ [`data_structure/`](../data_structure/readme.md)); the slot's byte layout, which is `ring_slot`'s.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Uncontended Claim and Publish](001_uncontended_claim_and_publish.md) | The producer's six steps, and the four things the multi-producer path needs that this one does not | 🔄 |
| 002 | [Single-Consumer Drain to the Published Bound](002_single_consumer_drain.md) | The consumer's five steps, and why its available-bound is a cursor read rather than a scan | 🔄 |


### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_spsc/docs/algorithm
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
| SP1 | the claim step | n/a — observation | No path in this crate performs a read-modify-write, so the producer cursor advances without a compare-exchange loop. |
| SP2 | `Reservation` | **latent hazard** | The same guard-drop-publishes contract as the multi-producer sibling, and the same sharp edge: an early return emits a default record. |
| SP3 | `drain` | n/a — observation | With one producer the cursor is the published watermark, so the drain reads a value rather than scanning for one. |
| SP4 | `drain_up_to` | n/a — coverage | `drain_up_to_zero_is_a_legitimate_no_op` and `drain_up_to_more_than_available_yields_what_there_is` make every `usize` a legal argument. |
