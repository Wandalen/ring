# Code style

Rules for this repository, written down from what the code already does. Lints and gates
enforce most of them; the rest are review. When the code and this file disagree, the code in
the surrounding crate wins and this file gets fixed.

## Formatting

- `./verb/fmt` formats everything; `./verb/fmt check::1` is the check. Never hand-format.
- Rust: nightly rustfmt with [`rustfmt.toml`](rustfmt.toml) — 4-space indent, 100 columns,
  imports grouped std → external → crate and merged per module, doc examples formatted too.
  Code that cannot fit in 100 columns fails `verb/fmt`; long comment and doc-table lines are
  tolerated but should not be added. Stable `cargo fmt` silently ignores the unstable options
  and produces a different result.
- TOML: taplo with [`taplo.toml`](taplo.toml) — 4-space indent, 100 columns, dependencies
  sorted alphabetically within each comment-separated group. `[workspace] members` keeps its
  dependency order.
- Not formatted, ever: `*/tests/ui/*.rs` (trybuild pins their line numbers) and
  `bench_harness/gate/declared/**` (gate fixtures). `cargo fmt` does not reach them; do not
  run `rustfmt` on them by hand.
- Shell (`verb/`, gates) and Python (gate checkers): 2-space indent.
  [`.editorconfig`](.editorconfig) carries all of this to editors.

## Lints

- The workspace table in [`Cargo.toml`](Cargo.toml) applies to every crate through
  `[lints] workspace = true` (G6 fails a crate without it). `verb/lint`, `verb/test` and G9
  run clippy with `-D warnings`, so `warn` means "fails the build".
- Silence a lint on the narrowest item with `#[expect(lint, reason = "…")]`, saying why the
  lint is wrong *here*. Exception: the crate-level unsafe opt-out is spelled
  `#![allow(unsafe_code)]` because G6 greps for that form.
- A new workspace lint lands together with the fixes it needs. Never weaken the table in a
  feature change.

## Crates

- One library per crate, all in `src/lib.rs`. `lib.rs` opens with a `//!` block whose first
  line is the crate summary — the same sentence as `readme.md` line 3 and the manifest
  `description` — followed by `#![deny(missing_docs)]`.
- Manifest: `version`, `edition`, `license`, `repository` inherited (`.workspace = true`);
  `readme = "readme.md"`, `description` and `publish` of its own; `[lints] workspace = true`.
- Every dependency — sibling or external, normal or dev — is `name = { workspace = true }`
  (plus `optional = true` where a feature enables it). Versions, paths, default features and
  feature sets live once, in the root `[workspace.dependencies]`. `publish = false` crates are
  not listed there, so a published crate cannot come to depend on one.
- `./verb/publish_check` (`just publish-check`) packages and builds every publishable crate the
  way crates.io would; it must pass before a release.
- Workspace `members` are in dependency order, not alphabetical: every crate appears below
  everything it depends on.
- Adding a crate touches a dozen places — use the `new-crate` checklist
  (`.claude/skills/new-crate/SKILL.md`).

## `unsafe`

- Only in crates on [`unsafe_allowlist.txt`](bench_harness/gate/declared/ring/unsafe_allowlist.txt),
  each justified in its `docs/workaround/readme.md` (G6).
- One unsafe operation per block (`clippy::multiple_unsafe_ops_per_block`), each preceded by
  `// SAFETY:` that says why no aliasing `&mut` exists and which happens-before edge makes the
  data visible — by the name of the ordering constant.
- Every `unsafe fn` has a `/// # Safety` section; bodies still use explicit `unsafe {}` blocks.
- `UnsafeCell` wraps one slot, never the whole buffer (Miri rejected the outer form, see
  `ring_spsc/src/lib.rs`).

## Atomics and concurrency

- Every ordering is a named constant whose doc says which load or store it pairs with and what
  breaks if it is weakened (`GATING`, `PUBLISH`, `OBSERVE`, `COMMIT`, `HANDOFF`, `OWN`). An
  inline ordering needs the same justification in a comment. No defaulted `SeqCst`.
- Ring atomics come from `ring_atomic` (`AtomicSeq`, `SeqCell`) — the one `cfg(loom)` seam.
  An atomic created elsewhere is invisible to loom.
- Cursors live on their own cache line: `ring_align::CacheAligned` / `ring_cursor::PaddedCursor`.
- `Seq` arithmetic stays unfolded (`ring_seqno`); fold to a slot index only through
  `ring_index` (`seq & mask`).
