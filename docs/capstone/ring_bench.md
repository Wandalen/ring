# `ring_bench` — capstone notes

**Role:** `criterion`-based benchmark harness comparing ring backends
**Tier:** 10/10 — the top of the dependency graph
**Depends on:** `ring_factory`, `ring_tls`, `ring_flush`, `ring_stats`, `ring_spsc`, `ring_mpsc`, `ring_core`, `ring_slot`, `ring_types`
**Depended on by:** none — the top of the graph
**Unsafe:** forbidden (default) — not on the allowlist
**no_std:** not applicable — a benchmark harness, not an embeddable library
**loom:** not applicable
**External API surface:** no — a dev/benchmark tool, not a library consumers depend on

Don't confuse this with `bench_harness` (outside the tier graph entirely) —
that's the doc-freshness/coverage/unsafe-review *gate* engine; this is the
actual `criterion` performance comparison (in-house ring vs. mutex-guarded
queue vs. `crossbeam-queue` vs. thread-local staging) that produced the
original build-vs-buy decision
(`docs/decision/121_workstream_008_contract_gaps_ruled.md` in the origin
monorepo, per the FAQ linked from this directory's `readme.md`).

## Most relevant topics

- **Topic 7 (continuous benchmarking):** this crate *is* that topic's
  entire subject — its existing `comparison.rs` example and `criterion`
  setup are the starting point, not something to build from scratch. The
  deliverable is making its output durable and regression-gated, not
  writing new benchmarks from nothing.
- **Topic 4 (publish readiness):** sits at the top of the dependency graph
  — a good crate to run the `cargo publish --dry-run` sweep against last,
  since success here implies every crate beneath it in the graph resolved
  correctly too.
