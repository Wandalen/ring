# Decision: `None` Rather Than Zero for an Empty Set

### Scope

- **Purpose**: Record why `slowest` returns `Option< Seq >` rather than folding an empty slice to `Seq::ZERO`, and show the decision was later validated by its consumers.
- **Responsibility**: State the alternatives, give the argument made at the time, and give the evidence that arrived afterwards.
- **In Scope**: `slowest`'s return type.
- **Out of Scope**: The fold's cost and its wrapping chain — see [`algorithm/002`](../algorithm/002_the_slowest_fold.md).

### The Decision

```rust
// ring_seqno/src/lib.rs:133
pub fn slowest( cursors : &[ Seq ] ) -> Option< Seq >
```

`min()` over an empty slice has no answer. The crate returns `None` rather than
supplying one.

### The Alternatives

| Option | Meaning for an empty set | Why not |
|--------|--------------------------|---------|
| `Seq::ZERO` | "the slowest consumer is at the start" | Conflates *no consumers* with *a consumer that has read nothing* — see below |
| `Seq( u64::MAX )` | "infinitely far ahead", the max-identity for a min-fold | Arithmetically tidy; `distance_to` against it saturates, so downstream readings silently produce garbage |
| Panic | "an empty set is a caller error" | It is not — an ungated ring is a legitimate configuration |
| `Option< Seq >` ✅ | "there is no slowest consumer" | Pushes one decision onto each caller, and the callers genuinely disagree about it |

### The Argument Made at the Time

The doc gives it in two sentences:

> Returning `None` rather than `Seq::ZERO` for an empty set keeps "no consumers"
> distinguishable from "a consumer at the start" — the two call for opposite
> decisions, since an ungated ring may publish freely.

`ring_cursor`'s wrapper restates it independently, which is unusual and worth
noting — the same reasoning written twice, in two crates, by an author who
evidently thought it load-bearing:

> `None` rather than [`Seq::ZERO`] for an empty slice, because the two callers
> resolve *no dependencies* to opposite values — full headroom on one side,
> nothing readable on the other — and a fold that picked either would be wrong
> for one of them.

### The Evidence That Arrived Afterwards

