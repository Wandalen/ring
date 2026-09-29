# invariant

Two properties, one per side of the cursor. The write side: the frontier moves at
one place, from its exact current value, forward only. The read side:
`is_published( seq )` is exactly `seq < published()`, exclusive at the boundary.

The second depends on the first. A frontier that could be assigned, or advanced
from wherever it happened to be, would make `seq < published()` an approximation
rather than a complete answer — so the read-side invariant is only as strong as
the grep that keeps `store` and `fetch_add` out of the crate.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [The Frontier Moves Only By Compare-Exchange](001_the_frontier_moves_only_by_compare_exchange.md) | PB19 — the invariant in three parts, the four `SeqCell` methods and what each of the other three would break, the grep that enforces it, and the hole `cursor()` leaves open |
| 002 | [`is_published` Is Exclusive of the Frontier](002_is_published_is_exclusive_of_the_frontier.md) | PB20 — 320 assertions over a moving boundary, why exclusive is forced rather than chosen, and the one extra sequence an inclusive reading would publish |

### Write Side, in Three Parts

| Part | Statement | Enforced by |
|------|-----------|-------------|
| One site | the cursor is written at `src/lib.rs:164` and nowhere else | `§ P2`, a grep expecting exactly one hit |
| From the exact current value | the exchange's `current` is the caller's `start` | the operation's own shape |
| Forward only | `end = start.advanced_by( len )` and `len : usize` | falls out of `advanced_by`'s signature |

Part 3 is a consequence, not a separate rule. Together they give the property
everything else rests on: the cursor's value sequence over the ring's whole life
is exactly the sequence of publication endpoints, in order, with no gaps and no
repeats.

### What the Other Three `SeqCell` Methods Would Do

| Method | Breaks | Symptom |
|--------|--------|---------|
| `store( value, order )` | parts 1, 2 and 3 | the frontier retreats; sequences already handed to a consumer are re-issued as duplicates |
| `fetch_add( n, order )` | parts 1 and 2 | two producers each advance by their own `len`; the frontier passes a range nobody published |
| `load( order )` | — | read-only; this is what `published()` is |

Nothing type-level prevents the first two — the field is in scope and the trait is
public. The invariant holds because the crate does not call them, which is a
property of *what code exists* and therefore checkable only by reading the code.

### Read Side, at the Boundary

| Frontier | Ask about | Correct | Inclusive reading |
|---------:|----------:|:-------:|:-----------------:|
| 3 | 2 | `true` | `true` |
| 3 | **3** | **`false`** | **`true`** |
| 3 | 4 | `false` | `false` |
| 0 | 0 | `false` | `true` |

Two rows differ, and both are the same failure: handing a consumer a sequence a
producer has claimed and may be writing right now. That is this crate's central
requirement, and the doc example states the middle row in four words —
*"claimed, perhaps, but not published"*.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# the one mutation site — expect exactly one hit
grep -vE "^[[:space:]]*//" ring_publish/src/lib.rs \
  | grep -nE "fetch_add|compare_exchange|\.store\("

# the four methods three of which could break it
command grep -m1 -A20 -F 'pub trait SeqCell' ring_atomic/src/lib.rs

# the read-side comparison, and the contract it implements
sed -n '/^  \/\/\/ How far publication has reached — one past the last readable sequence\.$/,/^  }$/p;/^  \/\/\/ Whether `seq` has been published and is therefore readable\.$/,/^  }$/p' ring_publish/src/lib.rs

# the exhaustive boundary test and its reach
command grep -m1 -A25 -F 'fn nothing_is_published_before_anything_is()' ring_publish/tests/publish_test.rs | tail -n 25
```

| | Value |
|--|------:|
| Sites writing the cursor | **1** |
| Operations that write it | 1 — `compare_exchange` |
| `SeqCell` methods available | 4 |
| …that could move the cursor | 3 |
| …that could move it *anywhere* | 2 |
| …type-level prevented | **0** |
| Expected hits from `§ P2` | exactly 1 |
| Boundary points asserted at a fixed frontier | 5 |
| Frontiers × candidates in the exhaustive test | 16 × 20 = **320** |
| Atomic loads that test issues | 320 |
| Compare-exchanges it issues | 16 |
| Sequences an inclusive reading would wrongly publish | **1** — the frontier itself |
| Tests that a single off-by-one would fail | both |
| …that a stale cached frontier would fail | 1 — the exhaustive one |

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| PB19 | `ring_publish` | n/a — observation | The invariant is enforced by absence and checked by grep, because a `store`-based `publish` would pass every behavioural test in the suite under a single producer and most of them under several |
| PB20 | `ring_publish` | n/a — observation | The boundary is asserted exhaustively twice, and both tests are kept because they fail differently: one names the boundary in its message, the other localises an arbitrary inconsistency with `"at frontier N, asking about M"` |
| PB50 | `ring_publish` | n/a — observation | `Acquire` appears twice in `src/lib.rs` and both are doc lines, so the crate that calls the Release/Acquire pairing *the entire happens-before edge* supplies one half and can only describe the other |
| PB51 | `ring_publish` | n/a — coverage | `is_published` — the question this crate is graded on — has thirteen call sites, all in a file gated `#![ cfg( not( loom ) ) ]`, so it is exercised where interleavings are sampled and never where they are enumerated |
