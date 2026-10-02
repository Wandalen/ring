# ring_batch

Batch claim objects spanning a sequence range.

Part of the `ring` family; [../readme.md](../readme.md) describes the whole.

`claim` takes a contiguous range of sequences with one `fetch_add`, whatever
the batch size. `claim_gated` checks for room first and is safe for one
producer only. With several producers, use `ring_claim::Claimer::claim`, which
re-checks the room inside a compare-exchange retry. `ring_claim` has its own
range type, and neither crate depends on the other.
[`tests/batch_test.rs`](tests/batch_test.rs) asserts the behaviour, and a reader
checks it by hand against [`tests/manual/readme.md`](tests/manual/readme.md).

## Run it

```sh
cargo nextest run -p ring_batch --all-features
cargo test --doc -p ring_batch --all-features
```

| File | Responsibility |
|------|-----------------|
| `src/lib.rs` | Crate root, holding the crate's whole public API |
| `tests/` | `batch_test.rs` and the manual plan under `manual/` |
