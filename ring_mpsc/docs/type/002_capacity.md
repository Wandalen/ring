# Type: Capacity

### Scope

- **Purpose**: Define the ring's fixed slot count as a value type with its own validation rules, and separate the two independent questions bundled inside it — how many slots, and whether that number is constrained to a power of two.
- **Responsibility**: State what capacity fixes, what it does not, the power-of-two trade in full, and which half of it the benchmark can settle.
- **In Scope**: The slot count, its immutability, its power-of-two question, and its relationship to occupancy and footprint.
- **Out of Scope**: The backpressure policy that acts when capacity is reached (→ [Bounded Capacity and Backpressure Policy](../non_functional_requirement/002_bounded_capacity_backpressure.md)); the occupancy states it bounds (→ [Ring Occupancy Between the Cursors](../lifecycle/004_ring_occupancy_between_cursors.md)); the allocation itself (→ [Ring Construction and Teardown](../lifecycle/001_ring_construction_and_teardown.md)).

### Definition

**Capacity** is the number of payload slots in a ring, fixed at construction
and immutable for the ring's lifetime. It is the single parameter that
determines the structure's entire memory footprint, and the only bound on how
far producers may run ahead of the consumer.

What capacity fixes:

| Quantity | Relationship |
|----------|--------------|
| Slot array size | `CAPACITY × size_of::<T>()` bytes, densely packed, no per-cell padding |
| Stamp array size | `CAPACITY × size_of::<Stamp>()` bytes |
| Maximum occupancy | `D` is bounded by `CAPACITY` (→ [Ring Occupancy Between the Cursors](../lifecycle/004_ring_occupancy_between_cursors.md)) |
| Lap length | `CAPACITY` sequences per lap; the stamp's lap discriminator advances by exactly this much |
| Producer run-ahead | At most `CAPACITY` claims may be outstanding before the consumer must drain |

**Capacity bounds footprint; arrival rate does not.** This is the structural
property that distinguishes a ring from the heap-node MPSC it would replace,
where each push allocates and the footprint tracks the backlog. A ring under
a producer burst does not grow — it reaches Full and the backpressure policy
takes over. That makes the memory characteristic a *design-time* number
rather than a runtime risk, and it is the main reason the pattern is a
candidate at all.

**Capacity is a constructor parameter, not a tuning knob.** There is no
`resize`, and adding one would not be a small change: growing means
reallocating both arrays while producers hold sequences addressing the old
one, which needs either a quiescence protocol this crate has no mechanism for
or an indirection on every access that costs exactly what the dense packing
was bought for. The immutability is therefore load-bearing, not incidental.

**What capacity does not fix: whether it is a compile-time constant.**
`CAPACITY` is written as a constant throughout these instances because the
arithmetic reads more clearly that way, but const-generic
(`Ring<T, const N: usize>`) versus runtime (`Ring::with_capacity(n)`) is a
separate question. Const-generic lets the compiler fold the modulo and
eliminates a field load per access; runtime lets one binary serve several
ring sizes chosen from configuration, which is what
[`ring_config`](../../../ring_config/readme.md) exists to supply.

**Resolved: runtime, and the lean was correct.** `Ring::new( Capacity )` and
`Ring::with_config( &RingConfig )` both take the value at construction; there
is no const-generic parameter. Two reasons beyond the lean:

- **A const-generic capacity is viral.** `Ring< S, const N : usize >` puts `N`
  into `Producer`, `Consumer`, `Reserved`, `Batch`, and every signature that
  names any of them — including `ring_handle`'s exported shape, which would
  then be generic over a number chosen from a config file. The parameter would
  have to be erased at exactly the boundary that is supposed to be stable.
- **The saving it buys is already bought.** `Capacity` is validated
  power-of-two at construction, so slot addressing is `seq & ( capacity - 1 )`
  regardless — one instruction, from a field load the compiler hoists out of
  the drain loop. Const-generic would remove that hoisted load, which is not
  measurable against a `Release` store per publication.

### Validation

| Rule | Statement | Consequence if violated |
|------|-----------|-------------------------|
| V1 | `CAPACITY >= 1` | A zero-slot ring has no valid slot index; every claim is unaddressable |
| V2 | `CAPACITY` is immutable after construction | Outstanding sequences address the old array; use-after-free |
| V3 | `CAPACITY × size_of::<T>()` does not overflow the address space | Allocation failure, or worse, a wrapped size and an undersized allocation |
| V4 | `CAPACITY` is a power of two — **conditional**, see below | Slot addressing must use `%` rather than `&`, at a cost stated below |

**V4 is the whole content of this type's open question.** Both sides are
real:

- **Power-of-two required.** `sequence % CAPACITY` compiles to
  `sequence & (CAPACITY - 1)` — one instruction, no division unit, no
  latency worth measuring. This is the standard ring constraint and the
  reason nearly every published implementation carries it. The cost is that
  capacity is quantized: a workload wanting 3000 slots takes 4096 and pays
  33% more memory, or takes 2048 and hits Full a third sooner.
