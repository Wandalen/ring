# Lifecycle: Occupancy Across Backends

### Scope

- **Purpose**: Model the ring's occupancy as the small state machine a caller actually programs against, and mark which transitions a caller can cause and which they merely observe.
- **Responsibility**: The states, the transitions, which readings are trustworthy in which state, and the two loops the model makes correct or incorrect.
- **In Scope**: Occupancy as seen through `len`, `is_empty`, `free_capacity`, `is_full`, and the two success/refusal outcomes.
- **Out of Scope**: Each backend's internal cursor states (→ its crate); closure and shutdown, which this crate does not model (→ `ring_shutdown`).

### States

Three, and the middle one is where a ring spends its life:

| State | `is_empty()` | `is_full()` | `try_recv` | `try_push` |
|---|---|---|---|---|
| **Empty** | true | false* | `None` | accepts |
| **Partial** | false | false | `Some` | accepts |
| **Full** | false | true | `Some` | refuses, or drops per policy |

\* A zero-capacity ring is not constructible — `RingConfig::new( 0 )` fails — so
Empty and Full are always distinct states.

### Transitions

```
        push                push
Empty ────────► Partial ────────► Full
      ◄────────         ◄────────
        recv                recv
```

**Only two of the four transitions are ones a single caller can cause.** A
producer can drive Empty→Partial→Full; a consumer can drive Full→Partial→Empty.
Neither can drive the other's, and in a live ring both are happening.

That is the whole reason the readings behave as they do, and it is worth
stating as the model rather than repeating per method:

| Reading | Direction it bounds | Safe conclusion |
|---|---|---|
| `len()` | how much is **there** | at least this many are available; more may arrive |
| `free_capacity()` | how much **room** | at SPSC, at least this many pushes will be accepted. Elsewhere: nothing |
| `is_empty()` | — | there were zero at the sample instant |
| `is_full()` | — | there was no room at the sample instant |

`len` only grows without the consumer's action because the consumer is the only
party that can subtract; `free_capacity` only holds at SPSC because a single
producer is the only party that can subtract *room*, and only SPSC guarantees
there is one (→ [`pitfall/001`](../pitfall/001_free_capacity_carries_two_contracts.md)).

### Behavioral Invariants

**Correct — the state is read from the operation:**

```rust
while let Some( record ) = consumer.try_recv() { … }        // drains until actually empty
match producer.try_push( record ) { Ok( () ) => …, Err( r ) => … }
```

**Incorrect — the state is read from a sample, then acted on:**

```rust
while !consumer.is_empty() { let r = consumer.try_recv().unwrap(); }   // unwrap can fire
for _ in 0..producer.free_capacity() { producer.try_push( … ).unwrap(); }  // panics at MPSC
```

Both broken forms have the same shape: **a sample taken in one state, used as if
the state had not moved.** The operations return the state *and* act on it in
one step, which is why they cannot drift.

The second broken form is the trap `pitfall/001` documents, and it is correct at
SPSC — which is what makes it survive review.

### What This Machine Does Not Model

**Closure.** There is no Closed state: a drained ring and a closed-and-drained
ring are the same three states here, both answering `None`.

That is not an omission at this layer — liveness belongs to `ring_shutdown`,
and holding a copy of its flag would break
[`invariant/001`](../invariant/001_no_atomic_of_its_own.md). `ring_handle`'s
specified surface *does* distinguish them, using `is_closed()`; the divergence
and its resolution are recorded in
[`integration/002`](../integration/002_handle_surface_divergence.md).

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_producer_surface.md](../api/001_producer_surface.md) | `free_capacity`/`is_full`, and the refusal that ends the push direction |
| [../api/002_consumer_surface.md](../api/002_consumer_surface.md) | `len`/`is_empty`, and why `None` is not an error |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_uniform_drain_over_three_shapes.md](../algorithm/002_uniform_drain_over_three_shapes.md) | How the drain transition is performed at each backend |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_uniform_delivery_across_backends.md](../invariant/002_uniform_delivery_across_backends.md) | That these states and transitions are the same at every backend |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_free_capacity_carries_two_contracts.md](../pitfall/001_free_capacity_carries_two_contracts.md) | The second broken loop, in full |

### Tests

| File | Relationship |
|------|--------------|
| `tests/core_test.rs` | `len_and_is_empty_agree_at_every_point_of_a_lap` — the state readings, checked mid-lap rather than at the ends |
| `tests/core_test.rs` | `free_capacity_never_overstates_the_room_available` — the room bound |
| `tests/core_test.rs` | `draining_an_empty_ring_is_zero_not_an_error` — the Empty state's answers |
| `tests/core_test.rs` | `the_same_program_behaves_identically_on_every_backend` — ~25 laps around the whole cycle |

### CO37 — Occupancy Is Two Numbers From Two Ends and They Need Not Agree

The two readings are taken through different handles, on different backends'
own accounting, with no synchronization between them. At SPSC they are
consistent because only one thread can change either. At MPSC and crossbeam a
reader can hold `free_capacity() == 4` and `len() == 4` on an 8-slot ring while
a third producer is mid-push, and observe a sum of anything from 8 down.

**This is not a defect and it is not stated anywhere.** The lifecycle instance
describes occupancy as a single quantity moving through states; it is two
independently-sampled views of that quantity.

### CO38 — One Test Walks a Full Lap and Asserts the Pair Agrees

The test is well-chosen for what it covers: a full wrap of the ring, checking
`len` and `is_empty` at every step. It is single-threaded, so it exercises the
regime in which the two readings are consistent by construction.

Recorded as coverage rather than as a gap, because the multi-threaded case CO37
describes is not obviously testable — the disagreement it predicts is a
transient a test would have to race to observe. What is missing is a sentence
saying the assertion is scoped to the single-threaded regime.
