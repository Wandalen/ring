# algorithm

Two computations. Neither is arithmetic this crate performs — one is a pair of
delegations with an identity choice between them, the other is two comparisons
in a load-bearing order.

### Overview Table

| ID | Name | Computes |
|----|------|----------|
| 001 | [`headroom` in Two Delegations](001_headroom_in_two_delegations.md) | How many slots a producer may claim, via a fold and a subtraction that both live elsewhere |
| 002 | [`check` Orders Its Two Refusals](002_check_orders_its_two_refusals.md) | The same answer plus a reason, and why the order of the two tests is the algorithm |

### What the Crate Actually Contributes

| Contribution | Where |
|--------------|-------|
| Choosing `capacity` as the empty-set identity | `headroom`'s `map_or` |
| Ordering `BatchTooLarge` before `Full` | `check`'s two `if`s |
| Multiplying the minimum by one lap | `limit`'s `advanced_by` |

Three decisions, none of them arithmetic. Everything numeric is
`ring_cursor::slowest` and `ring_seqno::free_slots`, and manual check M3 asserts
mechanically that it stays that way.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| GT1 | `headroom` | n/a — observation | It descends five steps through four crates and four tiers (4 → 3 → 1 → 0) to reach one subtraction, and that depth is what makes this crate's own body two lines |
| GT2 | The `map_or` arms | n/a — observation | The two arms share no arithmetic — one returns `self.capacity.get()` outright, the other calls `ring_seqno::free_slots` — and exactly one test holds them against each other. `an_ungated_ring_and_a_consumer_at_zero_disagree_after_one_lap` asserts they agree at `Seq::ZERO` and differ at `Seq( 4 )`, so this crate's test pins `free_slots`' boundary convention from outside the crate that defines it |
| GT3 | `check` and `admits` | n/a — duplication | Two readings of one state, agreeing across a 1,920-case sweep; the reason for the second is that a `bool` cannot say *why* |
| GT4 | The sweep | n/a — coverage | `check_and_admits_agree_across_the_whole_state_space` runs 16 consumer positions × 12 producer offsets × 10 counts, but the producer range starts at the consumer, so it never tests a producer behind its consumer, and the count range tops out one above a capacity of 8 — the `BatchTooLarge` boundary is a single case rather than a swept dimension |
| GT5 | The cost of the two refusals | n/a — observation | `check` tests `count > capacity` before computing `headroom`, so a too-wide claim was refused without the allocation the gating read then made. `admits` has no such shortcut and so always paid it. The cost difference is gone with the allocation — two readings of one state that differed in cost on exactly the input that is cheapest to refuse, and now differ only in a comparison |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the whole of headroom, and the whole of check --'
command grep -E '^    self\.slowest|^      ring_seqno::free_slots|^      return Err|^    if count' ring_gating/src/lib.rs
echo '  -- the two hops it delegates through --'
command grep '^pub fn slowest' ring_cursor/src/lib.rs
command grep '^pub fn free_slots' ring_seqno/src/lib.rs
echo '  -- and the one test that compares the two arms --'
command grep 'fn an_ungated_ring_and_a_consumer_at_zero_disagree' ring_gating/tests/gating_test.rs
```

Live output:

```
  -- the whole of headroom, and the whole of check --
    self.slowest().map_or( self.capacity.get(), | slowest |
      ring_seqno::free_slots( producer, slowest, self.capacity )
    if count > self.capacity.get()
      return Err( RingError::BatchTooLarge { requested : count, capacity : self.capacity.get() } );
    if count > self.headroom( producer )
      return Err( RingError::Full );
    self.slowest().map( | s | s.advanced_by( self.capacity.get() as u64 ) )
  -- the two hops it delegates through --
pub fn slowest( cursors : &[ PaddedCursor ] ) -> Option< Seq >
pub fn free_slots( producer : Seq, consumer : Seq, capacity : Capacity ) -> usize
  -- and the one test that compares the two arms --
fn an_ungated_ring_and_a_consumer_at_zero_disagree_after_one_lap()
```

**Two conditionals, two delegations, and no arithmetic of its own.** Every line
that computes anything is in another crate; what this crate contributes is the
`map_or` choosing between them and the order of the two `if`s, which is
[`002`](002_check_orders_its_two_refusals.md)'s subject. The fourth arm is the
only place the two `map_or` arms are held against each other.
