# The consumer caches the producer cursor, and a drain reloads it only when it knows too little

Status: Accepted — measured on an Apple M4 Pro with every ring at a fixed cache-line placement

## Context

ADR 001 gave the producer a private copy of the consumer cursor and left the consumer end untouched: every
`drain` and `drain_up_to` loaded the producer cursor at `GATING`. PR #22 had proposed caching both ends at once and
was closed for the producer-only #20, which became ADR 001; its consumer half was never measured alone.

PR #38 (`ring_slot::CopySlot`) then measured the two-thread one-record-per-pop cells 3–4x slower untagged than
tagged, while every single-thread and large-capacity cell improved. Five independent adversarial reviews of that
result, on the same machine, reached the same place from five directions:

- The ring is built on the bench thread's stack and `ring_align::CACHE_LINE` is 64 on a 128-byte-line machine, so
  whether the two cursors share one line is decided by the environment size and argv length. One unused environment
  variable of 1–113 characters flips the same binary from `CopySlot` at 0.26x of `TypedSlot` to 3.2x faster.
- The slow state is the ring full, the producer one slot behind the consumer, refreshing `cached_head` before nearly
  every push. Over 288 runs, time per record = 4.3 ns + 48 ns × producer refreshes per record (R² 0.91); one refresh
  is one cross-core round trip, measured at 37.8 ns one-way. Which end is faster decides whether the ring sits there,
  and `CopySlot` made the producer faster.
- With the consumer caching the producer cursor, every layout and every slot shape ran at 420–500 M/s at 1024 slots,
  and the layout stopped mattering. The orderings themselves are necessary: a `Relaxed` load or publish, measured
  only, left the collapse in place and corrupted up to 796k of 1M records.
- The disassembly shows one `ldapr` and one `stlur` per pop and one `stlr` per push, no `dmb`; `CopySlot` changes
  none of them. The one load the consumer makes of the producer's line on every pop is the whole per-record cost.

## Decision

`Consumer` carries one plain field, `cached_produced`: its latest `GATING` observation of the producer cursor. A
drain reloads it only when the records it already knows of, `start.distance_to(cached_produced)`, are fewer than
the drain asks for. `drain` asks for `usize::MAX`, so it always reloads. The reload writes the field only when the
observation moved. `available` and `is_empty` keep reading the shared cursor.

The reload rule is `distance < max`, not `start == cached`. The equality rule, which an earlier draft of this
change used, makes `drain()` return fewer records than `available()` reports after a bounded drain, and a cache that
starts at zero after a re-split reports an empty ring forever under it. Both are pinned by tests; the `<=` variant
of the rule is an equivalent mutant, reloading one call early and otherwise identical.

### Soundness

The cache can only lag the real cursor: the producer cursor is monotone and the cache is written from observations,
never extrapolated. A stale value under-reports a ring that has more; it never offers a record the producer has not
published. Every record it does offer was published before a `HANDOFF` store that a `GATING` load by this end
observed, on this drain or an earlier one, and program order carries that edge forward to the slot read. The model
`a_bounded_drain_hands_out_only_records_whose_writes_it_can_see` fails under a `Relaxed` reload and under a cache
that extrapolates past its observation; before it, no loom model called `drain_up_to`.

## Measurement

Apple M4 Pro, macOS, rustc 1.98.0, `bench` profile, threads not pinned, two rounds alternating before and after in
separate processes. For the two-thread cells the harness was patched, for the measurement only, to place each ring
on the heap at byte 0 or byte 64 of a 256-byte block: at 0 both cursors share one 128-byte line, at 64 they sit on
two. Before is master `9598875`; after is this change. Criterion medians, M records/s, both rounds.

| two threads, `spsc` | one line, before | one line, after | two lines, before | two lines, after |
|---|---|---|---|---|
| `push1_pop1`, 64 slots | 102, 101 | **136, 128** | 29, 30 | **122, 123** |
| `push1_pop1`, 1024 slots | 261, 252 | **333, 337** | 216, 188 | **332, 337** |
| `push1_pop1`, 16384 slots | 46, 47 | 36, 35 | 47, 48 | 36, 35 |
| `push1_popN`, 64 slots | 133, 137 | 133, 129 | 125, 123 | 123, 123 |
| `push1_popN`, 1024 slots | 292, 291 | 416, 272 | 295, 296 | 343, 423 |
| `push1_popN`, 16384 slots | 38, 38 | 35, 36 | 38, 40 | 36, 35 |

| one thread, `spsc` (criterion median, two rounds) | before | after |
|---|---|---|
| `push_pop` | 3.60 / 3.73 ns | 3.98 / 3.98 ns |
| `pop_empty` pop1 | 2.10 / 1.56 ns | 1.98 / 1.84 ns |
| `pop_empty` popN | 1.95 / 1.91 ns | 1.73 / 1.75 ns |
| `fill_drain` pop1, 64 / 1024 slots | 177 ns / 2.80 µs | **158 ns / 2.51 µs** |
| `fill_drain` pop1, 16384 slots | 338 / 324 µs | 318 / 368 µs |
| `fill_drain` popN, 64 / 1024 slots | 65.0 ns / 0.96 µs | 73.2 ns / 1.06 µs |
| `fill_drain` popN, 16384 slots | 307 / 298 µs | 297 / 301 µs |

- **The one-record hand-off no longer depends on where the ring lands.** 29 → 122 M/s on two lines at 64 slots, and
  the two layouts now agree within 5% at both capacities. The one-line layout gains 30% as well.
- **Whole-lap drains are unchanged.** They read the producer cursor once per lap already. The 1024-slot cell is
  bimodal in both columns; which mode a run lands in is the chase's initial race, recorded in PR #38's review.
- **`push1_pop1` at 16384 slots loses 23%.** That cell is the 256 KB `TypedSlot` array leaving the 128 KB L1D with
  the consumer polling right behind the producer; the reviews found it producer-bound in every layout. A smaller
  array (`CopySlot`) does not have it. Recorded, not explained.
- **Single-threaded, the one-record drain is 10% faster and the whole-lap drain 10% slower.** The first is one
  atomic load fewer per record. The second is one compare per lap, which cannot cost 100 ns; it moved with the
  conditional store and did not move with the reload rule, so it is code layout. `push_pop` pays 7%, the same shape
  ADR 001 recorded for the producer cache. Unexplained, and small against the two-thread change.

## Consequences

- `Consumer` is one word larger. The ends are still `!Clone` and `!Sync`; no public signature changes.
- `drain_up_to` can be satisfied from an earlier observation and then reads no line the producer writes. A caller
  that polls with `drain_up_to(1)` on an empty ring reloads every call, as before, and stores nothing.
- A consumer that is slower than its producer still parks the ring at full and pays the producer's refresh per
  record in `pop1`. The cache moves the threshold at which that happens; it does not remove the state. Whole-lap
  drains are the answer to that, as they were.
- `ring_align`'s 64-byte padding on a 128-byte-line host put the two cursors in one line or two by accident. With
  this change the two layouts measure the same, so raising `CACHE_LINE` to 128 (closed #23) can be decided on
  footprint rather than on this hand-off; without this change it measured 3–7x slower on `pop1`.
- Idea #22 in the family's improvement notes, "both ends cache both cursors", is no longer ruled out: its consumer
  half is this ADR. The remaining half of #22, the ends keeping their own position in a field, is not taken here.
- Revisit when a workload shows the 16384-slot `pop1` cell, or when hardware counters become available to replace
  the timing inference the two unexplained single-thread deltas rest on.
