# ring_debug

Runtime invariant checks over a live ring.

Part of the `ring` family; [../readme.md](../readme.md) describes the whole.

## Why it exists

The family's cursor arithmetic assumes its own invariants and does not check
them. That assumption is correct and the arithmetic is right to make it. But
when an invariant is violated, the readings report the healthiest state they
can express instead of the problem.

| Ring state (capacity 8) | `free_slots` | `pending` | `may_claim` |
|---|---:|---:|:---:|
| Healthy, producer 3, consumer 0 | 5 | 3 | `true` |
| **Consumer ahead**, producer 3, consumer 9 | **8** | **0** | **`true`** |
| Producer lapped, producer 30, consumer 0 | 0 | 30 | `false` |

The middle row is a corrupted ring reading exactly as a fresh empty one,
including on `may_claim`, the value a producer acts on. The test
`the_arithmetic_reports_an_empty_ring_for_a_consumer_ahead_cursor` pins it.

## What it offers

| Entry point | Needs | Catches |
|---|---|---|
| `check` | One observation | A consumer ahead of its producer; a producer more than a lap ahead |
| `Watch` | Two or more observations | Both of the above, plus a cursor that moved backwards |
| `check_ends` | A split, quiescent ring | Two public readings of one ring disagreeing |

**It is opt-in, and nothing in the family depends on it.** Making the check
automatic would put it on the claim path, and this crate exists as the
alternative to that.

## Decisions

- [A `Watch` reports only what it can observe now, and does not latch a violation it has reported](docs/decisions/001_a_watch_does_not_latch.md)
- [`Violation` stays a closed enum, without `#[non_exhaustive]`](docs/decisions/002_violation_stays_a_closed_enum.md)

## Known limitations

- From a live `ring_core::Ring` the only reachable check is `check_ends`.
  `check` and `Watch` take a `CursorPair`, and nothing in the family hands one
  out. `ring_core` has none, and the backends that have one keep it private.
  `check_ends` compares two readings that both come from saturating arithmetic,
  so it cannot see a consumer ahead of its producer. The test
  `check_ends_cannot_see_the_corruption_check_can` pins this. A second caller
  needing raw sequences, or an investigation that stalls without them, would
  justify `ring_core` forwarding the `position()` that `ring_spsc`'s ends
  already have.
- `Watch::observe` accepts any `CursorPair`, so a watch handed a pair from a
  different ring returns a confident report that mixes the two rings' numbers.
  The test `observing_a_foreign_pair_answers_about_neither_ring` pins this.

## Run it

```sh
cargo nextest run -p ring_debug --all-features
cargo test --doc -p ring_debug --all-features
```

| File | Responsibility |
|------|-----------------|
| `docs/decisions/` | Architecture decision records |
| `src/lib.rs` | The three checks, `Watch`, and `Violation` |
| `tests/debug_test.rs` | Every violation the tests catch is produced through `ring_cursor`'s own public `store` |
| `tests/manual/readme.md` | Manual plan and dated run record |
