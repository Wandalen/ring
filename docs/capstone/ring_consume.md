# `ring_consume` — capstone notes

**Role:** Single-consumer available-range computation and commit
**Tier:** 5/10
**Depends on:** `ring_types`, `ring_cursor`, `ring_barrier`, `ring_seqno`
**Depended on by:** `ring_publish`
**Unsafe:** forbidden (default) — not on the allowlist
**no_std:** not yet audited
**loom:** not covered
**External API surface:** no — internal to the family

## Most relevant topics

- **Topic 2 (fuzzing) / Topic 8 (loom):** "available-range computation and
  commit" is the read-side half of the claim/publish/commit protocol —
  worth fuzzing alongside the write-side crates even though it isn't
  itself unsafe-allowlisted, since a commit bug here is as dangerous as one
  in the allowlisted trio.
