# Algorithm: `headroom` in Two Delegations

### Scope

- **Purpose**: Trace `headroom` from its two-line body to the arithmetic it ultimately performs, and identify the single decision the crate contributes.
- **Responsibility**: Give the full call chain with the crate and tier of each step, name what each step adds, and show what the depth costs.
- **In Scope**: `headroom`, and `limit` as its position-valued sibling.
- **Out of Scope**: The empty-set identity as a *decision* — see [`decisions/001`](../decisions/001_capacity_for_an_empty_set.md).

### The Body

```rust
// ring_gating/src/lib.rs:221-228
pub fn headroom( &self, producer : Seq ) -> usize
{
  self.slowest().map_or( self.capacity.get(), | slowest |
  {
    ring_seqno::free_slots( producer, slowest, self.capacity )
  } )
}
```

Two calls and a `map_or`. `slowest` is itself one line:

```rust
// ring_gating/src/lib.rs:197-200
pub fn slowest( &self ) -> Option< Seq >
{
  ring_cursor::slowest( &self.cursors )
}
```

### G3 — The Full Chain

| Step | Crate | Tier | What it adds |
|------|-------|:----:|--------------|
| `GatingSet::headroom` | `ring_gating` | 4 | The empty-set identity, and the argument order `( producer, slowest )` |
| `GatingSet::slowest` | `ring_gating` | 4 | Nothing — a one-line forward over `&self.cursors` |
| `ring_cursor::slowest` | `ring_cursor` | 3 | The atomic loads, at `GATING` ordering, and the fold itself — `Iterator::min` over the loaded values, no `ring_seqno` call and no allocation since commit `b7e075ca` |
| `ring_seqno::free_slots` | `ring_seqno` | 1 | `capacity.saturating_sub( in_flight )`, and a `u64 → usize` narrowing |
| `Seq::distance_to` | `ring_types` | 0 | `later.0.saturating_sub( self.0 )` — the one subtraction |

**Five steps, four crates, four tiers, one subtraction** — `ring_cursor::slowest`
used to delegate a sixth step to `ring_seqno::slowest`; commit `b7e075ca` folded
the cursors directly instead, and that step no longer exists in the call graph
(already correctly recorded as "gone — no longer a call" in
[`nfr/001`](../non_functional_requirement/001_every_gating_read_allocates_nothing.md),
which this table had not been reconciled with until now). Verify the tiers with
`grep -m1 -oE 'Tier [0-9]+' ring_{gating,cursor,seq,types}/src/lib.rs` —
each crate states its own in its module documentation, and the chain descends
4 → 3 → 1 → 0 without skipping upward.

Every layer adds exactly one thing, and the two that add the most are the two
furthest from this crate: `ring_cursor` supplies the memory ordering, and
`ring_types` supplies the saturation.

The chain is not accidental depth. Each boundary corresponds to a decision that
must be made once for the whole family:

| Boundary | The decision it isolates |
|----------|--------------------------|
| `ring_gating` → `ring_cursor` | What ordering a gating read uses — stated once, as `GATING` |
| `ring_cursor` → `ring_seqno` | Whether the fold is over cursors or over values |
| `ring_seqno` → `ring_types` | Whether distance saturates or signs |

Collapsing any one of them duplicates its decision. Manual check M3 asserts the
first two are not collapsed, with an expected output of exactly
`1 ring_cursor::slowest` and `1 ring_seqno::free_slots`.

### What the Crate Contributes

Exactly one thing: **`map_or( self.capacity.get(), … )`**.

That is the whole of `ring_gating`'s arithmetic contribution to `headroom` — the
choice that an empty set means *full capacity* rather than zero. `ring_barrier`
makes the opposite choice at the same point in its own chain, and the fold returns
`Option` specifically so that both can.

Everything else in the body is plumbing: unwrapping a borrow, forwarding a slice,
naming two arguments in an order.

### The Argument Order Is the Second Contribution

```rust
ring_seqno::free_slots( producer, slowest, self.capacity )
```

`free_slots( producer, consumer, capacity )` — the producer first. The `slowest`
consumer is passed in the consumer position, which is the only sensible reading,
and nothing checks it: both parameters are `Seq`, so a swap compiles.

