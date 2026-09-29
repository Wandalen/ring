# Type: Sequence Number

### Scope

- **Purpose**: Define the single value that serves as claim ordinal, slot address, lap counter, and publication token at once, and state the decomposition that lets one integer do all four jobs.
- **Responsibility**: Fix the type's meaning, its arithmetic relationships to slot index and lap, its validation rules, and the representation questions the benchmark still owns.
- **In Scope**: The sequence value, its derivation of slot index and lap, its monotonicity guarantee, and its width.
- **Out of Scope**: Where sequences are stored (→ [Sequence-Stamped Ring](../data_structure/001_sequence_stamped_ring.md)); the states a stamped sequence encodes (→ [Slot State Across One Lap](../lifecycle/003_slot_state_across_one_lap.md)); the slot count it is reduced modulo (→ [Capacity](002_capacity.md)).

### Definition

A **sequence number** is a monotonically increasing unsigned integer,
allocated exactly once per claim by a winning compare-exchange on
`producer_cursor`, that identifies one publication event for the lifetime of
the ring. It is never
reused, never reset, and never decreases.

Its power is that four separate concepts are recoverable from it by
arithmetic alone, so none of them needs its own field:

| Derived concept | Derivation | Used by |
|-----------------|------------|---------|
| **Slot index** — which cell to write | `sequence % CAPACITY` | The payload write and the stamp write |
| **Lap** — how many times the ring has wrapped past this cell | `sequence / CAPACITY` | Nothing directly; it is implicit in the comparison below |
| **Publication token** — the value stamped to mark the slot readable | the sequence itself | The consumer's `stamps[i] == expected` test |
| **Total order position** — where this element sits in the consumer's output | the sequence itself | [Single-Consumer Total Order](../invariant/001_single_consumer_total_order.md) |

**The lap is never computed in practice, and that is the elegant part.** A
naive reading of the state machine suggests the consumer divides to recover
the lap and then compares. It does not: it tracks `consumer_cursor`, already
knows the next sequence it expects, and compares `stamps[i]` against that
expected sequence directly. The lap exists as a concept for reasoning about
why the comparison is sound across wraps — because sequences are unique
across all laps — and never as a computed quantity in the hot path. A
division per drained element would be a real cost; there is none.

**One value, one allocation point.** Every sequence originates at exactly one
place, the compare-exchange in
[Claim-Then-Publish](../algorithm/001_claim_then_publish.md) Step 1. There is
no other constructor. ("One instruction" until a later revision — the loop may execute the
instruction many times, but only the winning execution allocates, so the
allocation point is still singular even though the instruction count is not.) This is why uniqueness needs no separate enforcement
mechanism: hardware serializes the read-modify-write, so two producers cannot
receive the same value, and the property this crate's central invariant rests
on is a property of the instruction rather than of any code this crate
writes.

**Its relationship to the payload type is: none.** A sequence orders and
addresses; it says nothing about what was published. That separation is what
lets the same ring machinery carry any `T`, and what makes the payload
constraint an independent question
(→ [`ring_tls`'s POD payload requirement](../../../ring_tls/docs/non_functional_requirement/002_pod_pointer_free_payloads.md),
the same agnosticism stated for the sibling crate).

### Validation

| Rule | Statement | Checked where |
|------|-----------|---------------|
| V1 | Strictly increasing across allocations — no two claims yield equal values, and later claims yield larger ones | The atomicity of the claim's read-modify-write; not re-checked in software |
| V2 | Never decreasing on either cursor — both `producer_cursor` and `consumer_cursor` are monotonic for the ring's whole lifetime | Debug assertion on cursor advance |
| V3 | `consumer_cursor <= producer_cursor` always | Debug assertion in the drain |
| V4 | A stamp equals the sequence published into it, or a strictly smaller one — never a larger one | Debug assertion in the drain's watermark scan; a larger stamp means a producer published out of its own slot |
| V5 | The value fits its representation without wrapping for the ring's operational lifetime | **Not checked** — see below |

**V5 is the rule with no enforcement, and pretending otherwise would be
dishonest.** At `usize` width on a 64-bit target, one claim per nanosecond
exhausts the space in roughly 585 years, so wraparound is not a runtime
concern and no check is written for it. At `u32` — which halves the stamp
array's memory traffic, a real benefit on the consumer's sequential scan —
the space is ~4.3 × 10⁹ claims, reachable in about seventy seconds at the
same rate. Every comparison in the state machine then needs wrapping-aware
arithmetic (`wrapping_sub` against a half-range threshold rather than a plain
`<`), and every invariant above needs restating in those terms. **No source
specifies the width**, and the choice is a genuine trade rather than an
oversight: memory bandwidth on the hot path against a materially more
delicate correctness argument. It belongs to this family's own benchmark
harness, which can measure the bandwidth half; the delicacy half is not
measurable and is recorded here as the cost side of that measurement.

**V1 is the one rule the crate genuinely cannot violate**, though the reason
is narrower than this instance originally gave. V2, V3, and V4 are assertions
over code this crate writes and could in principle be broken by a bug. V1 was
attributed to "a property of `fetch_add` itself"; the claim is not a
`fetch_add`, it is a `compare_exchange` loop — but uniqueness survives the substitution
intact, because a CAS that loses simply retries and only the winner advances
the cursor. V1 rests on the atomicity of the read-modify-write, whichever
instruction supplies it. The distinction still matters for the reason stated:
it locates a total-order failure in the drain's cursor handling, not the
claim.

**Closed: a newtype, and it is `ring_types::Seq`.** The question was whether
the type is a newtype or a bare integer, noting a newtype makes sequence/slot-
index transposition a compile error at no runtime cost, and that nothing
decided yet specified either. The newtype exists, in `ring_types` rather than
here — which is the better placement, since `Seq` crosses every crate in the
family and a per-crate newtype would have to be converted at each boundary,
reintroducing exactly the transposition risk it exists to prevent.

Two consequences this instance did not anticipate, both good:

- **`Seq` is a `u64` newtype, not `usize`.** Width is therefore fixed rather
  than target-dependent, so V3's wraparound argument (2⁶⁴ publications) holds
  identically on a 32-bit target instead of collapsing to 2³² there.
- **The newtype is what makes `UNSTAMPED` safe to state.** `Seq( u64::MAX )`
  as a sentinel is only defensible because `Seq` is a distinct type whose
  every construction site is visible; as a bare integer it would be one more
  magic number among the slot indices.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_claim_then_publish.md](../algorithm/001_claim_then_publish.md) | Step 1 is this type's sole constructor; Steps 2–3 consume its slot-index derivation |
