# algorithm

One loop, written twice. Both claiming functions are a compare-exchange retry
whose gate sits in the loop condition rather than before it — a placement that
looks like a stylistic choice and is the entire correctness argument, because a
gate evaluated before the exchange describes a ring that the exchange's own
failure has already invalidated.

The two files split on structure and difference. The first is the shared loop:
what each retry re-reads, why the gate cannot move, and why both exits report
the same error for two situations that are not the same. The second is the
diff — one guard, one loop condition, one error variant, and one input on which
the two functions return opposite answers.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [The Gate Inside the Retry](001_the_gate_inside_the_retry.md) | CL7, CL8 — the four steps of an iteration, the three placements of the gate and why only one is safe, both exits, and the `AcqRel`/`GATING` pair |
| 002 | [Two Loops That Disagree at Zero](002_two_loops_that_disagree_at_zero.md) | CL9, CL10 — the diff between `claim` and `claim_up_to`, the zero divergence, the atomic a zero-width claim still pays for, and the per-retry grant width |

### The Loop, Once

```rust
let mut current = self.claimed();
while <the gate admits at current>
{
  let next = current.advanced_by( <width> as u64 );
  match self.cursor.compare_exchange( current, next, CLAIM_SUCCESS, GATING )
  {
    Ok( _ )      => return Ok( Claim::new( current, <width> ) ),
    Err( actual ) => current = actual,
  }
}
Err( RingError::Full )
```

`<the gate admits at current>` and `<width>` are the two holes, and the two
functions fill them differently:

| | `claim` | `claim_up_to` |
|--|---------|---------------|
| gate | `count <= headroom( current )` | `let granted @ 1.. = max.min( headroom( current ) )` |
| width | `count` — fixed | `granted` — recomputed each retry |

Everything else is identical, including the ordering pair, the adoption of
`actual`, and the terminal `Err( RingError::Full )`.

### Why the Placement Is the Algorithm

| Where the gate goes | What a retry then grants against |
|---------------------|----------------------------------|
| before the loop | headroom measured before any peer moved the cursor — stale |
| after a failed exchange | nothing on the first iteration — ungated |
| **as the loop condition** | headroom at the value the failed exchange returned — current |

The first row is the shape of `GatingSet::check`, which exists, is correct for
non-racing callers, and has no library callers anywhere in the family
([`integration/002`](../integration/002_four_predicates_and_the_one_that_is_called.md)).
A reader "simplifying" this loop by calling it would be writing the race.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order

# the two loops, side by side
diff <( command grep -m1 -A27 -F '  pub fn claim( &self, count : usize ) -> Result< Claim, RingError >' ring_claim/src/lib.rs ) \
     <( command grep -m1 -A18 -F '  pub fn claim_up_to( &self, max : usize ) -> Result< Claim, RingError >' ring_claim/src/lib.rs )

# the gate must appear in the `while` line itself
grep 'while.*headroom' ring_claim/src/lib.rs

# every compare_exchange in the family that is not a trait declaration or impl
command grep -r '\.compare_exchange(' ring_*/src/*.rs | command grep -vE '^\S+: *//'

# the ordering constants, and their values
grep -rhA1 'Ordering = core::sync::atomic::Ordering' ring_*/src/*.rs \
  | grep -oE 'Ordering::[A-Za-z]+' | sort | uniq -c

# every cast in the crate
grep -vE '^[[:space:]]*//' ring_claim/src/lib.rs | grep ' as '
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
    while count <= self.consumers.headroom( current )
    while let granted @ 1.. = max.min( self.consumers.headroom( current ) )
ring_atomic/src/lib.rs:      .compare_exchange( current.0, new.0, success, failure )
ring_atomic/src/lib.rs:    self.cell.compare_exchange( current, new, success, failure )
ring_claim/src/lib.rs:      match self.cursor.compare_exchange( current, next, CLAIM_SUCCESS, GATING )
ring_claim/src/lib.rs:      match self.cursor.compare_exchange( current, next, CLAIM_SUCCESS, GATING )
ring_cursor/src/lib.rs:    self.0.get().compare_exchange( current, new, success, failure )
ring_publish/src/lib.rs:    self.cursor.compare_exchange( start, end, PUBLISH, GATING ).map( | _ | end )
      1 Ordering::AcqRel
      2 Ordering::Release
    self.start.advanced_by( self.len as u64 )
      let next = current.advanced_by( count as u64 );
      let next = current.advanced_by( granted as u64 );
```

| | Value |
|--|------:|
| Claiming functions | 2 |
| …sharing the same retry structure | 2 |
| Lines that differ between them | 3 — guard, condition, width |
| Places the gate could go | 3 |
| …that are correct | **1** |
| Loop exits | 2 — one `Ok`, one `Err` |
| `Err( RingError::Full )` return sites | 2 — one per function |
| Retry budget | **none** — bounded by the ring, not a counter |
| `compare_exchange` sites in this crate | 2 |
| …in the family, excluding trait declarations and impls | 3 |
| Named ordering constants family-wide | 11 |
| …whose value is `AcqRel` | **1** — this crate's |
| Casts in the crate | 3 — all `usize as u64`, all widening |
| Inputs on which the two functions disagree | **1** — zero |

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CL7 | `ring_claim` | n/a — observation | The gate is the loop condition, so it is re-read against the value the failed exchange returned; of the three places it could go, the tempting one is the shape of `GatingSet::check` and is a race |
| CL8 | `ring_claim` | n/a — diagnostics | Both loop exits report `Err( RingError::Full )`, for a genuinely full ring and for a producer that lost every exchange; the collapse is operationally correct and diagnostically lossy, which is what `GatingSet::limit` exists to recover |
| CL9 | `ring_claim` | **misleading doc** | `claim( 0 )` returns `Ok` on a full ring and `claim_up_to( 0 )` returns `Err( Full )`; both are tested, both are defensible, and neither divergence appears in either `# Errors` section |
| CL10 | `ring_claim` | n/a — coverage | `claim_up_to` recomputes its grant width on every retry and `claim` does not, so a loser adapts to a smaller ring rather than failing; that also makes its retry arm strictly harder to reach, which is how the coverage gate found it untested |

Supporting observations recorded alongside those findings, carrying no ID of their own:

| Note | Where |
|------|-------|
| `tests/manual/readme.md § C3` asserts the placement by *shape* rather than behaviour, because the behavioural failure needs two threads to interleave inside a window a few instructions wide | [001](001_the_gate_inside_the_retry.md) |
| There is no retry budget and none is needed: the loop spins only while the gate admits, and the gate admits only after a consumer releases room | [001](001_the_gate_inside_the_retry.md) |
| `CLAIM_SUCCESS` is `AcqRel` — the family's only one — because success both releases the advance and acquires the replaced producer's writes; failure uses the cheaper `GATING` because it grants nothing | [001](001_the_gate_inside_the_retry.md) |
| `claim( 0 )` on a full ring performs a compare-exchange of a value against itself — the crate's most expensive instruction, on its most contended cache line, for a grant of nothing | [002](002_two_loops_that_disagree_at_zero.md) |
| `claim_up_to` has no `BatchTooLarge` guard by contract, because a `max` wider than the ring is a ceiling that never binds rather than a misconfiguration | [002](002_two_loops_that_disagree_at_zero.md) |

