# Non-Functional Requirement: Sixteen Bytes and No Allocation

### Scope

**Purpose:** Record the crate's second cost property — that nothing in it touches
the heap and the whole working set is sixteen stack bytes — what holds that
property in place, and what the family's allocating version of the same fold
costs.

**Responsibility:** The absence of `Vec`, `Box`, `String`, and `collect` in the
crate body; the size of `BatchClaim`; and `ring_index::run` against
`ring_batch::drain_order`.

**In Scope:** `ring_batch/src/lib.rs:32`, `:56-60`, `:162-165`, `:358-362`;
`ring_index/src/lib.rs:119-122`.

**Out of Scope:** Time is
[`non_functional_requirement/001`](001_what_one_batch_actually_buys.md). That
`Option< BatchClaim >` costs 24 bytes for lack of a niche is
[`data_structure/001`](../data_structure/001_sixteen_bytes_that_are_not_a_buffer.md)
BA9. That `drain_order` has no caller is
[`integration/001`](../integration/001_four_edges_in_two_written_down.md) BA19.

---

## The Census

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- everything in the body that could reach the heap --'
command grep 'Vec\|Box::\|String\|to_vec\|collect\|alloc' ring_batch/src/lib.rs | command grep -v '///' || echo '    (none)'
echo '  -- the whole working set --'
command grep -m1 -A4 -F 'pub struct BatchClaim' ring_batch/src/lib.rs
echo '  -- and the two folds, both lazy --'
command grep -m1 -A3 -F '  pub fn sequences( &self ) -> impl Iterator< Item = Seq > + use< >' ring_batch/src/lib.rs
command grep -m1 -A4 -F 'pub fn drain_order( claim : &BatchClaim, capacity : Capacity )' ring_batch/src/lib.rs
```

Live output:

```
  -- everything in the body that could reach the heap --
    (none)
  -- the whole working set --
pub struct BatchClaim
{
  start : Seq,
  count : usize,
}
  -- and the two folds, both lazy --
  pub fn sequences( &self ) -> impl Iterator< Item = Seq > + use< >
  {
    ( self.start.0..self.end().0 ).map( Seq )
  }
pub fn drain_order( claim : &BatchClaim, capacity : Capacity )
-> impl Iterator< Item = ( Seq, SlotIndex ) > + use< >
{
  claim.sequences().map( move | seq | ( seq, of( seq, capacity ) ) )
}
```

Measured on the same host:

```
--- what the caller holds ---
  BatchClaim               16 bytes, align 8
  Option< BatchClaim >     24 bytes
  Seq                       8 bytes
  SlotIndex                 8 bytes
  Vec< SlotIndex >         24 bytes, plus one heap block per call
```

---

### BA36 — The Property Is Real and Nothing Holds It

The crate never allocates. Both functions that yield a sequence of things yield
iterators rather than collections, `BatchClaim` is two `u64`-sized fields on the
caller's stack, and a `Vec< SlotIndex >` header alone is larger than the entire
claim before counting the heap block it points at.

That is exactly right for a Tier 2 crate on a hot publish path, and nothing in
the repository would notice if it stopped being true:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- what the crate declares about the heap --'
command grep '#!\[' ring_batch/src/lib.rs || echo '    (no crate-level attributes at all)'
echo '  -- and across the family --'
printf '    crates with #![no_std] : %s of 33\n' "$( command grep -rl 'no_std' --include=lib.rs ring_*/src/ | wc -l )"
```

Live output:

```
  -- what the crate declares about the heap --
#![ deny( missing_docs ) ]
  -- and across the family --
    crates with #![no_std] : 3 of 33
```

**Finding.** `#![ deny( missing_docs ) ]` is the crate's only attribute, and three
of the family's thirty-three crates are `no_std` — `ring_types`, `ring_stats` and
`ring_overflow`, none of them this one, though `ring_types` sits in this crate's
own dependency closure. So the allocation-freedom of the crate on the hottest path
in the family rests entirely on nobody having written a `Vec` yet — no `no_std`
here, no allocator shim in the tests, no size assertion, no lint. A single `collect()` added to `drain_order` for convenience
would compile, pass the whole suite, and change the crate's cost class silently.

