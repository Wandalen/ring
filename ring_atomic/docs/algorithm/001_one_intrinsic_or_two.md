# Algorithm: One Intrinsic, or Two

### Scope

**Purpose:** Record what each of the four `SeqCell` operations actually issues to
the hardware in each of the crate's two implementations, and what the difference
costs.

**Responsibility:** `impl SeqCell for AtomicSeq` and `impl SeqCell for
CountingSeq` — their bodies, the instructions they emit, and the measured cost of
each.

**In Scope:** `ring_atomic/src/lib.rs:211-236`, `:474-500`, `:321`.

**Out of Scope:** What `counts()` does with the four counters afterwards is
[`algorithm/002`](002_counts_is_four_reads_not_one.md). The orderings themselves
are [`decisions/001`](../decisions/001_orderings_named_never_defaulted.md).

---

## Two Bodies for the Same Four Operations

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the production cell: every method one intrinsic --'
awk '/^impl SeqCell for AtomicSeq$/{ n1 = NR } n1 && NR >= n1 + 2 && NR <= n1 + 5 { print } /^    self\.0\.store\( value\.0, order \);$/{ n2 = NR } n2 && NR >= n2 + 3 && NR <= n2 + 6 { print }' ring_atomic/src/lib.rs
echo '  -- the counting cell: every method two --'
awk '/^impl SeqCell for CountingSeq$/{ n1 = NR } n1 && NR >= n1 + 2 && NR <= n1 + 6 { print } /^    self\.cell\.store\( value, order \);$/{ n2 = NR } n2 && NR >= n2 + 3 && NR <= n2 + 7 { print }' ring_atomic/src/lib.rs
echo '  -- and the field the second one wraps --'
command grep -m1 -F '  cell : AtomicSeq,' ring_atomic/src/lib.rs
```

Live output:

```
  -- the production cell: every method one intrinsic --
  fn load( &self, order : Ordering ) -> Seq
  {
    Seq( self.0.load( order ) )
  }
  fn fetch_add( &self, n : u64, order : Ordering ) -> Seq
  {
    Seq( self.0.fetch_add( n, order ) )
  }
  -- the counting cell: every method two --
  fn load( &self, order : Ordering ) -> Seq
  {
    self.loads.fetch_add( 1, Ordering::Relaxed );
    self.cell.load( order )
  }
  fn fetch_add( &self, n : u64, order : Ordering ) -> Seq
  {
    self.fetch_adds.fetch_add( 1, Ordering::Relaxed );
    self.cell.fetch_add( n, order )
  }
  -- and the field the second one wraps --
  cell : AtomicSeq,
```

`CountingSeq` does not reimplement anything. It holds an `AtomicSeq`, increments
a counter, and delegates — which is what makes the counts trustworthy and what
makes them expensive.

---

### AT1 — The Production Cell Adds Nothing but a Newtype

Every one of `AtomicSeq`'s four methods is the corresponding `AtomicU64`
intrinsic with `Seq` wrapped around the result. No branch, no assertion, no
fallback, no ordering chosen on the caller's behalf. `compare_exchange` is the
longest at six lines and three of those are `.map( Seq )`, `.map_err( Seq )`, and
a closing brace.

**Finding.** The crate's production path is a rename. That is the whole design:
`AtomicSeq` exists so the family has exactly one place where an atomic is
*created* ([`pattern/001`](../pattern/001_one_place_where_an_atomic_is_created.md)),
not so it can do anything an `AtomicU64` could not.

It follows that the crate has no algorithm of its own to get wrong on the
production path, and no way to be slower than a bare `AtomicU64` — which the
measurement below confirms, and which is why every finding in this corpus about a
cost is about the *other* implementation.

---

### AT2 — The Instrument Doubles the Traffic It Exists to Measure

`CountingSeq` was built for two acceptance criteria that are negative claims
about atomic traffic — "accumulates N items with zero atomic operations" and "a
claim of 64 slots issues one fence, not 64". Its own bookkeeping is atomic, so
each logical operation issues two:

```
--- one thread, ns per logical operation ---
               AtomicSeq  CountingSeq   ratio
  fetch_add        4.83         9.17    1.90x
  load             1.70         5.80    3.42x
--- 8 threads on one cell, ns per logical operation ---
  AtomicSeq       10.85
  CountingSeq     19.76
--- hardware atomics issued per logical operation ---
  AtomicSeq::fetch_add     1  (the cell)
  CountingSeq::fetch_add   2  (the counter, then the cell)
```

**Finding.** The reported count is right and the traffic is not. A 64-slot claim
through `CountingSeq` still reports `fetch_adds == 1`, which is the assertion the
criterion needs — but the machine saw two atomic read-modify-writes, so the
number the shim certifies is a count of *logical* operations, never of fences.
Nothing in the crate says so, and both criteria are phrased in terms of fences.

`load` is the worse of the two at 3.42×, and for a reason that is not about
volume. `AtomicSeq::load` is a pure read: on a shared cache line it can complete
without invalidating any other core's copy. `CountingSeq::load` prefixes it with a
`fetch_add` on the counter — a read-modify-write, which must take the line
exclusively. The shim does not just add an operation to a read; it changes the
read into a write, which is exactly the traffic class the criteria are counting.

Under contention the ratio settles at 1.82× because both implementations are then
dominated by the same contended cache line. That is the measurement that matters
for the criteria's own scenario, and it is the one nothing records.

**Disposition:** declined — this is the same doubled-atomic-traffic measurement
as `non_functional_requirement/001` AT33 (1.9×–3.4×); the wording gap the
doubling exposes is the `ring_batch` criterion's own acceptance wording in
`bench_harness/docs/acceptance/001_feature_reached_tests.md:45`, which
`decisions/002` AT16 already names as the fix target — not this crate's own
doc.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`algorithm/002`](002_counts_is_four_reads_not_one.md) | What happens to the counters these two lines increment |
| [`pattern/002`](../pattern/002_the_counting_cell_is_not_a_mock.md) | Why the delegation is what makes the counts mean anything |
| [`non_functional_requirement/001`](../non_functional_requirement/001_what_the_instrument_costs.md) | The same measurement, read as a cost rather than a mechanism |
| [`data_structure/001`](../data_structure/001_one_word_and_five.md) | The five words the counting cell holds, and where they sit |

### Sources

| Fact | Where |
|------|-------|
| The production impl | `ring_atomic/src/lib.rs:211-236` |
| The counting impl | `ring_atomic/src/lib.rs:474-500` |
| The wrapped field | `ring_atomic/src/lib.rs:321` |
| The two acceptance criteria | `ring_atomic/src/lib.rs:25-33` |
| Per-operation costs and atomic counts | Release probe, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `the_counting_cell_is_the_production_cell_plus_bookkeeping` | That the two agree on every value, which is what makes the delegation sound |
| `each_operation_increments_exactly_its_own_counter` | That the counter incremented is the right one |
| *(to create)* | Nothing measures or asserts the shim's own atomic traffic, which is the quantity both criteria are phrased in |
