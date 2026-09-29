# Decision: `capacity` for an Empty Set

### Scope

- **Purpose**: Record why `headroom` resolves an empty gating set to a full capacity rather than to zero, and what the alternative costs.
- **Responsibility**: State the decision, trace the deadlock the alternative produces, identify the two tests that defend it, and show that the same `Option` is resolved the opposite way one crate over.
- **In Scope**: The `map_or` identity in `headroom`.
- **Out of Scope**: The chain that produces the `Option` — see [`algorithm/001`](../algorithm/001_headroom_in_two_delegations.md).

### The Decision

```rust
// ring_gating/src/lib.rs:224-227
self.slowest().map_or( self.capacity.get(), | slowest |
{
  ring_seqno::free_slots( producer, slowest, self.capacity )
} )
```

The crate states it before any code:

> `ring_cursor::slowest` returns `None` for an empty set rather than `Seq::ZERO`,
> and this crate carries that distinction through: a ring nobody is reading has no
> data anyone can lose, so `GatingSet::headroom` returns a full capacity rather
> than zero. Collapsing the two would deadlock every ungated ring at the first
> lap.

### The Two States That Look Alike

| State | `slowest()` | What it means |
|-------|-------------|---------------|
| No consumers | `None` | Nobody will ever read a slot; no data can be lost |
| One consumer, at zero | `Some( Seq::ZERO )` | Somebody will read slot 0 and has not yet |

Both are "the smallest sequence anyone has reached is nothing/zero". They are
opposite situations. An `Option`-free fold returning `Seq::ZERO` for the empty
case makes them literally indistinguishable downstream, and the distinction is
the one this decision turns on.

### The Deadlock, Traced

Suppose `headroom` resolved `None` to `0`. Walk an ungated ring of capacity 4:

| Producer at | `slowest()` | `headroom` under the real rule | …under `None → 0` |
|------------:|-------------|-------------------------------:|------------------:|
| 0 | `None` | 4 | `free_slots( 0, 0, 4 )` = 4 |
| 2 | `None` | 4 | `free_slots( 2, 0, 4 )` = 2 |
| 4 | `None` | 4 | `free_slots( 4, 0, 4 )` = **0** |

At the first lap boundary the fake consumer at zero is exactly one lap behind, so
the gate refuses. And it refuses **permanently**: no consumer exists to advance
the cursor that is blocking, so a retry loop keyed on `admits` spins forever
against a condition nothing can change.

That is the worst available failure shape — not a wrong number, but a live-lock
whose cause is a consumer that does not exist. The crate's own test says so in
its comment: *"Were it one, this would read 0 after a lap and every ungated ring
would deadlock."*

### What Defends It

Two tests, and the second exists for nothing else:

```rust
// tests/gating_test.rs:196-204
let ungated = GatingSet::new( cap( 4 ), 0 );

assert_eq!( ungated.slowest(), None );
assert_eq!( ungated.limit(), None );
for producer in [ 0u64, 4, 1_000, u32::MAX as u64 ]
{
  assert_eq!( ungated.headroom( Seq( producer ) ), 4, "at producer {producer}" );
  assert!( ungated.admits( Seq( producer ), 4 ) );
}
```

The producer sweep is doing real work. `0` is the case where both rules agree;
`4` is the first lap, where they part; `1_000` is 250 laps in; `u32::MAX as u64`
is past the point where a narrowing bug would show. A test fixed at producer `0`
would pass under the broken rule.

```rust
// tests/gating_test.rs:212-216
let ungated = GatingSet::new( cap( 4 ), 0 );
let gated_at_zero = set_at( 4, &[ 0 ] );

assert_eq!( ungated.headroom( Seq::ZERO ), gated_at_zero.headroom( Seq::ZERO ), "same while empty" );
assert_ne!( ungated.headroom( Seq( 4 ) ), gated_at_zero.headroom( Seq( 4 ) ), "and different after a lap" );
```

This one is adversarial in the strict sense: it constructs the two states a
`Seq::ZERO`-for-empty implementation would conflate and asserts they *disagree*.
Its `assert_eq!` on the first line is as important as the `assert_ne!` on the
second — it pins down that the two states are genuinely indistinguishable until
the lap, so the `assert_ne!` is testing the boundary rather than a difference
that was there all along.

| Broken rule | Caught by |
|-------------|-----------|
| `None → 0` | Both, at producer ≥ 4 |
| `None → capacity`, but only checked at producer 0 | Neither — that is the real rule |
| `None → capacity / 2` | `:182-197`, at every producer |

