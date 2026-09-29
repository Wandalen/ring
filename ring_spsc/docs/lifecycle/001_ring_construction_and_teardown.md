# Lifecycle: Ring Construction and Teardown

### Scope

- **Purpose**: Give the ring's phases from allocation to drop, and record the two decisions at its ends that are genuinely open — what a fresh ring's cursors start at, and what happens to undrained records when it dies.
- **Responsibility**: The phases, the transitions, the ordering dependencies, and the cleanup obligations.
- **In Scope**: The ring value's own life; the allocation; the drop.
- **Out of Scope**: The producer/consumer pairing nested inside it (→ [Producer and Consumer Pairing](002_producer_consumer_pairing.md)); `close`/`reset`/`drain_all`, which are [`ring_shutdown`](../../../ring_shutdown/readme.md)'s.

### Lifecycle Phases

| # | Phase | Begins | Ends | Invariants held |
|---|-------|--------|------|-----------------|
| P1 | **Unconstructed** | — | `Capacity` validated | None |
| P2 | **Allocated** | Slot storage obtained, cursors initialized | The `( Producer, Consumer )` pair is handed out | Cursors equal; ring Empty |
| P3 | **Live** | Either end is first used | The last end is dropped | All of [`invariant/`](../invariant/readme.md) |
| P4 | **Dropped** | The last end goes out of scope | Storage released | None — nothing may touch the slots |

**P1 → P2 is where `Capacity`'s power-of-two rule is enforced**, and it is the
only validation in the whole lifecycle. This crate's own reached-test states it
as a construction-time rejection: a non-power-of-two capacity is rejected at
construction. Enforcing it here rather than at each addressing site is what
makes step 4 of the publish a bare mask with no branch
(→ [Uncontended Claim and Publish](../algorithm/001_uncontended_claim_and_publish.md)).

**P2 allocates exactly once.** This crate's own reached-test asserts it of the
underlying storage — allocating exactly `N` slots once — and nothing in P3
allocates at all. A ring that grew would invalidate every borrowed batch
outstanding and would move the allocation onto the hot path; fixed capacity is
what [`ring_overflow`](../../../ring_overflow/readme.md)'s policies exist to
handle instead.

#### The cursor initialization question

Both cursors start equal, so the ring reads as Empty. **What value they start
*at* is open, and it is not cosmetic:**

- **Both zero** is the obvious choice and makes `producer - consumer == 0`
  trivially. It also means sequence 0 is a real, publishable sequence, so any
  code using 0 as a sentinel for "no sequence" is wrong from the first record.
- **Both at `CAPACITY`** costs nothing, keeps `D == 0`, and leaves the whole
  range below `CAPACITY` unreachable — so a stale or default-initialized
  sequence value cannot be mistaken for a live one.

The sibling crate records a concrete initialization hazard at exactly this
point: with per-slot stamps, `vec![ 0; capacity ]` makes slot 0 read as already
published. **This crate cannot have that bug**, because it has no stamps
(→ [Two-Cursor Ring Without Per-Slot State](../data_structure/001_two_cursor_ring.md))
— which is worth recording as a positive consequence of the simpler structure
rather than leaving the reader to infer it. The remaining question here is only
about sentinel values, not about correctness of the empty state.

**Decided: both at `Seq::ZERO`**, which is the first of the two options rather
than the `CAPACITY` one — and the reasoning above is what changed, not the
conclusion it was heading for.

The `CAPACITY` option's advantage was that it makes a stale or default-zero
sequence unmistakable for a live one. That advantage is real, and it is bought
elsewhere here: `Seq` is a distinct newtype from `ring_types`, not a bare
`u64`, so a default-initialized integer cannot be passed where a sequence is
expected in the first place. Paying a second time — by making `D == 0` mean
"both at 1024" and every arithmetic argument in these docs carry an offset —
would buy nothing and cost every reader.

`Producer::position()` and `Consumer::position()` both return `Seq( 0 )` on a
fresh ring, and `a_new_ring_is_empty_and_fully_free` asserts it.

### Phase Transitions