The argument predicted that two consumers would want opposite defaults. Both
exist, and they do:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A5 -F '  pub fn headroom( &self, producer : Seq ) -> usize' ring_gating/src/lib.rs
command grep -m1 -A4 -F '  pub fn available( &self, from : Seq ) -> u64' ring_barrier/src/lib.rs | tail -n 4
```

Live output:

```
  pub fn headroom( &self, producer : Seq ) -> usize
  {
    self.slowest().map_or( self.capacity.get(), | slowest |
    {
      ring_seqno::free_slots( producer, slowest, self.capacity )
    } )
  {
    self.frontier().map_or( 0, | frontier | from.distance_to( frontier ) )
  }
```

```rust
// ring_gating:217-222 — a producer asking how much room it has
pub fn headroom( &self, producer : Seq ) -> usize
{
  self.slowest().map_or( self.capacity.get(), | slowest |     // None → FULL capacity
  {
    ring_seqno::free_slots( producer, slowest, self.capacity )
  } )
}

// ring_barrier:217-220 — a consumer asking how much it may read
pub fn available( &self, from : Seq ) -> u64
{
  self.frontier().map_or( 0, | frontier | from.distance_to( frontier ) )   // None → ZERO
}
```

**Same `None`. Opposite resolutions.** `capacity.get()` on one side, `0` on the
other — the two extremes of the range.

Both are correct, and the reason each is correct is domain-specific rather than
arithmetic:

| Crate | Empty set means | Therefore |
|-------|-----------------|-----------|
| `ring_gating` | nobody is reading this ring | Nothing can be lost, so publish freely — full headroom |
| `ring_barrier` | nothing has been published to depend on | Nothing is readable — zero |

`ring_barrier`'s own doc flags the divergence explicitly, pointing at the other
crate:

> Zero when the barrier has no dependencies — see the module documentation for
> why that is not the same answer `ring_gating` gives an empty set.

### What an Identity Would Have Cost

Had `slowest` returned `Seq::ZERO` for an empty slice, both call sites would have
compiled, both would have dropped their `map_or`, and one would have been wrong:

| Crate | With `Seq::ZERO` | Correct? |
|-------|------------------|:--------:|
| `ring_barrier::available` | `from.distance_to( ZERO )` → `0` for any `from` | ✅ by luck |
| `ring_gating::headroom` | `free_slots( producer, ZERO, capacity )` → `0` once the producer passes one lap | ❌ |

The failure mode is specific and bad. A ring with **no consumers registered** —
which is exactly the case where publishing is unconstrained — would report zero
headroom forever after its first lap. `ring_claim`'s retry loop
(`ring_claim:411-445`) spins on `count <= headroom( current )`, so it would spin
without progress, permanently, on a ring nobody was reading.

That is a deadlock produced by a fold's identity element. It is the strongest
argument in this crate's documentation for a return type.

**And it is defended at three levels, which is unusual in this family.** Most
decisions here are argued in prose and then guarded by nothing — the `Relaxed`
fork that `ring_cursor:99` warns about has zero assertions against it. This one
has a test at every tier that touches it:

| Tier | Assertion | What it pins |
|------|-----------|--------------|
| `ring_seqno` | `tests/seq_test.rs:125-131` | `slowest( &[] )` is `None`, and `slowest( &[ ZERO ] )` is `Some( ZERO )` — the two states are distinguishable *in the return value* |
| `ring_cursor` | `src/lib.rs:109` (doctest) | The same, one tier up, for `&[ PaddedCursor ]` |
| `ring_gating` | `tests/gating_test.rs:182-197` | An ungated ring reports full headroom at producers `0`, `4`, `1_000` and `u32::MAX` — after many laps |
| `ring_gating` | `tests/gating_test.rs:199-209` | **The conflation itself, shown failing** |

The last is the one to read. It constructs both states side by side and asserts
they diverge:

```rust
fn an_ungated_ring_and_a_consumer_at_zero_disagree_after_one_lap()
{
  let ungated       = GatingSet::new( cap( 4 ), 0 );
  let gated_at_zero = set_at( 4, &[ 0 ] );

  assert_eq!( ungated.headroom( Seq::ZERO ), gated_at_zero.headroom( Seq::ZERO ), "same while empty" );
  assert_ne!( ungated.headroom( Seq( 4 ) ), gated_at_zero.headroom( Seq( 4 ) ), "and different after a lap" );
}
```

Its own comment names the decision: *"The two states that a `Seq::ZERO`-for-empty
implementation would conflate, shown disagreeing."* The first assertion is the
craft — it establishes the two states are **indistinguishable before one lap**,
so the second assertion is proving a real divergence rather than an accident of
setup. An implementation folding empty to `Seq::ZERO` passes line one and fails
line two.

That is what a defended decision looks like, and it is worth contrasting with the
undefended ones catalogued in
[`ring_cursor` `definition/readme.md`](../../../ring_cursor/docs/definition/readme.md)
§ Findings Recorded, Not Fixed. The difference is not that this decision is more
important — it is that someone wrote the adversarial test.

### The Cost of the Decision

| Cost | Assessment |
|------|------------|
| Every caller writes `map_or` | Real, and small — four sites, one line each |
| The default is chosen far from the fold | Real, and this is the point. The fold cannot know; the caller can |
| `Option< Seq >` is 16 bytes where `Seq` is 8 | `Seq` uses its whole `u64` range, so there is no niche to pack into. Never stored, only returned |

The third row is worth stating because it is the one place a reader might expect
zero cost and not get it. `Option< Seq >` genuinely doubles the width — but the
value is returned in registers and immediately consumed by a `map_or` at every
call site in the family, so nothing is ever laid out in memory holding one.

### SQ13 — The Same `None`, Resolved Both Ways

Two crates consume the same `Option` and disagree about what its absence means:

```
ring_gating   self.slowest().map_or( self.capacity.get(), … )   None -> capacity
ring_barrier  … available … resolves an empty set to             None -> 0
```

**Finding.** `GatingSet::headroom` and `Barrier::available` resolve the same `None` to opposite values, which is the empirical proof that an identity would have been wrong.

---

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_the_slowest_fold.md](../algorithm/002_the_slowest_fold.md) | The fold, its chain, and its cost |

### Decisions

| File | Relationship |
|------|--------------|
| [002_saturating_rather_than_signed.md](002_saturating_rather_than_signed.md) | The other refusal to invent a value |

### Items

| File | Relationship |
|------|--------------|
| [../item/002_the_two_readings_without_a_capacity.md](../item/002_the_two_readings_without_a_capacity.md) | `slowest`'s signature and coverage |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_the_shared_fold_that_declines_an_identity.md](../pattern/002_the_shared_fold_that_declines_an_identity.md) | The shape, stated generally |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_the_option_that_slowest_returns.md](../type/002_the_option_that_slowest_returns.md) | What the `Option` promises a caller |

### Sources

| File | Relationship |
|------|--------------|
| `ring_seqno/src/lib.rs:117-136` | The decision and its stated argument |
| `ring_cursor/src/lib.rs:101-104` | The same argument, restated a tier up |
| `ring_gating/src/lib.rs:222-224` | `None` → full capacity |
| `ring_barrier/src/lib.rs:196-219` | `None` → zero, with a pointer at the divergence |
| `ring_claim/src/lib.rs:411-445` | The loop that would have spun forever |

### Tests

| File | Relationship |
|------|--------------|
| `tests/seq_test.rs:125-131` | The two states, asserted distinguishable |
| `ring_seqno/src/lib.rs:129-130` | The doctest — `slowest( &[] )` is `None` |
| `ring_cursor/src/lib.rs:111` | Tier 2's doctest asserts the same for `&[ PaddedCursor ]` |
