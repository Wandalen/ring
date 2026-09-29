# `ring_wait` — capstone notes

**Role:** Wait strategies for space and data availability
**Tier:** 3/10
**Depends on:** `ring_types`, `ring_cursor`
**Depended on by:** `ring_barrier`, `ring_shutdown`
**Unsafe:** forbidden (default) — not on the allowlist
**no_std:** not yet audited — and likely can't be, see below
**loom:** not covered
**External API surface:** no — internal to the family

## Most relevant topics

- **Topic 6 (`no_std`):** named in the root `readme.md` itself as one of
  the likely `no_std`-infeasible crates — parking/wait strategies typically
  need `std::thread`. Worth confirming and documenting *why* rather than
  just marking it "not applicable" — that documented-negative is as
  valuable to this topic's deliverable as a positive audit.
