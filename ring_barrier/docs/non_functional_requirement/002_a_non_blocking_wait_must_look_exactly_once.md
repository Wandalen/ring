# NFR: A Non-Blocking Wait Must Look Exactly Once

### Scope

- **Purpose**: State the liveness requirement `WaitKind::None` places on `wait_for`, show what enforces it, and price it.
- **Responsibility**: Give the requirement, the mechanism one crate down, the measurement, and the test whose failure mode is a hang.
- **In Scope**: Bounded-time return, and what the `spins` budget does and does not control.
- **Out of Scope**: The safety property under concurrency — see [`invariant/001`](../invariant/001_the_frontier_never_exceeds_a_dependency.md).

### The Requirement

`ring_types` calls out one variant specifically:

> Return immediately with whatever is available, possibly nothing. **The only
> variant reachable from inside a tick.**
>
> — `ring_types/src/policy.rs:32-34`

A tick must never block, so `wait_for( …, WaitKind::None, spins )` must return in
bounded time **for every `spins`, including `usize::MAX`**. The budget is not
what stops it; the wait kind is.

`WaitKind::is_non_blocking` is the family's name for this, and exactly one
variant satisfies it:

```rust
// ring_types/src/policy.rs:81-88
pub const fn is_non_blocking( self ) -> bool
{
  match self
  {
    Self::None => true,
    Self::Spin | Self::Yield | Self::Park => false,
  }
}
```

### What Enforces It

The mechanism is a `bool` return one crate down:

```rust
// ring_wait/src/lib.rs:179-195
for attempt in 0..spins.max( 1 )
{
  if ready() { return Ok( attempt ); }
  if !pause( kind, attempt ) { break; }     // <- WaitKind::None returns false here
}
Err( RingError::Empty )
```

```rust
// ring_wait/src/lib.rs:144 — inside pause
WaitKind::None => false,
```

So the loop runs `ready()` once, calls `pause`, gets `false`, and breaks — no
matter what `spins` says. This crate contributes nothing to the guarantee except
not getting in its way: `wait_for` passes `kind` straight through and adds no
loop of its own ([`lifecycle/001`](../lifecycle/001_a_barrier_from_over_to_the_end_of_a_borrow.md)
— zero loops in the whole crate).

### Measured

Same counting allocator as
[`001`](001_every_frontier_read_allocates_nothing.md), release build:

| Call | Allocations | Elapsed |
|------|------------:|--------:|
| `wait_for( ZERO, 1, None, usize::MAX )`, stalled barrier | **1** | **200 ns** |
| `wait_for( ZERO, 1, None, usize::MAX )`, satisfied barrier | 2 | — |
| `wait_for( ZERO, 1, Spin, 10_000 )`, stalled barrier | 10 000 | — |

One look, one allocation, two hundred nanoseconds — against a nominal budget of
`usize::MAX`. The refusal path is the cheapest thing in the crate, and the
satisfied path costs exactly one more because of the second read
([`algorithm/002`](../algorithm/002_wait_for_asks_twice.md)).

That last row is the contrast the requirement exists for: change one argument
and the same call becomes ten thousand allocations. Nothing in the signature
distinguishes them — `kind` is a plain enum parameter — so a caller inside a tick
is one wrong `WaitKind` away from blocking a path that must not block.

### The Test Whose Failure Mode Is a Hang

```rust
// tests/barrier_test.rs:332-338
let started = std::time::Instant::now();
assert!( barrier.wait_for( Seq::ZERO, 1, WaitKind::None, usize::MAX ).is_err() );

// The budget is `usize::MAX`; only the non-blocking contract stops this.
assert!( started.elapsed() < std::time::Duration::from_secs( 1 ) );
```

If `pause` ever returned `true` for `None`, this does not fail — it runs
`usize::MAX` iterations, which at 200 ns per look is roughly **117 000 years.**
The elapsed-time assertion is never reached.

That is the honest reading, and it is worth stating rather than treating the
second `assert!` as the guard it looks like:

| | |
|--|--|
| What the timing assertion catches | A wait kind that pauses *briefly* but does not break — e.g. `None` accidentally routed to `Spin` |
| What it does not catch | The loop running to `usize::MAX` — the suite hangs first |
| Why the bound is one second, not one millisecond | The comment says it: a tight bound is a flaky test; anything under a second already proves the loop did not run |

