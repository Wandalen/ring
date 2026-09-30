# ring_tls

Per-thread, bump-allocated, zero-lock append log: every thread appends to its
own buffer with no atomics and no mutexes; a consolidation step later reads
each thread's buffer.

Depends on [`ring_types`](../ring_types/readme.md) and
[`ring_slot`](../ring_slot/readme.md). One of the 33 `ring_*` crates that make
up this family's concurrency write-path — a 33-crate dependency forest rooted
at `ring_types`, acyclic by construction. Build order follows
[`../Cargo.toml`](../Cargo.toml)'s member list; the family as a whole is
described in [`../README.md`](../README.md). This crate was `bump_log` before
the family adopted a single prefix.

It carries no *consumer*-side family prefix for the original reason: no
natural single owner. A similar append-discipline mechanism already exists
independently elsewhere for a different consumer's opcode encoding; this
crate generalizes only the discipline underneath it — bump-allocate, append,
reset — never any consumer-specific vocabulary, which stays that consumer's
own open question to settle.

A consumer's migration onto this crate is still undecided — that decision
belongs to the consumer, not to this crate. The buffer layout is not:
`TlsBuffer<T>` is a `Vec<T>` reserved once to its own refusal bound, which is
what makes the accumulation free.

Implemented — accumulates N items with zero atomic operations, lands them
with one. Both numbers are asserted by
[`tests/tls_test.rs`](tests/tls_test.rs) through `ring_atomic::CountingSeq`,
and the push path is read by hand against
[`tests/manual/readme.md`](tests/manual/readme.md); every line is covered.

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build — see [verb/readme.md](verb/readme.md) |
| `docs/` | Scope, related crates, and open trade-offs — see [docs/readme.md](docs/readme.md) |
| `src/lib.rs` | Crate root — the crate's whole public surface |
