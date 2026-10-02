# ring_claim

Sequence-range claiming without waiting.

Depends on [`ring_types`](../ring_types/readme.md), [`ring_cursor`](../ring_cursor/readme.md), [`ring_gating`](../ring_gating/readme.md).

One of the 33 `ring_*` crates that make up this family's concurrency
write path. The 33 crates form a dependency forest rooted at `ring_types`,
acyclic by construction. Build order follows
[`../Cargo.toml`](../Cargo.toml)'s member list, and
[`../readme.md`](../readme.md) describes the family as a whole.

Implemented. It delivers the claim half of the four-operation handshake
(claim → publish → available → commit) and the exclusivity guarantee that no
two producers are ever granted the same sequence. [`tests/claim_test.rs`](tests/claim_test.rs)
asserts the behaviour, and a reader checks it by hand against
[`tests/manual/readme.md`](tests/manual/readme.md). Every line is covered.

Claiming never waits. Every call returns a range or says why not. That lets
the same call serve a spinning producer, a parking producer, and the tick path
that must not block at all. A caller that wants to wait calls
`ring_wait::for_space` first. The grant itself is a compare-exchange loop
rather than a `fetch_add`, because the gate check and the advance have to be
one atomic step. The manual plan's first check is a mutation run that
deliberately breaks this and confirms the suite notices.

The design corpus under `docs/` has 13 doc definitions and 26 instances with
55 findings. A command verifies each finding, and its output is quoted where it
is used. Twenty of the 55 are about other crates. A Tier 5 crate is where four
tiers of decisions arrive, so reading it closely is the cheapest audit of
everything it touches. Among other things, the corpus found that every `claim`
performs a heap allocation inside `ring_cursor`, two crates down.
[`docs/definition/readme.md`](docs/definition/readme.md)
indexes all of them and ranks the thirteen that are reachable.

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build. See [verb/readme.md](verb/readme.md) |
| `docs/` | Design corpus of 13 definitions, 26 instances and 55 findings. See [docs/readme.md](docs/readme.md) |
| `src/lib.rs` | Crate root, holding the crate's whole public API |
| `tests/` | `claim_test.rs` and the manual plan under `manual/` |
