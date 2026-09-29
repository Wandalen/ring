# Non-Functional Requirement: Bounded Capacity and Backpressure Policy

### Scope

- **Purpose**: Make the ring's footprint a function of capacity rather than of arrival rate, and force the full-ring policy to be declared per priority class rather than picked once globally — because the three candidate policies are not interchangeable.
- **Responsibility**: State the boundedness requirement, enumerate the three policies with what each costs, map them onto the three priority classes the source names, and record what remains unspecified.
- **In Scope**: Memory footprint under sustained producer burst, and what a claim does when the ring is full.
- **Out of Scope**: Throughput and scalability, the other adoption axis (→ [Measured Before Adopted](001_measured_before_adopted.md)); the field the policy reads (→ [Sequence-Stamped Ring](../data_structure/001_sequence_stamped_ring.md)'s `free_capacity`); why an overwritten element is a contract violation rather than a tuning loss (→ [Single-Consumer Total Order](../invariant/001_single_consumer_total_order.md)).

### Quality Attribute

Resource-boundedness under load — peak memory footprint independent of
arrival rate — and the availability consequence that follows from how the
bound is enforced when it is reached.

### Statement

**Capacity is fixed at construction and the publish path never allocates.**
Footprint is `CAPACITY × sizeof( slot ) + stamps + two padded cursors`,
constant across a burst of arbitrary length. This is a design requirement
for the mechanism this crate would replace — a bounded queue exists
specifically to survive a burst without exhausting memory — and it is
precisely what the working mechanism does *not* deliver today: the CAS stack
heap-allocates one node per message, so its footprint tracks arrival rate
with no bound at all (the deleted predecessor mechanism's Step 1). Boundedness is therefore a genuine capability this crate adds, not
a property it inherits.

**Fixing the bound creates a second decision that must not be defaulted.**
Once capacity is finite, a claim can find the ring full, and there are
exactly three things it can do. They are not variants of one another:

| Policy | Behaviour on full ring | What it preserves | What it costs |
|---|---|---|---|
| **Block** | The producer waits until the consumer frees a slot | Every element; the total order is unbroken | Reintroduces an *unbounded* wait on a path that is otherwise lock-free (→ [Claim-Then-Publish Slot Acquisition](../algorithm/001_claim_then_publish.md)); a blocked I/O worker is also a stalled socket, so overload propagates outward instead of being absorbed |
| **Fail the claim** | The call returns `Err( RingError::Full )` and, for `push`, the payload back to the caller unconsumed | Lock-freedom; the producer keeps making progress | Moves the decision to every caller, each of which must then *have* one; a caller with no answer degrades this into "drop, later, less visibly" |
| **Overwrite oldest** | The claim proceeds and destroys an undrained element | Bounded latency; never blocks, never fails | Silently destroys a published element — a direct violation of [Single-Consumer Total Order](../invariant/001_single_consumer_total_order.md)'s exactly-once clause, not a tuning loss |

The third row is why a single global policy is wrong by construction.
Overwrite is not merely aggressive; for any element the contract must
deliver, it is *unavailable* — an overwritten element was published and never
drained, which is exactly the failure that instance names as diverging world
state with nothing erroring anywhere. Yet overwrite is also the correct
policy for one class of traffic, and refusing it there would mean blocking a
real-time input path on stale data.

**Hence: the policy is declared per priority class.** The design being
replaced names three classes and, for exactly one of them, names dropping as
acceptable — segregated so a burst of low-priority client-input traffic
can't starve a Critical message:

| Class | Character | Admissible policies |
|---|---|---|
| **Critical** | Control-plane messages — disconnects, shard-controlling commands | Block or fail. Never overwrite: losing one is losing a control decision |
| **Migration** | Entity-handoff messages from a neighbouring partition | Block or fail. Never overwrite: an overwritten handoff loses the entity itself, with the sender already having released it |
| **Droppable input** | Client input whose value expires — the source's own framing is "with the option to drop stale ones on overflow" | All three, including overwrite. A superseded input is worth less than the latency of waiting for it |

The generalization worth stating once: **overwrite is admissible exactly
where the element's value expires faster than the backlog clears.** That is a
property of the traffic, never of the ring, so the ring cannot decide it and
must not have one policy compiled in.

**Chosen: Fail, and it is the only one of the three that can be built on.**
`Producer::claim` returns `Result< Reserved< '_, S >, RingError >` and
`Producer::push` returns `Result< (), RingError >`, both yielding
`RingError::Full` rather than waiting or overwriting. The reason is
constructive rather than a preference among the three rows:

- **Block is three lines above Fail.** `while let Err( _ ) = producer.claim()
  { core::hint::spin_loop(); }`, or the same with a park/yield. A caller that
  wants Block writes it; a caller that wants Fail *inside* a Block ring cannot
  recover the error the ring already swallowed.
- **Drop-newest is one line above Fail.** `let _ = producer.push( value );`.
- **Overwrite-oldest is not reachable from any of them**, and is the one this
  crate declines outright — not as a policy preference but because it destroys
  a published element, which
  [Single-Consumer Total Order](../invariant/001_single_consumer_total_order.md)'s
  exactly-once clause forbids. A caller that genuinely wants it wants a
  different data structure.

So the table's three rows are not three peers to choose between: one of them
is the primitive and two of them are three-line callers of it. The
replaced design's own framing — pick a policy per class — remains right at
the *channel* layer
(→ [Channel-to-Ring Binding](../pattern/001_channel_to_ring_binding.md)),
which is where the class is known; it is wrong at the ring layer, where it
would delete the caller's ability to implement the other two.

This does not resolve the per-class question below. It removes the ring from
it.

**Open — and load-bearing: separate rings or one tagged ring.** Whether the
three priority tiers are physically separate queues or one queue with a
priority tag is not specified anywhere. Neither option is free, and the
consequences differ in kind:

- **Three rings.** Each class gets its own capacity and its own policy for
  free, and a droppable-input flood cannot consume a Critical message's
  space. But the consumer now performs three drains, and there is no longer
  one sequence counter — so [Single-Consumer Total Order](../invariant/001_single_consumer_total_order.md)'s
  total order holds *within* each ring and not *across* them. Restoring a
  cross-class order would need a merge step with a tie-break rule that no
  source supplies, and any such rule is a new decision, not a detail.
- **One tagged ring.** One sequence counter, one total order, one drain —
  the contract this crate already states, unchanged. But the claim is
  class-blind: a droppable-input burst consumes the sequence numbers and the
  slots a Critical message needed, which is the starvation the segregation
  exists to prevent. Applying a per-class policy at claim time would require
  inspecting occupancy per class, which the structure has no field for. (This
  read "the claim is `fetch_add` and therefore class-blind" until a later revision. The claim
  is a compare-exchange loop, and it is *still* class-blind — the blindness
  follows from there being one cursor, not from which instruction advances it,
  so the correction removes a wrong reason from a right conclusion.)

This crate does not resolve it. It is recorded here so that a mechanism
proposed under [Measured Before Adopted](001_measured_before_adopted.md) is
required to answer it rather than to inherit whichever shape its
implementation happened to make easy.

**Also open**: ring sizing, and wraparound handling — no source specifies
ring sizing, wraparound/overwrite handling, or multi-consumer coordination
beyond the single ticket-issuance, single-consumer shape.

**Wraparound is closed; sizing is not.** Wraparound needed no separate
specification once the stamp was a sequence rather than a flag: a slot on lap
`L + 1` is discriminated from the same slot on lap `L` by the stamp value
itself (→ [Slot State Across One Lap](../lifecycle/003_slot_state_across_one_lap.md)),
so the "wraparound handling" gap turned out to be a consequence of
the flag-shaped framing it was originally raised in rather than a decision anyone had
to make. `every_slot_is_reused_across_many_laps_without_loss_or_duplication`
is the assertion.

No capacity number is committed to here on purpose: a capacity is a claim
about observed burst depth, and no burst has been observed yet.
`Capacity` is a runtime constructor parameter precisely so that the number can
come from a measurement later without a type change
(→ [Capacity](../type/002_capacity.md)).

### Measurement Method

Three measurements, all under the family's own harness; none can run today,
so each names the artifact it needs.

**1 — Footprint plateau.** Run `N` producers offering elements faster than
the consumer drains, for a duration long enough that arrival count exceeds
capacity by orders of magnitude. Sample resident set size throughout.
Boundedness shows as a plateau, not as a slope: a footprint that rises with
total arrivals rather than settling at capacity means something on the
publish path allocated. Run the same workload against the CAS-stack baseline
in the same harness — it is expected to slope, and that contrast is the
measurement's control rather than a separate result.

**2 — Policy conformance, per class.** For each declared class, assert the
declared policy is what actually happens under overflow:

- *Block* — offered elements delivered equals offered elements; producer
  latency rises; drop count is zero.
- *Fail* — the rejected payload is returned to the caller intact and is
  still usable; the producer's own progress is unaffected; drop count is
  zero at the ring, whatever the caller subsequently does.
- *Overwrite* — drops occur only in the droppable class; a drop counter
  advances by exactly the number of elements destroyed.

**3 — Cross-class starvation.** Offer a sustained droppable-input flood
concurrently with a low-rate Critical stream. Measure Critical delivery
latency and loss. This is the measurement that discriminates the two open
shapes above: three rings should show Critical unaffected; one tagged ring
should show Critical latency tracking the flood.

The bound and the Fail policy now exist and are checkable directly; the
three measurements above still are not, because they need the harness. The
distinction is worth keeping sharp — this crate satisfies the *structural*
half of the requirement and none of the *measured* half:

```bash
# from the repository root — the bound and the policy, asserted rather than benched
cargo nextest run -p ring_mpsc a_claim_past_capacity_reports_full_rather_than_overwriting
cargo nextest run -p ring_mpsc a_commit_restores_exactly_the_capacity_it_released
```

The publish path's no-allocation property is likewise structural rather than
measured: `Ring::new` allocates both arrays once and `claim`/`push` write into
them in place, so measurement 1's plateau is a consequence of the code's shape
that a profile would confirm rather than discover. Recording it as
*unmeasured* rather than *satisfied* is deliberate — the reasoning is sound
and it is still not a number.

### Acceptance Threshold

- **Footprint is bounded by capacity, not by arrivals.** The plateau in
  measurement 1 is required; a slope fails the requirement outright. This is
  a shape, not a number — no absolute megabyte figure is committed to,
  because capacity itself is undecided.
- **Zero loss from any non-droppable class, at any offered load.** This is
  not a threshold to tune. It is
  [Single-Consumer Total Order](../invariant/001_single_consumer_total_order.md)
  restated at the overflow boundary, so a mechanism that loses a Critical or
  Migration element under burst fails the functional contract regardless of
  its throughput — the same subordination
  [Measured Before Adopted](001_measured_before_adopted.md) already states:
  speed never trades against the contract.
- **Droppable-class loss is permitted, and must be counted.** A drop the
  consumer cannot observe is indistinguishable at runtime from a delivery
  that never happened, which converts a declared policy into an undiagnosable
  one. The requirement is a monotonic drop counter the consumer can read, per
  class — originated here, not stated by any source, because the source names
  dropping as acceptable without naming it as observable.
- **Capacity itself: deliberately unset.** A number here would be a claim
  about burst depth ahead of the measurement that establishes it. The
  threshold on capacity is that measurement 1's plateau be reached below the
  deployment's memory budget, with the number chosen from measured burst
  depth rather than argued.

### Algorithms

| File | Relationship |
|------|--------------|
| ~~the deleted predecessor crate's push/drain mechanism~~ | The per-message heap allocation whose absence of any bound is the baseline measurement 1 contrasts against. **Deleted 2026-08-26**, and it has no successor. |
| [../algorithm/001_claim_then_publish.md](../algorithm/001_claim_then_publish.md) | The claim whose behaviour on a full ring this requirement governs; its own capacity check is fused into the compare-exchange, which is what makes Fail expressible as a return value rather than a race |
| [../algorithm/002_batch_drain_by_cursor_swap.md](../algorithm/002_batch_drain_by_cursor_swap.md) | The drain rate the fill level is measured against; the watermark's distance from the claim cursor is the occupancy a policy acts on |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_sequence_stamped_ring.md](../data_structure/001_sequence_stamped_ring.md) | Fixed capacity as a field-level property, and the `free_capacity` quantity a policy reads |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_single_consumer_total_order.md](../invariant/001_single_consumer_total_order.md) | The exactly-once clause that makes overwrite a contract violation rather than a configurable loss, and that the zero-loss threshold restates |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [001_measured_before_adopted.md](001_measured_before_adopted.md) | The sibling adoption gate on the throughput axis; both must pass, and this one's zero-loss threshold is the "speed never trades against the contract" clause applied at overflow |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_channel_to_ring_binding.md](../pattern/001_channel_to_ring_binding.md) | Bridges this NFR's three policies to the channel language's `overflow: fail \| stall \| drop` attribute by name, without restating which policy applies to which class |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_spinning_consumer_owns_a_core.md](../pitfall/001_spinning_consumer_owns_a_core.md) | Drain cadence sets how fast capacity is reclaimed — a tick-triggered drain makes the full-ring policy reachable in normal operation, where a spinning consumer would rarely reach it |

