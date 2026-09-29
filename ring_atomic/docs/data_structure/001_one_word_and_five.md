# Data Structure: One Word, and Five on One Line

### Scope

**Purpose:** Record what each of the crate's two cells occupies in memory, how
those words are laid out relative to a cache line, and what that layout costs or
saves.

**Responsibility:** `AtomicSeq`'s single field, `CountingSeq`'s five, their
measured sizes and alignment, and the contention behaviour that follows.

**In Scope:** `ring_atomic/src/lib.rs:165-166`, `:319-326`.

**Out of Scope:** `OpCounts`, which is a report rather than a cell, is
[`data_structure/002`](002_opcounts_and_the_total_it_stores.md). The per-operation
instruction cost is [`algorithm/001`](../algorithm/001_one_intrinsic_or_two.md) AT2.

---

## Two Declarations, No Alignment Attribute

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the two cells, declared --'
command grep -m1 -B1 -F 'pub struct AtomicSeq( AtomicU64 );' ring_atomic/src/lib.rs
command grep -m1 -A7 -F 'pub struct CountingSeq' ring_atomic/src/lib.rs
echo '  -- alignment attributes in the whole crate --'
echo "    repr(...) occurrences : $( command grep -c 'repr' ring_atomic/src/lib.rs || true )"
echo '  -- while one tier up the same problem is solved by name --'
command grep -m1 -A1 -F '#[ derive( Debug, Default ) ]' ring_cursor/src/lib.rs
command grep -m1 -A2 -F '#[ derive( Debug, Clone, Copy, Default, PartialEq, Eq ) ]' ring_align/src/lib.rs
```

Live output:

```
  -- the two cells, declared --
#[ derive( Debug ) ]
pub struct AtomicSeq( AtomicU64 );
pub struct CountingSeq
{
  cell : AtomicSeq,
  loads : AtomicUsize,
  stores : AtomicUsize,
  fetch_adds : AtomicUsize,
  compare_exchanges : AtomicUsize,
}
  -- alignment attributes in the whole crate --
    repr(...) occurrences : 0
  -- while one tier up the same problem is solved by name --
#[ derive( Debug, Default ) ]
pub struct PaddedCursor( CacheAligned< AtomicSeq > );
#[ derive( Debug, Clone, Copy, Default, PartialEq, Eq ) ]
#[ repr( align( 64 ) ) ]
pub struct CacheAligned< T >( T );
```

Measured:

```
  AtomicSeq                    size   8  align   8
  CountingSeq                  size  40  align   8
  OpCounts                     size  40  align   8
  ring_cursor::PaddedCursor    size  64  align  64

  -- a CountingSeq spans 40 bytes from one base --
  base address mod 64                        : 0
  first and last byte on the same cache line : true
```

---

### AT9 — Five Contended Atomics on One Cache Line, in the Family That Has a Type for Not Doing That

`CountingSeq` is 40 bytes at alignment 8: the cell plus four counters, packed, with
nothing between them. Every one of the five is written by every thread that uses
the cell — the counter on every operation, the cell on every operation — so all
five sit in one coherence unit and every operation by every thread invalidates the
whole thing for every other core.

The family knows this problem by name. `ring_align` exists for it and exports
`CacheAligned< T >` at `#[ repr( align( 64 ) ) ]`; `ring_cursor` wraps its
production cursor in it, taking a cursor from 8 bytes to 64 to keep the producer's
line away from the consumer's. That is one dependency edge away, and this crate is
already `ring_align`'s sibling at the same tier.

**Finding.** The crate whose entire purpose is to own the family's atomic
primitives is the one crate that packs five contended atomics into 40 unaligned
bytes, in a workspace that has a dedicated type for the opposite. Nothing in the
crate mentions the layout, argues for it, or notes that it differs from the
convention two of its own dependants follow.

---

### AT10 — And the Packed Layout Is the Faster One, for a Reason Nothing Records

The obvious remedy — one counter per cache line — makes it worse, not better, and
by a wide margin. The same shim with each counter `#[ repr( align( 64 ) ) ]`,
driven by threads split between advancing and reading so that two different
counters are hot:

