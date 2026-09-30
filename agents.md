# agents

Rules for a coding agent working in this repository, whichever agent it is. Humans follow the
same rules; the detail is in [`codestyle.md`](codestyle.md) and
[`architecture.md`](architecture.md).

`ring` is a lock-free many-producer, one-consumer write path: 33 `ring_*` crates, one mechanism
each, plus `bench_harness`, which grades them. Before non-trivial work read `architecture.md`,
`codestyle.md`, and the touched crate's `readme.md` and `docs/readme.md`.

## Hooking up an agent

Nothing agent-specific is tracked; keep your agent's config local (`.git/info/exclude`).

- Point the agent at this file: `CLAUDE.md` with `@agents.md`, or a local `AGENTS.md` link
  (`ln -s agents.md AGENTS.md`) for agents that look for that name.
- Checklists live in [`.agents/skills/`](.agents/skills) as `<name>/SKILL.md` — plain markdown
  with a `name`/`description` header. Agents that load skills can use the directory as is
  (Claude Code: `ln -s ../.agents/skills .claude/skills`); any other agent reads the file when
  the task matches the table below.

## Commands

Every operation is a `verb/` script; parameters are `key::val`, never `--flag`.

```sh
./verb/setup check::1                           # what is missing; ./verb/setup installs it
./verb/test_only crate::ring_spsc filter::wrap  # while iterating
./verb/fmt                                      # nightly rustfmt; check::1 verifies only
./verb/lint                                     # clippy -D warnings; crate::<name> narrows
./verb/test level::3                            # before declaring done: nextest + doctests + clippy
./verb/loom                                     # loom models; after any change to atomics or orderings
./verb/test level::5                            # + udeps, audit, the gate suite (slow, see below)
./verb/doc                                      # rustdoc from a clean slate, -D warnings
```

Each crate also has `./<crate>/verb/{test,test_only,lint,build}`.

## Hard rules

- **The checkout directory must be named `ring`.** The gate harness resolves crates through the
  parent directory. Worktrees too: `…/<anything>/ring`.
- **Gates are slow and G12 rewrites `src/`** (restores it afterwards). Never run `./verb/gate`
  or `level::5` alongside another cargo build of the same tree, never edit while it runs; use
  `CARGO_TARGET_DIR=target-gate`. Some gates are red on `master` from the extraction — compare
  against `master` before blaming a change.
- **Formatting is the author's.** `./verb/fmt` with `rustfmt.toml` as it is; no other formatter,
  no reformatting of lines the task does not touch.
- **`unsafe`** only in the allowlisted crates (`bench_harness/gate/declared/ring/unsafe_allowlist.txt`),
  one operation per block, each with a `// SAFETY:` naming why aliasing and ordering hold.
  Never add a crate to the allowlist on your own.
- **Atomics.** Orderings come from the named constants (`GATING`, `PUBLISH`, `OBSERVE`,
  `COMMIT`, `HANDOFF`, `OWN`) or are justified in a comment naming the paired load or store.
  Never "fix" a race by strengthening to `SeqCst`. A change to an atomic protocol needs a
  `loom` model or an extended one, run with `./verb/loom`.
- **Lints.** Fix the code. Silence a lint only on the narrowest item, with the reason in a
  comment. Never weaken `[workspace.lints]` in a feature change.
- **Trybuild** (`ring_handle/tests/ui/*.stderr`): never re-bless with `TRYBUILD=overwrite`
  blindly. Diff first; only `-->` path and `note:` lines may change without a design reason.
  Those files quote the source of `ring_spsc`, `ring_mpsc` and `ring_core` types, so editing
  those declarations can break `ring_handle`'s test.
- **Doc corpus.** A behaviour change updates the crate's `docs/` in the same change: recipes
  must still reproduce their `Live output:` byte for byte (G15). Recipes in *other* crates quote
  `Cargo.toml`, manifests and sources too, so after changing those run `./verb/gate gate::g15`
  and compare with `master`. Never hand-edit `bench_harness/gate/declared/ring/surveyed/`.
- **Do not format** `*/tests/ui/*.rs` or anything under `bench_harness/gate/declared/` — both
  are byte-exact fixtures.
- **Prose drifts; code does not.** Module docs cite monorepo files that do not exist here
  (`docs/feature/`, `docs/decision/`, `docs/plan/`, the rulebooks). Trust manifests, code and
  gate output over prose, and correct the prose you touch.

## How to work

- **Think before code** for anything touching a protocol, an ordering, `unsafe`, or a crate
  boundary: can the need go away, can it be solved with less, does a crate already do it —
  before writing anything new.
- **The diff is the review artifact.** Every changed line must be needed by the task. Before
  calling a branch ready, read `git diff master...HEAD` and justify every removed line.
- **Scope.** Capstone topics (`docs/capstone/`) have one owner each; do not edit another topic's
  area unless that is the task.
- **Tests are the spec.** A bug fix starts with a failing test; a concurrency bug with a loom
  model if the interleaving can be expressed.
- **Report honestly.** Paste failing output verbatim; say what you skipped and why; never claim
  a gate passed without running it.
- **Commits**: Conventional Commits (`fix(ring_mpsc): …`), imperative, lowercase after the
  colon, no trailing period, body says why.

## Skills

| Skill               | Use when                                                          |
|---------------------|-------------------------------------------------------------------|
| `verify`            | Running the verbs and gates, triaging a red one                   |
| `lock-free-review`  | Reviewing or writing atomics, orderings, `unsafe`, slot lifecycle |
| `concurrency-debug` | A hang, a lost or duplicated record, a flaky or loom-failing test |
| `doc-corpus`        | Adding or changing a crate's `docs/` instance, finding or recipe  |
| `new-gate`          | Adding a stage gate, or any check that must be able to fail       |
