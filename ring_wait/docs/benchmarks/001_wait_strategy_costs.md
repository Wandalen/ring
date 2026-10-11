# Wait-strategy costs: one run on an i7-10750H (DRAFT)

The commit under test is `master` at `9598875` plus the `wait`-suite wiring (`perf/Cargo.toml`,
`perf/benches/wait.rs`, `perf/readme.md`, `verb/bench`). The `claim`-batching half of this comparison
lives in `ring_mpsc/docs/benchmarks/002_batched_claims_and_the_primary_cache.md`.

**Status: draft.** Single process, one criterion run per cell (100 samples) — multi-process ranges still
to be collected before any of this becomes a gate threshold.

## Machine

| | |
|---|---|
| Model | Intel i7-10750H, 6 cores / 12 threads |
| Memory | 16 GB |
| OS | Windows, threads unpinned |
| Toolchain | rustc 1.98.0, the workspace `bench` profile (fat LTO, one codegen unit) |

## Method

- Criterion through the `perf` suite (`cargo bench -p perf --bench wait`). Every number is criterion's
  median for one process.
- `wait_ready`: `wait_until` against an immediately-ready predicate — one look plus call overhead.
- `wait_exhausted`: the full spins budget (spin 256, yield 32, park 4, none reports its single look)
  against a never-ready predicate.
- `wait_space`: `for_space` over a real `CursorPair` — room on an empty ring (`ready`), then none on a
  full one (`full`, 64-spin budget except park's 4).
- `wait_data`: `for_data` — three records pending (`ready`), then none (`empty`).

## Results

| Cell | spin | yield | park | none |
|---|---|---|---|---|
| `ready` (one look) | 1.48 ns | 1.48 ns | 1.49 ns | 1.49 ns |
| `exhausted` (full budget) | 40.23 µs | 2.24 µs | 2.30 ms | 1.38 ns |
| `space/ready` | 5.43 ns | 5.37 ns | 5.39 ns | 5.18 ns |
| `space/full` | 10.09 µs | 1.17 µs | 2.27 ms | 4.97 ns |
| `data/ready` | 1.81 ns | 1.57 ns | 1.96 ns | 1.61 ns |
| `data/empty` | 9.91 µs | 1.11 µs | 2.30 ms | 1.34 ns |

## Reading (draft)

- A ready ring costs the same (~5.4 ns) under every strategy: one headroom check. Strategy only matters
  when there is nothing to take.
- On a full ring, `spin` burns 10 µs hammering the consumer's cursor line — coherence traffic against the
  very thread that would free the space. `yield` is ~9x cheaper (1.17 µs); `none` returns in 5 ns and
  leaves backoff to the caller.
- The oversubscribed pathology this predicts is visible in `mpsc_oversubscribed/mpsc/push1_popN`:
  48.6 full-spins per record with an 8.6 ms max gap between receives. Spinning producers starve the
  consumer. Prefer `yield`/`none` with `for_space` once producer count approaches core count.
