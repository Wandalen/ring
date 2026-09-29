# `ring_overflow` — capstone notes

**Role:** Full-ring overflow policies
**Tier:** 2/10
**Depends on:** `ring_types`, `ring_stats`
**Depended on by:** `ring_core`
**Unsafe:** forbidden (default) — not on the allowlist
**no_std:** already declared
**loom:** not covered
**External API surface:** no — internal to the family

## Most relevant topics

- **Topic 2 (fuzzing):** "what happens when the ring is full" is precisely
  the kind of policy decision a fuzz harness should exercise once it
  reaches `ring_core` — this crate defines the exact behavior being
  fuzzed at that boundary.
- **Topic 6 (`no_std`):** already declared — third member of the
  already-`no_std` reference set alongside `ring_types`/`ring_stats`.
