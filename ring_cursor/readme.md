# ring_cursor

Producer and consumer sequence cursors, cache-line separated.

Depends on [`ring_types`](../ring_types/readme.md), [`ring_seqno`](../ring_seqno/readme.md), [`ring_atomic`](../ring_atomic/readme.md), [`ring_align`](../ring_align/readme.md).

One of the 33 `ring_*` crates that make up this family's concurrency
write-path — a 33-crate dependency forest rooted at `ring_types`, acyclic
by construction. Build order follows [`../Cargo.toml`](../Cargo.toml)'s
member list; the family as a whole is described in
[`../readme.md`](../readme.md).

Implemented, delivering a cursor padded to a cache line, so that
a producer and a consumer on two cores stop invalidating each other's copy of a
line they are not actually sharing data through. Behaviour is asserted by
[`tests/cursor_test.rs`](tests/cursor_test.rs) and read by hand against
[`tests/manual/readme.md`](tests/manual/readme.md); every line is covered.

The padding *decision* is not here — it is `ring_align`'s single
`CACHE_LINE` — and keeping it there is what stops the family from acquiring two
independent answers to the same question. This crate holds the cursors that
wear it.

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build — see [verb/readme.md](verb/readme.md) |
| `docs/` | Scope, related crates, and open trade-offs — see [docs/readme.md](docs/readme.md) |
| `src/lib.rs` | Crate root — the crate's whole public surface |
| `tests/` | `cursor_test.rs` and the manual plan under `manual/` |

Five module-level items and fourteen functions, with 13 doc definitions over 26
instances behind them. [`docs/readme.md`](docs/readme.md) § Where to Start routes
by question; the fifteen findings recorded against code — most of it in the ten
crates that depend on this one — are listed in
[`docs/definition/readme.md`](docs/definition/readme.md) § Findings Recorded,
Not Fixed, with three further items left open pending a compile or a measurement.
