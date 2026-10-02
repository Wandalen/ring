# ring_seqno

Sequence numbers and their comparison across laps.

Part of the `ring` family; [../readme.md](../readme.md) describes the whole.

It provides never-wrapping sequence comparison and the free-slot and pending
arithmetic that producer and consumer bounds are built on. The sequence does not
wrap; the slot index does. Two publications `capacity` apart land on the same
slot, but their `ring_types::Seq` values differ by exactly `capacity`, and that
difference is what a gate reads to decide whether the older one has been
consumed. Folding a sequence into a slot index is `ring_index`'s job, kept in
another crate so that no function does both.

[`tests/seq_test.rs`](tests/seq_test.rs) asserts the behaviour, and a reader
checks it by hand against [`tests/manual/readme.md`](tests/manual/readme.md).

## Run it

```sh
cargo nextest run -p ring_seqno --all-features
cargo test --doc -p ring_seqno --all-features
```

| File | Responsibility |
|------|-----------------|
| `src/lib.rs` | Crate root, holding the crate's whole public API |
