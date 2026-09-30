# Contributing

Short version. Code rules: [`CODESTYLE.md`](CODESTYLE.md). How the family fits together:
[`ARCHITECTURE.md`](ARCHITECTURE.md). What to work on: [`docs/capstone/`](docs/capstone/readme.md).

## Setup

```sh
git clone https://github.com/Wandalen/ring    # the directory MUST be named `ring`
cd ring
git config blame.ignoreRevsFile .git-blame-ignore-revs   # blame skips reformatting commits
./verb/setup check::1                           # what is missing
./verb/setup                                    # install it
./verb/test level::1                            # first build + tests
```

- **The checkout directory must be called `ring`.** The gate harness resolves crates as
  `<parent>/ring/<crate>` (`bench_harness/gate/common.sh`); under any other name every gate
  fails with "declared crate(s) absent". This applies to extra worktrees too:
  `git worktree add ../wt-fuzz/ring my-branch`.
- Toolchain is pinned in `rust-toolchain.toml` (rustup installs it on first use). A nightly is
  also needed for `verb/fmt` and `verb/test level::4`.
- Every task is a `verb/` script — see [`verb/readme.md`](verb/readme.md). Parameters are
  `key::val`, never `--flag`. `./verb/verbs` lists them.
- The [`justfile`](justfile) has the same tasks as `just` recipes — `just test 3`, `just lint`,
  `just test-only wrap`; run from a crate directory, a recipe scopes to that crate. The repo is
  moving to it; until then both run identical commands.

## Day to day

| When                                                | Run                                                                              |
|-----------------------------------------------------|----------------------------------------------------------------------------------|
| Iterating on one crate                              | `./ring_spsc/verb/test_only filter::wrap` or `./verb/test_only crate::ring_spsc` |
| Before every commit                                 | `./verb/fmt` then `./verb/lint`                                                  |
| Before opening a PR                                 | `./verb/test level::3` (nextest, doctests, clippy)                               |
| Touched atomics, orderings or the slot lifecycle    | `./verb/loom` — every loom model, in its own `target-loom/`                      |
| Touched gates, lints, `unsafe` or a crate's `docs/` | `./verb/test level::5` — adds udeps, audit and the gate suite                    |
| Rustdoc                                             | `./verb/doc` (always from a clean doc output dir)                                |

`level::5` runs `bench_harness/gate/`: slow (G1 is tarpaulin), and G12 temporarily rewrites
`src/`, so never run it next to another cargo build of the same tree. Launch it detached with
its own target dir: `CARGO_TARGET_DIR=target-gate ./verb/gate > -0001_gate.log 2>&1 &`.
Several gates are red on `master` from the extraction (G6, G13, and the doc-corpus gates G15–G17,
G20 — the corpus still quotes pre-rustfmt source). Run a red gate on `master` before blaming
your change.

## Workflow

1. Branch from `master` with a short slug: `ci-gates`, `fuzz-spsc`, `fix-mpsc-wrap`.
2. One topic or fix per PR. Capstone topics have one owner each — coordinate before touching
   another topic's files (CI → topic 1, `deny.toml`/audit → 3, MSRV, crate metadata and
   `CHANGELOG` → 4, rustdoc → 5, `no_std` → 6, `ring_bench` reporting → 7).
3. Commits follow [Conventional Commits](https://www.conventionalcommits.org), as the history
   does: `type(scope): summary` — `feat`, `fix`, `docs`, `test`, `refactor`, `perf`, `chore`,
   `ci`. Imperative, lowercase after the colon, no trailing period. Body says *why*.
4. Fill the PR template. "How it is proven" means commands and their output; a new check
   must be shown failing on a planted violation before it passes — the project's standard
   ([`bench_harness/docs/invariant/001_gate_non_vacuity.md`](bench_harness/docs/invariant/001_gate_non_vacuity.md)).
5. Get a teammate's review before merging.

## What a PR needs

- Tests that fail without the change. Concurrency fixes add or extend a `loom` model.
- Docs on every new `pub` item; the crate's `docs/` corpus and `readme.md` kept true
  (G14–G17, G20, G21 grade the corpus — see `CODESTYLE.md` § Doc corpus). A lint, manifest or
  source change can stale a recipe elsewhere that quotes it — `./verb/gate gate::g15` finds it.
- No new `#[allow]` — `#[expect(lint, reason = "…")]` on the item instead (the unsafe opt-out is
  the exception). No weakened workspace lint; a new lint lands with the fixes it needs.
- `unsafe` only in allowlisted crates
  ([`unsafe_allowlist.txt`](bench_harness/gate/declared/ring/unsafe_allowlist.txt)), one
  operation per block, each with `// SAFETY:`. A new crate on the list is a design decision,
  not a PR detail.
- A changed trybuild `.stderr` explained: toolchain wording or a real change in what is
  rejected ([`ring_handle/tests/ui_test.rs`](ring_handle/tests/ui_test.rs)).

## AI agents

Shared, tracked, tool-agnostic:

| File                        | Purpose                                                                                           |
|-----------------------------|---------------------------------------------------------------------------------------------------|
| [`AGENTS.md`](AGENTS.md)    | Rules for any coding agent (Codex, Cursor, Copilot, Gemini, Claude)                               |
| [`CLAUDE.md`](CLAUDE.md)    | Imports `AGENTS.md`, adds Claude Code specifics                                                   |
| `.claude/skills/*/SKILL.md` | Checklists (`/verify`, `/lock-free-review`, `/new-gate`, …) — plain markdown, usable by any agent |
| `.claude/settings.json`     | Shared permissions and the rustfmt-on-edit hook                                                   |

Yours, never committed (both in `.gitignore`):

| File                          | What belongs there                                                                         |
|-------------------------------|--------------------------------------------------------------------------------------------|
| `CLAUDE.local.md`             | Local MCP servers, IDE bridges, personal indexes, machine paths. Loads next to `CLAUDE.md` |
| `.claude/settings.local.json` | Permissions for tools only you have; merges with the shared file                           |

Nothing in a shared file may depend on a tool other contributors may not have. Knowledge that
must outlive a session goes into the repo — a crate's `docs/`, a decision record, a PR
description — not a personal note store.

Agent-written commits carry no AI attribution: no `Co-Authored-By` trailer for a model, no
"Generated with" line. The human who opens the PR owns the change.

## Reporting

Bugs and tasks: GitHub issues with the templates. Vulnerabilities: [`SECURITY.md`](SECURITY.md),
not a public issue.

## Conduct

Be direct, assume good intent, argue about the code with evidence — a command and its output
beats an opinion.
