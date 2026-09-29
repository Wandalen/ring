# Lifecycle Doc Definition

### Scope

- **Purpose**: Document the ring's phases from allocation to drop, the shorter arc the two handle ends run inside them, and the slot and ring states that arc passes through — including the contiguity property distinguishing this ring from its stamped sibling.
- **Responsibility**: Name the phases and the states, the transitions, the ordering dependencies, the cleanup obligations at each end, which transitions have no writer at all, and the observability invariants each thread holds.
- **In Scope**: The ring value's own life; a `Producer` and `Consumer` from creation to drop; the thread-affinity question; one slot's states across a lap; the ring-wide occupancy states.
- **Out of Scope**: `close`/`reset`/`drain_all`, which are `ring_shutdown`'s; the handle types themselves, which are `ring_handle`'s; what to do on Full or Empty, which are `ring_overflow`'s and `ring_wait`'s; the stamped variant (→ [`ring_mpsc`](../../../ring_mpsc/docs/lifecycle/readme.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Ring Construction and Teardown](001_ring_construction_and_teardown.md) | Four phases, the open cursor-initialization question, and the undrained-records-at-drop trade | 🔄 |
| 002 | [Producer and Consumer Pairing](002_producer_consumer_pairing.md) | Where the cardinality invariant stops being a precondition and becomes a fact about the program | 🔄 |
| 003 | [Slot State Without Holes](003_slot_state_without_holes.md) | Four derived states, a transition with no writer, and the contiguous-prefix property that eliminates per-slot stamps | 🔄 |
| 004 | [Ring Occupancy Between the Cursors](004_ring_occupancy.md) | Three states on one subtraction, and the exact asymmetry in which observations each end may act on | 🔄 |


### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_spsc/docs/lifecycle
printf 'instances:                '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### SP[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| SP[0-9]+ ' readme.md
# instances:                4
# finding headings inside:  6
# rows in the table below:  6
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SP31 | `teardown` | n/a — coverage | `the_ends_going_out_of_scope_in_either_order_releases_the_storage_once` covers the case a borrow-based design makes easy to get wrong. |
| SP32 | `with_config` | n/a — observation | `with_config_takes_the_capacity_and_ignores_the_rest` — the overflow policy and producer count are `ring_core`'s to interpret. |
| SP33 | `split` | n/a — observation | `a_second_pair_may_be_split_once_the_first_is_gone` — the ring is not consumed by pairing, only borrowed. |
| SP34 | slot state | n/a — coverage | `sequences_are_issued_consecutively_across_a_wrap` is what "without holes" means, asserted rather than argued. |
| SP35 | slot state | n/a — coverage | The move-out semantics are asserted at the wrap boundary specifically, which is where a stale payload would survive into the next lap. |
| SP36 | `occupancy` | n/a — coverage | `available_and_is_empty_agree_at_every_point_of_a_lap` guards two derived views of the same cursor gap. |
