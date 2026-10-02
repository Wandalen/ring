# ring_cursor

Producer and consumer sequence cursors, cache-line separated.

Part of the `ring` family; [../readme.md](../readme.md) describes the whole.

`ring_cursor` holds a cursor padded to a cache line, so that a producer and a
consumer on two cores stop invalidating each other's copy of a line they are
not sharing data through. [`tests/cursor_test.rs`](tests/cursor_test.rs)
asserts the behaviour, which is also read by hand against
[`tests/manual/readme.md`](tests/manual/readme.md). Every line is covered.

The padding *decision* lives in `ring_align`'s single `CACHE_LINE`, not here.
Keeping it there is what stops the family from acquiring two independent
answers to the same question. This crate holds the cursors that wear it.

## Decisions

- [`CursorPair`'s gating reads are fixed at `Acquire` and published once as `GATING`, not taken as a parameter](docs/decisions/001_gating_reads_are_fixed_at_acquire.md)

## Known limitations

- The layout tests check the result (`align_of`, `size_of`,
  `ring_cursor::CursorPair::on_distinct_lines`), so a
  `ring_cursor::PaddedCursor` that restated `#[repr(align(64))]` instead of
  wrapping `ring_align::CacheAligned` would still pass them. Only the manual
  plan's M1 (the padding comes from `ring_align`, not from a literal 64)
  guards the single source of the padding.

## Run it

```sh
cargo nextest run -p ring_cursor --all-features
cargo test --doc -p ring_cursor --all-features
```

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build. See the workspace [verb/readme.md](../verb/readme.md) |
| `docs/decisions/` | Architecture decision records |
| `src/lib.rs` | Crate root, holding the crate's whole public API |
| `tests/` | `cursor_test.rs` and the manual plan under `manual/` |
