# `ring_event` — capstone notes

**Role:** Slot translators that fill a claimed slot
**Tier:** 3/10
**Depends on:** `ring_types`, `ring_slot`, `ring_store`
**Depended on by:** `ring_tls`
**Unsafe:** forbidden (default) — not on the allowlist
**no_std:** not yet audited
**loom:** not covered
**External API surface:** no — internal to the family

## Most relevant topics

- **Topic 5 (rustdoc):** "translator" traits are exactly where a runnable
  doc example earns its keep — showing a minimal fill implementation makes
  the contract concrete in a way prose alone doesn't.
- **Topic 6 (`no_std`):** depends only on already-audited-or-in-progress
  Tier 0-2 crates — a reasonable next link in the chain.
