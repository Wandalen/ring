# ring_core

Composed ring over an SPSC, MPSC, or crossbeam backend behind one surface.

Depends on [`ring_config`](../ring_config/readme.md), [`ring_mpsc`](../ring_mpsc/readme.md), [`ring_overflow`](../ring_overflow/readme.md), [`ring_slot`](../ring_slot/readme.md), [`ring_spsc`](../ring_spsc/readme.md), [`ring_types`](../ring_types/readme.md), and optionally `crossbeam-queue`.

One of the 33 `ring_*` crates that make up this family's concurrency
write-path — a 33-crate dependency forest rooted at `ring_types`, acyclic
by construction. Build order follows [`../Cargo.toml`](../Cargo.toml)'s
member list; the family as a whole is described in
[`../README.md`](../README.md).

This is the **composition point**. `Ring::new` reads the configuration's
producer count and builds an [`ring_spsc`](../ring_spsc/readme.md) or
[`ring_mpsc`](../ring_mpsc/readme.md) ring behind the same handles;
`Ring::new_crossbeam`, behind the `crossbeam` cargo feature, builds a third on
`crossbeam_queue::ArrayQueue`. That third backend exists to decouple
schedules — a consumer needing a working multi-producer channel takes the
proven queue now and swaps later by changing a build flag.

**The surface is value-shaped, and that was forced rather than chosen.** Both
in-house backends publish through a *slot*: claim a reservation, write in
place, let the guard's `Drop` publish. `ArrayQueue` has no such thing — it
offers `push( value )` and `pop() -> Option< value >` and nothing else. So a
slot-shaped uniform surface cannot exist across all three, and the in-place
reservation API stops here. A caller who needs to build a record in the ring's
own memory reaches for `ring_spsc` or `ring_mpsc` directly and gives up the
swap. That is the trade, and the [module documentation](src/lib.rs) states it
where a caller will meet it.

**Three things the uniform surface cannot make uniform**, tabulated in full in
the module documentation: producer cardinality, drain order, and whether
`free_capacity` is binding or advisory. The last is the sharp one — one
signature over two contracts, with no compiler error between them. `try_clone`
returning `None` is the only machine-checkable way to tell the backends apart.

This crate adds **no atomic and no `unsafe` of its own**. `ring_spsc` asserts
zero read-modify-writes across a run and every publish reaches it through a
method here, so a counter added at this layer would break that assertion with
nothing in `ring_spsc`'s own dependency tree to blame. Instrumentation belongs
in `ring_stats`, which is deliberately not a dependency.

It is also the family's **only crate with a cargo feature**, which makes it two
programs rather than one. Both must be built and measured; running one and
reporting it as the crate is how a coverage figure comes out 22 points low
while nothing is untested. `tests/manual/readme.md` C3 and C4 record what that
cost the gates, which now check both.

```sh
cargo nextest run -p ring_core                      # 21 tests, two backends
cargo nextest run -p ring_core --features crossbeam # 23 tests, three
```

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build — see [verb/readme.md](verb/readme.md) |
| `docs/` | 19 doc instances across 12 definitions — see [docs/readme.md](docs/readme.md) |
| `src/lib.rs` | The `Ring`, its three backends, and the ends/producer/consumer handles |
| `tests/core_test.rs` | The crossbeam backend's reached-test and 21 tests across every backend the build offers |
| `tests/manual/readme.md` | The nine source readings and mutation checks automation cannot make |
