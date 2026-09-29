# `ring_shutdown` — capstone notes

**Role:** Publisher stop, drain, and waiter join
**Tier:** 7/10
**Depends on:** `ring_cursor`, `ring_wait`, `ring_core`, `ring_types`, `ring_config`
**Depended on by:** `ring_testkit`
**Unsafe:** forbidden (default) — not on the allowlist
**no_std:** not yet audited — depends on `ring_wait`, likely `no_std`-infeasible, probably inherits that
**loom:** not covered
**External API surface:** no — internal to the family

## Most relevant topics

- **Topic 2 (fuzzing) / Topic 8 (loom):** shutdown sequencing (stop, drain,
  join) is a classic source of lost-wakeup and use-after-shutdown bugs in
  concurrent systems — a strong candidate for property-testing or `loom`
  coverage even though it isn't unsafe-allowlisted itself.
- **Topic 6 (`no_std`):** likely inherits `ring_wait`'s infeasibility —
  confirm and document.
