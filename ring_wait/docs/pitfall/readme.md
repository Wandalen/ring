# pitfall

Two things in this crate are not what their names suggest, and neither is a bug.
`RingError::Empty` is what a producer gets when the ring is as full as it can be.
`Spin`'s attempt-dependent pause is called a backoff in two places and resets
every eight attempts.

Both are recorded here because both are the kind of thing a reader concludes
from the name and never re-checks — and in the first case, concluding wrongly
stops a producer that should have retried.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Reading `Empty` as "Nothing to Do"](001_reading_empty_as_nothing_to_do.md) | W2, the single `map_err`, the three public items that hand a producer the wrong word, and `ring_barrier`'s two sources of one error value |
| 002 | [The Backoff That Resets Every Eight Attempts](002_the_backoff_that_resets_every_eight_attempts.md) | WT5 — the period-8 sawtooth, 4608 hints over `DEFAULT_SPINS`, and why nothing asserts the shape |

### The Two Misreadings

| | `Empty` (001) | "backoff" (002) |
|--|---------------|-----------------|
| The name says | nothing available | grows as the wait drags on |
| The code does | the budget ran out | 1, 2, … 8, 1, 2, … 8 |
| Wrong conclusion leads to | a producer stops instead of retrying | expecting a `Spin` wait to become gentler |
| Guarded by | one `map_err`, in one wrapper | nothing |
| Asserted by | `tests/wait_test.rs:255-268` | — the count is not observable |
| Cost if wrong | a stalled producer | a wrong performance expectation |

The asymmetry is deliberate: the first can produce incorrect behaviour and gets a
guard, the second can only produce an incorrect belief and gets a paragraph.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# W2 — exactly two error mentions, and which
grep -vE "^[[:space:]]*//" ring_wait/src/lib.rs \
  | grep -E "RingError::[A-Za-z]+"

# the modulus, and anything asserting it
grep "% 8" ring_wait/src/lib.rs ring_wait/tests/*.rs

# the sawtooth's arithmetic
python3 -c "
n = 1024
print( 'total hints:', sum( ( a % 8 ) + 1 for a in range( n ) ) )
print( 'attempts 1000..1007:', [ ( a % 8 ) + 1 for a in range( 1000, 1008 ) ] )
"
```

Live output:

```
  Err( RingError::Empty )
  wait_until( kind, spins, || pair.may_claim() ).map_err( | _ | RingError::Full )
ring_wait/src/lib.rs:      for _ in 0..=( attempt % 8 )
total hints: 4608
attempts 1000..1007: [1, 2, 3, 4, 5, 6, 7, 8]
```

| | Value |
|--|------:|
| `RingError::` mentions in `src/lib.rs` | 2 |
| Public items that can hand a producer `Empty` | 3 of 7 |
| Public items with the correction applied | 1 — `for_space` |
| Places the correction is written out by hand elsewhere | 1 — `ring_shutdown:629` |
| `% 8` occurrences in `src/` | 1 |
| `% 8` occurrences in `tests/` | **0** |
| Hints over `DEFAULT_SPINS` | 4608 (mean 4.5/attempt) |
| Sawtooth period | 8 attempts |

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| WT5 | `ring_wait` | n/a — inconsistency | `attempt % 8` discards everything above the low three bits, so attempt 1000 and attempt 0 emit the same single hint; over `DEFAULT_SPINS` the arm emits 4608 hints in the same 8-step pattern 128 times |
| WT46 | family | n/a — inconsistency | Both production callers of `wait_until` hit the same budget-exhausted condition and name it differently — `ring_shutdown` rewrites it to `Full`, `ring_barrier` propagates `Empty` — so a caller branching on the variant gets a different answer depending on which consumer it went through |
| WT47 | `ring_wait` | n/a — unadopted | The crate's only `map_err` is the producer-side rename inside `for_space`, which nothing calls; every producer in the family that waits does so through `wait_until` and receives `Empty` for a ring with no room, so the fix was written and placed where the callers are not |
| WT48 | `ring_wait` | n/a — observation | `pause`'s rustdoc says `attempt` exists "so a strategy can behave differently early and late"; the only arm that reads it folds through `% 8`, so there is no late — over 99.2% of a default-budget wait the parameter distinguishes nothing |
