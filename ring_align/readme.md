# ring_align

Cache-line padding constants and alignment wrappers.

Depends on [`ring_types`](../ring_types/readme.md).

One of the 33 `ring_*` crates that make up this family's concurrency
write-path — a 33-crate dependency forest rooted at `ring_types`, acyclic
by construction. Build order follows [`../Cargo.toml`](../Cargo.toml)'s
member list; the family as a whole is described in
[`../README.md`](../README.md).

Implemented, delivering cache-line padding. Behaviour is asserted by
[`tests/align_test.rs`](tests/align_test.rs) and read by hand against
[`tests/manual/readme.md`](tests/manual/readme.md); every line is covered.

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build — see [verb/readme.md](verb/readme.md) |
| `docs/` | Scope, related crates, and open trade-offs — see [docs/readme.md](docs/readme.md) |
| `src/lib.rs` | Crate root — the crate's whole public surface |
| `tests/` | `align_test.rs` and the manual plan under `manual/` |

Three items and one attribute, with 13 doc definitions over 26 instances behind
them — the documentation is mostly about consequences rather than mechanics.
[`docs/readme.md`](docs/readme.md) § Where to Start routes by question; the
findings this crate records against code it does not own are listed in
[`docs/definition/readme.md`](docs/definition/readme.md) § Findings Recorded,
Not Fixed.
