# Pattern: The Counting Cell Is Not a Mock

### Scope

**Purpose:** Record what makes `CountingSeq` a substitute rather than a stand-in,
what the family's counting assertions actually rest on, and the two properties the
substitution does not preserve.

**Responsibility:** `impl SeqCell for CountingSeq`, the parity test that licenses
every downstream counting assertion, and how the type is exported.

**In Scope:** `ring_atomic/src/lib.rs:287-293`, `:318-319`, `:474-500`;
`ring_atomic/tests/atomic_test.rs:228-258`.

**Out of Scope:** How far the instrument reaches — only generic code — is
[`item/002`](../item/002_counts_the_method_that_is_not_a_snapshot.md) AT27. What the
substitution costs in time is
[`non_functional_requirement/001`](../non_functional_requirement/001_what_the_instrument_costs.md).
The creation-site pattern that makes it possible is
[`pattern/001`](001_one_place_where_an_atomic_is_created.md).

---

## Delegation, Export, and Use

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- what makes it a substitute rather than a stand-in: every method delegates --'
command grep -m1 -A26 -F 'impl SeqCell for CountingSeq' ring_atomic/src/lib.rs | command grep -E 'fn |self\.cell|self\.0'
echo '  -- how it is exported --'
command grep -m1 -A3 -F '/// assert_eq!( cell.counts().total, 1 );' ring_atomic/src/lib.rs | tail -n 2
printf '    feature gates in the crate : %s\n' "$( command grep -c 'feature =' ring_atomic/src/lib.rs || true )"
printf '    cfg( test ) gates          : %s\n' "$( command grep -c 'cfg( test )' ring_atomic/src/lib.rs || true )"
echo '  -- what the doc comment promises about the substitution --'
command grep -m1 -A6 -F '/// A sequence cell that behaves exactly like [`AtomicSeq`] and counts what it' ring_atomic/src/lib.rs
echo '  -- and where the family actually constructs it --'
command grep -rc 'CountingSeq::new\|CountingSeq::default' --include=*.rs \
  ring_batch/src/lib.rs ring_batch/tests/batch_test.rs \
  ring_tls/src/lib.rs ring_tls/tests/tls_test.rs | sed 's|ring/||'
printf '    of which are doctest lines : %s\n' \
  "$( command grep -rn 'CountingSeq::new\|CountingSeq::default' --include=*.rs \
      ring_batch/src ring_tls/src | command grep -c '///' || true )"
```

Live output:

```
  -- what makes it a substitute rather than a stand-in: every method delegates --
  fn load( &self, order : Ordering ) -> Seq
    self.cell.load( order )
  fn store( &self, value : Seq, order : Ordering )
    self.cell.store( value, order );
  fn fetch_add( &self, n : u64, order : Ordering ) -> Seq
    self.cell.fetch_add( n, order )
  fn compare_exchange( &self, current : Seq, new : Seq, success : Ordering, failure : Ordering )
    self.cell.compare_exchange( current, new, success, failure )
  -- how it is exported --
#[ derive( Debug ) ]
pub struct CountingSeq
    feature gates in the crate : 0
    cfg( test ) gates          : 0
  -- what the doc comment promises about the substitution --
/// A sequence cell that behaves exactly like [`AtomicSeq`] and counts what it
/// was asked to do.
///
/// Not a mock: the underlying operations are the same real atomics, so a test
/// running against this observes the same values production would — at
/// roughly double the cost per operation, so this is a correctness instrument
/// and not a timing one. Only the bookkeeping is added — which is why an
  -- and where the family actually constructs it --
ring_batch/src/lib.rs:1
ring_batch/tests/batch_test.rs:6
ring_tls/src/lib.rs:1
ring_tls/tests/tls_test.rs:5
    of which are doctest lines : 2
