# Lifecycle Doc Definition

### Scope

- **Purpose**: State the phases a thread's buffer passes through from creation to destruction — because in a thread-local design the *thread's* lifetime, not the program's and not the consumer's, is what bounds the data's — and the states it holds between consolidations, which are what make a concurrent read safe.
- **Responsibility**: Document thread registration and teardown, the consolidation cycle nested inside it, the data-loss window teardown opens, the buffer's epoch cycle, the transition that requires cross-thread agreement, and the registration states a buffer occupies relative to the consolidator that must find it.
- **In Scope**: One thread's buffer from creation to free; the repeating consolidation cycle; the interaction between thread exit and unconsolidated records; buffer states across one consolidation cycle; registration state relative to the global buffer set.
- **Out of Scope**: The append procedure's own steps (→ [`algorithm/`](../algorithm/readme.md)); the consumer's own shutdown, which is above this crate; the region layout being appended into, undecided (→ [`../readme.md`](../readme.md)); the payload vocabulary, which is deliberately not this crate's.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Thread Registration and Teardown](001_thread_registration_and_teardown.md) | The buffer's life bounded by its thread's, and the loss window a thread exit opens between the last append and the last consolidation | 🔄 |
| 002 | [Consolidation Cycle](002_consolidation_cycle.md) | The repeating phase nested inside the buffer's life — who triggers it, what it must complete before, and why its period is a correctness parameter | 🔄 |
| 003 | [Buffer Epoch Cycle](003_buffer_epoch_cycle.md) | The four states a buffer holds between consolidations, and the one transition no single thread can effect alone | 🔄 |
| 004 | [Registration State](004_registration_state.md) | A buffer's visibility to the consolidator — the states between a thread existing and its data being reachable | 🔄 |




### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_tls/docs/lifecycle
printf 'instances:                '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### TL[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| TL[0-9]+ ' readme.md
# instances:                4
# finding headings inside:  4
# rows in the table below:  4
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TL37 | the buffer lifecycle | **wrong doc** | A buffer's life is `with_capacity` to drop; no `register`, no thread-exit hook, and no registry to leave. |
| TL38 | the consolidation cycle | **wrong doc** | The cycle specified here is driven by a `consolidate_all` walking a thread registry; the built equivalent is a caller calling `flush_into`. |
| TL39 | the epoch cycle | **wrong doc** | The states are transitions of an epoch counter; `TlsBuffer` has two fields and neither is one. |
| TL40 | the registration state | **wrong doc** | Unregistered/registered/deregistered collapses to "exists", because construction is the only entry and drop the only exit. |
