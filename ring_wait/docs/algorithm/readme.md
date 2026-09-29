# algorithm

One loop, three wrappers over it, and no other repetition anywhere in the crate.
Both instances are about the same thirteen lines — the first reads them, the
second reads what fixing one of their parameters costs.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [One Loop and the Two Ways Out](001_one_loop_and_the_two_ways_out.md) | `wait_until`'s body, its three exits, the zero-budget clamp, and WT17 — the same clamp written twice in the family |
| 002 | [Two Wrappers Over a Predicate They Fix](002_two_wrappers_over_a_predicate_they_fix.md) | `for_space` and `for_data`, their one asymmetry, and WT3 — why fixing the predicate is what left them without a production caller |

### The Whole of the Crate's Control Flow

```rust
// ring_wait/src/lib.rs:183-194
for attempt in 0..spins.max( 1 )
{
  if ready()
  {
    return Ok( attempt );
  }
  if !pause( kind, attempt )
  {
    break;
  }
}
Err( RingError::Empty )
```

| | Count | |
|--|------:|--|
| Loops in the crate | 2 | this one, and `pause`'s `for _ in 0..=( attempt % 8 )` |
| Unbounded loops | **0** | W1's grep finds no `loop {` and no `while true` |
| Exits from `wait_until` | 3 | two of which return the same value from the same line |
| Distinct error values produced | 2 | `Empty` at `:194`, `Full` at `:241` |
| Functions that are one line over this loop | 3 | `wait`, `for_space`, `for_data` |

### Regenerate Both Censuses

```sh
cd "$(git rev-parse --show-toplevel)"

# W1 — every repetition is a counted `for`; expect no output
grep -vE "^[[:space:]]*//" ring_wait/src/lib.rs \
  | grep -E "loop[[:space:]]*\{|while[[:space:]]+true"

# W2 — exactly two `RingError::` mentions, and which
grep -vE "^[[:space:]]*//" ring_wait/src/lib.rs \
  | grep -E "RingError::[A-Za-z]+"

# who actually calls into this crate, across the whole family
grep -r --include=*.rs "ring_wait::" . \
  | grep -vE ':\s*(///|//!|//)' | grep -v '^ring_wait/'
```

Live output:

```
  Err( RingError::Empty )
  wait_until( kind, spins, || pair.may_claim() ).map_err( | _ | RingError::Full )
ring_shutdown/src/lib.rs:  ring_wait::wait_until( kind, spins, || shutdown.is_closed() )
ring_shutdown/src/lib.rs:  let outcome = ring_wait::wait_until( kind, spins, ||
ring_barrier/src/lib.rs:    ring_wait::wait_until( kind, spins, || self.admits( from, count ) )?;
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| WT3 | `ring_shutdown` | n/a — duplication | Fixing the predicate is what makes both named wrappers unreachable: a caller needing one extra exit condition has nowhere to put it, so `ring_shutdown::for_space_or_close` re-typed `for_space`'s closure over `wait_until` rather than wrapping it |
| WT9 | `ring_wait` | n/a — coverage | `for_data( pair, 0, … )` returns `Ok( 0 )` on a ring that was never published to, because `pending() >= 0` is a tautology; no test and no doctest covers it |
| WT17 | family | n/a — duplication | `spins.max( 1 )` and `ring_poll::Budget::new`'s `if attempts == 0 { Self( 1 ) }` are the same rule written twice, at the loop header and at the constructor; `ring_poll` may not depend on this crate, so they cannot be shared |
| WT25 | `ring_wait` | n/a — doc gap | `wait_until` returns `Result< usize, RingError >` and never documents what the `usize` counts: no `# Returns` section, and a doctest that asserts `is_ok()` plus the closure's own counter rather than the value |
