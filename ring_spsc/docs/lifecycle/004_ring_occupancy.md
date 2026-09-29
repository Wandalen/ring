# Lifecycle: Ring Occupancy Between the Cursors

### Scope

- **Purpose**: Give the ring as a whole its states, and record which of them a thread can act on with confidence — a question that has a different answer here than in the multi-producer ring.
- **Responsibility**: The states, the transitions that move between them, and the observability invariants each thread has over them.
- **In Scope**: The ring-wide occupancy states; who can observe each reliably.
- **Out of Scope**: An individual slot's states (→ [Slot State Without Holes](003_slot_state_without_holes.md)); what a thread should *do* on Full or Empty, which is [`ring_overflow`](../../../ring_overflow/readme.md)'s and [`ring_wait`](../../../ring_wait/readme.md)'s.

### States

The ring's state is one derived quantity: `D = producer_cursor - consumer_cursor`,
the count of published-but-undrained records. `D` is always in `0..=CAPACITY`.

| State | Holds when | Producer may publish | Consumer may drain |
|-------|-----------|----------------------|--------------------|
| **Empty** | `D == 0` | Yes | No — nothing to read |
| **Occupied** | `0 < D < CAPACITY` | Yes | Yes |
| **Full** | `D == CAPACITY` | No — would overwrite an undrained slot | Yes |

Nothing stores this state either. `D` is a subtraction of two values each of
which has a single writer.

**`D` cannot exceed `CAPACITY`**, and that is the bound the producer's free-space
check enforces (→ [Uncontended Claim and Publish](../algorithm/001_uncontended_claim_and_publish.md)'s
step 3). Exceeding it is not a full ring — it is the producer having lapped the
consumer and overwritten unread data. Preventing that in general is
`ring_gating`'s problem to solve; here the "gate" is one comparison against
one cursor, which is why this crate needs no gating dependency (→ [Family Dependency Seam](../integration/001_family_dependency_seam.md)).

### Transitions

| # | From → To | Trigger | Effect on `D` |
|---|-----------|---------|---------------|
| U1 | Empty → Occupied | Producer publishes | `D: 0 → 1` |
| U2 | Occupied → Occupied | Either end acts | `D` ± 1, or `D` − *n* on a batch drain |
| U3 | Occupied → Full | Producer publishes the `CAPACITY`-th undrained record | `D → CAPACITY` |
| U4 | Full → Occupied | Consumer drains | `D` − *n* |
| U5 | Occupied → Empty | Consumer drains everything available | `D → 0` |
| U6 | Full → Empty | Consumer drains a full ring in one batch | `D: CAPACITY → 0` |

**U6 is a single transition, not a walk through Occupied.** The drain's commit
is one release store covering the whole batch, so the ring goes from Full to
Empty in one observable step. Any reasoning that assumes intermediate states
are visible — a watcher sampling `D` expecting to see it decrease gradually —
is wrong about this ring.

### Behavioral Invariants

1. **`D` is bounded by `CAPACITY` and never negative.** `producer >= consumer`
   always, because the consumer never advances past a sequence the producer has
   not published (→ [Slot State Without Holes](003_slot_state_without_holes.md)'s
   invariant 2), and the producer never advances past `consumer + CAPACITY`.

2. **Each cursor moves in one direction only.** Both are monotonically
   non-decreasing for the ring's whole life; neither is ever reset, decremented,
   or wrapped. What wraps is the derived slot index. Reset is
   [`ring_shutdown`](../../../ring_shutdown/readme.md)'s operation, and it
   constructs a fresh state rather than rewinding this one.

3. **Full is observable and actionable by the producer** — and this is the
   sharp difference from the multi-producer ring. There, a producer that reads
   `D < CAPACITY` may still lose the race to another producer before it claims,
   so the observation is advisory. Here the producer is the only thread that
   can increase `D`, so an observation of "not Full" is still true when it
   acts on it (→ [Free Capacity](../type/002_free_capacity.md)).

4. **Empty is *not* symmetrically actionable by the consumer.** An observation
   of `D == 0` can become stale immediately — the producer may publish a
   nanosecond later. The asymmetry is exact and worth stating: the producer
   can rely on a negative observation (not Full) because it alone increases
   `D`; the consumer cannot rely on `D == 0` persisting, only on `D >= n`
   remaining true, since nothing decreases `D` but itself.

5. **Both threads always observe a `D` that was real at some instant**, never a
   torn value. Each cursor is a single aligned word with one writer; a
   subtraction of two such reads yields a value that held at some point between
   the two loads, even if not at either.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_uncontended_claim_and_publish.md](../algorithm/001_uncontended_claim_and_publish.md) | Its step 3 tests for Full; invariant 3 is why the test binds |
| [../algorithm/002_single_consumer_drain.md](../algorithm/002_single_consumer_drain.md) | Its step 3 tests for Empty; invariant 4 bounds what that answer means |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_producer_surface.md](../api/001_producer_surface.md) | `is_full()` and `free_capacity()` expose this state |
| [../api/002_consumer_surface.md](../api/002_consumer_surface.md) | `is_empty()` and `available()` expose it from the other side |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_two_cursor_ring.md](../data_structure/001_two_cursor_ring.md) | The two cursors `D` is derived from |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_family_dependency_seam.md](../integration/001_family_dependency_seam.md) | Why the `CAPACITY` bound needs no `ring_gating` dependency here |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_exactly_one_producer_one_consumer.md](../invariant/001_exactly_one_producer_one_consumer.md) | What invariants 3 and 4's asymmetry rests on |

### State Machines

| File | Relationship |
|------|--------------|
| [003_slot_state_without_holes.md](003_slot_state_without_holes.md) | The per-slot view; its invariant 2 supports invariant 1 here |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_free_capacity.md](../type/002_free_capacity.md) | `CAPACITY - D`, and the binding-versus-advisory distinction invariant 3 makes |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_gating/readme.md`](../../../ring_gating/readme.md) | The general lapping problem invariant 1 solves with one comparison in this configuration |
| [`ring_shutdown/readme.md`](../../../ring_shutdown/readme.md) | Reset, which invariant 2 declines to model as a transition here |

### Tests

| File | Relationship |
|------|--------------|
| `tests/spsc_test.rs` | `a_ring_smaller_than_the_traffic_still_loses_nothing` — invariant 1 at capacity 2 against 20 000 records, which is the same pressure as a deliberately slow consumer and reaches saturation on almost every push. `exhaustive::the_consumer_never_sees_further_than_the_producer_published` checks the same bound over every interleaving of a two-record case |
| `tests/spsc_test.rs` | `free_capacity_is_actionable_rather_than_advisory` — invariant 3, which does not hold in `ring_mpsc`: `n` successful pushes and then exactly one failure, with no intervening drain |

### SP36 — Occupancy and Emptiness Are Pinned to Agree Around a Whole Lap

`is_empty` could be written as `available() == 0` and is not; both derive from
the cursor pair independently. The test walks a full lap and compares them at
every step, which is the only way an off-by-one in one derivation shows up —
each is self-consistent in isolation.

`a_new_ring_is_empty_and_fully_free` covers the initial state, where both
derivations read a cursor pair that has never moved.
