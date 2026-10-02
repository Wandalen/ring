# ring_types

Shared ids, errors, and policy enums for the ring family, with no ring logic.

A leaf: it depends on nothing else in the workspace.

One of the 33 `ring_*` crates that make up this family's concurrency
write-path. The 33 crates form a dependency forest rooted at `ring_types`,
acyclic by construction. Build order follows [`../Cargo.toml`](../Cargo.toml)'s
member list; [`../readme.md`](../readme.md) describes the family as a whole.

Implemented. It delivers the ids, errors, and policy enums the rest of the family builds on.
[`tests/types_test.rs`](tests/types_test.rs) asserts the behaviour, which is also read by hand against
[`tests/manual/readme.md`](tests/manual/readme.md). Every line is covered.

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build. See the workspace [verb/readme.md](../verb/readme.md) |
| `docs/` | Scope, related crates, and open trade-offs. See [docs/readme.md](docs/readme.md) |
| `src/lib.rs` | Crate root, holding the crate's whole public API |
