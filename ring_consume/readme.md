# ring_consume

Single-consumer available-range computation and commit.

Part of the `ring` family; [../readme.md](../readme.md) describes the whole.

`ring_consume` holds the consumer half of the four-operation handshake (claim,
publish, available, commit). `available` says what a consumer may read right now, and `commit` reports how
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

## Known limitations

- The read chain pulls in blocking code it never calls. `ring_consume` depends
  on `ring_barrier`, which depends on `ring_wait` unconditionally for
  `ring_barrier::Barrier::wait_for`. `ring_consume` never calls `wait_for`, yet
  `ring_wait`, which calls `std::thread::yield_now` and `std::thread::sleep`,
  stays in its dependency tree. The chain cannot be made `no_std` or free of
  blocking code without a change to `ring_barrier`'s manifest.

## Run it

```sh
cargo nextest run -p ring_consume --all-features
cargo test --doc -p ring_consume --all-features
```

| File | Responsibility |
|------|-----------------|
| `src/lib.rs` | Crate root, holding the crate's whole public API |
| `tests/` | `consume_test.rs` and the manual plan under `manual/` |
