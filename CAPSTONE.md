# Capstone: Make `ring` Production-Ready

## The assignment

Five developers, one shared target: `ring`, a from-scratch 34-crate lock-free
concurrency write path (claim/publish/commit protocol, `ring_claim` /
`ring_publish` / `ring_barrier` / `ring_gating`), built as a measured
alternative to a mutex-guarded queue and `crossbeam-queue::ArrayQueue` for
many-producer/single-consumer, hard-latency-budget workloads.

- **Repo:** [`Wandalen/ring`](https://github.com/Wandalen/ring), branch `master`
- **Start here:** [`README.md`](README.md) — architecture, the 11-tier
  dependency graph, and the build-vs-buy rationale

This is a real codebase, not a toy: unsafe is `deny`-by-default with a
3-crate allowlist (`ring_spsc`, `ring_mpsc`, `ring_core` —
`bench_harness/gate/declared/ring/unsafe_allowlist.txt`), it already runs
`loom` model-checking on part of the concurrency-critical tier, and it has
its own home-grown stage-gate verification suite (`bench_harness/gate/`,
gates named `g1`...`g22`) enforcing coverage, doc freshness, and unsafe-code
review. Whatever a topic below adds, wire it in the same spirit: a gate that
can fail, not a claim that sounds true.

**Two things to know before anyone opens an editor:**

1. `git log` here has exactly **one commit** (`f0eb4c1`, "initial commit").
   The crate family was extracted out of a larger internal monorepo as a
   fresh squashed snapshot — there's no history to `git blame` past that
   commit.
2. As of 2026-09-29 (the day of extraction), the working tree still had a
   large post-extraction fixup sitting uncommitted (every crate's
   `Cargo.toml`/`src/lib.rs`, a rescoped gate-family declaration, a new
   `LICENSE` file). That's very likely resolved by the time you read this —
   just run `git status`/`git log` yourself rather than assuming either
   state.

## Topics — pick 5 (or all 9, if the team wants stretch goals)

Each is scoped to be ownable by one person, produces something
demonstrable, and closes a gap that's real right now, not hypothetical —
grounded in what's actually in (or missing from) the repo today.

### 1. CI/CD — make the gates actually gate

**The gap:** there is no `.github/workflows/` at all. The entire `g1`-`g22`
verification suite, the full test matrix, `loom`, `trybuild`, doc builds —
all of it only ever runs when a human remembers to run it locally.

**Deliverables:** a GitHub Actions workflow that runs on every push/PR:
build + test (stable, and ideally the MSRV once Topic 4 picks one), the
existing `bench_harness/gate/` suite, `loom` tests, `trybuild` UI tests, and
`cargo doc --no-deps`. Real status badges on `README.md` (it currently has
four static/aspirational badges — replace them with ones backed by an
actual workflow run). Don't reach for `cargo fmt` — this project uses its
own codestyle; a CI style check should call whatever local convention
already governs it, not the default formatter.

### 2. Fuzzing the lock-free core

**The gap:** no `fuzz/` directory, no `cargo-fuzz` anywhere in the repo.
`loom` checks a bounded number of thread interleavings; it says nothing
about the input space — capacity edge cases, claim/publish/commit sequences
that wrap the ring, overflow behavior.

**Deliverables:** a `cargo-fuzz` (or AFL) harness targeting the public API
of the three unsafe-allowlisted crates (`ring_spsc`, `ring_mpsc`,
`ring_core`), a documented corpus of interesting seed inputs, and a CI job
that runs a short fuzz pass on every PR (long-running fuzzing as a separate
scheduled job, not blocking every push). This is the topic most likely to
turn up a real bug in code everyone currently trusts.

### 3. Supply-chain & dependency security

**The gap:** no `deny.toml`, no `audit.toml`, nothing checking the ~5
external dependencies (crossbeam-queue, crossbeam-channel, crossbeam-utils,
etc.) for license compatibility or known advisories.

**Deliverables:** `cargo-deny` configured for license policy (everything
must stay compatible with the workspace's own MIT license) and duplicate/
banned-crate checks; `cargo-audit` wired into CI against the RUSTSEC
database; a documented policy for what happens when either one goes red.

### 4. crates.io publish readiness & release process

**The gap:** every crate is already `version = "0.1.0"` with `publish =
true` set (see `ring_core/Cargo.toml`) — this family is *meant* to ship,
but nothing verifies it actually can, and there's no release process.

**Deliverables:** `cargo publish --dry-run` passing for all 34 crates in
dependency order (the `README.md` mermaid graph gives you that order
already); an audit of per-crate metadata gaps (`keywords`, `categories`,
`documentation`) for crates.io discoverability; a documented MSRV policy
tested in CI; `cargo-semver-checks` wired in so a 0.1.x → 0.1.(x+1) bump
can't silently break a public API; a CHANGELOG convention.

### 5. Public API docs — docs.rs-grade rustdoc

**The gap:** this project has an unusually rich *internal* doc corpus per
crate (`docs/api/`, `docs/algorithm/`, `docs/decisions/`, `docs/invariant/`,
...) — but that's design-rationale documentation, not the `///` rustdoc
that actually renders on docs.rs. `cargo doc --no-deps` currently builds
clean, which is a good floor, not a ceiling.

**Deliverables:** `#![warn(missing_docs)]` (or `deny`, if the team's
ambitious) across all 34 crates; every public item gets a doc comment, and
the ones that benefit from it get a runnable example; a polished top-level
`lib.rs` doc comment per crate that links back to the richer `docs/`
corpus instead of duplicating it.

### 6. `no_std` / embedded-readiness audit

**The gap:** only 3 of 34 crates currently mention `no_std`
(`ring_types`, `ring_overflow`, `ring_stats`). A hand-built,
latency-critical ring buffer is exactly the kind of thing that's valuable
in `no_std`/embedded contexts — right now that's mostly unexplored.

**Deliverables:** a crate-by-crate audit of the dependency graph (start
from Tier 0 — `ring_types`, `ring_align` — and work up) deciding which
crates can realistically go `no_std` and which fundamentally can't
(`ring_wait`'s parking strategies and `ring_tls`'s thread-locals are the
obvious candidates that need `std`) — documented, not just asserted; `no_std`
feature-gated where it's achievable; a CI job building the `no_std` subset
with `--no-default-features`.

### 7. Continuous benchmark reporting

**The gap:** `ring_bench` (a `criterion` harness) exists and is how the
original build-vs-buy decision got made, but its results live in whoever's
terminal last ran it. There's no regression detection and no public report.

**Deliverables:** a `criterion` HTML report published somewhere durable
(GitHub Pages is the easy option), a CI benchmark job comparing the
in-house ring against the crossbeam baseline on every merge to `master`,
and a regression gate (fail if p99 latency regresses past a documented
threshold). This directly extends the project's own "measured, not argued"
principle from a one-time decision into an ongoing one.

### 8. Pay down the extraction debt

**The gap:** this family was extracted from a larger monorepo very
recently, and the seams show. Concretely, as of extraction: (a) one real
test failure — `ring_handle::ui_test::the_forbidden_programs_are_rejected`
(`ring_handle/tests/ui/producer_shared_across_threads.rs`), a `trybuild`
compile-fail test whose recorded expected-stderr likely embeds a
workspace-relative path that shifted when the crate moved — needs
investigating and a deliberate `TRYBUILD=overwrite` re-bless, not a blind
rerun; (b) the repo had no `verb/test` of its own — it was silently relying
on the parent monorepo's copy; (c) `loom` currently only covers 6 of 34
crates, and the unsafe-allowlisted tier is the obvious place to grow that
coverage first. Check current state first — some of this may already be
fixed.

**Deliverables:** the trybuild test fixed and understood (not just
re-blessed away), a self-contained dev-script/`verb/test` that doesn't
depend on anything outside this repo, and `loom` coverage extended to every
crate on the unsafe allowlist plus its direct dependents. This is the
topic with the most pre-existing investigation already done — a good pick
for whoever wants to start from a clear problem statement instead of a
blank page.

### 9. Onboarding, contributor experience & doc-corpus cleanup

**The gap:** two kinds. First, process: no `CONTRIBUTING.md`, no issue/PR
templates, no "five minutes to your first build" guide. Second, content
drift from the move itself — the family's own internal freshness gate
(`bench_harness/gate/corpus/recipes.py`, "G15") was still reporting ~362
problems (325 stale-output, 35 unverifiable-by-design, 2 missing-recipe) as
of the extraction.

**Deliverables:** `CONTRIBUTING.md` + a real quickstart; the G15 problem
count driven toward zero (`bench_harness/docs/guide/002_the_four_verdicts.md`
explains what each finding type means and what fixing it looks like).
Less flashy than fuzzing or CI, but a library nobody can onboard to isn't
production-ready no matter how correct the code underneath it is — and
this topic has the lowest floor to get started, useful if one teammate
wants a gentler on-ramp into the codebase before picking up something
harder.

## Running this as a 5-person capstone

- **One topic, one primary owner.** Pick 5 of the 9 (or split the two
  biggest — Fuzzing and CI — across a pair each, and take a 6th topic with
  the person freed up).
- **Definition of done, project-native:** wherever a topic can be phrased as
  a gate that passes or fails (a CI job, a fuzz corpus not crashing, a
  semver-check passing, `cargo publish --dry-run` succeeding for all 34
  crates), phrase it that way. This project already grades itself that way
  — match its own standard rather than inventing a new one.
  `bench_harness/docs/guide/001_running_the_verdicts_yourself.md` is the
  existing model to follow.
  - `bench_harness/docs/guide/003_what_the_gates_do_not_prove.md` is worth
    reading before designing a new gate — it's the project's own honest
    accounting of what a passing gate doesn't actually guarantee, and a
    useful template for scoping a new one honestly.
- **Integration point at the end:** a joint session where each owner
  demos their gate going from red to green on a real (planted) violation —
  the same "prove it can fail" standard
  `bench_harness/docs/invariant/001_gate_non_vacuity.md` already holds
  itself to.
