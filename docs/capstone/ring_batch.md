# `ring_batch` — capstone notes

**Role:** Batch claim objects spanning a sequence range
**Tier:** 2/10
**Depends on:** `ring_types`, `ring_seqno`, `ring_atomic`, `ring_index`
**Depended on by:** `ring_tls`
**Unsafe:** forbidden (default) — not on the allowlist
**no_std:** not yet audited
**loom:** not covered
**External API surface:** no — internal to the family

## Most relevant topics

- **Topic 2 (fuzzing):** a batch spans a sequence range — the boundary
  cases (empty range, range spanning a wraparound) are worth a property
  test or fuzz target even outside the main unsafe-allowlisted trio.
- **Topic 6 (`no_std`):** depends only on other Tier-0/1 crates already
  under `no_std` review — a natural next link in that audit chain.
