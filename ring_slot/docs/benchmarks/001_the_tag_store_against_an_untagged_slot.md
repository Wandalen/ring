# `TypedSlot`'s tag against `CopySlot`, on an Apple M4 Pro

`TypedSlot<T>` stores `Option<T>`, so every push writes a tag beside the record: `Option<u64>` is 16 bytes. Both
rings derive which slots hold a record from their cursors, so the tag repeats what they know. `CopySlot<T>` stores the
record alone. This record compares the two through the same rings.

The commit under test is `bench(perf): spsc-plain and mpsc-plain, the rings over CopySlot` (`c28bedb`), on master
`0aeea4e` plus `CopySlot` (`bfe34b5`) and `ring_spsc`'s `try_push` over it (`548df5e`). `spsc-plain` and `mpsc-plain`
run beside their tagged pairs, `spsc` and `mpsc`, in the same binary. The tagged adapters are master's, unchanged.

The two sides differ in the read as well as the write. The tagged adapters filter every record through
`TypedSlot::get`, a branch on the tag; the untagged ones read `CopySlot::get`. An earlier bench-only experiment
measured that branch at noise level (a tag written and checked, 27–28 M/s, against written and not checked, 32 M/s),
so the comparison is mostly the store. The slot array also halves: 16384 `u64` slots take 256 KB tagged and 128 KB
untagged, the size of a performance core's L1D.

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
- **Intra-run, 2026-10-08.** One build of the `spsc`, `mpsc` and `micro` bench binaries at `c28bedb`, each run as 4
  separate processes. Tagged and untagged rows come from the same process.
- The harness builds each queue as a temporary on the bench thread's stack, as master does. Where a ring's cursors
  fall against the 128-byte lines therefore depends on each candidate's own stack frame, and can differ between the
  tagged and the untagged candidate within one process. The Reading below says which cells that decides.

  ```sh
  <spsc>  --bench '^spsc(_payload)?/(spsc|spsc-plain|rtrb)/push1_pop(1|N)/'
  <mpsc>  --bench '^mpsc_producers/(mpsc|mpsc-plain)/push(1|32)_popN/(1|2|4|8)$'
  <micro> --bench '^(push_pop|push_full|pop_empty|fill_drain)/(spsc|spsc-plain|mpsc|mpsc-plain)(/|$)'
  ```

## Results

### `ring_spsc`, one thread (median and range of 4 processes)

| Cell | `TypedSlot` | `CopySlot` | Ratio |
|---|---|---|---|
| `push_pop` | 3.81 ns | **2.97 ns** | −0.84 ns |
| `push_full` | 0.550 ns | 0.551 ns | |
| `pop_empty` pop1 / popN | 1.96 / 1.85 ns | 1.90 / 1.93 ns | |
| `fill_drain` popN, 64 slots | 884 M/s (869–895) | **1111** (1077–1151) | 1.26x |
| `fill_drain` popN, 1024 slots | 971 M/s (948–975) | **1222** (1184–1236) | 1.26x |
| `fill_drain` popN, 16384 slots | 53.1 M/s (52.7–54.2) | **1004** (893–1038) | **19x** |
| `fill_drain` pop1, 16384 slots | 48.8 M/s (44.2–50.6) | **230** (212–236) | **4.7x** |
| `fill_drain` pop1, 64 / 1024 slots | 359 / 356 M/s | 343 / 361 | ±4% |

### `ring_spsc`, one producer thread, one consumer thread (M records/s, median and range of 4 processes)

| Cell | `TypedSlot` | `CopySlot` |
|---|---|---|
| `push1_popN`, 64 slots | 105.4 (104.0–119.4) | 97.3 (93.7–116.3) |
| `push1_popN`, 1024 slots | 153.7 (115.0–199.6) | 229.0 (185.0–495.8) |
| `push1_popN`, 16384 slots | 26.0 (24.9–30.4) | **326.0** (306.4–441.1) |
| `push1_pop1`, 64 slots | 77.2 (74.5–83.6) | **20.4** (19.4–20.9) |
| `push1_pop1`, 1024 slots | 158.4 (137.4–225.8) | **51.4** (34.8–73.9) |
| `push1_pop1`, 16384 slots | 34.2 (30.2–40.4) | 47.1 (44.0–57.6) |
| 8-byte record, 1024 slots | 237.7 (216.3–505.2) | 571.0 (242.6–919.9) |
| 64-byte record, 1024 slots | 74.9 (72.1–80.1) | **40.7** (37.7–49.7) |
| 256-byte record, 1024 slots | 42.0 (40.9–44.3) | **26.7** (26.1–27.4) |

