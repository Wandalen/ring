# ring_seqno

Sequence numbers and their comparison across laps.

Depends on [`ring_types`](../ring_types/readme.md).

One of the 33 `ring_*` crates that make up this family's concurrency
write-path — a 33-crate dependency forest rooted at `ring_types`, acyclic
by construction. Build order follows [`../Cargo.toml`](../Cargo.toml)'s
member list; the family as a whole is described in
[`../README.md`](../README.md).

Implemented, delivering never-wrapping sequence comparison and the arithmetic `ring_gating`/`ring_barrier` build their bound on. Behaviour is asserted by
[`tests/seq_test.rs`](tests/seq_test.rs) and read by hand against
[`tests/manual/readme.md`](tests/manual/readme.md); every line is covered.

Five functions, one import, no types and no state — and the tier every crate
above it computes with. [`docs/`](docs/readme.md) documents it at full depth for
that reason: this is the only place the family's shared arithmetic decisions are
all in view at once, and six of the eleven findings the corpus recorded belong to
other crates.

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build — see [verb/readme.md](verb/readme.md) |
| `docs/` | Scope, related crates, and open trade-offs — see [docs/readme.md](docs/readme.md) |
| `src/lib.rs` | Crate root — the crate's whole public surface |

### Reading Order

| Start with | For |
|------------|-----|
| [`docs/api/001`](docs/api/001_five_functions_and_no_types.md) | The whole surface in one table |
| [`docs/algorithm/001`](docs/algorithm/001_four_readings_of_one_subtraction.md) | Why four functions are one subtraction, and which equivalences are tested |
| [`docs/lifecycle/001`](docs/lifecycle/001_one_pair_across_one_lap.md) | Every reading tabulated across a full ring cycle |
| [`docs/definition/readme.md`](docs/definition/readme.md) | The full instance index and all eleven findings |
