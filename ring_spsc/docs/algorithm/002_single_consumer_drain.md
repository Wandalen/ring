# Algorithm: Single-Consumer Drain to the Published Bound

### Scope

- **Purpose**: Specify the single consumer's drain procedure, and establish why its available-bound is a plain cursor read rather than a scan for a contiguous published prefix.
- **Responsibility**: The step sequence, the ordering, the batch shape, and the reason the drain point is unambiguous here in a way it is not elsewhere.
- **In Scope**: The consumer side; the available computation; the commit.
- **Out of Scope**: The producer's publish (→ [Uncontended Claim and Publish](001_uncontended_claim_and_publish.md)); what the consumer does with drained records; the barrier machinery this crate does not use.

### Abstract

The consumer reads every record between its own cursor and the producer's,
then advances its cursor past them in one store. Because publication is
strictly sequential (→ [Slot State Without Holes](../lifecycle/003_slot_state_without_holes.md)),
the producer cursor *is* the available bound — there is nothing to scan for
and no per-slot readiness stamp to consult.

**This is the property that makes SPSC the configuration where "a single
consumer's drain point is unambiguous, so the barrier rule can be enforced
exactly."** In a
multi-producer ring, producer 2 can publish sequence 7 while producer 1 still
holds an unpublished claim on sequence 6; the highest *published* sequence and
the highest *contiguous published* sequence differ, and only the latter is
safe to drain to. With one producer that gap cannot open.

| | SPSC | MPSC |
|---|---|---|
| Available bound | The producer cursor, read directly | The contiguous published prefix, which must be established |
| Cost of computing it | One acquire load | A scan or per-slot stamp comparison |
| Can a hole exist? | No — publication is sequential | Yes — a slower producer's unpublished claim |

### Algorithm

Draining a batch:

1. **Read the consumer cursor** — a plain load. This thread is its only
   writer.
2. **Read the producer cursor** — an `Acquire` load. Pairs with the producer's
   release in step 6 of the publish, making that thread's slot writes visible
   before this thread reads them.
3. **Compute the batch.** `available = producer - consumer`. If zero the ring
   is empty and the drain returns without touching a slot; the configured
   [`ring_wait`](../../../ring_wait/readme.md) strategy decides what happens
   next, and `WaitKind::None` returns immediately.
4. **Read the slots** — indices `consumer & (CAPACITY - 1)` forward for
   `available` records, wrapping once at most, since `available` can never
   exceed `CAPACITY`. Reads are plain: the acquire in step 2 already
   established visibility for all of them.
5. **Commit** — a single `Release` store advancing the consumer cursor by
   `available`. This is the release the producer's step-2 acquire pairs with,
   and it is what marks the slots reusable.

**Step 5 is one store for the whole batch, not one per record.** Draining 64
records costs the same synchronization as draining one, which is the
amortization the batch shape exists for and the reason the API is
batch-shaped rather than item-shaped
(→ [Consumer Surface](../api/002_consumer_surface.md)).

**The window between step 4 and step 5 is the one that matters for
correctness.** Until the commit lands the producer still sees the slots as
occupied and will not overwrite them; after it lands they are reusable
immediately. A consumer that holds borrowed references into drained slots
past step 5 is reading memory the producer may already be writing — which is
why the borrow-versus-copy question in the surface instance is a correctness
question and not an ergonomics one.

**Open at this grain, and closed at the next one.** Whether step 4 yields
borrowed slices or copies out, and whether steps 3–5 are one call or three,
belongs to [Consumer Surface](../api/002_consumer_surface.md). The answer, for
a reader following the chain: one call, `drain()`, returning a `Batch` that
borrows the slots and commits on drop — so steps 3–5 are one call whose step 5
is the guard's destructor.
Whether the family's numbers justify the design is a judgment made elsewhere;
this instance specifies the procedure those numbers will be taken from.

### Algorithms

| File | Relationship |
|------|--------------|
| [001_uncontended_claim_and_publish.md](001_uncontended_claim_and_publish.md) | The producing half; its step 6 release is what step 2 acquires |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_consumer_surface.md](../api/002_consumer_surface.md) | The shape this procedure is exposed through, and the borrow-versus-copy question step 4 raises |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_two_cursor_ring.md](../data_structure/001_two_cursor_ring.md) | The cursors steps 1, 2 and 5 touch |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_exactly_one_producer_one_consumer.md](../invariant/001_exactly_one_producer_one_consumer.md) | Step 1's plain load and step 5's unsynchronized advance both depend on it |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_byte_parity_over_one_hundred_thousand.md](../non_functional_requirement/002_byte_parity_over_one_hundred_thousand.md) | The in-order, zero-loss condition this procedure must satisfy |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/003_slot_state_without_holes.md](../lifecycle/003_slot_state_without_holes.md) | Why step 3's bound needs no scan |
| [../lifecycle/004_ring_occupancy.md](../lifecycle/004_ring_occupancy.md) | The Empty case step 3 detects |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_free_capacity.md](../type/002_free_capacity.md) | The complement of step 3's `available` |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_wait/readme.md`](../../../ring_wait/readme.md) | The wait strategy invoked when the ring is empty, in step 3 |

### Tests

| File | Relationship |
|------|--------------|
| `tests/spsc_test.rs` | `one_producer_and_one_consumer_exchange_one_hundred_thousand_items` — records read back in issue order with zero loss |
| `tests/spsc_test.rs` | `tests/manual/readme.md` S2 — step 5's amortization. A counting shim was considered and rejected: `CursorPair` is not generic over its cell type, so wiring [`ring_atomic::CountingSeq`](../../../ring_atomic/readme.md) through would mean making `ring_cursor` generic for a test. The source reading is stronger anyway — it finds **exactly two** `.store(` sites in the whole crate, one per end, so a drain of any length performs one. A shim would only report what one run happened to do |

### SP3 — The Drain Has No Frontier to Compute

`ring_mpsc`'s drain walks stamps forward and stops at the first gap, because a
producer cursor that advances at claim time is not a frontier when publication
completes out of order. Here it is: the cursor advances after the write, so its
value *is* the highest published sequence.

**One load replaces a scan**, and that is the largest single difference between
the two drains. It is also why this crate has no analogue of the sibling's
slow-producer stall — there is no producer whose held claim could block another's
published record, because there is no other producer.

### SP4 — Both `drain_up_to` Boundaries Are Pinned

Two tests at the two ends. The zero case matters because a no-op drain still
constructs a `Batch`, and a `Batch` commits on drop — so "took nothing" must
also mean "advanced nothing", which is not automatic.

The over-large case matters for the same reason `ring_mpsc`'s does: the obvious
implementation scans `max` sequences from the read position and reads stamps for
sequences no producer has reached. Here the failure would be reading slots the
producer has not written.
