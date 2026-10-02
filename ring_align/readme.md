# ring_align

Cache-line padding constants and alignment wrappers.

Depends on [`ring_types`](../ring_types/readme.md).

One of the 33 `ring_*` crates that make up this family's concurrency
write-path. The 33 crates form a dependency forest rooted at `ring_types`,
acyclic by construction. Build order follows [`../Cargo.toml`](../Cargo.toml)'s
member list. [`../readme.md`](../readme.md) describes the family as a whole.

Implemented. It delivers cache-line padding.
[`tests/align_test.rs`](tests/align_test.rs) asserts the behaviour, which is also
read by hand against [`tests/manual/readme.md`](tests/manual/readme.md). Every
line is covered.

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build. See [verb/readme.md](verb/readme.md) |
| `docs/` | Scope, related crates, and open trade-offs. See [docs/readme.md](docs/readme.md) |
| `src/lib.rs` | Crate root, holding the crate's whole public API |
| `tests/` | `align_test.rs` and the manual plan under `manual/` |

Three items and one attribute, with 13 doc definitions over 26 instances behind
them. The documentation is mostly about consequences rather than mechanics.
[`docs/readme.md`](docs/readme.md) § Where to Start routes by question.
[`docs/definition/readme.md`](docs/definition/readme.md) § Findings Recorded,
Not Fixed lists the findings this crate records against code it does not own.
