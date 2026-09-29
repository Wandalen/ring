# Algorithm: The Gate Inside the Retry

### Scope

- **Purpose**: Describe the compare-exchange loop both claiming functions share, and account for the one structural property that makes it correct — the gate is the loop *condition*, so it is re-evaluated against the cursor value the failed exchange returned.
- **Responsibility**: Walk the loop, name what each retry re-reads, and show what the two alternative placements of the gate would cost.
- **In Scope**: The shared retry structure, its two exits, and the ordering pair it exchanges under.
- **Out of Scope**: How the two functions differ inside that structure — see [`algorithm/002`](002_two_loops_that_disagree_at_zero.md).

### The Loop

Both functions are the same five lines. `claim` (`src/lib.rs:404-411`):

```rust
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
```

Four things happen per iteration, and the order is the algorithm:

| Step | Reads | Why here |
|------|-------|----------|
| 1 | `headroom( current )` | the gate, against the value about to be exchanged |
| 2 | `current.advanced_by( … )` | the target, derived from that same value |
| 3 | `compare_exchange( current, next, … )` | grant, conditional on `current` still being the cursor |
| 4 | `current = actual` | adopt what was actually there and go back to step 1 |

Step 4 is what makes step 1 correct on the second pass: `actual` is another
producer's grant, so the headroom computed against the old `current` describes
a ring that no longer exists.

### CL7 — The Gate Is the Loop Condition, and That Placement Is the Correctness Argument

`src/lib.rs:400-403` says so in the source:

> The gate is the loop condition, and is therefore re-read on every iteration:
> on a failed exchange another producer moved the cursor, so the headroom
> computed against the old value is stale and granting on it would overlap that
> producer's range.

There are three places the gate could go, and only one of them is safe:

| Placement | Shape | Failure |
|-----------|-------|---------|
| before the loop | `if !admits(…) { return Err }` then `loop { CAS }` | a retry grants against headroom measured before another producer took it |
| inside the body, after the CAS fails | `loop { CAS; if !admits(…) { return Err } }` | the first iteration grants without ever consulting the gate |
| **as the loop condition** | `while admits(…) { CAS }` | — |

The first is the interesting one, because it is what a reader reaching for
`GatingSet::check` would write, and `check` exists and is shaped exactly for it
([`integration/002`](../integration/002_four_predicates_and_the_one_that_is_called.md) § CL5).
It looks like a refactor and it is a race.

`tests/manual/readme.md § C3` is the check that keeps this property from
drifting: it asserts the `headroom` call appears *in* the `while` line, not on a
line before it. That is a shape assertion rather than a behavioural one, which
is unusual and deliberate — the behavioural failure needs two threads to
interleave in a window a few instructions wide, so the test that can catch it
reliably is one that reads the source.

### CL8 — Both Exits Are `Err( RingError::Full )`, and Only One of Them Means Full

The loop has exactly two ways out and they return different things:

| Exit | Reached when | Returns |
|------|--------------|---------|
| `Ok( _ )` arm of the `match` | the exchange landed | `Ok( Claim )` |
| falling out of the `while` | the gate stopped admitting | `Err( RingError::Full )` |

The `while` is left when `headroom( current )` drops below what is asked — but
`current` at that moment may be a value this producer never proposed anything
against. A producer can enter the loop with room, lose the exchange to a peer,
adopt the peer's value, find no room at *that* value, and exit. It never had a
turn; it reports the same `Full` as a producer that found the ring full on its
first read.

This is correct and it is also the crate's one lossy report. The two situations
differ in what the caller should do:

| Situation | Right response |
|-----------|----------------|
| the ring is genuinely full | wait for a consumer |
| a peer took the last room mid-retry | the same wait — the room is gone either way |

They collapse because the *outcome* is identical: there is no space at the
current frontier. What is lost is diagnostic, not operational — a stall caused
by contention and a stall caused by a slow consumer are indistinguishable from
the return value, and `GatingSet::limit` exists precisely so a diagnostic can
tell them apart by absolute position rather than by error variant
([`integration/002`](../integration/002_four_predicates_and_the_one_that_is_called.md) § CL6).

