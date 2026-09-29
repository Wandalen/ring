# Non-Functional Requirement: What the Instrument Costs, and the Unit Nobody Measures

### Scope

**Purpose:** Record what the counting shim costs relative to the cell it wraps, what
the two criteria it exists to serve actually ask for, and the gap between the unit
they are written in and the unit the shim reports.

**Responsibility:** `CountingSeq`'s per-call overhead, the two acceptance criteria as
stated, and what a 64-slot claim emits on this target.

**In Scope:** `ring_atomic/src/lib.rs:223-226`, `:488-492`;
`bench_harness/docs/acceptance/001_feature_reached_tests.md:43`, `:45`.

**Out of Scope:** The cost of the counters' *placement* — five atomics on one cache
line — is [`data_structure/001`](../data_structure/001_one_word_and_five.md). Two
changes that measured as non-improvements are
[`non_functional_requirement/002`](002_the_optimizations_that_are_not.md). That a
`counts()` call reads four separate moments is
[`algorithm/002`](../algorithm/002_counts_is_four_reads_not_one.md).

---

## The Criteria, the Budget, and the Shim

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the two criteria this crate exists to serve, both stated as counts --'
sed -n '/^| 175 | Thread-local buffer and flush-into | `ring_tls` | S2 | A `TlsBuffer` accumulates `N` items with zero atomic operations (asserted by a counting allocator\/atomic shim), and one `flush_into` moves all `N` into the ring as a single contiguous claim | `ring_tls\/tests\/tls_test\.rs` |$/p;/^| 177 | Batch claim and batch drain | `ring_batch` | S2 | A claim of 64 slots issues one fence, not 64 (asserted against a counting ordering shim); the 64 sequences returned are contiguous; a batch drain reads them in issue order | `ring_batch\/tests\/batch_test\.rs` |$/p' bench_harness/docs/acceptance/001_feature_reached_tests.md | cut -c1-140
echo '  -- every latency, throughput, or budget figure in the crate --'
command grep -rcE '[0-9]+ *(ns|us|ms)\b|nanosecond|per second|throughput|budget|latency' \
  ring_atomic/src/lib.rs ring_atomic/readme.md ring_atomic/tests/atomic_test.rs
echo '  -- what one counted call issues, against the one it wraps --'
awk '/^    self\.0\.store\( value\.0, order \);$/{ n1 = NR } n1 && NR >= n1 + 3 && NR <= n1 + 6 { print } /^    self\.cell\.store\( value, order \);$/{ n2 = NR } n2 && NR >= n2 + 3 && NR <= n2 + 7 { print }' ring_atomic/src/lib.rs
```

Live output:

```
  -- the two criteria this crate exists to serve, both stated as counts --
