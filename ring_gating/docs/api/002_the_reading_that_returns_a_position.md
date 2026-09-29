# API: The Reading That Returns a Position

### Scope

- **Purpose**: Account for the five gating readings' return types — why one `Option< Seq >` is propagated, another resolved, and two more collapsed into a `bool` and a `Result`.
- **Responsibility**: Give the five shapes, the three dispositions of the single `Option` they all descend from, the one place the correspondence between them has a hole, and what a caller may conclude from any of them once the answer is a moment old.
- **In Scope**: Return types of `slowest`, `headroom`, `admits`, `check`, `limit`.
- **Out of Scope**: The `usize`/`u64` width split — see [`type/001`](../type/001_a_usize_headroom_and_a_u64_limit.md).

### Five Readings of One State

| Method | Returns | On an empty set | Question answered |
|--------|---------|-----------------|-------------------|
| `slowest` | `Option< Seq >` | `None` | *Where is the rearmost consumer?* |
| `limit` | `Option< Seq >` | `None` | *At what position will I block?* |
| `headroom` | `usize` | `capacity` | *How many slots may I take?* |
| `admits` | `bool` | `count <= capacity` | *May I take this many?* |
| `check` | `Result< (), RingError >` | `Ok` if `count <= capacity` | *…and if not, why not?* |

All five descend to a single `Option< Seq >` — the one
[`ring_seqno::slowest`](../../../ring_seqno/docs/pattern/002_the_shared_fold_that_declines_an_identity.md)
returns because a fold over an empty slice has no minimum. The crate disposes of
it three ways:

| Disposition | Method | Written |
|-------------|--------|---------|
| Return it unchanged | `slowest` | a direct forward |
| Propagate through a transform | `limit` | `.map( \| s \| s.advanced_by( … ) )` |
| Resolve it to a value | `headroom` | `.map_or( self.capacity.get(), … )` |

`admits` and `check` add nothing to the disposition — both consume `headroom`'s
already-resolved `usize`, which is why neither can be `Option`-shaped.

### Why `limit` Propagates Where `headroom` Resolves

This is the crate's one genuinely arbitrary-looking asymmetry, and it is not
arbitrary. The two answers are about different things:

> *How much room is there?* has an answer for every set. A ring nobody reads has
> room for everything, so `capacity` is not a fallback — it is the correct
> number.
>
> *At what position do I block?* has no answer for a set nobody reads. There is
> no such position.

The resolution `headroom` uses is therefore a **fact**, and the one `limit` would
need would be a **fiction**. The two candidate fictions are both worse than
`None`:

| Candidate | Failure |
|-----------|---------|
| `Seq::ZERO` | Reads as "you are already blocked" — the exact inversion |
| `Seq( u64::MAX )` | Reads as a real position 18 quintillion slots away; arithmetic on it overflows |

A caller holding `Some( limit )` may compare a producer position against it. A
caller holding `None` may not, and the type says so. See
[`decisions/001`](../decisions/001_capacity_for_an_empty_set.md) for the
`headroom` half of the same choice, which is the one that prevents a deadlock.

### The Correspondence, and Its One Hole

`limit` is a diagnostic for `headroom`, and a test pins the two together:

```rust
// tests/gating_test.rs:306-310
let set = set_at( 8, &[ 3 ] );
let limit = set.limit().unwrap();

assert_eq!( set.headroom( Seq( limit.0 - 1 ) ), 1, "one slot left just below the limit" );
assert_eq!( set.headroom( limit ), 0, "none at it" );
```

`.unwrap()` there is the hole. On an empty set the pairing has nothing to assert:

| Set | `headroom( p )` | `limit()` | Comparable? |
|-----|-----------------|-----------|:-----------:|
| Non-empty | a number ≤ capacity | `Some( q )` | ✅ — the test above |
| Empty | `capacity`, for every `p` | `None` | ❌ |

That is not a defect — an ungated ring genuinely has no blocking position — but
it means the property *"`limit` and `headroom` describe the same boundary"* is
established only where both are total. Nothing tests the empty case for `limit`,
and there is nothing there to test beyond `is_none()`.

### Every One of the Five Is Already Stale

All five read cursors other threads are writing. The value returned describes a
moment that has passed by the time the caller sees it, so what matters is which
*direction* it can be wrong in:

> Consumers only advance. `slowest` is monotonically non-decreasing, so
> `headroom` computed from it can only have grown since it was read.

