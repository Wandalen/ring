# ring_spsc

Single-producer single-consumer ring API.

Part of the `ring` family; [../readme.md](../readme.md) describes the whole.

A `Ring` owns the slot array and two cursors; `split` hands out a `Producer`
and a `Consumer` to be moved onto two threads. The producer claims a slot and
publishes it when the guard drops; the consumer drains everything published
since its last batch and commits when *that* guard drops. Neither path takes a
lock or performs a read-modify-write. The whole handshake is two release
stores, one per end.

**What single-producer buys is a shorter dependency list.** `ring_gating`,
`ring_claim`, `ring_publish` and `ring_consume` are all absent, because each
answers a question that only has an answer worth computing when producers can
overtake one another. The [module documentation](src/lib.rs) works through the
four in turn.

The `unsafe` that lets two threads touch one allocation lives here, not in
`ring_store` or `ring_slot`. [docs/workaround/readme.md](docs/workaround/readme.md)
records why the crate opts out of the workspace `unsafe` deny, what bounds the
unsafe code, and when to delete it.

The soundness argument is about *orderings*, so the ordinary suite cannot
falsify it. A 100 000-item run passes just as readily against a publish that
forgot its `Release`, even on a weakly ordered aarch64 host, because the window
is too narrow for sampling to open. A `loom` model checks every interleaving of
a two-record case instead.

## Known limitations

- A dropped `ring_spsc::Consumer` leaves its `Producer` returning
  `RingError::Full` from `Producer::claim` forever, which looks the same as a
  slow consumer. Neither the doc of `ring_types::RingError::Full` nor this crate
  points at `ring_shutdown`, which carries the closed signal.
- A `ring_spsc::Reservation` publishes on drop even if nothing was written
  through it. An early `?` between `Producer::claim` and the write delivers the
  slot as it stands. That is empty on the first lap, and otherwise whatever
  record the previous lap left there if the consumer did not take it.
- `ring_spsc::Producer::free_capacity` is exact, but callers going through
  `ring_core::Producer::free_capacity` must treat it as advisory because that
  contract is the same across backends, so the exactness has no consumer.
- The `--cfg loom` model in `tests/spsc_test.rs` reaches loom's instrumented
  atomics only through `ring_cursor`'s dependency on `ring_atomic`, which
  `ring_spsc/Cargo.toml` never names. Removing that edge from `ring_cursor`
  would silently make loom explore uninstrumented code. Naming `ring_atomic`
  under `[target.'cfg(loom)'.dev-dependencies]` would record the dependency,
  and `ring_mpsc`'s manifest, which shares the wording, should change with it.

## Run it

```sh
cargo nextest run -p ring_spsc --all-features
cargo test --doc -p ring_spsc --all-features
RUSTFLAGS="--cfg loom" cargo test -p ring_spsc --test spsc_test
```

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build. See the workspace [verb/readme.md](../verb/readme.md) |
| `docs/workaround/` | Why the crate opts out of the workspace `unsafe` deny. See [docs/workaround/readme.md](docs/workaround/readme.md) |
| `src/lib.rs` | The `Ring`, its two ends, and the two publish/commit guards |
| `tests/spsc_test.rs` | This crate's reached-test, the tests around the public API, and a `loom` model of every interleaving |
| `tests/manual/readme.md` | The source readings and mutation checks automation cannot make |
