# Invariant Doc Definition

### Scope

- **Purpose**: Document the two properties every cost saving in this crate is purchased with — the cardinality precondition, and the synchronization budget it buys.
- **Responsibility**: State each invariant, where enforcement can and cannot live, and what a violation actually does.
- **In Scope**: The one-producer-one-consumer constraint; the no-lock, no-RMW, wait-free budget.
- **Out of Scope**: The multi-producer relaxation, which is a different crate; the wait strategies invoked after a full or empty result, which are `ring_wait`'s.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Exactly One Producer, Exactly One Consumer](001_exactly_one_producer_one_consumer.md) | The precondition this crate states and cannot check, and the three violations that are all silent | 🔄 |
| 002 | [No Lock in the Path](002_no_lock_in_the_path.md) | Three claims of decreasing obviousness, of which the third distinguishes this crate from its sibling | 🔄 |


### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_spsc/docs/invariant
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
| SP22 | `cardinality` | n/a — observation | `split` takes `&mut self` so a second pair cannot coexist with the first, and the negatives are pinned where no runtime assertion could reach. |
| SP23 | `cardinality` | n/a — observation | The invariant is one producer and one consumer, not two threads, and the same-thread case is legal and unexercised. |
| SP24 | the orderings | n/a — unenforced | `the_two_orderings_are_the_ones_the_design_names` runs always; the `exhaustive` module that checks behaviour needs `--cfg loom`. |
| SP25 | `GATING` | n/a — observation | Every cross-end load carries `ring_cursor::GATING`, which this crate imports rather than declares and does not pin. |
