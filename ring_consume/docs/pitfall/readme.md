# pitfall

Two ways to be wrong here, and neither is a mistake in the source. The first is
a reader's: `commit_available` does not call `commit`, so every guarantee the
guarded function provides is absent from the convenient one — and the names
suggest otherwise. The second is a measurer's: the case most likely to be probed
first is the one case that reports zero, which is how an allocation on every read
survived in two Tier 5 crates.

They pair because both are traps laid by a reasonable expectation rather than by
bad code. Reading the names, you expect delegation. Measuring the simplest
configuration, you expect it to be representative. Neither holds.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [`commit_available` Does Not Call `commit`](001_commit_available_does_not_call_commit.md) | CN43, CN44 — a duplicated store rather than a delegation, and an idle poll that used to write |
| 002 | [The Empty Barrier Is the Case Nobody Measured](002_the_empty_barrier_is_the_case_nobody_measured.md) | CN45, CN46 — an asymmetry argued in one crate and asserted in three, and the configuration that reports clean |

### The Two, Side by Side

| | 001 — the two stores | 002 — the empty barrier |
|--|---------------------|-------------------------|
| The expectation | `commit_available` delegates to `commit` | the simplest config is representative |
| The reality | two independent `self.cursor.store` sites | it is the one config that allocates nothing |
| Why it survives | both are correct today | the cheap check passes |
| Would break when | a guard, a trace, or a counter is added to `commit` | never — it stays correct and stays unrepresentative |
| Found by | reading both bodies | a counting allocator |

### Why the Duplication Is Latent Rather Than Present

```rust
pub fn commit( &self, through : Seq ) -> Result< Seq, RingError >
{
  // ... two comparisons ...
  self.cursor.store( through, COMMIT );          // store site 1
  Ok( through )
}

pub fn commit_available( &self ) -> Seq
{
  let run = self.available();
  let end = run.end();
  // ... a debug assertion, and the CN44 emptiness guard ...
  self.cursor.store( end, COMMIT );              // store site 2
  end
}
```

Nothing is wrong. `commit_available`'s argument is `available().end()`, which
`commit` would accept unconditionally, so the guard it skips would never have
fired. The two functions agree today for a reason, not by luck.

The exposure is that the reason is arithmetic rather than structural. Anything
added to `commit` — a debug assertion, a trace hook, a statistic, a second
ordering — silently does not apply to `commit_available`, and no test would
notice because both still move the cursor to the same place.

### The Idle Poll

`commit_available` used to store unconditionally, including when the available
run was empty. A consumer polling an idle ring wrote its own current position
back to its own cursor on every iteration — a `Release` store to a cache line
the producer is reading, for no state change at all.

Correct, and the wrong shape for a cost: the case with nothing to do is the case
that runs most often. The body now guards the store with `if end != run.start()`,
so an idle poll performs none — and CN34's allocation went the same way in
`b7e075ca`. What an idle poll costs today is loads.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# the two stores, and the fact that neither calls the other
command grep 'self.cursor.store' ring_consume/src/lib.rs
command grep -A8 'pub fn commit_available' ring_consume/src/lib.rs

# CN44's guard. Counted by what it *does* -- a comparison of the run's two ends
# guarding the store -- not by one spelling of it: the fix is `end != run.start()`,
# so a grep for `is_empty` reports zero and reads as "still unconditional".
printf '  guarded stores in commit_available: %s\n' \
  "$( command grep -A8 'pub fn commit_available' ring_consume/src/lib.rs \
      | command grep -cE 'if .*(is_empty|end *[!=]= *run\.start)' )"

# the empty-barrier path: where frontier() short-circuits
command grep -vE "^[[:space:]]*//" ring_cursor/src/lib.rs | command grep -A4 'pub fn slowest'
command grep 'map_or' ring_consume/src/lib.rs

# the asymmetry argued once and asserted three times. It is the *empty-barrier*
# asymmetry -- an empty barrier means nothing published, where an empty gating
# set means unbounded -- not the permission/obligation one, which is CN19's and
# lives in neither of these crates.
command grep -i 'asymmetry\|the distinction the module documentation argues' \
  ring_barrier/src/lib.rs ring_gating/tests/gating_test.rs \
  ring_barrier/tests/barrier_test.rs ring_consume/tests/consume_test.rs \
  | sed 's|ring/||'
```

Live output:

```
    self.cursor.store( through, COMMIT );
      self.cursor.store( end, COMMIT );
  pub fn commit_available( &self ) -> Seq
  {
    let run = self.available();
    let end = run.end();
    debug_assert!( end >= run.start(), "available() must never return end < start" );
    if end != run.start()
    {
      self.cursor.store( end, COMMIT );
    }
  guarded stores in commit_available: 1
pub fn slowest( cursors : &[ PaddedCursor ] ) -> Option< Seq >
{
  cursors.iter().map( | c | c.load( GATING ) ).min()
}

      .map_or( 0, | frontier | ring_seqno::pending( frontier, position ) );
ring_barrier/src/lib.rs://! where an empty set means *unbounded*. The asymmetry is not an inconsistency:
ring_gating/tests/gating_test.rs:  // The distinction the module documentation argues: an empty gating set is
ring_barrier/tests/barrier_test.rs://! ## The asymmetry with `ring_gating` is asserted, not assumed
ring_barrier/tests/barrier_test.rs:  // The distinction the module documentation argues, read off one set of
ring_barrier/tests/barrier_test.rs:  // Stated as an assertion for the same reason as the gating-set asymmetry
ring_consume/tests/consume_test.rs:  // the asymmetry with `ring_gating` that `ring_barrier`'s own suite asserts,
```

The allocation figures — 1000/1000 with one dependency, 0/1000 with an empty
barrier — come from the counting `GlobalAlloc` probe quoted in
[`non_functional_requirement/001`](../non_functional_requirement/001_what_the_read_path_costs.md).

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CN43 | `ring_consume` | **latent hazard** | `commit_available` duplicates the store rather than delegating to `commit`; correct today for an arithmetic reason, and silently divergent the moment anything is added to `commit` |
| CN44 | `ring_consume` | **measured cost** | An idle poll used to write — `commit_available` stored unconditionally, so a consumer with nothing to read issued a `Release` store to a contended line every iteration; the store is now guarded by `end != run.start()` |
| CN45 | `ring_barrier` | n/a — observation | The empty-barrier asymmetry against `ring_gating` is argued once, in `ring_barrier`'s module doc, and referenced rather than restated by the three test files that inherit it |
| CN46 | `ring_consume` | n/a — coverage | The empty-barrier configuration is the one that allocates nothing, so the cheapest measurement is the one that reports clean — which is how the allocation survived in two crates |
