# How to run the benchmarks

How to run the repository's benchmark suite (`perf`) and read its results. What the suite
compares, which suite measures what, and the exact benchmark ids are documented in
[`../perf/readme.md`](../perf/readme.md) — this guide adds the workflow around it: validation,
the run order, where results land, baselines, and the platform traps. Requires a Rust toolchain.
Linux runs everything as-is; Windows and macOS have notes below.

## Step 1 — Get the code

The suite lives in `perf/` at the repository root; `master` carries it.

```sh
git checkout master
```

## Step 2 — Validate before measuring

Run every case once, without measurements.

Git Bash:

```sh
./verb/bench suite::all quick::1
```

PowerShell, equivalent:

```powershell
cargo bench -p perf --bench micro   -- --test
cargo bench -p perf --bench spsc    -- --test
cargo bench -p perf --bench mpsc    -- --test
cargo bench -p perf --bench batch   -- --test
cargo bench -p perf --bench latency -- --test
cargo run --release -p ring_bench --all-features --example comparison
```

The first build is slow: the bench profile uses fat LTO (link-time optimization) and one
codegen unit so the family's helpers inline across crates the way the comparison crates' do,
and that costs compile time. Rebuilds are faster.

On Windows and macOS, `spsc_pinned: no CPU topology to read, skipped` is expected — see
platform notes.

## Step 3 — Measure

