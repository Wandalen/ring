# Guide: Running the Verdicts Yourself

### Scope

- **Purpose**: Give every command that reproduces this crate's verdicts, in the order that builds understanding, with the output to expect from each.
- **Responsibility**: Make every claim in [`002`](002_the_four_verdicts.md) checkable by the reader rather than trusted.
- **In Scope**: The seven gates; the full suite; the comparison demo; the two ways to read coverage; the manual run records.
- **Out of Scope**: What the numbers mean (→ [`002`](002_the_four_verdicts.md)); what they fail to prove (→ [`003`](003_what_the_gates_do_not_prove.md)); the stage decomposition that produced them (the family's own implementation plan).

Every command below is run from the repository root:

```bash
cd "$(git rev-parse --show-toplevel)"
```

### 1. The Seven Gates — the machine verdict

This is the whole family in one command. It grades all 33 crates.

```bash
bash bench_harness/gate/run_all.sh --family ring
```

Expect `7/7 gates reached`. `--family ring` is required rather than optional: a
bare `run_all.sh` grades whichever family `declared/family.txt` names, and that
is now `orbital`. Omitting it silently grades a different family and reports
`11/11`, which looks like a better answer to the question you asked.

What each gate asks:

| Gate | Asks | Reached when |
|---|---|---|
| G1 | Is every line covered? | tarpaulin reports `n/n` for every file under `<crate>/src/` |
| G2 | Does every crate export and document something real? | ≥1 `pub` item, zero `missing_docs`, no prose still calling itself a skeleton |
| G3 | Is every feature claimed? | each of the 22 features cited by name in some crate's `tests/` |
| G4 | Was it run by hand? | every crate has `tests/manual/readme.md` with a `## Run Record` and an ISO date |
| G5 | Did the export surface leak? | no crate depends on anything outside the five Contract crates |
| G6 | Is `unsafe` confined? | only `ring_spsc`, `ring_mpsc`, `ring_core` contain it |
| G12 | Do the recorded defects still turn the suite red? | each mutation in `declared/ring/mutant/` reinstated, its crate's suite red, its file restored byte-identically |

**The gates run against an empty workspace too, and that is the point.** They
were written before the crates they grade and reported `0/6` first
(→ [`invariant/001`](../invariant/001_gate_non_vacuity.md)). A gate that has never
failed is not evidence of anything.

To grade one stage instead of all 33 crates:

```bash
bash bench_harness/gate/run_all.sh --stage S8
```

The stage names come from [`gate/declared/ring/stages.txt`](../../gate/declared/ring/stages.txt).

**Do not call a gate script with positional arguments.** They take none — scope
arrives through the `GATE_CRATES` / `GATE_FEATURES` / `GATE_STAGE` environment
variables that `run_all.sh` sets. Writing `bash gate/g1_coverage.sh ring_bench S8`
does not fail; the arguments are ignored and the script silently grades the
entire family. That mistake was made during this effort and produced a
correct-looking 1650-line result for what was supposed to be a 298-line check.

### 2. The Full Suite

```bash
verb/test
```

Expect `1265 tests run, 1265 passed, 0 failed`, plus doctests and a clean
clippy pass. Without `verb`, the equivalent is `will .test level::3`, and
without that:

```bash
RUSTFLAGS="-D warnings" cargo nextest run --all-features \
  && RUSTDOCFLAGS="-D warnings" cargo test --doc --all-features \
  && cargo clippy --all-targets --all-features -- -D warnings
```

This takes minutes across 33 crates. Launch it detached rather than watching a
tool call time out.

### 3. The Comparison — what the effort was for

Gates prove the family is built. This prints what it measured.

```bash
cargo run -p ring_bench --all-features --example comparison
```

Four sections. Real output from a run on 2026-08-28:

**A — 4096 slots, 256 records.** Everything fits, so everything is eligible.

```
candidate          offered  reported  received  dropped   silent     write ns
mutex_queue            256       256       256        0        0       499485
contract_ring          256       256       256        0        0        36481
tls_over_ring          256       256       256        0        0        45520
direct_spsc            256       256       256        0        0        32280
direct_mpsc            256       256       256        0        0       299283
off_the_shelf          256       256       256        0        0        46241
fastest lossless: direct_spsc
```

The mutex baseline is **15.5x** `direct_spsc` here (499485 / 32280). That
order-of-magnitude gap is the one durable timing result; see section D for why
nothing finer than it is asserted anywhere.

**B — 256 records into 16 slots.** Everything loses, in three different ways.

```
mutex_queue            256        16        16      240        0       205483
contract_ring          256       256        16      240      240        27040
tls_over_ring          256         0         0      256        0         9640
direct_spsc            256        16        16      240        0        20480
direct_mpsc            256        16        16      240        0       232643
off_the_shelf          256       256        16      240      240        47841
fastest lossless: none — every candidate dropped records
```

Read the `silent` column. `contract_ring` and `off_the_shelf` **report 256
successes and keep 16**. Their write API returned `Ok` 240 times for records
that no longer exist. `tls_over_ring` is worse in a different direction: it
keeps *nothing*, because a rejected first flush leaves records staged and every
later append fails against the full stage. And it is the fastest column entry
on the page, at 9640 ns — which is exactly why `fastest lossless` filters on
losslessness before it compares times.

**C — four producers.** Four of six candidates cannot be reached at all.

```
mutex_queue           1024      1024      1024        0        0       773168
direct_mpsc           1024      1024      1024        0        0       858449
refused: contract_ring admits 1 producer(s), asked for 4
refused: tls_over_ring admits 1 producer(s), asked for 4
refused: direct_spsc admits 1 producer(s), asked for 4
refused: off_the_shelf admits 1 producer(s), asked for 4
```

Two of those four refusals are the *door*, not the structure — `contract_ring`
and `off_the_shelf` sit on genuinely multi-producer queues.

**D — the same workload, ten times.**

```
distinct winners across 10 identical runs : 2 ["direct_spsc", "off_the_shelf"]
contract_ring's own spread                : 34321 – 72881 ns
ratio                                     : 2.1x
```

**Run it twice.** The immediately preceding run of the same command on the same
machine reported **3 distinct winners** and a spread of **33760 – 188602 ns**,
a ratio of **5.6x**. The stability reading is not itself stable, which is a
stronger argument for [`ring_bench/docs/decisions/002`](../../../ring_bench/docs/decisions/002_no_test_asserts_an_ordering.md)
than any single run of it.

Drop `--all-features` to see the comparison with five rows instead of six —
`crossbeam` is opt-in, so `off_the_shelf` is compiled out entirely:

```bash
cargo run -p ring_bench --example comparison
```

### 4. Coverage, Two Ways

The gate's answer:

```bash
bash bench_harness/gate/run_all.sh --stage S8
```

The raw numbers behind it:

```bash
cargo tarpaulin --all-features --skip-clean --out Stdout 2>/dev/null \
  | grep '^|| ring_bench/src/'
```

**G1 recomputes the percentage rather than reading tarpaulin's.** Tarpaulin's
summary figure is workspace-wide, so it is meaningless for a scoped stage; the
gate sums the per-file breakdown for the crates in scope. Note also what is
*not* in scope: `examples/` is not under `src/`, so the demo you just ran
contributes nothing to the coverage figure.

### 5. The Manual Records

Every crate carries one. `ring_bench`'s is the longest and the most useful:

```bash
sed -n '1,80p' ring_bench/tests/manual/readme.md
```

Five prediction-first stages, of which **three predictions were wrong**. Stage
B5 is the one to read — it is the subject of [`003`](003_what_the_gates_do_not_prove.md).

### Sources

| File | Relationship |
|------|--------------|
| [`gate/run_all.sh`](../../gate/run_all.sh) | The command in §1; sets the env vars the gate scripts read |
| [`gate/declared/ring/stages.txt`](../../gate/declared/ring/stages.txt) | The stage names `--stage` accepts |
| [`ring_bench/examples/comparison.rs`](../../../ring_bench/examples/comparison.rs) | The demo in §3 |
| [`ring_bench/tests/manual/readme.md`](../../../ring_bench/tests/manual/readme.md) | The records in §5 |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_gate_non_vacuity.md](../invariant/001_gate_non_vacuity.md) | Why §1's gates reported 0/6 before they reported 6/6 — and how G12, which cannot be baselined that way, was proved able to fail instead |

### Acceptance

| File | Relationship |
|------|--------------|
| [../acceptance/001_feature_reached_tests.md](../acceptance/001_feature_reached_tests.md) | The 22 reached-tests G3 checks for in §1 |
