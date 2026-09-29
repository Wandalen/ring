# Type: Producer Cursor

### Scope

- **Purpose**: Define the single-writer position value the producer owns, and record the validation rules that separate what this crate may assume from what it must check.
- **Responsibility**: The definition, the four roles the value plays at once, and the rules governing each.
- **In Scope**: The producer-side cursor; its width, monotonicity, ownership, and padding.
- **Out of Scope**: The consumer cursor's mirror-image rules, which differ only in owner; the padded representation itself, which is [`ring_cursor`](../../../ring_cursor/readme.md)'s and [`ring_align`](../../../ring_align/readme.md)'s.

### Definition

A monotonically non-decreasing `u64` counting every record the producer has
published since the ring was constructed, held in a cache-line-padded cell.

**One number, four simultaneous roles** — the same coincidence the sibling
crate's sequence-number instance records, and it is worth restating because
each role imposes a different constraint:

| Role | Reading | Constrains |
|------|---------|------------|
| **Publication count** | How many records exist | Must never decrease, or the consumer under-reads |
| **Slot address** | `cursor & ( CAPACITY - 1 )` names the next slot | `CAPACITY` must be a power of two for this to be a mask |
| **Lap counter** | `cursor / CAPACITY` is the lap | Must not wrap, or two laps become indistinguishable |
| **Publication boundary** | Every sequence below it is readable | Requires the contiguity property (→ [Slot State Without Holes](../lifecycle/003_slot_state_without_holes.md)) |

**The fourth role is SPSC-specific.** In `ring_mpsc` the producer cursor is a
*claim* boundary, not a publication boundary — sequences below it may be
claimed and unpublished. Here the two coincide, and that coincidence is what
lets the consumer read one cursor instead of establishing a prefix.

Its counterpart, the consumer cursor, is the identical type under identical
rules with the owner reversed: written only by the consumer, read by both,
naming the oldest undrained sequence.

### Validation

| # | Rule | Enforced by | Status |
|---|------|-------------|--------|
| V1 | Exactly one thread ever writes it | Nothing at runtime — structurally, by `ring_handle`'s non-`Clone` `Producer` | **Unenforceable here** (→ [Exactly One Producer, Exactly One Consumer](../invariant/001_exactly_one_producer_one_consumer.md)) |
| V2 | Monotonically non-decreasing; never reset, decremented, or wrapped | The only write is `+= 1` (or `+= n` on a batch) | Structural |
| V3 | Never exceeds `consumer + CAPACITY` | The free-space check before every claim | Checked, every publish |
| V4 | Never wraps within a reachable program lifetime | Width: a `u64` at 10⁹ publishes/second exhausts in ≈584 years | By width — asserted, not defended at runtime |
| V5 | Occupies a full cache line, and no other cursor shares it | `ring_align`'s padding asserts `align_of == 64`, `size_of == 64`, and ≥64-byte separation | Mechanically asserted |
| V6 | Written with `Release`, read cross-thread with `Acquire` | The publish's ordering (→ [Uncontended Claim and Publish](../algorithm/001_uncontended_claim_and_publish.md) step 6) | Structural — and invisible if wrong on x86-64 |

**V1 is the one with no enforcement in this crate and the largest blast
radius.** Every other rule is either mechanical or structural; V1 is a
precondition this crate states and something above it upholds.

**V4 is an assertion about width, not a runtime check, and the distinction
matters.** Nothing detects a wrap; the design's answer is that a `u64` cannot
wrap in any lifetime the program will have. That answer is only as good as the
width — a `u32` cursor exhausts in about four seconds at the same rate, and
`usize` is `u32` on a 32-bit target. **The type must be `u64` explicitly, not
`usize`**, and that is a real decision rather than a formality: `Seq` is a
`u64` newtype that never wraps in-range.

**V6 is the rule most likely to be violated without consequence during
development.** x86-64's memory model makes a `Relaxed` store behave as
`Release` in practice for this pattern; AArch64 does not. A cursor written
`Relaxed` passes every test on a developer laptop and corrupts data on a phone
or an Apple-silicon machine. `loom` under
[`ring_testkit`](../../../ring_testkit/readme.md) is what catches it, and it is
the reason the family carries a model-checking dependency at all.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_uncontended_claim_and_publish.md](../algorithm/001_uncontended_claim_and_publish.md) | Reads it plainly in step 1, advances it with a release store in step 6 |
| [../algorithm/002_single_consumer_drain.md](../algorithm/002_single_consumer_drain.md) | Acquire-loads it in step 2 as the available bound — role 4 |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_two_cursor_ring.md](../data_structure/001_two_cursor_ring.md) | Where it sits, and why it has its own cache line |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_family_dependency_seam.md](../integration/001_family_dependency_seam.md) | `ring_cursor` and `ring_align` supply the representation V5 requires |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_exactly_one_producer_one_consumer.md](../invariant/001_exactly_one_producer_one_consumer.md) | V1, stated as the crate's central precondition |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/003_slot_state_without_holes.md](../lifecycle/003_slot_state_without_holes.md) | Role 4's contiguity property |
| [../lifecycle/004_ring_occupancy.md](../lifecycle/004_ring_occupancy.md) | V3's bound, expressed as the Full state |

### Types

| File | Relationship |
|------|--------------|
| [002_free_capacity.md](002_free_capacity.md) | Derived from this cursor and its counterpart |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_cursor/readme.md`](../../../ring_cursor/readme.md) | The `u64` sequence width V4 rests on |
| [`ring_align/readme.md`](../../../ring_align/readme.md) | V5's cache-line padding contract |

### Tests

| File | Relationship |
|------|--------------|
| `tests/spsc_test.rs` | `sequences_are_issued_consecutively_across_a_wrap` and `one_producer_and_one_consumer_exchange_one_hundred_thousand_items` — V2: consecutive across a wrap, and never decreasing under a concurrent drain |
| `tests/spsc_test.rs` | `exhaustive::a_published_record_is_never_observed_before_the_write_that_preceded_it` — V6, which no ordinary test here catches. Measured on an aarch64 host: with `HANDOFF` mutated to `Relaxed`, the 100 000-item test still passes and this one fails (→ `tests/manual/readme.md` S9). The host being weakly ordered is what makes that a finding rather than an artifact of strong hardware |

### SP48 — The Producer Cursor Is a `SeqCell`, Not an Atomic Named Here

This crate declares no atomic type and names none. `CursorPair` and `SeqCell`
carry the whole representation, which is what lets `ring_atomic`'s loom
instrumentation reach this crate's own code without this crate participating
(→ [`../decisions/002`](../decisions/002_the_loom_seam_runs_through_a_crate_this_manifest_never_names.md)).

The cost is that a reader asking "what is the producer cursor, physically?" gets
no answer from this file.
