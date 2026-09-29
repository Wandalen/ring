# `ring_index` — capstone notes

**Role:** Sequence-to-slot index mapping for power-of-two capacities
**Tier:** 1/10
**Depends on:** `ring_types`
**Depended on by:** `ring_batch`, `ring_store`
**Unsafe:** forbidden (default) — not on the allowlist
**no_std:** not yet audited
**loom:** not covered
**External API surface:** no — internal to the family

## Most relevant topics

- **Topic 6 (`no_std`):** pure arithmetic over a power-of-two capacity —
  no obvious `std` dependency, a reasonable early `no_std` candidate.
- **Topic 5 (rustdoc):** the power-of-two capacity constraint is exactly
  the kind of invariant that's easy to state in a doc comment and easy to
  forget without one — a good target for a runnable doc example.
