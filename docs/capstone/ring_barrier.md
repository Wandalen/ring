# `ring_barrier` — capstone notes

**Role:** Consumer barrier over the minimum of dependent cursors
**Tier:** 4/10
**Depends on:** `ring_types`, `ring_cursor`, `ring_wait`, `ring_gating`
**Depended on by:** `ring_publish`, `ring_consume`
**Unsafe:** forbidden (default) — not on the allowlist
**no_std:** not yet audited
**loom:** not covered
**External API surface:** no — internal to the family

## Most relevant topics

- **Topic 2 (fuzzing) / Topic 8 (loom):** "minimum of dependent cursors" is
  a multi-input correctness computation — worth a property test with
  randomized cursor sets even before considering full `loom` coverage.
- **Topic 5 (rustdoc):** the "barrier over a minimum" concept is exactly
  the kind of thing worth a small diagram or worked example in the doc
  comment.
