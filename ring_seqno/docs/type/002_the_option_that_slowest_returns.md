# Type: The `Option` That `slowest` Returns

### Scope

- **Purpose**: Give what `Option< Seq >` promises a caller, what it costs, and what a caller is obliged to do with it.
- **Responsibility**: State the promise, enumerate the four call sites and what each does with `None`, and record the representation cost.
- **In Scope**: `slowest`'s return type as a contract.
- **Out of Scope**: Why it was chosen over an identity element — see [`decisions/001`](../decisions/001_none_rather_than_zero_for_an_empty_set.md).

### The Promise

```rust
pub fn slowest( cursors : &[ Seq ] ) -> Option< Seq >
```

| Variant | Means | Caller must |
|---------|-------|-------------|
| `Some( s )` | `s` is the minimum of a non-empty set, and every member is at or ahead of it | Treat `s` as the binding constraint |
| `None` | The set was empty — there is **no** constraint | Decide what an unconstrained ring means *for its own question* |

The second row is the whole contract. `None` is not a failure and not a default —
it is the absence of a bound, and the type refuses to guess what that implies.

Three further promises follow from the body and are worth stating because a
caller may rely on them:

| # | Promise | Why it holds |
|---|---------|--------------|
| P1 | `Some( s )` always holds a value **present in the slice** | `min()` returns an element, never a computed value |
| P2 | Order-independent | `min()` over `Ord` — `slowest_is_the_minimum_wherever_it_sits` pins it across four arrangements |
| P3 | Duplicates are fine | `slowest( &[ Seq( 7 ), Seq( 7 ) ] )` is `Some( Seq( 7 ) )`, asserted at `seq_test.rs:120` |

P1 matters more than it looks: it means a caller may compare the result by
identity against a specific cursor's position to learn *which* consumer is
slowest, not merely how far back it is.

### What Every Caller Does With `None`

