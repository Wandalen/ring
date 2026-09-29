# Non-Functional Requirement: Two Optimizations That Are Not, and One That Is

### Scope

**Purpose:** Record three plausible changes to the shim's counter block, measured
rather than argued, so that neither the two worthless ones nor the one worthwhile
one has to be rediscovered.

**Responsibility:** `CountingSeq`'s four `AtomicUsize` counters, the strong-only
`compare_exchange` on the trait, and what each alternative measures at.

**In Scope:** `ring_atomic/src/lib.rs:147-150`, `:241-244`;
`ring_atomic/tests/atomic_test.rs:265-266`.

**Out of Scope:** The cost of the shim *as it stands* against the production cell
is [`non_functional_requirement/001`](001_what_the_instrument_costs.md). The
cache-line placement of the counter block, and why spacing it out is worse, is
[`data_structure/001`](../data_structure/001_one_word_and_five.md) AT10. The tear
that packing would close is
[`algorithm/002`](../algorithm/002_counts_is_four_reads_not_one.md) AT3.

---

## The Counter Block, and How Much It Has to Hold

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the block, four separate words --'
command grep -m1 -A7 -F 'pub struct CountingSeq' ring_atomic/src/lib.rs
echo '  -- the exchange the trait offers, strong only --'
command grep -m1 -A3 -F '  /// The sequence actually found, when it was not `current` — the multi-producer' ring_atomic/src/lib.rs
echo '  -- the largest count any test drives a counter to --'
command grep -m1 -A2 -F '  // a passing "one operation" assertion.' ring_atomic/tests/atomic_test.rs | tail -n 2
echo '  -- against a 16-bit lane --'
printf '    4 x 10000 = %s, and a 16-bit lane holds %s\n' "$(( 4 * 10000 ))" "$(( 65536 - 1 ))"
```

Live output:

```
  -- the block, four separate words --
pub struct CountingSeq
{
  cell : AtomicSeq,
  loads : AtomicUsize,
  stores : AtomicUsize,
  fetch_adds : AtomicUsize,
  compare_exchanges : AtomicUsize,
}
  -- the exchange the trait offers, strong only --
  /// The sequence actually found, when it was not `current` — the multi-producer
  /// claim's retry input.
  fn compare_exchange( &self, current : Seq, new : Seq, success : Ordering, failure : Ordering )
  -> Result< Seq, Seq >;
  -- the largest count any test drives a counter to --
  const THREADS : usize = 4;
  const EACH : usize = 10_000;
  -- against a 16-bit lane --
    4 x 10000 = 40000, and a 16-bit lane holds 65535
```

---

## A Note on How These Were Measured

Every ratio below is a **median of nine paired ratios**: within each repetition the
two variants run back to back and the ratio is taken there, so both halves see the
same machine. The full min/max spread is printed beside each median.

This is not decoration. A first attempt took the ratio of two medians instead, and
produced 1.47× and 0.55× for the same code on two consecutive runs — the host was
running a concurrent build at load ~18 on 16 cores, and the two halves of each
comparison were drifting apart between measurements. Pairing fixes that; nothing
below reproduces unless it survives two independent runs, and where it does not, it
is reported as not reproducing.

---

### AT35 — The Weak Exchange Is Worth Nothing Here, and Padding Is Worth Less Than Nothing

`SeqCell` exposes the strong `compare_exchange` only. On aarch64 the weak variant is
allowed to fail spuriously, which lets the compiler emit a bare `ldxr`/`stxr` pair
instead of a retry loop — in a caller that already retries, as
`ring_claim::Claimer::claim` does, that is the textbook case for preferring weak.
Measured in exactly that loop shape:

```
  -- a claim-shaped retry loop, weak/strong ns per successful claim --
  paired within each rep; median of 9 ratios, with the full spread
    threads    median   min    max
          1     1.15x  0.92x  1.80x
          2     1.06x  0.91x  1.97x
          4     0.96x  0.52x  1.74x
          8     0.96x  0.69x  1.39x
         16     0.99x  0.74x  1.40x
