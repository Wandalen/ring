# Lifecycle Doc Definition

### Scope

- **Purpose**: Document the handle pair's arc from split to drop, the longer arrangement that determines whether the consume point is defined at all, and the two orthogonal axes a handle observes — who owns what, and whether the ring is still live.
- **Responsibility**: Name the phases and the states, the transitions, the ordering dependencies, the cleanup obligations at each end, which transitions are forbidden without a detector, and what each end may rely on.
- **In Scope**: The pair's own life; the consumer's placement across a tick; ownership states across the pair's life; liveness states across close and reset.
- **Out of Scope**: Ring construction, which is `ring_factory`'s; `close`/`reset`/`drain_all`, which are `ring_shutdown`'s; the ring's internal slot states, which are the backends'.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Split, Move and Drop](001_split_move_and_drop.md) | Six phases, an asymmetry between the two drops, and one unresolved question about publishing to a consumerless ring | 🔄 |
| 002 | [The Barrier Holds the Consumer](002_the_barrier_holds_the_consumer.md) | The arrangement this crate makes possible and cannot perform, with the crate that could named explicitly | 🔄 |
| 003 | [Handle Ownership](003_handle_ownership.md) | Six states, nine transitions, and the one forbidden transition with no compile-time detector | 🔄 |
| 004 | [Ring Liveness Through a Handle](004_ring_liveness_through_a_handle.md) | Two states that return the same value and mean opposite things, and the shutdown-loop ordering that follows | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_handle/docs/lifecycle
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### HD[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| HD[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| HD29 | phase L6 | **misleading doc** | L6 says the ring drops when the handles do, and the crate's own drop test drops the `Split` rather than the handles — what is measured is the owner's drop, not the phase the row describes |
| HD30 | the E2 debt row | n/a — drift | E2's debt row asks for a test proving the barrier is the only drain point, and `ring_poll`'s suite now contains one covering exactly that — the row records the debt as outstanding |
| HD31 | transition M9 | n/a — inconsistency | M9's row calls cloning the one forbidden transition with no compile-time detector, and the Tests table eleven rows later cites the case that detects it |
| HD32 | the preamble | n/a — observation | The preamble relocates one observation to a sibling instance and the entire transition set went with it, leaving a state machine whose states are described here and whose transitions are argued elsewhere |
