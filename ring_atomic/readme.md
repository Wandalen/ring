# ring_atomic

Atomic sequence helpers with explicit memory orderings.

Part of the `ring` family; [../readme.md](../readme.md) describes the whole.

Every cursor in the family is a sequence in a shared cell, and this crate holds
those cells so the family's memory orderings can be read in one place.
`SeqCell` is the cell, `AtomicSeq` implements it for production, and
`CountingSeq` implements it for tests that count atomic operations. Every method
takes an explicit `Ordering`, and the crate never picks one for a caller.
[`tests/atomic_test.rs`](tests/atomic_test.rs) asserts the behaviour, which is
also read by hand against [`tests/manual/readme.md`](tests/manual/readme.md).

## Run it

```sh
cargo nextest run -p ring_atomic --all-features
cargo test --doc -p ring_atomic --all-features
```

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build. See the workspace [verb/readme.md](../verb/readme.md) |
| `src/lib.rs` | Crate root, holding the crate's whole public API |