```

A second independent run gave medians of 0.98×, 0.91×, 0.96×, 1.05×, 1.13×. Every
median sits within a few percent of 1.00, every spread straddles it, and the two
runs disagree about the sign at every thread count.

**Finding.** There is no effect to find. Adding `compare_exchange_weak` to the trait
would widen the family's central abstraction by a fourth method — one that every
future implementor must supply, and whose contract is *harder* to state correctly
than the strong one — in exchange for a difference this measurement cannot
distinguish from zero.

The same verdict, from the other direction, applies to spacing the counters onto
their own cache lines: measured in
[`data_structure/001`](../data_structure/001_one_word_and_five.md) AT10, padding is
actively worse — around 0.50× at 8 and 16 threads — because every counted operation
touches a counter *and* the cell, so separating them turns one line acquisition into
two. Both changes are the kind a reader arrives at from first principles, and both
are recorded here so the arriving is cheap next time.

---

### AT36 — Packing the Four Counters Into One Word Is Faster *and* Removes the Tear

The change that does measure is the one that looks least like an optimization:
replace the four `AtomicUsize` counters with a single `AtomicU64` carrying four
16-bit lanes. A counted call still issues exactly one counter RMW —
`fetch_add( 1 << shift )` — and `counts()` becomes one load instead of four.

```
  -- spread/packed, ns per counted operation --
  paired A/B within each rep, so both halves see the same machine;
  median of 9 per-rep ratios, with the full spread beside it
    threads    median   min    max
          2     1.02x  0.63x  1.37x
          4     2.12x  1.25x  3.22x
          8     2.61x  2.16x  3.02x
         16     2.53x  2.11x  2.83x

  -- can counts() disagree with itself while traffic runs? --
    spread   snapshots reporting advances > loads : 17
    packed   snapshots reporting advances > loads : 0
```

A second independent run: 1.09×, 1.49×, 2.55×, 2.70×, and 12 → 0 on the tear.

**Finding.** Two results, and the second is the one that matters. The speedup is
real and reproduces — around 2.5× at eight and sixteen threads, flat at two, because
four separately-contended words become one — but the shim is an instrument, not a
hot path, and 2.5× on an instrument is a convenience.

The correctness result is not a convenience. Writers here read the cell and *then*
advance it, so at any true instant `loads >= fetch_adds`; `counts()` reads `loads`
first, so a snapshot reporting more advances than loads has combined an old field
with a newer one and describes a moment that never existed. The four-word block
produced seventeen such snapshots in five hundred thousand, and twelve on the second
run. The packed block produced none, in either run, because there is nothing left to
tear: one load reads all four lanes at once. That is
[`algorithm/002`](../algorithm/002_counts_is_four_reads_not_one.md) AT3 closed by
construction rather than documented around.

**What stops it being a recommendation as measured.** A 16-bit lane holds 65,535.
The family's largest counted run is `counts_are_exact_under_contention` at
4 × 10,000 = 40,000, which fits — with one doubling of `EACH` to spare, and no
warning anywhere that the ceiling exists. So the trade is range for atomicity, and
choosing it needs a number nobody has written down: how long the longest counted run
is ever meant to be. Two lanes of 32 bits would keep the atomicity for one pair of
counters and lose it across pairs; four lanes of 16 keeps all four and caps them.
Neither is obviously right, and that is precisely why the measurement belongs on
record rather than the conclusion.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`non_functional_requirement/001`](001_what_the_instrument_costs.md) | What the shim costs as it stands, and the unit its criteria are written in |
| [`data_structure/001`](../data_structure/001_one_word_and_five.md) | The padding measurement, and why the counters share a line with the cell |
| [`algorithm/002`](../algorithm/002_counts_is_four_reads_not_one.md) | The tear the packed layout closes |
| [`data_structure/002`](../data_structure/002_opcounts_and_the_total_it_stores.md) | The other stored-versus-derived question in the same struct |
| [`api/001`](../api/001_the_return_value_that_is_a_claim.md) | The trait's method set, which the weak exchange would widen |

### Sources

| Fact | Where |
|------|-------|
| The four-word counter block | `ring_atomic/src/lib.rs:319-326` |
| The strong-only exchange | `ring_atomic/src/lib.rs:147-150` |
| The largest counted run in the family | `ring_atomic/tests/atomic_test.rs:265-266` |
| Weak/strong at five thread counts, twice | Release probe, quoted above |
| Packed/spread and the tear counts, twice | Release probe, quoted above |
| Padding measured at ~0.50× | [`data_structure/001`](../data_structure/001_one_word_and_five.md) AT10 |

### Tests

| Test | Covers |
|------|--------|
| `counts_are_exact_under_contention` | The 40,000-operation run that sets the ceiling any packed lane would have to clear |
| `each_operation_increments_exactly_its_own_counter` | The property a lane packing must preserve — that one call moves one counter |
| `total_is_the_sum_of_the_four_and_not_an_independent_counter` | The relation that becomes free once all four are read at once |
| *(to create)* | Nothing asserts an upper bound on any counter, so nothing would fail if a 16-bit lane overflowed |
