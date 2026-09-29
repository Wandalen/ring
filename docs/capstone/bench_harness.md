# `bench_harness` — capstone notes

**Role:** Stage gates and workload oracle for several crate families, this one included — depends on no `ring_*` crate
**Tier:** outside the graph — family-neutral, graded by nothing, grades everyone
**Depends on:** no `ring_*` crate
**Depended on by:** no `ring_*` crate (not a library dependency of anything — it's tooling, invoked, not linked)
**Unsafe:** not in scope of the `ring_*` unsafe allowlist (separate concern)
**no_std:** not applicable — this is dev tooling, not an embeddable library
**loom:** not applicable
**External API surface:** none as a library — its surface is its gate scripts (`gate/g1_coverage.sh` ... `gate/g22_exemption_expiry.sh`) and CLI, not a Rust API

Don't confuse this with `ring_bench` (Tier 10) — that's a `criterion`
performance-benchmark crate comparing ring backends; this is the
verification/quality-gate engine every crate in the family (and other
families in the origin monorepo) is graded by.

## Most relevant topics

- **Topic 1 (CI):** this crate *is* what CI should be running. If you're
  taking this topic, `bench_harness/gate/` is your entire deliverable's
  payload — CI just needs to invoke it on every push/PR.
- **Topic 9 (onboarding & doc-corpus):** the G15 freshness gate
  (`gate/corpus/recipes.py`) that grades every other crate's docs lives
  here. `docs/guide/002_the_four_verdicts.md` and
  `docs/guide/003_what_the_gates_do_not_prove.md` are required reading
  before extending it.
- **Topic 8 (extraction debt):** the repo-wide `verb/test` gap (this repo
  was quietly depending on the parent monorepo's copy) is most naturally
  fixed as a script living alongside this harness.
