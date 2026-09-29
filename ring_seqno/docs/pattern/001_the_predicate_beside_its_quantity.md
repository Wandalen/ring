# Pattern: The Predicate Beside Its Quantity

### Scope

- **Purpose**: Name the shape by which a boolean question is shipped alongside the count it derives from, and give the conditions under which it is worth the duplication.
- **Responsibility**: State the shape, show its three instantiations in this family, and give the argument for each half existing.
- **In Scope**: `may_claim` beside `free_slots`, and the same pairing where it recurs.
- **Out of Scope**: The performance argument against collapsing them — see [`pitfall/001`](../pitfall/001_implementing_may_claim_with_laps_between.md).

### The Shape

> When a crate computes a quantity that callers overwhelmingly test against zero,
> export the test as its own function. Implement it independently in the cheapest
> form rather than as `quantity() != 0`, and pin the two together with a test.

Three parts, all required:

| Part | Why |
|------|-----|
| A separate function | Callers ask a yes/no question; making them phrase it as arithmetic invites each to phrase it differently |
| Implemented independently, cheaply | The predicate is usually strictly cheaper than the quantity — a comparison rather than a subtraction, a subtraction rather than a division |
| Pinned by a test | Two implementations of one fact will drift unless something asserts they agree |

Omitting the third turns the pattern into plain duplication.

### The Instantiation Here

```rust
// ring_seqno/src/lib.rs:73-76 and 95-99
pub fn may_claim( producer : Seq, consumer : Seq, capacity : Capacity ) -> bool
{
  consumer.distance_to( producer ) < capacity.get() as u64
}

pub fn free_slots( producer : Seq, consumer : Seq, capacity : Capacity ) -> usize
{
  let in_flight = consumer.distance_to( producer );
  ( capacity.get() as u64 ).saturating_sub( in_flight ) as usize
}
```

`may_claim` is `free_slots() > 0`, and is not written that way. It is a
comparison where `free_slots` is a saturating subtraction, and — more importantly
— `may_claim` never narrows. That used to be why the two *could* disagree on a
target where `usize` is under 64 bits: `free_slots` narrowed its input before
subtracting, `may_claim` did not narrow at all. `free_slots` now widens first
and narrows only the already-`capacity`-bounded result, so the independent
implementations can no longer drift apart on any target
([`nfr/002`](../non_functional_requirement/002_the_arithmetic_must_survive_a_narrow_usize.md)).
The independence that makes the pattern valuable was also what let them drift,
once.

The third part is present:

```rust
// tests/seq_test.rs:71-88
fn free_slots_agrees_with_may_claim_across_two_laps()
{
  for consumer in 0..8u64 {
    for producer in consumer..consumer + 20 {
      let free = free_slots( Seq( producer ), Seq( consumer ), c );
      let claimable = may_claim( Seq( producer ), Seq( consumer ), c );
      assert_eq!( free > 0, claimable, … );
```

160 pairs across two full laps. That is what makes the duplication safe rather
than merely convenient — and its limit is the sweep's range, which is why the
width hazard escapes it.

### It Recurs at Three Tiers

The shape is not local to this crate. It appears again at each level, and the
pairing survives every wrapper:

| Tier | Quantity | Span type | Predicate | Predicate's body |
|:----:|----------|:---------:|-----------|------------------|
| 1 `ring_seqno` | `free_slots` | `usize` | `may_claim` | independent comparison |
| 2 `ring_cursor` | `CursorPair::free_slots` | `usize` | `CursorPair::may_claim` | `ring_seqno::may_claim` — two loads, not four |
| 3 `ring_gating` | `headroom` | `usize` | `admits( producer, count : usize )` | `count <= self.headroom( producer )` |
| 3 `ring_barrier` | `available` | **`u64`** | `admits( from, count : u64 )` | `count <= self.available( from )` |

**The two tier-3 instantiations disagree about what a span is measured in.** Both
count sequences; one returns `usize` and the other `u64`, so their predicates take
different argument types for the same kind of quantity. The split is not arbitrary —
`headroom` is bounded by capacity, which is a `usize`, while `available` is bounded
by nothing and stays in `Seq`'s own width — but it means a caller holding both
barriers must convert between them, and it is the family-wide fault line recorded
in [`type/001`](../type/001_what_a_span_is_measured_in.md). The pattern propagates
the shape faithfully and carries the vocabulary's inconsistency along with it.

Tier 2's saving is concrete and is the crate's own stated reason:

> Exactly [`free_slots`] being non-zero, expressed as the question a caller
> actually asks. Kept as its own method because the two readings answer different
> questions.