That one-sidedness is what makes the whole surface usable:

| Reading | Safe to act on | Because |
|---------|----------------|---------|
| `headroom` → *n* | ✅ at most *n* | The true value now is ≥ *n* |
| `admits` → `true` | ✅ | Would still be true |
| `admits` → `false` | ⚠️ retry | May have become true |
| `check` → `Err( Full )` | ⚠️ retry | Same |
| `check` → `Err( BatchTooLarge )` | ❌ never retry | Capacity does not change |
| `limit` → `Some( q )` | ✅ as a floor | The true limit now is ≥ *q* |

The last row is the reason `limit` is useful despite being stale: a producer that
has not reached the reported limit is definitely not blocked, whatever has
happened since.

**A stale answer is safe in one direction and fatal in the other**, and only the
under-reporting direction is tested — `a_gate_read_concurrently_with_a_consumer_never_over_reports_room`
(`tests/gating_test.rs:394-428`) runs 10,000 reads against a moving consumer and
asserts the bound is never exceeded, never that it is tight. See
[`nfr/002`](../non_functional_requirement/002_the_gate_must_never_over_report.md)
for what that test does and does not establish.

### GT9 — Two Questions, One `None`

```
321:  pub fn limit( &self ) -> Option< Seq >
323:    self.slowest().map( |s| s.advanced_by( self.capacity.get() as u64 ) )
       slowest() == None  <=>  the set is empty  <=>  limit() == None
```

`limit` returns the furthest sequence a producer may reach. When there is no
such bound it returns `None`, and when there are no consumers it also returns
`None` — because `slowest` returned `None` and `map` passed it through.

**Finding.** Returns `None` for an ungated ring and, by construction, for any set with no consumers, so "no limit" and "no consumers" are the same value. A caller diagnosing a stall cannot separate an unbounded producer from a misconfigured set without also calling `is_empty`, and the doc comment describes only the first reading

---

### GT10 — The Crate's Only Cast Lives Here

```
323:      self.capacity.get() as u64
      Capacity::get   -> usize
      Seq::advanced_by -> takes u64
```

One `as` in 325 lines, in the one method that has to turn a size into a
position. It is the seam between the two width conventions the family carries.

**Finding.** `limit` holds the crate's only `as` conversion, `self.capacity.get() as u64`, forced by `Capacity::get` returning `usize` while `Seq::advanced_by` takes `u64`. The method that returns a position is also the single point where this crate crosses a width boundary

---


### APIs

| File | Relationship |
|------|--------------|
| [001_eleven_methods_over_one_owned_vec.md](001_eleven_methods_over_one_owned_vec.md) | The whole surface these five sit in |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_headroom_in_two_delegations.md](../algorithm/001_headroom_in_two_delegations.md) | `headroom` and `limit` traced to the same first step |
| [../algorithm/002_check_orders_its_two_refusals.md](../algorithm/002_check_orders_its_two_refusals.md) | Why the retry column above splits the way it does |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_capacity_for_an_empty_set.md](../decisions/001_capacity_for_an_empty_set.md) | The resolution, and the deadlock the alternative causes |
| [../decisions/002_a_result_rather_than_a_bool.md](../decisions/002_a_result_rather_than_a_bool.md) | Why one refusal is retryable and the other is not |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_the_gate_must_never_over_report.md](../non_functional_requirement/002_the_gate_must_never_over_report.md) | The one-sided guarantee, and its test |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_a_usize_headroom_and_a_u64_limit.md](../type/001_a_usize_headroom_and_a_u64_limit.md) | Why a count and a position are different widths |

### Sources

| File | Relationship |
|------|--------------|
| `ring_gating/src/lib.rs:197-200` | `slowest` — the `Option` unchanged |
| `ring_gating/src/lib.rs:222-228` | `headroom` — the `Option` resolved |
| `ring_gating/src/lib.rs:321-324` | `limit` — the `Option` propagated |
| `ring_gating/src/lib.rs:22-29` | The module's own statement of the resolution |
| `ring_cursor/src/lib.rs:120-123` | Where the `Option` originates |

### Tests

| File | Relationship |
|------|--------------|
| `tests/gating_test.rs:291-298` | `limit` is one lap past the slowest of three |
| `tests/gating_test.rs:300-311` | The correspondence, where both are total |
| `tests/gating_test.rs:190-205` | The empty set answering `capacity` |
| `tests/gating_test.rs:394-428` | Staleness, in the one direction that is checked |
