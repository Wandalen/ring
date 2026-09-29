# Pitfall: Returning the Request Instead of the Frontier

### Scope

- **Purpose**: Show why `wait_for` must return what it found rather than what was asked for, and measure how much of the test suite would notice if it did not.
- **Responsibility**: State the mutation, walk it through all six call sites, and explain why the structural check exists on top of the behavioural test.
- **In Scope**: `wait_for`'s return value.
- **Out of Scope**: The second read that produces it — see [`algorithm/002`](../algorithm/002_wait_for_asks_twice.md).

### The Mutation

```rust
// ring_barrier/src/lib.rs:282-287 — as written
ring_wait::wait_until( kind, spins, || self.admits( from, count ) )?;
self.frontier().ok_or( RingError::Empty )
```

```rust
// the mistake
ring_wait::wait_until( kind, spins, || self.admits( from, count ) )?;
Ok( Seq( from.0 + count ) )
```

It type-checks. It removes an allocation. It removes the only `Option` handling
in the method. It reads like a simplification of a redundant second read — and
manual check B5 puts it exactly:

> A consumer that waited for one item and found six should drain six. Returning
> `from + count` instead type-checks, satisfies any test that asks for exactly
> what is available, and throws away every batch that waiting discovered — which
> shows up as a throughput result, not a failure.
>
> — `tests/manual/readme.md` § B5

### One Call Site in Six Notices

Every `wait_for` in the suite, run against the mutant:

| Site | Call | Real | `from + count` | Caught |
|------|------|------|----------------|:------:|
| `:269` | `over( &[] ).wait_for( ZERO, 1, None, 4 )` | `Err( Empty )` | `Err( Empty )` — the predicate fails first, so the return is never reached | ✘ |
| `:311` | `over( deps_at( &[ 6 ] ) ).wait_for( ZERO, 1, None, 1 )` | `Ok( Seq( 6 ) )` | `Ok( Seq( 1 ) )` | **✔** |
| `:322` | `wait_for( ZERO, 2, Spin, 8 )` on `deps_at( &[ 2 ] )` | `Ok( Seq( 2 ) )` | `Ok( Seq( 2 ) )` | ✘ |
| `:323` | `wait_for( ZERO, 3, Spin, 8 )` on the same | `Err( Empty )` | `Err( Empty )` | ✘ |
| `:333` | `wait_for( ZERO, 1, None, usize::MAX ).is_err()` | `Err` | `Err` | ✘ |
| `:361` | the 512-item drain loop | a batch per wait | `position + 1` per wait | ✘ |

One test out of six, and it catches the mutation for one reason only: `:311`
seeds the dependency at **6** and asks for **1**. Every other satisfied call in
the suite asks for exactly what is there, where `from + count` and the frontier
are the same number.

The one that catches it says so in its own name and comment:

```rust
// tests/barrier_test.rs:305-314
fn wait_for_returns_the_frontier_and_not_the_requested_count()
{
  // A consumer that waited for one and found six should drain six.
  let deps = deps_at( &[ 6 ] );
  assert_eq!(
    Barrier::over( &deps ).wait_for( Seq::ZERO, 1, WaitKind::None, 1 ),
    Ok( Seq( 6 ) )
  );
}
```

### The Concurrent Test Cannot Catch It At All

```rust
// tests/barrier_test.rs:359-368
while position.0 < TOTAL
{
  if let Ok( frontier ) = barrier.wait_for( position, 1, WaitKind::Yield, 10_000 )
  {
    assert!( frontier.0 <= TOTAL, "read past what was ever published" );
    position = frontier;
  }
}
assert_eq!( position, Seq( TOTAL ) );
```

Under the mutant, `frontier` is `position + 1` every time. Then:

| Assertion | Under the mutant |
|-----------|------------------|
| `frontier.0 <= TOTAL` | **trivially true** — `position + 1` can never overshoot, so the guard is checking nothing |
| `position == Seq( TOTAL )` | still true — the loop advances by 1 and stops exactly at `TOTAL` |
| Iterations | 512 instead of however many batches the producer's head start allows |

The test passes, more slowly, with its safety assertion silently degraded into a
tautology. That is precisely B5's "shows up as a throughput result, not a
failure", and it is the reason the concurrent test — the most expensive test in
the crate — is not evidence for this property.

### So the Check Is Structural, Not Behavioural

```sh
cd "$(git rev-parse --show-toplevel)"
grep -vE "^[[:space:]]*//" ring_barrier/src/lib.rs \
  | grep -A 5 "pub fn wait_for"
```

Live output:

```
  pub fn wait_for( &self, from : Seq, count : u64, kind : WaitKind, spins : usize )
  -> Result< Seq, RingError >
  {
    ring_wait::wait_until( kind, spins, || self.admits( from, count ) )?;
    self.frontier().ok_or( RingError::Empty )
  }
```

