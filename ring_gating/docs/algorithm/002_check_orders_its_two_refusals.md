# Algorithm: `check` Orders Its Two Refusals

### Scope

- **Purpose**: Show that `check`'s two tests must run in the order they are written, give the failure a reversal produces, and establish that `check` and `admits` never disagree.
- **Responsibility**: State the order and its consequence, give the sweep that pins the two readings together, and identify what a structural check adds over the outcome tests.
- **In Scope**: `check`, and `admits` as the reading it wraps.
- **Out of Scope**: Why a `Result` rather than a `bool` — see [`decisions/002`](../decisions/002_a_result_rather_than_a_bool.md).

### The Body

```rust
// ring_gating/src/lib.rs:283-294
pub fn check( &self, producer : Seq, count : usize ) -> Result< (), RingError >
{
  if count > self.capacity.get()
  {
    return Err( RingError::BatchTooLarge { requested : count, capacity : self.capacity.get() } );
  }
  if count > self.headroom( producer )
  {
    return Err( RingError::Full );
  }
  Ok( () )
}
```

Two comparisons against the same `count`, in a fixed order, over two different
right-hand sides: the ring's **total** capacity, then its **currently free**
capacity.

### Why the Order Is the Algorithm

`headroom( p ) <= capacity` always. So for any `count` exceeding capacity, *both*
tests fire — the claim is too wide for the ring, and it also does not fit right
now. Which error the caller sees is decided purely by which `if` runs first.

| Order | `check( full_ring, count > capacity )` returns | What a retry loop does |
|-------|-----------------------------------------------|------------------------|
| As written | `BatchTooLarge` — `is_configuration() == true` | Stops. Correct: no consumer's progress can ever make this fit |
| Reversed | `Full` — `is_configuration() == false` | **Spins forever** on a claim that can never succeed |

The reversal is not a wrong answer in the usual sense. `Full` is *true* — the ring
really is full. It is simply the less useful of two true statements, and the
caller's control flow keys off exactly that difference:

```rust
// ring_types/src/error.rs:110-125
pub const fn is_configuration( self ) -> bool
{
  match self
  {
    Self::CapacityZero
    | Self::CapacityNotPowerOfTwo( _ )
    | Self::BatchTooLarge { .. }
    | Self::PolicyUnsupported => true,
    Self::Full
    | Self::Empty
    | Self::Closed
    | Self::NameTaken
    | Self::NameUnknown => false,
  }
}
```

`RingError` also carries `is_transient` (`error.rs:146`), the complementary
predicate. A caller that keys on either gets the wrong answer under a reversal.
`is_configuration` is now an exhaustive match rather than the `matches!` this
excerpt used to show — a `matches!` over a positive list answers `false` for
every variant it does not name, so an added `RingError` variant would have
silently misclassified itself instead of failing to compile; see the
`Fix(ring_error_classification_not_exhaustive)` comment on the definition
itself for the full history.

### What Tests It, and What Does Not

```rust
// tests/gating_test.rs:241-250
fn the_two_failures_are_distinguished_at_the_boundary()
{
  let set = set_at( 4, &[ 0 ] );

  assert_eq!( set.check( Seq( 4 ), 4 ), Err( RingError::Full ) );
  assert!( set.check( Seq( 4 ), 5 ).unwrap_err().is_configuration() );
}
```

This is the outcome test, and it is exactly the overlap case: a **full** ring
(`producer` one lap ahead of a consumer at zero), asked for 4 — which fits the
ring but not right now — and then 5, which fits neither.

The manual plan adds a structural check on top, and says why:

> `the_two_failures_are_distinguished_at_the_boundary` asserts the outcome; this
> check asserts the structure that produces it, because a `headroom`-first
> implementation can be made to pass that one test by special case.

```sh
cd "$(git rev-parse --show-toplevel)"
grep -vE "^[[:space:]]*(///|//!)" ring_gating/src/lib.rs \
  | grep -E "RingError::(BatchTooLarge|Full)"
```

Live output:

```
      return Err( RingError::BatchTooLarge { requested : count, capacity : self.capacity.get() } );
      return Err( RingError::Full );
```

**Expected:** two hits, `BatchTooLarge` on the lower line number.

That is the right shape of check for this property. A property whose whole
content is "A comes before B" is checked by reading the order, not by enumerating
outcomes — an outcome test can always be satisfied by a special case that
reintroduces the bug everywhere it was not enumerated.

### G4 — `check` and `admits` Never Disagree

`admits` answers the same question without the reason:

```rust
// ring_gating/src/lib.rs:242-245
pub fn admits( &self, producer : Seq, count : usize ) -> bool
{
  count <= self.headroom( producer )
}
```

Note that `admits` has **no capacity test**. It cannot need one: `headroom` never
exceeds `capacity`, so `count > capacity` implies `count > headroom`, and the
first `if` in `check` is refining a refusal `admits` would have made anyway.

That equivalence is swept rather than argued:

```rust
// tests/gating_test.rs:257-274
let set = GatingSet::new( cap( 8 ), 1 );

for consumer in 0..16u64
{
  set.cursor( 0 ).unwrap().store( Seq( consumer ), Ordering::Release );

  for producer in consumer..consumer + 12
  {
    for count in 0..10usize
    {
      assert_eq!(
        set.check( Seq( producer ), count ).is_ok(),
        set.admits( Seq( producer ), count ),
        …
      );
    }
  }
}
```

