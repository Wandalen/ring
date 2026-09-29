# Algorithm: `wait_for` Asks Twice

### Scope

- **Purpose**: Show that `wait_for` is a predicate loop followed by a second, independent read, and work out what that second read buys and what it costs.
- **Responsibility**: Give the two phases, the three ways they can end, the one input class where they disagree, and the information discarded between them.
- **In Scope**: The two-line body of `wait_for`.
- **Out of Scope**: The chain each read descends — see [`001`](001_the_frontier_in_two_delegations.md).

### The Body

```rust
// ring_barrier/src/lib.rs:282-287
pub fn wait_for( &self, from : Seq, count : u64, kind : WaitKind, spins : usize )
-> Result< Seq, RingError >
{
  ring_wait::wait_until( kind, spins, || self.admits( from, count ) )?;
  self.frontier().ok_or( RingError::Empty )
}
```

Two statements, and they do not share a value. The first waits until a predicate
holds; the second re-reads the cursors from scratch and reports what it finds.

### Why the Second Read Is Not the First One's Answer

The crate states the intent:

> The returned sequence is the frontier at the moment the wait succeeded, not
> `from + count` — a consumer that waited for one item and found six should drain
> six, and returning the requested count instead would throw away the batch that
> waiting just discovered.
>
> — `ring_barrier/src/lib.rs:246-249`

The doc argues against `from + count`. The mechanism it uses to avoid it is
sharper than that, and worth stating on its own: **the frontier reported is not
even the one that satisfied the predicate.** Between the successful `admits` and
the `frontier()` call, dependencies keep advancing, so the returned value is at
least what the wait found and possibly more.

| | Value | Relationship |
|--|-------|--------------|
| What the predicate accepted | `available ≥ count` at some instant during the loop | ≥ `count` |
| What is returned | `frontier()` read afterwards | ≥ the above |
| What `from + count` would be | The request | ≤ the above |

That ordering is monotone because a frontier is a minimum over cursors that only
advance, so the second read can never come back *behind* the first. The
staleness runs in the safe direction — the caller is told about less progress
than exists, never more. Compare
[`invariant/001`](../invariant/001_the_frontier_never_exceeds_a_dependency.md).

### The Three Endings

| Ending | Returns | Reached when |
|--------|---------|--------------|
| Budget exhausted | `Err( RingError::Empty )` | `wait_until` ran `spins` attempts without the predicate holding |
| Wait succeeded, frontier present | `Ok( frontier )` | The normal path |
| Wait succeeded, frontier absent | `Err( RingError::Empty )` | **Only one input class — see below** |

The first and third are indistinguishable to the caller: both are
`RingError::Empty`, with nothing saying whether the budget ran out or the barrier
had no dependencies at all.

### The One Input Class Where the Two Phases Disagree — [BR6](../pitfall/001_the_two_empty_answers_look_like_a_bug.md)

Recorded as a finding in
[`pitfall/001`](../pitfall/001_the_two_empty_answers_look_like_a_bug.md), which
is BR6's home; this section is the same disagreement read from the two-phase
side, and adds the probe below rather than a second finding.

`admits( from, 0 )` is `0 <= available( from )`, which is true for every barrier
including an empty one. So a zero-count wait on an empty barrier passes phase one
and fails phase two:

```rust
// verified against the real crate, not derived from the source
Barrier::over( &[] ).admits( Seq::ZERO, 0 )                        // true
Barrier::over( &[] ).wait_for( Seq::ZERO, 0, WaitKind::None, 1 )   // Err( Empty )
Barrier::over( &[] ).wait_for( Seq::ZERO, 1, WaitKind::None, 1 )   // Err( Empty )

let deps = [ PaddedCursor::default() ];  deps[ 0 ].store( Seq( 6 ), Ordering::Release );
Barrier::over( &deps ).wait_for( Seq::ZERO, 0, WaitKind::None, 1 ) // Ok( Seq( 6 ) )
```

**This is the only combination where `wait_until` returns `Ok` and `wait_for`
returns `Err`.** Any `count > 0` fails the predicate on an empty barrier, so
phase one refuses first and phase two is never reached.

| Is it wrong? | No |
|--------------|----|
| Is it surprising? | Yes — the wait succeeded and the call still failed |
| Is it tested? | **No.** `a_request_for_zero_is_always_admitted` covers `admits( ZERO, 0 )` on an empty barrier; `waiting_on_an_empty_barrier_fails_rather_than_hanging` covers `wait_for` with `count = 1`. The intersection of the two is untested |
| Is it documented? | No — `wait_for`'s `# Errors` section names only the budget case |

The behaviour is defensible: a caller asking to wait for nothing, from a barrier
with nothing behind it, has no frontier to be told about, and `Err` is more
honest than an invented `Seq`. What is missing is the assertion that pins it, and
the one-line addition to `# Errors` that says so. Recorded rather than applied; a
test change belongs to a run with its own verification.

### BR5 — The Discarded Attempt Count

`ring_wait::wait_until` returns `Ok( attempt )` — how many attempts it took
before the predicate held. `wait_for` drops it with `?`.

```sh
cd "$(git rev-parse --show-toplevel)"
for f in ring_*/src/*.rs; do
  grep -vE "^[[:space:]]*(///|//!)" "$f" | grep 'wait_until' | sed "s|^|$f:|"
done
```

Live output:

