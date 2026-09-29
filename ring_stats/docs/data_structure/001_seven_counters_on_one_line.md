# Data Structure: Seven Counters on One Cache Line

### Scope

**Purpose:** Record the layout `RingStats` ships, measure what it costs under the
access pattern the family actually has, and place that measurement beside the
opposite result the same workspace already recorded.

**Responsibility:** The seven `AtomicU64` fields, their total size and alignment,
and the paired measurement of packed against padded.

**In Scope:** `ring_stats/src/lib.rs:82-92`; `ring_align/src/lib.rs:7-12`,
`:52-53`.

**Out of Scope:** Which counter each field records is
[`data_structure/002`](002_three_drop_counters_behind_one_enum.md). The cost claim
the crate makes about itself is
[`non_functional_requirement/001`](../non_functional_requirement/001_cheap_enough_to_leave_on.md).

---

## The Layout, and the Crate That Exists to Fix It

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the whole layout, seven counters and no repr --'
command grep -m1 -A10 -F '#[ derive( Debug, Default ) ]' ring_stats/src/lib.rs
echo '  -- the crate that exists for exactly this, and who depends on it --'
command grep -m1 -A5 -F '//! `docs/feature/169_padded_cursor.md` states the problem this crate exists to' ring_align/src/lib.rs
for f in */Cargo.toml; do if command grep -q '^ring_align' "$f"; then echo "    declares ring_align: ${f}"; fi; done
```

Live output:

```
  -- the whole layout, seven counters and no repr --
#[ derive( Debug, Default ) ]
pub struct RingStats
{
  claimed : AtomicU64,
  published : AtomicU64,
  consumed : AtomicU64,
  dropped_newest : AtomicU64,
  dropped_oldest : AtomicU64,
  failed : AtomicU64,
  wait_nanos : AtomicU64,
}
  -- the crate that exists for exactly this, and who depends on it --
//! `docs/feature/169_padded_cursor.md` states the problem this crate exists to
//! solve: a producer cursor and a consumer cursor that share a cache line make
//! every write by either invalidate the other's cached copy, so two cores
//! contend on a line neither is actually sharing data through. The fix is to
//! give each its own line. This crate holds the constant and the wrapper;
//! `ring_cursor` holds the cursors that use them.
    declares ring_align: ring_cursor/Cargo.toml
```

---

### ST9 — Seven Contended Counters in 56 Bytes, Measured at Roughly 2.9×

Seven `AtomicU64` and no `repr`, so the struct is 56 bytes at alignment 8 — one
cache line if it happens to land aligned, two if it straddles. Every counter is
therefore on the same line as at least five others.

The access pattern is the one `ring_align`'s own opening paragraph describes.
Producers write `claimed` and `published`; consumers write `consumed`; the overflow
path writes one of the three drop counters. No thread reads a neighbour's counter —
there is no operation in the crate that touches two fields except the three
compositions, which only load. The sharing is false in the strict sense: cores
contend on a line they are not actually sharing data through.

**Finding.** Measured against the same counters given a line each, as a median of
nine paired ratios with the two variants run back-to-back inside every repetition:

```
    size_of::< RingStats >()   56 bytes
    align_of::< RingStats >()  8 bytes
    seven AtomicU64 packed     56 bytes

    threads   packed    padded    ratio   (median of 9 paired runs)
          3     12.8ms     3.9ms   3.34×   spread 2.65×–3.94×
          6     24.6ms     8.9ms   2.80×   spread 2.35×–3.29×
         12     50.1ms    17.3ms   2.96×   spread 2.41×–3.42×

    ratio > 1 means the packed layout ring_stats ships is slower
```

A second run:

```
    size_of::< RingStats >()   56 bytes
    align_of::< RingStats >()  8 bytes
    seven AtomicU64 packed     56 bytes

    threads   packed    padded    ratio   (median of 9 paired runs)
          3     12.4ms     4.5ms   2.82×   spread 2.17×–3.41×
          6     23.9ms     9.2ms   2.63×   spread 2.14×–3.04×
         12     51.0ms    17.3ms   3.01×   spread 2.72×–3.07×

    ratio > 1 means the packed layout ring_stats ships is slower
