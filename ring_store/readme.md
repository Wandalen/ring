# ring_store

Power-of-two slot storage array.

Part of the `ring` family; [../readme.md](../readme.md) describes the whole.

`Buffer<S>` is a ring's slots and nothing else. `Buffer::new` allocates as many
`Default` slots as its `Capacity`, once. The buffer addresses them by
`ring_types::SlotIndex` and leaves the fold from a sequence to `ring_index`. It
holds no cursor and no ordering state.
An SPSC ring and an MPSC ring want different cursor arrangements, so a buffer
that owned one would serve only one of them. There is no `unsafe`.

## Decisions

- [Ring capacity is a runtime `Capacity` over one heap allocation, while slot width stays a type parameter](docs/decisions/001_capacity_is_a_value_and_width_is_a_type.md)

## Known limitations

- `ring_store::Buffer::is_empty` always returns `false`, because a `Capacity` is
  never zero. A caller asking whether anything is stored wants
  `ring_store::Buffer::all_empty`, which sits next to it.

## Run it

```sh
cargo nextest run -p ring_store --all-features
cargo test --doc -p ring_store --all-features
```

| File | Responsibility |
|------|-----------------|
| `docs/decisions/` | Architecture decision records |
| `src/lib.rs` | Crate root, holding the crate's whole public API |
| `tests/` | `buffer_test.rs` and the manual plan under `manual/` |
