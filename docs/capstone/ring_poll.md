# `ring_poll` — capstone notes

**Role:** Non-blocking progress helpers
**Tier:** 7/10
**Depends on:** `ring_core`, `ring_config`, `ring_types`
**Depended on by:** none — a leaf
**Unsafe:** forbidden (default) — not on the allowlist
**no_std:** not yet audited
**loom:** not covered
**External API surface:** no — internal to the family

## Most relevant topics

- **Topic 6 (`no_std`):** "non-blocking" helpers are often `no_std`-friendly
  by nature (no need for OS-level blocking primitives) — worth checking
  even though it depends on `ring_core`, which isn't yet audited itself.
- **Topic 5 (rustdoc):** leaf, low risk — a fine crate to try a first
  full doc pass on.
