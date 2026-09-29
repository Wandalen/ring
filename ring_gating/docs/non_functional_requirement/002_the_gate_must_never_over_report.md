# NFR: The Gate Must Never Over-Report

### Scope

- **Purpose**: State the crate's one-sided safety requirement, and establish that the test named for it cannot detect a violation of it.
- **Responsibility**: Give the requirement, show the assertion is true by construction, identify what the test does still establish, and give two assertions that would close the gap.
- **In Scope**: The concurrent correctness requirement and its coverage.
- **Out of Scope**: The sequential invariant — see [`invariant/001`](../invariant/001_the_bound_is_the_minimum_and_only_the_minimum.md).

### The Requirement

The crate's own test states it, in its comment:

> The gate is read by a producer while consumers advance. It may under-report
> headroom — a consumer that moved after the read is simply not yet seen — but
> it must never report more room than existed at the moment it looked, because
> that is the reading a producer overwrites a live slot on.

This is the right requirement and it is stated precisely. **Asymmetry is the
whole content:** an under-report costs throughput and is recovered on the next
read; an over-report corrupts a slot a consumer is still reading, silently, with
the damage visible only later as data that does not match what was sent.

### G12 — The Assertion Cannot Fail

```rust
// tests/gating_test.rs:394-428, as originally written
const CAPACITY : usize = 64;
let set = GatingSet::new( cap( CAPACITY ), 1 );
let producer = Seq( CAPACITY as u64 );

std::thread::scope( | scope |
{
  scope.spawn( ||
  {
    for position in 0..CAPACITY as u64
    {
      set.cursor( 0 ).unwrap().store( Seq( position ), Ordering::Release );
    }
  } );

  for _ in 0..10_000
  {
    let headroom = set.headroom( producer );
    assert!( headroom <= CAPACITY, "reported {headroom} slots of a {CAPACITY}-slot ring" );
  }
} );
```

`headroom <= CAPACITY` is true for **every possible input**, not merely for the
ones this test produces:

```rust
// ring_gating/src/lib.rs:222-228
self.slowest().map_or( self.capacity.get(), | slowest |
{
  ring_seqno::free_slots( producer, slowest, self.capacity )
} )

// ring_seqno/src/lib.rs:95-99
( capacity.get() as u64 ).saturating_sub( in_flight ) as usize
```

Both arms are bounded by `capacity` by construction — the `map_or` default *is*
`capacity`, and `saturating_sub` from `capacity` can only go down. So the
assertion holds regardless of what the consumer thread does, what the producer
position is, or whether the fold returns the right cursor.

**A second, independent slack.** Even restricted to this test's own state space
the bound is loose. The writer stores `0..=63`; the cursor starts at `0`; the set
has one consumer so `slowest()` is never `None`. Therefore

```
headroom( Seq( 64 ) ) = 64 - ( 64 - s ) = s,  for s ∈ [ 0, 63 ]
```

— the reachable range is `0..=63`, and the assertion permits `64`. The one value
an off-by-one over-report would produce is exactly the one the comparison lets
through.

| Broken implementation | Detected by this test? |
|-----------------------|:----------------------:|
| Inclusive boundary (`headroom + 1`) | ❌ — produces 64, and `64 <= 64` |
| Swapped `free_slots` arguments | ❌ — produces a constant 64 |
| Reads the wrong cursor | ❌ — still ≤ capacity |
| `Relaxed` instead of `GATING` | ❌ — a torn read is still ≤ capacity |
| Abandons `free_slots` for raw arithmetic that overflows | ✅ |

Only the last row is caught, and it is the one failure nobody is likely to write.
The test's title names the property; its assertion checks a different, weaker one.

### What the Test Does Still Establish

It is not worthless, and the two things it does prove are worth naming:

| Established | How |
|-------------|-----|
| `GatingSet : Sync` | `&set` crosses into the spawned closure; the code would not compile otherwise — [`invariant/002`](../invariant/002_this_crate_names_no_ordering.md) |
| No panic or deadlock under concurrent access | 10,000 reads against a live writer, and `thread::scope` joins both |

The first is a real compile-time guarantee obtained for free, and the second is
the kind of thing only a concurrent test can give. What is missing is the
assertion the name promises.

### Two Assertions That Would Close It

**The one-character fix.** The consumer never reaches `CAPACITY`, so:

```rust
assert!( headroom < CAPACITY, "reported {headroom} slots of a {CAPACITY}-slot ring" );
```

catches the inclusive-boundary bug deterministically, at the moment the writer
reaches position 63. It is still a bound rather than a race check, but it is a
*reachable* bound.

**The property assertion.** Consumers only advance, so a cursor read *after* the
gate read is at or ahead of the position the gate saw:

```rust
let headroom = set.headroom( producer );
let after = set.cursor( 0 ).unwrap().load( Ordering::Acquire );
assert!( headroom as u64 <= after.0, "gate saw {headroom}, cursor is at {}", after.0 );
```

For this producer, `headroom` *is* the position the gate observed. Any
implementation reporting a position the consumer has not reached fails the
moment the consumer has not moved between the two reads — which, over 10,000
iterations against a 64-store writer, is almost every iteration.

