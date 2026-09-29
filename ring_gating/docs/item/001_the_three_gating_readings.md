# Item: The Three Gating Readings

### Scope

- **Purpose**: Catalogue `headroom`, `admits` and `check` — signature, contract, doctest and failure mode for each.
- **Responsibility**: State each item's contract as its caller must rely on it, and record what its doctest actually establishes.
- **In Scope**: The three producer-facing readings.
- **Out of Scope**: Their shared implementation chain — see [`algorithm/001`](../algorithm/001_headroom_in_two_delegations.md).

### `GatingSet::headroom`

```rust
#[ must_use ]
pub fn headroom( &self, producer : Seq ) -> usize     // :222
```

> How many slots a producer at `producer` may claim right now.
>
> Zero when the ring is full. A full capacity when the set is empty, since a
> ring nobody reads has no data anyone can lose.

| Property | Value |
|----------|-------|
| Range | `0 ..= capacity`, always |
| Empty set | `capacity`, at every `producer` |
| Staleness | Non-decreasing — the true value now is ≥ the value returned |
| Panics | None. `free_slots` saturates |

Its doctest is the crate's clearest, because it moves a consumer mid-scenario:

```rust
let set = GatingSet::new( Capacity::new( 4 ).unwrap(), 1 );
assert_eq!( set.headroom( Seq::ZERO ), 4 );
assert_eq!( set.headroom( Seq( 3 ) ), 1 );
assert_eq!( set.headroom( Seq( 4 ) ), 0, "a full lap ahead" );

set.cursor( 0 ).unwrap().store( Seq( 2 ), Ordering::Release );
assert_eq!( set.headroom( Seq( 4 ) ), 2, "the consumer released two slots" );
```

Four assertions covering empty, partial, full, and *recovery from full*. The last
is the one a smoke-check doctest would omit and the one that would catch a
`headroom` that cached its answer.

### `GatingSet::admits`

```rust
#[ must_use ]
pub fn admits( &self, producer : Seq, count : usize ) -> bool     // :242
{
  count <= self.headroom( producer )
}
```

| Property | Value |
|----------|-------|
| `count == 0` | Always `true`, on any set in any state |
| `count > capacity` | Always `false` — no capacity test needed, because `headroom <= capacity` |
| Staleness | `true` stays true; `false` may become true |
| Panics | None |

```rust
let set = GatingSet::new( Capacity::new( 4 ).unwrap(), 1 );
assert!( set.admits( Seq::ZERO, 4 ) );
assert!( !set.admits( Seq::ZERO, 5 ), "wider than the ring" );
assert!( !set.admits( Seq( 2 ), 3 ), "only two slots left" );
```

The second and third assertions are the two distinct reasons a claim is refused,
returning the same `false` — which is exactly the ambiguity `check` exists to
remove.

### `GatingSet::check`

```rust
pub fn check( &self, producer : Seq, count : usize ) -> Result< (), RingError >   // :283
```

The only method without `#[ must_use ]`, because `Result` carries its own.

| Outcome | When | Caller |
|---------|------|--------|
| `Ok( () )` | `count <= headroom( producer )` | Proceed — but re-read before acting; see below |
| `Err( BatchTooLarge { requested, capacity } )` | `count > capacity` | **Stop.** `is_configuration() == true` |
| `Err( Full )` | Fits the ring, not the moment | **Retry.** `is_transient() == true` |

The order of the two tests is load-bearing and is the subject of
[`algorithm/002`](../algorithm/002_check_orders_its_two_refusals.md).

**`Ok` is the weakest of the three answers.** `BatchTooLarge` is permanent and
`Full` is a floor, but `Ok` describes a moment that may already have passed — and
it is exactly the answer a caller wants to act on. That is why the family's one
production retry loop puts the headroom read in its loop condition rather than
calling `check` at all
([`decisions/002`](../decisions/002_a_result_rather_than_a_bool.md)).

### The Three Together

| | `headroom` | `admits` | `check` |
|---|---|---|---|
| Answers | how many | whether | whether, and why not |
| Returns | `usize` | `bool` | `Result< (), RingError >` |
| Tests capacity separately | ❌ | ❌ | ✅ |
| Callers outside this crate | 3 (`ring_claim`) | 0 | 0 |
| `#[ must_use ]` | ✅ | ✅ | inherited |

