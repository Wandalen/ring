# Data Structure: The Slice `slowest` Reads

### Scope

- **Purpose**: Examine `&[ Seq ]`, the only aggregate this crate touches, and trace what its shape costs the crate above.
- **Responsibility**: State what the slice must and need not satisfy, show why no caller in the family naturally holds one, and give the alternative parameter types with their consequences.
- **In Scope**: `slowest`'s parameter.
- **Out of Scope**: Its `Option` return — see [`type/002`](../type/002_the_option_that_slowest_returns.md).

### The Parameter

```rust
// ring_seqno/src/lib.rs:133
pub fn slowest( cursors : &[ Seq ] ) -> Option< Seq >
```

Borrowed, unsized, `Copy` elements. The crate's only non-scalar parameter.

### What It Requires, and What It Does Not

| # | Requirement | Holds? |
|---|-------------|:------:|
| R1 | Elements are `Ord` | ✅ `Seq` derives it, over `u64` |
| R2 | Non-empty | ❌ **Not required** — the empty case is the design point ([`decisions/001`](../decisions/001_none_rather_than_zero_for_an_empty_set.md)) |
| R3 | Sorted | ❌ Not required, and asserted not to matter — `slowest_is_the_minimum_wherever_it_sits` checks four arrangements |
| R4 | Distinct | ❌ Not required — `slowest( &[ Seq( 7 ), Seq( 7 ) ] )` is `Some( Seq( 7 ) )` |
| R5 | Any particular length | ❌ One element, many, or none |
| R6 | Elements read at the same instant | ❌ **Not required, and never true in practice** |

R6 is the one worth dwelling on. The slice is a set of *values*, so `slowest` is
a pure function — but the values came from atomic loads performed one at a time
by `ring_cursor`, and the cursors keep moving between them. The minimum returned
is of a set that may never have existed simultaneously.

That is acceptable and is not documented anywhere. It is acceptable because the
answer is used as a *lower bound*: a consumer only advances, so any position read
is a position it has at least reached, and a minimum over stale reads is
conservative in the safe direction. A caller acting on it under-estimates
available headroom, never over-estimates it. That reasoning is worth having
written down, since "the minimum of a set that never existed" sounds alarming and
is in fact the benign direction of wrong.

### No Caller in the Family Holds One

This is the shape's real consequence, and it has gone from expensive to fatal.
`&[ Seq ]` is a slice of plain values, and the family stores positions in
**atomics**, never in plain `Seq`:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- every plain-value sequence slice in the family --'
grep -r '\[ *Seq *\]\|Vec< Seq >' ring_*/src/*.rs | grep -vE ':\s*///'
echo '  -- and who calls the function it is the parameter of, outside this crate --'
# the trailing filter drops `//`, `///` and `//!` lines: two crates *mention*
# this function in doc comments, and a mention is not a call
out=$( grep -r 'ring_seqno::slowest' --include='*.rs' . \
       | grep -v '^ring_seqno/' | grep -vE ':[[:space:]]*//' )
if [ -z "$out" ]
then echo '    (no call outside ring_seqno)'
else printf '%s\n' "$out" | sed 's/^/    /'
fi
```

Live output:

```
  -- every plain-value sequence slice in the family --
ring_seqno/src/lib.rs:pub fn slowest( cursors : &[ Seq ] ) -> Option< Seq >
  -- and who calls the function it is the parameter of, outside this crate --
    (no call outside ring_seqno)