`rtrb` in the same processes, for reference: 94.6 / 120.6 / 124.4 M/s `push1_popN` and 219 / 334 / 357 M/s
`push1_pop1` at 64 / 1024 / 16384 slots.

### `ring_mpsc` (median of 4 processes)

| Cell | `TypedSlot` | `CopySlot` |
|---|---|---|
| `push1_popN`, 1 / 2 / 4 / 8 producers | 16.9 / 15.3 / 6.40 / 3.41 M/s | 16.3 / 14.7 / 6.41 / 3.45 |
| `push32_popN`, 1 / 2 / 4 / 8 producers | 192 / 354 / 260 / 142 M/s | 187 / 272 / 270 / 152 |
| `fill_drain` push32_popN, 64 / 1024 / 16384 slots | 600 / 612 / 473 M/s | 626 / 637 / 498 |
| `push_pop` | 4.56 ns | 4.09 ns |
| `push_full` | 1.76 ns | 1.50 ns |

## Reading

- **One thread, the gain is clear and repeatable.** `push_pop` drops 0.84 ns, and a fill then drain moves 26% more
  records at 64 and 1024 slots. Every range is narrow, and none overlaps the other side.
- **At 16384 slots the tagged ring collapses, and not because of a second core.** One thread filling and draining
  16384 tagged slots moves 53 M/s, against 1004 untagged; two threads, 26 against 326. The tagged array is 256 KB and
  the untagged one fits the 128 KB L1D. Why leaving L1D costs this much is not established.
- **The two-thread cells at 64 and 1024 slots follow placement, not the slot.** One-at-a-time hand-off measures 20 and
  51 M/s untagged against 77 and 158 tagged. An earlier run of the same comparison, with every ring built at one fixed
  128-byte offset, measured those untagged cells at 99 and 196, within 7% and 23% of the tagged ones. The slot code is
  the same; what changed is where each candidate's cursors land. `push1_popN` at 1024 slots spans 115–200 tagged and
  185–496 untagged across four processes, the same shape. These cells cannot be read on master's harness. The
  addendum below has the cause and the remeasurement.
- **Records of 64 and 256 bytes are slower untagged,** by 46% and 36% here and 30% and 44% in the fixed-offset run.
  The untagged strides are powers of two (64, 256 bytes) against 72 and 264 tagged, and `rtrb`, whose strides are
  powers of two as well, measures close to the untagged slot. The addendum separates the two rows: the 256-byte one
  was the harness, the 64-byte one is the stride.
- **`ring_mpsc` barely moves through the rings.** Its per-record cost is the claim, not the slot. Single-threaded,
  `push_pop` and `push_full` gain about 0.4 and 0.26 ns.

Verdict, as written on `c28bedb`: `CopySlot` pays for small `Copy` records in `ring_spsc` wherever placement does not
decide the cell: single thread at every capacity, and two threads at large capacities. The small-capacity two-thread
cells need a harness that fixes placement before they say anything about the slot, and wide records need the stride
question answered before a caller with them chooses `CopySlot`. The addendum revises the two-thread and 256-byte parts.

## Addendum, 2026-10-10: the two-thread cells explained, the 256-byte row corrected

