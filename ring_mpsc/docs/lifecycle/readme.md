# Lifecycle Doc Definition

### Scope

- **Purpose**: State the phases a ring passes through from allocation to teardown, and name the states a slot and the ring as a whole occupy inside them — so that "the ring exists", "published" and "full" each become a position in a stated graph rather than an adjective two procedures use in their own sense.
- **Responsibility**: Document construction, the steady state, the drain-to-quiescence a correct shutdown requires, what each phase owes the next, the slot's per-lap state cycle, the ring's occupancy states, the exact field write that effects each transition, and which transitions are concurrent with which.
- **In Scope**: Ring construction and teardown; the producer handle's own attach/detach cycle; slot states across one lap and across the lap boundary; ring occupancy as the producer and consumer cursors move; the transitions each of this crate's two algorithms performs.
- **Out of Scope**: The capacity value fixed at construction (→ [Capacity](../type/002_capacity.md)); cross-crate wiring of the shutdown signal (→ [`integration/`](../integration/readme.md)); the fields themselves (→ [`data_structure/`](../data_structure/readme.md)); the step sequences that drive the transitions (→ [`algorithm/`](../algorithm/readme.md)); what a full ring *does* about being full, which is an undecided policy rather than a state (→ [Bounded Capacity and Backpressure Policy](../non_functional_requirement/002_bounded_capacity_backpressure.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Ring Construction and Teardown](001_ring_construction_and_teardown.md) | The four phases of a ring's life, and why teardown is the only one with a correctness obligation rather than merely a resource one | 🔄 |
| 002 | [Producer Attachment and Detachment](002_producer_attachment_and_detachment.md) | A producer's own cycle against a longer-lived ring, and the outstanding-claim window that makes detachment non-trivial | 🔄 |
| 003 | [Slot State Across One Lap](003_slot_state_across_one_lap.md) | A slot's four states and the single field write that effects each transition — the state a stamp encodes, not a flag | 🔄 |
| 004 | [Ring Occupancy Between the Cursors](004_ring_occupancy_between_cursors.md) | The ring's three occupancy states as a function of cursor distance, and why only one of the three boundaries is observable without a race | 🔄 |


### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_mpsc/docs/lifecycle
printf 'instances:                '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### MP[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| MP[0-9]+ ' readme.md
# instances:                4
# finding headings inside:  6
# rows in the table below:  6
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| MP31 | `Ring::new` | n/a — observation | The slot buffer and the stamp array are the only two allocations in the crate, both at construction. |
| MP32 | `with_config` | n/a — observation | The config constructor takes the capacity and deliberately does not read the overflow policy or the producer count. |
| MP33 | `Producer` | n/a — observation | A producer that goes away leaves no trace, because it never had state of its own to release. |
| MP34 | `Reserved` | n/a — observation | The producer can read back what it wrote before the stamp store makes it visible to the consumer. |
| MP35 | slot state | n/a — coverage | `a_taken_record_leaves_its_slot_empty` is what separates "drained" from "still present but past the cursor". |
| MP36 | `occupancy` | n/a — coverage | `a_commit_restores_exactly_the_capacity_it_released` is the conservation law occupancy depends on. |