```

**One hit, and it is this function's own parameter.** For as long as the family
used it, exactly one caller bridged the gap, by building the values the
parameter needs:

```rust
// ring_cursor::slowest in ring_cursor/src/lib.rs, before commit b7e075ca
pub fn slowest( cursors : &[ PaddedCursor ] ) -> Option< Seq >
{
  let positions : Vec< Seq > = cursors.iter().map( | c | c.load( GATING ) ).collect();
  ring_seqno::slowest( &positions )
}
```

**A heap allocation per call, and its only purpose was to satisfy this
parameter's type.** Nothing else consumed `positions`; it was built, borrowed
once, and dropped.

The position of that call made it expensive: `ring_claim::claim` uses `headroom`
— which reached `slowest` — as the **condition of a compare-exchange retry
loop**, so each contended spin allocated. Recorded as F1 in
[`ring_cursor` `nfr/002`](../../../ring_cursor/docs/non_functional_requirement/002_the_gating_read_allocates_nothing.md).

Commit `b7e075ca` removed the allocation, and the way it removed it is what the
recipe's second half now reports. `ring_cursor::slowest` no longer builds a
`Vec`, because it no longer calls this function at all — it folds the loads in
place with `.min()`. The parameter type is unchanged, so nothing here broke;
what happened instead is that the one caller who could not afford it left, and
this function is now reachable only from its own tests. A parameter shape that
costs its single caller a heap allocation does not stay expensive
indefinitely — it stops having a caller.

### The Alternatives

| Parameter | Removes the `Vec`? | Cost |
|-----------|:------------------:|------|
| `&[ Seq ]` — as written | ❌ | The allocation above — and, in the end, the caller |
| `impl IntoIterator< Item = Seq >` | ✅ | Callers pass `.iter().copied()`; this crate's own tests need one adapter each |
| `&[ PaddedCursor ]` | ✅ | **Wrong.** `ring_seqno` would depend on `ring_atomic`, and the fold would own a memory-ordering decision that belongs a tier up |
| `&[ impl SeqCell ]` | ✅ | Same objection, plus a trait bound this crate has no business naming |
| `Option< Seq >` accumulator, folded by the caller | ✅ | Pushes the identity question back onto every caller — undoes [`decisions/001`](../decisions/001_none_rather_than_zero_for_an_empty_set.md) |

The second row is the real candidate:

```rust
pub fn slowest< I : IntoIterator< Item = Seq > >( cursors : I ) -> Option< Seq >
{
  cursors.into_iter().min()
}
```

`ring_cursor` would then pass `cursors.iter().map( | c | c.load( GATING ) )`
directly, materialising nothing. The `Option` contract is unchanged, `min()` over
an iterator behaves identically, and no ordering decision moves.

The break is small and real: `&[ Seq ]` does not satisfy `IntoIterator< Item = Seq >`
— a slice iterates as `&Seq` — so this crate's own tests and doctest gain a
`.iter().copied()` each. Four call sites in `seq_test.rs`, one in the doctest.

**It was not applied, and the allocation went away regardless.** Declining it
here was reasonable on its own terms — a public signature change in this crate
plus a body change in another, belonging to a run with its own verification. But
declining a change does not hold the world still. `ring_cursor` needed the
allocation gone more than it needed this crate, so commit `b7e075ca` took the
only route that required no agreement from here: it stopped calling, and
reimplemented `.min()` on its own side of the boundary.

The cost of that is not the four extra `.iter().copied()` calls this row was
weighed against. It is that the same one-line reduction now exists twice, in two
crates, with nothing keeping the two equal — see
[`integration/002`](../integration/002_how_the_fold_crossed_four_tiers.md), which
was written when it existed once. The row above is still the right change; it is
simply no longer available as a fix for a problem that has already routed around
it.

**The third and fourth rows are worth rejecting explicitly**, because they look
like the obvious fix — "take the cursors directly and skip the intermediate". They
would work, and they would move the `GATING` ordering decision into this crate.
`ring_seqno` currently has no dependency on `ring_atomic` and no opinion about
memory ordering at all, which is what lets it be a pure arithmetic library. That
separation is worth more than an allocation.

### Why a Slice and Not a Fixed Array

The gating set's size is a runtime value: `GatingSet::new( capacity, consumers )`
takes a count and `resize_with`s a `Vec` to it. A const-generic `[ Seq; N ]`
would push that count into the type system, where the family does not have it —
`ring_mpsc` passes a literal `1`, but `ring_claim` borrows whatever set it is
handed and cannot know the size at compile time.

The count is nonetheless **fixed once the set exists** — `GatingSet` has no
`&mut self` method at all, so consumers can be neither added nor removed after
construction:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -E '&mut self|fn push|resize|insert|remove' ring_gating/src/lib.rs
# one hit: resize_with, inside `new` itself
```

