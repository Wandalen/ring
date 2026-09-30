# Algorithm: Two Loops That Disagree at Zero

### Scope

- **Purpose**: Account for the differences between `claim` and `claim_up_to` — one guard, one loop condition, one error variant — and for the one input on which they return opposite answers.
- **Responsibility**: Compare the two functions line by line, establish that `claim( 0 )` succeeds on a full ring while `claim_up_to( 0 )` fails, and argue whether the divergence is a defect.
- **In Scope**: Both function bodies, the zero case, and the `BatchTooLarge` guard only one of them has.
- **Out of Scope**: The retry structure they share — see [`algorithm/001`](001_the_gate_inside_the_retry.md).

### The Diff

```sh
cd "$(git rev-parse --show-toplevel)"
diff <( command grep -m1 -A27 -F '  pub fn claim( &self, count : usize ) -> Result< Claim, RingError >' ring_claim/src/lib.rs ) \
     <( command grep -m1 -A18 -F '  pub fn claim_up_to( &self, max : usize ) -> Result< Claim, RingError >' ring_claim/src/lib.rs )
# The command above can legitimately exit nonzero -- a count of zero, a diff
# that differs -- and the sweep that keeps this output current skips any block
# that exits nonzero, so without this the block freezes instead of refreshing.
:
```

Live output:

```
1c1
<   pub fn claim( &self, count : usize ) -> Result< Claim, RingError >
---
>   pub fn claim_up_to( &self, max : usize ) -> Result< Claim, RingError >
3,15c3,6
<     if count > self.consumers.capacity().get()
<     {
<       return Err( RingError::BatchTooLarge
<       {
<         requested : count,
<         capacity : self.consumers.capacity().get(),
<       } );
<     }
< 
<     // The gate is the loop condition, and is therefore re-read on every
<     // iteration: on a failed exchange another producer moved the cursor, so
<     // the headroom computed against the old value is stale and granting on it
<     // would overlap that producer's range.
---
>     // `granted @ 1..` binds the grant and gates on it in one expression, which
>     // is what keeps the headroom re-read in the loop condition rather than
>     // duplicated between a pre-loop computation and the retry arm. A grant of
>     // zero — no room, or a `max` of zero — exits to the `Full` below.
17c8
<     while count <= self.consumers.headroom( current )
---
>     while let granted @ 1.. = max.min( self.consumers.headroom( current ) )
19c10
<       let next = current.advanced_by( count as u64 );
---
>       let next = current.advanced_by( granted as u64 );
22c13
<         Ok( _ ) => return Ok( Claim::new( current, count ) ),
---
>         Ok( _ ) => return Ok( Claim::new( current, granted ) ),
```

| | `claim( count )` | `claim_up_to( max )` |
|--|------------------|----------------------|
| Line | `:388` | `:442` |
| Capacity guard | `if count > capacity` → `BatchTooLarge` | **none** |
| Loop condition | `while count <= headroom( current )` | `while let granted @ 1.. = max.min( headroom( current ) )` |
| Grant width | always `count` | `max.min( headroom )`, which varies per retry |
| Error variants | `BatchTooLarge`, `Full` | `Full` only |
| Behaviour at `0` | `Ok( empty claim )` | `Err( Full )` |

The missing guard is stated in `claim_up_to`'s own `# Errors` section
(`:448-450`):

> Never `BatchTooLarge` — a `max` wider than the ring is not an error here, it
> is simply more than will be granted.

That is consistent: `max` is a ceiling, so exceeding capacity is not a
misconfiguration, it is a ceiling that will never bind. `claim( 5 )` on a
4-slot ring can never succeed no matter how patient the caller is, which is why
it is a configuration error rather than back-pressure; `claim_up_to( 5 )` on the
same ring succeeds immediately with four.

### CL9 — The Two Functions Return Opposite Answers for Zero, on Purpose

```sh
cd "$(git rev-parse --show-toplevel)"
grep 'claim( 0 )\|claim_up_to( 0 )' ring_claim/tests/claim_test.rs
```

Live output:

```
  let empty = claimer.claim( 0 ).expect( "zero always fits, even on a full ring" );
  assert_eq!( Claimer::new( &consumers ).claim_up_to( 0 ), Err( RingError::Full ) );
```

On a **full** ring — `headroom == 0`:

| Call | Loop condition evaluates to | Result |
|------|------------------------------|--------|
| `claim( 0 )` | `0 <= 0` → **true**, enters the loop | `Ok( Claim { start : current, len : 0 } )` |
| `claim_up_to( 0 )` | `0.min( 0 ) == 0`, no match on `1..` | `Err( RingError::Full )` |

Both are defensible in isolation and they cannot both be right in the same API.
The argument for each:

- **`claim( 0 )` → `Ok`.** The caller asked for zero sequences and got zero
  sequences. Nothing about the ring's state can prevent that. Returning `Full`
  would report back-pressure for a request that cannot experience it.
- **`claim_up_to( 0 )` → `Err`.** `claim_up_to` promises "as many as are
  available, down to **one**". A zero-width grant is not a grant, and a caller
  that batches would loop forever on `Ok( empty )` rather than backing off.

Both are tested, so neither is accidental. The uncomfortable part is that the
two contracts are stated in different places — `claim`'s zero behaviour is not
in its `# Errors` section at all, only in the test — so a reader comparing the
two signatures has no way to predict the divergence from the documentation.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -A2 -F 'A `count` of zero always succeeds' ring_claim/src/lib.rs
```

Live output:

```
    /// A `count` of zero always succeeds, even on a full ring — there is
    /// nothing for back-pressure to block. [`claim_up_to`] treats a zero grant
    /// as `Full` instead; the two functions disagree here on purpose.
