# ring_event

Slot translators that fill a claimed slot.

Part of the `ring` family; [../readme.md](../readme.md) describes the whole.

`ring_event` holds the publish and drain halves of slot translation.
[`tests/event_test.rs`](tests/event_test.rs) asserts the behaviour, which is also
read by hand against [`tests/manual/readme.md`](tests/manual/readme.md). Every
line is covered.

## Run it

```sh
cargo nextest run -p ring_event --all-features
cargo test --doc -p ring_event --all-features
```

| File | Responsibility |
|------|-----------------|
| `src/lib.rs` | Crate root, holding the crate's whole public API |
