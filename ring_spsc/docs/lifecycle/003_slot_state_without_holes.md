# Lifecycle: Slot State Without Holes

### Scope

- **Purpose**: Give a slot its states across one lap, and establish the property that distinguishes this ring from its sibling — that the published region is always a contiguous prefix, never a set with gaps.
- **Responsibility**: The states, the transitions, which transitions have no writer, and the invariants the contiguity buys.
- **In Scope**: One slot's states over one lap; the ring-wide contiguity property.
- **Out of Scope**: Occupancy across the whole ring (→ [Ring Occupancy Between the Cursors](004_ring_occupancy.md)); the stamped variant (→ [`ring_mpsc`](../../../ring_mpsc/docs/lifecycle/003_slot_state_across_one_lap.md)).

### States

A slot at index *i* passes through four states per lap. **None of them is
stored.** Every state below is derived by comparing the two cursors against the
slot's sequence `S = i + L × CAPACITY` for the current lap `L` — the ring
carries no per-slot field at all
(→ [Two-Cursor Ring Without Per-Slot State](../data_structure/001_two_cursor_ring.md)).

| State | Holds when | Who may touch the slot |
|-------|-----------|------------------------|
| **Reusable** | `consumer > S` for the previous lap's occupant, and `producer <= S` | The producer, exclusively — no reader can reach it |
| **Claimed** | `producer == S` and the payload write is in flight | The producer, exclusively |
| **Published** | `producer > S` and `consumer <= S` | The consumer, read-only. The producer must not touch it |
| **Drained** | `consumer > S` | Nobody until the producer's next lap reaches it, at which point it is Reusable again |

**Claimed is not observable by the consumer**, and that is the point of the
release store: a slot is Claimed only between the payload write and the cursor
advance, and the consumer's acquire load of the producer cursor cannot return
a value that includes a Claimed slot.

#### The published region is a contiguous prefix

This is the property the whole crate turns on:

> For every slot, `Published` holds if and only if its sequence lies in
> `[ consumer, producer )`. There is no sequence in that range that is not
> published, and none outside it that is.

In `ring_mpsc` the equivalent statement is false. Producer 2 may publish
sequence 7 while producer 1 still holds an unpublished claim on 6, so the
published *set* is `{ 7 }` while the contiguous published *prefix* is empty —
and only the prefix is safe to drain. That gap is exactly what the sibling's
per-slot stamps exist to detect, and exactly why its consumer must establish
the prefix rather than read a cursor.

With one producer, publication is issued in sequence order by a single thread,
so a gap cannot open. The consumer reads a cursor instead of establishing a
prefix — one acquire load instead of a scan
(→ [Single-Consumer Drain to the Published Bound](../algorithm/002_single_consumer_drain.md)).

### Transitions

| # | From → To | Trigger | Writer | Ordering |
|---|-----------|---------|--------|----------|
| T1 | Reusable → Claimed | Producer begins writing slot `producer & (CAPACITY-1)` | Producer | None — the slot is exclusively the producer's |
| T2 | Claimed → Published | Producer advances its cursor past `S` | Producer | **Release** — makes the payload write visible |
| T3 | Published → Drained | Consumer advances its cursor past `S` | Consumer | **Release** — makes the slot reusable |
| T4 | Drained → Reusable | The producer's cursor comes round again, a full lap later | **Nobody** | None |

**T4 has no writer at all**, and this is worth stating because it is where a
reader expects to find code and finds none. Nothing marks a drained slot as
reusable; it simply is, once the producer's cursor has advanced a full lap and
the consumer's cursor has passed it. The "transition" is a change in which
comparison holds, not an event. A ring implementation that adds a reset step
here has added work with no purpose — and, worse, a window during which the
slot is in neither state.

**T2 and T3 are the only synchronizing transitions**, one release store each.
T1 and T4 are free.

### Behavioral Invariants

