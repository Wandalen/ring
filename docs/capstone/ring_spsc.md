# `ring_spsc` — capstone notes

**Role:** Single-producer single-consumer ring API
**Tier:** 3/10
**Depends on:** `ring_store`, `ring_config`, `ring_cursor`, `ring_slot`, `ring_types`
**Depended on by:** `ring_bench`, `ring_core`
**Unsafe:** **allowlisted** — one producer and one consumer hold two ends of one slot array (`bench_harness/gate/declared/ring/unsafe_allowlist.txt`)
**no_std:** not yet audited
**loom:** covered
**External API surface:** no — internal to the family (reached through `ring_core`/`ring_handle`)

## Most relevant topics

- **Topic 2 (fuzzing):** a primary fuzz target — one of the 3 crates
  actually holding `unsafe` code. Focus fuzz inputs on claim/publish/commit
  sequences that fill, wrap, and drain the ring at its capacity boundary.
- **Topic 8 (extraction debt / loom):** already has `loom` coverage —
  extend it rather than starting from scratch, and use it as the reference
  pattern when adding `loom` to `ring_core` or `ring_mpsc`'s neighbors.
- **Topic 6 (`no_std`):** SPSC ring buffers are a classic embedded/`no_std`
  use case — worth prioritizing in the audit precisely because it's
  unsafe-allowlisted and latency-critical, the exact profile that benefits
  most from avoiding `std`'s overhead.
