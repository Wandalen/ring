# verb

do-protocol verb scripts for the `ring` repository. The repository is one Cargo
workspace, so every verb here reaches the whole family directly.

This is the repo root's `verb/`. Every crate but the `perf` benchmark suite has
its own `verb/` (`test`, `test_only`, `lint`, `build`) for iterating on one
crate without touching the others. Those are thin wrappers. Each one `exec`s
this directory's own `_crate_dispatch` with its crate name baked in, so the
logic (argument parsing, the cargo invocation shape) lives in one place.
`_crate_dispatch` is not itself a verb. It takes a verb name and a crate name
as its first two positional args and has no meaning invoked on its own, which
is why `verbs` (below) skips anything starting with `_`.

**Parameter convention.** Every parameter is `key::val` (`level::3`, `crate::ring_spsc`,
`dry::1`), never `--flag val`. Every verb rejects an unrecognized parameter loudly
(exit 2) rather than silently ignoring or mis-forwarding it. `dry::1` (where supported)
prints the command(s) the verb would run without running them.

| File | Responsibility |
|------|-----------------|
| `test` | Leveled full-suite verification, from `level::1` (nextest) through `level::5` (+doctests, clippy, udeps, audit, the family gate suite). Default `level::3`. Final verification only |
| `test_only` | Filtered nextest run by `filter::<substring>`, `crate::<name>`. Ordinary verification during development |
| `lint` | Clippy, warnings as errors. `crate::<name>` narrows to one package |
| `build` | Compile the workspace. `crate::<name>` narrows to one package |
| `fmt` | Apply this repo's adopted rustfmt style workspace-wide (`+nightly`, `rustfmt.toml`). `check::1` verifies without writing |
| `doc` | Rebuild rustdoc from a clean slate (`rm -rf target/doc` first, because incremental `cargo doc` hides errors in unchanged crates) |
| `gate` | Dispatch to `bench_harness/gate/run_all.sh`, the family's gate suite. `family::<name>` (default `ring`), `gate::<name>` (repeatable), `stage::<name>` |
| `bench` | Run the benchmarks — `suite::comparison` (default, `ring_bench`'s example, release), `micro` / `spsc` / `mpsc` / `batch` (`perf`, criterion), `latency`, `report` (markdown comparison of the last results) or `all`. `quick::1` validates every `perf` case without numbers; `filter::<text>`, `save::<name>`, `baseline::<name>`, `out::<file>` |
| `publish_check` | Dry-run `cargo publish` for every publishable crate, building each from its package. `crate::<name>` narrows |
| `clean` | Remove `target/` and gate scratch logs |
| `verify` | Full pre-push gate, an alias for `test level::5` |
| `verbs` | List all verbs with their purpose line |
| `package_info` | Family manifest info as flat JSON. No single crate here is "the" package (the members are peers), so this describes the family the workspace root declares |
| `_crate_dispatch` | Shared implementation behind every per-crate `verb/{test,test_only,lint,build}` wrapper. Not a verb. It takes a verb name and crate name as its first two args and is never invoked directly |

### Why `test` is leveled

The levels mirror the `will .test level::N` ladder this repo was asked to
follow, from nextest alone at level 1 up to nextest, doctests, clippy, udeps and
audit. That ladder's level 5 adds `will .test dry:0`, a self-check of the `will`
tool with no equivalent here, so this repo runs the family's gate suite in its
place (`level::5` runs `gate` internally).
