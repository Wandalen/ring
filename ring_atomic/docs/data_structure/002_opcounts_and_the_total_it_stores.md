# Data Structure: `OpCounts`, and the `total` It Stores

### Scope

**Purpose:** Record what `OpCounts` holds, which of its fields carries information
the others do not, and what the type does to keep the five in agreement.

**Responsibility:** The five public fields, the derives, the doc sentence defining
`total`, and the one test that constructs the type by hand.

**In Scope:** `ring_atomic/src/lib.rs:272-285`;
`ring_atomic/tests/atomic_test.rs:307-319`.

**Out of Scope:** How the four counters are read to produce one of these is
[`algorithm/002`](../algorithm/002_counts_is_four_reads_not_one.md). The 40 bytes
they are read *from* are [`data_structure/001`](001_one_word_and_five.md).

---

## Five Public Fields, One of Them Derived From the Other Four

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the report type, in full --'
command grep -m1 -A14 -F '#[ derive( Debug, Clone, Copy, PartialEq, Eq, Default ) ]' ring_atomic/src/lib.rs
echo '  -- and the crate hand-writing one in a test --'
command grep -m1 -A14 -F '  assert_eq!( shim.counts().fetch_adds, 1 );' ring_atomic/tests/atomic_test.rs | tail -n 12
```

Live output:

```
  -- the report type, in full --
#[ derive( Debug, Clone, Copy, PartialEq, Eq, Default ) ]
pub struct OpCounts
{
  /// Reads served.
  pub loads : usize,
  /// Writes served.
  pub stores : usize,
  /// Advances served — the batch claim's own operation.
  pub fetch_adds : usize,
  /// Compare-exchanges served, successful or not — the contended claim's.
  pub compare_exchanges : usize,
  /// Every operation above, summed.
  pub total : usize,
}

  -- and the crate hand-writing one in a test --
#[ test ]
fn op_counts_are_comparable_and_printable()
{
  // Comparable so a test can assert a whole shape at once rather than four
  // fields; printable so a failure says what was actually counted.
  let a = OpCounts::default();
  let b = OpCounts { loads : 1, total : 1, ..OpCounts::default() };

  assert_ne!( a, b );
  assert_eq!( a, OpCounts::default() );
  assert!( format!( "{b:?}" ).contains( "loads: 1" ) );
}
```

No constructor, no `#[ non_exhaustive ]`, no validation. Every field is `pub` and
the type is `Copy`.

---

### AT11 — `total` Is Documented as a Definition and Enforced as Nothing

The doc comment on `total` is not a description of what the field usually holds; it
is a definition of what the field *is*: "Every operation above, summed." Only
`counts` ever satisfies it. Anyone else with the type in scope can write any number
there:

```
  a hand-built report                    : OpCounts { loads: 3, stores: 1, fetch_adds: 0, compare_exchanges: 0, total: 999 }
  its total vs the sum of its own fields : 999 vs 4
  the same four counters, measured       : OpCounts { loads: 3, stores: 1, fetch_adds: 0, compare_exchanges: 0, total: 4 }
  derived PartialEq calls them equal     : false
  ... and with total corrected           : true
```

**Finding.** `total` is eight of the type's forty bytes and carries no information
the other four do not — it is a cached sum, stored rather than computed, on a
`Copy` struct with no constructor. The type's only stated invariant is enforceable
by a one-line method (`pub const fn total( &self ) -> usize`) and is instead a
sentence in a doc comment.

The crate's own test is where this is most visible. `op_counts_are_comparable_and_printable`
writes `OpCounts { loads : 1, total : 1, ..OpCounts::default() }` — maintaining the
invariant by hand, in a struct literal, because nothing else will. Had it written
`total : 2`, every assertion in the test would still pass.

---

### AT12 — Derived Equality Compares the Redundant Field, So a Report Can Disagree With Itself

`PartialEq` is derived over all five fields, so two `OpCounts` with identical
counters compare unequal when their `total`s differ — which the probe above shows
directly: `real == lying` is `false` and `real == consistent` is `true`, on the
same four counters.

**Finding.** The consequence lands on the type's stated purpose. Its own test
comment says it is `PartialEq` "so a test can assert a whole shape at once rather
than four fields" — and a downstream test that builds the expected shape by hand
must now get five numbers right instead of four, with the fifth being one it can
derive but has no method to derive *with*. A wrong `total` in an expected value
fails the assertion for a reason that has nothing to do with what the code under
test did.

Making `total` a method would remove the field, the invariant, the redundant
comparison, and eight bytes, and would cost one `+` at each of the few call sites.
Nothing in the crate or the design corpus argues for the stored form; there is no
`#[ non_exhaustive ]` to add it later without a breaking change, either, so the
shape is fixed for every dependant the moment one asserts on it.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`algorithm/002`](../algorithm/002_counts_is_four_reads_not_one.md) | The four separately-timed reads this struct is assembled from |
| [`item/002`](../item/002_counts_the_method_that_is_not_a_snapshot.md) | `counts`, the only producer that satisfies the documented invariant |
| [`invariant/002`](../invariant/002_every_increment_survives.md) | The counting property the type reports on |
| [`type/002`](../type/002_the_report_that_is_all_public.md) | The same fields read as a question about what the type commits to |

### Sources

| Fact | Where |
|------|-------|
| The five public fields and the derives | `ring_atomic/src/lib.rs:272-285` |
| `total`'s defining doc sentence | `ring_atomic/src/lib.rs:283-284` |
| The hand-written literal in the suite | `ring_atomic/tests/atomic_test.rs:314` |
| An inconsistent value constructed and compared | Release probe, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `op_counts_are_comparable_and_printable` | `Debug` and `PartialEq` — while itself hand-maintaining the invariant nothing checks |
| `total_is_the_sum_of_the_four_and_not_an_independent_counter` | That `counts` produces a consistent `total`, which is the one producer that does |
| *(to create)* | Nothing establishes that a hand-built `OpCounts` can disagree with itself, which is the reachable case |
