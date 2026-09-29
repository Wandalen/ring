# Algorithm: Position, Frontier, Pending

### Scope

**Purpose:** Set out how the crate computes what may be read, step by step, and
establish that the computation is three reads and one subtraction with no loop
and no retry.

**Responsibility:** `Consumer::available` and `Consumer::available_up_to` — their
steps, their termination, and their behaviour at the boundaries.

**In Scope:** `ring_consume/src/lib.rs:336-370`; the `map_or` default;
the clamping in `available_up_to`.

**Out of Scope:** What the computation costs — that is
[`non_functional_requirement/001`](../non_functional_requirement/001_what_the_read_path_costs.md).
The commit that follows it, which is
[`002`](002_the_two_sided_guard.md).

---

## Three Steps, No Loop

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A34 -F '  pub fn available( &self ) -> Available' ring_consume/src/lib.rs
```

Live output:

```
  pub fn available( &self ) -> Available
  {
    let position = self.position();
    let readable = self
      .barrier
      .frontier()
      .map_or( 0, | frontier | ring_seqno::pending( frontier, position ) );

    Available::new( position, readable )
  }

  /// At most `max` of what is available, for a consumer with a batch limit.
  ///
  /// ```
  /// use core::sync::atomic::Ordering;
  /// use ring_barrier::Barrier;
  /// use ring_consume::Consumer;
  /// use ring_cursor::{ PaddedCursor, SeqCell };
  /// use ring_types::Seq;
  ///
  /// let published = [ PaddedCursor::default() ];
  /// published[ 0 ].store( Seq( 9 ), Ordering::Release );
  ///
  /// let position = PaddedCursor::default();
  /// let consumer = Consumer::new( &position, Barrier::over( &published ) );
  /// assert_eq!( consumer.available_up_to( 4 ).len(), 4 );
  /// assert_eq!( consumer.available_up_to( 100 ).len(), 9, "capped by what is there" );
  /// ```
  #[ must_use ]
  pub fn available_up_to( &self, max : u64 ) -> Available
  {
    let run = self.available();
    Available::new( run.start(), run.len().min( max ) )
  }
```

The body:

```rust
pub fn available( &self ) -> Available
{
  let position = self.position();
  let readable = self
    .barrier
    .frontier()
    .map_or( 0, | frontier | ring_seqno::pending( frontier, position ) );

  Available::new( position, readable )
}
```

| Step | Operation | Where it happens |
|------|-----------|------------------|
| 1 | read this consumer's own cursor | `ring_cursor`, one atomic `Acquire` load |
| 2 | read the slowest dependency cursor | `ring_barrier` → `ring_cursor::slowest`, `n` loads |
| 3 | subtract | `ring_seqno::pending` |

### CN14 — The Whole Computation Is Straight-Line, and That Is the Point

No loop, no retry, no compare-exchange, no branch except the `map_or`. The
function reads two positions and subtracts them.

This matters more than its simplicity suggests, because the *write* half cannot
be shaped this way. `ring_claim`'s `claim` is a compare-exchange retry loop
whose termination rests on other producers making progress; its cost model is
`k × C` for `k` retries and `C` consumers, and both factors rise together under
load. `ring_consume`'s `available` has no `k`. Its cost is exactly `C + 1`
atomic loads, every time, regardless of contention — because there is nothing
to contend with. One consumer, one cursor, no writers to race. Until `b7e075ca`
it was those loads plus one heap allocation, which was equally unconditional
and equally independent of contention.

So the read half is not merely simpler than the write half; it is
*unconditionally* bounded where the write half is only probabilistically
bounded. A caller can reason about the worst case of `available()` exactly. A
caller cannot do that for `claim()` and `ring_claim`'s corpus records that the
gap is undocumented there.

Neither crate states the contrast. It is the single most useful thing a caller
choosing where to put a latency budget could know about the two halves.

**Cost:** none. Recorded as a fact about the pair that neither crate makes
available.

---

### CN15 — `available_up_to` Clamps the Length and Never the Start

```rust
pub fn available_up_to( &self, max : u64 ) -> Available
{
  let run = self.available();
  Available::new( run.start(), run.len().min( max ) )
}
```

The start is passed through untouched; only the length is capped. That is the
correct choice and it is worth recording because the alternative is tempting
and wrong: clamping the *end* to `start + max` produces the same answer here but
invites an implementation that computes an end and subtracts, which reintroduces
the wraparound question `ring_seqno::pending` already answers.

Two boundary behaviours follow, and both are tested:

| Call | Result | Test |
|------|--------|------|
| `available_up_to( 0 )` | empty, starting where the consumer stands | `available_up_to_zero_is_empty_not_everything` |
| `available_up_to( n )`, `n > len` | the full run, unchanged | `available_up_to_caps_without_changing_the_start` |

The first test's name records a real hazard that this implementation avoids: a
`max` of zero meaning "no limit" is a common convention, and a caller passing a
computed zero would then drain the whole ring rather than nothing. `min` gives
the safe reading, and the test asserts it by name rather than leaving it to be
inferred.

What `available_up_to` does *not* do is avoid any of `available`'s cost. It
calls `available()` first, in full, and then discards part of the answer. A
caller batching in chunks of eight pays the same `C + 1` loads as a caller
draining everything — and, before `b7e075ca`, the same allocation
([`non_functional_requirement/001`](../non_functional_requirement/001_what_the_read_path_costs.md)
measured both at 1000 allocations per 1000 calls, which is where the equality
was first shown; both now measure zero). That is unavoidable given the
shape — you cannot know a run is longer than eight without computing its length
— but it means `available_up_to` is a convenience over the result, not a cheaper
path to it, and nothing says so.

**Cost:** none for correctness. Recorded because the name suggests a bounded
query and the implementation is an unbounded one with a `min` on the end.

---

## Termination

Trivial, and stated for completeness because `ring_claim`'s equivalent is not:

| | `ring_consume::available` | `ring_claim::claim` |
|--|--------------------------|---------------------|
| Loops | 0 | 1, unbounded |
| Terminates because | straight-line code | another producer made progress |
| Worst case | `C + 1` loads | unbounded in principle |

---

## Cross-References

| Definition | Instance | Relationship |
|------------|----------|--------------|
| algorithm | [002](002_the_two_sided_guard.md) | the commit that follows the read |
| non_functional_requirement | [001](../non_functional_requirement/001_what_the_read_path_costs.md) | what each of the three steps costs |
| pitfall | [002](../pitfall/002_the_empty_barrier_is_the_case_nobody_measured.md) | the `map_or( 0, … )` default and why it is zero |
| integration | [002](../integration/002_eight_methods_and_the_one_that_is_called.md) | `frontier`, the step-2 call |
| item | [001](../item/001_the_six_of_a_run.md) | `Available`, the value step 3 produces |

### Sources

| What | Where |
|------|-------|
| `available` | `ring_consume/src/lib.rs:336-344` |
| `available_up_to` | `ring_consume/src/lib.rs:365-369` |
| The one load | `ring_consume/src/lib.rs:309` |
| `ring_seqno::pending` | `ring_seqno/src/lib.rs` |

### Tests

| Claim | Verified by |
|-------|-------------|
| No loop or retry | reading the body; `grep -c '\bloop\b'` on the crate → 0 |
| `available_up_to( 0 )` is empty | `consume_test.rs:186-192` |
| The start survives clamping | `consume_test.rs:171` |
| Both cost the same | `available_up_to` calls `available()` and clamps the result, so the loads are identical by construction; the frozen allocation probe showed the same equality at 1000/1000 for each, before `b7e075ca` took both to zero |
