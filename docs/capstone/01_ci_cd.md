# Topic 1: CI/CD — make the gates actually gate

## The gap

There is no `.github/workflows/` at all. The entire `g1`-`g22` verification
suite (`../../bench_harness/gate/`), the full test matrix, `loom`,
`trybuild`, and the doc build — all of it only ever runs when a human
remembers to run it locally. A regression can sit unnoticed indefinitely.

## Deliverables

- A GitHub Actions workflow that runs on every push/PR:
  - `cargo build` + `cargo nextest run` (or `cargo test`) across the
    workspace, on stable (and the MSRV once [Topic 4](04_publish_readiness.md)
    picks one)
  - The existing `../../bench_harness/gate/` suite — this repo already
    grades itself; CI should just be the thing that runs that grading on
    every change instead of leaving it to memory
  - `loom` tests (they're a separate, usually-slower job — run them on
    their own rather than blocking every quick push)
  - `trybuild` UI tests
  - `cargo doc --no-deps` (already passes clean today — keep it that way)
- Real status badges on the root [`../../readme.md`](../../readme.md). It
  currently ships four static/aspirational badges (Rust edition, "unsafe
  forbidden", crate count, "loom-checked") — replace them with ones backed
  by an actual workflow run, so a green badge means something.
- A style/lint job — but don't reach for `cargo fmt`. This project uses its
  own codestyle (2-space indents and other conventions from its rulebooks);
  a CI style check should enforce whatever local convention already
  governs the codebase, not the default formatter.

## Why this crate family in particular

The project's own culture is "a gate that can fail, not a claim that
sounds true" (see `../../bench_harness/docs/invariant/001_gate_non_vacuity.md`).
Right now every one of those gates can still fail — just not automatically,
and not on every change. This topic is the one that makes every other
topic's gate actually mean something going forward.