### Sources

| File | Relationship |
|------|--------------|
| `src/lib.rs` | `Producer::claim` and `Producer::push` return `RingError::Full` — the Fail policy, and the whole of this requirement that is implemented rather than measured |
| `../../../ring_types/src/lib.rs` | Where `Capacity` and `RingError::Full` live; the bound is a family-level type rather than a per-crate constant |

### Tests

| File | Relationship |
|------|--------------|
| `tests/mpsc_test.rs::a_claim_past_capacity_reports_full_rather_than_overwriting` | The Fail policy at the bound: the claim errors rather than destroying an undrained element — measurement 2's *Fail* row, asserted rather than benched |
| `tests/mpsc_test.rs::a_commit_restores_exactly_the_capacity_it_released` | That the bound is exactly recovered rather than approximately — the accounting measurement 1's plateau would show as a shape |
| `tests/mpsc_test.rs::every_slot_is_reused_across_many_laps_without_loss_or_duplication` | Zero loss across wraparound, which is the non-droppable-class threshold restated structurally |
| `ring_bench` benchmarks (not yet written) | This NFR's three actual measurements: the footprint plateau against the CAS-stack baseline's slope, per-class policy conformance, and cross-class starvation. Named as outstanding, because none of the rows above is a substitute for a number |

### MP39 — Backpressure Is Refusal, and Refusal Is the Only Response

`a_claim_past_capacity_reports_full_rather_than_overwriting` is the assertion.
The crate has no `DropOldest` path and no wait — those are `ring_overflow`'s and
`ring_wait`'s, applied above.

**That makes this crate's contract unusually narrow and unusually checkable**:
one failure mode, one error, no policy. The policy variety a caller sees comes
entirely from the layers around it.

### MP40 — Capacity Is Bounded at Construction and Cannot Change

`a_capacity_that_is_not_a_power_of_two_is_refused_before_a_ring_exists` puts the
validation before the allocation, so no ring exists in an invalid state at any
point.

The power-of-two requirement is `ring_types::Capacity`'s
(→ [`../type/002`](../type/002_capacity.md)), and this crate consumes the
guarantee without rechecking it — the mask fold in the slot lookup is unchecked
subtraction made safe two crates away.
