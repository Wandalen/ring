# ring_atomic

Atomic sequence helpers with explicit memory orderings.

Depends on [`ring_types`](../ring_types/readme.md).

One of the 33 `ring_*` crates that make up this family's concurrency
write-path. The 33 crates form a dependency forest rooted at `ring_types`,
acyclic by construction. Build order follows [`../Cargo.toml`](../Cargo.toml)'s
member list. [`../readme.md`](../readme.md) describes the family as a whole.

Implemented. It delivers the ordering helpers that features 175 and 177 are
stated in terms of. [`tests/atomic_test.rs`](tests/atomic_test.rs) asserts the
behaviour, which is also read by hand against
[`tests/manual/readme.md`](tests/manual/readme.md). Every line is covered.

The design documentation is a full typed corpus: 13 doc definitions, 26 instances
and 52 verified findings. Each finding is backed by a command whose output is
quoted where it is used. Exactly half the findings name something a caller can
reach today. Every one of the ten latent hazards sits in a decision the crate made
deliberately and recorded nowhere: a trait that declines `Sync` and so accepts a
`Cell< u64 >`, an irreversible `fetch_add` whose return may be discarded in
silence, a `counts()` that is four separate reads behind a contract reading as an
instant, and an advance that wraps past `u64::MAX` where its arithmetic sibling
panics. After that wrap, every gate in the family reads the ring as empty. Start
at [`docs/readme.md`](docs/readme.md).
[`docs/definition/readme.md`](docs/definition/readme.md) indexes every finding and
ranks it by severity.

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build. See [verb/readme.md](verb/readme.md) |
| `docs/` | Scope, related crates, and open trade-offs. See [docs/readme.md](docs/readme.md) |
| `src/lib.rs` | Crate root, holding the crate's whole public API |
