# ring_stats

Ring counters.

Part of the `ring` family; [../readme.md](../readme.md) describes the whole.

`RingStats` counts claims, publishes, consumes, wait time, and drops per
overflow policy, so a ring under mild pressure can be told apart from one
quietly discarding traffic. The counters are relaxed atomics, since a stats read
is a diagnostic and never a synchronisation point, so `RingStats` is shared by
reference across threads without a lock. No live ring's write path moves these
counters yet. `ring_core` calls the counter-free `ring_overflow::would_resolve`,
so outside `ring_bench` every reader returns a structural zero.

## Run it

```sh
cargo nextest run -p ring_stats --all-features
cargo test --doc -p ring_stats --all-features
```

| File | Responsibility |
|------|-----------------|
| `src/lib.rs` | Crate root, holding the crate's whole public API |
| `tests/` | `stats_test.rs` and the manual plan under `manual/` |
