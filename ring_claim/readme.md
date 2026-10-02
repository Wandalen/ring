# ring_claim

Sequence-range claiming without waiting.

Part of the `ring` family; [../readme.md](../readme.md) describes the whole.

It is the claim half of the four-operation handshake
(claim → publish → available → commit) and holds the exclusivity guarantee that
no two producers are ever granted the same sequence.
[`tests/claim_test.rs`](tests/claim_test.rs) asserts the behaviour, and a reader
checks it by hand against [`tests/manual/readme.md`](tests/manual/readme.md).

Claiming never waits. Every call returns a range or says why not. That lets
the same call serve a spinning producer, a parking producer, and the tick path
that must not block at all. A caller that wants to wait calls
`ring_wait::for_space` first. The grant itself is a compare-exchange loop
rather than a `fetch_add`, because the gate check and the advance have to be
one atomic step. The manual plan's first check is a mutation run that
deliberately breaks this and confirms the suite notices.

## Run it

```sh
cargo nextest run -p ring_claim --all-features
cargo test --doc -p ring_claim --all-features
```

| File | Responsibility |
|------|-----------------|
| `src/lib.rs` | Crate root, holding the crate's whole public API |
| `tests/` | `claim_test.rs` and the manual plan under `manual/` |
