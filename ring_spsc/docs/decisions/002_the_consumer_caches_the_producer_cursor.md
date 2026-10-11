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
bounded drain decides on the records it already knows of, `start.distance_to(cached_produced)`, first: when they
cover the request the batch is `max` and nothing is loaded; otherwise it reloads and takes what the fresh observation
allows. `drain` reloads on every call. The reload writes the field only when the observation moved. `available` and
`is_empty` keep reading the shared cursor.

The order of those two decisions is about the compiled code, not the semantics. The first version reloaded inside a
helper and computed the length afterwards, through one value that was either cached or reloaded. Inlined into a
caller, a one-record drain then had no constant length, LLVM kept the batch loop around it, and the x86 CI runner
measured single-threaded one-record drains 22% slower (#50): seven taken branches per record became nine. Deciding the
hit first lets the length fold to the constant the caller asked for.

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
- **`push1_pop1` at 16384 slots loses 23%, on this machine only.** That cell is the 256 KB `TypedSlot` array leaving
  the 128 KB L1D with the consumer polling right behind the producer; the reviews found it producer-bound in every
  layout, and a 128 KB array (`CopySlot`) measures 445–475 M/s on the same cell with the cache. On the x86_64 CI
  runner the same cell measures +87% against master. A worst-case shape on one core, not a decision-changing cell;
  tracked in #49 with the counters that would settle it.
- **The single-thread deltas in the tables above belong to the first version, and a second round settled both.**
  - *M4, whole-lap drain 10% slower: code layout, confirmed.* The whole-lap drain loop is 16 instructions with one
    taken branch, and its speed follows the address of its head: at ≡ 4 mod 8 it runs about 0.10 ns per record slower
    than at ≡ 0 (523–530 against 419–425 ns per 1024-record drain, the fill phase constant). rustc's default AArch64
    tuning leaves these loop heads unaligned, so each build lands on one or the other by chance; an unrelated 4-byte
    change flipped this cell's sign, and with `-C llvm-args=-align-loops=16` master and the cache measured equal within
    0.5%, `push_pop` 3.7 against 3.7 ns. Whether the suite should pin loop alignment is #53.
  - *x86, one-record drain 22% slower: the first version's codegen, described in the Decision above.* Read from the
    cross-compiled x86_64 assembly; the CI runner's next comparison on this PR confirms it or not (#50).
  - *The final version on the M4* (rings placed at a fixed layout through the harness of #51, criterion medians):
    one-record `fill_drain` 2.44 µs per 1024 (first version 2.50–2.62, master 2.89–3.39); whole-lap 0.92 µs (first
    version 1.05–1.11, master 0.98–0.99); `pop_empty` 1.84 and 1.55 ns (master 2.28 and 2.15–2.30); two threads,
    `push1_pop1` at 1024 slots 305–403 M/s against the first version's 212–343, in both layouts. `push1_popN` at 1024
    stays bimodal for every build; its mode follows the process's history, not the code.

## Consequences

- `Consumer` is one word larger. The ends are still `!Clone` and `!Sync`; no public signature changes.
- `drain_up_to` can be satisfied from an earlier observation and then reads no line the producer writes. A caller
  that polls with `drain_up_to(1)` on an empty ring reloads every call, as before, and skips the write to its own
  field; dropping the empty batch still stores the consumer cursor, with the value it already held, as it did before
  this change. Skipping that store too is sound (one writer, one live batch; every model and test stays green under
  it) and is left as a measured decision for later, since an earlier measurement of it was mixed.
- A consumer that is slower than its producer still parks the ring at full and pays the producer's refresh per
  record in `pop1`. The cache moves the threshold at which that happens; it does not remove the state. Whole-lap
  drains are the answer to that, as they were.
- `ring_align`'s 64-byte padding on a 128-byte-line host put the two cursors in one line or two by accident. With
  this change the two layouts measure the same, so raising `CACHE_LINE` to 128 (closed #23) can be decided on
  footprint rather than on this hand-off; without this change it measured 3–7x slower on `pop1`.
- Idea #22 in the family's improvement notes, "both ends cache both cursors", is no longer ruled out: its consumer
  half is this ADR. The remaining half of #22, the ends keeping their own position in a field, is not taken here.
- The two-thread numbers above come from a measurement-only harness patch that places each ring at a chosen byte of
  a 256-byte block. The stock suite still builds rings on the bench thread's stack, so it cannot reproduce them and
  keeps reporting layout-dependent `push1_pop1` cells. The placement harness lands as its own change
  (`bench/perf-aligned-rings`, with the offset switch), and the cells here should be re-run on it once it does.
- Revisit when a workload shows the 16384-slot `pop1` cell (#49), when the x86 runner contradicts the codegen
  account in the Decision (#50), or when the suite decides how to treat loop alignment (#53).
