# Pitfall: Unwrapping the Empty Set to Zero

### Scope

- **Purpose**: Describe the one-line caller-side mistake that undoes this crate's central decision, and assess how well the family is protected against it.
- **Responsibility**: Show the mistake, trace its consequence, census every `Option< Seq >` disposal in the family, and identify what actually prevents it.
- **In Scope**: What a caller does with `slowest()` and `limit()`.
- **Out of Scope**: Why the crate resolves it to `capacity` — see [`decisions/001`](../decisions/001_capacity_for_an_empty_set.md).

### The Mistake

```rust
// A caller wanting a Seq rather than an Option< Seq >:
let bound = set.slowest().unwrap_or( Seq::ZERO );
```

It compiles. It reads as defensive. `Seq::ZERO` is the obvious neutral value, and
`unwrap_or` is the obvious way to reach it.

It also reintroduces exactly the conflation `ring_seqno::slowest` returns an
`Option` to prevent. After that line, *"no consumers"* and *"one consumer that
has read nothing"* are the same value, and the caller cannot recover the
difference.

### The Consequence

An ungated ring of capacity 4, with the caller computing its own headroom from
`bound`:

| Producer | True headroom | After `unwrap_or( ZERO )` |
|---------:|--------------:|--------------------------:|
| 0 | 4 | 4 |
| 2 | 4 | 2 |
| 4 | 4 | **0** |

At the first lap the caller blocks, waiting for a consumer that does not exist to
advance a cursor that does not exist. **A live-lock, not a wrong number** — no
event can ever change the condition being waited on.

The crate's module documentation states the same outcome from its own side:
*"Collapsing the two would deadlock every ungated ring at the first lap."*

### Why the Type Alone Does Not Stop It

`Option< Seq >` makes the distinction *visible*; it does not make it *survivable*.
`unwrap_or`, `unwrap_or_default` and `unwrap_or_else( \|\| Seq::ZERO )` are all one
call away, and `Seq` derives `Default` — so `unwrap_or_default()` compiles and
produces `Seq( 0 )`:

```sh
cd "$(git rev-parse --show-toplevel)"
grep '#\[ derive' ring_types/src/id.rs | head -1
# #[ derive( Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default ) ]
```

Live output:

```
#[ derive( Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default ) ]
```

The bare pattern `grep 'derive'` is not safe here — it false-positives on the
module doc's own *"the slot index **derive**d from it"* two lines above the
`Default` derive it means to find, and `head -1` would silently return that
prose line instead. `#\[ derive` anchors on the attribute syntax and skips it.

`Default` is on `Seq` for good reasons — `PaddedCursor::default` needs it, and
that is how every cursor starts at zero. But it means the dangerous resolution is
available through the shortest possible spelling.

### G16 — What the Family Actually Does

