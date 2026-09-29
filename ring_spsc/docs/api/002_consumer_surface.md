# API: Consumer Surface

### Scope

- **Purpose**: State the single consumer's caller-facing surface, and establish that its batch shape is forced by the commit's amortization rather than chosen for convenience.
- **Responsibility**: The operations, the borrow-versus-copy decision that determines whether the drain is zero-copy, the error surface, and the compatibility position.
- **In Scope**: The consumer end's operations; the batch shape; the lifetime question.
- **Out of Scope**: The producer end (→ [Producer Surface](001_producer_surface.md)); the procedure behind the surface (→ [Single-Consumer Drain to the Published Bound](../algorithm/002_single_consumer_drain.md)); what the consumer does with drained records.

### Abstract

One consumer takes everything the producer has published since the last drain.
The surface is **batch-shaped, and this is forced rather than chosen**: the
commit is one release store regardless of how many records it covers
(→ [Single-Consumer Drain to the Published Bound](../algorithm/002_single_consumer_drain.md)'s
step 5), so an item-at-a-time API would pay per record for synchronization the
batch API pays once.

An item-shaped `pop()` can be *built* on the batch, but it cannot be the
primitive without giving up the amortization that is the drain's whole cost
advantage.

### Operations

| Operation | Shape | Cost | Blocks? |
|-----------|-------|------|---------|
| `drain()` | `Batch< '_, T >` or `Vec< T >` | Acquire load, subtract, N reads, release store | No |
| `available()` | `usize` | Acquire load, subtract | No |
| `is_empty()` | `bool` | Acquire load, compare | No |

**`available()` costs the same as `drain()`'s bound computation, and this is
worth stating plainly:** both are one acquire load and a subtraction. A caller
polling `available()` before deciding whether to drain has paid for the drain's
expensive step and then paid it again. `is_empty()` is the cheap poll — it
compares rather than subtracts, and it is the operation a tick loop should
use.

**The borrow-versus-copy question decides whether the drain is zero-copy, and
it is a correctness question rather than an ergonomics one.**

| Shape | Zero-copy | Hazard |
|-------|-----------|--------|
| `drain()` yields `&[ T ]` borrowed from the slots | Yes | The slots become reusable the instant the commit lands. If the commit is inside `drain()`, the returned borrow points at memory the producer may overwrite |
| `drain()` yields a guard that commits on drop | Yes | Correct — the borrow cannot outlive the commit, because the commit *is* the drop. Costs a `Drop` impl and a lifetime on the return type |
| `drain()` copies out into a `Vec< T >` | No | Correct and simple; pays a copy per record, which for the payloads this family targets is the cost the design exists to avoid |

**The second shape is the only one that is both zero-copy and sound**, and
naming that is the point of this instance. The first is the shape a reader
reaches for and it is unsound; the third is sound and gives up the property
the crate is measured on.

**Chosen: the second.** `drain()` returns a `Batch` that commits on drop, and
the borrow cannot outlive it because the commit *is* the drop — asserted as a
`compile_fail` doc test in the crate's module documentation, under "What the
type system refuses", rather than left as prose. `drain_up_to( max )` is the
same shape with a bound, for a caller working to a frame budget.

### Error Handling

| Condition | Surfaces as | Caller's options |
|-----------|-------------|------------------|
| Ring empty | An empty batch, or `available() == 0` | Return and retry next tick. The configured [`ring_wait`](../../../ring_wait/readme.md) strategy decides; `WaitKind::None` returns immediately |
| A second consumer exists | **Nothing** — silent data race, and records are lost rather than duplicated | None at runtime (→ [Exactly One Producer, Exactly One Consumer](../invariant/001_exactly_one_producer_one_consumer.md)) |
| Borrow held past the commit | **Unrepresentable** — the borrow's lifetime is the guard's | None needed. This is why the guard shape was chosen |

