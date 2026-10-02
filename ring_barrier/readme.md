# ring_barrier

Consumer barrier over the minimum of dependent cursors.

Part of the `ring` family; [../readme.md](../readme.md) describes the whole.

It is the consumer half of the gating mechanism, which decides how far a
consumer may read, given the minimum across everything it depends on.
`ring_gating` is the producer half and reads the same set from the other side.
[`tests/barrier_test.rs`](tests/barrier_test.rs) asserts the behaviour, which is
also read by hand against [`tests/manual/readme.md`](tests/manual/readme.md).

The difference from `ring_gating` is that capacity is not in the answer. A
producer may run one lap ahead of the slowest consumer and no further, because
capacity is what makes the next slot the one that consumer is reading. A
consumer may read up to whatever its dependencies have finished, and the number
of slots the ring has does not enter into it. The two share the cursors, not the
arithmetic. That is why `Barrier::over` takes a bare `&[ PaddedCursor ]` and
never names `ring_gating`. A barrier points at whatever cursors it depends on,
wherever they live. A gating set's are one such place, a publisher's is another.

## Run it

```sh
cargo nextest run -p ring_barrier --all-features
cargo test --doc -p ring_barrier --all-features
```

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build. See the workspace [verb/readme.md](../verb/readme.md) |
| `src/lib.rs` | Crate root, holding the crate's whole public API |
| `tests/` | `barrier_test.rs` and the manual plan under `manual/` |
