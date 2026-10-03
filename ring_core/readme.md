# ring_core

Composed ring over an SPSC, MPSC, or crossbeam backend behind one API.

Part of the `ring` family; [../readme.md](../readme.md) describes the whole.

This is the **composition point**. `Ring::new` reads the configuration's
producer count and builds an [`ring_spsc`](../ring_spsc/readme.md) or
[`ring_mpsc`](../ring_mpsc/readme.md) ring behind the same handles.
`Ring::new_crossbeam`, behind the `crossbeam` cargo feature, builds a third on
`crossbeam_queue::ArrayQueue`. That third backend exists to decouple
schedules. A consumer needing a working multi-producer channel takes the
proven queue now and swaps later by changing a build flag.

**The API is value-shaped, and that was forced rather than chosen.** Both
in-house backends publish through a *slot*: claim a reservation, write in
place, let the guard's `Drop` publish. `ArrayQueue` has no such thing. It
offers `push( value )` and `pop() -> Option< value >` and nothing else. So a
slot-shaped uniform API cannot exist across all three, and the in-place
reservation API stops here. A caller who needs to build a record in the ring's
own memory reaches for `ring_spsc` or `ring_mpsc` directly and gives up the
swap. That is the trade, and the [module documentation](src/lib.rs) states it
where a caller will meet it.

**Three things the uniform API cannot make uniform**, tabulated in full in
the module documentation: producer cardinality, drain order, and whether
`free_capacity` is binding or advisory. The last is the sharp one. It is one
signature over two contracts, with no compiler error between them. `try_clone`
returning `None` is the only machine-checkable way to tell the backends apart.

This crate adds **no atomic and no `unsafe` of its own**. `ring_spsc` asserts
zero read-modify-writes across a run and every publish reaches it through a
method here, so a counter added at this layer would break that assertion with
nothing in `ring_spsc`'s own dependency tree to blame. Instrumentation belongs
in `ring_stats`, which is deliberately not a dependency.

The `crossbeam` feature makes it two programs rather than one, and both must
be built and measured. Running one and reporting it as the crate is how a
coverage figure comes out low while nothing is untested. The
[manual plan](tests/manual/readme.md)'s C3 (both feature configurations are
built by something) and C4 (the coverage figure depends on the feature set)
check that the gates cover both.

## Decisions

- [`free_capacity` keeps one `usize` signature on every backend, binding at SPSC and advisory elsewhere](docs/decisions/001_free_capacity_keeps_one_signature_across_backends.md)
- [`crossbeam-queue`'s `ArrayQueue` is an interim third backend inside `ring_core`, behind the `crossbeam` feature](docs/decisions/002_crossbeam_queue_is_an_interim_backend_inside_ring_core.md)
- [`try_push_batch` hands back the record it refused, as `Err((n, record))`](docs/decisions/003_the_batch_push_hands_back_the_refused_record.md)
- [`Fail` is the default overflow policy, so `Ok` from a ring nobody configured means the record was kept](docs/decisions/004_fail_is_the_default_overflow_policy.md)

## Known limitations

- `ring_core::Ring` exposes no cursor, and `ring_core::Producer::try_push_batch`
  pushes one record at a time. So `ring_tls::TlsBuffer::flush_into`, which
  claims a whole batch with one `fetch_add`, has no caller outside `ring_tls`'s
  own tests, and `ring_testkit::Script::run` flushes staged records one push at
  a time. A consumer that stages into a `TlsBuffer` and publishes into a
  `ring_core::Ring` on a hot path, with the per-batch saving measured, would
  justify exposing a cursor.

## Run it

```sh
cargo nextest run -p ring_core                  # default build: two backends
cargo nextest run -p ring_core --all-features   # adds the crossbeam backend
cargo test --doc -p ring_core --all-features
```

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build. See the workspace [verb/readme.md](../verb/readme.md) |
| `docs/decisions/` | Architecture decision records |
| `src/lib.rs` | The `Ring`, its three backends, and the ends/producer/consumer handles |
| `tests/core_test.rs` | One program run against every backend the build offers, and the tests around it |
| `tests/manual/readme.md` | The source readings and mutation checks automation cannot make |
