# How to run the benchmarks

How to run the repository's benchmark suite (`perf`) and read its results. Commands work in Git
Bash and PowerShell alike unless noted. What each suite measures, in detail:
[`../perf/readme.md`](../perf/readme.md).

## What is compared

The suite races six queue implementations — the candidates — on the same track: every candidate
gets the same capacity and the same record type. Most suites measure time per operation or
records per second through criterion; the `latency` suite reports arrival-delay percentiles from
its own driver.

| Candidate | Queue |
|-----------|-------|
| `spsc` | `ring_spsc` — this repository's single-producer, single-consumer (SPSC) ring |
| `mpsc` | `ring_mpsc` — this repository's multi-producer, single-consumer (MPSC) ring |
| `rtrb` | `rtrb` 0.4 — an established SPSC ring, the reference point |
| `sync_channel` | `std::sync::mpsc::sync_channel` — the standard library's bounded channel |
| `arrayqueue` | `crossbeam_queue::ArrayQueue` — a bounded multi-producer, multi-consumer (MPMC) array queue |
| `mutex` | `Mutex<VecDeque<T>>` — the naive baseline |

## Step 1 — Get the code

The suite lives in `perf/` at the repository root; `master` carries it. Requires a Rust
toolchain, and on Windows Git Bash for the `verb` wrapper — the `cargo` commands run in any
terminal.

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

On Windows, `spsc_pinned: no CPU topology to read, skipped` is expected — see Windows notes.

## Step 3 — Measure

| Suite | Measures |
|-------|----------|
| `micro` | one thread alone: pushing a record and popping it back, pushing into a full queue and being refused, popping from an empty queue, and filling the queue to capacity before draining it — each reported as time per operation |
| `spsc` | one producer thread and one consumer thread: across queue sizes, across record sizes, and with the two threads placed on one shared core or on separate cores |
| `mpsc` | several producer threads against one consumer: how the queue holds up as the producer count grows, as the capacity changes, and when there are more producers than logical CPUs |
| `batch` | one producer moving several records in a single operation — 1, 8, 32 or 128 at a time |
| `latency` | how late records arrive relative to their scheduled send time, as percentiles; prints the per-clock-read cost first, since every number carries up to one |
| `comparison` | `ring_bench`'s comparison example — the family's own harness |

One suite at a time, Git Bash:

```sh
./verb/bench suite::micro     # or spsc | mpsc | batch | latency | comparison
```

PowerShell:

```powershell
cargo bench -p perf --bench micro   # or spsc | mpsc | batch | latency
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
cargo run -q -p perf --example report -- --out perf_report.md
```

Without `--out` it prints to stdout. A trailing word filters groups by substring: `-- spsc`
keeps `spsc`, `spsc_payload`, `spsc_pinned`. The report's first line identifies the machine,
rustc version and commit — compare only runs whose machine line matches.

## Step 5 — Baselines (before/after)

```sh
cargo bench -p perf --bench spsc -- --save-baseline before   # on the "before" commit
# change the code
cargo bench -p perf --bench spsc -- --baseline before        # on the "after" commit
cargo run -q -p perf --example report -- --baseline before --out perf_report.md
```

The report then shows each cell's change against the baseline, positive when faster. Criterion
saves baselines per benchmark — compare each suite against its own baseline, on the same machine.

Git Bash shorthand: `./verb/bench suite::spsc save::before`, then on the other commit
`./verb/bench suite::spsc baseline::before` and `./verb/bench suite::report baseline::before out::perf_report.md`.

## Windows notes

PowerShell and cmd cannot execute the `verb/bench` bash script: Windows resolves an
extensionless file through its default file association, which usually opens a text editor.
Either run it in Git Bash, call it via `bash ./verb/bench ...`, or use the `cargo` commands
above directly.

- `spsc_pinned` skips itself on Windows. It reads Linux sysfs to find two hardware threads
  sharing one physical core (SMT, simultaneous multithreading) and pairs on separate cores;
  that file layout does not exist here, so it prints `spsc_pinned: no CPU topology to read,
  skipped` and stops. All other groups run.
- The report's machine line degrades on Windows: the processor model falls back to the
  architecture, governor and clocksource read as `unknown`. The numbers are unaffected.

## Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| `error: package ID specification 'perf' did not match any packages` | the branch lacks the suite | switch to a branch with `perf/` (Step 1) |
| `./verb/bench` opens a text editor | PowerShell cannot run bash scripts | Git Bash, `bash ./verb/bench ...`, or the cargo commands |
| `verb/bench: unrecognized parameter ...` | a typo in an option | options: `suite::`, `quick::1`, `filter::`, `save::`, `baseline::`, `out::`, `dry::1` |
| `verb/bench: suite::X is not one of ...` | wrong suite name | one of `comparison`, `micro`, `spsc`, `mpsc`, `batch`, `latency`, `report`, `all` |
| `No criterion results under target/criterion` | the report ran before any benchmark | run Step 3 first |
| the first build takes very long | fat LTO and one codegen unit in `[profile.bench]` | expected; rebuilds are faster |
| a test panics, naming a candidate | the harness detected a lost, duplicated, reordered or torn record | that is a finding — no number is printed for that run |
| `spsc_pinned: no CPU topology to read, skipped` | no Linux sysfs topology on Windows | expected, nothing to do |

## Fair measurements

- Close background programs; on Windows pick the best-performance power plan and run plugged in.
- Compare only same-machine numbers, through baselines rather than memory.
- The harness controls for the rest: a fresh queue per run, checksum and per-producer order on
  every record, the first 200 ms of a latency run excluded, identical capacity and record type
  for every candidate.
