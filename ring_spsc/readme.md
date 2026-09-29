# ring_spsc

Single-producer single-consumer ring API.

Depends on [`ring_store`](../ring_store/readme.md), [`ring_config`](../ring_config/readme.md), [`ring_cursor`](../ring_cursor/readme.md), [`ring_slot`](../ring_slot/readme.md), [`ring_types`](../ring_types/readme.md).

One of the 33 `ring_*` crates that make up this family's concurrency
write-path — a 33-crate dependency forest rooted at `ring_types`, acyclic
by construction. Build order follows [`../Cargo.toml`](../Cargo.toml)'s
member list; the family as a whole is described in
[`../readme.md`](../readme.md).

A `Ring` owns the slot array and two cursors; `split` hands out a `Producer`
and a `Consumer` to be moved onto two threads. The producer claims a slot and
publishes it when the guard drops; the consumer drains everything published
since its last batch and commits when *that* guard drops. Neither path takes a
lock, and neither performs a read-modify-write — the whole handshake is two
release stores, one per end.

**What single-producer buys is a shorter dependency list.** `ring_gating`,
`ring_claim`, `ring_publish` and `ring_consume` are all absent, because each
answers a question that only has an answer worth computing when producers can
overtake one another. The [module documentation](src/lib.rs) works through the
four in turn.

The `unsafe` that lets two threads touch one allocation is sited here rather
than in `ring_store` or `ring_slot`; it is two private functions and one
`unsafe impl Sync`, justified in
[docs/workaround/readme.md](docs/workaround/readme.md).

Because that soundness argument is about *orderings*, the ordinary suite cannot
falsify it — measured, 100 000 items pass just as readily against a publish that
forgot its `Release`. That is not because the host hides it: this host is
aarch64 (Neoverse-N1), weakly ordered. The window is just too narrow for
sampling to open.
A `loom` model checks every interleaving of a two-record case instead:

```sh
RUSTFLAGS="--cfg loom" cargo test -p ring_spsc --test spsc_test
```

| File | Responsibility |
|------|-----------------|
| `docs/` | Scope, related crates, and closed trade-offs — see [docs/readme.md](docs/readme.md) |
| `src/lib.rs` | The `Ring`, its two ends, and the two publish/commit guards |
| `tests/spsc_test.rs` | This crate's reached-test, 26 tests around the surface, and a `loom` model of every interleaving |
| `tests/manual/readme.md` | The nine source readings and mutation checks automation cannot make |