Four sites in the family, and they do not agree — which is the point:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r 'slowest()\|frontier()' ring_*/src/*.rs | grep -vE ':\s*///'
```

Live output:

```
ring_barrier/src/lib.rs:    self.frontier().map_or( 0, | frontier | from.distance_to( frontier ) )
ring_barrier/src/lib.rs:    self.frontier().ok_or( RingError::Empty )
ring_consume/src/lib.rs:      .frontier()
ring_gating/src/lib.rs:    self.slowest().map_or( self.capacity.get(), | slowest |
ring_gating/src/lib.rs:    self.slowest().map( | s | s.advanced_by( self.capacity.get() as u64 ) )
```

| Site | Question | `None` → | Reasoning |
|------|----------|----------|-----------|
| `ring_gating:224` `headroom` | how much room has the producer? | `capacity.get()` | A ring nobody reads can lose nothing |
| `ring_gating:323` `limit` | what sequence bounds the producer? | `None`, propagated | There is no limit; the caller decides again |
| `ring_barrier:219` `available` | how much may the consumer read? | `0` | Nothing was published to depend on |
| `ring_consume:342` (via `frontier`) | same | `0` | Same |

**Three different answers to one `None`**: full capacity, propagate, and zero. A
return type supplying any single default would have been wrong for at least two
of them.

`ring_gating:323` is the interesting one — it is the only site that *keeps* the
`Option` rather than resolving it:

```rust
pub fn limit( &self ) -> Option< Seq >
{
  self.slowest().map( | s | s.advanced_by( self.capacity.get() as u64 ) )
}
```

`map` rather than `map_or`. The absence of a bound propagates upward as an
absence, because "the sequence one lap past the slowest consumer" genuinely has
no value when there is no slowest consumer. That is the type used as intended —
carried until a caller exists that can answer it.

### The Cost

| Cost | Measure |
|------|---------|
| Width | `Option< Seq >` is **16 bytes**; `Seq` is 8 |
| A branch at every call site | Four `map_or`/`map` calls in the family |
| Stored anywhere? | **No** |

The width doubling is real and unavoidable. `Seq( pub u64 )` uses its full `u64`
range, so there is no niche for the discriminant to hide in — unlike
`Option< &T >` or `Option< NonZeroU64 >`, which are free.

It costs nothing in practice because no type in the family stores one. Every
`Option< Seq >` in existence is returned in registers and consumed by a `map_or`
in the same expression:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r 'Option< Seq >' ring_*/src/*.rs | grep -vE ':\s*///'
```

Live output:

```
ring_barrier/src/lib.rs:  pub fn frontier( &self ) -> Option< Seq >
ring_cursor/src/lib.rs:pub fn slowest( cursors : &[ PaddedCursor ] ) -> Option< Seq >
ring_gating/src/lib.rs:  pub fn slowest( &self ) -> Option< Seq >
ring_gating/src/lib.rs:  pub fn limit( &self ) -> Option< Seq >
ring_mpsc/src/lib.rs:  pub fn published_through( &self ) -> Option< Seq >
ring_seqno/src/lib.rs:pub fn slowest( cursors : &[ Seq ] ) -> Option< Seq >
```

Every one of the six is a return type. None is a struct field.

Had it been a field — a cached `slowest` on `GatingSet`, say — the eight wasted
bytes would sit on a hot cache line beside the cursors, and the trade would need
re-examining.

### What the Type Does Not Promise

| # | Not promised | Consequence |
|---|--------------|-------------|
| N1 | Freshness | The slice was read before the call; by the time `Some( s )` returns, the real cursors may have moved. `ring_cursor` loads them at `GATING` and calls straight through — the answer is a snapshot |
| N2 | That `s` is behind any particular producer | `slowest` knows nothing about a producer. A consumer ahead of one is representable and `slowest` will happily return it — see [`decisions/002`](../decisions/002_saturating_rather_than_signed.md) |
| N3 | Which consumer it belongs to | P1 gives a value, not an index. `GatingSet` has no `slowest_index` |
| N4 | A stable answer across calls | Two calls on a live ring can differ, and nothing marks the result as a point-in-time reading |

N1 and N4 are the same fact seen twice, and neither is visible in the signature.
`slowest( &[ Seq ] )` looks pure — and it *is* pure — but its input was produced
by an atomic read, so purity here does not mean the answer stays true. This is
why `ring_debug`'s `Watch` exists as a separate mechanism: comparing two readings
over time needs something that holds both, and no single reading can.

### SQ49 — Sixteen Bytes That Never Land Anywhere

The type is doubled in size by the `None` case, and only ever exists in flight:

```
Seq            = u64          8 bytes, every bit pattern valid
Option< Seq >  =              16 bytes, no niche to pack into

and no struct in the family holds one as a field.
```

**Finding.** `Option< Seq >` is 16 bytes — `Seq` has no niche — and is never stored anywhere in the family.

---

### SQ50 — One `expect` Behind Ten Tests

Every test in the file routes through one helper, and the helper has one failure mode:

```
seq_test.rs:20   Capacity::new( n ).expect( "test capacities are powers of two" )

the only expect / unwrap / panic! in the file.
```

**Finding.** The test suite's one `expect` is in the `cap` helper, so every assertion in ten tests rests on a capacity constructor from another crate and the suite has no way to fail other than by asserting.

---

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_the_slowest_fold.md](../algorithm/002_the_slowest_fold.md) | The fold, and its four-tier chain |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_none_rather_than_zero_for_an_empty_set.md](../decisions/001_none_rather_than_zero_for_an_empty_set.md) | Why `Option` rather than an identity |
| [../decisions/002_saturating_rather_than_signed.md](../decisions/002_saturating_rather_than_signed.md) | N2 — the state `slowest` will return without complaint |

### Items

| File | Relationship |
|------|--------------|
| [../item/002_the_two_readings_without_a_capacity.md](../item/002_the_two_readings_without_a_capacity.md) | `slowest`'s signature, constness and coverage |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_the_shared_fold_that_declines_an_identity.md](../pattern/002_the_shared_fold_that_declines_an_identity.md) | The shape, stated generally |

### Types

| File | Relationship |
|------|--------------|
| [001_what_a_span_is_measured_in.md](001_what_a_span_is_measured_in.md) | The other four return types |

### Sources

| File | Relationship |
|------|--------------|
| `ring_seqno/src/lib.rs:117-136` | The signature and its stated contract |
| `ring_gating/src/lib.rs:197-200, 222-228, 321-324` | Three of the four call sites, including the one that propagates |
| `ring_barrier/src/lib.rs:191-194, 217-220` | The fourth |
| `ring_types/src/id.rs:25` | `Seq( pub u64 )` — full range, no niche |

### Tests

| File | Relationship |
|------|--------------|
| `tests/seq_test.rs:113-121` | P1, P2 and P3 |
| `tests/seq_test.rs:125-130` | The `None`/`Some( ZERO )` distinction the contract rests on |
| `ring_gating/tests/gating_test.rs:199-209` | The contract exercised where it matters — the conflation shown failing |
