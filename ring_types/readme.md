# ring_types

Shared ids, errors, and policy enums for the ring family, with no ring logic.

Part of the `ring` family; [../readme.md](../readme.md) describes the whole.

`Seq` is a publication's position in a ring's whole history, and `SlotIndex` is the wrapping slot derived from it.
Keeping them as separate types lets a gate compare two positions a full lap apart. `Capacity` is a slot count
validated to a power of two, `WaitKind` and `OverflowPolicy` are the two configuration enums, and `RingError` is the
error type the ring path returns. The "no ring logic" rule means this crate owns the discriminants of `WaitKind` and
`OverflowPolicy`, and `ring_wait` and `ring_overflow` own the handlers that act on them.
[`tests/types_test.rs`](tests/types_test.rs) asserts the behaviour, which is also read by hand against
[`tests/manual/readme.md`](tests/manual/readme.md). Every line is covered.

## Decisions

- [Delete `RingError::NameTaken` and `RingError::NameUnknown`, which nothing constructs](docs/decisions/001_delete_the_unconstructed_name_errors.md)

## Known limitations

- The summary of `ring_types::RingError::is_transient` says a retry "could succeed without anything else changing".
  That is false for `RingError::Full` against a stalled consumer. Only the second sentence, which says both
  conditions clear when a peer makes progress, is right.
- `ring_types::Capacity` has no `TryFrom<usize>` impl, so the family has no `try_into()` path to it. A non-`const`
  impl that delegates to the `const fn` `Capacity::new` compiles on stable today; only a `const` impl would need
  `const_trait_impl`.

## Run it

```sh
cargo nextest run -p ring_types --all-features
cargo test --doc -p ring_types --all-features
```

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build. See the workspace [verb/readme.md](../verb/readme.md) |
| `docs/decisions/` | Architecture decision records |
| `src/lib.rs` | Crate root, holding the crate's whole public API |