```

---

### AT39 — Every Counting Assertion in the Family Rests on One Twenty-Line Straight-Line Test

`CountingSeq` is not a mock in any sense that matters. It contains a real
`AtomicSeq` and every one of its four methods increments a counter and then
*delegates* to it. There is no reimplementation to drift, no scripted return value,
no "when called with X, return Y". The sequence behaviour of the counting cell is
the sequence behaviour of the production cell, by construction, because it is
literally the production cell being called.

By construction, and — because construction can be edited — also by assertion.
`the_counting_cell_is_the_production_cell_plus_bookkeeping` exists precisely to
check it, and says so in its own comment: "Every assertion made against CountingSeq
elsewhere is a statement about production only if the two cells agree on every
operation." It drives an identical script through both and compares at every step,
including the failed-exchange case.

**Finding.** That test is load-bearing for thirteen counting assertions in two other
crates, and it is twenty lines of straight-line single-threaded script over four
fixed values: start at 3, add 5, store 20, exchange 20→21, exchange 99→0.

What it does not cover, and nothing else does: `fetch_add( 0 )`, `fetch_add(
u64::MAX )` and the wrap ([`pitfall/002`](../pitfall/002_the_wrap_that_reads_as_an_empty_ring.md)),
a `compare_exchange` whose `current` equals `new`, any ordering other than the
`Acquire`/`Release`/`AcqRel` trio it happens to use, and any concurrent
interleaving at all. Divergence in any of those would leave every downstream
`counts()` assertion silently meaning something other than what it says, and this
test would still pass.

The remedy is not more assertions of the same kind — it is a loop. The four methods
have small argument spaces (`n` over a handful of interesting values; the exchange
over equal/unequal/self), and driving both cells through the cross product in one
`for` loop would take about as many lines as the current script while covering the
cases that actually differ.

---

### AT40 — Faithful in Sequence, Unfaithful in Time and Space, and Nothing Marks the Difference

The substitution preserves exactly one thing: what the cell reads and returns. It
does not preserve two others.

**Time.** A counted call issues two hardware atomics instead of one, costing about
1.9× on `fetch_add` and about 2.7× on `load`
([`non_functional_requirement/001`](../non_functional_requirement/001_what_the_instrument_costs.md)
AT33). **Space.** `CountingSeq` is 40 bytes against `AtomicSeq`'s 8, and its
counters share a cache line with the cell — deliberately, and faster for it, but
different ([`data_structure/001`](../data_structure/001_one_word_and_five.md)).

Neither difference is a defect; both are the price of the instrument. What is
missing is any marker that they exist — and the crate's own doc comment, which is
otherwise the best-written paragraph about this type, is what makes the omission
bite. It opens "A sequence cell that behaves exactly like `AtomicSeq` and counts
what it was asked to do", then argues the substitution correctly and at length:
"Not a mock: the underlying operations are the same real atomics, so a test running
against this observes the same values production would."

Read closely, that is precise — *the same values*. Read at speed, "behaves exactly
like `AtomicSeq`" is an unqualified equivalence claim, and the very next sentence
reinforces it. Nothing in the paragraph, or anywhere else, says the equivalence
covers values and not cost.

**Finding.** `CountingSeq` is exported unconditionally `pub` from the crate root —
no feature gate, no `cfg( test )`, no `#[ doc( hidden ) ]` — and its documentation
argues for its fidelity without bounding it. What keeps it out of production is
convention, and the convention is currently perfect: thirteen construction sites
across `ring_batch` and `ring_tls`, eleven of them in `tests/` and the other two
doctest lines inside `src/lib.rs` comments. Nothing enforces it, and a consumer who
reads the doc comment and reaches for the type has been told the substitution is
free.

A `#[ cfg( feature = "counting" ) ]` gate would break the doctests and cost a
feature flag. One clause on the existing sentence would cost nothing: *the same
values production would — at roughly double the cost per operation, so this is a
correctness instrument and not a timing one.*

The doc comment now carries exactly that clause:

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
carries the suggested clause verbatim; the crate's 21 unit tests plus 8
doctests re-verified passing (`cargo test --all-features`, 2026-09-03). Now
prints:
`roughly double the cost per operation, so this is a correctness instrument`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`pattern/001`](001_one_place_where_an_atomic_is_created.md) | The creation-site pattern that makes one substitution point sufficient |
| [`item/002`](../item/002_counts_the_method_that_is_not_a_snapshot.md) | Which code the substitution can actually reach |
| [`non_functional_requirement/001`](../non_functional_requirement/001_what_the_instrument_costs.md) | What the substitution costs per call |
| [`data_structure/001`](../data_structure/001_one_word_and_five.md) | The 40-byte layout the substitution introduces |
| [`invariant/002`](../invariant/002_every_increment_survives.md) | The counter/cell relation the delegation order creates |

### Sources

| Fact | Where |
|------|-------|
| Four delegating methods | `ring_atomic/src/lib.rs:474-500` |
| Unconditional `pub`, no gates | `ring_atomic/src/lib.rs:318-319`; census above |
| The fidelity claim, unbounded | `ring_atomic/src/lib.rs:287-293` |
| The parity test and its own rationale | `ring_atomic/tests/atomic_test.rs:228-258` |
| Thirteen construction sites, eleven under `tests/` | Census above |

### Tests

| Test | Covers |
|------|--------|
| `the_counting_cell_is_the_production_cell_plus_bookkeeping` | The parity that licenses every downstream counting assertion — over one fixed four-value script |
| `each_operation_increments_exactly_its_own_counter` | That the bookkeeping half is correct, separately from the delegation half |
| `a_cell_drives_through_the_trait_alone` | That a caller need not know which of the two it holds |
| *(to create)* | Nothing drives both cells through a cross product of interesting arguments, so parity is asserted only at the five points the script visits |
