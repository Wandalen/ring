# ring_slot

Slot payload views — typed and raw bytes.

Depends on [`ring_types`](../ring_types/readme.md).

One of the 33 `ring_*` crates that make up this family's concurrency
write-path — a 33-crate dependency forest rooted at `ring_types`, acyclic
by construction. Build order follows [`../Cargo.toml`](../Cargo.toml)'s
member list; the family as a whole is described in
[`../README.md`](../README.md).

Implemented, delivering the two slot shapes `ring_event` publishes and drains through. Behaviour is asserted by
[`tests/slot_test.rs`](tests/slot_test.rs) and read by hand against
[`tests/manual/readme.md`](tests/manual/readme.md); every line is covered.

Design documentation is a full typed corpus — 13 doc definitions, 26 instances,
52 verified findings, each backed by a command whose output is quoted where it is
used. Nine of the findings are about other crates: a Tier 1 leaf has almost
nothing beneath it to audit, so what it turns up it turns up in the consumers its
shape reaches. Start at [`docs/readme.md`](docs/readme.md); every finding is
indexed and ranked by severity in
[`docs/definition/readme.md`](docs/definition/readme.md).

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build — see [verb/readme.md](verb/readme.md) |
| `docs/` | The design corpus — 13 definitions, 26 instances, 52 findings; see [docs/readme.md](docs/readme.md) |
| `src/lib.rs` | Crate root — the crate's whole public surface |
| `tests/` | `slot_test.rs` and the manual plan under `manual/` |
