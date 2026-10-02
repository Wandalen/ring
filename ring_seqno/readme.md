# ring_seqno

Sequence numbers and their comparison across laps.

Depends on [`ring_types`](../ring_types/readme.md).

One of the 33 `ring_*` crates that make up this family's concurrency
write-path. The 33 crates form a dependency forest rooted at `ring_types`,
acyclic by construction. Build order follows [`../Cargo.toml`](../Cargo.toml)'s
member list, and [`../readme.md`](../readme.md) describes the family as a
whole.

Implemented. It provides never-wrapping sequence comparison and the arithmetic `ring_gating`/`ring_barrier` build their bound on.
[`tests/seq_test.rs`](tests/seq_test.rs) asserts the behaviour, and a reader checks it by hand against
[`tests/manual/readme.md`](tests/manual/readme.md); every line is covered.

Five functions, one import, no types and no state. Every crate above this tier
computes with it, so [`docs/`](docs/readme.md) documents it at full depth. This
is the only place where all of the family's shared arithmetic decisions are in
view at once, and six of the eleven findings the corpus recorded belong to
other crates.

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build, described in [verb/readme.md](verb/readme.md) |
| `docs/` | Scope, related crates, and open trade-offs, indexed in [docs/readme.md](docs/readme.md) |
| `src/lib.rs` | Crate root, holding the crate's whole public API |

### Reading order

| Start with | For |
|------------|-----|
| [`docs/api/001`](docs/api/001_five_functions_and_no_types.md) | The whole public API in one table |
| [`docs/algorithm/001`](docs/algorithm/001_four_readings_of_one_subtraction.md) | Why four functions are one subtraction, and which equivalences are tested |
| [`docs/lifecycle/001`](docs/lifecycle/001_one_pair_across_one_lap.md) | Every reading tabulated across a full ring cycle |
| [`docs/definition/readme.md`](docs/definition/readme.md) | The full instance index and all eleven findings |
