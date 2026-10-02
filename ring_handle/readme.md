# ring_handle

Shareable producer and consumer ends.

Part of the `ring` family; [../readme.md](../readme.md) describes the whole.

**`ring_core` already partitions these capabilities.** Its `Producer` cannot
drain and its `Consumer` cannot publish, so the producer/consumer split is
not invented here. What this crate adds is four narrowings, and the list is
short enough to read at once:

| Added | What it prevents |
|---|---|
| The ring is taken **by value** | Splitting the same ring twice, which `Ring::ends`'s borrow permits |
| `try_clone` is **withheld** | A second producer, refused at compile time here and at runtime one crate down |
| A drain iterator bounded at call time | A full drain that never terminates under a live producer |
| Nothing reaches the backend | No `Deref`, no `inner()`, no public field, on any of the three types |

**Two things this crate deliberately leaves out.** `is_closed()` would need
`ring_shutdown`, which depends on `ring_wait`. That would put a parking
operation within reach of the tick path and break the non-parking guarantee
`ring_poll` enforces. `&self` receivers are left out because `&mut self` is
what makes "exactly one producer" a borrow-checker property rather than a
convention.

## Decisions

- [`ring_handle` stays a separate narrowing layer over `ring_core`'s ends](docs/decisions/001_handles_are_a_narrowing_layer_over_ring_core.md)
- [Neither handle has `is_closed()`; close-awareness comes from wrapping a producer in `ring_shutdown::Guarded`](docs/decisions/002_handles_have_no_is_closed.md)

## Known limitations

- The compile-fail guarantees in `tests/ui_test.rs` rest on pinned trybuild
  `.stderr` files under `tests/ui/`. A rustc upgrade, or a rename of
  `ring_core`'s private `ProducerInner`, breaks this crate's suite from a crate
  it does not control. `TRYBUILD=overwrite`, the regeneration command, accepts a
  real regression just as readily, so reviewing the diff is the only way to
  tell the two apart.

## Run it

```sh
cargo nextest run -p ring_handle --all-features
cargo test --doc -p ring_handle --all-features
```

| File | Responsibility |
|------|-----------------|
| `docs/decisions/` | Architecture decision records |
| `src/lib.rs` | `Split`, `Ends`, the two handles, and the bounded `Drain` |
| `tests/handle_test.rs` | This crate's runtime half, across the public API |
| `tests/ui_test.rs` | This crate's compile-fail half, and the family's only test that gets redder as the public API grows |
| `tests/ui/` | The programs that must not compile, each with a pinned `.stderr` |
| `tests/manual/readme.md` | The readings and measurements automation cannot make |
