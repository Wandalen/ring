# `ring_mpsc` — capstone notes

**Role:** Multi-producer single-consumer ring API
**Tier:** 5/10
**Depends on:** `ring_atomic`, `ring_store`, `ring_claim`, `ring_config`, `ring_cursor`, `ring_gating`, `ring_slot`, `ring_types`
**Depended on by:** `ring_bench`, `ring_core`
**Unsafe:** **allowlisted** — many producers, per-slot sequence stamps, same shared-slot pattern as `ring_spsc` (`bench_harness/gate/declared/ring/unsafe_allowlist.txt`)
**no_std:** not yet audited
**loom:** covered
**External API surface:** no — internal to the family (reached through `ring_core`/`ring_handle`)

## Most relevant topics

- **Topic 2 (fuzzing):** a primary fuzz target — the multi-producer case is
  strictly harder than `ring_spsc`'s single-producer one, so this is where
  a fuzz harness is most likely to find a real race.
- **Topic 8 (extraction debt / loom):** already `loom`-covered, but
  multi-producer interleavings are combinatorially larger than
  single-producer ones — worth checking whether the existing coverage
  actually exercises enough concurrent-producer-count variety, not just
  extending coverage elsewhere.
- **Topic 6 (`no_std`):** highest-value crate in the unsafe-allowlisted
  trio to get `no_std`-correct, given it's also the most concurrency-heavy.
