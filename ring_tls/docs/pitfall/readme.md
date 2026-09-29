# Pitfall Doc Definition

### Scope

- **Purpose**: Record the traps this crate's own vocabulary invites, so the mitigation is found while the design is still a skeleton rather than after a storage choice has been built on.
- **Responsibility**: Document `ring_tls`'s confirmed traps — the readings of "per-thread" and "bump-allocated" that produce a working program with the wrong structural properties.
- **In Scope**: Incorrect assumptions about what this crate's per-thread and zero-lock claims actually require of a caller.
- **Out of Scope**: External constraints this crate absorbs, which carry a deletion condition a trap never has (→ [`../workaround/`](../workaround/readme.md)); the merge half's own traps (→ [`ring_mpsc`'s Spinning Consumer Owns a Core](../../../ring_mpsc/docs/pitfall/001_spinning_consumer_owns_a_core.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Implicit Thread-Locals Are Hidden Global State](001_implicit_thread_locals_are_hidden_state.md) | `thread_local!` keys the region by thread, not by owner — multiple independent instances become inexpressible | 🔄 |




### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_tls/docs/pitfall
printf 'instances:                '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### TL[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| TL[0-9]+ ' readme.md
# instances:                2
# finding headings inside:  3
# rows in the table below:  3
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TL45 | the hidden-state trap | n/a — observation | This instance argues against implicit `thread_local!` storage, and the crate contains none — nor does any of the thirty-three. |
| TL46 | `Flush` | **measured cost** | Dropping a `Flush` after two of sixty-four pairs leaves sixty-two claimed sequences owned by nothing, and the buffer empty either way. |
| TL47 | `drain` | n/a — observation | `drain` also empties when its iterator is dropped unread, but advances no cursor — which is exactly why `ring_flush` needed it. |
