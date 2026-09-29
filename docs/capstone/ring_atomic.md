# `ring_atomic` — capstone notes

**Role:** Atomic sequence helpers with explicit memory orderings
**Tier:** 1/10
**Depends on:** `ring_types`
**Depended on by:** `ring_batch`, `ring_mpsc`, `ring_tls`, `ring_debug`, `ring_cursor`
**Unsafe:** forbidden (default) — not on the allowlist, despite sitting right next to the concurrency-critical tier
**no_std:** not yet audited
**loom:** covered
**External API surface:** no — internal to the family

## Most relevant topics

- **Topic 8 (extraction debt / loom):** already has `loom` coverage — a
  good reference example for what "covered" looks like when extending
  coverage to the unsafe-allowlisted crates (`ring_spsc`, `ring_mpsc`,
  `ring_core`).
- **Topic 6 (`no_std`):** explicit memory orderings and atomics are exactly
  the kind of thing `core::sync::atomic` supports without `std` — a strong
  `no_std` candidate.