`CursorPair::may_claim` performs **two** atomic loads. `free_slots() != 0` at a
call site would perform two as well — but a caller wanting both readings would
perform four, and get an inconsistent pair if a cursor moved between them. The
predicate is not merely cheaper; it is *atomic in a way the composition is not*.

**At tier 3 the pattern is present in form and one half is unused.** `Barrier::admits`
calls `Barrier::available`, and `available` has no caller outside `ring_barrier`
at all — `ring_consume`, which holds a `Barrier`, re-expands its body inline
through `ring_seqno::pending` instead (**Finding SQ20**,
[`integration/002`](../integration/002_how_the_fold_crossed_four_tiers.md)). So the
quantity half of the tier-3 pair is exported, correct, and reached only by its own
predicate.

That is the pattern's failure mode: it doubles the surface, and if consumers
bypass one half the crate carries an export nothing outside it uses.

### When the Pattern Is Not Worth It

| Condition | Then |
|-----------|------|
| The quantity is as cheap as the predicate | Just export the quantity. Two functions buy nothing |
| Callers want the count more often than the test | The predicate accumulates no callers and becomes dead surface — tier 3's `available`, inverted |
| No test pins the two | Plain duplication. Two implementations of one fact, free to drift |
| The predicate is not more atomic than the composition | Tier 1's `may_claim` over plain values gains only cheapness; tier 2's gains consistency, which is worth more |

The last row explains why the pattern is stronger the further up it goes. At tier
1 it saves a subtraction. At tier 2 it saves a torn read.

### Why Not a Single Function Returning Both

```rust
pub fn claim_state( … ) -> ( bool, usize )   // not done
```

One computation, both answers, guaranteed consistent. It was not taken, and the
reasons are worth recording:

| Against | Detail |
|---------|--------|
| Callers wanting one pay for both | `ring_wait:241`'s spin predicate wants only the `bool`, evaluated in a tight loop |
| `#[ must_use ]` weakens | A tuple discarded in part is not caught |
| The names disappear | `may_claim` and `free_slots` are the domain's words; `.0` and `.1` are not |

The first is decisive given where `may_claim` ends up — inside
`wait_until( kind, spins, || pair.may_claim() )`.

### SQ39 — The Same Pair at Three Tiers

The shape survives two crate boundaries and loses a half at the third:

```
ring_seqno      may_claim        / free_slots        both used
ring_cursor   may_claim()      / free_slots()      both used
ring_barrier  admits()         / available()       available unused outside
```

**Finding.** The predicate/quantity pair recurs at three tiers of the family, and at the third tier one half of it is unused.

---

### SQ40 — A Sweep That Never Goes Backward

The loop bounds are the finding — the backward half of the domain is not in them:

```
for consumer in 0..8u64
  for producer in consumer..consumer + 20
                  ^^^^^^^^ starts at the consumer, never below it
```

**Finding.** The sweep that pins the predicate to its quantity iterates the producer forward only, so the saturating branch both functions depend on is exercised by three point assertions and never by the sweep.

---

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_four_readings_of_one_subtraction.md](../algorithm/001_four_readings_of_one_subtraction.md) | The equivalence lattice the pattern sits inside |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_three_capacity_readings.md](../item/001_the_three_capacity_readings.md) | Both halves, with coverage |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_how_the_fold_crossed_four_tiers.md](../integration/002_how_the_fold_crossed_four_tiers.md) | Why tier 3's quantity half went unused |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_the_arithmetic_must_survive_a_narrow_usize.md](../non_functional_requirement/002_the_arithmetic_must_survive_a_narrow_usize.md) | Where the independent implementations diverge |

### Patterns

| File | Relationship |
|------|--------------|
| [002_the_shared_fold_that_declines_an_identity.md](002_the_shared_fold_that_declines_an_identity.md) | The complementary shape — a decision pushed out rather than kept in |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_implementing_may_claim_with_laps_between.md](../pitfall/001_implementing_may_claim_with_laps_between.md) | What collapsing the pair costs |

### Sources

| File | Relationship |
|------|--------------|
| `ring_seqno/src/lib.rs:55-99` | Tier 1's pair |
| `ring_cursor/src/lib.rs:351-420` | Tier 2's, with the stated argument |
| `ring_gating/src/lib.rs:217-245` | Tier 3's `headroom`/`admits` |
| `ring_barrier/src/lib.rs:214-241` | Tier 3's `available`/`admits` |
| `ring_wait/src/lib.rs:241` | The spin predicate that wants only the `bool` |

### Tests

| File | Relationship |
|------|--------------|
| `tests/seq_test.rs:71-88` | The third part of the pattern — 160 pairs |
| `tests/seq_test.rs:58-67` | The predicate's own boundary |
| `tests/seq_test.rs:92-101` | The quantity's own range |
