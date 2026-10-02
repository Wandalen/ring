# ring_slot

Slot payload views: typed and raw bytes.

Depends on [`ring_types`](../ring_types/readme.md).

One of the 33 `ring_*` crates that make up this family's concurrency
write-path. The 33 crates form a dependency forest rooted at `ring_types`,
acyclic by construction. Build order follows [`../Cargo.toml`](../Cargo.toml)'s
member list. [`../readme.md`](../readme.md) describes the family as a whole.

Implemented. It delivers the two slot shapes `ring_event` publishes and drains
through. [`tests/slot_test.rs`](tests/slot_test.rs) asserts the behaviour, which
is also read by hand against [`tests/manual/readme.md`](tests/manual/readme.md).
Every line is covered.

The design documentation is a full typed corpus: 13 doc definitions, 26
instances and 52 verified findings. Each finding is backed by a command whose
output is quoted where it is used. Nine of the findings are about other crates.
A Tier 1 leaf has almost nothing beneath it to audit, so what it turns up it
turns up in the consumers its shape reaches. Start at
[`docs/readme.md`](docs/readme.md).
[`docs/definition/readme.md`](docs/definition/readme.md) indexes every finding
and ranks it by severity.

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build. See [verb/readme.md](verb/readme.md) |
| `docs/` | The design corpus: 13 definitions, 26 instances, 52 findings. See [docs/readme.md](docs/readme.md) |
| `src/lib.rs` | Crate root, holding the crate's whole public API |
| `tests/` | `slot_test.rs` and the manual plan under `manual/` |
