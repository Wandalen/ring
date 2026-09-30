---
name: new-crate
description: Add a new ring_* crate to the family with everything the workspace, verbs and bench_harness gates expect.
disable-model-invocation: true
argument-hint: "<ring_name> <one-line purpose>"
---

# New crate

A new crate is a design decision first: which mechanism does it own that no crate owns now,
and why can it not live in an existing one? Answer that in the PR before step 1.

## Code

1. `ring_<name>/Cargo.toml` — copy `ring_config/Cargo.toml`: `version = "0.1.0"`,
   `edition`/`license`/`repository` `.workspace = true`, `readme = "readme.md"`, the
   one-line `description`, `publish`, sibling deps as `{ path = "../x", version = "0.1.0" }`,
   and `[lints] workspace = true` (G6).
2. Root `Cargo.toml` `members`: insert below every crate it depends on (dependency order, not
   alphabetical). Update the "33-crate forest" comment.
3. `src/lib.rs`: `//!` summary line (same sentence as `description`), rationale, then
   `#![deny(missing_docs)]`. At least one line starting with `pub ` (G2). No "skeleton"
   wording anywhere (G2). No `unsafe` — the allowlist is a separate decision.
4. `tests/<stem>_test.rs` opening with `//!` naming the crate and its `docs/feature/<id>_…`;
   100% line coverage of `src/` (G1).
5. `tests/manual/readme.md` with stages and a dated `## Run Record` (G4).

## Tooling

6. `ring_<name>/verb/{build,lint,test,test_only}` — copy from `ring_config/verb/`, replace the
   crate name; `chmod +x`. Copy `verb/readme.md` too.
7. `readme.md` in the short shape of `ring_config/readme.md`.

## Gates

8. `bench_harness/gate/declared/ring/crates.txt` — add the name (G18). Before any mutation
   survey.
9. If it implements a declared feature: `features.txt`, a `stages.txt` entry, and a test citing
   the feature (G3).
10. Outside consumers may depend on it? Only with a ruling, via `export_surface.txt` (G5).
11. `docs/` corpus: 13 definitions + `definition/`, ≥ 26 instances, ≥ 52 findings, recipes with
    live output — follow the `doc-corpus` skill. This is the bulk of the work.
12. After the code is final: `bash bench_harness/gate/mutant_survey.sh ring_<name>` until it
    exits 0 (G13; slow).

## Family docs

13. Root `README.md`: crates table row, tier graph, the crate-count badge and prose.
    `ARCHITECTURE.md` code map row. `verb/readme.md` member count.

## Check

`./verb/test level::3`, then `./verb/gate stage::<S>` for the stage it joined, then the full
`./verb/test level::5` in the background. Compare against `master` (`verify` skill).