```
ring_barrier/src/lib.rs:    ring_wait::wait_until( kind, spins, || self.admits( from, count ) )?;
ring_shutdown/src/lib.rs:    // `budget.max( 1 )` matches `ring_wait::wait_until`'s reading of its own
ring_shutdown/src/lib.rs:  ring_wait::wait_until( kind, spins, || shutdown.is_closed() )
ring_shutdown/src/lib.rs:  let outcome = ring_wait::wait_until( kind, spins, ||
ring_wait/src/lib.rs:pub fn wait_until< F >( kind : WaitKind, spins : usize, mut ready : F ) -> Result< usize, RingError >
ring_wait/src/lib.rs:  wait_until( kind, DEFAULT_SPINS, ready )
ring_wait/src/lib.rs:  wait_until( kind, spins, || pair.may_claim() ).map_err( | _ | RingError::Full )
ring_wait/src/lib.rs:  wait_until( kind, spins, || pair.pending() >= count )
```

| Caller | Attempt count |
|--------|---------------|
| `ring_wait::wait` (own wrapper) | Propagated |
| `ring_wait::for_space` | Propagated |
| `ring_wait::for_data` | Propagated |
| `ring_shutdown` (two sites) | One propagated, one bound to `outcome` |
| **`Barrier::wait_for`** | **Discarded** |

Every other caller in the family keeps it, and this is the one that does not.
The count is exactly the diagnostic a caller tuning a `spins` budget needs — *did
the wait return on the first look, or on the 9,999th?* — and the signature has
nowhere to put it, because the return slot is spent on the frontier.

That is a real tension rather than an oversight: a `Result< ( Seq, usize ), … >`
would carry both and complicate every call site, and no caller has asked. It is
recorded because the information exists, is free, and is thrown away at exactly
the boundary where a consumer would want it.

### The Cost of Asking Twice

Each phase descends the full chain in [`001`](001_the_frontier_in_two_delegations.md),
so each performs one allocation:

| Call | Allocations |
|------|------------:|
| `wait_for( …, spins = 1 )`, predicate holds immediately | 2 |
| `wait_for( …, spins = n )`, predicate holds on the last attempt | n + 1 |
| `wait_for( …, spins = n )`, budget exhausted | n |

The `+ 1` is the second read. A `WaitKind::Yield` wait with a 10,000-spin budget
— the one in `a_consumer_waiting_on_a_producer_thread_makes_progress` — can
therefore allocate ten thousand times in a single call, on a path whose whole
purpose is to block cheaply.

### BR24 — One Error Variant Reached by Two Conditions That Are Not the Same Failure

`wait_for` produces `RingError::Empty` twice, four lines apart, for conditions
that share no cause. The `?` on line 285 propagates it when the budget ran out
with fewer than `count` sequences available. The `ok_or` on line 286 produces it
when the barrier has no dependencies at all.

The first is transient and says *wait longer*. The second is structural and says
*this barrier will never admit anything*, because a barrier over an empty slice
folds to `None` on every future call as well. A caller that retries on `Empty` —
which is what the variant's own documentation invites, *"which for a consumer
means exactly what it says"* — will spin forever on the second.

The crate's own tests separate the two cases and assert them independently, so
the distinction is known; it is the return type that cannot carry it.

```sh
cd "$(git rev-parse --show-toplevel)"
grep "RingError::Empty" ring_barrier/src/lib.rs
# both cases are tested, under names that distinguish them
grep "fn waiting_on_an_empty_barrier\|fn wait_for_gives_up" ring_barrier/tests/barrier_test.rs
```

Live output:

```
  /// [`RingError::Empty`] when the `spins` budget runs out with fewer than
  /// fabricated rule. [`RingError::Empty`] is the honest answer.
    self.frontier().ok_or( RingError::Empty )
fn waiting_on_an_empty_barrier_fails_rather_than_hanging()
fn wait_for_gives_up_with_empty_when_the_budget_runs_out()
```

### Algorithms

| File | Relationship |
|------|--------------|
| [001_the_frontier_in_two_delegations.md](001_the_frontier_in_two_delegations.md) | The chain each of the two phases descends |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_nine_methods_over_one_borrowed_slice.md](../api/001_nine_methods_over_one_borrowed_slice.md) | The one method that is not `#[ must_use ]`, and why |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_the_frontier_never_exceeds_a_dependency.md](../invariant/001_the_frontier_never_exceeds_a_dependency.md) | Why the second read can only be further ahead |

### Items

| File | Relationship |
|------|--------------|
| [../item/002_the_five_accessors_and_the_wait.md](../item/002_the_five_accessors_and_the_wait.md) | `wait_for`'s contract and coverage |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_every_frontier_read_allocates_nothing.md](../non_functional_requirement/001_every_frontier_read_allocates_nothing.md) | Where the per-spin allocation lands |
| [../non_functional_requirement/002_a_non_blocking_wait_must_look_exactly_once.md](../non_functional_requirement/002_a_non_blocking_wait_must_look_exactly_once.md) | The contract phase one must honour |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_returning_the_request_instead_of_the_frontier.md](../pitfall/002_returning_the_request_instead_of_the_frontier.md) | What the second read exists to prevent |

### Sources

| File | Relationship |
|------|--------------|
| `ring_barrier/src/lib.rs:243-287` | `wait_for`, doc and body |
| `ring_wait/src/lib.rs:179-195` | `wait_until`, and the attempt count it returns |
| `ring_types/src/policy.rs:22-34` | The four `WaitKind`s phase one dispatches on |

### Tests

| File | Relationship |
|------|--------------|
| `tests/barrier_test.rs:305-314` | The second read returning six for a request of one |
| `tests/barrier_test.rs:316-324` | The budget-exhausted ending |
| `tests/barrier_test.rs:265-272` | The empty-barrier ending, at `count = 1` |
| `tests/barrier_test.rs:326-339` | Phase one's non-blocking contract |
