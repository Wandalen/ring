# Invariant: Every Increment Survives, and Nothing Relates the Two Numbers

### Scope

**Purpose:** Record the properties the crate's two concurrency tests establish,
what those properties actually rest on, and the one relation between the shim's two
atomics that no test touches.

**Responsibility:** `concurrent_fetch_adds_partition_the_sequence_space`,
`counts_are_exact_under_contention`, and the bodies they run against.

**In Scope:** `ring_atomic/tests/atomic_test.rs:103-132`, `:274-302`;
`ring_atomic/src/lib.rs:223-226`, `:488-492`.

**Out of Scope:** The tear inside a single `counts()` call is
[`algorithm/002`](../algorithm/002_counts_is_four_reads_not_one.md) AT3. What the
crate does not require of a caller is
[`invariant/001`](001_monotonicity_is_relied_on_and_not_required.md).

---

## Two Tests, Sixty Thousand Operations

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the property the disjointness test asserts --'
command grep -m1 -A5 -F '  let mut sorted = claimed;' ring_atomic/tests/atomic_test.rs
echo '  -- against the implementation it asserts it of --'
command grep -m1 -A6 -F '    self.0.store( value.0, order );' ring_atomic/src/lib.rs | tail -n 4
echo '  -- and the counting test, asserting its two numbers separately --'
command grep -m1 -A1 -F '  assert_eq!( cell.counts().fetch_adds, THREADS * EACH );' ring_atomic/tests/atomic_test.rs
echo '  -- from a body that increments the counter first --'
command grep -m1 -A7 -F '    self.cell.store( value, order );' ring_atomic/src/lib.rs | tail -n 5
echo '  -- and the ordering decision, now stated above the impl --'
command grep -m1 -A4 -F '/// Each method bumps its counter **before** delegating to the cell.' ring_atomic/src/lib.rs
```

Live output:

```
  -- the property the disjointness test asserts --
  let mut sorted = claimed;
  sorted.sort_unstable();
  let expected : Vec< u64 > = ( 0..( THREADS * EACH ) as u64 ).collect();
  assert_eq!( sorted, expected, "every sequence issued exactly once" );
  assert_eq!( cell.load( Ordering::Acquire ), Seq( ( THREADS * EACH ) as u64 ) );
}
  -- against the implementation it asserts it of --
  fn fetch_add( &self, n : u64, order : Ordering ) -> Seq
  {
    Seq( self.0.fetch_add( n, order ) )
  }
  -- and the counting test, asserting its two numbers separately --
  assert_eq!( cell.counts().fetch_adds, THREADS * EACH );
  assert_eq!( cell.load( Ordering::Acquire ), Seq( ( THREADS * EACH ) as u64 ) );
  -- from a body that increments the counter first --
  fn fetch_add( &self, n : u64, order : Ordering ) -> Seq
  {
    self.fetch_adds.fetch_add( 1, Ordering::Relaxed );
    self.cell.fetch_add( n, order )
  }
  -- and the ordering decision, now stated above the impl --
/// Each method bumps its counter **before** delegating to the cell.
///
/// Fix(AT24): the order is deliberate and was undocumented. A concurrent
/// observer reading the counter and the cell while both are moving can
/// therefore see a counter that has already been incremented for an operation
```

---

### AT23 — Both Concurrency Tests Assert a Property of `AtomicU64` Through a One-Line Delegation

`concurrent_fetch_adds_partition_the_sequence_space` runs four threads for 5,000
`fetch_add`s each, collects all 20,000 returned values, sorts them, and asserts the
result is exactly `0..20000`. That is disjointness *and* completeness, and it is a
genuinely strong shape of assertion — it would catch a lost update, a duplicate, or
an off-by-one in the returned value.

It cannot catch any of them here, because the implementation it runs against is
`Seq( self.0.fetch_add( n, order ) )`. Every property the test asserts is a
property of `core::sync::atomic::AtomicU64`, guaranteed by the language. The only
crate-owned behaviour in the path is the newtype wrapper, and
`fetch_add_returns_the_value_before_the_advance` establishes that single-threaded
in six lines.

`counts_are_exact_under_contention` has the same shape one layer out: 40,000
operations, and both of the atomics involved are `fetch_add`s, neither of which can
lose an increment.

**Finding.** Sixty thousand operations across two tests, and neither can fail
unless the standard library is wrong. This is the same shape found in `ring_batch`
one tier up — a disjointness test asserting the property that `fetch_add` makes
free ([`ring_batch` § BA22](../../../ring_batch/docs/invariant/001_disjointness_is_free.md))
— and here it is more direct still, because there is no claim arithmetic between
the test and the intrinsic.

That is not an argument for deleting them. Both are cheap, both would catch a
future implementor that got the trait wrong, and
`counts_are_exact_under_contention`'s own comment gives a real reason ("a shim that
under-counted would turn a real regression into a passing 'one operation'
assertion"). It is an argument for not reading them as coverage of the shim's
concurrency behaviour, which is what their names and their thread counts suggest.

---

### AT24 — The Relation Between the Two Atomics Is the One Thing That Can Break, and Nothing Asserts It

`counts_are_exact_under_contention` ends with two assertions, and they are two
assertions rather than one: the counter equals 40,000, and the cell equals 40,000.
Both are taken after every thread has joined, when nothing is in motion. Nothing in
the suite reads the counter and the cell while both are moving.

The shim's body makes a specific promise about that case. It increments the counter
*first* and then delegates, so at any instant the counter is ahead of or level with
the cell's advance count and never behind. Measured across a million paired reads
with four writers running:

```
  -- 1,000,000 paired reads of counts().fetch_adds and load(), 4 writers --
    counter ahead of cell : 948121
    counter behind cell   : 7176
    the two agreeing      : 44703
    widest gap observed   : 103
