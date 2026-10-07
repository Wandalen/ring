# Topic 7: Continuous benchmark reporting

## The gap

`ring_bench` (a `criterion`-based harness, Tier 10 — the top of the
dependency graph) already exists —
`../../ring_bench/examples/comparison.rs` benchmarks the in-house ring
against the alternatives this project considered: a mutex-guarded queue,
`crossbeam-queue` (the interim backend `ring_core` still carries behind a
feature flag), and thread-local staging over the ring. But its results
live in whoever's terminal last ran it. There's no regression detection
and no report anyone else can see.

## Deliverables

- A `criterion` HTML report published somewhere durable — GitHub Pages is
  the easy option
- A CI job ([Topic 1](01_ci_cd.md)) that runs the comparison on every merge
  to `master`: in-house ring vs. the `crossbeam-queue` baseline
  `ring_core` already carries as a feature-gated interim backend
- A regression gate — fail the build if p99 latency regresses past a
  documented threshold, not just "fail if it crashes"

## Where this stands

The CI job exists: `.github/workflows/benchmarks.yml` runs
`verb/bench suite::all` on three hosted architectures — x86_64 linux,
aarch64 linux, apple silicon — nightly, on `master` pushes that touch
the benchmarked code, and on manual dispatch. Each run publishes its
markdown report into the run summary and a per-architecture artifact
carrying the raw `target/perf/` results beside it.

The other two deliverables are deferred, on the record:

- The durable `criterion` HTML report. A shared hosted runner's
  `target/criterion` is hundreds of megabytes of HTML nobody reads
  there; until a publisher (GitHub Pages, or a longer artifact
  retention) is chosen deliberately, the markdown report and the raw
  JSON in the artifact are the durable form.
- The regression gate. The benchmarking guide's § Fair measurements
  says unpinned threads on shared hardware swing tens of percent in
  both directions; a p99 threshold fed by such numbers fails on noise,
  not regressions. It waits for numbers from one dedicated machine with
  an established noise floor.

## Why this one matters more than it looks

This project's whole design philosophy is "measured, not argued" — adoption
gated on benchmarking, never a crates.io popularity contest. Right now that
principle only ever got applied *once*, at design time. This topic is what
turns it into an ongoing property of the codebase instead of a decision
frozen in a document from whenever `ring_bench` was last run by hand.
