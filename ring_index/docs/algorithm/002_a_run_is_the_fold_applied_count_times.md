# Algorithm: A Run Is the Fold Applied `count` Times

### Scope

**Purpose:** Record that `run` is `of` in a loop with a heap allocation around
it, that the loop advances a sequence rather than an index, and that the crate's
one consumer of the same idea built it as an iterator instead.

**Responsibility:** `run`'s body, the addition it performs before each fold, and
the `Vec` it collects into.

**In Scope:** `ring_index/src/lib.rs:118-122`;
`ring_types/src/id.rs:65-68`; `ring_batch/src/lib.rs:358-362`.

**Out of Scope:** What the allocation costs is
[`non_functional_requirement/002`](../non_functional_requirement/002_the_one_allocation_and_the_zero_callers.md).
The addition's behaviour at the end of the sequence space is
[`pitfall/001`](../pitfall/001_the_run_that_panics_in_debug_and_wraps_in_release.md).

---

## Three Lines, Two of Them Not This Crate's

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A6 -F '/// assert_eq!( run( Seq( 0 ), 0, cap ), vec![] );' ring_index/src/lib.rs | tail -n 5
command grep -m1 -A3 -F '  pub const fn advanced_by( self, n : u64 ) -> Self' ring_types/src/id.rs
```

Live output:

```
#[ must_use ]
pub fn run( start : Seq, count : usize, capacity : Capacity ) -> Vec< SlotIndex >
{
  ( 0..count as u64 ).map( | n | of( start.advanced_by( n ), capacity ) ).collect()
}
  pub const fn advanced_by( self, n : u64 ) -> Self
  {
    Self( self.0 + n )
  }
```

The body is a `map` and a `collect`. Everything it does that `of` does not is
the `advanced_by` — a plain `+` in `ring_types` — and the `collect`.

---

### IX3 — The Loop Walks Sequence Space, Not Slot Space

An implementation that walked slots would increment the index and mask once per
step, or simply wrap it. This one recomputes the whole fold from a fresh
sequence every time. The difference is visible when the run is longer than the
ring:

```
--- (3) what one `run` costs, by count ---
  count 0      0 alloc       0 bytes  len 0  capacity 0
  count 1      1 alloc       8 bytes  len 1  capacity 1
  count 8      1 alloc      64 bytes  len 8  capacity 8
  count 1024   1 alloc    8192 bytes  len 1024  capacity 1024
  count 4096   1 alloc   32768 bytes  len 4096  capacity 4096
```

**Finding.** `run( start, 4096, cap 1024 )` returns 4096 indices, not 1024. The
function does not clamp, does not deduplicate, and does not report that the
caller asked for four laps of a one-lap ring — it answers the question as asked
and returns the repeats.

The crate's test names this and calls it correct:
`an_oversized_run_repeats_slots` asserts `produced[ 0 ] == produced[ 4 ]` at
capacity 4, with the comment *"which is the caller's problem to prevent by
gating — this crate reports the truth rather than clamping"*. That is a defensible
ruling, and it is the same one `of` makes about aliasing. What is worth
recording is that `run`'s own doc comment states the opposite as though it were
guaranteed: *"A batch claim is contiguous in sequence space, so its slots wrap at
most once"*. The premise is about batch claims, the conclusion is written as a
property of `run`, and `run` accepts any `count` at all.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_index
command grep -F 'returns repeated slots rather than stopping at one wrap' src/lib.rs
```

Live output:

```
/// returns repeated slots rather than stopping at one wrap.
```

**Disposition:** applied — `run`'s own doc comment no longer lets the
batch-claim guarantee read as a property of the function itself; it now says
the guarantee belongs to the caller's batch claim, and that `run` neither
clamps nor deduplicates, so an oversized `count` repeats slots rather than
wrapping once — matching what `an_oversized_run_repeats_slots` already
asserts.
Now prints: `returns repeated slots rather than stopping at one wrap`

---

### IX4 — The One Crate That Wanted This Built It Again, Lazily

`run` produces the slots for a contiguous range. `ring_batch` needs exactly
that, and has `ring_index` as a dependency:

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
command grep 'use ring_index' ring_batch/src/lib.rs
command grep -m1 -A4 -F 'pub fn drain_order( claim : &BatchClaim, capacity : Capacity )' ring_batch/src/lib.rs
```

Live output:

```
use ring_index::of;
pub fn drain_order( claim : &BatchClaim, capacity : Capacity )
-> impl Iterator< Item = ( Seq, SlotIndex ) > + use< >
{
  claim.sequences().map( move | seq | ( seq, of( seq, capacity ) ) )
}
```

**Finding.** `ring_batch::drain_order` is `run` with two changes: it yields
`( Seq, SlotIndex )` pairs rather than bare indices, and it returns an
`impl Iterator` rather than a `Vec`. It imports `of`, not `run` — the crate that
had the exact use case reached past the function built for it and composed the
primitive itself.

The result is that the family's one range-folding call site allocates nothing,
while the function named for range folding allocates once per call
([`non_functional_requirement/002`](../non_functional_requirement/002_the_one_allocation_and_the_zero_callers.md)
IX23). Whether `run` was rejected deliberately or simply not noticed is not
recorded anywhere: `ring_batch` has no comment about it, and `run`'s doc cites
this family's own batch-claim contract — the same contract `drain_order`
implements — as its own justification.

So the citation runs one way and the dependency runs the other. `run` points at
the batch feature; the batch feature's implementation points at `of`.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`algorithm/001`](001_one_and_of_a_mask.md) | The fold this applies repeatedly |
| [`non_functional_requirement/002`](../non_functional_requirement/002_the_one_allocation_and_the_zero_callers.md) | What the `Vec` costs, measured |
| [`decisions/002`](../decisions/002_a_vec_where_an_iterator_would_do.md) | The return type, priced against the alternative |
| [`workaround/002`](../workaround/002_the_iterator_ring_batch_built_instead.md) | `drain_order` read as the workaround it is |
| [`pitfall/001`](../pitfall/001_the_run_that_panics_in_debug_and_wraps_in_release.md) | What `advanced_by` does at the top of the range |

### Sources

| Fact | Where |
|------|-------|
| `run`'s body | `ring_index/src/lib.rs:118-122` |
| `advanced_by`'s body | `ring_types/src/id.rs:65-68` |
| `drain_order`'s body and its import | `ring_batch/src/lib.rs:36, 358-362` |
| Allocation and length by count | Release probe, quoted above |
| The "wraps at most once" sentence | `ring_index/src/lib.rs:82-84` |

### Tests

| Test | Covers |
|------|--------|
| `a_run_wraps_at_most_once_within_one_capacity` | The in-range case the doc describes |
| `a_full_capacity_run_covers_every_slot_once` | A full lap from every start across two laps |
| `an_oversized_run_repeats_slots` | The out-of-range case the doc does not describe |
| `an_empty_run_is_empty` | `count = 0` yielding nothing rather than one element |
| *(to create)* | That `run` and `ring_batch::drain_order` agree on the same range — nothing asserts the two folds match |