Neither is added here: a test change belongs to a run with its own verification.
Recorded as **the highest-value test addition in this crate**, ahead of the C1
sweep proposed in
[`invariant/001`](../invariant/001_the_bound_is_the_minimum_and_only_the_minimum.md) —
that one strengthens a clause with three assertions behind it, this one gives a
clause its first.

### Why the Sequential Tests Do Not Cover It

The suite has good boundary coverage — `the_lap_boundary_is_exclusive_on_both_sides`,
the batched full-lap walk, the 1,920-state `check`/`admits` sweep. All of them
run on one thread. The test file's own header says why that is not enough:

> an inclusive boundary passes every "the ring fills up" test and corrupts
> exactly one slot per lap under load — the failure that is hardest to reproduce
> and easiest to write.

That sentence describes the gap precisely, and it was written in the same file as
the assertion that leaves it open. The sequential tests catch an inclusive
boundary because they walk a stalled ring to exactly one lap; the concurrent test
was written to catch the *concurrent* form of the same failure and does not.

### What Would Catch It Elsewhere

| Mechanism | Present? |
|-----------|:--------:|
| `loom` model checking of the gate | ❌ — `[target.'cfg(loom)'.dev-dependencies]` exists in `ring_publish`, not here |
| A `ring_debug` invariant over producer/consumer pairs | Partly — `check_seqs` exists but this crate does not depend on `ring_debug` |
| ThreadSanitizer in CI | Not configured |
| An end-to-end handshake assertion | `ring_publish/tests/handshake_test.rs` — closest, and it is about publication rather than gating |

The second row is the interesting one. `ring_debug::check_seqs` already encodes
*"the producer has not lapped the consumer"* as a checkable violation, and it is
the exact property this test is trying to assert. Nothing wires the two together
— recorded from `ring_debug`'s side in
[`ring_seqno` `workaround/001`](../../../ring_seqno/docs/workaround/001_the_diagnostic_that_reimplements_the_readings.md).

### GT40 — The Concurrency Assertion Cannot Fail

```
assert!( headroom <= CAPACITY )
  empty arm      -> returns exactly CAPACITY
  free_slots arm -> cannot exceed the capacity it is given
```

The test spawns a consumer, races it against a gate read, and then asserts
something that is true of every possible return value. The race is real; the
assertion is not load-bearing.

**Finding.** It asserts `headroom <= CAPACITY`, which is true by construction for every possible input; the test cannot fail

**Disposition:** applied — `tests/gating_test.rs:394-428`'s own doc comment
(`Root Cause`/`Why Not Caught`/`Fix Applied`/`Prevention`, at `:361-393`) records
that both assertions proposed below under "Two Assertions That Would Close It"
have been added: `<=` tightened to `<` (`:418`), and the property assertion
comparing the gate's reading against a fresh cursor load (`:420-425`). GT40's
analysis above describes the assertion as originally written and remains an
accurate account of why that version could not fail; the gap it identifies is
now closed in source.

---

### GT41 — What a Real Over-Report Would Take

```
the hazard : a stale cursor read reporting room a consumer has not released
the control: ring_cursor::GATING, one crate over
named in this crate : 0 times
```

Over-reporting is an ordering failure. The ordering that prevents it is not
named in this crate, so a test of it here would be a test of the delegate — which
is why the requirement is stated here and can only be verified there.

**Finding.** The real hazard is a stale cursor read — reporting room a consumer has not released — which is an ordering question answered in `ring_cursor` by `GATING`. Nothing here tests it and nothing here could: the ordering is not named in this crate, so a test of it would be a test of the delegate

---


### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_the_bound_is_the_minimum_and_only_the_minimum.md](../invariant/001_the_bound_is_the_minimum_and_only_the_minimum.md) | The sequential form of the same rule, and its own thin clause |
| [../invariant/002_this_crate_names_no_ordering.md](../invariant/002_this_crate_names_no_ordering.md) | What the concurrent test proves for free |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_reading_that_returns_a_position.md](../api/002_the_reading_that_returns_a_position.md) | The staleness table this requirement underwrites |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [001_every_gating_read_allocates_nothing.md](001_every_gating_read_allocates_nothing.md) | What the correct reading costs |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_reversing_the_two_refusals.md](../pitfall/002_reversing_the_two_refusals.md) | The other failure whose test is weaker than its name |

### Sources

| File | Relationship |
|------|--------------|
| `ring_gating/src/lib.rs:222-228` | `headroom` — bounded by capacity in both arms |
| `ring_seqno/src/lib.rs:95-99` | `free_slots` — the `saturating_sub` that makes the bound structural |
| `ring_debug/src/lib.rs:282-312` | `check_seqs` — the same property, encoded, unwired |

### Tests

| File | Relationship |
|------|--------------|
| `tests/gating_test.rs:394-428` | The test in question |
| `tests/gating_test.rs:100-109` | The sequential boundary check that does catch an inclusive bound |
| `tests/gating_test.rs:111-131` | The batched full-lap walk |
