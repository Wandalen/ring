# ring_tls

Per-thread, bump-allocated, zero-lock append log. Every thread appends to its
own buffer with no atomics and no mutexes; a consolidation step later reads
each thread's buffer.

Depends on [`ring_types`](../ring_types/readme.md) and
[`ring_slot`](../ring_slot/readme.md). One of the 33 `ring_*` crates that make
up this family's concurrency write-path. The 33 crates form a dependency forest
rooted at `ring_types`, acyclic by construction. Build order follows
[`../Cargo.toml`](../Cargo.toml)'s member list; [`../readme.md`](../readme.md)
describes the family as a whole. This crate was `bump_log` before the family
adopted a single prefix.

It carries no *consumer*-side family prefix for the original reason, which is
that it has no natural single owner. A similar append-discipline mechanism
already exists independently elsewhere for a different consumer's opcode
encoding. This crate generalizes only the discipline underneath it:
bump-allocate, append, reset. It takes on no consumer-specific vocabulary,
which stays that consumer's own open question to settle.

A consumer's migration onto this crate is still undecided, and that decision
belongs to the consumer, not to this crate. The buffer layout is settled.
`TlsBuffer<T>` is a `Vec<T>` reserved once to its own refusal bound, which is
what makes the accumulation free.

The crate is implemented. It accumulates N items with zero atomic operations
and lands them with one. [`tests/tls_test.rs`](tests/tls_test.rs) asserts both
numbers through `ring_atomic::CountingSeq`, and the push path is read by hand
against [`tests/manual/readme.md`](tests/manual/readme.md). Every line is
covered.

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build, described in [verb/readme.md](verb/readme.md) |
| `docs/` | Scope, related crates, and open trade-offs, indexed in [docs/readme.md](docs/readme.md) |
| `src/lib.rs` | Crate root, holding the crate's whole public API |
