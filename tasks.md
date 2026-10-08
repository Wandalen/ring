# Tasks: production readiness + benchmark coverage

Source: code review of the workspace at `master` (post-#31/#32/#33).
Rule: **one task = one branch = one PR**. Check the box when the PR merges.

Bench tasks live under `perf/benches/` (criterion) and reuse
`perf/benches/harness/` (`Candidate`, `Run`, `Tx`/`Rx`, `ThreadedGroup`).
Prod tasks follow the repo's own standard: a gate that can fail, not a claim.

## P0 — production blockers

- [ ] **P0-1 — bench `ring_wait` strategies.** `pause` / `wait_until` /
  `for_space` / `for_data` (`ring_wait/src/lib.rs`) have zero benchmark
  coverage today (no bench file mentions `ring_wait`). Add a criterion suite
  across `WaitKind` × `spins` on the contended path.
  Done when: `perf/benches/wait.rs` (or equivalent) runs in `verb/bench`
  and shows per-strategy numbers on all bench runners.
- [ ] **P0-2 — split claim/publish benchmarks.** `micro::push_pop` measures
  everything together, but `claim` is documented as the only contended step.
  Measure claim / slot-write / publish separately for SPSC and MPSC.
  Done when: the three stages report independently and the contended stage
  is identifiable from the report tables.
- [ ] **P0-3 — fuzz the lock-free core.** No `fuzz/`, no `cargo-fuzz`.
  Harness over the public API of `ring_spsc` / `ring_mpsc` (`claim` /
  publish / commit sequences, capacity edges, wrap-around), seed corpus,
  short fuzz pass on PR + long scheduled run.
  Done when: `cargo fuzz` runs in CI and the corpus is documented.
- [ ] **P0-4 — benchmark regression gate.** `benchmarks.yml` deliberately
  blocks nothing. Add a fail-if-p99-regresses-past-threshold gate so a
  latency regression fails a check.
  Done when: a planted regression turns the gate red (prove-it-can-fail).
- [ ] **P0-5 — `DropNewest` production path.** Only `Fail` is usable;
  `DropOldest` is rejected with `PolicyUnsupported` outside the crossbeam
  backend (`ring_overflow/src/lib.rs:60`), Block/Overwrite are unimplemented
  by design (`ring_mpsc/src/lib.rs:878`). Decide per-policy: implement or
  document as out-of-scope, wire the decided set through `ring_core`.
  Done when: every `OverflowPolicy` variant is either exercised by tests +
  benches or explicitly documented as unsupported with a reason.

## P1 — benchmark coverage gaps

- [ ] **B-1 — batch drain parity.** `batch.rs` covers push batches; there is
  no matching scale for `try_recv_batch` / `drain_up_to`
  (`ring_core/src/lib.rs:604`).
- [ ] **B-2 — overflow-policy cost.** `Fail` retry-loop vs `DropNewest`:
  cost of `dropped()` accounting and the `resolve()` branch
  (`ring_overflow/src/lib.rs:181`).
- [ ] **B-3 — shutdown / drain-at-rest.** `ring_shutdown` graceful-stop +
  tail drain — the production shutdown scenario, unmeasured.
- [ ] **B-4 — `ring_poll` readiness polling.** Separate crate, zero benches.
- [ ] **B-5 — `ring_flush` / `ring_tls` staged flush.** Cost of `append`
  and staging flush (`append_cost_test` exists as a test only).
- [ ] **B-6 — construction cost.** `Ring::new` / `ends` / `split` /
  `ring_factory` across capacities 64/1024/16384 (matters for tests and
  short-lived rings).
- [ ] **B-7 — latency under load.** `latency.rs` measures the empty
  ping-pong path; add burst / N-producer / near-full-ring p99 — the shape
  that hurts in production.
- [ ] **B-8 — backends: native vs crossbeam.** `ring_core::new` vs
  `new_crossbeam` as separate candidates (the interim backend is currently
  compared against only indirectly via `arrayqueue`).
- [ ] **B-9 — `BytesSlot` vs `TypedSlot`.** `ring_event` promises one
  identical path for both shapes — prove it with numbers (after P0-5
  connects `publish_into` / `drain_from` in `ring_core`, or against the
  direct `set`/`take` path as baseline).

## P2 — release hardening

- [ ] **R-1 — MSRV + semver + changelog.** No `rust-version`, no
  `cargo-semver-checks`, no CHANGELOG. Pin MSRV, enforce in CI, adopt a
  changelog convention.
- [ ] **R-2 — `cargo publish --dry-run` for all publishable crates** in
  dependency order; fix metadata gaps (`keywords`, `categories`,
  `documentation`).
- [ ] **R-3 — supply chain.** `cargo-deny` (licenses/duplicates) +
  `cargo-audit` (RUSTSEC) in CI with a documented red-policy.
- [ ] **R-4 — connect `ring_event` into `ring_core`.** `publish_into` /
  `drain_from` are declared but dead (`ring_event/src/lib.rs:19`); route
  the production path through them or delete the crate.
- [ ] **R-5 — grow `no_std`.** 3/34 crates today (`ring_types`,
  `ring_overflow`, `ring_stats`); audit tier-by-tier from Tier 0 upward.
- [ ] **R-6 — grow `loom` to the unsafe allowlist + direct dependents**
  (today: `ring_atomic`, `ring_cursor`, `claim`, `consume`, `core`, …).
