# `CACHE_LINE` 64 → 128: before and after, on an Apple M4 Pro

These measurements support [ADR 002](../decisions/002_cache_line_follows_the_target_architecture.md), which
replaced ADR 001's unconditional 64. They were taken for an unconditional 128, the step before ADR 002's
per-target table. The table keeps AArch64 at 128 on Apple targets (`target_vendor = "apple"`, which includes
the measured `aarch64-apple-darwin`), so the numbers stand for it. The commit under test is
`perf(ring_align)!: pad to 128-byte cache lines` on `perf/cache-line-128`.

It was measured as `a8037d0`. The branch was later rebased under a docs-only commit, so on the branch it is `3d7e8fd`,
with identical source. The refactor before it (`refactor: reach the cache-line size only through ring_align`) changes
no layout.

**Before** and **after** differ only in the line size. Both sides already carry `ring_spsc`'s end-local cursor copies
from `perf/spsc-end-local-state`, and neither carries `ring_mpsc`'s empty-drain change.

| | Before | After |
|---|---|---|
| Commit measured | `e42ac0a` (SPSC runs), `e70c182` (MPSC runs). The second adds only a gate record | `a8037d0` |
| `ring_align::CACHE_LINE`, `CacheAligned` alignment | 64 | 128 |

## Machine

| | |
|---|---|
| Model | Apple M4 Pro (`Mac16,7`), 14 cores: 10 performance, 4 efficiency |
| Memory | 24 GB |
| Caches | 128-byte lines (`sysctl -n hw.cachelinesize`). Performance cores: 128 KB L1D each, 16 MB L2 per cluster. Efficiency cores: 64 KB L1D each, 4 MB L2 |
| OS | macOS 27.0.1, read on 2026-10-05. The 2026-10-04 runs came before a reboot, on the same machine |
| Toolchain | rustc 1.97.1 stable, the workspace `bench` profile (fat LTO, one codegen unit) |
| Thread placement | Not pinned: the harness pins only on Linux |
| Background, 2026-10-04 | Desktop session, 1-minute load average 1.3–6.8 at the start of each run |
| Background, 2026-10-05 | 13 minutes after a reboot, with a video call taking about half a core. 1-minute load average 3.3–4.3 |

No 64-byte-line host has been measured. The cost the ADR expects, doubled padding on such hosts, is unmeasured.

## Method

- Criterion through the `perf` suite. Every number is criterion's median. Each run is a fresh process. `e` is the
  harness's count of empty consumer polls per record.
- **2026-10-04, full suites, two runs per side:**

  ```sh
  cargo bench -p perf --bench spsc -- '^spsc/(spsc|rtrb)/'
  cargo bench -p perf --bench micro -- '/(spsc|rtrb)'
  cargo bench -p perf --bench mpsc -- '^mpsc_producers/(mpsc|arrayqueue)/'
  ```

  - The SPSC before-runs came about 1.5 hours before the after-runs.
  - The MPSC before-runs and after-runs ran back to back in one job.
- **2026-10-05, a paired A/B of the bistable cell `spsc/spsc/push1_pop1/1024`.** The bench binary was built once at
  each side (`e70c182` and the then-tip `7f64d5e`, same source as `a8037d0`). The two binaries were alternated 16 times
  with no rebuild in between, run as `<binary> --bench '^spsc/spsc/push1_pop1/1024$'`.

## Results

### Two threads, SPSC (M records/s), 2026-10-04

| Cell | Before, run 1 / 2 | After, run 1 / 2 | `rtrb`, same session |
|---|---|---|---|
| `push1_pop1`, 64 slots | 87 / 90 | 86 / 85 | 256–286 |
| `push1_popN`, 64 slots | 98 / 99 | 89 / 89 | 120–133 |
| `push1_pop1`, 1024 slots | 34 (e 0.55) / 178 (e 0.06) | **408** (e 0.01) / **252** (e 0.04) | 320–374 |
| `push1_popN`, 1024 slots | 34 / 151 | **353** / **245** | 145–150 |
| `push1_pop1`, 16384 slots | 31 / 31 | 31 / 31 | 378–387 |
| `push1_popN`, 16384 slots | 31 / 31 | 31 / 31 | 145–152 |