```sh
cd "$(git rev-parse --show-toplevel)"
for f in ring_*/src/*.rs; do
  grep -vE "^[[:space:]]*(///|//!)" "$f" | grep -E '\.(headroom|admits|check)\(' | sed "s|^|$f:|"
done
```

Live output:

```
ring_barrier/src/lib.rs:    ring_wait::wait_until( kind, spins, || self.admits( from, count ) )?;
ring_claim/src/lib.rs:    self.consumers.headroom( self.claimed() )
ring_claim/src/lib.rs:    while count <= self.consumers.headroom( current )
ring_claim/src/lib.rs:    while let granted @ 1.. = max.min( self.consumers.headroom( current ) )
ring_gating/src/lib.rs:    count <= self.headroom( producer )
ring_gating/src/lib.rs:    if count > self.headroom( producer )
ring_mpsc/src/lib.rs:    self.claimer.headroom()
```

**Seven hits, and only three of them are this crate's methods.** The census has
to be read rather than counted, because two other types carry same-named methods:

| Hit | Receiver | Counts? |
|-----|----------|:-------:|
| `ring_claim:384,437,488` | `self.consumers` — a `&GatingSet` | ✅ ×3 |
| `ring_gating:72,81` | `self` — internal, `admits` and `check` calling `headroom` | ❌ |
| `ring_barrier:285` | `self` — `Barrier::admits`, a different type | ❌ |
| `ring_mpsc:805` | `self.claimer` — `Claimer::headroom`, a forward | ❌ |

(Line numbers are positions in the doc-stripped stream, not in the file;
`ring_claim`'s three are `:353`, `:404` and `:449` as written.)

`ring_barrier`'s hit is the sharpest reminder that a method-name grep is not a
call census — [`integration/002`](../integration/002_the_other_half_of_feature_178.md)
records that the two crates deliberately share the name `admits` for the same
question asked from opposite sides.

Three methods, one production caller, and it uses only the first. The two richer
readings are exercised entirely by this crate's own tests — a 1,920-state sweep
proves `check` and `admits` agree, and nothing outside asks either of them.

### GT27 — The Doc Examples Are Scenarios

```
store calls inside doc comments : 5, across 4 examples
63, 66   <- the module-level example, twice
192, 218, 317
```

A doctest that only constructs and asserts is a smoke check. These drive a
cursor to a chosen position first, which makes them small specifications of
behaviour rather than proof that the code links.

**Finding.** They are *scenarios* rather than smoke checks — four of the twelve drive a cursor with `store` before asserting, and the module-level one does it twice, for five `store` calls in all

---

### GT28 — One Reading, Three Entry Points

```
222:  headroom  -> the reading
244:  admits    -> count <= self.headroom( producer )
283:  check     -> the same comparison, plus a reason
```

Three public methods, one underlying question. The two derived ones differ from
each other only in what they return when the answer is no.

**Finding.** `admits` calls `headroom`; `check` calls `headroom`. There is one reading of the state and three entry points to it, and the two derived ones differ from each other only in what they do with an identical comparison

---


### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_headroom_in_two_delegations.md](../algorithm/001_headroom_in_two_delegations.md) | The chain all three descend |
| [../algorithm/002_check_orders_its_two_refusals.md](../algorithm/002_check_orders_its_two_refusals.md) | Why `check`'s two tests are ordered |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_reading_that_returns_a_position.md](../api/002_the_reading_that_returns_a_position.md) | Which of these answers survives being stale |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_a_result_rather_than_a_bool.md](../decisions/002_a_result_rather_than_a_bool.md) | Why `check` exists, and who does not call it |

### Items

| File | Relationship |
|------|--------------|
| [002_the_four_accessors_and_the_limit.md](002_the_four_accessors_and_the_limit.md) | The eight items these three read through |

### Sources

| File | Relationship |
|------|--------------|
| `ring_gating/src/lib.rs:202-228` | `headroom`, doc and body |
| `ring_gating/src/lib.rs:230-245` | `admits` |
| `ring_gating/src/lib.rs:247-294` | `check` |
| `ring_claim/src/lib.rs:384,437,488` | The three external `headroom` calls |

### Tests

| File | Relationship |
|------|--------------|
| `tests/gating_test.rs:161-188` | `headroom` across a capacity, and past saturation |
| `tests/gating_test.rs:252-275` | `check` and `admits` agreeing over 1,920 states |
| `tests/gating_test.rs:277-287` | `count == 0` on a full ring |
