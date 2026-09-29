# ring_shutdown

Publisher stop, drain, and waiter join.

Depends on [`ring_core`](../ring_core/readme.md), [`ring_cursor`](../ring_cursor/readme.md), [`ring_wait`](../ring_wait/readme.md), [`ring_types`](../ring_types/readme.md).

One of the 33 `ring_*` crates that make up this family's concurrency
write-path — a 33-crate dependency forest rooted at `ring_types`, acyclic
by construction. Build order follows [`../Cargo.toml`](../Cargo.toml)'s
member list; the family as a whole is described in
[`../readme.md`](../readme.md).

This crate holds the family's **only liveness flag**. Every other crate that
could plausibly carry an `is_closed` deliberately does not — most explicitly
`ring_core`, whose producer surface lists it as *settled — absent*, because a
handle carrying its own copy of liveness is the failure this crate exists to
prevent.

**The hard part of this crate's three operations is not any one of them in
isolation. It is that drain-all only terminates because close came first.**
A drain loop against an open ring with a live
producer never ends. So that ordering is not documented, it is typed:
`close()` returns a `Stopped` token, `drain_all` is a method on the token, and
`reopen` consumes it. A drain that did not follow a close cannot be written,
and neither can one that follows a reopen.

**What is still convention, and it is the thing to know before using this
crate:** nothing forces a producer to consult the flag. `Shutdown::guard`
wraps one so that it must — a `Guarded` producer's only push checks first — but
a caller holding a raw `ring_core::Producer` publishes into a closed ring
without complaint. The gap is the price of the flag living here rather than in
`ring_core`, and it is stated as a limitation in
[`docs/pitfall/001`](docs/pitfall/001_close_is_advisory_to_an_unguarded_producer.md)
rather than implied away.

```sh
cargo nextest run -p ring_shutdown    # 16 tests
cargo test --doc -p ring_shutdown     # 8 doc tests
```

| File | Responsibility |
|------|-----------------|
| `docs/` | 13 doc instances across 10 definitions — see [docs/readme.md](docs/readme.md) |
| `src/lib.rs` | The flag, the `Stopped` token, the guarded producer, and the two close-aware waits |
| `tests/shutdown_test.rs` | This crate's reached-test and 15 tests across the surface |
| `tests/manual/readme.md` | The four readings and measurements automation cannot make |
