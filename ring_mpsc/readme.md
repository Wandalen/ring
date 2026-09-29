# ring_mpsc

Sequence-numbered multi-producer ring buffer: many threads claim-and-publish
concurrently, one consumer thread drains in total order.

Depends on [`ring_atomic`](../ring_atomic/readme.md),
[`ring_store`](../ring_store/readme.md), [`ring_claim`](../ring_claim/readme.md),
[`ring_config`](../ring_config/readme.md), [`ring_cursor`](../ring_cursor/readme.md),
[`ring_gating`](../ring_gating/readme.md), [`ring_slot`](../ring_slot/readme.md),
and [`ring_types`](../ring_types/readme.md).
One of the 33 `ring_*` crates that make up this family's concurrency
write-path — a 33-crate dependency forest rooted at `ring_types`, acyclic
by construction. Build order follows [`../Cargo.toml`](../Cargo.toml)'s
member list; the family as a whole is described in
[`../readme.md`](../readme.md). This crate was `mpsc_ring` before the family
adopted a single prefix.

**The dependency list changed on implementation, in both directions, and the
substitutions are the interesting part.** Originally scoped for seven
dependencies; the crate ended up with eight. `ring_slot` and `ring_types` and
`ring_atomic` were added — the first two because slot storage and the
`Seq`/`Capacity`/`RingError` vocabulary are genuinely shared, the third
because loom instrumentation has to be swapped in at the atomic type rather
than at this crate. `ring_publish` and `ring_consume` were *removed*:
publication here is a per-slot stamp write and a scan, not a cursor advance,
so neither crate's shape fits. Both now have no consumer in the family, which
is a consequence of that shape rather than an oversight.

It carries no *consumer*-side family prefix for the original reason — no
natural single owner. More than one consumer needs the same staging mechanism
independently, and prefixing it into any one consumer's namespace would
misrepresent genuinely co-equal shared infrastructure as one stack's internals
loaned to another.

The ring pattern is now implemented and its correctness is established; a
consumer's migration onto it is a decision that belongs to that consumer, not
to this crate (→ [`docs/non_functional_requirement/001`](docs/non_functional_requirement/001_measured_before_adopted.md)).
This crate exists so that decision gets built on exactly one implementation
rather than reinvented per consumer. Implemented is deliberately not adoption:
no consumer manifest names it.

Implemented. 33 integration tests plus 2 loom models, 100% line coverage,
clippy clean under `-D warnings`.

```sh
cd "$(git rev-parse --show-toplevel)"
cargo nextest run -p ring_mpsc                              # 33 passed
RUSTFLAGS="--cfg loom" cargo nextest run -p ring_mpsc exhaustive::   # 2 passed
```

| File | Responsibility |
|------|-----------------|
| `docs/` | Scope, related crates, and open trade-offs — see [docs/readme.md](docs/readme.md) |
| `src/lib.rs` | The ring, its two handles, and the two RAII guards that publish and commit |
| `tests/` | Integration tests and the loom models — see [tests/manual/readme.md](tests/manual/readme.md) for what was checked by hand |
