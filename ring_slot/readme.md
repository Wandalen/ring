# ring_slot

Slot payload views: typed, bare `Copy`, and raw bytes.

Part of the `ring` family; [../readme.md](../readme.md) describes the whole.

The crate gives a ring three slot shapes. `TypedSlot<T>` holds traffic whose
type is known at compile time, `CopySlot<T>` holds a `Copy` record bare, without
`TypedSlot`'s tag, and `BytesSlot<N>` holds up to `N` opaque bytes for traffic
decoded later. All three implement `Slot`, which carries only `is_empty` and
`clear`, so code downstream is written against the trait and cannot branch on
which shape it holds. `ring_event` publishes and drains through these shapes.

## Decisions

- [`BytesSlot` marks occupancy with a length over a zeroed array, not with `MaybeUninit` or a flag byte](docs/decisions/001_bytes_slot_marks_occupancy_with_a_length.md)
- [`CopySlot` stores the record bare, and a slot published without a write hands back the previous lap's record](docs/decisions/002_copy_slot_stores_the_record_bare.md)

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
| `docs/benchmarks/` | Before/after measurements, each naming the machine it ran on. [001](docs/benchmarks/001_the_tag_store_against_an_untagged_slot.md): `TypedSlot`'s tag against `CopySlot` |
| `src/lib.rs` | Crate root, holding the crate's whole public API |
| `tests/` | `slot_test.rs` and the manual plan under `manual/` |