16 × 12 × 10 = **1,920 states**, each asserting the two readings agree. The test's
own comment states the contract precisely: *"`check` adds the reason; it must not
add a different answer."*

The sweep's ranges are worth reading: `consumer` crosses two laps of an
8-capacity ring, `producer` runs from the consumer's position to 12 past it —
covering under-full, exactly-full and over-full — and `count` reaches 9, past the
capacity of 8, so the `BatchTooLarge` branch is inside the sweep rather than
beside it.

### The Zero Case

```rust
// tests/gating_test.rs:278-287
fn a_claim_of_zero_is_always_admitted()
{
  let full = set_at( 4, &[ 0 ] );

  assert!( full.check( Seq( 4 ), 0 ).is_ok() );
  assert!( full.admits( Seq( 4 ), 0 ) );
}
```

`0 > capacity` is false and `0 > headroom` is false for every headroom including
zero, so a zero-length claim always succeeds — on a full ring, on an empty one, on
an ungated one. The test's comment is explicit that this is *not* a special case
in the implementation and is asserted so that it does not become one.

**`ring_barrier` handles zero differently.** `Barrier::wait_for(
from, 0, … )` on a dependency-free barrier passes its `admits` gate — `0 <= 0` —
and then returns `Err( RingError::Empty )` from the `frontier().ok_or( … )` that
follows. So a zero-count operation is unconditionally `Ok` on the producer side
and can be `Err` on the consumer side. Neither is wrong; the asymmetry follows
from `GatingSet` having no wait operation at all. Recorded in
[`ring_seqno` `pattern/002`](../../../ring_seqno/docs/pattern/002_the_shared_fold_that_declines_an_identity.md)
§ *What `Option` Costs*.

### GT3 — Two Readings of One State, Swept

```
16 consumer positions x 12 producer offsets x 10 counts = 1,920 cases
check( p, n ).is_ok() == admits( p, n )   for every one
```

`admits` answers *may I* and `check` answers *why not*. They are computed from
the identical comparison, so a sweep can hold them against each other over the
whole reachable state space rather than at sampled points.

**Finding.** Two readings of one state, agreeing across a 1,920-case sweep; the reason for the second is that a `bool` cannot say *why*

---

### GT4 — What the Sweep Does Not Reach

```
producer offsets: 0 ..= 11, measured from the consumer  -> never behind it
counts:           0 ..= 9,  against a capacity of 8    -> one case above it
```

A sweep over three dimensions looks exhaustive, and is, within the box it
draws. The box's edges are where the interesting refusals live.

**Finding.** `check_and_admits_agree_across_the_whole_state_space` runs 16 consumer positions × 12 producer offsets × 10 counts, but the producer range starts at the consumer, so it never tests a producer behind its consumer, and the count range tops out one above a capacity of 8 — the `BatchTooLarge` boundary is a single case rather than a swept dimension

---

### GT5 — Only One of the Two Readings Can Refuse Cheaply

```
285:    if count > self.capacity.get()     <- no allocation reached
289:    if count > self.headroom( producer )  <- allocated, one crate over
242:  pub fn admits( &self, producer : Seq, count : usize ) -> bool
244:    count <= self.headroom( producer )    <- always allocated
```

The order of `check`'s two `if`s is usually discussed as a correctness question.
It was also a cost question, and the two readings answered it differently — until
`b7e075ca` removed the allocation from `ring_cursor::slowest`, at which point the
cost side of the argument evaporated and only the comparison count remained.

**Finding.** `check` tests `count > capacity` before computing `headroom`, so a too-wide claim was refused without the allocation the gating read then made. `admits` has no such shortcut and so always paid it. The cost difference is gone with the allocation — two readings of one state that differed in cost on exactly the input that is cheapest to refuse, and now differ only in a comparison

---


### Algorithms

| File | Relationship |
|------|--------------|
| [001_headroom_in_two_delegations.md](001_headroom_in_two_delegations.md) | The reading both branches consume |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_a_result_rather_than_a_bool.md](../decisions/002_a_result_rather_than_a_bool.md) | Why the reason is worth a `Result` |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_three_gating_readings.md](../item/001_the_three_gating_readings.md) | `check`, `admits` and `headroom` with their contracts |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_the_predicate_the_quantity_and_the_reason.md](../pattern/002_the_predicate_the_quantity_and_the_reason.md) | The three-rung shape this completes |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_reversing_the_two_refusals.md](../pitfall/002_reversing_the_two_refusals.md) | The reversal, and how far it gets |

### Sources

| File | Relationship |
|------|--------------|
| `ring_gating/src/lib.rs:247-294` | `check`, its doc, and its two ordered tests |
| `ring_gating/src/lib.rs:242-245` | `admits`, without the capacity test |
| `ring_types/src/error.rs:110-125` | `is_configuration` — what the distinction is for |
| `ring_types/src/error.rs:145-159` | `is_transient`, the complement |
| `tests/manual/readme.md` § M4 | The structural check on the order |

### Tests

| File | Relationship |
|------|--------------|
| `tests/gating_test.rs:222-229` | A claim wider than the ring is a configuration error |
| `tests/gating_test.rs:232-238` | One that merely does not fit yet is back-pressure |
| `tests/gating_test.rs:241-250` | Both, at the overlap where the order decides |
| `tests/gating_test.rs:253-275` | G4 — 1,920 states, `check` and `admits` agreeing |
| `tests/gating_test.rs:278-287` | A zero-length claim is always admitted |
