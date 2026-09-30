# Pitfall: The Two Empty Answers Look Like a Bug

### Scope

- **Purpose**: Warn against "fixing" the empty-set answer to match `ring_gating`'s, and give the three places the word *empty* means something different in this crate.
- **Responsibility**: Show each disagreement, why it is deliberate, what guards it, and the one instance that is guarded by nothing.
- **In Scope**: What an empty barrier answers, and what an empty answer means.
- **Out of Scope**: Why `0` rather than another default — see [`decisions/001`](../decisions/001_zero_for_a_barrier_over_nothing.md).

### The Pitfall

Two crates, the same cursors, the same-shaped question, opposite answers:

```rust
// tests/barrier_test.rs:255-262
let empty = GatingSet::new( cap( 8 ), 0 );

assert_eq!( empty.headroom( Seq( 8 ) ), 8, "a producer with nobody behind it may write" );
assert_eq!(
  Barrier::over( empty.cursors() ).available( Seq::ZERO ),
  0,
  "a consumer with nobody ahead of it may not read"
);
```

An empty set means **unbounded** to the producer and **nothing** to the
consumer. Read side by side that looks like one of them is wrong, and the
correction is a one-token edit either way — `map_or( 0, … )` becomes
`map_or( u64::MAX, … )`, or `headroom` starts returning zero. Both compile.
Both are wrong.

They are not two answers to one question. They are one answer — *no constraint
from dependencies* — resolved against what a dependency-free participant
actually has. A producer nobody is reading behind may write the whole ring. A
consumer nobody is publishing ahead of has nothing to read. The symmetry is in
the reasoning, not in the number.

### Why the Test Exists Rather Than a Comment

The assertion is in the test suite specifically because prose does not fail:

> Stated as an assertion because it reads like an inconsistency: the same empty
> set means "unbounded" to a producer and "nothing readable" to a consumer.
> Both are "no constraint from dependencies" resolved to what a dependency-free
> participant actually has.
>
> — `tests/barrier_test.rs:251-254`

And it is why `ring_gating` is a dev-dependency at all
([`workaround/002`](../workaround/002_ring_gating_as_a_dev_dependency.md)) —
asserting the relationship against a hand-rolled stand-in would be asserting it
against nothing.

### Empty, Three Ways

| Reading | `Barrier::over( &[] )` | `Barrier::over( &deps_zeroed( 1 ) )` | Distinguishable? |
|---------|------------------------|--------------------------------------|:----------------:|
| `is_empty()` | `true` | `false` | ✔ |
| `frontier()` | `None` | `Some( Seq::ZERO )` | ✔ |
| `available( ZERO )` | `0` | `0` | **✘** |
| `admits( ZERO, 1 )` | `false` | `false` | **✘** |

`frontier` is the only reading that keeps *no dependencies* apart from
*dependencies, all at zero*. That is what the `Option` is for, and it is why
manual check B3 greps for the token that would collapse it:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -vE "^[[:space:]]*//" ring_barrier/src/lib.rs \
  | grep -E "map_or|ok_or|unwrap_or"
```

Live output:

```
        self.frontier().map_or(0, |frontier| from.distance_to(frontier))
        self.frontier().ok_or(RingError::Empty)
```

**Expected: exactly two hits** — `map_or( 0, … )` in `available` and
`ok_or( RingError::Empty )` in `wait_for`. An `unwrap_or( Seq::ZERO )` anywhere
in the file would type-check, read as a tidy simplification, and silently merge
the two cases the `Option` exists to separate.

Today it returns exactly those two:

```
    self.frontier().map_or( 0, | frontier | from.distance_to( frontier ) )
    self.frontier().ok_or( RingError::Empty )