1. **A slot is writable by exactly one party at a time.** Reusable and Claimed
   belong to the producer; Published belongs to the consumer read-only; Drained
   belongs to nobody. There is no state in which both threads may touch a slot,
   and this is what makes the plain (non-atomic) slot accesses sound.

2. **The published region is contiguous** — the prefix property above. This is
   the invariant that eliminates per-slot stamps, and it is the one that fails
   first if the single-producer precondition is violated
   (→ [Exactly One Producer, Exactly One Consumer](../invariant/001_exactly_one_producer_one_consumer.md)).

3. **T2 cannot be observed before the payload write.** The release store in T2
   and the consumer's acquire load of the producer cursor form the pair. A
   relaxed store here would let the consumer read a slot whose payload write
   has not landed — the classic publication bug, and one that is invisible on
   x86-64 and reproducible on AArch64.

4. **A slot cannot go from Published back to Claimed.** The producer never
   revisits a slot it has published within a lap. Absent this, the borrowed-
   batch shape in [Consumer Surface](../api/002_consumer_surface.md) would be
   unsound even before the commit.

5. **Lap identity is carried by the sequence, not by the slot.** Two occupants
   of index *i* one lap apart are distinguished only by their sequence numbers
   `S` and `S + CAPACITY`. The slot itself has no memory of which lap it is
   in — which is what makes T4 free, and what makes the never-wrapping
   sequence load-bearing rather than decorative.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_uncontended_claim_and_publish.md](../algorithm/001_uncontended_claim_and_publish.md) | Performs T1 and T2 |
| [../algorithm/002_single_consumer_drain.md](../algorithm/002_single_consumer_drain.md) | Performs T3; its bound is invariant 2 |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_consumer_surface.md](../api/002_consumer_surface.md) | Its borrowed-batch hazard is the Published → Drained boundary of T3 |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_two_cursor_ring.md](../data_structure/001_two_cursor_ring.md) | Records that none of these states is stored |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_exactly_one_producer_one_consumer.md](../invariant/001_exactly_one_producer_one_consumer.md) | What invariant 2 depends on |

### State Machines

| File | Relationship |
|------|--------------|
| [004_ring_occupancy.md](004_ring_occupancy.md) | The ring-wide view of the same two cursors |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_producer_cursor.md](../type/001_producer_cursor.md) | The value T2 advances |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_types/readme.md`](../../../ring_types/readme.md) | The never-wrapping sequence type invariant 5 relies on |

### Tests

| File | Relationship |
|------|--------------|
| `tests/spsc_test.rs` | `available_and_is_empty_agree_at_every_point_of_a_lap` and `get_and_iter_agree_at_every_offset` — invariant 2 at each step of a scripted run: what `available()` reports is exactly what `get`/`iter` will yield, and nothing outside the range is reachable through either |
| `tests/spsc_test.rs` | `exhaustive::a_published_record_is_never_observed_before_the_write_that_preceded_it` — invariant 3, under `RUSTFLAGS="--cfg loom"`. The assertion is over a loom atomic stored before the publish rather than over the slot payload itself: loom does not instrument the plain memory behind the ring's `UnsafeCell`, and the first version of this model passed under a `Relaxed` publish for exactly that reason (→ `tests/manual/readme.md` S9) |

### SP34 — Sequences Are Consecutive Across a Wrap

The title of this instance is a claim about the sequence space, and the wrap is
where it would break: a mask fold that lost a step would produce a gap invisible
to any test that never fills the ring twice.

Its companion `a_batch_spanning_a_wrap_reads_the_right_slots` covers the read
side of the same boundary.

### SP35 — A Record Taken Through `get_mut` Leaves Its Slot Empty Across a Wrap

`a_record_taken_through_get_mut_leaves_its_slot_empty_across_a_wrap` is the
test. Taking rather than copying is what makes
`every_record_written_is_dropped_exactly_once` hold without depending on drop
order at teardown.

**Checked at the wrap rather than in the middle of a lap**, because a slot that
kept its payload would be overwritten harmlessly within one lap and would
double-drop only on reuse.
