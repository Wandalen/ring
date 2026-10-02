# ring_batch

Batch claim objects spanning a sequence range.

Depends on [`ring_types`](../ring_types/readme.md), [`ring_seqno`](../ring_seqno/readme.md).

One of the 33 `ring_*` crates that make up this family's concurrency
write-path. The 33 crates form a dependency forest rooted at `ring_types`,
acyclic by construction. Build order follows [`../Cargo.toml`](../Cargo.toml)'s
member list. [`../readme.md`](../readme.md) describes the family as a whole.

Implemented. [`tests/batch_test.rs`](tests/batch_test.rs) asserts the
behaviour, and a reader checks it by hand against
[`tests/manual/readme.md`](tests/manual/readme.md). Every line is covered.

Design documentation is a full typed corpus of 13 doc definitions, 26 instances
and 53 verified findings, each backed by a command whose output is quoted where
it is used. Eighteen of the findings are about other crates, and six of those
eighteen are about one crate: `ring_claim`, three tiers up, which wrote the same
sixteen-byte range struct a second time and gave it the `must_use`, the owned
producer cursor, and the compare-exchange retry loop this crate does without.
Neither crate names the other and no dependency edge exists in either direction.
Start at [`docs/readme.md`](docs/readme.md).
[`docs/definition/readme.md`](docs/definition/readme.md) indexes every finding
and ranks it by severity.

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build. See [verb/readme.md](verb/readme.md) |
| `docs/` | The design corpus of 13 definitions, 26 instances and 53 findings. See [docs/readme.md](docs/readme.md) |
| `src/lib.rs` | Crate root, holding the crate's whole public API |
| `tests/` | `batch_test.rs` and the manual plan under `manual/` |
