# Candidates built at a 128-byte boundary: before and after, on an Apple M4 Pro

The commit under test is `bench(perf): build every candidate at the start of a 128-byte block` on
`bench/perf-aligned-rings`. Before it, every run built its queue as a temporary on the bench thread's stack, so a
ring's inline cursors fell wherever the stack put them. After it, `candidates::build` boxes every candidate at
128-byte alignment.

**Before** is master at `0aeea4e`. **After** is `b2e1b33`, master plus that commit. Only `perf/` differs; the
measured crates are the same source on both sides.

## Machine

| | |
|---|---|
| Model | Apple M4 Pro, 14 cores: 10 performance, 4 efficiency |
| Memory | 24 GB |
| Caches | 128-byte lines. Performance cores: 128 KB L1D each, 16 MB L2 per cluster |
| OS | macOS 27.0.1 |
| Toolchain | rustc 1.98.0 stable, the workspace `bench` profile (fat LTO, one codegen unit) |
| Thread placement | Not pinned: the harness pins only on Linux |
| Background | Desktop session; load average not recorded |

## Method

- Criterion through the `perf` suite. Every number is criterion's median for one process.
- **Paired A/B, 2026-10-08.** Each side's `spsc` and `mpsc` bench binaries were built once, in its own worktree and
  target directory, then run alternately as separate processes: 8 rounds, the before side first in every round. A
  new process places the ring anew, so the spread across rounds is the quantity under test.
- `rtrb` builds its own ring inside `split`, so the change cannot reach its rows: they show the noise and the drift
  of this method. `ArrayQueue` keeps its storage behind an `Arc` on both sides.

  ```sh
  <binary> --bench '^spsc/(spsc|rtrb)/push1_popN/(64|1024|16384)$'
  <binary> --bench '^mpsc_producers/(mpsc|arrayqueue)/push(1|32)_popN/(1|8)$'
  ```

## Results (M records/s, median and range of 8 processes)

### One producer thread, one consumer thread

| Cell | Before | After |
|---|---|---|
| `spsc` `push1_popN`, 64 slots | 131.1 (127.2–132.5) | 134.8 (114.8–136.5) |
| `spsc` `push1_popN`, 1024 slots | 289.9 (279.8–295.8) | 282.9 (266.9–290.6) |
| `spsc` `push1_popN`, 16384 slots | 37.3 (29.4–38.7) | 38.4 (25.1–39.8) |
| `rtrb` `push1_popN`, 64 slots (control) | 128.6 (122.1–134.0) | 118.2 (108.1–119.5) |
| `rtrb` `push1_popN`, 1024 slots (control) | 151.0 (145.8–152.0) | 147.0 (124.4–151.7) |
| `rtrb` `push1_popN`, 16384 slots (control) | 152.1 (122.7–155.0) | 146.3 (139.4–150.6) |

### One consumer thread plus one thread per producer, 1024 slots

| Cell | Before | After |
|---|---|---|
| `mpsc` `push1_popN`, 1 producer | 17.8 (17.6–18.2) | 17.9 (17.5–20.2) |
| `mpsc` `push1_popN`, 8 producers | 3.75 (3.50–3.92) | 3.69 (3.17–4.00) |
| `mpsc` `push32_popN`, 1 producer | 210.1 (192.9–215.0) | 237.9 (226.7–275.1) |
| `mpsc` `push32_popN`, 8 producers | 119.2 (112.9–144.7) | **97.5 (94.0–98.1)** |
| `ArrayQueue` `push1_popN`, 1 producer | 186.6 (123.6–213.9) | 201.8 (185.9–205.7) |
| `ArrayQueue` `push1_popN`, 8 producers | 76.4 (53.2–102.4) | **105.1 (101.5–106.2)** |

## Reading

- **`ring_spsc` was already placement-stable, and stays so.** Its medians move by 3% or less and every range
  overlaps. The 25-against-72 M/s jump this commit was written against was measured before #20, when every push still
  read the consumer's line; since #20 the cell no longer depends on where that line falls.
- **Batched `ring_mpsc` at 8 producers is now repeatable, at its slower placement.** Before, it landed either near
  113 or near 145 M/s; after, every process lands at 94–98. An earlier bench-only experiment forced the two
  placements and measured 91–95 M/s at offset 0 against 137–143 at offset 64, so offset 0 is the slow mode. The cell
  now reads about 18% under the old median, and stays there.
- **`ArrayQueue` at 8 producers narrowed from 53–102 to 101–106.** Its storage is on the heap on both sides, so what
  moved is something placed with the candidate itself. Not explained.
- **The before side ran first in every round,** and the `rtrb` rows, which the change cannot touch, read 3–8% lower
  after. That bias bounds every ratio here: a median shift under about 8% is not attributable to the change.

Verdict: the harness change does what it is for on the cells where placement mattered, and costs nothing where it
did not. It also fixes batched `ring_mpsc` at 8 producers to the slower of its two placements, which later
comparisons on that cell should state.

## Reproduce

1. Check out each side in its own worktree and build with a separate `CARGO_TARGET_DIR` per side:
   `cargo bench -p perf --bench spsc --bench mpsc --no-run`. A shared target directory reuses the first side's
   binary for the second, because cargo fingerprints the source files of the worktree that built it first.
2. Copy each executable it prints out of the target directory.
3. Alternate the copies on the two filters above, one process per run.