| 175 | Thread-local buffer and flush-into | `ring_tls` | S2 | A `TlsBuffer` accumulates `N` items with zero atomic operations (asserted by 
| 177 | Batch claim and batch drain | `ring_batch` | S2 | A claim of 64 slots issues one fence, not 64 (asserted against a counting ordering
  -- every latency, throughput, or budget figure in the crate --
ring_atomic/src/lib.rs:0
ring_atomic/readme.md:0
ring_atomic/tests/atomic_test.rs:0
  -- what one counted call issues, against the one it wraps --
  fn fetch_add( &self, n : u64, order : Ordering ) -> Seq
  {
    Seq( self.0.fetch_add( n, order ) )
  }
  fn fetch_add( &self, n : u64, order : Ordering ) -> Seq
  {
    self.fetch_adds.fetch_add( 1, Ordering::Relaxed );
    self.cell.fetch_add( n, order )
  }
```

---

### AT33 — Counting a Load Costs More Than Loading

Every counted call is two hardware atomics where the production call is one: the
counter's `fetch_add`, then the cell's own operation. Measured on this target as a
median of nine **paired** ratios — the two variants run back to back inside each
repetition, so both halves see the same machine, which on a host under concurrent
build load is the difference between a reproducible number and a coin flip
([`non_functional_requirement/002`](002_the_optimizations_that_are_not.md)):

```
  -- CountingSeq / AtomicSeq, ns per logical operation --
  paired within each rep; median of 9 ratios, with the full spread
                            median   min    max
    fetch_add, 1 thread       1.91x  1.69x  2.03x
    load, 1 thread            2.67x  2.50x  2.77x
    fetch_add, 8 threads      1.70x  1.39x  1.86x

  -- hardware atomics issued per logical operation --
    AtomicSeq::fetch_add     1   the cell
    CountingSeq::fetch_add   2   the counter, then the cell
    AtomicSeq::load          1   the cell
    CountingSeq::load        2   the counter, then the cell
    CountingSeq::counts      4   four Relaxed loads, then a sum
```

A second independent run gave 1.84×, 2.70×, 1.75× — the spreads are narrow and the
ordering is stable.

**Finding.** The load is where the instrument distorts most, and by more than the
doubled-atomic-count model predicts. `fetch_add` costs about 1.9×: it was already a
read-modify-write, so counting it adds a second one of the same kind. A `load` costs
about 2.7×, because acquire on aarch64 is a load instruction rather than a barrier —
so the production `load` is nearly free, and counting it replaces "nearly free" with
a full RMW. Doubling the atomic *count* does not double the cost in either case; it
matters what kind of atomic is being doubled.

The practical consequence is for anyone reading a `CountingSeq` benchmark as a
performance figure: a read-weighted workload is penalised noticeably harder than a
claim-weighted one, and that difference is invisible in the counts the shim reports —
it reports 1 either way.

This is not an argument that the shim is wrong. It is a correctness instrument, not
a timing one, and the crate never claims otherwise. It is an argument that the
distinction is nowhere written: there are zero latency, throughput, or budget
figures in the source, the readme, or the test file, so nothing in the crate marks
which of its two cells is the one you may time.

The doc comment now states the cost inline, immediately after the same "Not a
mock" sentence quoted above:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A3 -F '/// Not a mock: the underlying operations are the same real atomics, so a test' ring_atomic/src/lib.rs
```

Live output:

```
/// Not a mock: the underlying operations are the same real atomics, so a test
/// running against this observes the same values production would — at
/// roughly double the cost per operation, so this is a correctness instrument
/// and not a timing one. Only the bookkeeping is added — which is why an
```

**Disposition:** applied — `CountingSeq`'s doc comment in `src/lib.rs` now
states the doubled cost inline, and the crate's 21 unit tests plus 8 doctests
re-verified passing (`cargo test --all-features`, 2026-09-03). Now prints:
`roughly double the cost per operation, so this is a correctness instrument`

---

### AT34 — The `ring_batch` Criterion Asks for One Fence; the Claim Emits Zero, and the Shim Counts Neither

The `ring_batch` criterion is stated as "A claim of 64 slots issues one fence, not 64 (asserted
against a counting ordering shim)". The intent is unmistakable and correct: the cost
of claiming a batch must not scale with the batch. What the sentence asks for
literally is a count of *fences*, and that is not what happens or what is asserted.

The whole body of a 64-slot claim on this target:

```
836c:	a9bf7bfd 	stp	x29, x30, [sp, #-16]!
8370:	910003fd 	mov	x29, sp
8374:	aa0003e1 	mov	x1, x0
8378:	52800800 	mov	w0, #0x40                  	// #64
837c:	9400c855 	bl	3a4d0 <__aarch64_ldadd8_acq_rel>
8380:	9100fc08 	add	x8, x0, #0x3f
8384:	b101001f 	cmn	x0, #0x40
8388:	9a9f3100 	csel	x0, x8, xzr, cc	// cc = lo, ul, last
838c:	a8c17bfd 	ldp	x29, x30, [sp], #16
8390:	d65f03c0 	ret
```

Nine instructions and one call. The helper it calls is an LSE `ldaddal` when the
CPU supports it, and an `ldaxr`/`stlxr` retry loop otherwise:

```
3a4d0:	d0000130 	adrp	x16, 60000 <__dso_handle>
3a4d4:	396a3210 	ldrb	w16, [x16, #2700]
3a4d8:	34000070 	cbz	w16, 3a4e4 <__aarch64_ldadd8_acq_rel+0x14>
3a4dc:	f8e00020 	ldaddal	x0, x0, [x1]
3a4e0:	d65f03c0 	ret
3a4e4:	aa0003f0 	mov	x16, x0
3a4e8:	c85ffc20 	ldaxr	x0, [x1]
3a4ec:	8b100011 	add	x17, x0, x16
3a4f0:	c80ffc31 	stlxr	w15, x17, [x1]
3a4f4:	35ffffaf 	cbnz	w15, 3a4e8 <__aarch64_ldadd8_acq_rel+0x18>
3a4f8:	d65f03c0 	ret
```

Not one `dmb`, `dsb`, or `isb` on either path. Acquire-release on aarch64 is
expressed by the *addressing* of the atomic instructions themselves, so the
criterion's "one fence" is, on this target, zero fences.

**Finding.** The criterion is written in a unit that does not exist on the hardware
it runs on, and is asserted with a shim that measures a third thing again — calls.
Three units in one sentence: fences (what it says), atomic instructions (what the
CPU does — two per counted call, one per production call), and calls (what
`counts()` returns).

The assertion is still the right assertion. `fetch_adds == 1` after a 64-slot claim
does establish that batch cost is constant in batch size, which is the property
worth having, and the disassembly above confirms it directly. The defect is purely
in the wording: a reader auditing "one fence, not 64" against `counts().fetch_adds
== 1` has to reconstruct, unaided, that the two are talking about different things
and that the substitution happens to be sound. Restating the criterion as "one
atomic operation, not 64" would cost four words and remove the reconstruction
entirely.

**Disposition:** declined — restating the `ring_batch` criterion's own acceptance
wording ("one fence, not 64" → "one atomic operation, not 64") means editing
`bench_harness/docs/acceptance/001_feature_reached_tests.md:45`, the
same out-of-scope bench_harness acceptance-criteria fix target `decisions/002`
AT16 already declined; not this crate's own doc to change in a corpus
disposition pass.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`non_functional_requirement/002`](002_the_optimizations_that_are_not.md) | Two plausible changes that measured worse or flat |
| [`data_structure/001`](../data_structure/001_one_word_and_five.md) | Where the counters sit, and why spacing them is slower |
| [`decisions/002`](../decisions/002_a_trait_because_the_criteria_needed_two.md) | Why the trait exists, and the two criteria it was built for |
| [`algorithm/001`](../algorithm/001_one_intrinsic_or_two.md) | The one-versus-two intrinsic structure this measures |
| [`item/002`](../item/002_counts_the_method_that_is_not_a_snapshot.md) | Why only generic code can be instrumented at all |

### Sources

| Fact | Where |
|------|-------|
| The `ring_tls` and `ring_batch` criteria as written | `bench_harness/docs/acceptance/001_feature_reached_tests.md:43`, `:45` |
| The one-intrinsic and two-intrinsic bodies | `ring_atomic/src/lib.rs:223-226`, `:488-492` |
| Zero cost figures anywhere in the crate | Census above |
| 1.91× / 2.67× / 1.70× | Release probe, median of nine paired ratios, quoted above |
| Nine instructions, one atomic, zero fences | `objdump -d --disassemble=claim_64`, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `one_fetch_add_buys_a_whole_batch` | The property the `ring_batch` criterion is actually about, asserted as a call count |
| `the_counting_cell_is_the_production_cell_plus_bookkeeping` | That the shim's sequence behaviour matches the production cell's, which is what makes the substitution sound |
| `each_operation_increments_exactly_its_own_counter` | That a counted call increments one counter, not that it issues two atomics |
| *(to create)* | Nothing asserts a cost, because the crate states no budget to assert against |
