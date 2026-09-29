# ring_index

Sequence-to-slot index mapping for power-of-two capacities.

Depends on [`ring_types`](../ring_types/readme.md).

One of the 33 `ring_*` crates that make up this family's concurrency
write-path — a 33-crate dependency forest rooted at `ring_types`, acyclic
by construction. Build order follows [`../Cargo.toml`](../Cargo.toml)'s
member list; the family as a whole is described in
[`../readme.md`](../readme.md).

Implemented, delivering sequence-to-slot folding. Behaviour is asserted by
[`tests/index_test.rs`](tests/index_test.rs) and read by hand against
[`tests/manual/readme.md`](tests/manual/readme.md); every line is covered.

Design documentation is a full typed corpus — 13 doc definitions, 26 instances,
56 verified findings, each backed by a command whose output is quoted where it is
used. Twenty-two of the findings are about other crates, which is the ratio a
seventeen-line crate produces: it has almost no surface of its own to be wrong
about, and what it has instead is a claim — *this fold is the only one* — that
can only be checked by reading everyone else. Start at
[`docs/readme.md`](docs/readme.md); every finding is indexed and ranked by
severity in [`docs/definition/readme.md`](docs/definition/readme.md).

| File | Responsibility |
|------|-----------------|
| `docs/` | The design corpus — 13 definitions, 26 instances, 56 findings; see [docs/readme.md](docs/readme.md) |
| `src/lib.rs` | Crate root — the crate's whole public surface |
| `tests/` | `index_test.rs` and the manual plan under `manual/` |
