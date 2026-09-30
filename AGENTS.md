# AGENTS.md

Rules for any coding agent in this repository (Codex, Cursor, Copilot, Gemini, Claude Code —
`CLAUDE.md` imports this file). Humans: the same rules live in `CONTRIBUTING.md` and
`CODESTYLE.md`.

`ring` is a lock-free many-producer, one-consumer write path: 33 `ring_*` crates, one
mechanism each, plus `bench_harness`, which grades them. Before non-trivial work read
`ARCHITECTURE.md`, `CODESTYLE.md`, and the touched crate's `readme.md` and `docs/readme.md`.

## Commands

Every operation is a `verb/` script; parameters are `key::val`, never `--flag`.

```sh
./verb/test_only crate::ring_spsc filter::wrap  # while iterating
./verb/fmt                                      # rustfmt (nightly) + taplo; check::1 verifies only
./verb/lint                                     # clippy -D warnings; crate::<name> narrows
./verb/test level::3                            # before declaring done: nextest + doctests + clippy
./verb/loom                                     # loom models; after any change to atomics or orderings
./verb/test level::5                            # + udeps, audit, the gate suite (slow, see below)
./verb/doc                                      # rustdoc from a clean slate, -D warnings
```

Each crate also has `./<crate>/verb/{test,test_only,lint,build}`. The root `justfile` runs the
same commands (`just --list`; `just test 3`, `just test-only wrap ring_spsc`); from a crate
directory its recipes scope to that crate.

## Hard rules

- **The checkout directory must be named `ring`.** The gate harness resolves crates through the
  parent directory. Worktrees too: `…/<anything>/ring`.
- **Gates are slow and G12 rewrites `src/`** (restores it afterwards). Never run
  `./verb/gate` or `level::5` alongside another cargo build of the same tree, never edit while
  it runs; use `CARGO_TARGET_DIR=target-gate`. Some gates are red on `master` from the
  extraction — compare against `master` before blaming a change.
- **`unsafe`** only in the allowlisted crates (`bench_harness/gate/declared/ring/unsafe_allowlist.txt`),
  one operation per block, each with a `// SAFETY:` naming why aliasing and ordering hold.
  Never add a crate to the allowlist on your own.
- **Atomics.** Orderings come from the named constants (`GATING`, `PUBLISH`, `OBSERVE`,
  `COMMIT`, `HANDOFF`, `OWN`) or are justified in a comment naming the paired load or store.
  Never "fix" a race by strengthening to `SeqCst`. A change to an atomic protocol needs a
  `loom` model or an extended one, run with `./verb/loom`.
- **Lints.** Fix the code. Silence a lint only on the narrowest item with
  `#[expect(lint, reason = "…")]`. The one exception is the unsafe opt-out, which stays
  `#![allow(unsafe_code)]` because G6 greps for that form. Never weaken `[workspace.lints]` in
  a feature change.
- **Trybuild** (`ring_handle/tests/ui/*.stderr`): never re-bless with `TRYBUILD=overwrite`
  blindly. Diff first; only `-->` path and `note:` lines may change without a design reason.
  Those files also quote the source of `ring_spsc`, `ring_mpsc` and `ring_core` types, so
  editing those declarations can break `ring_handle`'s test.
- **Doc corpus.** A behaviour change updates the crate's `docs/` in the same change: recipes
  must still reproduce their `Live output:` byte for byte (G15). Recipes in *other* crates quote
  `Cargo.toml`, manifests and sources too, so after changing those run `./verb/gate gate::g15`
  and compare with `master`. Rules in `CODESTYLE.md` § Doc corpus. Never hand-edit `bench_harness/gate/declared/ring/surveyed/`.
- **Do not format** `*/tests/ui/*.rs` or anything under `bench_harness/gate/declared/` — both
  are byte-exact fixtures outside `cargo fmt`'s reach.
- **Prose drifts; code does not.** Module docs still cite monorepo files that do not exist here
  (`docs/feature/`, `docs/decision/`, `docs/plan/`). Trust manifests, code and gate output over
  prose, and correct the prose you touch.

## How to work

- **Think before code** for anything touching a protocol, an ordering, `unsafe`, or a crate
  boundary: ELIMINATE → SIMPLIFY → REUSE → CREATE — can the need go away, can it be solved with
  less, does a crate already do it — before writing anything new. Write the calling code
  first. For a one-line fix, don't.
- **The diff is the review artifact.** Every changed line must be needed by the task: no drive-by
  reformatting, no rewording of untouched comments or docs, no reflowed paragraphs. Before
  calling a branch ready, read `git diff master...HEAD` and justify every removed line.
- **Scope.** Capstone topics (`docs/capstone/`) have one owner each; do not edit another topic's
  area (CI workflows, `deny.toml`, MSRV and crate metadata, `CHANGELOG`, `no_std`, benchmark
  reporting) unless that is the task.
- **Tests are the spec.** A bug fix starts with a failing test; a concurrency bug with a loom
  model if the interleaving can be expressed.
- **Report honestly.** Paste failing output verbatim; say what you skipped and why; never claim
  a gate passed without running it.
- **Commits**: Conventional Commits (`fix(ring_mpsc): …`), imperative, lowercase after the
  colon, no trailing period, body says why. No AI attribution anywhere — no model
  `Co-Authored-By`, no "Generated with" line — in commits, PRs, code or docs.

## Checklists

`.claude/skills/<name>/SKILL.md` — plain markdown, usable by any agent:

| Skill                 | Use when                                                               |
|-----------------------|------------------------------------------------------------------------|
| `verify`              | Running the verbs and gates, triaging a red one                        |
| `lock-free-review`    | Reviewing or writing atomics, orderings, `unsafe`, slot lifecycle      |
| `concurrency-debug`   | A hang, a lost or duplicated record, a flaky or loom-failing test      |
| `doc-corpus`          | Adding or changing a crate's `docs/` instance, finding or recipe       |
| `new-crate`           | Adding a crate to the family                                           |
| `new-gate`            | Adding a stage gate, or any check that must be able to fail            |
| `decision`            | Recording an open trade-off in a crate's `docs/decisions/`             |
| `bench`               | Running the write-path comparison and recording numbers                |
| `capstone-topic`      | Driving a capstone topic to a demonstrable, failing-then-passing check |
| `architecture-review` | Reviewing a proposal that moves a crate boundary, protocol or contract |
| `plan`                | Writing an implementation plan or a prompt for another agent           |
| `grill`               | Stress-testing a design by interview, one round of questions at a time |

## Local setup

The repository assumes a Rust toolchain (`rust-toolchain.toml`), a nightly for `verb/fmt`, and
what `./verb/setup` installs — nothing else. Machine-specific tools (MCP servers, IDE bridges,
personal indexes) go in `CLAUDE.local.md` / `.claude/settings.local.json`, both gitignored.
Never make a shared instruction depend on a tool other contributors may not have.
