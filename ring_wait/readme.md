# ring_wait

Wait strategies for space and data availability.

Depends on [`ring_types`](../ring_types/readme.md), [`ring_cursor`](../ring_cursor/readme.md).

One of the 33 `ring_*` crates that make up this family's concurrency
write-path — a 33-crate dependency forest rooted at `ring_types`, acyclic
by construction. Build order follows [`../Cargo.toml`](../Cargo.toml)'s
member list; the family as a whole is described in
[`../readme.md`](../readme.md).

Implemented, delivering the four wait strategies, over one loop.
`ring_types::WaitKind` holds the four discriminants and this crate holds the
four handlers, so a `WaitKind` can travel through a config into a struct field
without dragging a thread-parking implementation behind it. Behaviour is
asserted by [`tests/wait_test.rs`](tests/wait_test.rs) and read by hand against
[`tests/manual/readme.md`](tests/manual/readme.md); every line is covered.

`None` is the variant the rest exist around: it evaluates the predicate once and
returns, because a tick has a deadline and a strategy that might block has
already missed it. Every other wait is bounded by an explicit budget rather than
a deadline, so the same call takes the same number of samples on every machine.

The crate declares no type of its own — no `struct`, no `enum`, no `trait`, one
of three in the family that declare nothing — and performs no atomic operation,
names no `Ordering`, contains no `unsafe`, no cast and no bare `loop`. Its whole
executable content is a thirteen-line counted `for` and a four-arm `match` on a
one-byte enum. What it holds instead is policy: when to stop asking, and what to
do between asks. The corpus under `docs/` is about those two decisions and what
they cost.

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build — see [verb/readme.md](verb/readme.md) |
| `docs/` | 13 doc definitions, 26 instances, 24 findings — see [docs/readme.md](docs/readme.md) |
| `src/lib.rs` | Crate root — the crate's whole public surface |
| `tests/` | `wait_test.rs` and the manual plan under `manual/` |

### Reading Order

| Read | For |
|------|-----|
| [`docs/definition/readme.md`](docs/definition/readme.md) | The Module Index — every definition, every instance, and all 24 findings with severity |
| [`docs/algorithm/001`](docs/algorithm/001_one_loop_and_the_two_ways_out.md) | The loop itself: thirteen lines, two exits, one clamp |
| [`docs/item/001`](docs/item/001_the_four_arms_of_the_pause.md) | The four things a wait does between asks |
| [`docs/api/001`](docs/api/001_seven_items_and_the_one_with_a_caller.md) | The whole surface, and which parts anything actually calls |
| [`docs/non_functional_requirement/002`](docs/non_functional_requirement/002_what_each_strategy_costs_per_attempt.md) | Measured cost per attempt — 55 ns to 119 µs, a 1700× span |
| [`docs/integration/001`](docs/integration/001_two_dependencies_two_dependents_and_a_roster.md) | Why the tick path may not reach this crate, and how that is enforced |
| [`docs/pitfall/001`](docs/pitfall/001_reading_empty_as_nothing_to_do.md) | The one thing to get right before calling `wait_until` from a producer |
