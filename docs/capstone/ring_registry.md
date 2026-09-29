# `ring_registry` — capstone notes

**Role:** Named ring registry
**Tier:** 8/10
**Depends on:** `ring_handle`, `ring_config`, `ring_core`
**Depended on by:** `ring_factory`
**Unsafe:** forbidden (default) — not on the allowlist
**no_std:** not yet audited — a global named registry likely needs allocation/collections, worth checking against `no_std` + `alloc`
**loom:** not covered
**External API surface:** no — internal to the family

## Most relevant topics

- **Topic 6 (`no_std`):** a named registry is a good test case for the
  `no_std` audit's harder edge: it may be achievable under `no_std` +
  `alloc` even if not under pure `no_std`, which is a distinction worth
  documenting explicitly rather than collapsing to a single yes/no.
- **Topic 9 (onboarding):** "named ring registry" is the kind of crate
  whose purpose is easy to state but easy to under-document — a good
  target for a concrete usage example.
