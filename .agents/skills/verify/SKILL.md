---
name: verify
description: Run the right verb level for a change in ring and triage whatever goes red — nextest, doctests, clippy, udeps, audit, loom, and the bench_harness gates G1–G22. Use before declaring work done or when a gate reports NOT REACHED.
---

# Verify

## Pick the level

| Change | Run |
|---|---|
| Iterating | `./verb/test_only crate::<name> filter::<substr>` |
| Any code change, before "done" | `./verb/test level::3` — nextest, doctests, clippy, all `-D warnings` |
| Atomics, orderings, slot lifecycle | `./verb/loom` (or `crate::<name>`) as well |
| Lints, `unsafe`, a crate's `docs/`, manifests, gates | `./verb/test level::5` — adds udeps, audit, the gate suite |
| Formatting | `./verb/fmt check::1` |

`level::5` is slow, G12 rewrites `src/` while it runs, and G1 needs `cargo-tarpaulin`
(`./verb/setup check::1`). Run it in the background with `CARGO_TARGET_DIR=target-gate`, never
beside another build of the tree, and do not edit crates meanwhile. One gate:
`./verb/gate gate::g6`; one stage: `./verb/gate stage::S5`.

## Baseline first

Several gates are red on `master` from the extraction (as of 2026-09-30: G6 stale `ring_core`
allowlist entry, G13 never surveyed, G15 quoted output predating the rustfmt pass, a handful
in G16/G17/G20). Before attributing a red gate to the change, run the same gate on `master` in
a second checkout — its directory must also be named `ring`:
`git worktree add ../base/ring master`. Report the delta, not the absolute.

## Triage

- **nextest** — rerun alone: `./<crate>/verb/test_only filter::<name>`. A trybuild failure in
  `ring_handle` (`ui_test`): diff expected vs actual; `-->` path or `note:` lines only → edit
  those lines; `error[E…]`/`help:` changed → a real change in what is rejected, stop and ask.
  Never `TRYBUILD=overwrite` blindly.
- **doctests** — a failing example is a documentation bug; fix the example, not the test.
- **clippy** — fix the code; `#[expect(lint, reason = "…")]` on the item only if the lint is
  wrong there. Never touch `[workspace.lints]` to get green.
- **udeps** — remove the dependency, or explain why it is used only under a cfg.
- **audit** — skipped without `Cargo.lock` (gitignored); say so in the report.
- **loom** — read the failing interleaving; see `concurrency-debug`.
- **G2** — a crate with no `pub` item, a "skeleton" phrase, a warning under
  `--no-default-features --all-targets`, or a failing doctest.
- **G3** — a declared feature id no test cites as `docs/feature/<id>_…`.
- **G4** — `tests/manual/readme.md` lacks a dated `## Run Record`.
- **G5** — a manifest outside the family depends on a non-export crate.
- **G6** — missing `[lints] workspace = true`, an unlisted `#![allow(unsafe_code)]`, an
  allowlisted crate whose `docs/workaround/readme.md` does not mention unsafe, or a stale entry.
- **G12** — a `.mutant`'s `from` block no longer matches after a refactor: update the mutant
  file, keep the defect it plants. "Restoration mismatch" means a file still carries a planted
  defect — believe it and restore.
- **G13** — any edit to `src/` or `tests/` makes the crate stale. Re-survey with
  `bash bench_harness/gate/mutant_survey.sh <crate>` (slow; needs `cargo-mutants`).
- **G14–G17, G20, G21** — the doc corpus; follow the `doc-corpus` skill. A new G15 `STALE` in a
  crate you did not touch is usually a recipe quoting a file you did (`Cargo.toml` lint table,
  a manifest, a signature): refresh its `Live output` and any prose the change made false.
  A diff that is only line order is a recipe missing `sort` — not your regression.

## Report

Per step: passed / failed (with the output) / skipped (with why). Never say "all gates pass"
without the run's `N/N gates reached` line.

## Script edits

A patch script whose anchor does not match is a silent no-op, and a trailing `echo ok` still
prints. Assert the anchor matches exactly once, then grep the result.
