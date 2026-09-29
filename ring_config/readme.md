# ring_config

Ring construction parameters.

Depends on [`ring_types`](../ring_types/readme.md).

One of the 33 `ring_*` crates that make up this family's concurrency
write-path — a 33-crate dependency forest rooted at `ring_types`, acyclic
by construction. Build order follows [`../Cargo.toml`](../Cargo.toml)'s
member list; the family as a whole is described in
[`../readme.md`](../readme.md).

Implemented, delivering the configuration surface of features 172, 180 and 183. Behaviour is asserted by
[`tests/config_test.rs`](tests/config_test.rs) and read by hand against
[`tests/manual/readme.md`](tests/manual/readme.md); every line is covered.

| File | Responsibility |
|------|-----------------|
| `docs/` | Scope, related crates, and open trade-offs — see [docs/readme.md](docs/readme.md) |
| `src/lib.rs` | Crate root — the crate's whole public surface |
