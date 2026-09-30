---
name: bench
description: Run ring_bench's write-path comparison and record numbers that can be compared later — machine, commit, features, workload.
disable-model-invocation: true
---

# Bench

`ring_bench` is a custom harness, not criterion: `Comparison::run(workload)` drives one
`Workload` through every candidate (mutex queue, contract ring, TLS over ring, direct SPSC,
direct MPSC, and crossbeam with `--features crossbeam`) and times it with `std::time::Instant`.
A candidate that loses records is not eligible to be fastest.

1. Quiet machine. Record: `lscpu | grep 'Model name'`, `nproc`, the CPU governor
   (`cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor`), `rustc -V`, `git rev-parse --short HEAD`.
2. `./verb/bench` runs the comparison example (sections A–D: roomy, cramped, four producers,
   ten repeats) in the dev profile — fine for correctness columns. For timings, build release:
   `cargo run --release -p ring_bench --all-features --example comparison`.
3. Run each side at least three times; report the median and the spread, not the best run.
4. Comparing two commits: same machine, same session, alternate them (A B A B), not A A A B B B.
5. Record in the PR: the machine line, commit, command, and the table. A result that changes
   what the family adopts belongs in `ring_bench/docs/` as a measured instance with a recipe
   (`doc-corpus` skill) — "measured before adopted"
   (`ring_mpsc/docs/non_functional_requirement/001_measured_before_adopted.md`).

Numbers without the machine line and commit are not comparable — do not record them.
Continuous reporting and a regression gate are capstone topic 7 (`docs/capstone/07_benchmarking.md`).