```
  -- packed(40B) / spaced(4 lines), ns per operation --
  half the threads advancing and half reading; paired within each rep,
  median of 9 ratios, with the full spread beside it
    threads    median   min    max
          2     1.21x  0.58x  2.14x
          4     0.65x  0.50x  1.10x
          8     0.48x  0.39x  0.59x
         16     0.47x  0.43x  0.55x
```

A second independent run gave 1.31×, 0.54×, 0.49×, 0.54×. Padding wins at two
threads and loses by roughly 2× at eight and sixteen, and both ends reproduce.
Each ratio is taken from a paired A/B measurement inside one repetition so that
both halves see the same machine. That pairing is what makes the number
reproducible: on this host under concurrent build load, a sibling probe comparing
two separately-taken medians of the same code swung between 1.47× and 0.55× on
consecutive runs
([`non_functional_requirement/002`](../non_functional_requirement/002_the_optimizations_that_are_not.md)).

**Finding.** The crossover has a cause, and it is the thing that makes this shim
different from a cursor. A cursor's producer and consumer touch *different* words,
so keeping them apart is pure gain. Here every operation touches its counter **and**
the cell — always both, never one — so separating them means acquiring two lines
per operation instead of one. Past four threads the extra line acquisition costs
more than the false sharing it avoids.

So the layout is right, and it is right for a reason the crate never states. There
is no comment, no doc sentence, and no test recording that the packing is
deliberate. A reader who knows `ring_align` exists, sees five contended atomics
without it, and "fixes" the omission would halve the shim's throughput at the
thread counts the benchmarks actually use — and every signal available to them
says they were correcting an oversight.

This is the second measured non-problem in this crate, alongside the weak
`compare_exchange` variant ([`non_functional_requirement/002`](../non_functional_requirement/002_the_optimizations_that_are_not.md)).
Both are cheap to record and expensive to rediscover.

The doc comment now states the packing is deliberate:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A3 -F '/// The four counters are packed against the cell with no padding between' ring_atomic/src/lib.rs
```

Live output:

```
/// The four counters are packed against the cell with no padding between
/// them, deliberately: every operation touches its own counter *and* the
/// cell, so separating them onto their own cache lines pays for an extra
/// line acquisition on every call rather than avoiding one, and measures
```

**Disposition:** applied — `CountingSeq`'s struct doc in `src/lib.rs` now
states the packed layout is deliberate and names the crossover past four
threads; the crate's 21 unit tests plus 8 doctests re-verified passing
(`cargo test --all-features`, 2026-09-03). Now prints:
`The four counters are packed against the cell with no padding between`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`data_structure/002`](002_opcounts_and_the_total_it_stores.md) | The 40 bytes the counters are read *into*, and the field that need not agree |
| [`non_functional_requirement/002`](../non_functional_requirement/002_the_optimizations_that_are_not.md) | The other measured non-problem, and why both are worth writing down |
| [`algorithm/001`](../algorithm/001_one_intrinsic_or_two.md) | The two atomics per operation this layout determines the cost of |
| [`integration/002`](../integration/002_five_crates_downstream.md) | `ring_cursor`, which pads, and the four other dependants that do not |

### Sources

| Fact | Where |
|------|-------|
| `AtomicSeq`'s single field | `ring_atomic/src/lib.rs:165-166` |
| `CountingSeq`'s five fields | `ring_atomic/src/lib.rs:319-326` |
| No `repr` anywhere in the crate | Census above |
| `CacheAligned` and `PaddedCursor` | `ring_align/src/lib.rs:67-69`; `ring_cursor/src/lib.rs:143-144` |
| Sizes, alignment, and the one-line span | Release probe, quoted above |
| Packed vs spaced at four thread counts | Release probe, medians of 5, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `the_counting_cell_is_the_production_cell_plus_bookkeeping` | That the shim's values match the production cell — nothing about its layout |
| *(to create)* | Nothing asserts `size_of::< CountingSeq >()`, so the packing no test depends on is also the packing no test protects |
