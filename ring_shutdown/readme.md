# ring_shutdown

Publisher stop, drain, and waiter join.

Depends on [`ring_core`](../ring_core/readme.md), [`ring_cursor`](../ring_cursor/readme.md), [`ring_wait`](../ring_wait/readme.md), [`ring_types`](../ring_types/readme.md).

One of the 33 `ring_*` crates that make up this family's concurrency
write-path. The 33 crates form a dependency forest rooted at `ring_types`,
acyclic by construction. Build order follows [`../Cargo.toml`](../Cargo.toml)'s
member list. [`../readme.md`](../readme.md) describes the family as a whole.

This crate holds the family's **only liveness flag**. Every other crate that
could plausibly carry an `is_closed` deliberately does not. The clearest case is
`ring_core`, whose producer API lists `is_closed` as absent and *settled*,
because a handle carrying its own copy of liveness is the failure this crate
exists to prevent.

**None of this crate's three operations is hard alone. The hard part is that
drain-all only terminates because close came first.** A drain loop against an
open ring with a live producer never ends. So the crate enforces that ordering
with types. `close()` returns a `Stopped` token, `drain_all` is a method on the
token, and `reopen` consumes it. A caller cannot write a drain that did not
follow a close, or one that follows a reopen.

**One rule is still convention, and it is the thing to know before using this
crate.** Nothing forces a producer to consult the flag. `Shutdown::guard` wraps
a producer so that it must, because a `Guarded` producer's only push checks
first. A caller holding a raw `ring_core::Producer`, though, publishes into a
closed ring without complaint. The gap is the price of the flag living here
rather than in `ring_core`.
[`docs/pitfall/001`](docs/pitfall/001_close_is_advisory_to_an_unguarded_producer.md)
states it as a limitation rather than implying it away.

```sh
cargo nextest run -p ring_shutdown    # 16 tests
cargo test --doc -p ring_shutdown     # 8 doc tests
```

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build. See the workspace [verb/readme.md](../verb/readme.md) |
| `docs/` | 13 doc instances across 10 definitions. See [docs/readme.md](docs/readme.md) |
| `src/lib.rs` | The flag, the `Stopped` token, the guarded producer, and the two close-aware waits |
| `tests/shutdown_test.rs` | This crate's reached-test and 15 tests across the public API |
| `tests/manual/readme.md` | The four readings and measurements automation cannot make |
