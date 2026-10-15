# ring_overflow

Full-ring overflow policies.

Part of the `ring` family; [../readme.md](../readme.md) describes the whole.

This crate holds the handlers for `ring_types::OverflowPolicy`. Given a full
ring and a policy, they decide what happens to the data and say so with a
`Resolution` instead of a boolean. `resolve` also records the event in
`RingStats`, and it counts on every call, the `Err` path of
`OverflowPolicy::Fail` included, so a retry double-counts. `would_resolve` is
the same mapping without the counter, for a caller that only decides or holds no
`RingStats`.

[`tests/overflow_test.rs`](tests/overflow_test.rs) asserts the behaviour, and a
reader checks it by hand against [`tests/manual/readme.md`](tests/manual/readme.md).

## Run it

```sh
cargo nextest run -p ring_overflow --all-features
cargo test --doc -p ring_overflow --all-features
```

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build. See the workspace [verb/readme.md](../verb/readme.md) |
| `src/lib.rs` | Crate root, holding the crate's whole public API |
