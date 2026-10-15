# An empty batch commits nothing: before and after, on an Apple M4 Pro

The commit under test is `perf(ring_mpsc): an empty batch commits nothing` on `perf/mpsc-empty-drain`. Before it,
`Batch::drop` stored the consumer cursor even when a drain found nothing.

**Before** is master at `c65f4e4`. **After** is the commit on top of it, measured as `750cd22`. The two sides differ
only in `ring_mpsc/src/lib.rs`. Both use 64-byte `ring_align::CACHE_LINE` padding.

## Machine

| | |
|---|---|
| Model | Apple M4 Pro (`Mac16,7`), 14 cores: 10 performance, 4 efficiency |
| Memory | 24 GB |
| Caches | 128-byte lines. Performance cores: 128 KB L1D each, 16 MB L2 per cluster. Efficiency cores: 64 KB L1D each, 4 MB L2 |
| OS | macOS 27.0.1 |
| Toolchain | rustc 1.97.1 stable, the workspace `bench` profile (fat LTO, one codegen unit) |
| Thread placement | Not pinned: the harness pins only on Linux |
| Background | Desktop session, 1-minute load average 3.1–5.9 at the start of each run |

## Method

- Criterion through the `perf` suite. Every number is criterion's median.
- **Paired A/B, 2026-10-05.** Each side's `mpsc` and `micro` bench binaries were built once, then run alternately
  with no rebuild: 8 pairs for the threaded cells, 6 for the micro cells. The side that runs first swaps every round.
  `ArrayQueue` cells in the same binaries act as a control.

  ```sh
  <binary> --bench '^mpsc_producers/(mpsc/push32_popN/1|mpsc/push1_popN/(1|2)|arrayqueue/push1_popN/1)$'
  <binary> --bench '^(push_pop|push_full)/(mpsc|arrayqueue)$|^pop_empty/mpsc/'
  ```

## Results

### One consumer thread plus one thread per producer, at 1024 slots (M records/s, median and range of 8 runs)

| Cell | Before | After | Ratio |
|---|---|---|---|
| `push32_popN`, 1 producer | 110 (98–115) | **193** (144–198) | **1.76x** |
| `push1_popN`, 1 producer | 15.7 (13.6–16.1) | 17.5 (14.4–21.0) | 1.12x, ranges overlap |
| `push1_popN`, 2 producers | 11.3 (10.1–13.6) | 14.2 (11.4–16.6) | 1.25x, ranges overlap |
| `ArrayQueue` `push1_popN`, 1 producer (control) | 185 (148–194) | 193 (157–196) | 1.05x |

Round 1 was the slowest round for both sides. It holds the low end of five of the eight ranges above.

### One thread (median of 6 runs)

| Cell | Before | After |
|---|---|---|
| `pop_empty/mpsc` pop1 | 2.76 ns | **1.65 ns** (−40%) |
| `pop_empty/mpsc` popN | 2.67 ns | **1.50 ns** (−44%) |
| `push_pop/mpsc` | 4.04 ns | 4.49 ns (+11%) |
| `push_full/mpsc` | 1.52 ns | 1.77 ns (+17%) |
| `push_pop/arrayqueue` (control) | 3.56 ns | 3.54 ns |
| `push_full/arrayqueue` (control) | 1.52 ns | 1.50 ns |

The six `push_full/mpsc` runs span 1.50–1.54 ns before and 1.76–1.78 ns after.

## An earlier measurement, on a 128-byte base

The same change was first measured on top of `perf/cache-line-128`, which pads cursors to 128 bytes and carries
`ring_spsc`'s end-local cursor copies. The base was `384e982`, and the change was `8d661a7`, on `296fc9a`, which adds
only Markdown to `384e982`. Some of the runs used earlier local builds of the same two commits that were never pushed.
They differ from the pushed ones only in Markdown and mutant-survey files, which the benchmarks do not compile.

That base's `ring_mpsc` is not quite master's. It also carries `e57df95`, under which `Producer::on_distinct_lines`
asks `ring_align::on_distinct_lines` instead of comparing the two addresses against 64 itself, and `ring_mpsc` depends
on `ring_align`.

The paired A/B used the same method on 2026-10-05. Its ratios match the ones above.

| Cell | Before | After | Ratio |
|---|---|---|---|
| `push32_popN`, 1 producer | 115 (108–122) | 223 (208–233) | 1.94x |
| `push1_popN`, 1 producer | 17.9 (16.5–18.4) | 20.9 (19.7–23.4) | 1.16x |
| `push1_popN`, 2 producers | 13.3 (11.6–14.9) | 14.8 (11.4–16.3) | 1.12x, ranges overlap |
| `ArrayQueue` `push1_popN`, 1 producer (control) | 188 (165–193) | 146 (115–212) | 0.77x |
| `pop_empty/mpsc` pop1 / popN | 2.67 / 2.66 ns | 1.61 / 1.49 ns | −40% / −44% |
| `push_pop/mpsc` | 3.99 ns | 4.47 ns | +12% |
| `push_full/mpsc` | 1.49 ns | 1.75 ns | +17% |

The full producer sweep ran on that base only, on 2026-10-04, two runs per side, with
`cargo bench -p perf --bench mpsc -- '^mpsc_producers/(mpsc|arrayqueue)/'`. It covers 4–13 producers, which the
paired runs skip. Nobody has repeated it on master.

