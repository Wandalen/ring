# Workaround: The Iterator `ring_batch` Built Instead

### Scope

**Purpose:** Record the constraint `run`'s return type imposed on its intended
consumer, the function that consumer wrote to absorb it, what that duplication
costs, and the one-line change that deletes it.

**Responsibility:** `ring_batch::drain_order` read as a workaround for
`ring_index::run` — same computation, different container.

**In Scope:** `ring_index/src/lib.rs:118-122`;
`ring_batch/src/lib.rs:358-362`;
`ring_batch/tests/batch_test.rs:101-110`.

**Out of Scope:** the return type as a decision is
[`decisions/002`](../decisions/002_a_vec_where_an_iterator_would_do.md). What the
allocation measures is
[`non_functional_requirement/002`](../non_functional_requirement/002_the_one_allocation_and_the_zero_callers.md).

---

## The Two Functions, Side by Side

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A4 -F 'pub fn drain_order( claim : &BatchClaim, capacity : Capacity )' ring_batch/src/lib.rs
echo '  -- and the run it declined --'
command grep -m1 -A6 -F '/// assert_eq!( run( Seq( 0 ), 0, cap ), vec![] );' ring_index/src/lib.rs | tail -n 5
echo '  -- the iterator it folds over --'
command grep -m1 -A3 -F '  pub fn sequences( &self ) -> impl Iterator< Item = Seq > + use< >' ring_batch/src/lib.rs
```

Live output:

```
pub fn drain_order( claim : &BatchClaim, capacity : Capacity )
-> impl Iterator< Item = ( Seq, SlotIndex ) > + use< >
{
  claim.sequences().map( move | seq | ( seq, of( seq, capacity ) ) )
}
  -- and the run it declined --
#[ must_use ]
pub fn run( start : Seq, count : usize, capacity : Capacity ) -> Vec< SlotIndex >
{
  ( 0..count as u64 ).map( | n | of( start.advanced_by( n ), capacity ) ).collect()
}
  -- the iterator it folds over --
  pub fn sequences( &self ) -> impl Iterator< Item = Seq > + use< >
  {
    ( self.start.0..self.end().0 ).map( Seq )
  }
```

---

### IX47 — `drain_order` Is `run` With the Allocation Removed

**Finding.** The two bodies compute the same thing. Both walk a contiguous run of
sequences starting from a claim's first, fold each through `of`, and produce them
in order. The differences are exactly three, and only one of them is substantive:

- **The container.** `run` calls `.collect()`; `drain_order` returns the
  `Map` unevaluated. That is the constraint being absorbed.
- **The payload.** `drain_order` yields `( Seq, SlotIndex )` pairs; `run` yields
  bare slots. A drain needs both — the sequence to stamp against and the slot to
  read — so pairing avoids a second walk, but nothing stopped `run` from doing
  the same.
- **The arguments.** `drain_order` takes a `&BatchClaim` and derives the run from
  it; `run` takes `start` and `count` loose. `BatchClaim::sequences` is where the
  range actually comes from, and it is already an iterator.

That last point is what makes this a workaround rather than a redesign.
`ring_batch` had a lazy sequence iterator in hand, and `run`'s signature would
have required collecting it into a `Vec` of slots and then zipping it back
against the sequences it came from. The workaround is one `.map` that keeps both
halves together and allocates nothing.

The absorption is complete on the consumer's side and invisible from
`ring_index`. Nothing here records that its own `run` was offered and declined —
`run`'s doc comment still cites the feature `ring_batch` implements
([`item/002`](../item/002_the_two_that_nothing_calls.md) IX36).

---

### IX48 — The Workaround Carries the Only Mechanical Test of the One-Owner Rule

`ring_batch` did not silently reimplement the fold. It reimplemented the
*container* and then wrote a test asserting the fold is still `ring_index`'s:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -B4 -A8 -F 'fn drain_order_folds_through_ring_index_and_not_a_second_implementation()' ring_batch/tests/batch_test.rs | tail -n 10
echo '  -- every import of anything from ring_index, family-wide --'
command grep -r 'use ring_index' --include=*.rs . | command grep -v '^ring_index/'
echo "  ring_index::run mentions outside this crate: $( command grep -rn 'ring_index::run' --include=*.rs . | command grep -vc '^ring_index/' )"
echo "  drain_order mentions outside ring_batch:     $( command grep -rn 'drain_order' --include=*.rs . | command grep -vc '^ring_batch/' )"
```

