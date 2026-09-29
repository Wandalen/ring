# Pattern Doc Definition

### Scope

- **Purpose**: Record the two structural choices everything else in this crate follows from — moving behaviour outward, and moving a check inward.
- **Responsibility**: For each, state the problem, the solution, its applicability, and its consequences.
- **In Scope**: The discriminant/handler split this crate documents; the validating newtype that removes a check from fifteen crates.
- **Out of Scope**: The handlers themselves, owned by `ring_wait` and `ring_overflow`; the validation as a procedure (→ [`../algorithm/001`](../algorithm/001_validating_a_slot_count_to_a_power_of_two.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Discriminants Here, Handlers Elsewhere](001_discriminants_here_handlers_elsewhere.md) | Why `WaitKind::Park` is a name here and a `thread::park` two crates away — and the third dispatcher the ruling does not account for | 🔄 |
| 002 | [A Newtype That Makes a Check Unnecessary](002_a_newtype_that_makes_a_check_unnecessary.md) | Applied once out of three opportunities, and the two declines are the evidence it was applied deliberately rather than as a habit | 🔄 |

**The two patterns pull in opposite directions and both protect the same
property.** 001 pushes behaviour out of the crate so tier 0 keeps no
dependencies. 002 pulls a check into the crate so fifteen consumers keep none of
their own. Between them they are why `ring_types` compiles against nothing and
why nothing downstream calls `is_power_of_two`.

**Each instance ends on a cost that is measured rather than asserted.** 001's is
that the split is ruled and undocumented by any gate — nothing stops a `pause()`
from being added to `WaitKind` tomorrow. 002's is that the guarantee travels
while the code using it does not: `ring_mpsc` re-implements `ring_index`'s fold
inline, correctly, because the pattern makes duplicating the operation harmless
— **which is a different and stronger property than preventing the duplication.**


### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_types/docs/pattern
printf 'instances:               '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### TY[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| TY[0-9]+ ' readme.md
# instances:               2
# finding headings inside:  2
# rows in the table below:  2
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TY47 | ring family | n/a — observation | `ring_wait` names no `OverflowPolicy` and `ring_overflow` names no `WaitKind`, so neither handler leaked into the other despite both discriminant sets being declared in one file |
| TY48 | ring family | n/a — observation | The newtype made re-validation unnecessary in 31 crates, and the only way to see that is that none of them contains a power-of-two check |
