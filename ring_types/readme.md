# ring_types

Shared ids, errors, and policy enums for the ring family — no ring logic.

A leaf: it depends on nothing else in the workspace.

One of the 33 `ring_*` crates that make up this family's concurrency
write-path — a 33-crate dependency forest rooted at `ring_types`, acyclic
by construction. Build order follows [`../Cargo.toml`](../Cargo.toml)'s
member list; the family as a whole is described in
[`../readme.md`](../readme.md).

Implemented, delivering the ids, errors, and policy enums the rest of the family builds on. Behaviour is asserted by
[`tests/types_test.rs`](tests/types_test.rs) and read by hand against
[`tests/manual/readme.md`](tests/manual/readme.md); every line is covered.

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build — see [verb/readme.md](verb/readme.md) |
| `docs/` | Scope, related crates, and open trade-offs — see [docs/readme.md](docs/readme.md) |
| `src/lib.rs` | Crate root — the crate's whole public surface |