| [../algorithm/002_batch_drain_by_cursor_swap.md](../algorithm/002_batch_drain_by_cursor_swap.md) | Compares stamps against expected sequences — the comparison V4 governs |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_sequence_stamped_ring.md](../data_structure/001_sequence_stamped_ring.md) | Stores sequences in `stamps` and in both cursors; owns the inline-versus-parallel-array question this type's width interacts with |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_single_consumer_total_order.md](../invariant/001_single_consumer_total_order.md) | Rests entirely on V1; the total order *is* the sequence order |
| [../invariant/002_publication_ordering.md](../invariant/002_publication_ordering.md) | Governs the orderings the stamp store and load carry |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/003_slot_state_across_one_lap.md](../lifecycle/003_slot_state_across_one_lap.md) | Its four states are all defined as comparisons against an expected sequence |

### Types

| File | Relationship |
|------|--------------|
| [002_capacity.md](002_capacity.md) | The modulus in this type's slot-index derivation; whether that reduction is a mask or a division is its question |

### Sources

| File | Relationship |
|------|--------------|
| `src/lib.rs` | Consumes `ring_types::Seq` throughout; defines `UNSTAMPED` as the one sentinel value and `stamp( seq )` as the slot-index derivation V2 governs |
| `../../../ring_types/src/lib.rs` | Where `Seq` actually lives — the newtype decision's placement, one crate down |

### Tests

| File | Relationship |
|------|--------------|
| `tests/mpsc_test.rs::four_producers_exchange_one_hundred_thousand_items_with_byte_parity` | Concurrent claims across four threads yield exactly the set `0 .. total` with no duplicates and no gaps — V1 asserted directly rather than assumed from the instruction |
| `tests/mpsc_test.rs::every_slot_is_reused_across_many_laps_without_loss_or_duplication` | Slot index and lap round-trip for values spanning several laps — V2 |
| `tests/mpsc_test.rs::a_stale_stamp_from_the_previous_lap_does_not_read_as_published` | V4's equality comparison, in the case where an ordering comparison would have gone wrong |
| `tests/mpsc_test.rs::stamps_start_unstamped_and_there_is_exactly_one_per_slot` | That `UNSTAMPED` is `Seq( u64::MAX )` and not something a real sequence reaches |
| `tests/mpsc_test.rs::the_batch_reports_the_sequences_it_covers` | That a drained batch names its sequences, so the type is observable to a caller rather than only internal |

### MP48 — The Sequence Is `ring_types`' and Every Rule About It Lives There

The no-wrap bound, the `ZERO` constant, the ordering — all `ring_types`'. What
this crate adds is one use of the value the defining crate does not anticipate:
`Seq( u64::MAX )` as `UNSTAMPED`
(→ [`../data_structure/002`](../data_structure/002_a_second_array_of_sequence_stamps.md), MP12).

**A sentinel carved out of a sibling's value space, with nothing in the sibling
reserving it.** `ring_types` is free to give `u64::MAX` a meaning tomorrow.