**Rows two and three are both silent, and both are unrepresentable rather than
checkable.** No runtime check can distinguish a second consumer from the
first, and no runtime check can tell whether a borrow is still alive. Each has
exactly one remedy and it is structural: the handle split for row two,
the commit-on-drop guard for row three.

**Nothing blocks.** An empty ring returns an empty batch; it does not park.
That is what the family requires of everything reachable from the tick path,
enforced at [`ring_poll`](../../../ring_poll/readme.md) and
[`ring_handle`](../../../ring_handle/readme.md) rather than here.

### Compatibility Guarantees

1. **This surface is internal.** A change reaches `ring_core`
   and `ring_handle` and stops, which is what makes the borrow-versus-copy
   question affordable to leave open.
2. **The batch shape is not negotiable downward.** An item-shaped primitive
   would move the release store onto the per-record path and invalidate the
   drain's cost profile; `pop()` may be added *over* `drain()`, never under it.
3. **The surface must remain expressible by the crossbeam backend**,
   whose own queue does not offer a borrowed-batch drain. If
   the guard shape is chosen, the swap has to reconcile that
   (→ [Family Dependency Seam](../integration/001_family_dependency_seam.md)).
4. **Version movement is lockstep.**

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_single_consumer_drain.md](../algorithm/002_single_consumer_drain.md) | The procedure behind every operation; its step 5 is why the shape is batched |

### APIs

| File | Relationship |
|------|--------------|
| [001_producer_surface.md](001_producer_surface.md) | The other end; this end's commit frees the capacity that end reports |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_two_cursor_ring.md](../data_structure/001_two_cursor_ring.md) | The slots a borrowed batch points into |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_family_dependency_seam.md](../integration/001_family_dependency_seam.md) | Guarantee 3's swap constraint |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_exactly_one_producer_one_consumer.md](../invariant/001_exactly_one_producer_one_consumer.md) | The Error Handling table's second row |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_byte_parity_over_one_hundred_thousand.md](../non_functional_requirement/002_byte_parity_over_one_hundred_thousand.md) | The in-order, zero-loss condition this end must deliver |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/004_ring_occupancy.md](../lifecycle/004_ring_occupancy.md) | The Empty state `is_empty()` detects |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_free_capacity.md](../type/002_free_capacity.md) | The complement of `available()` |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_wait/readme.md`](../../../ring_wait/readme.md) | The wait strategy an empty ring hands control to |

### Tests

| File | Relationship |
|------|--------------|
| `tests/spsc_test.rs` | `one_producer_and_one_consumer_exchange_one_hundred_thousand_items` — items read back in issue order, zero loss |
| `tests/spsc_test.rs` | `a_batch_commits_exactly_its_own_length`, `drain_up_to_zero_is_a_legitimate_no_op`, `a_batch_spanning_a_wrap_reads_the_right_slots` |
| `src/lib.rs` | The `compile_fail` doc test asserting a drained record cannot outlive its commit — in the library, because rustdoc collects doc tests from the library target only |
| `tests/manual/readme.md` | S4 — that each `compile_fail` block fails for its own reason rather than incidentally |

### SP7 — A `Batch` Commits Its Own Length, Not What Was Read

The commit is the batch's length on drop, regardless of how many `get` calls the
caller made. That is the right design — a partial commit would leave the ring in
a state where the read cursor sits inside a batch nobody holds — and it means
`drain()` is a claim on records, not an offer of them.

**A caller that wants to take fewer must ask for fewer**, via `drain_up_to`.
Reading three of ten and dropping the batch discards seven records that were
never read and cannot be re-drained.

### SP8 — `get` and `iter` Are Pinned to Agree

Two accessors over one range is a duplication the type could not avoid — an
iterator is the ergonomic path and indexed access is the random one — so the
test is what keeps them one implementation in behaviour if not in code.

The failure it guards is off-by-one in one path only, which no throughput test
would surface: both paths would still return records, just different ones.
