# `ring_types` — capstone notes

**Role:** Shared ids, errors, and policy enums for the ring family — no ring logic
**Tier:** 0/10 — foundational, along with `ring_align`
**Depends on:** none — foundational
**Depended on by:** nearly every other crate in the family (31 of 32) — the single most depended-on crate here
**Unsafe:** forbidden (default) — not on the allowlist
**no_std:** already declared
**loom:** not covered (nothing to interleave — it's types, not concurrency logic)
**External API surface:** yes — one of the 5 crates meant for outside consumption

## Most relevant topics

- **Topic 4 (publish readiness / semver):** this is the highest-blast-radius
  crate in the family by a wide margin — 31 direct dependents. A breaking
  change here breaks nearly everything downstream in the same breath.
  `cargo-semver-checks` earns its keep most here.
- **Topic 5 (rustdoc):** external-facing and foundational — worth being the
  first crate whose public docs get a full pass.
- **Topic 6 (`no_std`):** already declared here; use this crate as the
  reference example when auditing the rest of Tier 0/1 for `no_std`
  feasibility.
