# ring_gating

Producer gating so it never laps the slowest consumer.

Depends on [`ring_types`](../ring_types/readme.md), [`ring_cursor`](../ring_cursor/readme.md), [`ring_seqno`](../ring_seqno/readme.md).

One of the 33 `ring_*` crates that make up this family's concurrency
write-path. The 33 crates form a dependency forest rooted at `ring_types`,
acyclic by construction. Build order follows [`../Cargo.toml`](../Cargo.toml)'s
member list. [`../readme.md`](../readme.md) describes the family as a whole.

Implemented. It delivers the producer half of the gating mechanism. Given a set
of consumer cursors, it computes how far a producer may advance without landing
on a slot one of them has not finished with. `ring_barrier` is the consumer half
and reads the same minimum from the other side.
[`tests/gating_test.rs`](tests/gating_test.rs) asserts the behaviour, which is
also read by hand against [`tests/manual/readme.md`](tests/manual/readme.md).
Every line is covered.

The bound is the minimum across the whole set, so one stalled consumer stops the
producer for everyone. That is the correct behaviour, and the reason a stalled
consumer is worth detecting rather than routing around. An *empty* set is not a
consumer at zero. A ring nobody reads has no data anyone can lose, and
collapsing the two would deadlock every ungated ring at its first lap.

The crate is 325 lines, with one type and eleven methods. It has no atomic load,
no fold, no sequence arithmetic, no loop, no `unsafe` and no destructor. The
compiler cannot check any of those absences, so [`docs/`](docs/readme.md)
documents the crate at full depth. What is left after the delegation is which
question to ask and which refusal to return, and both are decisions rather than
code.

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build. See the workspace [verb/readme.md](../verb/readme.md) |
| `docs/` | Scope, related crates, and open trade-offs. See [docs/readme.md](docs/readme.md) |
| `src/lib.rs` | Crate root, holding the crate's whole public API |
| `tests/` | `gating_test.rs` and the manual plan under `manual/` |

### Reading order

| Start with | For |
|------------|-----|
| [`docs/api/001`](docs/api/001_eleven_methods_over_one_owned_vec.md) | The whole public API in one table |
| [`docs/algorithm/001`](docs/algorithm/001_headroom_in_two_delegations.md) | The six-step descent from a gate read to one subtraction |
| [`docs/lifecycle/002`](docs/lifecycle/002_the_producer_walking_a_lap_against_a_stall.md) | The sequence this crate's behaviour is accepted against, step by step |
| [`docs/decisions/002`](docs/decisions/002_a_result_rather_than_a_bool.md) | Why a `Result`, and why nothing calls it |
| [`docs/definition/readme.md`](docs/definition/readme.md) | The full instance index and all sixty-three findings |
