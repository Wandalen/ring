# `ring_store` — capstone notes

**Role:** Power-of-two slot storage array
**Tier:** 2/10
**Depends on:** `ring_types`, `ring_slot`, `ring_index`
**Depended on by:** `ring_mpsc`, `ring_tls`, `ring_event`, `ring_spsc` (4 dependents, two of which are the unsafe-allowlisted `ring_spsc`/`ring_mpsc`)
**Unsafe:** forbidden (default) — not on the allowlist, despite backing the storage the allowlisted crates build on
**no_std:** not yet audited
**loom:** not covered
**External API surface:** no — internal to the family

## Most relevant topics

- **Topic 2 (fuzzing):** the actual storage array backing the two
  unsafe-allowlisted SPSC/MPSC crates — capacity edge cases here matter to
  any fuzz harness targeting `ring_spsc`/`ring_mpsc` even if this crate
  itself carries no `unsafe`.
- **Topic 6 (`no_std`):** a plain fixed-capacity array — a strong `no_std`
  candidate, and one worth prioritizing since two unsafe-allowlisted
  crates depend on it.
