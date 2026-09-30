# Non-Functional Requirement: What the Spin Costs

### Scope

- **Purpose**: State the cost of `publish`'s unbounded loop — per iteration exactly, in total not at all — and record why the total is not a property of this crate.
- **Responsibility**: Cost one iteration, show the iteration count depends only on terms the crate cannot see, identify the longest deliberate spin in the repository and what it was built to prove, and name what a measurement would need.
- **In Scope**: `publish`'s retry loop (`src/lib.rs:200-210`), and the CPU it occupies.
- **Out of Scope**: Static per-operation cost — see [`non_functional_requirement/001`](001_what_a_publication_costs.md).

### One Iteration, Exactly

```rust
loop
{
  if let Ok( end ) = self.try_publish( start, len )
  {
    return end;
  }
  core::hint::spin_loop();
}
```

| Per failed iteration | Count |
|----------------------|------:|
| `compare_exchange`, failing | 1 |
| `Acquire` load (the CAS's failure ordering) | included in the above |
| `u64` add (`start.advanced_by`) | 1 |
| `core::hint::spin_loop()` | 1 |
| Branches | 1 |
| Memory written | **0** |
| Memory allocated | 0 |

A failed `compare_exchange` still performs a read-modify-write cycle on the cache
line at the hardware level, so *n* spinning producers keep one cache line
ping-ponging between *n* cores for the duration. That is the real cost, and it is
not visible in the instruction count above.

`core::hint::spin_loop()` is a hint, not a delay — `pause` on x86, `yield` on
aarch64. It reduces the memory-order-violation penalty on exit from the loop and
lets a hyperthreaded sibling make progress. It does **not** yield to the
scheduler, so a spinning producer holds its core for the whole wait
([`decisions/002`](../decisions/002_a_plain_spin_rather_than_a_wait_kind.md)
records why yielding would be worse here, and
`ring_mpsc/docs/pitfall/001_spinning_consumer_owns_a_core.md` records the
general hazard from the other side of the family).

### PB31 — The Iteration Count Is a Function of Two Terms, Neither of Them in This Crate

For a producer whose `start` is not yet the frontier:

```
iterations  ≈  ( time until the predecessor publishes )  /  ( cost of one iteration )
```

| Term | Determined by | Known to this crate |
|------|---------------|:-------------------:|
| Time until the predecessor publishes | the peer's payload write — an arbitrary amount of caller code | **no** |
| Cost of one iteration | CAS latency under contention, i.e. core count, interconnect, cache state | **no** |
| Number of predecessors ahead | how many claims were taken and not yet published | **no** |

Not one of the three is a parameter, a field, or an argument. `Publisher` holds
8 bytes of live state — the frontier — and the frontier does not encode how far
behind it a producer is, how many producers exist, or what any of them is doing.

Three consequences, in decreasing obviousness:

1. **The crate cannot bound its own spin**, which is why no budget parameter
   exists — a budget would have to be supplied by a caller who also cannot
   compute it, and its exhaustion would have no correct handling
   ([`decisions/002`](../decisions/002_a_plain_spin_rather_than_a_wait_kind.md)).
2. **The worst case is a peer's payload write, not a queue depth.** With *k*
   producers ahead of you, all *k* writes must finish before your turn arrives —
   but they run concurrently, so your wait is the time until the *last* of them
   completes, not the sum of *k* waits. `tests/publish_test.rs:157-181` is the
   concrete case: four ranges handed out in the order `[ 3W, W, 0, 2W ]`, so the
   thread holding `3W` waits behind three publications it cannot influence, and
   the four threads are the most this crate has ever run at once.
3. **Latency is unfair by construction.** A producer that claims a low sequence
   publishes immediately; one that claims a high sequence waits for everyone
   below it. The design has no notion of fairness because it has no notion of
   waiting at all — it has a turn order, and the turn order is the sequence
   order.

### PB32 — Seven Crates' Tests Look at a Clock; This One Never Does

```sh
cd "$(git rev-parse --show-toplevel)"
grep -rlE 'Instant|Duration|elapsed' ring_*/tests/*.rs
grep -rE 'Instant|Duration|elapsed|sleep|yield_now' ring_publish/tests/*.rs
```

Live output:

```
ring_barrier/tests/barrier_test.rs
ring_bench/tests/bench_test.rs
ring_core/tests/core_test.rs
ring_handle/tests/handle_test.rs
ring_mpsc/tests/mpsc_test.rs
ring_poll/tests/poll_test.rs
ring_wait/tests/wait_test.rs
ring_publish/tests/handshake_test.rs:                        std::thread::yield_now();
ring_publish/tests/handshake_test.rs:                            std::thread::yield_now();
ring_publish/tests/publish_test.rs:            std::thread::yield_now();
```

| Crate | Times something |
|-------|:---------------:|
| `ring_bench`, `ring_handle`, `ring_mpsc`, `ring_poll`, `ring_core`, `ring_wait`, `ring_barrier` | ✔ |
| **`ring_publish`** | **—** |

`ring_publish`'s tests name no clock type at all. The only scheduling primitive
they use is `std::thread::yield_now()`, at three sites
(`publish_test.rs:147`, `handshake_test.rs:457`, `:524`), and every one is a
progress nudge rather than a measurement.

`ring_wait` and `ring_poll` — the two crates whose subject *is* waiting — both
time their bounded loops, because a budget that does not expire in bounded time
is a defect. This crate has no budget, so there is nothing a clock could assert.

The absence is correct and is worth recording precisely because it looks like a
gap. There is no duration this crate could promise: any bound would be a claim
about caller code.

### The Longest Deliberate Spin in the Repository, and What It Proves

`tests/publish_test.rs:126-155` runs a producer in a spin for the duration of
1 000 `yield_now()` rounds on another thread, and every one of those rounds
asserts the spinning thread has *not* published:

```rust
let waiting = scope.spawn( ||
{
  // Starts blocked: the frontier is at 0, and this range begins at 4.
  publisher.publish( Seq( 4 ), 4 )
} );

for _ in 0..1_000
{
  assert!( publisher.published() <= Seq( 4 ), "B published before A" );
  std::thread::yield_now();
}

assert_eq!( publisher.publish( Seq::ZERO, 4 ), Seq( 4 ), "A goes first" );
assert_eq!( waiting.join().expect( "B never panics" ), Seq( 8 ) );
```

The comment at `:142-143` says what the loop is for:

> Nothing this thread can do makes B's publication land early; the only thing
> that unblocks it is A's own publication below.

So the duration *is* the instrument: a long spin is what makes "B never published
early" a meaningful assertion rather than a race the scheduler happened to win.
The number of failed compare-exchanges B accumulates during those 1 000 rounds is
whatever the machine produces, is different on every run, and is recorded
nowhere.

This is the only place in the crate where the spin runs long on purpose, and it
is a safety probe. The cost was never the question.

### What a Measurement Would Need

| Requirement | Present |
|-------------|:-------:|
| A caller outside this crate's tests | **no** — [`api/001`](../api/001_six_methods_and_no_caller.md) § PB10 |
| A benchmark candidate routing through `Publisher` | **no** — [`non_functional_requirement/001`](001_what_a_publication_costs.md) § PB29 |
| A payload larger than one word | **no** — `handshake_test.rs:268-271`'s `payload` is a `u64` |
| A clock anywhere in the crate's tests | **no** — PB32 above |
| Contention above four producers | **no** — `publish_test.rs:157-181` runs four with no consumer; `handshake_test.rs:497-561` runs three with one |

All five are absent, and the first two are the ones that matter: the other three
are cheap to add, and adding them would measure a synthetic caller rather than a
real one.

The one figure that *is* recorded is the scale at which the spin has been
exercised without deadlocking: `tests/handshake_test.rs:497-561` runs three
producers × 3 000 items through a small ring, every publication going through the
loop, and `tests/handshake_test.rs:274-328` runs 20 000 items through 16 slots.
Neither is timed; both would hang rather than fail if the termination argument
([`algorithm/002`](../algorithm/002_a_loop_with_no_budget.md)) were wrong, which
makes them liveness evidence rather than performance evidence.

### The Payload-Size Question, Left Open

[`decisions/002`](../decisions/002_a_plain_spin_rather_than_a_wait_kind.md)'s
reversal table names this as the live condition: *"producer payloads grew large
enough that spinning wasted real cycles"*. The argument that `Spin` beats `Yield`
and `Park` is an argument about *duration* — at a one-word payload the wait is
nanoseconds and yielding is absurd; at a 4 KiB `memcpy` the balance must flip
somewhere.

Nothing in the repository establishes where, and the reason is structural rather
than an oversight: measuring it needs a caller with a real payload, and the two
crates that would have been that caller built stamps instead
([`integration/002`](../integration/002_the_two_crates_that_declined.md)). Under
stamps no producer waits for a peer at all, so the question this crate leaves
open is one the family, as built, never has to answer.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_a_loop_with_no_budget.md](../algorithm/002_a_loop_with_no_budget.md) | The loop, and why it ends |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_refused_rather_than_reordered.md](../decisions/001_refused_rather_than_reordered.md) | The refusal that creates the wait |
| [../decisions/002_a_plain_spin_rather_than_a_wait_kind.md](../decisions/002_a_plain_spin_rather_than_a_wait_kind.md) | Why no strategy and no budget, and what would reverse it |