**Expected:** the body waits and then returns `self.frontier()`, with no
arithmetic on `from` or `count` anywhere in it.

`-A 5` rather than `-A 3`, and B5 says why: the signature spans two lines, so a
three-line window stops at the `wait_until` call and never reaches the return —
a check that cannot see the line its expected result is about. Worth noting as a
general shape: a grep-based structural check is only as good as its window, and
this crate's two-line signatures are exactly the case where the obvious window
is one line short.

### Why the Frontier Is the Right Answer

| | Returns | A consumer then drains |
|--|---------|------------------------|
| `from + count` | what was asked for | `count` — the batch the wait discovered is discarded and re-waited for |
| **`self.frontier()`** | what was found | everything available — one wait, one batch |

`ring_consume` is built on the second reading: `Consumer::available()` returns a
run from the current position to the frontier, and `available_up_to( max )`
exists to *clamp* it — a method that only makes sense if the unclamped answer is
routinely bigger than any one request
([`lifecycle/002`](../lifecycle/002_a_consumer_draining_behind_a_producer.md)).

The cost of being right is one extra read, which is one extra allocation
([`non_functional_requirement/001`](../non_functional_requirement/001_every_frontier_read_allocates_nothing.md)
§ BR13) — the `+ 1` in that instance's ledger is this pitfall's price.

### BR47 — The Pitfall Is Guarded by a Test Whose Name Is the Whole Specification

`wait_for_returns_the_frontier_and_not_the_requested_count` is the only defence
against a change that would compile, pass every other test, and silently cap
every batch at the size its consumer asked for. Nothing in the type system
distinguishes the two: both are `Seq`, both are plausible, and the wrong one is
the one a reader implementing `wait_for` from its signature would reach for
first.

The rustdoc argues the point in three lines and the test asserts it in one. What
neither does is make the wrong version fail to build — and the correct version
is a single expression, `self.frontier()`, that a refactor toward *"return what
was asked for"* would replace without a second thought.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A7 -F '  /// Wait until at least `count` sequences are readable from `from`, then' ring_barrier/src/lib.rs
grep -A6 "fn wait_for_returns_the_frontier_and_not_the_requested_count" \
  ring_barrier/tests/barrier_test.rs
```

Live output:

```
  /// Wait until at least `count` sequences are readable from `from`, then
  /// report the frontier.
  ///
  /// The returned sequence is the frontier as re-read immediately after the
  /// wait succeeded, not the frontier at the exact instant it succeeded — a
  /// dependency may have advanced between the two reads, so the value is
  /// only guaranteed to be at least as far as what was checked. It is also
  /// not `from + count` — a consumer that waited for one item and found six
fn wait_for_returns_the_frontier_and_not_the_requested_count()
{
  // A consumer that waited for one and found six should drain six.
  let deps = deps_at( &[ 6 ] );
  assert_eq!(
    Barrier::over( &deps ).wait_for( Seq::ZERO, 1, WaitKind::None, 1 ),
    Ok( Seq( 6 ) )
```

### Pitfalls

| File | Relationship |
|------|--------------|
| [001_the_two_empty_answers_look_like_a_bug.md](001_the_two_empty_answers_look_like_a_bug.md) | The other `wait_for` ending, and the one nothing guards |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_wait_for_asks_twice.md](../algorithm/002_wait_for_asks_twice.md) | Both phases, and why the second read is not redundant |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_nine_methods_over_one_borrowed_slice.md](../api/001_nine_methods_over_one_borrowed_slice.md) | The only method returning a `Result` |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_a_consumer_draining_behind_a_producer.md](../lifecycle/002_a_consumer_draining_behind_a_producer.md) | The drain that depends on getting a batch back |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_every_frontier_read_allocates_nothing.md](../non_functional_requirement/001_every_frontier_read_allocates_nothing.md) | What the second read costs |

### Workarounds

| File | Relationship |
|------|--------------|
| [../workaround/001_the_check_that_capacity_stays_out.md](../workaround/001_the_check_that_capacity_stays_out.md) | The family's other structural grep, and its own blind spot |

### Sources

| File | Relationship |
|------|--------------|
| `ring_barrier/src/lib.rs:282-287` | The body, and the return |
| `ring_consume/src/lib.rs:336,365` | `available` and `available_up_to` |
| `tests/manual/readme.md` § B5 | The check, the window, and the reason |

### Tests

| File | Relationship |
|------|--------------|
| `tests/barrier_test.rs:305-314` | The one test that distinguishes |
| `tests/barrier_test.rs:316-324` | Two calls, neither distinguishing |
| `tests/barrier_test.rs:341-370` | The drain loop whose guard the mutant makes vacuous |
