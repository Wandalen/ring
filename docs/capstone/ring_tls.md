# `ring_tls` — capstone notes

**Role:** Thread-local staging buffers ahead of a ring flush
**Tier:** 4/10
**Depends on:** `ring_types`, `ring_atomic`, `ring_batch`, `ring_store`, `ring_event`, `ring_slot`
**Depended on by:** `ring_flush`, `ring_bench`, `ring_testkit`
**Unsafe:** forbidden (default) — not on the allowlist
**no_std:** not yet audited — and likely can't be, see below
**loom:** not covered
**External API surface:** yes — one of the 5 crates meant for outside consumption

## Most relevant topics

- **Topic 6 (`no_std`):** named in the root `readme.md` itself as one of
  the likely `no_std`-infeasible crates — thread-locals need `std`. Same
  advice as `ring_wait`: document the negative verdict explicitly rather
  than skipping it.
- **Topic 5 (rustdoc):** external-facing — worth a full doc pass, including
  a worked example of the staging-then-flush lifecycle.
- **Topic 4 (semver):** external-facing with 3 dependents — include it in
  the `cargo-semver-checks` rollout alongside the other 4 public crates.
