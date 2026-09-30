# ring_claim

Sequence-range claiming without waiting.

Depends on [`ring_types`](../ring_types/readme.md), [`ring_cursor`](../ring_cursor/readme.md), [`ring_gating`](../ring_gating/readme.md).

One of the 33 `ring_*` crates that make up this family's concurrency
write-path — a 33-crate dependency forest rooted at `ring_types`, acyclic
by construction. Build order follows [`../Cargo.toml`](../Cargo.toml)'s
member list; the family as a whole is described in
[`../README.md`](../README.md).

Implemented, delivering the claim half of the four-operation handshake
(claim → publish → available → commit) and the exclusivity guarantee that no
two producers are ever granted the same sequence. Behaviour is asserted by [`tests/claim_test.rs`](tests/claim_test.rs)
and read by hand against [`tests/manual/readme.md`](tests/manual/readme.md);
every line is covered.

Claiming never waits. Every call returns a range or says why not, which is what
lets one primitive serve a spinning producer, a parking producer, and the tick
path that must not block at all — a caller that wants to wait composes
`ring_wait::for_space` in front of it. The grant itself is a compare-exchange
loop rather than a `fetch_add`, because the gate check and the advance have to
be one atomic step; the manual plan's first check is a mutation run that
deliberately breaks this and confirms the suite notices.

The design corpus under `docs/` is 13 doc definitions and 26 instances carrying
55 findings, each verified by a command whose output is quoted where it is used.
Twenty of the 55 are about other crates: a Tier 5 primitive is where four
tiers of decisions arrive, so reading it closely is the cheapest audit of
everything it touches — the corpus found, among other things, that every
`claim` performs a heap allocation inside `ring_cursor`, two crates down.
[`docs/definition/readme.md`](docs/definition/readme.md)
indexes all of them and ranks the thirteen that are reachable.

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build — see [verb/readme.md](verb/readme.md) |
| `docs/` | Design corpus — 13 definitions, 26 instances, 55 findings; see [docs/readme.md](docs/readme.md) |
| `src/lib.rs` | Crate root — the crate's whole public surface |
| `tests/` | `claim_test.rs` and the manual plan under `manual/` |