Six suites: `micro` (one thread alone), `spsc` (one producer, one consumer), `mpsc` (producers
against one consumer), `batch` (several records per operation), `latency` (arrival-delay
percentiles), `comparison` (the family's own example). Modes, parameters and record types per
suite: [`../perf/readme.md`](../perf/readme.md).

One suite at a time:

```sh
./verb/bench suite::spsc     # or micro | mpsc | batch | latency | comparison
```

PowerShell:

```powershell
cargo bench -p perf --bench spsc    # or micro | mpsc | batch | latency
cargo run --release -p ring_bench --all-features --example comparison
```

Everything in one go, ending with the comparison tables:

```sh
./verb/bench suite::all
```

## Step 4 — Read the results

Results land under the target directory:

- `target/criterion/` — criterion's estimates and HTML charts (`target/criterion/report/index.html`)
- `target/perf/` — what criterion has no column for: spin counts, gaps, latency percentiles

The report pivots the last results into one markdown page — candidates and modes as rows,
parameters as columns, the best of each column in bold:

```sh
cargo run -q -p perf --example report -- --out target/perf_report.md
```

Without `--out` it prints to stdout. A trailing word filters groups by substring: `-- spsc`
keeps `spsc`, `spsc_payload`, `spsc_pinned`. Write the report into `target/`: an untracked file
at the repository root would put "with uncommitted changes" into every later report's machine
line.

The machine line names the CPU, governor, clocksource, rustc version and commit — on Linux.
Elsewhere it falls back to the architecture and thread count, which does not distinguish chip
generations. Compare runs from the same machine, and read cross-machine tables with care.

## Step 5 — Baselines (before/after)

Baselines exist for the criterion suites (`micro`, `spsc`, `mpsc`, `batch`). The `latency`
driver ignores the flags — it takes only `--test` and a name filter — and the `comparison`
example prints to stdout only, so neither saves or reads a baseline.

```sh
cargo bench -p perf --bench spsc -- --save-baseline before   # on the "before" commit
# change the code
cargo bench -p perf --bench spsc -- --baseline before        # on the "after" commit
cargo run -q -p perf --example report -- --baseline before --out target/perf_report.md
```

The report then shows each cell's change against the baseline, positive when faster. Criterion
saves baselines per benchmark — compare each suite against its own baseline, on the same
machine. Git Bash shorthand: `./verb/bench suite::spsc save::before`, then on the other commit
`./verb/bench suite::spsc baseline::before` and `./verb/bench suite::report baseline::before
out::target/perf_report.md`.

A run with `--baseline` stops when a suite it covers has no baseline of that name — criterion
panics with `Baseline 'before' must exist before comparison is allowed`. Save the baseline for
every suite you plan to compare, or compare suite by suite.

**Measure the noise floor first.** Before trusting any before/after delta, run an unchanged
commit against its own baseline: same suite, same machine, back to back. Criterion will still
flag some benchmarks as improved or regressed — those deltas are the machine's noise, not
signal, and only deltas outside that spread mean something. The floor is wide in practice:
every thread runs unpinned (only `spsc_pinned` pins), and laptops mix performance and
efficiency cores; on such machines the spread reaches tens of percent in both directions.

## Platform notes

### Windows

- PowerShell and cmd cannot execute the `verb/bench` bash script: Windows resolves an
  extensionless file through its default file association, which usually opens a text editor.
  Run it from a Git Bash terminal, or from PowerShell through Git for Windows' own bash:
  `& "C:\Program Files\Git\bin\bash.exe" ./verb/bench suite::spsc`. Plain `bash` from
  PowerShell is usually WSL's launcher — the script then runs against Linux cargo, not this
  toolchain.
- `spsc_pinned` skips itself: the topology file it reads exists only on Linux. All other
  groups run.
- The machine line shows the architecture instead of the processor model; governor and
  clocksource read as `unknown`. The numbers are unaffected.

### macOS

- The stock `/bin/bash` is 3.2, where `set -u` rejects an empty-array expansion; `verb/bench`
  guards its arrays and runs on it. A Homebrew bash works too.
- `spsc_pinned` skips with the same message as on Windows: macOS exposes no core-topology file
  to this harness, and threads cannot be pinned through it.
- The machine line reads `aarch64, … unknown` and does not name the chip: an M1 and an M4 with
  the same thread count produce the same line. A matching machine line is not matching
  silicon.
- `Instant` can tick in coarse steps on Apple silicon — tens of nanoseconds. The reported
  clock-read cost is not the tick size; latency p50s in the tens of nanoseconds sit within one
  or two ticks of each other. Read them as approximate.

### Linux

The reference platform: core topology resolves, `spsc_pinned` runs, and the machine line names
the processor, governor and clocksource.

## Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| `error: package ID specification 'perf' did not match any packages` | the branch lacks the suite | switch to a branch with `perf/` (Step 1) |
| `./verb/bench` opens a text editor | PowerShell cannot run bash scripts | Git Bash, or Git Bash's bash by full path (Windows notes) |
| the run stops at `Baseline 'X' must exist before comparison is allowed` | a covered suite has no baseline of that name | save it for every suite you compare (Step 5), or compare suite by suite |
| `verb/bench: unrecognized parameter ...` | a typo in an option | options: `suite::`, `quick::1`, `filter::`, `save::`, `baseline::`, `out::`, `dry::1` |
| `verb/bench: suite::X is not one of ...` | wrong suite name | one of `comparison`, `micro`, `spsc`, `mpsc`, `batch`, `latency`, `report`, `all` |
| `No criterion results under target/criterion` | the report ran before any benchmark | run Step 3 first |
| the first build takes very long | fat LTO and one codegen unit in `[profile.bench]` | expected; rebuilds are faster |
| a test panics, naming a candidate | the harness detected a lost, duplicated, reordered or torn record | that is a finding — no number is printed for that run |
| `spsc_pinned: no CPU topology to read, skipped` | no core-topology file on Windows or macOS | expected, nothing to do |

## Fair measurements

- Establish the noise floor first (Step 5): an unchanged commit against its own baseline draws
  the spread that counts as noise.
- Close background programs; pick the best-performance power plan where the OS offers one.
- Compare only same-machine numbers, through baselines rather than memory; off Linux, a
  matching machine line means the same architecture and thread count, not the same silicon.
- The harness controls for the rest: a fresh queue per run, checksum and per-producer order on
  every record, the first 200 ms of a latency run excluded, identical capacity and record type
  for every candidate.
