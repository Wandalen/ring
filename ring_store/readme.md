# ring_store

Power-of-two slot storage array.

Depends on [`ring_types`](../ring_types/readme.md), [`ring_slot`](../ring_slot/readme.md), [`ring_index`](../ring_index/readme.md).

One of the 33 `ring_*` crates that make up this family's concurrency
write-path — a 33-crate dependency forest rooted at `ring_types`, acyclic
by construction. Build order follows [`../Cargo.toml`](../Cargo.toml)'s
member list; the family as a whole is described in
[`../readme.md`](../readme.md).

Implemented. Behaviour is asserted by
[`tests/buffer_test.rs`](tests/buffer_test.rs) and read by hand against
[`tests/manual/readme.md`](tests/manual/readme.md); every line is covered.

Design documentation is a full typed corpus — 13 doc definitions, 26 instances,
53 verified findings, each backed by a command whose output is quoted where it is
used. Eight of the findings are about other crates, all immediate neighbours: a
storage tier reaches nowhere, and is reached into by everything — every `unsafe`
block in the family dereferences into this array. Start at
[`docs/readme.md`](docs/readme.md); every finding is indexed and ranked by
severity in [`docs/definition/readme.md`](docs/definition/readme.md).

| File | Responsibility |
|------|-----------------|
| `docs/` | The design corpus — 13 definitions, 26 instances, 53 findings; see [docs/readme.md](docs/readme.md) |
| `src/lib.rs` | Crate root — the crate's whole public surface |
| `tests/` | `buffer_test.rs` and the manual plan under `manual/` |
