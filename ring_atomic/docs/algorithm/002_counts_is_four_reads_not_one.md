# Algorithm: `counts` Is Four Reads, Not One

### Scope

**Purpose:** Record how `counts` and `reset_counts` traverse the four counters,
what that makes the returned `OpCounts` a statement about, and how often it is a
statement about nothing.

**Responsibility:** `CountingSeq::counts` and `CountingSeq::reset_counts`: their
read and write order, and the single test that exercises either concurrently.

**In Scope:** `ring_atomic/src/lib.rs:410-424`, `:446-452`;
`ring_atomic/tests/atomic_test.rs:259-287`.

**Out of Scope:** The two atomics each counted operation issues are
[`algorithm/001`](001_one_intrinsic_or_two.md) AT2. The consequence for a caller
reading a torn snapshot is
[`pitfall/001`](../pitfall/001_the_snapshot_that_never_happened.md).

---

## Four Counters, Traversed One at a Time, Twice

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- four reads, then a sum of the four --'
command grep -m1 -A11 -F '    let loads = self.loads.load( Ordering::Relaxed );' ring_atomic/src/lib.rs
echo '  -- four writes, in a loop --'
command grep -m1 -A3 -F '    for counter in [ &self.loads, &self.stores, &self.fetch_adds, &self.compare_exchanges ]' ring_atomic/src/lib.rs
echo '  -- the only test that runs either under contention, and what it asserts --'
command grep -m1 -A5 -F '        for _ in 0..EACH' ring_atomic/tests/atomic_test.rs
command grep -m1 -A1 -F '  assert_eq!( cell.counts().fetch_adds, THREADS * EACH );' ring_atomic/tests/atomic_test.rs
echo '  -- and what the crate now says about the seam --'
printf '    doc sections naming it : %s\n' "$( command grep -cE '# Not a Snapshot|# Not a Coherent Observation' ring_atomic/src/lib.rs )"
printf '    the test that pins it  : %s\n' "$( command grep -c 'fn total_is_derived_from_the_same_four_reads' ring_atomic/tests/atomic_test.rs )"
```

Live output:

```
  -- four reads, then a sum of the four --
    let loads = self.loads.load( Ordering::Relaxed );
    let stores = self.stores.load( Ordering::Relaxed );
    let fetch_adds = self.fetch_adds.load( Ordering::Relaxed );
    let compare_exchanges = self.compare_exchanges.load( Ordering::Relaxed );
    OpCounts
    {
      loads,
      stores,
      fetch_adds,
      compare_exchanges,
      total : loads + stores + fetch_adds + compare_exchanges,
    }
  -- four writes, in a loop --
    for counter in [ &self.loads, &self.stores, &self.fetch_adds, &self.compare_exchanges ]
    {
      counter.store( 0, Ordering::Relaxed );
    }
  -- the only test that runs either under contention, and what it asserts --
        for _ in 0..EACH
        {
          let _ = cell.fetch_add( 1, Ordering::AcqRel );
        }
      } );
    }
  assert_eq!( cell.counts().fetch_adds, THREADS * EACH );
  assert_eq!( cell.load( Ordering::Acquire ), Seq( ( THREADS * EACH ) as u64 ) );
  -- and what the crate now says about the seam --
    doc sections naming it : 2
    the test that pins it  : 1
```

Each individual counter is exact. The set of four is not read together.

---

### AT3 — The Returned `OpCounts` Can Describe a State That Never Existed

`counts` reads `loads`, then `stores`, then `fetch_adds`, then
`compare_exchanges`, four separate `Relaxed` loads at four different instants,
and builds one struct from them. Nothing prevents the cell from being used
between any two of those reads, so the four fields can come from four different
moments — and `total`, summed from them, from none.

Measured with one writer whose loop makes `loads >= stores` true at every single
instant, because it always loads before it stores and never stores twice in a
row:

```
--- one writer holding `loads >= stores` true at every instant ---
  snapshots taken                    2000000
  snapshots reporting stores > loads 11575
  widest impossible skew             5456
  a final quiet snapshot             loads 130173 stores 130173 total 260346