- Waits are bounded (`ring_wait`). No locks on the write path; `ring_trace` and the
  `ring_bench` mutex baseline are the deliberate exceptions.
- A guard whose drop does the work is `#[must_use = "…"]` with the consequence in the message
  (`ring_claim::Claim`).

## Rustdoc

- Every `pub` item is documented (`missing_docs`). The first sentence is a noun phrase for
  types, constructors and getters ("A value given a whole cache line to itself."), `How …`
  for counts, `Whether …` for predicates, and the imperative for actions ("Publish one
  record, or hand it back.").
- Refer to items as intra-doc links: ``[`RingConfig::new`]``. Broken and private links are
  denied.
- `# Errors` on every function returning `Result`; `# Panics` when a caller's input can panic
  it; `# Safety` on every `unsafe fn`.
- Examples follow the prose without an `# Examples` heading.
- Crate-level docs link to the crate's `docs/` corpus rather than repeating it.

## Tests

- Integration tests only: `tests/<stem>_test.rs` (`ring_spsc/tests/spsc_test.rs`); no
  `#[cfg(test)]` modules in `src/`. Each file opens with `//!` naming the crate and the
  feature it claims as `docs/feature/<id>_…` — G3 greps for that token.
- Test names are behaviour sentences: `sequences_are_issued_consecutively_across_a_wrap`.
  A `///` line says what the test proves when the name cannot.
- `unwrap`/`expect` are fine in tests.
- Files that are not loom models carry `#![cfg(not(loom))]`; models sit in a `#[cfg(loom)]`
  module (usually `mod exhaustive`) and use `ring_testkit::{ leak, leak_ends }` for `'static`
  ends. Run them with `./verb/loom`.
- Every crate keeps `tests/manual/readme.md` for what automation cannot establish: stages
  `## <Letter><n> — <claim>` with `**Expected:**` written before running, and a
  `## Run Record` section with ISO dates (G4).

## Doc corpus

Each crate's `docs/` is graded by G14–G17, G20, G21 (`bench_harness/gate/corpus/`). The
rulebooks it cites (`doc_des.rulebook.md` and others) were not extracted with the family; the
gates, [`corpus_standard.txt`](bench_harness/gate/declared/ring/corpus_standard.txt) and the
fixtures under `bench_harness/gate/declared/ring/corpus_control/` are the standard now.

- Exactly 13 definition directories (`algorithm`, `api`, `data_structure`, `decisions`,
  `integration`, `invariant`, `item`, `lifecycle`, `non_functional_requirement`, `pattern`,
  `pitfall`, `type`, `workaround`) plus `definition/`, the Module Index. No others.
- Instances are `NNN_snake_case.md`, numbered from `001` per directory, titled
  `# <Type>: <Title>`, opening with `### Scope`.
- Findings are `### XXn — Title` with the crate's two-letter prefix, numbered contiguously,
  and appear in three places that must agree exactly: the heading, the definition readme's
  `### Findings Recorded Here` row, and the Module Index `## Findings` row. Tiers come from
  the declared set; a **bold** tier needs a `**Disposition:** applied|declined — …` line.
- Recipes are ```` ```sh ```` blocks that start with `cd "$(git rev-parse --show-toplevel)"`,
  use `command grep`, never address source by line number, and are followed by `Live output:`
  and a bare fence that must match byte for byte (G15).
- Relative links must resolve; test names in a `### Tests` table must exist (G16).

Procedure and templates: `.claude/skills/doc-corpus/SKILL.md`.

## Comments

Say what the code cannot: why, the constraint, the consequence, a pointer to the doc instance
that holds the argument (`→ docs/invariant/002`). Do not narrate code. Gate scripts record
corrections as `# Fix(<slug>):` / `# Finding(<slug>):` blocks with root cause and pitfall.

## Scripts

`justfile`: canonical `just --fmt` form (`just fmt` applies it), a one-line `#` comment above
each recipe — it is what `just --list` shows. `verb/` scripts: `#!/usr/bin/env bash`, `#@ purpose:` and `#@ tags:` header lines (read by
`./verb/verbs`), `set -euo pipefail`, `cd` to the repo root, parameters as `key::val`, an
unknown parameter exits 2, `dry::1` prints the commands instead of running them. Gate scripts
use `set -uo pipefail` and end in exactly one `pass` or `fail` (see
[`bench_harness/gate/readme.md`](bench_harness/gate/readme.md)).