A swapped call would compute `free_slots( slowest, producer, cap )` =
`cap.saturating_sub( producer.distance_to( slowest ) )`, and for any producer at
or ahead of the slowest consumer that inner distance saturates to `0`, giving a
**full capacity of headroom at every position** — the gate would never refuse
anything.

That failure is loud rather than silent, and the acceptance test catches it
immediately (`admitted` would exceed `CAPACITY` on the first lap, tripping the
in-loop assertion at `gating_test.rs:79`). Worth recording anyway, because the
family has no type-level protection against the swap — see
[`ring_seqno` `api/002`](../../../ring_seqno/docs/api/002_the_argument_order_split.md).

### `limit` Is the Same Chain, Multiplied

```rust
// ring_gating/src/lib.rs:321-324
pub fn limit( &self ) -> Option< Seq >
{
  self.slowest().map( | s | s.advanced_by( self.capacity.get() as u64 ) )
}
```

Same first step, then `advanced_by` instead of `free_slots`, and `map` instead of
`map_or` — the `Option` is propagated rather than resolved, because an ungated
ring has no limit to report.

The two must describe the same boundary, and a test says so:

```rust
// tests/gating_test.rs:306-310
let set = set_at( 8, &[ 3 ] );
let limit = set.limit().unwrap();

assert_eq!( set.headroom( Seq( limit.0 - 1 ) ), 1, "one slot left just below the limit" );
assert_eq!( set.headroom( limit ), 0, "none at it" );
```

with the reason given in the test's own comment: *"A diagnostic that reported a
different blocking point than the gate actually enforces is worse than no
diagnostic."*

**`limit` is also the family's only multiplication by a capacity.** Where
`ring_seqno::laps_between` divides by capacity to count laps, `limit` multiplies by
one lap to place a position — and `laps_between` has no callers at all
([`ring_seqno` `workaround/002`](../../../ring_seqno/docs/workaround/002_laps_between_has_no_caller.md)),
while `limit` does. The family wants positions, not counts.

One consequence to note: `advanced_by` is `Self( self.0 + n )` — plain addition,
not saturating. `limit()` on a set whose slowest consumer is within `capacity` of
`u64::MAX` overflows, panicking in a debug build and wrapping in release. At 1M
sequences per second that is ~584,000 years away, and it is the same latent
condition as `Seq::next`'s
([`ring_seqno` finding S11](../../../ring_seqno/docs/definition/readme.md)) rather
than a new one.

### The Cost of the Depth

| Cost | Detail |
|------|--------|
| ~~One heap allocation per call~~ | `ring_cursor::slowest` collected a `Vec< Seq >` until `b7e075ca`; it now folds the cursors directly — see [`nfr/001`](../non_functional_requirement/001_every_gating_read_allocates_nothing.md) |
| Four crates to read to understand two lines | Mitigated by each boundary isolating one decision |
| Inlining is the optimiser's problem | All five steps are small and non-generic; none is annotated |

