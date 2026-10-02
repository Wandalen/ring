# verb

do-protocol verb scripts for the `ring` repository, which is one Cargo workspace with
34 members (33 `ring_*` crates plus `bench_harness`). Unlike `codename_space_sandbox`'s
`verb/` (twelve independent workspaces, fanned out via `each_workspace`), every verb
here reaches the whole family directly. No fan-out step exists or is needed.

This is the repo root's `verb/`. Every one of the 34 crates also has its own `verb/`
(`test`, `test_only`, `lint`, `build`; see e.g. [`../ring_spsc/verb/readme.md`](../ring_spsc/verb/readme.md))
for iterating on one crate without touching the other 33. Those are thin wrappers.
Every crate's `verb/test` is a 4-line file that `exec`s this directory's own
`_crate_dispatch` with its own crate name baked in. The logic (argument parsing, the
cargo invocation shape) lives in exactly one place, however many crates call into it.
`_crate_dispatch` is not itself a verb. It takes a verb name and a crate name as its
first two positional args and has no meaning invoked on its own, which is why `verbs`
(below) skips anything starting with `_`.

**Parameter convention.** Every parameter is `key::val` (`level::3`, `crate::ring_spsc`,
`dry::1`), never `--flag val`. Every verb rejects an unrecognized parameter loudly
(exit 2) rather than silently ignoring or mis-forwarding it. `dry::1` (where supported)
prints the command(s) the verb would run without running them.

No `verb.rulebook.md` exists in this repo to govern these, same as `codename_space_sandbox`.

| File | Responsibility |
|------|-----------------|
| `test` | Leveled full-suite verification, from `level::1` (nextest) through `level::5` (+doctests, clippy, udeps, audit, the family gate suite). Default `level::3`. Final verification only |
| `test_only` | Filtered nextest run by `filter::<substring>`, `crate::<name>`. Ordinary verification during development |
| `lint` | Clippy, warnings as errors. `crate::<name>` narrows to one package |
| `build` | Compile the workspace. `crate::<name>` narrows to one package |
| `fmt` | Apply this repo's adopted rustfmt style workspace-wide (`+nightly`, `rustfmt.toml`). `check::1` verifies without writing |
| `doc` | Rebuild rustdoc from a clean slate (`rm -rf target/doc` first, because incremental `cargo doc` hides errors in unchanged crates) |
| `gate` | Dispatch to `bench_harness/gate/run_all.sh`, the family's own G1-G21 corpus/quality suite. `family::<name>` (default `ring`), `gate::<name>` (repeatable), `stage::<name>` |
| `bench` | Run `ring_bench`'s comparison example, the family's benchmark |
| `publish_check` | Dry-run `cargo publish` for every publishable crate, building each from its package. `crate::<name>` narrows |
| `clean` | Remove `target/` and gate scratch logs |
| `verify` | Full pre-push gate, an alias for `test level::5` |
| `verbs` | List all verbs with their purpose line |
| `package_info` | Family manifest info as flat JSON. No single crate here is "the" package (34 peer members), so this describes the family the workspace root declares |
| `_crate_dispatch` | Shared implementation behind every per-crate `verb/{test,test_only,lint,build}` wrapper. Not a verb. It takes a verb name and crate name as its first two args and is never invoked directly |

### Why `test` is leveled and the monorepo's `test` is not

`codename_space_sandbox/verb/test` always runs the same three-step suite (nextest,
doctests, `cargo doc`), with no levels. This repo's `test` instead mirrors the separate
`will .test level::N` ladder from `CLAUDE.md` (levels 1-5, escalating from nextest
alone up to nextest+doctests+clippy+udeps+audit), because that is the convention this
repo was asked to follow. Level 5 in the original ladder inserts `will .test dry:0`,
a self-check specific to the `will` tool. That tool is absent here, so this family's
own maximal check replaces it. That check is the full gate suite (`level::5` runs
`gate` internally).
