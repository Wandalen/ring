# Pitfall Doc Definition

### Scope

- **Purpose**: Record the two traps this crate's shape sets — a policy that cannot observe its own trigger, and two batch sizes that look like one.
- **Responsibility**: For each, state the scope, the trap, the failure it produces, and the mitigation.
- **In Scope**: `OnBarrier`'s blindness; the `OnBatch(n)` / `ring_batch` size collision.
- **Out of Scope**: Traps in the buffer itself, which are `ring_tls`'s.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [`OnBarrier` Cannot See the Barrier](001_on_barrier_cannot_see_the_barrier.md) | The crate names a trigger it has no dependency capable of observing — the central structural fact, stated as a trap | 🔄 |
| 002 | [Two Batch Sizes That Must Not Diverge](002_two_batch_sizes_that_must_not_diverge.md) | `OnBatch(n)` and `ring_batch`'s claim width are different numbers with the same name, tuned by different people for different reasons | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_flush/docs/pitfall
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### FL[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| FL[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| FL41 | the closure recipe | n/a — drift | The instance that names closure counts perishable ships two perished ones, behind the fence type that guarantees the gate never reruns its regeneration recipe |
| FL42 | Mitigation 4 | n/a — doc gap | Mitigation 4 claims this instance is cited from the crate root; the crate root cites four of this crate's instances and neither pitfall is among them |
| FL43 | the claim width | **wrong doc** | `ring_batch` owns no claim width — `claim` takes it as a per-call argument and the crate declares no constant, so the coupling's other half is in call-site form |
| FL44 | the benchmark binding | **measured cost** | The only benchmark spends one scalar as buffer capacity, ring config batch and `OnBatch`'s `n`, which makes `OnBatch` and `OnFull` fire at identical instants |
