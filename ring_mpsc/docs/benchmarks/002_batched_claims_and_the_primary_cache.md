# Batched claims and the primary-producer cache: one run on an i7-10750H (DRAFT)

The commit under test is `master` at `9598875`. Nothing under test changed between runs: `batch/mpsc/*`,
`mpsc_producers`, `mpsc_capacity` and `micro` (`mpsc` vs `mpsc-primary`) ran from one build, back to back.

**Status: draft.** Single process, one criterion run per cell (10 flat samples threaded, 100 micro) — no
multi-process ranges, so no verdict on bimodality. The `wait`-strategy half of this comparison lives in
`ring_wait/docs/benchmarks/001_wait_strategy_costs.md`.

## Machine

| | |
|---|---|
| Model | Intel i7-10750H, 6 cores / 12 threads |
| Memory | 16 GB |
| OS | Windows, threads unpinned |
| Toolchain | rustc 1.98.0, the workspace `bench` profile (fat LTO, one codegen unit) |

## Method

- Criterion through the `perf` suite. Every number is criterion's median for one process.
- `cargo bench -p perf --bench batch -- batch/mpsc` — one producer, 2²⁰ records, 1 / 8 / 32 / 128 records
  per push (`Mode::batch`), consumer takes everything available.
- `cargo bench -p perf --bench mpsc -- mpsc/push` — 2²⁰ records over 1, 2, 4, 8 and 11 producers at
  1024 slots (`mpsc_producers`), plus 4 producers at 64 and 16384 slots (`mpsc_capacity`),
  `push1_popN` against `push32_popN`.
- `cargo bench -p perf --bench micro -- mpsc` — matches `mpsc` and `mpsc-primary` only; single thread.

## Results

### `batch/mpsc`: records per push, one producer (throughput from 2²⁰ records / median time)

| Records per push | Median time | Throughput | vs 1 |
|---|---|---|---|
| 1 | 30.60 ms | 34.3 M/s | — |
| 8 | 5.30 ms | 197.7 M/s | **5.8x** |
| 32 | 4.04 ms | 259.9 M/s | **7.6x** |
| 128 | 3.98 ms | 263.2 M/s | 7.7x, plateau |

### `mpsc_producers`: push1 against push32 under contention (1024 slots)

| Producers | `push1_popN` | `push32_popN` | Ratio |
|---|---|---|---|
| 1 | 31.7 M/s | 250.3 M/s | 7.9x |
| 2 | 22.0 M/s | 237.8 M/s | 10.8x |
| 4 | 13.6 M/s | 242.6 M/s | 17.9x |
| 8 | 8.1 M/s | 233.7 M/s | 28.7x |
| 11 | 7.1 M/s | 216.1 M/s | 30.3x |

`push32` wall time is flat across producer counts (4.19–4.85 ms); `push1` degrades monotonically.
Spins per record explain it: `push1` empty-spins grow 0.044 → 0.284 → 0.741 → 1.595 → 1.916 (the
consumer starves while producers CAS-retry; at 11 producers 1.253 full-spins per record), while
`push32` empty-spins stay ≈ 0.001 throughout.

### `mpsc_capacity`: 4 producers, always-full vs never-full

| Capacity | `push1_popN` | `push32_popN` | Ratio |
|---|---|---|---|
| 64 | 13.6 M/s | 168.5 M/s | 12.4x |
| 16384 | 14.6 M/s | 258.6 M/s | 17.7x |

### `micro`: `mpsc` against `mpsc-primary`, one thread

| Cell | `mpsc` | `mpsc-primary` |
|---|---|---|
| `push_pop` | 13.73 ns | 12.91 ns (~6%: one cached `GATING` load saved) |
| `push_full` (refusal) | 3.84 ns | 3.16 ns (~18%: no consumer-line read on the refusal path) |
| `fill_drain` popN, 1024 slots | 98.7 M/s | 110.2 M/s (~12%) |
| `fill_drain` popN, 16384 slots | ~99 M/s | ~112 M/s (~12%) |
| `fill_drain` push32_popN, 1024 slots | 290.6 M/s | — (single-producer batching alone is ~3x even with no contention) |

## Reading (draft)

- Batching is the contention cure: under 11 producers it, not a different claim primitive, recovers 30x.
  The gain saturates at 32 records per push — the harness's `BATCH` default.
- The primary cache is a free, stable ~6–18% wherever one producer can hold the handle. Narrow single-thread
  cells; multi-process ranges still to be collected before this becomes a gate threshold.
- Neither result needs `fetch_add` at the claim: see `ring_claim`'s "Why the CAS loop is not `fetch_add`".
