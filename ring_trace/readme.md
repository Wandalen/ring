# ring_trace

Optional sequence-operation trace log.

Part of the `ring` family; [../readme.md](../readme.md) describes the whole.

`Trace` records one `TraceEntry` per sequence operation when enabled, and
nothing when disabled, which is the default. The enabled flag is fixed at
construction, and a disabled `Trace::record` returns before it touches the
lock. `ring_stats` counts operations in constant space and is always on, while
a trace records which operations ran, in the order producers took its lock,
and grows without bound.

[`tests/trace_test.rs`](tests/trace_test.rs) asserts the
behaviour, and a reader checks it by hand against
[`tests/manual/readme.md`](tests/manual/readme.md). Every line is covered.

## Run it

```sh
cargo nextest run -p ring_trace --all-features
cargo test --doc -p ring_trace --all-features
```

| File | Responsibility |
|------|-----------------|
| `src/lib.rs` | Crate root, holding the crate's whole public API |
