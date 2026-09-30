# ring_debug

Runtime invariant checks over a live ring.

Depends on [`ring_core`](../ring_core/readme.md),
[`ring_cursor`](../ring_cursor/readme.md),
[`ring_types`](../ring_types/readme.md),
[`ring_atomic`](../ring_atomic/readme.md).

One of the 33 `ring_*` crates that make up this family's concurrency
write-path — a 33-crate dependency forest rooted at `ring_types`, acyclic
by construction. Build order follows [`../Cargo.toml`](../Cargo.toml)'s
member list; the family as a whole is described in
[`../README.md`](../README.md).

Originally scoped for two dependency edges; it has four. Reading a
cursor at all needs `ring_atomic::SeqCell` in scope — `load` and `store` are
trait methods — and `Seq`/`Capacity` appear in every public signature. The
two-edge design assumed the *derived* readings would suffice, and the finding
below is that they do not.

## Why it exists

The family's cursor arithmetic assumes its own invariants and does not check
them. That assumption is correct and the arithmetic is right to make it. But
when it is violated, the readings do not merely fail to report the problem —
they report the healthiest state they can express.

| Ring state (capacity 8) | `free_slots` | `pending` | `may_claim` |
|---|---:|---:|:---:|
| Healthy — producer 3, consumer 0 | 5 | 3 | `true` |
| **Consumer ahead** — producer 3, consumer 9 | **8** | **0** | **`true`** |
| Producer lapped — producer 30, consumer 0 | 0 | 30 | `false` |

The middle row is a corrupted ring reading exactly as a fresh empty one,
including on `may_claim` — the value a producer acts on. Measured, not inferred:
[`docs/pitfall/001`](docs/pitfall/001_saturating_arithmetic_reports_health.md).

## What it offers

| Entry point | Needs | Catches |
|---|---|---|
| `check` | One observation | A consumer ahead of its producer; a producer more than a lap ahead |
| `Watch` | Two or more observations | Both of the above, plus a cursor that moved backwards |
| `check_ends` | A split, quiescent ring | Two public readings of one ring disagreeing |

**It is opt-in, and nothing in the family depends on it.** Making the check
automatic would put it on the claim path — the alternative this crate exists
instead of. Constraints and their measurements:
[`docs/non_functional_requirement/001`](docs/non_functional_requirement/001_absent_unless_called.md).

## The limitation worth knowing before using it

`check` and `Watch` take a `CursorPair`, and **nothing in the family hands one
out** — `ring_core` has none, and the backends that do keep it private. From a
live `ring_core::Ring` the only reachable check is `check_ends`, which is built
from the derived readings and therefore **cannot see the consumer-ahead case at
all**. Pinned as a test, not left as a caveat:
[`docs/integration/001`](docs/integration/001_reaching_the_cursors_of_a_live_ring.md).

## What the implementation settled

| Question | Answer |
|---|---|
| `verb/` | Crate-scoped test/lint/build — see [verb/readme.md](verb/readme.md) |
| Is this belt-and-braces? | No — the failure mode is silent and lands on `may_claim` |
| Should `ring_seqno` be made defensive instead? | No — a branch on the family's hottest read, for a state a correct program never reaches |
| One entry point or two? | Two — D3 costs the caller a baseline that D1 and D2 do not |
| Can the strongest check reach a real ring? | No, and that is recorded rather than worked around |

## Layout

| File | Responsibility |
|------|-----------------|
| `docs/` | Scope, invariants, and open trade-offs — see [docs/readme.md](docs/readme.md) |
| `src/lib.rs` | The three checks, `Watch`, and `Violation` |
| `tests/debug_test.rs` | 28 tests — every violation caught through `ring_cursor`'s own public `store` |
| `tests/manual/readme.md` | Manual plan and dated run record |
