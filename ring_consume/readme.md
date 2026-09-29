# ring_consume

Single-consumer available-range computation and commit.

Depends on [`ring_types`](../ring_types/readme.md), [`ring_cursor`](../ring_cursor/readme.md), [`ring_barrier`](../ring_barrier/readme.md), [`ring_seqno`](../ring_seqno/readme.md).

One of the 33 `ring_*` crates that make up this family's concurrency
write-path — a 33-crate dependency forest rooted at `ring_types`, acyclic
by construction. Build order follows [`../Cargo.toml`](../Cargo.toml)'s
member list; the family as a whole is described in
[`../readme.md`](../readme.md).

Implemented, delivering the consumer half of the four-operation handshake — what a consumer may
read right now (`available`), and reporting how far it got (`commit`). Behaviour
is asserted by [`tests/consume_test.rs`](tests/consume_test.rs) and read by hand
against [`tests/manual/readme.md`](tests/manual/readme.md); every line is
covered. The four-operation handshake this closes is asserted end to end in
[`ring_publish/tests/handshake_test.rs`](../ring_publish/tests/handshake_test.rs).

A `Consumer` *borrows* the cursor it reports into, and the lender is almost
always a producer's `ring_gating::GatingSet`. That is the whole mechanism: the
producer decides what it may overwrite by reading the cursors in its set, so a
consumer holding a private cursor would be invisible to it. Such a ring
compiles, runs, passes every single-threaded test, and overwrites unread slots
on the first lap — which is why the handshake test asserts the wiring with
`ptr::eq` rather than trusting it.

`commit` refuses in both directions. Past what is available would free slots
that were never read; behind the current position would re-read slots the
producer has already been cleared to reuse. Neither is caught by a suite that
only ever commits exactly what `available` returned, so the test file sweeps
every position/candidate pair up to the frontier instead.

Design documentation is a full typed corpus — 13 doc definitions, 26 instances,
54 verified findings, each backed by a command whose output is quoted where it is
used. Twenty-four of the findings are about other crates, which is what
documenting a Tier 5 primitive is for. Start at
[`docs/readme.md`](docs/readme.md); every finding is indexed and ranked by
severity in [`docs/definition/readme.md`](docs/definition/readme.md).

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build — see [verb/readme.md](verb/readme.md) |
| `docs/` | The design corpus — 13 definitions, 26 instances, 54 findings; see [docs/readme.md](docs/readme.md) |
| `src/lib.rs` | Crate root — the crate's whole public surface |
| `tests/` | `consume_test.rs` and the manual plan under `manual/` |
