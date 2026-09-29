# `ring_debug` — capstone notes

**Role:** Runtime invariant checks over a live ring
**Tier:** 7/10
**Depends on:** `ring_core`, `ring_cursor`, `ring_types`, `ring_atomic`, `ring_config`
**Depended on by:** none — a leaf
**Unsafe:** forbidden (default) — not on the allowlist
**no_std:** not yet audited — likely needs `std` if it does any formatting/reporting, worth confirming either way
**loom:** not covered
**External API surface:** no — internal to the family

## Most relevant topics

- **Topic 9 (onboarding):** "runtime invariant checks over a live ring" is
  exactly the kind of crate whose own doc comments should double as a
  mini-tutorial on what the family's invariants actually are — a good fit
  for someone leaning toward the onboarding topic rather than a purely
  technical one.
- **Topic 5 (rustdoc):** leaf crate, low risk — document each checked
  invariant with a one-line rationale for why it matters.
