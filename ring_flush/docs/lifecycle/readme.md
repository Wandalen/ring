# Lifecycle Doc Definition

### Scope

- **Purpose**: Describe the cycle this crate sequences, the arc a policy itself travels, and the two state machines a flush spans — the buffer's, as this crate drives it, and the policy's own armed/fired cycle.
- **Responsibility**: State each phase and each state, its transitions, its dependencies, its cleanup obligations, and the behavioural invariants each machine preserves.
- **In Scope**: The consolidation cycle; a policy's life from configuration to the final drain; buffer state across a flush; policy arming.
- **Out of Scope**: Thread registration and teardown, which are `ring_tls`'s (→ its [`lifecycle/001`](../../../ring_tls/docs/lifecycle/001_thread_registration_and_teardown.md)); the buffer's epoch mechanism itself (→ [`ring_tls`'s Buffer Epoch Cycle](../../../ring_tls/docs/lifecycle/003_buffer_epoch_cycle.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [The Consolidation Cycle](001_the_consolidation_cycle.md) | The cycle `ring_tls` named and deferred, taken up here with its trigger attached | 🔄 |
| 002 | [From Configuration to the Final Drain](002_from_configuration_to_the_final_drain.md) | A policy's own arc, including the shutdown case where every policy must behave identically | 🔄 |
| 003 | [Buffer State Through a Flush](003_buffer_state_through_a_flush.md) | The three-primitive sequence seen as a state machine, including the window where the buffer is neither writable nor drained | 🔄 |
| 004 | [Policy Arming and Firing](004_policy_arming_and_firing.md) | Why `OnBatch(n)` has state and `OnFull` does not, and what that asymmetry costs | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_flush/docs/lifecycle
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### FL[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| FL[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| FL29 | U3 | **misleading doc** | U3 claims no allocation and the test it cites observes only whether a log exists; four sibling crates ship a counting allocator and this crate has not adopted one |
| FL30 | the reuse question | n/a — drift | A question closed by measurement in the decisions table is still recorded open in all three places it was asked |
| FL31 | the invariant tally | **wrong doc** | The tally counts two invariants, names three, and omits the one whose justification rests on two transitions the same instance calls unreachable |
| FL32 | the corpus checkers | n/a — coverage | Four corpus checkers were added since this instance was written and all eighteen verdicts they can emit are structural; none opens a second document to compare claims |
