# ring_batch

Batch claim objects spanning a sequence range.

Depends on [`ring_types`](../ring_types/readme.md), [`ring_seqno`](../ring_seqno/readme.md).

One of the 33 `ring_*` crates that make up this family's concurrency
write-path — a 33-crate dependency forest rooted at `ring_types`, acyclic
by construction. Build order follows [`../Cargo.toml`](../Cargo.toml)'s
member list; the family as a whole is described in
[`../readme.md`](../readme.md).

Implemented. Behaviour is asserted by
[`tests/batch_test.rs`](tests/batch_test.rs) and read by hand against
[`tests/manual/readme.md`](tests/manual/readme.md); every line is covered.

Design documentation is a full typed corpus — 13 doc definitions, 26 instances,
53 verified findings, each backed by a command whose output is quoted where it is
used. Eighteen of the findings are about other crates, and six of those eighteen
are about one crate: `ring_claim`, three tiers up, which wrote the same
sixteen-byte range struct a second time and gave it the `must_use`, the owned
producer cursor, and the compare-exchange retry loop this crate does without.
Neither crate names the other and no dependency edge exists in either direction.
Start at [`docs/readme.md`](docs/readme.md); every finding is indexed and ranked
by severity in [`docs/definition/readme.md`](docs/definition/readme.md).

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build — see [verb/readme.md](verb/readme.md) |
| `docs/` | The design corpus — 13 definitions, 26 instances, 53 findings; see [docs/readme.md](docs/readme.md) |
| `src/lib.rs` | Crate root — the crate's whole public surface |
| `tests/` | `batch_test.rs` and the manual plan under `manual/` |