`#![no_std]` would be the honest declaration: the crate has no reason to need
`std`, its only dependencies are three sibling crates and `core::sync::atomic`,
and the attribute turns a habit into a compiler error. The absence is not a
defect today; it is the difference between a property and a guarantee.

---

### BA37 — The Family Has an Allocating Version of This Fold and Nobody Calls It

`ring_index` — which `ring_batch` depends on — already contains the same fold:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the same fold, one crate down, returning a Vec --'
command grep -m1 -A3 -F 'pub fn run( start : Seq, count : usize, capacity : Capacity ) -> Vec< SlotIndex >' ring_index/src/lib.rs
echo '  -- what ring_batch imports from that crate --'
command grep 'use ring_index' ring_batch/src/lib.rs
echo '  -- who calls the allocating one, outside its own crate --'
command grep -r 'ring_index::run\|index::run' --include=*.rs . | command grep -v '^ring_index/' || echo '    (nobody)'
```

Live output:

```
  -- the same fold, one crate down, returning a Vec --
pub fn run( start : Seq, count : usize, capacity : Capacity ) -> Vec< SlotIndex >
{
  ( 0..count as u64 ).map( | n | of( start.advanced_by( n ), capacity ) ).collect()
}
  -- what ring_batch imports from that crate --
use ring_index::of;
  -- who calls the allocating one, outside its own crate --
    (nobody)
```

`run` and `drain_order` map the same range through the same `of`. `run` collects
and returns slot indices; `drain_order` stays lazy and returns `( Seq, SlotIndex )`
pairs, which is strictly more information for strictly less memory. `ring_batch`
imports `of` from `ring_index` and reimplements the rest.

Measured — the same fold, both ways, on the same host:

```
--- the same fold, one allocating and one not ---
    items      ring_index::run            drain_order      ratio
        1              22.9 ns                 2.4 ns       9.6x
        8              23.5 ns                 3.7 ns       6.4x
       64              40.0 ns                23.5 ns       1.7x
     1024             406.6 ns               377.2 ns       1.1x
```

**Finding.** The allocating version costs 9.6× the lazy one at a single item and
converges to 1.1× at 1024, which is the signature of a fixed cost: roughly 20 ns
of allocator round-trip, constant, amortised away as the actual fold work grows.
At the batch sizes this family is built for — this crate's own contract names sixty-four —
that fixed cost is still 1.7×.

Which makes the reimplementation the right call, and leaves two things wrong
around it. `ring_index::run` has no caller anywhere outside its own crate and its
own tests, so the family carries an allocating public function that exists only
to be tested. And neither crate mentions the other's version, so the next author
who needs slot indices for a range finds `run` first — it is the one with the
obvious name — and pays 9.6× without ever learning that the lazy one is two
crates' worth of `use` away.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`non_functional_requirement/001`](001_what_one_batch_actually_buys.md) | The time cost of the same operation |
| [`data_structure/001`](../data_structure/001_sixteen_bytes_that_are_not_a_buffer.md) | The sixteen bytes, and the niche the empty state costs |
| [`integration/001`](../integration/001_four_edges_in_two_written_down.md) | The `ring_index` edge this finding runs along |
| [`workaround/002`](../workaround/002_the_usize_u64_seam.md) | The width cast the same fold performs on every item |

### Sources

| Fact | Where |
|------|-------|
| No allocation in the body | Census above |
| The crate's only attribute | `ring_batch/src/lib.rs:32` |
| The two lazy folds | `ring_batch/src/lib.rs:162-165`, `:358-362` |
| The allocating fold | `ring_index/src/lib.rs:119-122` |
| Sizes, and the cost of both folds | Release probe, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `a_batch_drain_reads_in_issue_order` | `drain_order`'s output, not its cost |
| *(to create)* | That `size_of::< BatchClaim >()` is 16 — the property has no assertion anywhere |
| *(to create)* | The allocation count, which no test in the family observes |