```

**Finding.** 7,176 of 1,000,000 observations had the cell ahead of the counter — a
state the shim's own body makes unreachable at any instant. The reason is the same
as AT3's, one level out: the reader's `counts()` and `load()` are two calls, so
they see two moments, and the count-then-delegate ordering inside `fetch_add`
guarantees nothing across them.

The ordering inside the body is a real decision with a real consequence — reverse
the two lines and the bias reverses — and it was undocumented. A caller who reasons
from the body (correctly) to "the counter can never lag the cell" and writes an
assertion on it gets a test that fails roughly seven times in ten thousand, and the
only place that reasoning was written down was the body itself.

**Disposition:** applied — the `impl SeqCell for CountingSeq` block in `src/lib.rs`
now carries a doc comment above it stating the ordering as a decision rather than
leaving it to be read off four method bodies, naming what it buys (at any instant
the counter is ahead of or level with the cell, never behind), and carrying a
`Fix(AT24)` line with this entry's own measurement — 7,176 of 1,000,000 paired
reads seeing the cell ahead, widest gap 103 — so the number that contradicts the
naive reading sits next to the code that invites it. The ordering itself was **not**
changed: count-then-delegate is the correct choice, because the opposite ordering
loses an increment outright if a panic unwinds between the two lines, whereas this
one only over-counts. What this does not buy: the doc explains why a cross-call
assertion is unsound, it does not prevent one. A caller can still write
`assert!( cell.counts().fetch_adds >= observed_advance )` against a live writer and
get the flake this entry measures; the two-moments problem is the same one
[`algorithm/002`](../algorithm/002_counts_is_four_reads_not_one.md) AT3 documents
inside a single method, and is equally unfixable without changing what
`CountingSeq` is. Now prints: `Each method bumps its counter **before** delegating to the cell.`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`algorithm/002`](../algorithm/002_counts_is_four_reads_not_one.md) | The same two-moments problem inside one method rather than across two |
| [`invariant/001`](001_monotonicity_is_relied_on_and_not_required.md) | The property the family relies on that this crate does not require |
| [`pattern/002`](../pattern/002_the_counting_cell_is_not_a_mock.md) | Why delegation makes the counts trustworthy, and where the trust stops |
| [`item/002`](../item/002_counts_the_method_that_is_not_a_snapshot.md) | `counts`' contract, read against what a caller does with it |

### Sources

| Fact | Where |
|------|-------|
| The disjointness assertions | `ring_atomic/tests/atomic_test.rs:127-131` |
| The delegation they run against | `ring_atomic/src/lib.rs:223-226` |
| The counting test's two separate assertions | `ring_atomic/tests/atomic_test.rs:300-301` |
| Counter incremented before the cell | `ring_atomic/src/lib.rs:488-492` |
| 7,176 impossible orderings in 1,000,000 | Release probe, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `concurrent_fetch_adds_partition_the_sequence_space` | Disjointness and completeness — of `AtomicU64`, through a one-line wrapper |
| `counts_are_exact_under_contention` | That no increment is lost, asserted at rest |
| `fetch_add_returns_the_value_before_the_advance` | The only crate-owned behaviour in the path, single-threaded |
| *(to create)* | Nothing reads the counter and the cell while both are moving, which is where the ordering decision has its only effect |
