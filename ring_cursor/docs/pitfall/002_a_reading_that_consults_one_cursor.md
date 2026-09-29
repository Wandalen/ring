# Pitfall: A Reading That Consults One Cursor

### Scope

- **Purpose**: Describe the reading that is correct for the initial state and wrong for every other, show why an ordinary test suite cannot see it, and record the fragility in the one test that can.
- **Responsibility**: Derive why `Seq::ZERO` hides the defect, give the two instruments that catch it, and name the value choice the third assertion silently depends on.
- **In Scope**: `free_slots`, `pending`, `may_claim` and their two loads each; `the_pair_reads_both_cursors_for_every_reading`; M4.
- **Out of Scope**: Whether the two loads are atomic *together*, which is [`algorithm/002`](../algorithm/002_three_readings_of_two_cursors.md) § How Far That Test Reaches.

### The Trap

Each of the pair's three readings is a two-cursor question:

```rust
pub fn pending( &self ) -> u64
{
  ring_seqno::pending( self.producer.load( GATING ), self.consumer.load( GATING ) )
}
```

and `ring_seqno::pending` is `consumer.distance_to( producer )`. Now substitute the
state a freshly constructed pair is in:

```rust
CursorPair::new( capacity )   // both cursors at Seq::ZERO
```

`Seq::ZERO.distance_to( producer )` is just `producer.0`. **So a `pending` that
never read the consumer at all would return the identical value** — for as long
as the consumer stays where it was constructed.

The same substitution works on the other two:

| Reading | Correct | One-cursor version | Agree while consumer is `ZERO`? |
|---------|---------|--------------------|:-------------------------------:|
| `pending` | `consumer.distance_to( producer )` | `producer.0` | ✅ |
| `free_slots` | `capacity - distance` | `capacity - producer.0` | ✅ |
| `may_claim` | `distance < capacity` | `producer.0 < capacity` | ✅ |

### Why an Ordinary Suite Cannot See It

A test that exercises a ring end to end **moves the consumer**, so it would
catch this. A test that checks one reading in isolation typically does not:

```rust
let pair = CursorPair::new( cap( 4 ) );
pair.producer().store( Seq( 3 ), Ordering::Release );
assert_eq!( pair.free_slots(), 1 );
```

That is the doc example for `free_slots` at `src/lib.rs:357-366`, and the
one-cursor version passes it. So does the `pending` doc example's producer store,
and so would most unit tests anyone would naturally write, because **the
constructor puts the consumer at the identity element of the operation being
tested**.

This is the general shape:

> A binary function whose second argument defaults to the operation's identity is
> indistinguishable from a unary function until a test moves that argument.

It is not specific to cursors. It is why "the test suite is green" and "the
function reads its second parameter" are independent facts.

### The Two Instruments

| # | Instrument | Sees | Kind |
|---|-----------|------|------|
| I1 | `the_pair_reads_both_cursors_for_every_reading` | That moving **only** the consumer changes all three readings | Behavioural |
| I2 | `tests/manual/readme.md` M4 | That the three bodies contain exactly **6** `self.{producer,consumer}.load` occurrences | Structural |

I1 is the right test and it is deliberately constructed — it moves the consumer
and nothing else, so a one-cursor reading cannot produce a different answer:

```rust
let pair = CursorPair::new( cap( 8 ) );
pair.producer().store( Seq( 8 ), Ordering::Release );

let ( free, pending, claimable ) = ( pair.free_slots(), pair.pending(), pair.may_claim() );
pair.consumer().store( Seq( 4 ), Ordering::Release );

assert_ne!( pair.free_slots(), free );
assert_ne!( pair.pending(), pending );
assert_ne!( pair.may_claim(), claimable );
```

I2 exists because I1 is behavioural and could be satisfied by a reading that
consults the consumer *incidentally*. Counting the loads is the direct check.

### M4 Was Wrong on First Run, and the Wrongness Was Instructive

M4's own note records it:

> `grep -o | wc -l` rather than `grep -c`, because each body puts both loads on
> one line and `-c` counts matching lines.

The first version used `grep -c`, got `3` against an expected `6`, and **that `3`
was read as a finding** — as evidence that three of the six loads were missing —
before the instrument was corrected.

A structural check that miscounts in the *pessimistic* direction manufactures
defects. It is the less dangerous direction, but only because someone looked; had
the expectation also been written as `3`, the check would have passed forever
while counting lines instead of loads.

### The Third Assertion Depends on a Value Nothing Documents

`may_claim` returns a `bool`, so `assert_ne!` against its previous value means
"it flipped". That only happens if the chosen positions **straddle the capacity
boundary**, and I1's do — barely:

| Step | `distance` | `free_slots` | `pending` | `may_claim` |
|------|-----------:|-------------:|----------:|:-----------:|
| producer 8, consumer 0 | 8 | 0 | 8 | `8 < 8` → **false** |
| producer 8, consumer 4 | 4 | 4 | 4 | `4 < 8` → **true** |

Change the producer's store from `Seq( 8 )` to `Seq( 4 )` and the first two
assertions still pass while the third fails **on correct code**:

| Step | `distance` | `may_claim` |
|------|-----------:|:-----------:|
| producer 4, consumer 0 | 4 | `4 < 8` → true |
| producer 4, consumer 4 | 0 | `0 < 8` → true |