| # | Transition | Trigger | May fail | Ordering constraint |
|---|-----------|---------|----------|---------------------|
| L1 | P1 → P2 | `Factory::build( cfg )` | Yes — invalid `Capacity` | Must complete before either end exists |
| L2 | P2 → P3 | First use of either end | No | — |
| L3 | P3 → P3 | Publishes and drains | No | The whole of [`algorithm/`](../algorithm/readme.md) |
| L4 | P3 → P4 | The last of the two ends is dropped | No | **Both** ends must be gone — see Cleanup |

**L1 belongs to [`ring_factory`](../../../ring_factory/readme.md), not to this
crate.** `RingConfig` is the only constructor input, and `Factory::build( cfg )`
is the constructor, returning a handle pair. This crate
supplies the ring; it does not supply the way to obtain one, which is what
keeps `ring_spsc` off the export list while its behaviour is still reachable
(→ [Reached Through the Export Surface](../integration/002_reached_through_the_export_surface.md)).

**L4's trigger is not "the ring is dropped" but "the second end is dropped"**,
because the two ends are what a consumer holds. Which end goes first is not
determined, and both orders must be safe.

### Dependencies

| Depends on | For | Phase |
|-----------|-----|-------|
| [`ring_config`](../../../ring_config/readme.md) | `RingConfig` — capacity, wait kind, overflow policy, batch size | P1 → P2 |
| [`ring_store`](../../../ring_store/readme.md) | The one-time slot allocation | P2 |
| [`ring_cursor`](../../../ring_cursor/readme.md), [`ring_align`](../../../ring_align/readme.md) | The padded cursors, and their 64-byte separation | P2 |
| [`ring_slot`](../../../ring_slot/readme.md) | Typed or bytes translation into a slot | P3 |
| [`ring_factory`](../../../ring_factory/readme.md) | L1 itself — the construction entry point | P1 → P2 |

**`ring_factory` is a dependency of this crate's lifecycle but not of this
crate.** The arrow runs the other way in `Cargo.toml`: `ring_factory` depends
on `ring_core`, which composes this one. That inversion is deliberate — the
constructor lives above so that the export surface stays five crates wide.

### Cleanup Requirements

1. **Both ends must be dropped before the storage is released** — and as
   built, that is a borrow-checker fact rather than a rule anyone has to
   follow. **Decided: neither end owns the storage and there is no `Arc`.**
   `split` takes `&mut self` and hands out two `&Ring` reborrows, so the ring
   outlives both ends by construction and L4 is simply the ring going out of
   scope. Drop order between the ends cannot be load-bearing because neither
   end has a destructor at all — clippy's `drop_non_drop` is what established
   that, by rejecting a test that tried to order two non-events.
   `the_ends_going_out_of_scope_in_either_order_releases_the_storage_once`
   asserts both orders anyway, since "cannot matter" is a claim worth a test.

2. **Undrained records at drop are dropped, not leaked.** **Decided**, and it
   follows from the storage shape rather than from a policy: `Buffer< S >` is a
   `Box< [ S ] >`, so releasing it runs each slot's destructor, and a
   `TypedSlot` holding a record drops that record.
   `every_record_written_is_dropped_exactly_once` measures it — eight records
   written over two laps, eight destructors run, no more and no fewer, with the
   second lap's overwrites accounted for separately from the teardown's.

   The cost that made this an open question is real and is now simply paid: the
   thread that drops the ring runs whatever destructors the undrained records
   carry, and it need not be the thread that created them. That is the first
   row of the table below, chosen over the other two:

   | Option | Cost |
   |--------|------|
   | Drop them | The dropping thread runs arbitrary user destructors at teardown, possibly not the thread that created them |
   | Leak them | Silent resource leak proportional to how far behind the consumer was |
   | Refuse to drop a non-empty ring | Turns a leak into a panic in a destructor, which is worse |

   Leaking was rejected because a silent leak proportional to consumer lag is
   the failure mode hardest to attribute back here; refusing to drop was
   rejected because a panic in a destructor is worse than either.

   [`ring_shutdown`](../../../ring_shutdown/readme.md)'s `drain_all()` —
   returning exactly the items outstanding at close — remains the orderly
   path, and it is now an optimisation rather than a necessity — the drop
   path is correct when nobody calls it.