```

**Disposition:** applied — `claim`'s doc comment now states the zero-case
behaviour explicitly in its `# Errors` section, cross-referencing
`claim_up_to`'s opposite treatment, so the divergence is predictable from
the documentation alone. Now prints: `the two functions disagree here on purpose`

### The Cost of `claim( 0 )` on a Full Ring

The zero case is not free, and this is the one place the crate performs a
visible-but-pointless atomic operation:

```rust
let next = current.advanced_by( 0 );        // == current
self.cursor.compare_exchange( current, current, CLAIM_SUCCESS, GATING )
```

A compare-exchange of a value against itself. On x86 that is a `lock cmpxchg`
with a full barrier, on a cache line every producer is contending for — the
same cost as a real claim, for a grant of nothing. It can also *fail*, if a peer
moves the cursor in the window, in which case the loop adopts the peer's value
and does it again.

Nothing calls `claim( 0 )` outside the tests, so the cost is theoretical. It is
recorded because the shape is not obvious from the signature: a request for zero
work performs the most expensive instruction in the crate.

### CL10 — `claim_up_to`'s Grant Width Is Recomputed Per Retry, and `claim`'s Is Not

This is the deeper of the two differences, and it changes what a retry *means*:

| | On retry, `claim` | On retry, `claim_up_to` |
|--|-------------------|--------------------------|
| Re-reads headroom | yes | yes |
| Re-derives the width | no — it is `count`, fixed by the caller | **yes** — `max.min( headroom( current ) )` |
| A retry can grant less than the previous attempt would have | no | **yes** |
| A retry can turn a would-be success into `Full` | yes | yes |

So `claim_up_to` under contention is *adaptive*: a producer that loses the
exchange re-reads a ring with less room and asks for less, rather than asking
for the same amount and failing. That is the behaviour a batching producer
wants, and it is also why the function needs the binding pattern — the grant
must be computed inside the condition, because it is derived from the value the
condition tests.

The consequence for testing is that `claim_up_to`'s retry arm is much harder to
reach than `claim`'s. `claim`'s arm is taken whenever two producers race;
`claim_up_to`'s is taken only when they race *and* the loser still finds a
non-zero grant afterwards. `claim_up_to_under_contention_loses_no_sequences_either`
exists because the coverage gate showed the arm was never taken by the tests
that existed at the time.

### Both Loops Advance by a `u64`

```sh
cd "$(git rev-parse --show-toplevel)"
grep -vE '^[[:space:]]*//' ring_claim/src/lib.rs | grep ' as u64'
```

Live output:

```
        self.start.advanced_by(self.len as u64)
            let next = current.advanced_by(count as u64);
            let next = current.advanced_by(granted as u64);
```

Three casts, all `usize as u64`, all widening on every target this family
supports, and all feeding `Seq::advanced_by`:

| Site | Expression |
|------|------------|
| `Claim::end` | `self.start.advanced_by( self.len as u64 )` |
| `claim` | `current.advanced_by( count as u64 )` |
| `claim_up_to` | `current.advanced_by( granted as u64 )` |

`usize` is the width of a *count of slots*; `Seq` is a `u64` position that never
wraps in practice. The direction of the cast is what makes it safe, and it is
the same single-direction cast `ring_publish` makes once
([`type/001`](../type/001_a_seq_a_usize_and_three_casts.md)).

### Algorithms

| File | Relationship |
|------|--------------|
| [001_the_gate_inside_the_retry.md](001_the_gate_inside_the_retry.md) | The structure both functions share |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_two_constructors_of_a_range.md](../api/002_the_two_constructors_of_a_range.md) | The two signatures, and the variant only one produces |

### Non Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_what_contention_costs.md](../non_functional_requirement/002_what_contention_costs.md) | The atomic a zero-width claim still pays for, and the per-attempt cost both loops share |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_claiming_zero.md](../pitfall/002_claiming_zero.md) | What a caller who does not know about the divergence writes |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_a_seq_a_usize_and_three_casts.md](../type/001_a_seq_a_usize_and_three_casts.md) | The three casts both loops depend on |

### Sources

| File | Relationship |
|------|--------------|
| `ring_claim/src/lib.rs:387-420` | `claim`'s doc, with the `# Errors` contract that omits zero |
| `ring_claim/src/lib.rs:421-447` | `claim`, its capacity guard and its loop |
| `ring_claim/src/lib.rs:450-480` | `claim_up_to`'s doc, and the stated absence of `BatchTooLarge` |
| `ring_claim/src/lib.rs:481-488` | `claim_up_to`, its binding pattern and its comment |
| `ring_types/src/id.rs` | `Seq::advanced_by`, the target of all three casts |

### Tests

| File | Relationship |
|------|--------------|
| `tests/claim_test.rs:224` — `claiming_zero_succeeds_and_moves_nothing` | `claim( 0 )` on a full ring, and the cursor unmoved after it |
| `tests/claim_test.rs:262` — `claim_up_to_of_zero_is_full_not_an_empty_claim` | The other half of the divergence, with the contract quoted in the test |
| `tests/claim_test.rs:249` — `claim_up_to_never_reports_batch_too_large` | The guard `claim_up_to` deliberately lacks |
| `tests/claim_test.rs:390` — `claim_up_to_under_contention_loses_no_sequences_either` | The retry arm the coverage gate found unreachable |
| `tests/manual/readme.md § C6` | The contention constants and the five `thread::scope` blocks |
