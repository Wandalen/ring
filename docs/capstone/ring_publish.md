# `ring_publish` — capstone notes

**Role:** Publication of claimed slots to consumers
**Tier:** 6/10
**Depends on:** `ring_types`, `ring_cursor`, `ring_claim`, `ring_consume`, `ring_barrier`, `ring_gating`
**Depended on by:** none — a leaf despite sitting mid-graph, and the one crate with the widest direct dependency fan-in (6) of any non-Tier-0/1 crate
**Unsafe:** forbidden (default) — not on the allowlist
**no_std:** not yet audited
**loom:** covered
**External API surface:** no — internal to the family

## Most relevant topics

- **Topic 8 (extraction debt / loom):** already `loom`-covered — this
  crate ties together claim, consume, barrier, and gating, so its coverage
  is a good sanity check that those four crates' individual invariants
  actually compose correctly. Worth reviewing before assuming "5 crates
  covered" means the interesting interactions are all exercised.
- **Topic 5 (rustdoc):** zero dependents means low risk for documenting
  experimentally — a good crate to try a more ambitious worked example on.
