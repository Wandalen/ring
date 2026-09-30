# Pitfall: Claiming Zero

### Scope

- **Purpose**: Record that `claim` and `claim_up_to` answer "what does a request for zero mean" in opposite directions, that both answers are deliberate, and that neither appears in the documentation a caller reads.
- **Responsibility**: Show the divergence on both an empty and a full ring, locate where each behaviour *is* explained, and show that `claim_up_to`'s stated error condition is false for exactly this input.
- **In Scope**: `claim( 0 )` and `claim_up_to( 0 )`.
- **Out of Scope**: The loop mechanics that produce the divergence — see [`algorithm/002`](../algorithm/002_two_loops_that_disagree_at_zero.md).

### The Divergence

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/^  pub fn claim( &self, count : usize ) -> Result< Claim, RingError >$/,/^    Err( RingError::Full )$/p;/^  pub fn claim_up_to( &self, max : usize ) -> Result< Claim, RingError >$/,/^  }$/p' ring_claim/src/lib.rs
```

Live output:

```
  pub fn claim( &self, count : usize ) -> Result< Claim, RingError >
  {
    if count > self.consumers.capacity().get()
    {
      return Err( RingError::BatchTooLarge
      {
        requested : count,
        capacity : self.consumers.capacity().get(),
      } );
    }

    // The gate is the loop condition, and is therefore re-read on every
    // iteration: on a failed exchange another producer moved the cursor, so
    // the headroom computed against the old value is stale and granting on it
    // would overlap that producer's range.
    let mut current = self.claimed();
    while count <= self.consumers.headroom( current )
    {
      let next = current.advanced_by( count as u64 );
      match self.cursor.compare_exchange( current, next, CLAIM_SUCCESS, GATING )
      {
        Ok( _ ) => return Ok( Claim::new( current, count ) ),
        Err( actual ) => current = actual,
      }
    }

    Err( RingError::Full )
  pub fn claim_up_to( &self, max : usize ) -> Result< Claim, RingError >
  {
    // `granted @ 1..` binds the grant and gates on it in one expression, which
    // is what keeps the headroom re-read in the loop condition rather than
    // duplicated between a pre-loop computation and the retry arm. A grant of
    // zero — no room, or a `max` of zero — exits to the `Full` below.
    let mut current = self.claimed();
    while let granted @ 1.. = max.min( self.consumers.headroom( current ) )
    {
      let next = current.advanced_by( granted as u64 );
      match self.cursor.compare_exchange( current, next, CLAIM_SUCCESS, GATING )
      {
        Ok( _ ) => return Ok( Claim::new( current, granted ) ),
        Err( actual ) => current = actual,
      }
    }

    Err( RingError::Full )
  }
```

Reproduced against the shipping crate, capacity 4, one consumer:

```
on an EMPTY ring
  claim( 0 )        -> Ok(Claim { start: Seq(0), len: 0 })
  claim_up_to( 0 )  -> Err(Full)
  cursor after both -> Seq(0)

on a FULL ring
  claim( 0 )        -> Ok(Claim { start: Seq(4), len: 0 })
  claim_up_to( 0 )  -> Err(Full)
  claim( 1 )        -> Err(Full)
  claim_up_to( 1 )  -> Err(Full)
```

Two rows are worth reading twice.

`claim( 0 )` **succeeds on a completely full ring** — it is the only call in the
crate that does. Every other request returns `Full` there; a request for nothing
is granted, because nothing always fits.

`claim_up_to( 0 )` **fails on a completely empty ring** — the only call that
does. Four slots are free and the answer is `Full`.

So on either ring, the two methods disagree, and on each ring the disagreement
points the opposite way.

### CL45 — Both Answers Are Deliberate, and Both Are Explained Only in the Test File

Neither behaviour is an accident. Both are pinned by a test, and each test
carries the reasoning in its assertion message or its comment:

| Behaviour | Test | The reasoning, quoted |
|-----------|------|-----------------------|
| `claim( 0 )` → `Ok` | `claiming_zero_succeeds_and_moves_nothing:224` | *"zero always fits, even on a full ring"* |
| `claim_up_to( 0 )` → `Err( Full )` | `claim_up_to_of_zero_is_full_not_an_empty_claim:262` | *"`claim_up_to`'s contract is 'as many as are available, down to one' — there is no partial success at zero to report"* |

Both arguments are good, and together they are coherent: `claim( n )` means
*exactly n*, and exactly zero is satisfiable always; `claim_up_to( n )` means
*at least one, at most n*, and that has no solution when `n` is zero. The
divergence is not a bug — it follows from the two contracts, correctly.

Now where a caller would look. `claim`'s documentation (`src/lib.rs:387-403`)
runs four lines of prose and a six-line `# Errors` section, and never mentions
zero. `claim_up_to`'s (`:417-426`) does the same. The word appears twice in the
whole source file (`grep -n 'zero' ring_claim/src/lib.rs`):

- `:258` — *"A claimer starting at sequence zero"*, about the initial cursor
- `:447` — *"zero — no room, or a `max` of zero — exits to the `Full` below"*

The second is the only acknowledgement anywhere in the source that `max = 0` is
a distinct case, and it is a **`//` implementation comment inside the function
body**, stripped by the comment filter the rest of this corpus uses and invisible
in `cargo doc`. The reasoning a caller needs lives in two places, and both of
them are files a caller has no reason to open.

### CL46 — `claim_up_to`'s Documented Error Condition Is False for This Input

The `# Errors` section is the whole contract a caller gets (`src/lib.rs:424-426`):

> [`RingError::Full`] when not even one slot is free. Never `BatchTooLarge` — a
> `max` wider than the ring is not an error here, it is simply more than will be
> granted.

Against the reproduction above:

| Ring state | Slots free | `claim_up_to( 0 )` | Doc predicts |
|------------|-----------:|--------------------|--------------|
| empty, capacity 4 | **4** | `Err( Full )` | `Ok` — the stated condition does not hold |
| full, capacity 4 | 0 | `Err( Full )` | `Err( Full )` ✔ |

`Full` is documented as meaning one thing — *not even one slot is free* — and
returned for two, the second being *you asked for none*. A caller who reads the
sentence and reasons from it concludes the ring is saturated, and the ring may be
entirely empty.

That matters because of what `Full` is documented to mean *elsewhere* in the same
file, for the sibling method (`:369-370`):

> back-pressure, so a retry loop should keep going

Put the two together and the natural drain loop is a live trap:

```rust
// `batch` is empty this tick — nothing to write.
let claim = match claimer.claim_up_to( batch.len() )
{
  Ok( c ) => c,
  Err( RingError::Full ) => { backoff(); continue; }   // documented advice
  Err( e ) => return Err( e ),
};
```

With `batch.len() == 0` this spins on `backoff()` forever, waiting for a
consumer to free space in a ring that is already empty. The loop is correct
against the documentation and wrong against the code, and the guard it needs —
`if batch.is_empty() { continue; }` — is only obviously necessary once you know
the divergence exists.

This is the second place the crate's single `Full` variant carries more than one
meaning. [`pattern/001`](../pattern/001_retrying_against_a_moving_target.md)
§ CL40 records the first: a first-evaluation refusal and a forty-attempt loss are
also indistinguishable. Three distinct situations, one error value, and the
documented interpretation is correct for exactly one of them.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -A1 -F 'A `max` of zero is also `Full`' ring_claim/src/lib.rs
```

Live output:

```
    /// A `max` of zero is also `Full`, since there is no partial success at
    /// zero to report — this differs from [`claim`], which treats a `count`
```

**Disposition:** applied — both clauses now exist: `claim_up_to`'s
`# Errors` section states the `max = 0` case (this crate), and `claim`'s
`# Errors` section states the `count = 0` case (→ CL9, same crate, same
fix session). Now prints: `since there is no partial success at`

### What Is Correctly Absent

| Not present | Correctly so |
|-------------|--------------|
| a `BatchTooLarge` for `claim_up_to( 0 )` | zero is not too large; the variant would be a lie in the other direction |
| an `Empty`/`NothingRequested` variant | it would widen `RingError` family-wide for one caller mistake, and `RingError` is a `ring_types` concern shared by 33 crates |
| a `debug_assert!( max > 0 )` | `claim_up_to( 0 )` is not a bug — the contract genuinely has no solution at zero |
| making `claim( 0 )` fail for symmetry | it would break `claiming_zero_succeeds_and_moves_nothing` and the contract that test defends |

The fix is documentation, not code. One clause in each `# Errors` section — *"a
`max` of zero is `Full`, since there is no partial success at zero to report"*
and *"a `count` of zero always succeeds, even on a full ring"* — moves both
already-written arguments from the test file to the page a caller reads, and
changes no behaviour.

### The One Real Cost

`claim( 0 )` does not merely return `Ok`; it takes the full path to get there.
The loop condition `0 <= headroom( current )` holds unconditionally, so the body
runs, `advanced_by( 0 )` yields `current`, and the crate issues a
`compare_exchange( current, current, AcqRel, Acquire )` — a genuine atomic RMW
that changes nothing.

| Call | Ring | Result | Atomic RMWs issued |
|------|------|--------|-------------------:|
| `claim( 0 )` | any | `Ok( empty )` | **1** |
| `claim_up_to( 0 )` | any | `Err( Full )` | **0** |
| `claim( 1 )` | full | `Err( Full )` | 0 |

The cheapest possible request is the one that pays, and the two refusals are
free. That inversion is the same one
[`invariant/002`](../invariant/002_a_refused_claim_moves_nothing.md) records
from the other direction, and it is harmless — a producer polling `claim( 0 )` in
a loop is not a shape anyone writes — but it is the reason "zero costs nothing"
is false as stated.

### Pitfalls

| File | Relationship |
|------|--------------|
| [001_dropping_a_claim.md](001_dropping_a_claim.md) | The other undocumented behaviour, and the third meaning of `Full` |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_two_loops_that_disagree_at_zero.md](../algorithm/002_two_loops_that_disagree_at_zero.md) | The loop mechanics that produce the divergence |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_two_constructors_of_a_range.md](../api/002_the_two_constructors_of_a_range.md) | The two granting methods compared as a surface |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_a_refused_claim_moves_nothing.md](../invariant/002_a_refused_claim_moves_nothing.md) | The refusal that costs nothing, against the grant that costs one |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_retrying_against_a_moving_target.md](../pattern/001_retrying_against_a_moving_target.md) | `Full`'s first overloaded meaning |

### Sources

| File | Relationship |
|------|--------------|
| `ring_claim/src/lib.rs:394-400` | `claim`'s contract — no mention of zero |
| `ring_claim/src/lib.rs:450-463` | `claim_up_to`'s contract, and the `Full` clause that is false at zero |
| `ring_claim/src/lib.rs:486` | The only acknowledgement in the source, in a stripped implementation comment |
| `ring_claim/src/lib.rs:432-445,483-496` | The two loops, and why one admits zero and the other cannot |

### Tests

| File | Relationship |
|------|--------------|
| `tests/claim_test.rs:224` — `claiming_zero_succeeds_and_moves_nothing` | Pins `Ok`, and carries the reasoning in its `expect` message |
| `tests/claim_test.rs:262` — `claim_up_to_of_zero_is_full_not_an_empty_claim` | Pins `Err( Full )`, and carries the reasoning in a comment |
| `tests/claim_test.rs:249` — `claim_up_to_never_reports_batch_too_large` | The other end of the same contract — an over-wide `max` is a cap, not an error |
