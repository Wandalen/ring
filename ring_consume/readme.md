# ring_consume

Single-consumer available-range computation and commit.

Depends on [`ring_types`](../ring_types/readme.md), [`ring_cursor`](../ring_cursor/readme.md), [`ring_barrier`](../ring_barrier/readme.md), [`ring_seqno`](../ring_seqno/readme.md).

One of the 33 `ring_*` crates that make up this family's concurrency
write-path. The 33 crates form a dependency forest rooted at `ring_types`,
acyclic by construction. Build order follows [`../Cargo.toml`](../Cargo.toml)'s
member list. [`../readme.md`](../readme.md) describes the family as a whole.

Implemented. It delivers the consumer half of the four-operation handshake.
`available` says what a consumer may read right now, and `commit` reports how
far it got. [`tests/consume_test.rs`](tests/consume_test.rs) asserts the
behaviour, which is also read by hand against
[`tests/manual/readme.md`](tests/manual/readme.md). Every line is covered.
[`ring_publish/tests/handshake_test.rs`](../ring_publish/tests/handshake_test.rs)
asserts end to end the four-operation handshake that this crate closes.

A `Consumer` *borrows* the cursor it reports into, and the lender is almost
always a producer's `ring_gating::GatingSet`. That is the whole mechanism. The
producer decides what it may overwrite by reading the cursors in its set, so it
never sees a consumer that holds a private cursor. Such a ring compiles, runs,
passes every single-threaded test, and overwrites unread slots on the first
lap. That is why the handshake test asserts the wiring with `ptr::eq` rather
than trusting it.

`commit` refuses in both directions. A commit past what is available would
free slots that were never read. A commit behind the current position would
re-read slots the producer has already been cleared to reuse. A suite that only
ever commits exactly what `available` returned catches neither, so the test file
sweeps every position/candidate pair up to the frontier instead.

Design documentation is a full typed corpus of 13 doc definitions, 26 instances
and 54 verified findings, each backed by a command whose output is quoted where
it is used. Twenty-four of the findings are about other crates, which is what
documenting a Tier 5 crate is for. Start at [`docs/readme.md`](docs/readme.md).
[`docs/definition/readme.md`](docs/definition/readme.md) indexes every finding
and ranks it by severity.

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build. See the workspace [verb/readme.md](../verb/readme.md) |
| `docs/` | The design corpus of 13 definitions, 26 instances and 54 findings. See [docs/readme.md](docs/readme.md) |
| `src/lib.rs` | Crate root, holding the crate's whole public API |
| `tests/` | `consume_test.rs` and the manual plan under `manual/` |