Every disposal of an `Option< Seq >` produced by a gating fold, family-wide:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -rE 'slowest\(\)|frontier\(\)|limit\(\)' ring_*/{src,tests}/*.rs \
  | grep -E 'unwrap|expect|map_or'
```

Live output:

```
ring_barrier/src/lib.rs:    self.frontier().map_or( 0, | frontier | from.distance_to( frontier ) )
ring_gating/src/lib.rs:    self.slowest().map_or( self.capacity.get(), | slowest |
ring_barrier/tests/barrier_test.rs:      let frontier = barrier.frontier().expect( "two dependencies" );
ring_claim/tests/claim_test.rs:          let limit = consumers.limit().expect( "one consumer" );
ring_gating/tests/gating_test.rs:  let limit = set.limit().expect( "one consumer, so a limit exists" );
ring_gating/tests/gating_test.rs:  let limit = set.limit().unwrap();
```

| Site | Disposal | Justification |
|------|----------|---------------|
| `ring_gating/src:224` | `map_or( self.capacity.get(), … )` | Documented decision — [`decisions/001`](../decisions/001_capacity_for_an_empty_set.md) |
| `ring_barrier/src:218` | `map_or( 0, … )` | Documented decision, the opposite one |
| `ring_barrier/tests:394` | `.expect( "two dependencies" )` | Precondition named |
| `ring_gating/tests:118` | `.expect( "one consumer, so a limit exists" )` | Precondition named |
| `ring_claim/tests:524` | `.expect( "one consumer" )` | Precondition named |
| `ring_gating/tests:307` | `.unwrap()` | **Bare** |

**Six sites, and not one is the mistake.** Both production sites resolve with an
identity their own crate argues for; three of the four test sites name the
precondition that makes the `Option` total.

The one bare `.unwrap()` is in `the_limit_is_exactly_where_headroom_reaches_zero`,
where the set is built one line above with a single consumer — so the precondition
holds and is locally obvious. It is inconsistent with its three siblings rather
than wrong, and the fix is the message the other three already carry.

### What Prevents It Today

| Mechanism | Present |
|-----------|:-------:|
| Type-level — a `Seq` that cannot be defaulted | ❌ `Seq : Default` |
| Lint — a deny on `unwrap_or_default` for this type | ❌ |
| A `headroom`-shaped method so callers never see the `Option` | ✅ **this is the real answer** |
| Convention — every `expect` names its precondition | ✅ 3 of 4 |
| A test asserting an ungated ring does not block | ✅ `tests/gating_test.rs:190-205` |

The third row is what actually protects the family, and it is a design property
rather than a check: **`headroom`, `admits` and `check` exist so that no producer
ever holds an `Option< Seq >` at all.** A caller reaching for `slowest()` is
already off the intended path — the method is public for diagnostics and for
`ring_barrier`'s cross-check, not for gating.

That reframes the pitfall. It is not "callers must remember to resolve the
`Option` correctly"; it is "a caller resolving the `Option` at all has usually
picked the wrong method." The one production consumer, `ring_claim`, never calls
`slowest` — it calls `headroom` three times and nothing else
([`item/001`](../item/001_the_three_gating_readings.md)).

### The Same Shape One Layer Down

`is_empty()` invites the parallel mistake in English rather than in code. It
reports *no consumers*, a permanent structural fact. Read as *nothing to read* —
which is what "empty" means for most collections — it inverts:

| `set.is_empty()` | Means | Does **not** mean |
|:----------------:|-------|-------------------|
| `true` | Ungated; every claim is admitted | The ring has no data |
| `false` | At least one consumer gates the producer | The ring has data |

No caller makes this mistake today; `is_empty` has no external caller at all.
Recorded because the name is inherited from `Vec` and carries `Vec`'s meaning,
which is the wrong one here.

### GT47 — The Family Never Disposes of an Option Carelessly

```
bare .unwrap() in ring_*/src/ ( non-doc ) : 0
224: self.slowest().map_or( self.capacity.get(), .. )   <- this crate's
ring_core:219, ring_mpsc:554, ring_bench:1066          <- expect, with a reason
ring_trace:279                                          <- unwrap_or_else, recovers
```

The pitfall is a disposal that would silently produce a wrong number. Measuring
how the family actually disposes of `Option` shows the habit that keeps it
hypothetical.

**Finding.** Every `Option< Seq >` disposal in the family is either a justified `map_or` or an `expect` with a stated precondition; four of the five `unwrap`-family calls name their reason, one does not. There is no bare `.unwrap()` in any of the 33 crates' production source, which is what makes the pitfall documented here hypothetical rather than latent

---

### GT48 — Which of the Two Guards Is Load-Bearing

```
an_ungated_ring_has_a_full_capacity_of_headroom
      -> passes for a variant that returns capacity at Seq::ZERO and 0 later
an_ungated_ring_and_a_consumer_at_zero_disagree_after_one_lap
      -> fails for it
```

Two tests stand between the crate and the pitfall. Only one of them survives a
partial version of the mistake.

**Finding.** Both ungated tests do, and only the second is load-bearing: a variant returning the capacity at sequence zero and zero after one lap passes `an_ungated_ring_has_a_full_capacity_of_headroom` and fails `an_ungated_ring_and_a_consumer_at_zero_disagree_after_one_lap`

---


### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_capacity_for_an_empty_set.md](../decisions/001_capacity_for_an_empty_set.md) | The resolution done correctly, inside the crate |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_reading_that_returns_a_position.md](../api/002_the_reading_that_returns_a_position.md) | The three dispositions, and why `limit` keeps its `Option` |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_three_gating_readings.md](../item/001_the_three_gating_readings.md) | The methods that keep the `Option` away from callers |
| [../item/002_the_four_accessors_and_the_limit.md](../item/002_the_four_accessors_and_the_limit.md) | `is_empty`'s contract, stated |

### Pitfalls

| File | Relationship |
|------|--------------|
| [002_reversing_the_two_refusals.md](002_reversing_the_two_refusals.md) | The other one-line mistake |

### Sources

| File | Relationship |
|------|--------------|
| `ring_gating/src/lib.rs:22-29` | The deadlock, named by the crate |
| `ring_types/src/id.rs:24-25` | `Seq : Default`, which makes `unwrap_or_default` compile |
| `ring_seqno/src/lib.rs:133-136` | The fold that returns the `Option` |

### Tests

| File | Relationship |
|------|--------------|
| `tests/gating_test.rs:190-205` | The ungated ring not blocking, across four producer positions |
| `tests/gating_test.rs:207-217` | The two states, shown disagreeing |
| `tests/gating_test.rs:307` | The one bare `unwrap` |
