# ring_slot

Slot payload views: typed and raw bytes.

Part of the `ring` family; [../readme.md](../readme.md) describes the whole.

The crate gives a ring two slot shapes. `TypedSlot<T>` holds traffic whose type
is known at compile time, and `BytesSlot<N>` holds up to `N` opaque bytes for
traffic decoded later. Both implement `Slot`, which carries only `is_empty` and
`clear`, so code downstream is written against the trait and cannot branch on
which shape it holds. `ring_event` publishes and drains through these shapes.

## Decisions

- [`BytesSlot` marks occupancy with a length over a zeroed array, not with `MaybeUninit` or a flag byte](docs/decisions/001_bytes_slot_marks_occupancy_with_a_length.md)

## Known limitations

- The copy-cost asymmetry that justifies having both `ring_slot::TypedSlot` and
  `ring_slot::BytesSlot` is argued but not measured, because `ring_bench`
  instantiates only `TypedSlot` and never `BytesSlot`.
- `ring_slot::Slot` neither requires nor mentions `Default`, yet
  `ring_store::Buffer::new`, `ring_spsc::Ring::new` and `ring_mpsc::Ring::new`
  all need `S: Default`. A new slot shape that implements only `Slot` fails to
  compile in a crate its author never opened.

## Run it

```sh
cargo nextest run -p ring_slot --all-features
cargo test --doc -p ring_slot --all-features
```

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build. See the workspace [verb/readme.md](../verb/readme.md) |
| `docs/decisions/` | Architecture decision records |
| `src/lib.rs` | Crate root, holding the crate's whole public API |
| `tests/` | `slot_test.rs` and the manual plan under `manual/` |