A tighter guarantee would need a smaller sentinel budget and an assertion on the
*attempt count* — which `wait_for` discards
([`algorithm/002`](../algorithm/002_wait_for_asks_twice.md) § BR5). The
information that would make this test sharp is the information the signature
throws away.

### What the `spins` Budget Actually Controls

| `WaitKind` | Budget's effect | Bounded by |
|------------|-----------------|------------|
| `None` | **none** — always one look | the contract |
| `Spin` | attempts | `spins` × spin-hint cost |
| `Yield` | attempts | `spins` × scheduler |
| `Park` | attempts | `spins` × 50 µs — [`item/002`](../item/002_the_five_accessors_and_the_wait.md) § BR12 |

Only three of the four rows are budget-controlled, and the fourth is the one a
tick uses. A caller that reasons "`spins` bounds my latency" is right for three
kinds and wrong about which one matters — for `None` the latency is one read and
the budget is dead weight in the signature.

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [001_every_frontier_read_allocates_nothing.md](001_every_frontier_read_allocates_nothing.md) | The per-look cost this multiplies |

### BR43 — The One-Look Guarantee Is Asserted Here and Implemented Two Crates Away

`a_non_blocking_wait_looks_exactly_once` counts predicate invocations under
`WaitKind::None` and asserts one. The behaviour it measures is not in this
crate: it is the `false` returned by `ring_wait::pause`'s `None` arm, which
breaks the loop before a second attempt.

So the crate that owns the guarantee has one test for it and the crate that
merely passes `kind` through has another, and neither references the other. If
`pause`'s arm changed, both would fail — which is the good case. If
`wait_for` stopped forwarding `kind` and hard-coded a strategy, only this test
would fail, and it would fail for a reason its name does not describe.

```sh
cd "$(git rev-parse --show-toplevel)"
grep -A6 "fn a_non_blocking_wait_looks_exactly_once" ring_barrier/tests/barrier_test.rs
# where the behaviour actually lives
grep -B2 -A2 "WaitKind::None => false" ring_wait/src/lib.rs
```

Live output:

```
fn a_non_blocking_wait_looks_exactly_once()
{
  let deps = deps_zeroed( 1 );
  let barrier = Barrier::over( &deps );

  let started = std::time::Instant::now();
  assert!( barrier.wait_for( Seq::ZERO, 1, WaitKind::None, usize::MAX ).is_err() );
      true
    }
    WaitKind::None => false,
  }
}
```

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_wait_for_asks_twice.md](../algorithm/002_wait_for_asks_twice.md) | The two phases, and the discarded attempt count |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_nine_methods_over_one_borrowed_slice.md](../api/001_nine_methods_over_one_borrowed_slice.md) | The one method that takes a `WaitKind` |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_the_dependency_that_is_not_ring_seqno.md](../integration/002_the_dependency_that_is_not_ring_seqno.md) | The dependency that supplies the contract |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_the_frontier_never_exceeds_a_dependency.md](../invariant/001_the_frontier_never_exceeds_a_dependency.md) | The safety half of correctness under concurrency |

### Items

| File | Relationship |
|------|--------------|
| [../item/002_the_five_accessors_and_the_wait.md](../item/002_the_five_accessors_and_the_wait.md) | Which `WaitKind`s reach `wait_for` at all |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_a_barrier_from_over_to_the_end_of_a_borrow.md](../lifecycle/001_a_barrier_from_over_to_the_end_of_a_borrow.md) | Zero loops in the crate |

### Sources

| File | Relationship |
|------|--------------|
| `ring_types/src/policy.rs:21-88` | The four variants and `is_non_blocking` |
| `ring_wait/src/lib.rs:179-195` | The loop and the `break` |
| `ring_wait/src/lib.rs:144` | `pause`'s one `false` arm |
| `ring_barrier/src/lib.rs:282-285` | The pass-through |

### Tests

| File | Relationship |
|------|--------------|
| `tests/barrier_test.rs:326-339` | The `usize::MAX` budget, stopped by the contract alone |
| `tests/barrier_test.rs:316-324` | `Spin` with a real budget, exhausted |
