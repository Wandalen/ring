# Lifecycle Doc Definition

### Scope

- **Purpose**: Specify the two things in this crate that move through states — `Watch` at run time, and the crate's own implementation task.
- **Responsibility**: For each, the states it occupies, what moves it between them, and the transitions that are easy to get wrong or easy to leave untaken.
- **In Scope**: `Watch::new`, `Watch::observe`, `Watch::last` and the baseline's update rule; the task lifecycle `unverified` → `verified` → executed, and the three records of this crate's build state.
- **Out of Scope**: The comparison performed at each observation (→ [`algorithm/001`](../algorithm/001_checking_a_pair_without_touching_it.md)); D3 itself (→ [`invariant/001`](../invariant/001_cursor_invariants_over_a_live_ring.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [From One Observation to a Sequence](001_from_one_observation_to_a_sequence.md) | The states a watch occupies, and why D3 needs a second observation rather than a cleverer single reading | 🔄 |
| 002 | [A Finished Crate in the First Stage](002_a_finished_crate_in_the_first_stage.md) | The crate's own lifecycle, and the three records of its state that give two answers | 🔄 |

**Two lifecycles, one document each, because they are checked by different means.**
`001` is a runtime state machine whose transitions are executable and whose
coverage can be reconstructed from the test suite. `002` is a bookkeeping
lifecycle whose state is a directory name, and whose evidence is three files that
do not agree.

They share a definition because both answer *what state is this in and how did it
get there*, which is what a reader comes to `lifecycle/` for. Keeping them separate
matters because the second is not about the crate's behaviour at all, and reading
it as though it were would suggest the code has a problem.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_debug/docs/lifecycle
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### DB[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| DB[0-9]+ ' readme.md )"
```

Live output:

```
instances:                2
finding headings inside:  4
rows in the table below:  4
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| DB37 | failure mode M3 | **misleading doc** | The guard against reporting a consequence instead of a cause is credited to a test whose pair breaks no ordering invariant, so it would pass under the failure it is cited against — while the test that does discriminate is listed two clauses earlier under a different heading. |
| DB38 | transition T6 | n/a — coverage | The recovery transition, defended at greater length than any other and carrying the whole of the no-latch decision, is the one transition no test exercises; a change making the failed state absorbing would pass the entire suite. |
| DB39 | task 126's state | n/a — inconsistency | The implementation task sits in the state meaning "not yet scoped" and contains, twenty lines lower, a 6/6 gate verdict and a satisfied acceptance criterion — both halves accurate, because the work went through the staged run instead and all 33 ring tasks are in the same position. |
| DB40 | the Implementation Record | n/a — drift | The prose inventory added so the task file would not understate the crate now understates it, naming a definition directory that has since been renamed away and omitting seven that exist — a hand-written count in a file on no regeneration path. |