**`producer = Seq( 8 )` is load-bearing and unremarked.** It is not a bug — the
test is correct as written — but the value was chosen to sit exactly at
`capacity`, and nothing in the test says so. A later edit "tidying" it to a
rounder number breaks a passing test for reasons that look unrelated to the
change.

This is a **Fragile Test** in the project's own sense: correct, and reliant on a
parameter value whose significance is not stated. The one-line fix is a comment;
the better fix is asserting the two `may_claim` values explicitly rather than
asserting they differ.

### What Still Is Not Covered

| # | Gap | Why |
|---|-----|-----|
| N1 | The **set** readings — `slowest` over a slice | I1 and M4 are `CursorPair`-only; `slowest`'s one test here, in `tests/allocation_test.rs`, asserts what it costs and not what it answers |
| N2 | Consumers that wire their own cursors | Four of ten do — see [`integration/002`](../integration/002_who_reads_a_cursor.md) § Two Shapes of Consumer. Nothing checks that *their* readings consult both |
| N3 | That a future fourth reading gets the same treatment | M4's expected `6` would have to be hand-edited to `8`, and nothing prompts it |

**N2 is where the risk actually lives now.** This crate's own readings are
checked twice over. The crates that build their own producer/consumer logic out
of loose `PaddedCursor`s inherit none of that, and `ring_batch::claim_gated`'s
gating check — the same two-cursor question — is verified only by `ring_batch`'s
own suite.

### CU43 — The Negative Case Does Not Occur Here, and the Document Says So by Counting

```
occurrences of the two-load expression: 3
the one reading that is not of that shape:
  cursors.iter().map( | c | c.load( GATING ) ).min()
```

All three readings consult both cursors. The one function that does not —
`slowest` — consults a slice instead, which is a different shape rather than a
one-cursor reading. That line has changed since this count was first taken (it
used to `collect()` into a `Vec` before folding, and now folds in place) without
changing what the count says: the shape is still a slice, and still not a single
cursor.

**Finding.** The claim "no reading here consults a single cursor" is established
by a count over a pattern, not by a reader's survey. That is the difference
between a pitfall that has been checked and one that has been asserted, and the
count is cheap enough to re-run on every corpus gate.

---

### CU44 — The Shape This Pitfall Warns About Exists in `ring_mpsc`

`ring_mpsc` holds a claimer's cursor and the ring's consumer cursor — two
cursors, two owners, no `CursorPair`.

**Finding.** That is exactly the arrangement this pitfall describes: the two
positions that must be read together are reachable independently, and nothing
requires them to be read together. Whether it is a defect there depends on
`ring_mpsc`'s own invariants, which this crate cannot see — but nothing checks
that a consumer emulating a pair is not simply the wrong shape for it, and no
document in either crate raises the question.

**Disposition:** declined — the finding's own text says this crate "cannot
see" `ring_mpsc`'s invariants, so there is nothing in `ring_cursor`'s `src/`
to fix; any real fix (a check, or a documented contract) belongs in
`ring_mpsc`, a different crate from this pass's three assigned crates
(`ring_config`, `ring_consume`, `ring_cursor`), and asserting in this crate's
own docs that `ring_mpsc`'s arrangement is a defect would be a claim this
corpus cannot verify.

---

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_three_readings_of_two_cursors.md](../algorithm/002_three_readings_of_two_cursors.md) | The three readings, and the separate question of whether their loads are atomic together |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_surface_that_decides.md](../api/002_the_surface_that_decides.md) | `pending`'s missing capacity parameter, which is what makes it look unary |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_cursor_pair.md](../data_structure/002_the_cursor_pair.md) | R4 — the readings' coverage, tabulated |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_who_reads_a_cursor.md](../integration/002_who_reads_a_cursor.md) | N2 — the four consumers holding loose cursors |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_a_cursor_from_new_to_shared.md](../lifecycle/001_a_cursor_from_new_to_shared.md) | Why `Seq::ZERO` is where both cursors start |

### Non Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_the_layout_claim_is_testable.md](../non_functional_requirement/001_the_layout_claim_is_testable.md) | The verification layering I1 and I2 belong to |

### Pitfalls

| File | Relationship |
|------|--------------|
| [001_the_obvious_implementation_forks_the_constant.md](001_the_obvious_implementation_forks_the_constant.md) | The other trap — a change that passes every check for a different reason |

### Sources

| File | Relationship |
|------|--------------|
| `ring_cursor/src/lib.rs:367-420` | The three readings, six loads |
| `ring_cursor/src/lib.rs:268-291` | `CursorPair::new` — both cursors at `Seq::ZERO`, in both `cfg` variants |
| `ring_seqno/src/lib.rs:73-115` | `consumer.distance_to( producer )`, whose identity element `ZERO` is |

### Tests

| File | Relationship |
|------|--------------|
| `tests/cursor_test.rs` § `the_pair_reads_both_cursors_for_every_reading` | I1, and the `Seq( 8 )` the third assertion needs |
| `tests/manual/readme.md` M4 | I2, and its corrected instrument |
| `src/lib.rs:357-366` | The `free_slots` doc example a one-cursor reading passes |
