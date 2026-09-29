# `ring_cursor` — capstone notes

**Role:** Producer and consumer sequence cursors, cache-line separated
**Tier:** 2/10
**Depends on:** `ring_types`, `ring_seqno`, `ring_atomic`, `ring_align`
**Depended on by:** `ring_barrier`, `ring_gating`, `ring_claim`, `ring_consume`, `ring_mpsc`, `ring_debug`, `ring_publish`, `ring_wait`, `ring_shutdown`, `ring_spsc` (10 dependents — the single widest fan-in after `ring_types`/`ring_config`)
**Unsafe:** forbidden (default) — not on the allowlist, despite being the cursor pairing the allowlist's own justification file names as central to soundness
**no_std:** not yet audited
**loom:** not covered
**External API surface:** no — internal to the family

## Most relevant topics

- **Topic 4 (publish readiness / semver):** 10 direct dependents including
  both unsafe-allowlisted crates — treat any public signature change here
  as needing the semver-check gate before anything else.
- **Topic 8 (extraction debt / loom):** the allowlist's own justification
  file (`bench_harness/gate/declared/ring/unsafe_allowlist.txt`) explains
  the shared-slot invariant entirely in terms of cursors — this crate isn't
  itself unsafe, but it's the one whose correctness the unsafe code in
  `ring_spsc`/`ring_mpsc`/`ring_core` actually depends on. A strong
  candidate for `loom` coverage even though it holds no unsafe of its own.
