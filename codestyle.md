# Code style

Written down from what the code already does. The conventions come from the author's
rulebooks (`doc_des.rulebook.md` and others the docs cite), which were not extracted with the
family. When the code and this file disagree, the surrounding crate wins and this file gets
fixed.

## Files

- Lower-case file names: `readme.md`, `license`, `docs/<definition>/NNN_snake_case.md`.
- Every operation is a `verb/` script (see [`verb/readme.md`](verb/readme.md)).

## Formatting

- `./verb/fmt` formats everything; `./verb/fmt check::1` is the check.
- Rust: nightly rustfmt with the author's [`rustfmt.toml`](rustfmt.toml) — 2-space indent,
  130 columns, imports grouped std → external → crate. Stable `cargo fmt` ignores the unstable
  options and produces a different result. Do not change `rustfmt.toml` in a feature PR.
- Never formatted: `*/tests/ui/*.rs` (trybuild pins their line numbers) and
  `bench_harness/gate/declared/**` (gate fixtures).
- Shell (`verb/`, gates) and Python (gate checkers): 2-space indent.

## Lints

- The workspace tables in [`Cargo.toml`](Cargo.toml) reach every crate through
  `[lints] workspace = true` (G6 fails a crate without it). `verb/lint`, `verb/test` and G9 run
  clippy with `-D warnings`, so `warn` fails the build.
- Silence a lint with `#[allow(lint)]` on the narrowest item, with the reason in the comment
  right above it (`ring_core/tests/core_test.rs`, `len_zero`). The crate-level unsafe opt-out
  is `#![allow(unsafe_code)]`; G6 greps for that form.
- A new workspace lint lands together with the fixes it needs. Never weaken the tables in a
  feature change.

## Crates

- Code lives in `src/lib.rs`; `ring_types` alone splits by concept into modules.
- `lib.rs` opens with a `//!` block whose first line is the crate summary — the same sentence
  as `readme.md` line 3 and the manifest `description` — then `#![deny(missing_docs)]`.
- Manifest: copy a neighbouring crate's. Shared fields come from the workspace; the crate sets
  `description`, `readme = "readme.md"`, `publish` and `[lints] workspace = true`.
- Workspace `members` are in dependency order, not alphabetical.

## `unsafe`

- Only in crates on [`unsafe_allowlist.txt`](bench_harness/gate/declared/ring/unsafe_allowlist.txt),
  each justified in its `docs/workaround/readme.md` (G6).
- One unsafe operation per block, each preceded by `// SAFETY:` saying why no aliasing `&mut`
  exists and which happens-before edge makes the data visible, by the ordering constant's name.
- Every `unsafe fn` has a `/// # Safety` section; its body still uses explicit `unsafe {}`.
- `UnsafeCell` wraps one slot, never the whole buffer (Miri rejects the outer form — see
  `ring_spsc/src/lib.rs`).

## Atomics and concurrency

- Every ordering is a named constant whose doc says which load or store it pairs with and what
  breaks if it is weakened (`GATING`, `PUBLISH`, `OBSERVE`, `COMMIT`, `HANDOFF`, `OWN`). An
  inline ordering needs the same justification in a comment. No defaulted `SeqCst`.
- Ring atomics come from `ring_atomic` (`AtomicSeq`, `SeqCell`) — the one `cfg(loom)` seam. An
  atomic created elsewhere is invisible to loom.
- Cursors sit on their own cache line: `ring_align::CacheAligned`, `ring_cursor::PaddedCursor`.
- `Seq` arithmetic stays unfolded (`ring_seqno`); fold to a slot index only through `ring_index`
  (`seq & mask`).
- Waits are bounded (`ring_wait`). No locks on the write path; `ring_trace` and the
  `ring_bench` mutex baseline are the deliberate exceptions.
- A guard whose drop does the work is `#[must_use = "…"]` with the consequence in the message
  (`ring_claim::Claim`).

## Rustdoc

- Every `pub` item is documented. The first sentence is a noun phrase for types, constructors
  and getters, `How …` for counts, `Whether …` for predicates, the imperative for actions.
- Refer to items as intra-doc links: ``[`RingConfig::new`]``.
- `# Errors`, `# Panics` and `# Safety` sections where they apply.
- Examples follow the prose, without an `# Examples` heading.
- Crate-level docs link to the crate's `docs/` corpus rather than repeat it.

## Tests

- Integration tests only: `tests/<stem>_test.rs`; no `#[cfg(test)]` modules in `src/`. Each file
  opens with `//!` naming the crate; a file that claims a feature names it as
  `docs/feature/<id>_…`, which G3 greps for.
- Test names are behaviour sentences: `sequences_are_issued_consecutively_across_a_wrap`. A
  `///` line says what the test proves when the name cannot.
- Loom models sit in a `#[cfg(loom)]` module (usually `mod exhaustive`) and use
  `ring_testkit::{ leak, leak_ends }` for `'static` ends; the crate's other test files carry
  `#![cfg(not(loom))]`. Run them with `./verb/loom`.
- Every crate keeps `tests/manual/readme.md` for what automation cannot establish: stages with
  `**Expected:**` written before running, and a `## Run Record` with ISO dates (G4).

## Doc corpus

Each crate's `docs/` is graded by G14–G17, G20, G21 (`bench_harness/gate/corpus/`). With the
rulebooks absent, the gates,
[`corpus_standard.txt`](bench_harness/gate/declared/ring/corpus_standard.txt) and the fixtures
under `bench_harness/gate/declared/ring/corpus_control/` are the standard.

- Exactly 13 definition directories plus `definition/`, the Module Index. No others.
- Instances are `NNN_snake_case.md`, titled `# <Type>: <Title>`, opening with `### Scope`.
- Findings are `### XXn — Title` with the crate's two-letter prefix and appear in three places
  that must agree exactly.
- Recipes are ```` ```sh ```` blocks followed by `Live output:` that must match byte for byte
  (G15). They address content, never line numbers (G21).

Procedure and templates: [`.agents/skills/doc-corpus/SKILL.md`](.agents/skills/doc-corpus/SKILL.md).

## Comments

Say what the code cannot: why, the constraint, the consequence, a pointer to the doc instance
that holds the argument (`→ docs/invariant/002`). Do not narrate code. Gate scripts record
corrections as `# Fix(<slug>):` / `# Finding(<slug>):` blocks.

## Scripts

`verb/` scripts: `#!/usr/bin/env bash`, `#@ purpose:` and `#@ tags:` header lines (read by
`./verb/verbs`), `set -euo pipefail`, `cd` to the repo root, parameters as `key::val`, an
unknown parameter exits 2, `dry::1` prints the commands instead of running them. Gate scripts
use `set -uo pipefail` and end in exactly one `pass` or `fail` (see
[`bench_harness/gate/readme.md`](bench_harness/gate/readme.md)).
