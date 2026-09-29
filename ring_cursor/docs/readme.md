# docs

Design documentation for `ring_cursor`, as typed doc definitions.

Scope of this crate: producer and consumer sequence cursors, cache-line separated.

| Directory | Responsibility |
|------|-----------------|
| `algorithm/` | The fold over a cursor slice, and the three readings of a pair |
| `api/` | The public surface, split by what forwards and what decides |
| `data_structure/` | The layouts of `PaddedCursor` and `CursorPair` |
| `decisions/` | Why `GATING` is fixed, and why the pair holds the capacity |
| `definition/` | Module Index — every definition and every instance in this crate, in one place |
| `integration/` | The four dependencies declared, and the ten crates that declare this one |
| `invariant/` | One cursor per cache line, and no literal `64` in this source |
| `item/` | Per-function inventory — signature, `const`ness, loads, coverage |
| `lifecycle/` | A cursor's six stages, and a pair's cyclic lap |
| `non_functional_requirement/` | The layout claim's testability, and the allocation on the gating read |
| `pattern/` | The forwarding newtype, and one fold serving two questions |
| `pitfall/` | Two changes that pass every automatic check and are still wrong |
| `type/` | `GATING` and `PaddedCursor` as promises a caller may rely on |
| `workaround/` | External constraints this crate absorbs, with costs and deletion conditions |

13 definitions, 26 instances. The full index, with every instance's one-line
summary, is [`definition/readme.md`](definition/readme.md).

### Where to Start

| If you want | Read |
|-------------|------|
| What the crate is for, in one page | [`type/002_padded_cursor.md`](type/002_padded_cursor.md) |
| Why a newtype rather than a type alias | [`pattern/001_the_forwarding_newtype.md`](pattern/001_the_forwarding_newtype.md) |
| What the tests actually establish | [`non_functional_requirement/001_the_layout_claim_is_testable.md`](non_functional_requirement/001_the_layout_claim_is_testable.md) |
| What is wrong that nobody has fixed | [`definition/readme.md`](definition/readme.md) § Findings Recorded, Not Fixed |
| How this crate reaches the other 32 | [`integration/002_who_reads_a_cursor.md`](integration/002_who_reads_a_cursor.md) |

### On the Instance Count

Two instances per definition is the floor, not a target. Where a definition had
only one thing worth saying it was still given two, and the second is
deliberately the *narrower* of the pair — `invariant/002` is a grep, `item/002`
is a table of eight signatures. Padding a definition to reach a number produces
documents nobody reads; the rule followed here was that a second instance had to
carry at least one finding the first did not.

Fifteen verified findings came out of writing them, plus three items left open
because they need a compile or a measurement. All eighteen are collected in
[`definition/readme.md`](definition/readme.md) § Findings Recorded, Not Fixed
rather than repeated here. **None has been fixed** — every repair either turns a
passing test into a failing one or touches another crate, and both belong to a
change with its own verification run.