### The Same `Option`, Resolved the Other Way

`ring_barrier` reaches the identical `Option< Seq >` from the identical fold and
resolves it to the opposite value:

| Crate | Method | Empty → | Because |
|-------|--------|--------:|---------|
| `ring_gating` | `headroom` | `capacity` (max) | No consumer means no data can be lost |
| `ring_barrier` | `frontier` | `0` (min) | No dependency means nothing has been produced |

Both are correct, and they are correct for the same structural reason: the
question each asks has a different *safe* answer when there is nothing to fold
over. A producer with no consumers is unconstrained; a consumer with no upstream
dependency has nothing available.

**This is the whole argument for `ring_seqno::slowest` returning `Option`.** A fold
that picked either identity for its callers would be right for one of these two
and wrong for the other — and "wrong" here means a permanent deadlock in one
direction or a read of unwritten memory in the other. Recorded from the fold's
side in
[`ring_seqno` `pattern/002`](../../../ring_seqno/docs/pattern/002_the_shared_fold_that_declines_an_identity.md).

There is a third resolution in the family and a fourth, both listed in that same
document: `ring_barrier:286` turns it into `Err( RingError::Empty )`, and
`ring_consume:342` resolves to `0`. Four call sites, four dispositions, one
`Option`.

### Why `limit` Does Not Get the Same Treatment

`limit` propagates the `Option` instead of resolving it, and the asymmetry is
deliberate — *how much room* has an answer for an ungated ring, *where do I
block* does not. Argued in
[`api/002`](../api/002_the_reading_that_returns_a_position.md).

### GT15 — The Rejected Alternative Has Its Own Test

```
191:fn an_ungated_ring_has_a_full_capacity_of_headroom()
208:fn an_ungated_ring_and_a_consumer_at_zero_disagree_after_one_lap()
210:  // The two states that a `Seq::ZERO`-for-empty implementation would conflate,
211:  // shown disagreeing.
```

A decision is only as good as the thing that would notice it being reversed. The
second test names the rejected implementation in a comment and then demonstrates
the two states it would have merged.

**Finding.** It is caught by exactly two tests, one of which exists only to catch it — `an_ungated_ring_and_a_consumer_at_zero_disagree_after_one_lap` builds both states the rejected implementation would conflate and asserts they agree while empty and differ after a lap

---

### GT16 — One Rule, Three Phrasings, One File

```
  //! the empty set is not a consumer at zero
   /// consumers of zero is legal and means ungated       ( new )
   /// a full capacity when the set is empty              ( headroom )
```

The rule is short enough to restate, so it was restated. None of the three
points at either of the others.

**Finding.** Stated three times in one 325-line file — the module doc's "the empty set is not a consumer at zero", `new`'s "consumers of zero is legal and means ungated", and `headroom`'s "a full capacity when the set is empty". Three phrasings of one decision, none citing the others

---


### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_reading_that_returns_a_position.md](../api/002_the_reading_that_returns_a_position.md) | The three dispositions of this `Option` within the crate |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_headroom_in_two_delegations.md](../algorithm/001_headroom_in_two_delegations.md) | Where the `Option` comes from, and what the crate adds |

### Decisions

| File | Relationship |
|------|--------------|
| [002_a_result_rather_than_a_bool.md](002_a_result_rather_than_a_bool.md) | The crate's other stated decision |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_the_bound_is_the_minimum_and_only_the_minimum.md](../invariant/001_the_bound_is_the_minimum_and_only_the_minimum.md) | The rule that has no empty-set case |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_unwrapping_the_empty_set_to_zero.md](../pitfall/001_unwrapping_the_empty_set_to_zero.md) | The same mistake made one layer up, by a caller |

### Sources

| File | Relationship |
|------|--------------|
| `ring_gating/src/lib.rs:22-29` | The decision, stated |
| `ring_gating/src/lib.rs:221-228` | The decision, implemented |
| `ring_barrier/src/lib.rs:218` | The opposite resolution of the same `Option` |
| `ring_seqno/src/lib.rs` § `slowest` | The fold that declines to choose |

### Tests

| File | Relationship |
|------|--------------|
| `tests/gating_test.rs:190-205` | The ungated set across four producer positions |
| `tests/gating_test.rs:207-217` | The two conflatable states, shown disagreeing |
