# `ring_stats` — capstone notes

**Role:** Ring counters
**Tier:** 1/10
**Depends on:** `ring_types`
**Depended on by:** `ring_bench`, `ring_overflow`
**Unsafe:** forbidden (default) — not on the allowlist
**no_std:** already declared
**loom:** not covered
**External API surface:** no — internal to the family

## Most relevant topics

- **Topic 6 (`no_std`):** already declared — use alongside `ring_types` and
  `ring_overflow` as the reference set for what a correctly-audited
  `no_std` crate looks like in this family.
- **Topic 5 (rustdoc):** small, low-risk (only 2 dependents) — a fine
  practice crate for a first doc-comment pass.