| Cell (M records/s, run 1 / run 2) | Before | After |
|---|---|---|
| `push1_popN`, 1 producer | 17 / 17 | 20 / 20 |
| `push32_popN`, 1 producer | 121 / 120 | 225 / 224 |
| `push1_popN`, 2 producers | 15 / 15 | 17 / 17 |
| `push32_popN`, 2 producers | 401 / 416 | 412 / 409 |
| `push1_popN`, 4 producers | 10 / 9 | 10 / 11 |
| `push32_popN`, 4 producers | 386 / 393 | 381 / 382 |
| `push1_popN`, 8 producers | 4 / 4 | 4 / 4 |
| `push32_popN`, 8 producers | 117 / 110 | 110 / 109 |
| `push1_popN`, 13 producers | 2 / 2 | 2 / 2 |
| `push32_popN`, 13 producers | 75 / 71 | 75 / 75 |
| `ArrayQueue` `push1_popN`, 1 producer (control) | 152 / 151 | 204 / 202 |
| `ArrayQueue` `push1_popN`, 13 producers (control) | 52 / 52 | 110 / 112 |

The `ArrayQueue` control drifted 1.3–2.1x between the before and after sessions of the sweep. Read its deltas against
that drift.

## The single-thread shift, disassembled

Both sides' `micro` executables were rebuilt from `c65f4e4` and `750cd22` on 2026-10-07 with the same toolchain and
disassembled with `objdump -d`.

- **`push_full` executes the same instructions on both sides.** Its loop and `MpscTx::try_push`, the one function the
  loop calls, match instruction for instruction once addresses are masked. Only their addresses differ. After the
  change, `Batch`'s drop glue is a function of its own, placed ahead of them in the binary. That moves `try_push` 164
  bytes, from 76 to 112 bytes past a 128-byte boundary.
- **`push_pop` executes different instructions.** Its loop drains, so it inlines `Batch::drop`. Before, the landing
  pad that drops a `Batch` during an unwind inlined the drop as well, one `stlr` reading the batch's fields from
  registers. After, the drop has a branch, and LLVM calls `drop_glue::<Batch>` out of line on that cold path. The
  call takes the batch by address, so the loop now stores the batch's three fields to the stack on every iteration,
  one `stp` and one `str` that the before loop does not have. The unwind edge comes from the slot bounds checks that
  run while a batch is alive. `#[inline(always)]` on `Batch::drop` leaves the glue out of line and the loop unchanged.

### Placement held equal (ns, median and range of 6 runs, 2026-10-07)

The paired run alternated four executables. Two are each side as built by default. The other two are each side built
with `RUSTFLAGS="-C llvm-args=-align-all-functions=7"`, which starts every function on a 128-byte boundary, so every
function these cells execute starts at the same offset on both sides.

| Cell | Before | After | Before, aligned | After, aligned |
|---|---|---|---|---|
| `push_full/mpsc` | 1.75 (1.74–1.76) | 1.51 (1.50–1.53) | 1.50 (1.49–1.52) | 1.49 (1.49–1.50) |
| `push_pop/mpsc` | 4.24 (4.23–4.29) | 4.23 (4.15–4.26) | 4.02 (3.99–4.06) | 4.28 (4.21–4.35) |
| `push_full/arrayqueue` (control) | 1.48 | 1.48 | 1.48 | 1.48 |
| `push_pop/arrayqueue` (control) | 3.52 | 3.51 | 3.50 | 3.51 |

## Reading

- **The gain is where the consumer often finds the ring empty: one producer, and empty polls.**
  - Batched push at one producer gains 1.76x, and its ranges do not overlap.
  - An empty `pop` is 40–44% cheaper.
  - Per-record push leans the same way at one and two producers, 1.12x and 1.25x, but the ranges overlap.
  - With 4–13 producers the consumer rarely finds the ring empty, so there is little to save. The sweep on the
    128-byte base did not move there.
- **The gain does not depend on the padding.** It shows at 64 bytes and at 128 bytes, at 1.76x and 1.94x.
- **The control does not explain the gain.** `ArrayQueue` in the same paired binaries moved 1.05x here and 0.77x on
  the 128-byte base.
- **`push_full`'s +17% is code placement.** Both sides run the same instructions. With functions aligned they measure
  1.50 and 1.49 ns. Rebuilt by default on 2026-10-07, the gap comes back on the other side, before at 1.75 ns and
  after at 1.51.
- **`push_pop` pays about 0.26 ns, 6%, for the change.** That is the aligned pair's difference. The two stores per
  iteration are what the disassembly shows the loop gaining, so they are the likely cause. The default builds of
  2026-10-07 hide it, because before's build loses 0.22 ns to placement there. The +0.45 ns of 2026-10-05 is about
  this cost plus a placement shift the size of `push_full`'s.
  - The cost is per drain, not per record, so this cell, which drains one record at a time, shows it at its largest.
  - It buys the 40–44% cheaper empty poll and the 1.76x batched push above.
  - Removing it means removing the unwind edge, which means unchecked slot indexing. That is outside this change.

## Reproduce

1. Build each side with `cargo bench -p perf --bench mpsc --no-run` and `cargo bench -p perf --bench micro --no-run`.
2. Copy each executable they print out of `target/release/deps/`.
3. Alternate the copies on the two filters above.
4. For the placement-controlled pair, build `micro` again with
   `RUSTFLAGS="-C llvm-args=-align-all-functions=7"` and a separate `CARGO_TARGET_DIR`, and alternate all four
   executables on `'^(push_pop|push_full)/(mpsc|arrayqueue)$'`.
5. For the disassembly, find the `Bencher::iter` instances for `PushFull` and `PushPop` over `MpscTx` and the
   `MpscTx::try_push` symbol with `nm -C -n`, then disassemble each address range with
   `objdump -d --no-show-raw-insn --start-address=… --stop-address=…`.
