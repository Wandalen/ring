# Decisions: A Trait, Because Two Criteria Needed Two Implementations

### Scope

**Purpose:** Record why the crate exports a trait rather than a struct, whether the
two acceptance criteria it names say what the crate says they say, and whether the
decision delivered what it was made for.

**Responsibility:** The `Why a trait rather than a struct` paragraph, both
acceptance criteria as written, and every assertion in the family that the
decision made possible.

**In Scope:** `ring_atomic/src/lib.rs:23-33`;
`bench_harness/docs/acceptance/001_feature_reached_tests.md:43`, `:45`.

**Out of Scope:** What the shim costs to run is
[`algorithm/001`](../algorithm/001_one_intrinsic_or_two.md) AT2. The trait's missing
bounds are [`api/002`](../api/002_a_shared_cell_that_is_not_sync.md).

---

## The Argument, the Criteria, and What Was Built on Them

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the decision, as the crate states it --'
command grep -m1 -A10 -F '//! ## Why a trait rather than a struct' ring_atomic/src/lib.rs
echo '  -- the two criteria it names, as they are written --'
command grep -n 'zero atomic operations\|one fence, not 64' bench_harness/docs/acceptance/001_feature_reached_tests.md | cut -c1-190 | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
echo '  -- and every assertion the shim makes possible --'
command grep -rn '\.counts()' --include=*.rs . | command grep -v '^ring_atomic/' | command grep 'assert' | sed 's|ring/|    |' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
```

Live output:

```
  -- the decision, as the crate states it --
//! ## Why a trait rather than a struct
//!
//! Two acceptance criteria in
//! `bench_harness/docs/acceptance/001_feature_reached_tests.md` are
//! *negative* claims about atomic traffic — feature 175's "accumulates N items
//! with zero atomic operations" and feature 177's "a claim of 64 slots issues
//! one fence, not 64". Neither can be asserted against a bare `AtomicU64`,
//! because the count is not observable from outside. [`SeqCell`] exists so the
//! crates that perform those operations can be written once and run against
//! either [`AtomicSeq`] in production or [`CountingSeq`] in a test that needs
//! the count.
  -- the two criteria it names, as they are written --
| 175 | Thread-local buffer and flush-into | `ring_tls` | S2 | A `TlsBuffer` accumulates `N` items with zero atomic operations (asserted by a counting allocator/atomic shim), and one `fl
| 177 | Batch claim and batch drain | `ring_batch` | S2 | A claim of 64 slots issues one fence, not 64 (asserted against a counting ordering shim); the 64 sequences returned are contiguo
  -- and every assertion the shim makes possible --
    ring_batch/tests/batch_test.rs:  assert_eq!( cursor.counts().fetch_adds, 1, "one fetch_add bought all 64" );
    ring_batch/tests/batch_test.rs:  assert_eq!( cursor.counts().total, 1, "and nothing else was touched" );
    ring_batch/tests/batch_test.rs:    assert_eq!( cursor.counts().total, 1, "a claim of {count} must cost exactly one operation" );
    ring_batch/tests/batch_test.rs:  assert_eq!( producer.counts().fetch_adds, 1, "gating adds a load, not a second advance" );
    ring_batch/tests/batch_test.rs:  assert_eq!( producer.counts().total, 0, "the cursors were never read" );
    ring_batch/tests/batch_test.rs:  assert_eq!( consumer.counts().total, 0 );
    ring_batch/src/lib.rs:/// assert_eq!( cursor.counts().total, 1, "64 slots, one atomic operation" );
    ring_tls/tests/tls_test.rs:  assert_eq!( cursor.counts().total, 0, "512 pushes must touch no atomic at all" );
    ring_tls/tests/tls_test.rs:  assert_eq!( cursor.counts().fetch_adds, 1 );
    ring_tls/tests/tls_test.rs:  assert_eq!( cursor.counts().total, 1, "64 items, one atomic operation" );
    ring_tls/tests/tls_test.rs:  assert_eq!( cursor.counts().total, 1, "the whole landing cost one operation" );
    ring_tls/tests/tls_test.rs:  assert_eq!( cursor.counts().total, 1 );
    ring_tls/src/lib.rs:/// assert_eq!( cursor.counts().total, 0, "accumulation touched no atomic" );
    ring_tls/src/lib.rs:/// assert_eq!( cursor.counts().total, 1, "64 items, one atomic operation" );
```

---

### AT15 — The Decision Was Made for a Reason and the Reason Was Delivered

