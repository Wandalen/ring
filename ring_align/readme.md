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

- [`CACHE_LINE` follows the target architecture, using `crossbeam-utils`' table](docs/decisions/002_cache_line_follows_the_target_architecture.md)
- [`CACHE_LINE` is one unconditional 64 for every target](docs/decisions/001_cache_line_is_one_unconditional_constant.md), superseded by 002

## Known limitations

- `CacheAligned` pads every wrapped value to a whole line, 128 bytes on x86-64
  and AArch64, so a wrapped `u64` spends 120 of its 128 bytes on padding there.
  That pays only when another thread writes a neighbouring value. A wrapped
  value owns its whole line, so wrapping one of two hot fields already puts
  them on different lines. A build sees the architecture, not the processor,
  so a 64-byte-line part of a 128-byte architecture (AWS Graviton, most x86-64)
  gets twice the padding separation needs, as decision 002 records.

## Run it

```sh
cargo nextest run -p ring_align --all-features
cargo test --doc -p ring_align --all-features
```

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build. See the workspace [verb/readme.md](../verb/readme.md) |
| `docs/decisions/` | Architecture decision records |
| `docs/benchmarks/` | Before/after measurements of performance changes, each naming the machine it ran on |
| `src/lib.rs` | Crate root, holding the crate's whole public API |
| `tests/` | `align_test.rs` and the manual plan under `manual/` |
