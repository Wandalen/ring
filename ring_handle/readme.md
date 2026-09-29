# ring_handle

Shareable producer and consumer ends.

Depends on [`ring_core`](../ring_core/readme.md).

One of the 33 `ring_*` crates that make up this family's concurrency
write-path — a 33-crate dependency forest rooted at `ring_types`, acyclic
by construction. Build order follows [`../Cargo.toml`](../Cargo.toml)'s
member list; the family as a whole is described in
[`../readme.md`](../readme.md).

**`ring_core` already partitions these capabilities** — its `Producer` cannot
drain and its `Consumer` cannot publish — so the producer/consumer split is
not invented here. What this crate adds is four narrowings, and the honest
version of that list is short enough to read at once:

| Added | What it prevents |
|---|---|
| `verb/` | Crate-scoped test/lint/build — see [verb/readme.md](verb/readme.md) |
| The ring is taken **by value** | Splitting the same ring twice, which `Ring::ends`'s borrow permits |
| `try_clone` is **withheld** | A second producer — refused at compile time here, at runtime one crate down |
| A drain iterator bounded at call time | A full drain that never terminates under a live producer |
| Nothing reaches the backend | No `Deref`, no `inner()`, no public field, on any of the three types |

Whether four narrowings justify a crate on the five-name export surface is an
open question, recorded at
[`docs/decisions/001`](docs/decisions/001_what_this_crate_is_for.md) rather than
argued away.

**Two things this crate deliberately leaves out.** `is_closed()`
would need `ring_shutdown`, which depends on `ring_wait` — putting a parking
operation within reach of the tick path and breaking the non-parking guarantee
`ring_poll` enforces, whose guard
in `ring_poll` would fail on the next run
([`docs/decisions/002`](docs/decisions/002_why_is_closed_is_absent.md)).
`&self` receivers were wrong: `&mut self` is what makes "exactly one producer"
a borrow-checker property rather than a convention.

```sh
cargo nextest run -p ring_handle       # 12 tests, one of which compiles 5 programs that must not
cargo test --doc -p ring_handle        # 2 doc tests
```

| File | Responsibility |
|------|-----------------|
| `docs/` | 21 doc instances across 12 definitions — see [docs/readme.md](docs/readme.md) |
| `src/lib.rs` | `Split`, `Ends`, the two handles, and the bounded `Drain` |
| `tests/handle_test.rs` | This crate's runtime half, and 11 tests across the surface |
| `tests/ui_test.rs` | This crate's compile-fail half — the family's only test that gets redder as the surface grows |
| `tests/ui/` | The five programs that must not compile, each with pinned `.stderr` |
| `tests/manual/readme.md` | The readings and measurements automation cannot make |