Live output:

```
#[ test ]
fn drain_order_folds_through_ring_index_and_not_a_second_implementation()
{
  let capacity = cap( 8 );
  let batch = BatchClaim::new( Seq( 3 ), 20 );

  for ( seq, slot ) in drain_order( &batch, capacity )
  {
    assert_eq!( slot, ring_index::of( seq, capacity ), "the fold must be ring_index's" );
  }
  -- every import of anything from ring_index, family-wide --
ring_batch/src/lib.rs:use ring_index::of;
ring_store/tests/buffer_test.rs:use ring_index::of;
ring_store/src/lib.rs:use ring_index::of;
  ring_index::run mentions outside this crate: 0
  drain_order mentions outside ring_batch:     0
```

**Finding.** That test name is the one-owner pattern written as an assertion, and
it is the only place in the 33 crates where the rule is checked by anything other
than a reader. `ring_mpsc`, which broke the rule, has no counterpart
([`pattern/001`](../pattern/001_one_owner_for_one_arithmetic_fact.md) IX41).

The census underneath it is the honest part. Every import of anything from
`ring_index` anywhere in the family is `use ring_index::of;` — three of them, and
not one names `run`. Neither function has a caller outside its own crate: `run`
because it was declined, `drain_order` because the consumer that will drain a
batch has not been written yet. So the cost this
workaround avoids has never actually been paid: no allocation has been elided in
any running code, because no code runs.

That is not an argument against it. The shape was fixed before there were
callers, which is the cheapest moment to fix a shape, and it was fixed with a
test that pins the part that matters. What it does mean is that the duplication
is currently pure cost: two functions computing one thing, in two crates, with
zero consumers between them.

**Deletion condition.** Change `run` to return
`impl Iterator< Item = SlotIndex > + use< >`
([`decisions/002`](../decisions/002_a_vec_where_an_iterator_would_do.md) IX28)
and `drain_order` becomes
`run( claim.start(), claim.len(), capacity )` zipped with `claim.sequences()` —
or stays as it is, now delegating rather than duplicating. Because both functions
have zero external callers, the change breaks nothing outside the two crates'
own tests.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`workaround/001`](001_the_mask_that_lives_one_crate_up.md) | The other absorbed constraint, imposed on this crate rather than by it |
| [`decisions/002`](../decisions/002_a_vec_where_an_iterator_would_do.md) | The return type this works around, and the change that deletes it |
| [`pattern/001`](../pattern/001_one_owner_for_one_arithmetic_fact.md) | The rule this workaround's test is the only mechanical check of |
| [`item/002`](../item/002_the_two_that_nothing_calls.md) | `run`'s doc comment, still citing the feature that declined it |

### Sources

| Fact | Where |
|------|-------|
| `drain_order` and its body | `ring_batch/src/lib.rs:358-362` |
| `run` and its body | `ring_index/src/lib.rs:118-122` |
| The lazy sequence iterator it folds over | `ring_batch/src/lib.rs:162-165` |
| The one-owner assertion | `ring_batch/tests/batch_test.rs:101-110` |
| Zero external callers for either | Census above |

### Tests

| Test | Covers |
|------|--------|
| `ring_batch::drain_order_folds_through_ring_index_and_not_a_second_implementation` | The one-owner rule, mechanically, in the workaround's own crate — declared there, not here |
| `ring_batch::an_empty_claim_drains_to_nothing` | The `count = 0` case both functions handle — declared in `ring_batch/tests/`, not here |
| `an_empty_run_is_empty` | The same case on this side of the boundary |
| *(to create)* | An assertion that `drain_order` and `run` agree element for element |
