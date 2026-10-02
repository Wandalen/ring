# ring_align

Cache-line padding constants and alignment wrappers.

Part of the `ring` family; [../readme.md](../readme.md) describes the whole.

It gives a value a cache line of its own, so two cursors written by different
cores do not contend on one line. `CACHE_LINE` is the line size,
`CacheAligned<T>` pads a value to whole lines, and `on_distinct_lines` checks
two addresses. [`tests/align_test.rs`](tests/align_test.rs) asserts the
behaviour, which is also read by hand against
[`tests/manual/readme.md`](tests/manual/readme.md).

## Decisions

- [`CACHE_LINE` is one unconditional 64 for every target, raised by hand at port time](docs/decisions/001_cache_line_is_one_unconditional_constant.md)

## Known limitations

- `CacheAligned` pads every wrapped value to a whole 64-byte line, so a wrapped
  `u64` spends 56 of its 64 bytes on padding. That pays only when another
  thread writes a neighbouring value. A wrapped value owns its whole line, so
  wrapping one of two hot fields already puts them on different lines. The
  128-byte-line host is covered in the decision above.

## Run it

```sh
cargo nextest run -p ring_align --all-features
cargo test --doc -p ring_align --all-features
```

| File | Responsibility |
|------|-----------------|
| `docs/decisions/` | Architecture decision records |
| `src/lib.rs` | Crate root, holding the crate's whole public API |
| `tests/` | `align_test.rs` and the manual plan under `manual/` |