Both criteria exist, both are quoted accurately, and both name a test file that
uses the shim. `ring_tls/tests/tls_test.rs` and `ring_batch/tests/batch_test.rs`
are precisely the two files the two acceptance criteria point at, and both drive the code
under test through `CountingSeq` and assert on `counts()`. Fourteen assertions
across four files, and every one is an assertion that a bare `AtomicU64` makes
impossible.

**Finding.** This is the crate's stated reason for existing, and it is fully
realized — a decision, its argument, the criteria that forced it, and the
assertions that could not otherwise be written, all reachable from one paragraph.
Nothing in this corpus contradicts it.

It is also the crate's only fully-documented decision. The nine ordering literals
have no such paragraph ([`decisions/001`](001_orderings_named_never_defaulted.md)
AT13), the packed layout has none
([`data_structure/001`](../data_structure/001_one_word_and_five.md) AT9), and the
absent `must_use` and `Sync` have none
([`api/001`](../api/001_the_return_value_that_is_a_claim.md),
[`api/002`](../api/002_a_shared_cell_that_is_not_sync.md)). The crate documents the
decision it is proud of and is silent on the four it made in passing.

---

### AT16 — The Shim Counts Calls, and One of the Two Criteria Is About Fences

The two criteria are not the same shape, and the difference decides whether the
shim can settle them.

The `ring_tls` criterion is sound through it. "Accumulates N items with zero atomic operations"
is asserted as `counts().total == 0`, and zero calls really does imply zero atomic
instructions — nothing ran, so nothing was issued. `tls_test.rs:48` is a valid
proof of its criterion.

The `ring_batch` criterion is not. "A claim of 64 slots issues one fence, not 64" is asserted as
`counts().total == 1` — one *call*. The call was `CountingSeq::fetch_add`, which
increments a counter and then advances the cell: two hardware read-modify-writes,
measured ([`algorithm/001`](../algorithm/001_one_intrinsic_or_two.md) AT2). The
assertion is true and the criterion it certifies is about a quantity the assertion
does not measure.

**Finding.** The shim's presence is what makes the number two, so the instrument
changes the value of the thing it is reading. That is fine for a criterion phrased
as "did we call it once or 64 times" and wrong for one phrased as "one fence, not
64" — and six of the fourteen assertions above are phrased against `total` or
`fetch_adds` with a message naming atomics or operations rather than calls
(`"64 slots, one atomic operation"`, `"a claim of {count} must cost exactly one
operation"`, `"the whole landing cost one operation"`).

The gap is small and entirely fixable in wording: the criterion means "one claim
operation, not 64", which is what the tests establish and what actually matters for
the amortisation argument. As written, the `ring_batch` criterion asks for a fence count, the test
supplies a call count, and the crate that mediates between them says nothing about
the difference — while its own module comment repeats the criteria's phrasing
verbatim.

**Disposition:** declined — the fix is rewording the `ring_batch` criterion's own
acceptance wording in
`bench_harness/docs/acceptance/001_feature_reached_tests.md:45` from
"one fence" to "one atomic operation"; that edit belongs to a bench_harness
acceptance-criteria change with its own review, not a ring_atomic corpus
disposition pass.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`decisions/001`](001_orderings_named_never_defaulted.md) | The decisions this crate makes and does not write down |
| [`algorithm/001`](../algorithm/001_one_intrinsic_or_two.md) | The two-atomics measurement that makes AT16 a discrepancy rather than a quibble |
| [`integration/002`](../integration/002_five_crates_downstream.md) | `ring_tls` and `ring_batch`, the two dependants the decision was made for |
| [`pattern/002`](../pattern/002_the_counting_cell_is_not_a_mock.md) | Why delegation rather than substitution is what makes the counts admissible evidence |

### Sources

| Fact | Where |
|------|-------|
| The trait-versus-struct paragraph | `ring_atomic/src/lib.rs:23-33` |
| The `ring_tls` criterion as written | `bench_harness/docs/acceptance/001_feature_reached_tests.md:43` |
| The `ring_batch` criterion as written | `bench_harness/docs/acceptance/001_feature_reached_tests.md:45` |
| The fourteen assertions | Census above |
| Two hardware atomics per counted call | `algorithm/001` AT2 |

### Tests

| Test | Covers |
|------|--------|
| `the_counting_cell_is_the_production_cell_plus_bookkeeping` | That the shim agrees with production on every value, which is what makes the substitution legitimate |
| `a_cell_drives_through_the_trait_alone` | That one piece of code can be run against either, which is the decision's mechanism |
| *(to create)* | Nothing asserts a fence count anywhere in the family, which is what the `ring_batch` criterion literally asks for |
