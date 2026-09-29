# `ring_align` — capstone notes

**Role:** Cache-line padding constants and alignment wrappers
**Tier:** 0/10 — foundational, along with `ring_types`
**Depends on:** none — foundational, and the only crate besides `ring_types` with zero `ring_*` dependencies
**Depended on by:** `ring_cursor` only
**Unsafe:** forbidden (default) — not on the allowlist
**no_std:** not yet audited
**loom:** not covered
**External API surface:** no — internal to the family

## Most relevant topics

- **Topic 6 (`no_std`):** tiny, zero-dependency, and almost certainly
  `no_std`-safe by construction (padding constants and alignment wrappers
  don't typically need the standard library) — a good first crate to prove
  the `no_std` audit process out on before tackling something bigger.
- **Topic 5 (rustdoc):** low-risk, single-dependent crate — a safe place to
  practice a full doc-comment pass before moving to higher-fan-in crates.
