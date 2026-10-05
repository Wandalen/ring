# Each end keeps copies of both cursors: before and after, on an Apple M4 Pro

Measured 2026-10-04 for two commits on `perf/spsc-end-local-state`:

- `6ff64eb` `perf(ring_spsc): an empty batch commits nothing`
- `e42ac0a` `perf(ring_spsc): keep each end's position and its last reading of the peer in the end`

**Before** is `4cbd73c`. That commit adds tests only, so its library source is master's (`c65f4e4`). **After** is
`e42ac0a`. Both sides use 64-byte `ring_align::CACHE_LINE` padding.

## Machine

| | |
|---|---|
| Model | Apple M4 Pro (`Mac16,7`), 14 cores: 10 performance, 4 efficiency |
| Memory | 24 GB |
| Caches | 128-byte lines. Performance cores: 128 KB L1D each, 16 MB L2 per cluster. Efficiency cores: 64 KB L1D each, 4 MB L2 |
| OS | macOS 27.0.1, read on 2026-10-05 after a reboot. These runs predate the reboot, on the same machine |
| Toolchain | rustc 1.97.1 stable, the workspace `bench` profile (fat LTO, one codegen unit) |
| Thread placement | Not pinned: the harness pins only on Linux. Either thread can land on an efficiency core |
| Background | Desktop session, 1-minute load average 2.7–4.1 at the start of each run |

## Method

- Criterion through the `perf` suite. Every number is criterion's median (the middle estimate). Each run is a fresh
  process.
- Two full runs per side:

  ```sh
  cargo bench -p perf --bench spsc -- '^spsc/(spsc|rtrb)/'
  cargo bench -p perf --bench micro -- '/(spsc|rtrb)'
  ```

- Six more single-cell runs of `spsc/spsc/push1_pop1/1024` on the after side, because that cell is bistable.
- A per-commit split of `push_pop` and `pop_empty`, built in a separate worktree. Absolute values there sit about
  0.1–0.3 ns off the main runs.
- `e` is the harness's count of empty consumer polls per record.

## Results

### Two threads, one producer and one consumer (M records/s)

| Cell | Before, run 1 / 2 | After, run 1 / 2 | `rtrb`, same session |
|---|---|---|---|
| `push1_pop1`, 64 slots | 26 (e 0.49) / 25 (e 0.47) | **87** (e 0.14) / **90** (e 0.14) | 256–295 |
| `push1_popN`, 64 slots | 34 / 31 | **98** / **99** | 120–133 |
| `push1_pop1`, 1024 slots | 32 (e 0.38) / 32 (e 0.39) | 34 (e 0.55) / **178** (e 0.06) | 323–374 |
| `push1_popN`, 1024 slots | 32 / 32 | 34 / **151** | 145–148 |
| `push1_pop1`, 16384 slots | 73 (e 0.01) / 70 (e 0.02) | **31** (e 0.53) / **31** (e 0.49) | 378–389 |
| `push1_popN`, 16384 slots | 69 / 67 | **31** / **31** | 143–149 |

The six extra after-side runs of `push1_pop1` at 1024 slots gave 144, 139, 66, 169, 134 and 173 M/s. Counting the two
full runs, 6 of 8 land at 134–178 and 2 at 34 and 66.

### One thread

| Cell | Before, run 1 / 2 | After, run 1 / 2 | `rtrb` |
|---|---|---|---|
| `push_pop` | 3.58 / 3.69 ns | **4.41 / 4.43 ns** | 2.72–2.82 ns |
| `push_full` | 0.53 / 0.50 ns | 0.53 / 0.54 ns | 0.67–0.85 ns |
| `pop_empty`, pop1 | 1.78 / 1.98 ns | **0.73 / 0.73 ns** | 0.48–0.49 ns |
| `pop_empty`, popN | 1.84 / 1.91 ns | **0.55 / 0.56 ns** | 0.33–0.48 ns |
| `fill_drain` `push1_pop1`, 64 / 1024 / 16384 slots, run 1 | 268 / 253 / 50 M/s | **411 / 403** / 50 M/s | 374 / 358 / 359 M/s |
| `fill_drain` `push1_popN`, 64 / 1024 / 16384 slots, run 1 | 419 / 466 / 55 M/s | **905 / 971** / 54 M/s | 1073 / 1236 / 1206 M/s |

The `fill_drain` rows show run 1. Run 2 is within 2% of it in every cell.

### Per commit, one thread (separate build)

| Commit | `push_pop` | `pop_empty` popN |
|---|---|---|
| `4cbd73c`, before | 3.68 ns | 1.62 ns |
| `6ff64eb`, empty batch commits nothing | 4.02 ns | 0.48 ns |
| `e42ac0a`, end-local copies | 4.16 ns | 0.56 ns |

## Reading

- **Gains:**
  - 64 slots, 3.4x.
  - 1024 slots, 4–5.5x in 6 of 8 runs.
  - The single-thread drain paths, 1.5–2.2x, and `pop_empty`, 2.4–3.4x.
- **1024 slots is still bistable.** Two runs out of eight stayed in the regime where the consumer polls right behind
  the producer. With 64-byte padding the two cursors can share a 128-byte line on this machine. Raising the padding to
  128 bytes removed most of this (`ring_align/docs/benchmarks/001_cache_line_128.md`, on `perf/cache-line-128`).
- **16384 slots is a regression, 70–73 → 31 M/s.** The before side sat in a placement where the old protocol happens to
  run well. An earlier review of the same machine measured the old protocol at 71 M/s with the ring at offset 0 mod 128
  and 23 M/s at offset 64. Every cached-cursor variant it tried landed at 24–37 M/s with `TypedSlot`. The likely cap
  is `TypedSlot`'s `Option` tag, an extra store per record into an array larger than L1D. That fix was deliberately
  not taken.
- **`push_pop` is 0.8 ns slower.** Most of it arrives with `6ff64eb`, whose only change, the empty-batch return, never
  runs in that loop. That points at code layout rather than the protocol. Not confirmed with a disassembly.

## Reproduce

Check out each commit and run the two commands above twice. `cargo run -q -p perf --example report -- --out
target/perf_report.md` pivots the last results into one table.