```

### BR6 — The One Empty Answer Nothing Guards

The two phases of `wait_for` disagree with each other on exactly one input
class, and both phases are this crate's own:

| Call | Result | Because |
|------|--------|---------|
| `Barrier::over( &[] ).admits( Seq::ZERO, 0 )` | **`true`** | `0 <= available` is `0 <= 0` |
| `Barrier::over( &[] ).wait_for( Seq::ZERO, 0, None, 1 )` | **`Err( Empty )`** | the predicate passes; then `frontier()` is `None`, and `ok_or` fires |

So a caller who asks *may I take zero?* is told yes, and a caller who asks *let
me take zero* is refused. Both are defensible in isolation —
`admits( …, 0 )` is vacuously true, and there is no frontier to return — and
nothing in the crate reconciles them.

| | |
|--|--|
| Tested | **No.** `count = 0` appears at no `wait_for` call site |
| Documented | **No.** Neither method's doc comment mentions a zero count |
| Reachable in production | No — `wait_for` has no `src/` caller at all ([`pattern/002`](../pattern/002_the_quantity_the_predicate_and_the_wait.md) § BR20) |
| Guarded by B3 | No — B3 checks the *tokens*, and both tokens are the correct ones |

This is the honest shape of the finding: an unreachable inconsistency, in a
method nothing calls, that would become a real disagreement the first time a
caller passes a computed `count` that happens to be zero. Worth recording, not
worth an urgent fix — and the recording is the point, because the next reader to
notice it will otherwise "fix" one of the two and break the other's test.

### What to Do Instead

| Tempted to | Don't, because |
|------------|----------------|
| Make `available` return `u64::MAX` on empty | It would let a consumer read a ring nobody has published into — [`decisions/001`](../decisions/001_zero_for_a_barrier_over_nothing.md) |
| Replace `frontier`'s `Option` with `unwrap_or( Seq::ZERO )` | It merges *no dependencies* with *all at zero*; B3 exists to catch exactly this |
| Align `headroom` and `available` on one convention | They are opposite ends of the same ring; `an_empty_barrier_and_an_empty_gating_set_answer_oppositely` fails |
| "Fix" `admits( …, 0 )` to `false` for consistency with `wait_for` | `admits`'s contract is *may I take `count`*, and taking zero always may — fix `wait_for`'s ending instead, if either |

### BR46 — The Test Asserting the Two Halves Disagree Needs a Dev-Dependency to Say So

`an_empty_barrier_and_an_empty_gating_set_answer_oppositely` is the assertion
that makes the asymmetry deliberate rather than accidental, and it can only be
written because `ring_gating` is a `[dev-dependencies]` entry of this crate.

A property of two crates has to be asserted from inside one of them, and the
edge that permits it exists only in test builds. Remove the dev-dependency and
the code still compiles, the library is unchanged, and the one statement that
the asymmetry is intentional stops being checkable — which BR14 measures from
the other direction, by simulating exactly that removal.

```sh
cd "$(git rev-parse --show-toplevel)"
awk '/^\[dev-dependencies\]/{f=1;next} /^\[/{f=0} f' ring_barrier/Cargo.toml \
  | grep -oE '^ring_[a-z_]+'
grep -A5 "fn an_empty_barrier_and_an_empty_gating_set_answer_oppositely" \
  ring_barrier/tests/barrier_test.rs
```

Live output:

```
ring_gating
fn an_empty_barrier_and_an_empty_gating_set_answer_oppositely()
{
  // Stated as an assertion because it reads like an inconsistency: the same
  // empty set means "unbounded" to a producer and "nothing readable" to a
  // consumer. Both are "no constraint from dependencies" resolved to what a
  // dependency-free participant actually has.
```

### Pitfalls

| File | Relationship |
|------|--------------|
| [002_returning_the_request_instead_of_the_frontier.md](002_returning_the_request_instead_of_the_frontier.md) | The other `wait_for` mistake, and how much of the suite sees it |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_wait_for_asks_twice.md](../algorithm/002_wait_for_asks_twice.md) | The two phases, and the third ending BR6 lives in |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_zero_for_a_barrier_over_nothing.md](../decisions/001_zero_for_a_barrier_over_nothing.md) | The default, with its alternatives priced |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_capacity_never_enters_the_arithmetic.md](../invariant/002_capacity_never_enters_the_arithmetic.md) | The other thing `ring_gating` has and this crate does not |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_three_barrier_readings.md](../item/001_the_three_barrier_readings.md) | The three readings the table above distinguishes |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_the_quantity_the_predicate_and_the_wait.md](../pattern/002_the_quantity_the_predicate_and_the_wait.md) | BR20 — why the inconsistency is unreachable today |

### Workarounds

| File | Relationship |
|------|--------------|
| [../workaround/002_ring_gating_as_a_dev_dependency.md](../workaround/002_ring_gating_as_a_dev_dependency.md) | Why the cross-crate assertion can be written at all |

### Sources

| File | Relationship |
|------|--------------|
| `ring_barrier/src/lib.rs:191-194,216-219,282-287` | `frontier`, `available`, `wait_for` |
| `ring_gating/src/lib.rs:222` | `headroom`, the opposite answer |
| `tests/manual/readme.md` § B3 | The grep, and what it expects |

### Tests

| File | Relationship |
|------|--------------|
| `tests/barrier_test.rs:234-246` | `None` frontier, zero available, four `from` values |
| `tests/barrier_test.rs:248-263` | The two crates asserted together |
| `tests/barrier_test.rs:265-272` | Waiting on an empty barrier fails rather than hanging |