The first was real and measurable and landed in `ring_claim`'s retry loop; it is
gone, and `ring_claim/tests/allocation_test.rs` now measures that loop at zero.
The second is the price of the family's shape and is unchanged — the depth that
cost the allocation is still four crates deep. The third is unverified, and the
absence is family-wide rather than local:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -rc '#\[ *inline' ring_*/src/*.rs | awk -F: '{s+=$2} END {print s}'
# 2
```

Live output:

```
2
```

Both hits are in `ring_trace/src/lib.rs`: one real attribute, at line 255, on a
disabled-path check; the other is the doc comment two lines above it (line 252)
that names `` #[ inline ] `` in backticks to explain the attribute below —
the naive count-only recipe has no doc-line exclusion, unlike the pattern this
crate's own manual checks use (M2's `grep -vE "^[[:space:]]*(///|//!)"`), so it
counts prose mentioning the attribute the same as the attribute itself. The
chain-specific claim is the one that matters here and it is unchanged: none of
`ring_gating`, `ring_cursor`, `ring_seqno` or `ring_types` — the four crates the
five-step chain actually passes through — carries `#[ inline ]` anywhere, real or
mentioned. Whether the chain collapses across crate boundaries therefore rests
entirely on LTO and the optimiser's own judgement, and is recorded as open in
[`nfr/001`](../non_functional_requirement/001_every_gating_read_allocates_nothing.md).

### GT1 — Five Steps to One Subtraction

```
ring_gating::headroom          -> ring_cursor::slowest      (tier 4 -> 3)
ring_cursor::slowest           -> PaddedCursor::load        (tier 3 -> 1)
PaddedCursor::load             -> AtomicSeq::load           (tier 1 -> 1)
ring_gating::headroom          -> ring_seqno::free_slots      (tier 4 -> 0)
```

The method reads as two lines because everything under it is somewhere else. Its
own body chooses an arm; the arithmetic that arm needs is four tiers down.

**Finding.** It descends five steps through four crates and four tiers (4 → 3 → 1 → 0) to reach one subtraction, and that depth is what makes this crate's own body two lines

---

### GT2 — The Two Arms Are Compared Exactly Once

```
208:fn an_ungated_ring_and_a_consumer_at_zero_disagree_after_one_lap()
215:  assert_eq!( ungated.headroom( Seq::ZERO ), gated_at_zero.headroom( Seq::ZERO ) );
216:  assert_ne!( ungated.headroom( Seq( 4 ) ),  gated_at_zero.headroom( Seq( 4 ) ) );
```

One arm returns the capacity outright, the other calls into `ring_seqno`. Nothing
in the type system makes them agree at the point where they should, so the
question is whether anything checks. One test does, and it checks both
directions — equal while empty, different after a lap.

**Finding.** The two arms share no arithmetic — one returns `self.capacity.get()` outright, the other calls `ring_seqno::free_slots` — and exactly one test holds them against each other. `an_ungated_ring_and_a_consumer_at_zero_disagree_after_one_lap` asserts they agree at `Seq::ZERO` and differ at `Seq( 4 )`, so this crate's test pins `free_slots`' boundary convention from outside the crate that defines it

---


### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_reading_that_returns_a_position.md](../api/002_the_reading_that_returns_a_position.md) | `limit` beside `headroom`, as a surface decision |

### Algorithms

| File | Relationship |
|------|--------------|
| [002_check_orders_its_two_refusals.md](002_check_orders_its_two_refusals.md) | The same reading, wrapped in a reason |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_capacity_for_an_empty_set.md](../decisions/001_capacity_for_an_empty_set.md) | The `map_or` identity, argued |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_three_dependencies_and_two_dependents.md](../integration/001_three_dependencies_and_two_dependents.md) | The three crates this chain passes through |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_the_bound_is_the_minimum_and_only_the_minimum.md](../invariant/001_the_bound_is_the_minimum_and_only_the_minimum.md) | What the chain must compute |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_every_gating_read_allocates_nothing.md](../non_functional_requirement/001_every_gating_read_allocates_nothing.md) | The allocation the chain's third step used to introduce, and the four un-inlinable calls it still costs |

### Sources

| File | Relationship |
|------|--------------|
| `ring_gating/src/lib.rs:221-228` | `headroom` |
| `ring_gating/src/lib.rs:197-200` | `slowest`, the one-line forward |
| `ring_gating/src/lib.rs:321-324` | `limit`, the multiplied sibling |
| `ring_cursor/src/lib.rs:120-123` | The loads and the `Vec` |
| `ring_seqno/src/lib.rs:95-99` | `free_slots` |
| `ring_types/src/id.rs:82-85` | `distance_to` — the one subtraction |
| `ring_types/src/id.rs:65-68` | `advanced_by` — plain addition |
| `tests/manual/readme.md` § M3 | The check that the delegation stays one call each |

### Tests

| File | Relationship |
|------|--------------|
| `tests/gating_test.rs:161-174` | Headroom falls by one per slot across a full capacity |
| `tests/gating_test.rs:176-188` | It saturates rather than wrapping, across 60 positions |
| `tests/gating_test.rs:291-298` | `limit` is one lap past the slowest of three |
| `tests/gating_test.rs:306-310` | `limit` and `headroom` describe the same boundary |
