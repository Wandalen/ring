# ring_mpsc

Sequence-numbered multi-producer ring buffer. Many threads claim and publish
concurrently, and one consumer thread drains in total order.

Part of the `ring` family; [../readme.md](../readme.md) describes the whole.

More than one independent consumer needs this mechanism, so it is factored out
and built once. Publication is a per-slot stamp write and a scan, not a cursor
advance. A slot is published when its stamp equals the sequence addressing it,
so no producer waits for another to publish and the consumer pays a scan
instead. That shape is why the crate uses neither `ring_publish` nor
`ring_consume`.

The crate opts out of the workspace `unsafe` deny so many producers can write
disjoint slots of one array. [docs/workaround/readme.md](docs/workaround/readme.md)
records why, what bounds the unsafe code, and when to delete it.

## Known limitations

- `ring_mpsc::Ring::committed`, `Ring::published_through`, `Ring::stamps`,
  `Producer::claimed`, `Producer::on_distinct_lines`, `Consumer::position` and
  `Batch::sequences` have no production caller. They are public only because the
  crate's integration tests can reach nothing else.
- `ring_mpsc::Ring::stamps` returns the raw `&[AtomicSeq]`, so a caller gets the
  stamp values without the rule that a slot is published only when its stamp
  equals its sequence. Only the private `Ring::contiguous_end` applies that rule,
  and `!= UNSTAMPED` or `>=` would get it wrong by reading a previous lap's stamp
  as published.
- The private `ring_mpsc::Ring::stamp` masks the sequence with
  `Ring::capacity()`, which is the `GatingSet`'s copy, while the stamp array was
  sized from `Ring::new`'s argument. The index stays in bounds only because
  `Ring::new` passes the same `Capacity` to both, not for the power-of-two
  reason the comment in `stamp` gives.
- A `ring_mpsc::Reserved` publishes on drop even if nothing was written
  through it. An early `?` between `Producer::claim` and the write delivers the
  slot as it stands. That is empty on the first lap, and otherwise whatever
  record the previous lap left there if the consumer did not take it.

## Run it

```sh
cargo nextest run -p ring_mpsc --all-features
cargo test --doc -p ring_mpsc --all-features
RUSTFLAGS="--cfg loom" cargo nextest run -p ring_mpsc exhaustive::
```

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build. See the workspace [verb/readme.md](../verb/readme.md) |
| `docs/workaround/` | Why the crate opts out of the workspace `unsafe` deny. See [docs/workaround/readme.md](docs/workaround/readme.md) |
| `src/lib.rs` | The ring, its two handles, and the two RAII guards that publish and commit |
| `tests/` | Integration tests and the loom models. [tests/manual/readme.md](tests/manual/readme.md) records what was checked by hand |
