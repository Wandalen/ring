# Lifecycle: Ring Occupancy Between the Cursors

### Scope

- **Purpose**: State the ring's three occupancy conditions as a function of the distance between the two cursors, and establish which of them a caller can act on without racing.
- **Responsibility**: Define occupancy, name the transitions that change it, and be explicit that two of the three states are advisory readings rather than decidable facts.
- **In Scope**: The `producer_cursor - consumer_cursor` distance, the states it partitions into, and what each state permits; the observability of each boundary.
- **Out of Scope**: What a producer *does* on reaching Full, an undecided policy (→ [Bounded Capacity and Backpressure Policy](../non_functional_requirement/002_bounded_capacity_backpressure.md)); the per-slot view (→ [Slot State Across One Lap](003_slot_state_across_one_lap.md)); how the consumer decides to drain (→ [The Spinning Consumer Owns a Core](../pitfall/001_spinning_consumer_owns_a_core.md)).

### States

Occupancy is `D = producer_cursor - consumer_cursor`, the count of sequences
claimed but not yet drained. Because both cursors are monotonic and the
producer never trails the consumer, `D` is always in `0 ..= CAPACITY`.

| State | Condition | A producer may claim? | The consumer may drain? |
|-------|-----------|-----------------------|-------------------------|
| **Empty** | `D == 0` | Yes | No — nothing claimed, so nothing can be published |
| **Occupied** | `0 < D < CAPACITY` | Yes | Only up to the published watermark, which may be below `producer_cursor` |
| **Full** | `D == CAPACITY` | No — the next claim would target a slot the consumer has not released | Yes, and it is the only thing that can restore progress |

