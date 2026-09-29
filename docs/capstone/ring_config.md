# `ring_config` — capstone notes

**Role:** Ring construction parameters
**Tier:** 1/10
**Depends on:** `ring_types`
**Depended on by:** `ring_mpsc`, `ring_factory`, `ring_registry`, `ring_handle`, `ring_core`, `ring_shutdown`, `ring_poll`, `ring_testkit`, `ring_debug`, `ring_flush`, `ring_spsc` (11 dependents — second only to `ring_types` for fan-in)
**Unsafe:** forbidden (default) — not on the allowlist
**no_std:** not yet audited
**loom:** not covered
**External API surface:** no — internal to the family (reached indirectly through the 5 external-facing crates)

## Most relevant topics

- **Topic 4 (publish readiness / semver):** 11 direct dependents makes this
  the third-highest-blast-radius crate after `ring_types`/`ring_cursor` —
  a signature change here ripples wide.
- **Topic 5 (rustdoc):** high fan-in, worth documenting well even though
  it's not itself externally consumed — everything downstream inherits
  whatever confusion lives here.
