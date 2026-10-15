# ring_config

Ring construction parameters.

Part of the `ring` family; [../readme.md](../readme.md) describes the whole.

`RingConfig` collects everything that varies between rings into one value:
capacity, wait strategy, overflow policy, producer count and batch size. This
crate holds the record and its validation, and `ring_factory` turns one into a
ring. A configuration reads as one expression, `RingConfig::new(slots)?`
followed by `with_*` setters.
[`tests/config_test.rs`](tests/config_test.rs) asserts the behaviour, which is also
read by hand against [`tests/manual/readme.md`](tests/manual/readme.md). Every
line is covered.

## Decisions

- [Capacity is `RingConfig::new`'s argument and its only rejection; every other field has an infallible setter](docs/decisions/001_capacity_is_a_constructor_argument_not_a_setter.md)

## Run it

```sh
cargo nextest run -p ring_config --all-features
cargo test --doc -p ring_config --all-features
```

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build. See the workspace [verb/readme.md](../verb/readme.md) |
| `docs/decisions/` | Architecture decision records |
| `src/lib.rs` | Crate root, holding the crate's whole public API |