### Paired A/B, `push1_pop1` at 1024 slots (M records/s), 2026-10-05

| Padding | All 16 runs, in order | Median | Runs below 100 |
|---|---|---|---|
| 64 | 268, 34, 205, 240, 150, 138, 211, 33, 125, 209, 216, 199, 213, 174, 32, 189 | 194 | **3** (32–34) |
| 128 | 273, 268, 204, 224, 214, 103, 139, 220, 205, 201, 249, 259, 261, 256, 182, 241 | 222 | **0** (lowest 103) |

### Two threads, MPSC producer sweep at 1024 slots (M records/s), 2026-10-04

| Cell | Before, run 1 / 2 | After, run 1 / 2 |
|---|---|---|
| `push1_popN`, 1 producer | 17 / 17 | 17 / 17 |
| `push32_popN`, 1 producer | 120 / 121 | 121 / 120 |
| `push1_popN`, 2 producers | 14 / 15 | 15 / 15 |
| `push32_popN`, 2 producers | 340 / 396 | 401 / 416 |
| `push1_popN`, 4 producers | 10 / 10 | 10 / 9 |
| `push32_popN`, 4 producers | 415 / 360 | 386 / 393 |
| `push1_popN`, 8 producers | 3 / 3 | 4 / 4 |
| `push32_popN`, 8 producers | 91 / 91 | **117 / 110** |
| `push1_popN`, 13 producers | 2 / 2 | 2 / 2 |
| `push32_popN`, 13 producers | 49 / 49 | **75 / 71** |
| `ArrayQueue` `push1_popN`, 1 / 4 / 13 producers (control) | 205 / 177 / 109 and 135 / 116 / 52 | 152 / 132 / 52 and 151 / 132 / 52 |

The `ArrayQueue` control moves by up to 2x between runs of identical code. Read every MPSC difference against that
spread.

### One thread, 2026-10-04

No cell moved beyond run-to-run noise:

| Cell | Before | After |
|---|---|---|
| `push_pop` | 4.41–4.43 ns | 4.35–4.44 ns |
| `pop_empty` pop1 / popN | 0.73 / 0.55–0.56 ns | 0.73–0.74 / 0.53 ns |
| `fill_drain` popN at 64 / 1024 / 16384 slots | 905 / 971–972 / 54–55 M/s | 893–907 / 965–978 / 54–56 M/s |

### Type sizes, measured with `size_of`

| Type | 64 | 128 |
|---|---|---|
| `ring_spsc::Ring<TypedSlot<u64>>` | 256 B | 512 B |
| `ring_mpsc::Ring<TypedSlot<u64>>` | 192 B | 256 B |
| `ring_mpsc::Ends<TypedSlot<u64>>` | 256 B | 512 B |
| `(ring_registry::RegistryError, ring_handle::Split<u64>)` | 384 B | 768 B |

## Reading

- **At 1024 slots, 128-byte padding removes the collapse.** With 64-byte padding, 1 of 2 full-suite runs and 3 of 16
  paired runs fell to 32–34 M/s. No 128-byte run fell below 103. The median gain in the paired runs is 14%, smaller
  than the collapse it removes.
- **Batched MPSC rises 1.2x at 8 producers and 1.5x at 13.** Both before-runs at 8 producers sat at 91 M/s. That is
  the value an earlier review of this machine measured with the ring forced to offset 0 mod 128; it got 137–143 M/s at
  offset 64.
- **No other cell moved:** SPSC at 64 and 16384 slots, the single-thread cells, per-record MPSC, and batched MPSC at
  1–4 producers.
- **Cost:** none measurable on this host. The doubled padding costs memory, as the type sizes show. Any cache-footprint
  cost would show on a 64-byte-line host, which is the ADR's remaining revisit trigger.

## Reproduce

Check out each side, then run the three commands above twice. For the paired A/B:

1. Build each side with `cargo bench -p perf --bench spsc --no-run`.
2. Copy each executable it prints out of `target/release/deps/`.
3. Alternate the two copies on `--bench '^spsc/spsc/push1_pop1/1024$'`.
