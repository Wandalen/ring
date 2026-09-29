# `ring_gating` — capstone notes

**Role:** Producer gating so it never laps the slowest consumer
**Tier:** 3/10
**Depends on:** `ring_types`, `ring_cursor`, `ring_seqno`
**Depended on by:** `ring_publish`, `ring_barrier`, `ring_mpsc`, `ring_claim`
**Unsafe:** forbidden (default) — not on the allowlist
**no_std:** not yet audited
**loom:** not covered
**External API surface:** no — internal to the family

## Most relevant topics

- **Topic 2 (fuzzing) / Topic 8 (loom):** "never laps the slowest
  consumer" is a correctness invariant, not just a performance one — the
  kind of property that benefits from both a fuzz target (does gating ever
  let a producer overtake?) and eventual `loom` coverage once the
  allowlisted trio's coverage is extended.
- **Topic 5 (rustdoc):** the gating invariant deserves to be stated
  explicitly in the public doc comment, not just implied by the
  implementation.
