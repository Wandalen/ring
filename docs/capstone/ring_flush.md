# `ring_flush` — capstone notes

**Role:** Flush policies deciding when thread-local staging reaches the ring
**Tier:** 7/10
**Depends on:** `ring_tls`, `ring_core`, `ring_types`, `ring_config`
**Depended on by:** `ring_bench`
**Unsafe:** forbidden (default) — not on the allowlist
**no_std:** not yet audited (depends on `ring_tls`, which is likely `no_std`-infeasible — this crate probably inherits that constraint)
**loom:** not covered
**External API surface:** yes — one of the 5 crates meant for outside consumption

## Most relevant topics

- **Topic 5 (rustdoc):** external-facing — the three flush triggers
  mentioned in this crate's own docs
  (`docs/non_functional_requirement/001_three_triggers_proven_by_a_flush_log.md`)
  are exactly the kind of policy that benefits from a worked example
  showing each trigger firing.
- **Topic 4 (semver):** external-facing — include in the
  `cargo-semver-checks` rollout.
- **Topic 6 (`no_std`):** likely inherits `ring_tls`'s `no_std`-infeasible
  verdict — confirm and document rather than assume.
