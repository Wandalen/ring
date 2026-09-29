# `ring_seqno` — capstone notes

**Role:** Sequence numbers and their wrapping arithmetic
**Tier:** 1/10
**Depends on:** `ring_types`
**Depended on by:** `ring_batch`, `ring_gating`, `ring_consume`, `ring_cursor`
**Unsafe:** forbidden (default) — not on the allowlist
**no_std:** not yet audited
**loom:** not covered
**External API surface:** no — internal to the family

## Most relevant topics

- **Topic 2 (fuzzing):** wrapping arithmetic is a classic fuzz target even
  though this crate isn't unsafe-allowlisted — wraparound edge cases here
  feed directly into the unsafe-allowlisted crates that depend on it
  transitively. Worth a property-test or fuzz pass even if the main fuzz
  harness targets `ring_spsc`/`ring_mpsc`/`ring_core` directly.
- **Topic 6 (`no_std`):** wrapping integer arithmetic doesn't need `std` —
  a strong candidate.