Live output:

```
    cursors.resize_with( consumers, PaddedCursor::default );
```

So the slice is right for a different reason than "the set changes over time": it
is right because the size is a *constructor* argument rather than a type
parameter. Nothing in the family mutates a gating set's membership — see
[`ring_gating` `data_structure/001`](../../../ring_gating/docs/data_structure/001_the_set_that_cannot_grow.md).

`Barrier::over( &cursors )` is likewise handed whatever slice the caller has.
Both tier-3 wrappers pass a field straight through, so the slice shape matches
the storage shape exactly at that level — and only mismatches at tier 2, where
the element type changes from atomic to value.

### SQ11 — The Parameter That Cost an Allocation One Tier Up, Then Cost the Call

The two signatures differ by one element type, and that difference was an
allocation for as long as one called the other:

```
ring_seqno::slowest    ( cursors : &[ Seq ] )          -> Option< Seq >
ring_cursor::slowest ( cursors : &[ PaddedCursor ] ) -> Option< Seq >
                       ^ had to load each cursor into a Vec< Seq > first
```

**Finding, and where it went.** `&[ Seq ]` cannot be produced from
`&[ PaddedCursor ]` without materialising the loads, which was the whole origin
of `ring_cursor`'s per-call `Vec`. That is still true of the types; it is no
longer true of the family, because the caller stopped calling rather than keep
paying it. The element-type mismatch survived the allocation it caused, and
outlived the dependency edge as well.

---

### SQ12 — Tested at Three, Deployed at Whatever

Every call in the test suite passes a slice small enough to write out by hand:

```
slowest_is_the_minimum_wherever_it_sits    1, 2, 2, 3, 2 elements
slowest_of_nothing_is_none_not_zero        0, 1 elements
                                           max tested: 3
```

**Finding.** `slowest` is tested with at most three elements and never with a set larger than a single cache line's worth, while the wrapper that faces real gating sets is one crate up.

---

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_the_slowest_fold.md](../algorithm/002_the_slowest_fold.md) | The fold over this slice, and its chain |

### Data Structures

| File | Relationship |
|------|--------------|
| [001_the_crate_that_declares_no_type.md](001_the_crate_that_declares_no_type.md) | Why there is no collection type to hold these |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_none_rather_than_zero_for_an_empty_set.md](../decisions/001_none_rather_than_zero_for_an_empty_set.md) | R2 — the empty slice as the design point |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_every_reading_is_allocation_free.md](../non_functional_requirement/001_every_reading_is_allocation_free.md) | The allocation this parameter forces, and where it lands |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_the_option_that_slowest_returns.md](../type/002_the_option_that_slowest_returns.md) | What comes back out |

### Sources

| File | Relationship |
|------|--------------|
| `ring_seqno/src/lib.rs:133` | The parameter |
| `ring_cursor/src/lib.rs:120-123` | The `Vec` built to satisfy it |
| `ring_claim/src/lib.rs:411-445` | The retry loop the allocation sits in |
| `ring_gating/src/lib.rs:197-200` | Tier 3b, passing a field straight through |
| `ring_barrier/src/lib.rs:191-194` | Tier 3a, likewise |

### Tests

| File | Relationship |
|------|--------------|
| `tests/seq_test.rs:113-122` | R3 and R4 — order and duplicates |
| `tests/seq_test.rs:125-130` | R2 — the empty slice |
| `ring_seqno/src/lib.rs:125-131` | The doctest, which would need `.iter().copied()` under the iterator signature |
