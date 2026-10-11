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
- **`push1_pop1` at 16384 slots loses 23–27%: a cliff the cache moves, not one it causes (#49).** Past a
  slot-array size the producer's slot stores stall, the consumer catches up and polls one record behind it, and the
  pair stays there. A store prefetch in the producer removed that state at every capacity tested (16384 slots: 30–33
  to 272–410 M/s), and so did eight independent multiplies per push, so it is a latency effect, not a capacity cost.
  Its onset follows slot size, not slot type: none up to 65536 for 8-byte slots, from 16384 for 16-byte slots
  (`TypedSlot<u64>` and `CopySlot<Wide<2>>` alike), from 2048 for 32-byte slots. The cache moves the 16-byte onset
  from 16384 down to 8192, because a faster consumer catches the producer sooner: across 6 processes per cell it is
  +20…+30% at 2048 slots, −1…−18% at 4096, then in the stuck state. Inside that state its cells run 13–36% slower, for
  a reason not established (not empty polls, not the empty-batch store, not the hit path, not loop alignment). The
  comparison with `CopySlot` at 445–475 M/s that an earlier version of this paragraph made was confounded by its
  8-byte slots. A producer prefetch is #54; nothing in the cache changes for this cell.
- **The single-thread deltas in the tables above belong to the first version, and a second round settled both.**
  - *M4, whole-lap drain 10% slower: code layout, confirmed.* The whole-lap drain loop is 16 instructions with one
    taken branch, and its speed follows the address of its head: at ≡ 4 mod 8 it runs about 0.10 ns per record slower
    than at ≡ 0 (523–530 against 419–425 ns per 1024-record drain, the fill phase constant). rustc's default AArch64
    tuning leaves these loop heads unaligned, so each build lands on one or the other by chance; an unrelated 4-byte
    change flipped this cell's sign, and with `-C llvm-args=-align-loops=16` master and the cache measured equal within
    0.5%, `push_pop` 3.7 against 3.7 ns. Whether the suite should pin loop alignment is #53.
  - *x86, one-record drain 22% slower: the first version's codegen, described in the Decision above, confirmed.* On
    the CI runner (EPYC 7763, Zen 3, intra-run against master) the final version's one-record `fill_drain` measures
    +17.5% / +20.7% / +20.3% at 64 / 1024 / 16384 slots, where the first version measured −20…−25%. Whole-lap is
    −2…−4%, `push_pop` +16%. `pop_empty` is 33–50% slower there (about 1.9 ns): on x86 the Acquire load the cache
    skips is a plain load, so an empty poll pays the cache's compare and bookkeeping and saves nothing.
  - *The final version on the M4*: the table below.
- **On x86, the two-thread one-record cell depends on the core, and on Zen 3 this change makes it slower.** The CI
  runner compares master and this PR in one run on one machine, but the machine varies. On a Zen 5 runner (EPYC 9V45,
  first version) `push1_pop1` measured +27% / +76% / +87% at 64 / 1024 / 16384 slots. On a Zen 3 runner (EPYC 7763),
  first and final version alike, it measured −38…−44% / −33…−38% / −54…−74%, with about one refused push per record:
  the full-ring lockstep, with the consumer as the slow end. Whole-lap there is −22% / −20% / +100%. Why the cache
  deepens the lockstep on Zen 3 is not established; the CI runners give no hardware counters. Tracked in #55.
- **Whole-lap drains at 1024 slots have three regimes, and the slot array's placement picks one** (a 15,000-run
  review): A, producer ahead and the consumer draining whole laps; B, consumer ahead by about 30 records; C, consumer
  polling right behind. Whether the 16 KiB page after a page-aligned array has a translation picks A or B; an array
  starting at the first byte of the page after the ring's picks C. Neither the initial race nor code layout does.
  This change's sign depends on the regime: A 852 → 680 M/s (−20%), B 285 → 396 (+39%). The stock harness, and every
  two-thread figure in this ADR, measures B, because malloc hands out a page-aligned array with an untouched page
  after it. A harness that places the slot array is proposed on #51.

### The final version, master against it (2026-10-11)

Master `f2fad02` (ring code identical to `9598875`) against this PR's head, two rounds alternating in separate
processes on a quiet machine, rings at byte 0 or 64 of a 256-byte block as above. Single-thread cells also with loop
heads pinned (`-C llvm-args=-align-loops=16`), to separate the change from the alignment lottery of #53.

| two threads, M/s (r1, r2) | one line, master | one line, this | two lines, master | two lines, this |
|---|---|---|---|---|
| `push1_pop1`, 64 slots | 74, 101 | **131, 131** | 31, 31 | **116, 117** |
| `push1_pop1`, 1024 slots | 261, 266 | **403, 412** | 257, 230 | **407, 406** |
| `push1_pop1`, 16384 slots | 48, 48 | 35, 35 | 48, 48 | 35, 35 |
| `push1_popN`, 64 slots | 126, 138 | 130, 131 | 123, 124 | 121, 121 |
| `push1_popN`, 1024 slots (regime B) | 296, 297 | **402, 397** | 294, 294 | **411, 426** |
| `push1_popN`, 16384 slots | 39, 39 | 34, 35 | 39, 39 | 35, 34 |

| one thread, criterion median | master | this | master, pinned | this, pinned |
|---|---|---|---|---|
| `fill_drain` one record, 64 slots | 168, 173 ns | **149, 154** | 169, 176 | **155, 158** |
| `fill_drain` one record, 1024 slots | 2.67, 2.79 µs | **2.35, 2.39** | 2.70, 2.77 | **2.35, 2.44** |
| `fill_drain` whole lap, 64 slots | 63.9, 65.2 ns | 62.1, 62.1 | 63.5, 64.8 | 61.5, 62.3 |
| `fill_drain` whole lap, 1024 slots | 928, 951 ns | 927, 914 | 940, 939 | 923, 944 |
| `push_pop` | 3.53, 3.69 ns | 3.88, 3.91 | 3.49, 3.62 | 3.83, 3.90 |

- One-record hand-off gains 30% to 3.7x and no longer depends on the cursor layout; whole-lap is flat or better.
- The whole-lap single-thread regression of the first version is gone, pinned or not.
- `push_pop` pays about 8%, pinned or not, and that one is real: strict push-then-pop alternation makes every pop
  reload, and the observation has always moved, so the cache's compare and field write are pure overhead there. The
  same shape ADR 001 recorded for the producer cache.

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
- On x86 an empty poll costs more than before (`pop_empty` about 1.9 ns, 33–50% slower on Zen 3), and on a Zen 3 core
  the two-thread one-record hand-off is slower, in lockstep (#55). The gains this ADR measured are the M4's and a Zen 5
  core's.
- Revisit when #55 explains the Zen 3 lockstep, when a workload shows the 16384-slot `pop1` cell (#49), or when the
  suite decides how to treat loop alignment (#53).
