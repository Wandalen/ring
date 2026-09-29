# `ring_claim` — capstone notes

**Role:** Sequence-range claiming without waiting
**Tier:** 4/10
**Depends on:** `ring_types`, `ring_cursor`, `ring_gating`
**Depended on by:** `ring_publish`, `ring_mpsc`
**Unsafe:** forbidden (default) — not on the allowlist
**no_std:** not yet audited
**loom:** not covered
**External API surface:** no — internal to the family

## Most relevant topics

- **Topic 2 (fuzzing):** "claiming without waiting" implies a failure mode
  (claim rejected) that's easy to under-test manually and cheap to fuzz —
  concurrent claim storms are a natural target once the main fuzz harness
  exists.
- **Topic 8 (loom):** feeds directly into `ring_mpsc` (already `loom`
  -covered) — a candidate for extending that coverage one hop upstream.
