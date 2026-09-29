# `ring_core` — capstone notes

**Role:** Composed ring over an SPSC or MPSC core with overflow, batch, and event support
**Tier:** 6/10
**Depends on:** `ring_config`, `ring_mpsc`, `ring_overflow`, `ring_slot`, `ring_spsc`, `ring_types`
**Depended on by:** `ring_shutdown`, `ring_factory`, `ring_debug`, `ring_handle`, `ring_poll`, `ring_registry`, `ring_flush`, `ring_bench`, `ring_testkit` (9 dependents)
**Unsafe:** **allowlisted** — "the shared assembly both [SPSC/MPSC] configurations are built from" (`bench_harness/gate/declared/ring/unsafe_allowlist.txt`)
**no_std:** not yet audited
**loom:** not covered — notably, the crate that assembles the two `loom`-covered backends isn't itself covered
**External API surface:** no — internal to the family, but sits directly underneath 2 of the 5 crates that are (`ring_handle`, `ring_flush`, `ring_factory` all depend on it)

This crate also carries the `crossbeam` feature flag — the interim
`crossbeam-queue::ArrayQueue` backend adopted so downstream consumers
weren't blocked on the in-house ring (see `docs/workaround/
001_crossbeam_queue_as_interim_backend.md` and the origin monorepo's
`faq.md`). Its test suite already validates the in-house ring against
crossbeam's actual runtime behavior — `crossbeam_honours_drop_oldest_by_evicting`
is the test to read before touching this crate's overflow handling.

## Most relevant topics

- **Topic 2 (fuzzing):** the primary fuzz target — it's the assembly point
  where SPSC, MPSC, overflow, batch, and event logic all meet, and it's
  unsafe-allowlisted. Most likely single crate to reward a fuzz harness.
- **Topic 8 (extraction debt / loom):** the gap noted above — `ring_spsc`
  and `ring_mpsc` are individually `loom`-covered, but the crate composing
  them isn't. Composition bugs (a race only visible when the two backends'
  invariants interact through this crate) wouldn't be caught by either
  backend's own coverage alone.
- **Topic 7 (benchmarking):** this is the crate `ring_bench` actually
  measures — any benchmark regression gate is really a gate on this crate's
  behavior.
