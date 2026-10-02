# ring_trace

Optional sequence-operation trace log.

Depends on [`ring_types`](../ring_types/readme.md).

One of the 33 `ring_*` crates that make up this family's concurrency
write-path. The 33 crates form a dependency forest rooted at `ring_types`,
acyclic by construction. Build order follows [`../Cargo.toml`](../Cargo.toml)'s
member list. [`../readme.md`](../readme.md) describes the family as a whole.

Implemented. [`tests/trace_test.rs`](tests/trace_test.rs) asserts the
behaviour, and a reader checks it by hand against
[`tests/manual/readme.md`](tests/manual/readme.md). Every line is covered.

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build. See [verb/readme.md](verb/readme.md) |
| `docs/` | Scope, related crates, and open trade-offs. See [docs/readme.md](docs/readme.md) |
| `src/lib.rs` | Crate root, holding the crate's whole public API |