```

Between roughly 2.6× and 3.3× at every thread count tried, with the whole spread of
eighteen paired ratios lying above 2.1×. The cost is not a tail effect and it does
not wash out with more threads — three threads, one per counter, already show it.

The remedy is in the workspace, is safe, and is one line of `use`. `ring_align`
ships `CacheAligned< T >` with `#[ repr( align( 64 ) ) ]` and no `unsafe`, and
exactly one crate in thirty-three declares it: `ring_cursor`, the crate the
`ring_align` module comment names. `ring_stats` is not a dependant, and its manifest
lists one dependency, `ring_types`.

**Disposition:** declined — `ring_align::CacheAligned< T >` is `pub struct
CacheAligned< T >( T )` at `ring_align/src/lib.rs:69`, one field per line; wrapping
all seven `AtomicU64` fields in it grows `RingStats` from the measured 56 bytes to
roughly 448 bytes per ring, an 8x per-instance memory tradeoff against the crate's
own "cheap enough to leave on" claim in
`non_functional_requirement/001_cheap_enough_to_leave_on.md` — a design decision
for a shared public type's layout, not a documentation correction, so it is left
for an explicit follow-up rather than applied silently here.

---

### ST10 — The Family Has Measured This Twice, With Opposite Answers, and Neither Struct Says Which

`ring_atomic` ran the same experiment on `CountingSeq` and got the reverse result:
padding its four counters onto separate lines was **2× slower** — 0.50× and 0.49× at
8 and 16 threads — because every operation there touches its counter *and* the
sequence cell beside it, so the packing is buying real locality
([`ring_atomic` § AT10](../../../ring_atomic/docs/data_structure/001_one_word_and_five.md)).

Two structs, one workspace, same author, same conventions, same `ring_align`
available to both. In one, packing is worth 2×; in the other, it costs 2.9×. The
distinguishing property is not size, field count, or atomic type — it is whether the
fields sharing the line are touched by the same operation or by different threads.

**Finding.** Neither struct records which regime it is in. `CountingSeq` has no
comment saying its packing is deliberate and load-bearing, and `RingStats` has none
saying its packing is incidental and costly. A reader arriving at either one finds
seven (or five) atomics in a struct with no `repr`, and the two most obvious readings
— "pad these, it's a known concurrency fix" and "leave these, the family packs
counters" — are each correct for exactly one of them and a regression for the other.

The measurements exist now; what is missing is one sentence per struct naming the
regime. On `RingStats` that sentence also has to note it is describing a cost that
has not been paid down, not a decision that was made.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`data_structure/002`](002_three_drop_counters_behind_one_enum.md) | What the seven fields are, and why three of them are one concept |
| [`non_functional_requirement/001`](../non_functional_requirement/001_cheap_enough_to_leave_on.md) | The crate's own cost claim, against this measurement |
| [`decisions/001`](../decisions/001_relaxed_with_a_reason_that_covers_one_load.md) | The one performance argument the crate does make, and its scope |
| [`algorithm/001`](../algorithm/001_eleven_operations_and_three_compositions.md) | Every operation that lands in this layout |

### Sources

| Fact | Where |
|------|-------|
| Seven `AtomicU64`, no `repr` | `ring_stats/src/lib.rs:82-92` |
| 56 bytes at alignment 8 | Probe, quoted above |
| 2.6×–3.3× packed against padded | Probe, two runs quoted above |
| `ring_align` exists, is safe, and states this exact problem | `ring_align/src/lib.rs:7-21`, `:52-53` |
| Only `ring_cursor` declares it | Census above |
| `ring_stats` declares only `ring_types` | `ring_stats/Cargo.toml` |
| The opposite result on `CountingSeq` | `ring_atomic` § AT10 |

### Tests

| Test | Covers |
|------|--------|
| `counts_are_exact_under_contention` | That contention does not lose counts — not what it costs |
| `distinct_policy_counters_do_not_interfere_under_contention` | That neighbouring counters stay independent, which is the correctness half of false sharing |
| *(to create)* | Nothing asserts anything about layout: no `size_of`, no `align_of`, no cost bound |
