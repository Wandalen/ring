# ring_index

Sequence-to-slot index mapping for power-of-two capacities.

Part of the `ring` family; [../readme.md](../readme.md) describes the whole.

`of` maps a `Seq` to the `SlotIndex` it addresses in a ring of a given
`Capacity`. Capacity is a power of two so that this fold is a bitmask rather
than a division. The validation lives in `ring_types::Capacity`, so `of` needs
no check and no error path.
[`tests/index_test.rs`](tests/index_test.rs) asserts the behaviour, and a reader
checks it by hand against [`tests/manual/readme.md`](tests/manual/readme.md).

## Run it

```sh
cargo nextest run -p ring_index --all-features
cargo test --doc -p ring_index --all-features
```

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build, described in the workspace [verb/readme.md](../verb/readme.md) |
| `src/lib.rs` | Crate root, holding the crate's whole public API |
| `tests/` | `index_test.rs` and the manual plan under `manual/` |
