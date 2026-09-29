# workaround

External constraints `ring_mpsc` absorbs.

### Scope

- **Purpose**: Record every external constraint this crate compensates for, so each one carries a cost and a deletion condition.
- **Responsibility**: Document this crate's workarounds, or record explicitly that it has none.
- **In Scope**: Constraints originating outside this repository — for a crate whose only dependencies are workspace siblings, that means the language, the toolchain, and the targets.
- **Out of Scope**: This crate's own design decisions, which are not workarounds however unusual they look; constraints compensated in shared tooling elsewhere in the workspace.

### Overview

**Two, both from the language.**

This crate is the family's only multi-producer ring, and it is one of two crates
that hold storage *and* the cursors bounding it. That combination is what puts
it on the workspace unsafe allowlist: two producers writing two different slots
of one array is a disjointness the borrow checker cannot express, and no safe
formulation of it exists. Both workarounds below are that one constraint seen
from two sides — the opt-out that lets the code be written
([`001`](001_the_unsafe_code_opt_out_and_its_obligations.md)), and the `Sync`
impl that lets the type cross a thread boundary
([`002`](002_an_unsafe_impl_sync_the_compiler_cannot_derive.md)).

**The earlier reading of this file was that there were none**, on the argument
that a crate with no published dependency has nothing external to compensate
for. Every dependency being a workspace sibling is true and it is the wrong
test: the language is external, and it is the language this crate works around.
The supporting dependency list was also wrong in both directions — it named
`ring_publish` and `ring_consume`, which are not dependencies, and omitted
`ring_atomic`, `ring_slot` and `ring_types`, which are.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_mpsc
printf 'path deps:        '; grep -c 'path = "\.\./ring_' Cargo.toml
printf 'published deps:   '; grep -cE '^[a-z-]+ = \{ version' Cargo.toml
printf 'the siblings:     '; grep -oE 'ring_[a-z]+' Cargo.toml | sort -u | tr '\n' ' '; echo
printf 'unsafe in code:   '
grep -vE '^\s*(//|///|//!)' src/lib.rs | grep -c 'unsafe'
```

Live output:

```
path deps:        8
published deps:   0
the siblings:     ring_atomic ring_store ring_claim ring_config ring_consume ring_cursor ring_gating ring_mpsc ring_publish ring_slot ring_types 
unsafe in code:   10
```

**This file is where the allowlist points.** `gate/declared/ring/unsafe_allowlist.txt`
names three crates and requires each to justify its opt-out in its own
this file; G6 enforces that by scanning for the word here. So
a **None.** in this file was not merely incomplete — it was the enforced
obligation left undischarged, and it survived because G6's own scan pattern
omitted the house codestyle's spaces and matched no crate at all
(→ [`../decisions/001`](../decisions/001_ring_core_sits_on_the_unsafe_allowlist_without_unsafe.md)).

### Sources

| File | Relationship |
|------|-----------------|
| `Cargo.toml` | The dependency surface: eight path dependencies, no published crate |
| `src/lib.rs` | The ten unsafe lines both workarounds account for |
| `../../../bench_harness/gate/declared/ring/unsafe_allowlist.txt` | Names this crate and requires the justification to live in this file |
| `../../../bench_harness/gate/g6_unsafe.sh` | Enforces that requirement — the gate this file answers to |


### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_mpsc/docs/workaround
printf 'instances:                '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### MP[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| MP[0-9]+ ' readme.md
# instances:                2
# finding headings inside:  3
# rows in the table below:  3
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| MP50 | the unsafe sites | n/a — observation | The four call sites of `slot`/`slot_mut` are all inside `Reserved` or `Batch`, neither of which can be constructed for an ungated sequence. |
| MP51 | the unsafe census | n/a — observation | `ring_mpsc` and `ring_spsc` each carry exactly ten `unsafe` lines in code, and no other crate in the family carries any. |
| MP52 | consumer uniqueness | **latent hazard** | The `Sync` impl's first safety clause cites `ends`' and `split`'s receivers, so a convenience change to either invalidates it silently. |