Five independent adversarial reviews of the run above, on the same machine, traced the two-thread `push1_pop1` cells
to one read: `ring_spsc`'s consumer loaded the producer cursor on every drain. With `ring_align::CACHE_LINE` at 64 on a
128-byte-line host, whether the two cursors share one line or two depends on the ring's address mod 128, which the
stack-built harness takes from the environment size and argv; one unused environment variable of 1–113 characters
flips the same binary from `spsc-plain` at 0.26x of `spsc` to 3.2x faster. The slow state is the ring full with the
producer one slot behind the consumer, refreshing its cache before nearly every push at one cross-core round trip each
(time per record 4.3 ns + 48 ns × refreshes, R² 0.91 over 288 runs). `CopySlot` makes the producer faster, which tips a
one-record-per-pop consumer into that state sooner. The orderings are necessary: a `Relaxed` load or publish, measured
only, kept the collapse and corrupted up to 796k of 1M records. `ring_spsc` ADR 002 (#41) gives the consumer the cache
the producer already had.

**Two threads, this branch rebased on #41, with and without the consumer cache.** Harness patched, for the measurement
only, to place each ring at byte 0 or 64 of a 256-byte block: at 0 both cursors share one 128-byte line, at 64 they sit
on two. Two rounds, M records/s.

| `push1_pop1` | one line, no cache | one line, cache | two lines, no cache | two lines, cache |
|---|---|---|---|---|
| `TypedSlot`, 64 slots | 101, 86 | 129, 131 | 31, 30 | 120, 121 |
| `CopySlot`, 64 slots | 102, 100 | **136, 142** | 24, 24 | **122, 124** |
| `TypedSlot`, 1024 slots | 191, 258 | 264, 267 | 227, 236 | 337, 284 |
| `CopySlot`, 1024 slots | 187, 199 | **444, 360** | 24, 24 | **443, 439** |
| `TypedSlot`, 16384 slots | 39, 47 | 32, 30 | 47, 47 | 33, 37 |
| `CopySlot`, 16384 slots | 260, 264 | **475, 445** | 29, 29 | **469, 393** |

`push1_popN` does not move with the cache on either slot; it reads the cursor once per lap already.

**Wide records: the check, not the slot, made the 256-byte row.** `Wide::whole` compared the first and last word only.
At the tagged 264-byte stride the middle 128-byte line of a 256-byte record was never read and never crossed cores,
so the tagged consumer pulled one line fewer per record than the untagged one. `fix(perf): the wide record's
whole-record check reads every word` makes the check read every word. Stock harness, 1024 slots, `push1_popN`, two
rounds, M records/s:

| record | `TypedSlot`, old check | `TypedSlot`, every word | `CopySlot`, old check | `CopySlot`, every word | `rtrb`, every word |
|---|---|---|---|---|---|
| 8 bytes | 271, 369 | 361, 281 | 570, 910 | 881, 728 | 144, 136 |
| 64 bytes | 80.7, 91.0 | 99.3, 93.7 | 42.0, 42.6 | **62.6, 44.6** | 65.9, 52.7 |
| 256 bytes | 46.9, 52.0 | **31.8, 30.2** | 26.6, 26.9 | **29.4, 29.1** | 31.0, 29.6 |

- **256 bytes: a tie at about 30 M/s** for `TypedSlot`, `CopySlot` and `rtrb` once every word is read. The 36–44%
  "regression" was the checker's footprint. The tagged side lost 40% because it now pays for the line it skipped.
- **64 bytes: the gap stays**, 94–99 tagged against 45–63 untagged. The reviews placed it on the 64-byte stride, not
  the slot: `CopySlot` padded to a 72-byte stride measured 81–96 M/s, a tagless `TypedSlot` at stride 64 was slow too,
  stride 128 against 136 was within 10%, and a consumer held 64 records behind the producer removed the dependence.
  4K aliasing is ruled out: a same-core effect that does not vanish with lag. The mechanism is open without hardware
  counters. A caller with 64-byte records should measure, and may pad.

Revised verdict: `CopySlot` pays for `Copy` records in `ring_spsc` at every capacity and in both threading shapes once
the consumer caches the producer cursor, and the 256-byte row was never about the slot. The one open row is 64-byte
records at a 64-byte stride.

## Reproduce

1. Build `cargo bench -p perf --bench spsc --bench mpsc --bench micro --no-run` at `c28bedb`, with a target directory
   of its own if another checkout shares one.
2. Run each executable it prints on its filter above, one process per run.
