# algorithm

The rules `ring_cursor` computes with — what it folds, and what it delegates.

This crate computes almost nothing. Both entries here are about *arrangement*
rather than arithmetic: which cursors get read, at which ordering, and who owns
the arithmetic performed on the results.

### Overview Table

| ID | Name | Computes | Delegates to |
|----|------|----------|--------------|
| 001 | [The Slowest Fold](001_the_slowest_fold.md) | The minimum position across a cursor slice, read at `GATING` | `ring_seqno::slowest` — via a `Vec` |
| 002 | [Three Readings of Two Cursors](002_three_readings_of_two_cursors.md) | Nothing — two loads, then a call | `ring_seqno::free_slots`, `pending`, `may_claim` |

**The delegation is the design.** The lap boundary — whether a producer exactly
`capacity` ahead of a consumer may claim — is an off-by-one that
`ring_seqno` names explicitly and decides once. This crate reads cursors and hands
the numbers over. `tests/manual/readme.md` M5 is the check that it stays that way.

001 is where the delegation costs something: reusing a function that takes
`&[ Seq ]` forces a `Vec` to exist so that a slice can be handed to it.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- what this crate folds --'
command grep -E '^pub fn slowest|^  ring_seqno::' ring_cursor/src/lib.rs
echo '  -- and the arithmetic it hands over --'
command grep -E '^pub fn (slowest|free_slots|pending|may_claim)' ring_seqno/src/lib.rs
```

Live output:

```
  -- what this crate folds --
pub fn slowest( cursors : &[ PaddedCursor ] ) -> Option< Seq >
  -- and the arithmetic it hands over --
pub fn may_claim( producer : Seq, consumer : Seq, capacity : Capacity ) -> bool
pub fn free_slots( producer : Seq, consumer : Seq, capacity : Capacity ) -> usize
pub fn pending( producer : Seq, consumer : Seq ) -> u64
pub fn slowest( cursors : &[ Seq ] ) -> Option< Seq >
```

**One line of this crate against four functions of another.** The fold is the
only arithmetic named here, and it no longer delegates: `slowest`'s signature
is at 120 and its entire body is the single line at 122, since
[`algorithm/001`](001_the_slowest_fold.md) removed the `Vec` and the
cross-crate call that used to sit between them.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CU1 | `slowest` | n/a — coverage | The `Vec` and the cross-crate call it fed were one line apart, and the root manifest configures no LTO, so the hoped-for elision would have had to survive a call the profile never asks the linker to inline. It did not: measured from `ring_barrier`, the fold charged one 8-byte allocation per call. The fold is now `min()` over the iterator, and `tests/allocation_test.rs` asserts zero at three arities and across a thousand consecutive reads, behind a control arm that must allocate |
| CU2 | `slowest` | n/a — observation | The function has no caller inside this crate. Its two production callers are in `ring_barrier` and `ring_gating`, so the crate that owns the allocation is not the crate that pays for it, and neither of those two can see the `Vec` without reading this source |
| CU3 | The three readings | n/a — duplication | `free_slots`, `pending` and `may_claim` each issue the identical `self.producer.load( GATING ), self.consumer.load( GATING )` pair, so a caller asking two of the three questions in a row pays four loads where two would do, and no combined reading is offered |
| CU4 | `ring_seqno` | n/a — inconsistency | The three delegated functions return `bool`, `usize` and `u64` for readings over the same two sequences, and this crate forwards all three widths unchanged. The split is inherited rather than chosen and neither crate documents it, so a caller relating `pending` to `free_slots` casts without being told why |
