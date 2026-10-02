# ring_publish

Publication of claimed slots to consumers.

Part of the `ring` family; [../readme.md](../readme.md) describes the whole.

It delivers the publish half of the four-operation handshake. A producer
advances the published cursor only when it has *finished writing* the range it
claimed, and only when its predecessor has already published.
[`tests/publish_test.rs`](tests/publish_test.rs) asserts the behaviour, which is
also read by hand against [`tests/manual/readme.md`](tests/manual/readme.md).

The end-to-end test of the whole handshake lives here too, in
[`tests/handshake_test.rs`](tests/handshake_test.rs). It runs claim → publish →
available → commit as two test suites. Real threads carry many items through a
small ring; under `--cfg loom` the same file becomes an exhaustive model of one
claim against one drain, checking every interleaving. Publication is where that
test belongs because it is the moment the other three operations become
observable together. Before it a claim is invisible; after it the consumer's
whole contract is decided.

The published cursor is deliberately *not* `ring_claim`'s cursor. Between a
producer taking a range and finishing it, the slot is claimed and unwritten, and
the handshake's central requirement is that no consumer sees it. Conflating the
two publishes uninitialised memory, passes every single-threaded test, and fails
only under load. That is the bug the loom model exists to catch, and it does
catch it. Weakening the publish ordering to `Relaxed` still passes every
threaded test on x86, and fails the model immediately.

The crate's executable content is a compare-exchange and a loop around it. What
it holds instead is a boundary. The boundary is the single moment a consumer is
allowed to see a slot, and the guarantee that it sees nothing earlier.

## Decisions

- [`Publisher::publish` takes a bare `(start, len)`, so publishing exactly the claimed range is the caller's job](docs/decisions/001_publish_takes_a_bare_start_and_len.md)

## Run it

```sh
cargo nextest run -p ring_publish --all-features
cargo test --doc -p ring_publish --all-features
```

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build. See the workspace [verb/readme.md](../verb/readme.md) |
| `docs/decisions/` | Architecture decision records |
| `src/lib.rs` | Crate root, holding the crate's whole public API |
| `tests/` | `publish_test.rs`, the handshake's `handshake_test.rs`, and the manual plan under `manual/` |