There is no retry budget. The loop spins while the gate admits, and the gate
admits only when a consumer has released room — so an unbounded spin requires an
unbounded supply of *successful peer claims*, each of which consumes headroom.
The loop is bounded by the ring, not by a counter
([`non_functional_requirement/002`](../non_functional_requirement/002_what_contention_costs.md)).

### The Ordering Pair

```rust
const CLAIM_SUCCESS : core::sync::atomic::Ordering = core::sync::atomic::Ordering::AcqRel;  // :76
```

exchanged against `GATING` on failure, which `ring_cursor` defines as the
family's shared gate-read ordering. `src/lib.rs:70-75` argues the success side:

> `AcqRel` rather than `Release`: the success case is both a release of the
> cursor advance to other producers and an acquire of whatever the producer
> whose value we replaced had done. A bare `Release` would let this producer's
> slot writes be reordered before it observed the previous producer's claim.

The failure ordering is `GATING` rather than `Acquire` because a failed exchange
grants nothing — it only produces the value for step 4, and step 1 re-reads the
gate anyway. So the strong ordering is paid on the path that succeeds once, and
the cheap one on the path that may repeat.

### Where the Loop Is Verified

| Test | What it establishes |
|------|---------------------|
| `no_two_producers_are_ever_granted_the_same_sequence` | 4 threads × 2,000 claims, every sequence seen once |
| `claims_under_contention_lose_no_sequences` | 4 × 500 × 3 — the grant space is contiguous with no gaps |
| `each_producers_own_claims_stay_in_issue_order` | 3 × 1,000 — a producer's own grants are monotonic |
| `no_grant_ever_passes_the_limit_under_contention` | capacity 4, 4,000 releases — the gate holds under maximal retry pressure |
| `tests/manual/readme.md § C1` | the loop replaced by check-then-`fetch_add` fails the suite on every run |

`§ C1` is the one that makes the rest evidence rather than ceremony: it is a
mutation check, and the mutation it applies is exactly the rejected design.

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_two_constructors_of_a_range.md](../api/002_the_two_constructors_of_a_range.md) | The two error variants the loop can produce |

### Algorithms

| File | Relationship |
|------|--------------|
| [002_two_loops_that_disagree_at_zero.md](002_two_loops_that_disagree_at_zero.md) | What the two functions do differently inside this structure |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_compare_exchange_rather_than_fetch_add.md](../decisions/001_compare_exchange_rather_than_fetch_add.md) | The rejected alternative, and what `§ C1` mutates to |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_four_predicates_and_the_one_that_is_called.md](../integration/002_four_predicates_and_the_one_that_is_called.md) | Why the gate cannot be a call to `GatingSet::check` |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_no_two_producers_hold_one_sequence.md](../invariant/001_no_two_producers_hold_one_sequence.md) | The property this placement exists to hold |

### Non Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_what_contention_costs.md](../non_functional_requirement/002_what_contention_costs.md) | What a retry costs, and why the loop needs no budget |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_retrying_against_a_moving_target.md](../pattern/001_retrying_against_a_moving_target.md) | The same structure, against the family's other compare-exchange loop |

### Sources

| File | Relationship |
|------|--------------|
| `ring_claim/src/lib.rs:52-62` | The module doc's rejection of `fetch_add` |
| `ring_claim/src/lib.rs:70-76` | `CLAIM_SUCCESS`, and the argument for `AcqRel` |
| `ring_claim/src/lib.rs:432-447` | The loop, its comment, and both exits |
| `ring_claim/src/lib.rs:483-498` | The same loop in `claim_up_to` |
| `ring_gating/src/lib.rs:222-228` | The gate the condition calls |
| `ring_cursor/src/lib.rs:216-220` | The forwarding `compare_exchange` |

### Tests

| File | Relationship |
|------|--------------|
| `tests/claim_test.rs` — the 5 `thread::scope` blocks | Every concurrency property the loop is responsible for |
| `tests/manual/readme.md § C1` | The mutation to check-then-`fetch_add`, and its expected failure |
| `tests/manual/readme.md § C3` | The gate asserted to be the loop condition by shape |