**Occupied is not the same as readable, and conflating them is the error this
table exists to prevent.** `D` counts *claimed* sequences, so it includes
slots currently in the Written state — claimed by a producer that has not yet
published. The drain's watermark is therefore at or below `consumer_cursor +
D`, never above it, and a caller reasoning "there are `D` items to read"
overstates. The readable count is discovered only by scanning stamps
(→ [Batch Drain](../algorithm/002_batch_drain_by_cursor_swap.md) Step 3);
`D` bounds it and does not equal it.

### Transitions

| # | From → To | Effected by | Field write |
|---|-----------|-------------|-------------|
| U1 | Empty → Occupied | Producer claim | `producer_cursor` compare-exchange |
| U2 | Occupied → Occupied | Either side | Either cursor moves without crossing a boundary |
| U3 | Occupied → Full | Producer claim taking `D` to `CAPACITY` | `producer_cursor` compare-exchange |
| U4 | Full → Occupied | Consumer drain | `consumer_cursor` advance |
| U5 | Occupied → Empty | Consumer drain catching the producer | `consumer_cursor` advance |

**Only the consumer can leave Full, and only the producer can leave Empty.**
The machine has no self-recovery: a Full ring with a stopped consumer stays
Full forever, and an Empty ring with no producers stays Empty forever. Both
are correct terminal behaviours rather than deadlocks — nothing is held, no
thread is blocked by this machine itself — but a caller that spins waiting
for a state only the other side can produce has built a livelock on top of a
machine that never promised liveness.

### Behavioral Invariants

1. **`D` never exceeds `CAPACITY`.** Enforced inside the claim's own
   compare-exchange rather than by a separate pre-claim check — which is the
   resolution of invariant 3 below. This read "enforced by the producer's own
   pre-claim check, not by the fetch-add — which is exactly the weakness in
   invariant 3 below" until a later revision, and the weakness it named is real: a check
   outside the claim cannot enforce this. Fusing them is what makes the
   enforcement sound rather than hopeful.
2. **`D` never goes negative.** The consumer never advances past the
   published watermark, and the watermark never exceeds `producer_cursor`,
   so `consumer_cursor <= producer_cursor` holds unconditionally.
3. **Empty and Full are advisory when read, decidable only by their owner.**
   This is the invariant with teeth. A producer reading `D < CAPACITY` and
   then claiming has performed two separate operations: between them, other
   producers may have claimed the remaining room, so the check does not make
   the claim safe. Only two things resolve it — a claim that can fail (a
   compare-exchange loop rather than a fetch-add, giving up the wait-free
   property [Claim-Then-Publish](../algorithm/001_claim_then_publish.md)
   Step 1 was chosen for), or an external guarantee that producers never
   collectively outrun capacity.

   **Taken: the failing claim.** `ring_claim`'s `Claimer::claim` is a
   `compare_exchange` loop that re-reads the gating headroom on every attempt,
   so the check and the claim are one atomic operation rather than two —
   which is the only shape that closes the gap this invariant names. The
   wait-free property is genuinely surrendered: it was never available on a
   bounded ring in the first place, because a `fetch_add` claim hands out
   sequences past the consumer's tail and has no wait-free undo.

   **This instance's own contribution is what made the choice obvious**, and
   it is worth naming: reading the problem as *state observability* rather
   than *policy* shows that the policy question
   ([Bounded Capacity and Backpressure Policy](../non_functional_requirement/002_bounded_capacity_backpressure.md))
   is downstream. Full is not observable atomically alongside the action it
   gates, so *no* policy can be implemented correctly on top of a separate
   check — the fusion has to happen first, and only then does "what do we do
   when it fails" become answerable. It is answered `RingError::Full`.

   `free_capacity` remains exposed under its plain name, because it is
   genuinely useful for *sizing* (how many might I push before yielding) even
   though it is useless for *gating*. The misuse it invites is documented on
   the method itself, and
   [the producer surface](../api/001_producer_publish_surface.md) states the
   rule that follows: do not read room and then claim — just claim, and handle
   the error.
4. **Empty is safely observable by the consumer, and this asymmetry is
   real.** The consumer is the only writer of `consumer_cursor` and the only
   reader that matters, so a consumer reading `D == 0` knows there is nothing
   to drain *at that instant*, and a producer arriving immediately after only
   makes the reading stale in the harmless direction — it means an extra
   drain call that finds work, never a missed slot or a corrupted read.
   Full has no such benign direction: acting on a stale Full reading is
   either an unnecessary stall (harmless) or, for a stale not-Full reading,
   an overwrite of live data (fatal). One boundary tolerates staleness and
   the other does not.

**Closed: exposed, and under its plain name.** The question was whether
`free_capacity` is exposed at all — publishing a value invariant 3 says cannot
be acted on safely invites the check-then-claim misuse described there, while
withholding it denies a caller the only signal available for load-shedding
above the ring. A middle option was noted: expose it as `free_capacity_hint`
and document the race at the call site.

`Producer::free_capacity` is exposed, and the `_hint` suffix was not taken.
Two reasons, the second of which is the one that decided it:

- The misuse the suffix warns against is *check-then-claim*, and that misuse
  has a positive replacement rather than only a prohibition: just claim, and
  handle `RingError::Full`. A name cannot carry that; a doc comment on the
  method can, and does.
- Every value on this machine is advisory, so a suffix marking one of them as
  advisory implies the others are not. `claimed`, `available`, and
  `is_empty` are all equally stale the instant they return. Marking one is
  worse than marking none, because it converts a uniform property into an
  apparent distinction.

The uncomfortable half, stated rather than hidden: `available` and `is_empty`
on the consumer side are *not* equally safe — invariant 4 says the consumer's
Empty reading is stale only in the harmless direction, while the producer's
Full reading is not. So the naming is uniform across a property that is not
uniform. That is a real defect in the surface, documented as a warning on the
methods rather than repaired by a rename
(→ [Consumer Drain Surface](../api/002_consumer_drain_surface.md)), because a
rename would need all four names to change and the family has not agreed one
vocabulary for it.

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_consumer_drain_surface.md](../api/002_consumer_drain_surface.md) | Exposes the Empty reading that invariant 4 says is safe, and is where the `free_capacity_hint` question lands if answered yes |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_claim_then_publish.md](../algorithm/001_claim_then_publish.md) | Effects U1 and U3; its Step 1 compare-exchange is the fused operation invariant 3 requires — the fusion invariant 3 said a fetch-add could not provide |
| [../algorithm/002_batch_drain_by_cursor_swap.md](../algorithm/002_batch_drain_by_cursor_swap.md) | Effects U4 and U5; the only escape from Full |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_sequence_stamped_ring.md](../data_structure/001_sequence_stamped_ring.md) | Owns both cursors and states `free_capacity` as an operation; this instance states what its value can and cannot be used for |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_bounded_capacity_backpressure.md](../non_functional_requirement/002_bounded_capacity_backpressure.md) | Owns the policy question this instance shows is downstream of Full's non-atomic observability |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_spinning_consumer_owns_a_core.md](../pitfall/001_spinning_consumer_owns_a_core.md) | A consumer polling for Occupied is the spin this pitfall costs a core for; the Empty reading is what it polls |

### State Machines

| File | Relationship |
|------|--------------|
| [003_slot_state_across_one_lap.md](003_slot_state_across_one_lap.md) | The per-slot view; Full is precisely the condition that blocks its T1 on the oldest undrained slot |

### Sources

| File | Relationship |
|------|--------------|
| `src/lib.rs` | `Producer::free_capacity` and `Producer::claimed` are this machine's observables; the fused check-and-claim that Behavioral Invariant 3 requires lives in `ring_claim` |

### Tests

| File | Relationship |
|------|--------------|
| `tests/mpsc_test.rs::a_claim_past_capacity_reports_full_rather_than_overwriting` | `D` reaches exactly `CAPACITY` and no further under a producer loop with a stopped consumer — U3, and the failing-claim resolution of Behavioral Invariant 3 |
| `tests/mpsc_test.rs::a_commit_restores_exactly_the_capacity_it_released` | The return to Occupied on the first drain — U4, with the exact accounting rather than a "roughly recovers" check |
| `tests/mpsc_test.rs::an_unpublished_claim_blocks_every_later_sequence_while_it_is_held` | With `D > 0` and no publication yet, the drain yields zero elements — the Occupied-is-not-readable claim |
| `tests/mpsc_test.rs::a_live_batch_still_holds_its_slots_against_reuse` | That `D` does not shrink until the batch commits, which is what makes Behavioral Invariant 2's bound hold across a live borrow |

### MP36 — A Commit Restores Exactly the Capacity It Released

Occupancy is the gap between two cursors, so it is correct only if every advance
is matched. The test asserts the round trip: claim `n`, commit `n`, and
`free_capacity` returns to what it was.

An off-by-one in either direction is invisible to a throughput test — records
still flow — and shows up only as a ring that slowly loses or gains apparent
room across many laps.