### Items

| File | Relationship |
|------|--------------|
| [../item/002_the_two_publications.md](../item/002_the_two_publications.md) | `try_publish`, the escape for a caller that cannot afford this |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_a_slot_from_claim_to_visibility.md](../lifecycle/001_a_slot_from_claim_to_visibility.md) | The 2 → 3 window whose duration this waits out |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [001_what_a_publication_costs.md](001_what_a_publication_costs.md) | Everything about the crate that *is* a number |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_publishing_a_range_you_never_claimed.md](../pitfall/001_publishing_a_range_you_never_claimed.md) | The one case where the spin never ends at all |

### Sources

| File | Relationship |
|------|--------------|
| `ring_publish/src/lib.rs:42-53,200-210` | The argument for the bare spin, and the loop itself |
| `ring_wait/src/lib.rs:112-146` | The four strategies this crate declines, including `Park`'s 50 µs sleep |
| `ring_mpsc/docs/pitfall/001_spinning_consumer_owns_a_core.md` | The same hazard, recorded from the consumer side of the family |

### Tests

| File | Relationship |
|------|--------------|
| `tests/publish_test.rs:126-155` | 1 000 rounds of deliberate spinning, as a safety probe |
| `tests/publish_test.rs:157-181` | Four producers, starts `[ 3W, W, 0, 2W ]` — three predecessors deep |
| `tests/handshake_test.rs:274-328` | 20 000 items through 16 slots, every publication spinning or not |
| `tests/handshake_test.rs:497-561` | Three producers × 3 000 — the most contention ever applied |