3. **Nothing may touch a slot after P4 begins.** A borrowed batch outliving
   the ring is a use-after-free; the guard shape in
   [Consumer Surface](../api/002_consumer_surface.md) is what makes it
   unrepresentable, and it is the same lifetime argument as the commit hazard
   there, extended to teardown. As built the `Batch` borrows the `Consumer`,
   which reborrows the `Ring`, so the chain is enforced by the borrow checker
   and asserted by a `compile_fail` block in the crate's module documentation.

4. **No cleanup is required in P3.** Nothing is allocated during Live, so
   nothing accumulates. This is the property that makes a long-running ring's
   memory profile flat, and it follows from P2's single allocation.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_uncontended_claim_and_publish.md](../algorithm/001_uncontended_claim_and_publish.md) | The L3 operation whose mask step P1's validation enables |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_consumer_surface.md](../api/002_consumer_surface.md) | Cleanup 3's lifetime argument, in its commit-hazard form |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_two_cursor_ring.md](../data_structure/001_two_cursor_ring.md) | The fields P2 initializes, and the absent stamps that spare this crate the sibling's initialization hazard |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_family_dependency_seam.md](../integration/001_family_dependency_seam.md) | The Dependencies table's crates, and the inverted `ring_factory` arrow |
| [../integration/002_reached_through_the_export_surface.md](../integration/002_reached_through_the_export_surface.md) | Why L1 lives above this crate |

### Lifecycles

| File | Relationship |
|------|--------------|
| [002_producer_consumer_pairing.md](002_producer_consumer_pairing.md) | The shorter cycle nested inside P3 |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/004_ring_occupancy.md](../lifecycle/004_ring_occupancy.md) | The Empty state P2 establishes |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_producer_cursor.md](../type/001_producer_cursor.md) | The value the initialization question is about |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_store/readme.md`](../../../ring_store/readme.md) | P2's allocate-exactly-once contract |
| [`ring_factory/readme.md`](../../../ring_factory/readme.md) | L1's owner, and `RingConfig` as the only constructor input |
| [`ring_shutdown/readme.md`](../../../ring_shutdown/readme.md) | The orderly teardown Cleanup 2 defers to |

### Tests

| File | Relationship |
|------|--------------|
| `tests/spsc_test.rs` | `with_config_takes_the_capacity_and_ignores_the_rest` — this crate's own reached-test does not need restating here, because an invalid capacity is *unrepresentable* at this crate's construction point: `Ring::new` takes a `Capacity`, and `Capacity::new` is where the power-of-two check lives ([`ring_types`](../../../ring_types/readme.md)). The stronger property is what this test asserts instead — that `with_config` reads only the capacity and silently honours no other field |
| `tests/spsc_test.rs` | `every_record_written_is_dropped_exactly_once` and `the_ends_going_out_of_scope_in_either_order_releases_the_storage_once` — Cleanup 1 and 2, with a payload type that records its own destruction. Two laps, so the overwrite path is exercised: replacing an occupied slot must drop what it replaces |

### SP31 — Teardown Releases the Storage Once Regardless of Drop Order

Both ends borrow the ring, so neither owns the allocation and neither frees it —
the `Ring` does, when it goes out of scope after both. The test exists because
the property is invisible: nothing observable differs between "freed once" and
"freed once, by luck of this drop order".

Its companion `every_record_written_is_dropped_exactly_once` covers the payloads
rather than the allocation, which is the half a leak detector would not catch.

### SP32 — `with_config` Reads One Field, Same as the Sibling

The two composed cores agree exactly here, and both agree for the same reason:
policy belongs to the layer that chose the backend, not to the backend.

**Unlike the sibling's, this crate's `with_config` does have a caller** —
`ring_core::Storage::Spsc` constructs through it, and so does `ring_bench`. It is
one of the few methods in either crate reached from two directions.
