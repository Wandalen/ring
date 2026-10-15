# Capstone: Make `ring` Production-Ready

## The assignment

Five developers, one shared target: `ring`, a from-scratch 34-crate lock-free
concurrency write path (claim/publish/commit protocol, `ring_claim` /
`ring_publish` / `ring_barrier` / `ring_gating`), built as a measured
alternative to a mutex-guarded queue and `crossbeam-queue::ArrayQueue` for
many-producer/single-consumer, hard-latency-budget workloads.

- **Repo:** [`Wandalen/ring`](https://github.com/Wandalen/ring), branch `master`
- **Start here:** [`../../readme.md`](../../readme.md) — architecture, the
  11-tier dependency graph, and the "Why in-house, not off-the-shelf"
  section covering what was measured and why

This is a real codebase, not a toy: unsafe is `deny`-by-default with a
2-crate allowlist (`ring_spsc`, `ring_mpsc` —
[`unsafe_allowlist.txt`](../../bench_harness/gate/declared/ring/unsafe_allowlist.txt)),
it already runs `loom` model-checking on part of the concurrency-critical
tier, and it has its own home-grown stage-gate verification suite
(`bench_harness/gate/`, gates named `g1`...`g22`) enforcing coverage, rustdoc
coverage, and unsafe-code review. Whatever a topic below adds, wire it in
the same spirit: a gate that can fail, not a claim that sounds true.

**Two things to know before anyone opens an editor:**

1. `git log` here has exactly **one commit** (`f0eb4c1`, "initial commit").
   The crate family was extracted out of a larger private monorepo as a
   fresh squashed snapshot — there's no history to `git blame` past that
   commit.
2. As of 2026-09-29 (the day of extraction), the working tree still had a
   large post-extraction fixup sitting uncommitted. That's very likely
   resolved by the time you read this — just run `git status`/`git log`
   yourself rather than assuming either state.

## Topics — pick 5 (or all 7, if the team wants stretch goals)

Each is scoped to be ownable by one person, produces something
demonstrable, and closes a gap that's real right now, not hypothetical —
grounded in what's actually in (or missing from) the repo today.

| # | Topic | File | The gap, in one line |
|---|-------|------|-----------------------|
| 1 | CI/CD — make the gates actually gate | [01_ci_cd.md](01_ci_cd.md) | No `.github/workflows/` at all — nothing runs automatically |
| 2 | Fuzzing the lock-free core | [02_fuzzing.md](02_fuzzing.md) | No `cargo-fuzz`; `loom` covers interleavings, not the input space |
| 3 | Supply-chain & dependency security | [03_supply_chain_security.md](03_supply_chain_security.md) | No `deny.toml`/`audit.toml` |
| 4 | crates.io publish readiness & release process | [04_publish_readiness.md](04_publish_readiness.md) | `publish = true` everywhere, but never dry-run, no MSRV/semver policy |
| 5 | Public API docs — docs.rs-grade rustdoc | [05_rustdoc.md](05_rustdoc.md) | Rich internal `docs/` corpus, unaudited public rustdoc |
| 6 | `no_std` / embedded-readiness audit | [06_no_std.md](06_no_std.md) | Only 3 of 34 crates declare `no_std` |
| 7 | Continuous benchmark reporting | [07_benchmarking.md](07_benchmarking.md) | `ring_bench` exists, results live in a terminal, no regression gate |

## Running this as a 5-person capstone

- **One topic, one primary owner.** Pick 5 of the 7 (or split the two
  biggest — Fuzzing and CI — across a pair each, and take a 6th topic with
  the person freed up).
- **Definition of done, project-native:** wherever a topic can be phrased as
  a gate that passes or fails (a CI job, a fuzz corpus not crashing, a
  semver-check passing, `cargo publish --dry-run` succeeding for all 34
  crates), phrase it that way. This project already grades itself that way
  — match its own standard rather than inventing a new one.
  [`../../bench_harness/gate/readme.md`](../../bench_harness/gate/readme.md)
  is the existing model to follow.
  - [`../../bench_harness/docs/decisions/002_defects_are_graded_by_mutation_not_coverage.md`](../../bench_harness/docs/decisions/002_defects_are_graded_by_mutation_not_coverage.md)
    is worth reading before designing a new gate — it's the project's own
    honest accounting of what a passing gate doesn't actually guarantee,
    and a useful template for scoping a new one honestly.
- **Integration point at the end:** a joint session where each owner
  demos their gate going from red to green on a real (planted) violation —
  the same "prove it can fail" standard
  [`../../bench_harness/docs/decisions/001_a_gate_counts_only_after_failing_for_its_own_reason.md`](../../bench_harness/docs/decisions/001_a_gate_counts_only_after_failing_for_its_own_reason.md)
  already holds itself to.
