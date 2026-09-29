# verb

do-protocol verb scripts for the `ring` repository — one Cargo workspace, 34 members
(33 `ring_*` crates plus `bench_harness`). Unlike `codename_space_sandbox`'s `verb/`
(twelve independent workspaces, fanned out via `each_workspace`), every verb here
reaches the whole family directly — no fan-out step exists or is needed.

This is the repo root's `verb/`, and the only one in the tree — no per-crate `verb/`
exists yet (34 crates × N verbs is real duplication weight; deferred until the
shared-implementation question is settled). Every invocation below is root-level.

**Parameter convention:** every parameter is `key::val` (`level::3`, `crate::ring_spsc`,
`dry::1`) — never `--flag val`. Every verb rejects an unrecognized parameter loudly
(exit 2) rather than silently ignoring or mis-forwarding it. `dry::1` (where supported)
prints the command(s) the verb would run without running them.

No `verb.rulebook.md` exists in this repo to govern these, same as `codename_space_sandbox`.

| File | Responsibility |
|------|-----------------|
| `test` | Leveled full-suite verification — `level::1` (nextest) through `level::5` (+doctests, clippy, udeps, audit, the family gate suite). Default `level::3`. Final verification only |
| `test_only` | Filtered nextest run — `filter::<substring>`, `crate::<name>`. Ordinary verification during development |
| `lint` | Clippy, warnings as errors. `crate::<name>` narrows to one package |
| `build` | Compile the workspace. `crate::<name>` narrows to one package |
| `doc` | Rebuild rustdoc from a clean slate (`rm -rf target/doc` first — incremental `cargo doc` hides errors in unchanged crates) |
| `gate` | Dispatch to `bench_harness/gate/run_all.sh` — the family's own G1-G21 corpus/quality suite. `family::<name>` (default `ring`), `gate::<name>` (repeatable), `stage::<name>` |
| `bench` | Run `ring_bench`'s comparison example — the family's benchmark harness |
| `clean` | Remove `target/` and gate scratch logs |
| `verify` | Full pre-push gate — alias for `test level::5` |
| `verbs` | List all verbs with their purpose line |
| `package_info` | Family manifest info as flat JSON — no single crate here is "the" package (34 peer members), so this describes the family the workspace root declares |

### Why `test` is leveled and the monorepo's `test` is not

`codename_space_sandbox/verb/test` always runs the same three-step suite (nextest,
doctests, `cargo doc`) — no levels. This repo's `test` instead mirrors the separate
`will .test level::N` ladder from `CLAUDE.md` (levels 1-5, escalating from nextest
alone up to nextest+doctests+clippy+udeps+audit), because that is the convention this
repo was asked to follow. Level 5 in the original ladder inserts `will .test dry:0` —
a self-check specific to the `will` tool, absent here — substituted with this family's
own maximal check: the full gate suite (`level::5` runs `gate` internally).