- **Arbitrary capacity allowed.** A general `%` on a runtime divisor is a
  hardware division — on the order of 20–40 cycles, versus one. On the
  producer's path that sits alongside a ~500 ns payload write and is
  invisible. On the consumer's drain it is once per element, in the tightest
  loop the crate has, where it is not obviously invisible at all. A
  reciprocal-multiplication strategy recovers most of it at the cost of
  precomputed state and a less obvious implementation.

The asymmetry is what makes this measurable rather than arguable: the
producer side does not care and the consumer side might. So the question is
not "is division slow" — it is *how much of the drain's per-element cost is
the modulo, at realistic batch sizes*, and that is a benchmark this family can
run directly.

**V4 is required in this crate, and the analysis above is why that is a
deferral rather than a contradiction.** `Capacity::new` refuses a
non-power-of-two before a ring exists, so `ring_mpsc` asserts exactly the
constraint this instance declined to assert in general.

The distinction is real and worth keeping. This instance's argument is that
arbitrary capacity is *admissible* — that V4 is a performance constraint, not
a correctness one, and the crate should not carry it as though it were
correctness. That remains true, and it is a `ring_types::Capacity` question
rather than a `ring_mpsc` one: the validation lives there, so relaxing it
later relaxes it for the whole family at once, with the benchmark this
instance asks for as the evidence. What `ring_mpsc` does is *consume* the
strict type, which costs nothing to reverse — a `Ring` built from a relaxed
`Capacity` needs no change here beyond the addressing expression.

`a_capacity_that_is_not_a_power_of_two_is_refused_before_a_ring_exists`
asserts the current state, so a future relaxation has to change a test that
names the reason rather than silently widening a validation.

**V1 and V3 need real checks; V2 needs a type, not a check.** V1 and V3 are
constructor-time validations with an obvious failure path. V2 is enforced by
not providing a mutator — there is nothing to check at runtime because there
is no operation that could violate it. Recording the difference keeps the
constructor's validation list honest rather than padded.

**Open at this grain.** The capacity *value*, distinct from V4's constraint on
its shape. Nothing decided so far states a slot count for any consumer, and
the right number is a function of producer count, burst shape, and the
consumer's drain interval — none of which are known yet. Both prospective
consumers named in [`../readme.md`](../readme.md) would supply different
answers. It is a per-instantiation configuration value
([`ring_config`](../../../ring_config/readme.md)'s), not a constant this
crate should pick.

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_sequence_stamped_ring.md](../data_structure/001_sequence_stamped_ring.md) | Sizes both arrays by this value; owns the dense-packing trade V4's memory quantization interacts with |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_ring_construction_and_teardown.md](../lifecycle/001_ring_construction_and_teardown.md) | Where V1 and V3 are actually checked, and where immutability begins |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_measured_before_adopted.md](../non_functional_requirement/001_measured_before_adopted.md) | The gate V4's open half is deferred to |
| [../non_functional_requirement/002_bounded_capacity_backpressure.md](../non_functional_requirement/002_bounded_capacity_backpressure.md) | Owns what happens at the bound this type defines |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/004_ring_occupancy_between_cursors.md](../lifecycle/004_ring_occupancy_between_cursors.md) | Its Full state is `D == CAPACITY`; this type is that state's sole parameter |

### Types

| File | Relationship |
|------|--------------|
| [001_sequence_number.md](001_sequence_number.md) | Reduced modulo this value to yield a slot index — the reduction V4 governs |

### Sources

| File | Relationship |
|------|--------------|
| `src/lib.rs` | Takes `Capacity` as a runtime value in both constructors; `Ring::capacity` is the accessor, and there is no const-generic parameter anywhere in the crate |
| `../../../ring_types/src/lib.rs` | Where `Capacity` and its V1/V3/V4 validation actually live — which is why relaxing V4 later is a family-wide change rather than a per-crate one |

### Tests

| File | Relationship |
|------|--------------|
| `tests/mpsc_test.rs::a_capacity_that_is_not_a_power_of_two_is_refused_before_a_ring_exists` | V4 as this crate consumes it, refused at construction rather than at first claim — and named so a future relaxation must edit a reason, not just a bound |
| `tests/mpsc_test.rs::a_config_supplies_the_capacity_and_its_other_fields_are_deliberately_unread` | The runtime-capacity decision's payoff: one binary, capacity from `RingConfig` |
| `tests/mpsc_test.rs::every_slot_is_reused_across_many_laps_without_loss_or_duplication` | The reduction V4 governs, exercised past wraparound where an off-by-one in the mask would show |
| `tests/mpsc_test.rs::drain_up_to_more_than_capacity_is_capped_rather_than_scanning_past_the_ring` | That capacity bounds the drain's scan as well as the claim's headroom |

### MP49 — The Mask Fold Is Unchecked Here Because It Was Checked There

The guarantee crosses a crate boundary and arrives as the absence of code. That
is the strongest form and the least visible one: a reader of this crate's slot
lookup sees no validation and has no local reason to believe the index is in
range.

`ring_types`' `invariant/001` is where the reason lives. Recorded here because
the place a guarantee is *consumed* is where its removal would first be
felt.
