# algorithm

There is barely an algorithm here, and that is the finding. `available()` is
three steps with no branch and no loop; `commit()` adds two comparisons. The
whole crate is straight-line code, which is the sharpest possible contrast with
the write half, whose defining feature is a retry loop against a moving target.

Both instances are about what the absence of a loop implies. The first records
that the read path never retries because it never contends. The second takes
`commit`'s two comparisons apart and finds that the upper bound is inclusive
where every other range operation in the family is half-open — deliberately, so
that `commit( run.end() )` means "all of it".

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Position, Frontier, Pending](001_position_frontier_pending.md) | CN14, CN15 — three steps, no branch, and a clamp that bounds the length and never the start |
| 002 | [The Two-Sided Guard](002_the_two_sided_guard.md) | CN16, CN17 — an `# Errors` section documenting both conditions, and a refused commit costing exactly what an accepted one costs |

### The Whole Read Path, Once

```rust
pub fn available( &self ) -> Available
{
  let position = self.position();                        // 1 load
  let readable = self
    .barrier
    .frontier()                                          // n loads, 1 alloc
    .map_or( 0, | frontier | ring_seqno::pending( frontier, position ) );
  Available::new( position, readable )
}
```

Three steps. No branch a caller can influence, no loop, no failure. The
`map_or`'s two arms are the only fork and it is decided by whether the barrier
has dependencies, not by contention.

`commit` prefixes two comparisons:

```rust
if through < run.start() || through > run.end() { return Err( RingError::Empty ); }
self.cursor.store( through, COMMIT );
```

And `commit_available` drops even those.

### Why There Is No Loop

`ring_claim` retries because two producers race for the same range and the loser
must re-read and try again. A consumer has no competitor by construction
([`decisions/002`](../decisions/002_plain_stores_rather_than_compare_exchange.md)),
so there is nothing to lose a race to and nothing to retry. The absence of the
loop is the single-consumer premise made visible in the control flow.

That makes the read path's cost fully predictable — no unbounded spin, no
attempt count, no back-off decision. It used to make one cost stand out against
that flatness — an allocation, per call, inside `frontier()` — and since commit
`b7e075ca` there is nothing left to stand out
([`workaround/002`](../workaround/002_a_vector_to_change_a_slices_type.md)).

### The Inclusive Upper Bound

`commit`'s upper comparison is `through > run.end()`, so `through == run.end()`
is accepted. Every other range operation in the crate is half-open. The
exception is deliberate and load-bearing: `end` is the *next* unread sequence,
so committing "through `end`" is the natural way to say "all of it", and
`commit_available` is exactly that call spelled shorter.

The 900-case sweep at `consume_test.rs:253` asserts the boundary in both
directions, which is the right test and is where the intent is recorded.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# the whole read path, comment lines stripped
grep -vE "^[[:space:]]*//" ring_consume/src/lib.rs | grep -A10 'pub fn available('

# no loop, no retry, anywhere in the crate
grep -E '\bloop\b|\bwhile\b|compare_exchange' ring_consume/src/lib.rs || echo "  none"

# the write half, for contrast
grep -E '\bloop\b|\bwhile\b|compare_exchange' ring_claim/src/lib.rs

# commit's two comparisons, and the inclusive upper bound
grep -A8 'pub fn commit(' ring_consume/src/lib.rs

# what asserts the boundary in both directions
grep 'fn the_accepted_commits' ring_consume/tests/consume_test.rs
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

//! those slots for the producer while they are still being read — the exact
/// while the reads that freed them are still in flight.
//! ## Why the CAS loop is not `fetch_add`
//! The compare-exchange loop re-reads the gate inside the retry, so the
  /// diagnostic, worth avoiding in a hot loop with many consumers.
  /// configuration error no consumer's progress can fix, so a retry loop must
  /// back-pressure, so a retry loop should keep going.
    // The gate is the loop condition, and is therefore re-read on every
    while count <= self.consumers.headroom( current )
      match self.cursor.compare_exchange( current, next, CLAIM_SUCCESS, GATING )
    // is what keeps the headroom re-read in the loop condition rather than
    // duplicated between a pre-loop computation and the retry arm. A grant of
    while let granted @ 1.. = max.min( self.consumers.headroom( current ) )
      match self.cursor.compare_exchange( current, next, CLAIM_SUCCESS, GATING )
  pub fn commit( &self, through : Seq ) -> Result< Seq, RingError >
  {
    let run = self.available();
    if through < run.start() || through > run.end()
    {
      return Err( RingError::Empty );
    }

    self.cursor.store( through, COMMIT );
fn the_accepted_commits_are_exactly_the_available_range_inclusive_of_both_ends()
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CN14 | `ring_consume` | n/a — observation | The whole computation is straight-line: no loop, no retry, no contention to resolve — the exact inverse of the write half, and neither crate says so |
| CN15 | `ring_consume` | n/a — doc gap | `available_up_to` clamps the length and never the start, which the name does not convey |
| CN16 | `ring_consume` | n/a — observation | `commit`'s `# Errors` documents both failure conditions, which the write half's equivalent does not |
| CN17 | `ring_consume` | n/a — doc gap | A refused commit performs every load an accepted one does, so the guard is not a cheap early-out and nothing says so |
