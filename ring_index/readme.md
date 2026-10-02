# ring_index

Sequence-to-slot index mapping for power-of-two capacities.

Depends on [`ring_types`](../ring_types/readme.md).

One of the 33 `ring_*` crates that make up this family's concurrency
write-path. The 33 crates form a dependency forest rooted at `ring_types`,
acyclic by construction. Build order follows [`../Cargo.toml`](../Cargo.toml)'s
member list, and [`../readme.md`](../readme.md) describes the family as a
whole.

Implemented. It provides sequence-to-slot folding.
[`tests/index_test.rs`](tests/index_test.rs) asserts the behaviour, and a reader checks it by hand against
[`tests/manual/readme.md`](tests/manual/readme.md); every line is covered.

Design documentation is a full typed corpus: 13 doc definitions, 26 instances,
56 verified findings, each backed by a command whose output is quoted where it is
used. Twenty-two of the findings are about other crates, which is the ratio a
seventeen-line crate produces. It has almost no public API of its own to be
wrong about. What it has instead is the claim that *this fold is the only one*,
and only reading everyone else can check it. Start at [`docs/readme.md`](docs/readme.md).
[`docs/definition/readme.md`](docs/definition/readme.md) indexes every finding
and ranks it by severity.

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build, described in [verb/readme.md](verb/readme.md) |
| `docs/` | The design corpus: 13 definitions, 26 instances, 56 findings, indexed in [docs/readme.md](docs/readme.md) |
| `src/lib.rs` | Crate root, holding the crate's whole public API |
| `tests/` | `index_test.rs` and the manual plan under `manual/` |
