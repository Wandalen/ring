# ring_store

Power-of-two slot storage array.

Depends on [`ring_types`](../ring_types/readme.md), [`ring_slot`](../ring_slot/readme.md), [`ring_index`](../ring_index/readme.md).

One of the 33 `ring_*` crates that make up this family's concurrency
write path. The 33 crates form a dependency forest rooted at `ring_types`,
acyclic by construction. Build order follows
[`../Cargo.toml`](../Cargo.toml)'s member list, and
[`../readme.md`](../readme.md) describes the family as a whole.

Implemented. [`tests/buffer_test.rs`](tests/buffer_test.rs) asserts the
behaviour, and a reader checks it by hand against
[`tests/manual/readme.md`](tests/manual/readme.md). Every line is covered.

Design documentation is a full typed corpus of 13 doc definitions, 26
instances and 53 verified findings. A command backs each finding, and its output
is quoted where it is used. Eight of the findings are about other crates, all
immediate neighbours. A storage tier reaches nowhere, and everything reaches
into it: every `unsafe` block in the family dereferences into this array. Start
at [`docs/readme.md`](docs/readme.md).
[`docs/definition/readme.md`](docs/definition/readme.md) indexes every finding
and ranks it by severity.

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build. See the workspace [verb/readme.md](../verb/readme.md) |
| `docs/` | The design corpus of 13 definitions, 26 instances and 53 findings. See [docs/readme.md](docs/readme.md) |
| `src/lib.rs` | Crate root, holding the crate's whole public API |
| `tests/` | `buffer_test.rs` and the manual plan under `manual/` |