```

**Finding.** 11,575 of 2,000,000 snapshots reported `stores > loads`, which the
writer's own loop makes unreachable at any instant — the widest by 5,456. The
returned struct is not a snapshot; it is four independent readings presented in
one value, and the sum in `total` is the sum of readings that were never
simultaneously true.

Concurrent use is not a hypothetical for this type. `CountingSeq` is a `SeqCell`
and every `SeqCell` in the family is shared between a producer and a consumer;
the crate's own contention test spawns four threads against one. A test that
asserts a *ratio* between two counters — say that a batched path issued as many
`fetch_add`s as it did `store`s — is asserting against a value that can be off by
thousands, and would fail intermittently for a reason its author would look for
in the code under test.

The rate is small and the failure mode is not: this is the shape that produces a
flake nobody can reproduce.

**Disposition:** applied — as documentation, not as a fix: the seam is what the type
is, and the only way to remove it is to serialize the four counters behind a lock or
collapse them into one word, either of which changes `CountingSeq` from a free
observer into something with its own contention profile and defeats the purpose of a
counting shim. So `counts`'s contract in `src/lib.rs` now carries a `# Not a
Snapshot` section stating the four-`Relaxed`-loads mechanism, the rule that follows
from it (assert on a single field, or read while nothing else touches the cell,
never on a relationship between two fields), and this entry's own numbers —
11,575 of 2,000,000 with the widest skew 5,456 — as a `Fix(AT3)` line, so the
measurement is attached to the thing it is about rather than only to this document.
`OpCounts` itself carries the matching `# Not a Coherent Observation` section, which
is what the census counts. What this does not buy: a reader who never opens the
rustdoc for `counts` still gets a struct with five fields and no indication that
four of them were read at four different moments, and the type's own consistency
relation still certifies every torn reading as sound
([`pitfall/001`](../pitfall/001_the_snapshot_that_never_happened.md) AT43). Now
prints: `    doc sections naming it : 2`

---

### AT4 — `reset_counts` Has the Same Seam on the Write Side, and Neither Says So

`reset_counts` walks the same four counters in the same order and stores zero
into each. Between the first store and the last, a concurrent operation can
increment a counter that has already been zeroed — so a reset can leave the cell
with three counters at zero and one at some small value, and the following
`counts()` reports a mix of before and after.

**Finding.** Both traversals are correct for the single-threaded use the crate's
own tests make of them, and neither method's doc comment says that is the
constraint. `counts` says "What this cell has been asked to do so far"; the
`OpCounts` type says `total` is "every operation above, summed". Both sentences
read as descriptions of an instant.

Nothing in the crate can make the four atomic, short of a lock or packing the
four counters into one word — and neither is obviously right for a test shim
whose whole point is to add as little as possible. The gap is not the seam; it is
that the seam is undocumented on a type built specifically to be trusted about
numbers.

The suite cannot see it. `counts_are_exact_under_contention` is the only
concurrent test, and every thread in it performs one operation kind — so exactly
one of the four counters is ever non-zero, and a torn read of a single counter is
not possible. The test proves the increments are not lost, which is true, and is
a different property from the one that fails.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`pitfall/001`](../pitfall/001_the_snapshot_that_never_happened.md) | The same seam, read as the hazard rather than the mechanism |
| [`item/002`](../item/002_counts_the_method_that_is_not_a_snapshot.md) | `counts`' doc comment against what it returns |
| [`data_structure/002`](../data_structure/002_opcounts_and_the_total_it_stores.md) | `total` as a stored field rather than a computed one |
| [`invariant/002`](../invariant/002_every_increment_survives.md) | The property the contention test does establish |

### Sources

| Fact | Where |
|------|-------|
| The four reads and the sum | `ring_atomic/src/lib.rs:412-422` |
| The four writes | `ring_atomic/src/lib.rs:448-451` |
| The one concurrent test and its single operation kind | `ring_atomic/tests/atomic_test.rs:259-287` |
| 11,575 impossible snapshots in 2,000,000 | Release probe, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `counts_are_exact_under_contention` | That no increment is lost — one counter only, so the tear is out of reach |
| `resetting_the_counts_leaves_the_sequence_alone` | The reset's effect, single-threaded |
| *(to create)* | Nothing reads two counters concurrently, which is the only way to observe either seam |
