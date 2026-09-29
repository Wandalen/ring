# Type: Free Capacity

### Scope

- **Purpose**: Define the derived free-space quantity, and establish the property that makes it a *binding guarantee* here and only an advisory hint in the multi-producer ring.
- **Responsibility**: The definition, the validation rules, and the precise reason the same arithmetic carries a stronger contract under single-producer.
- **In Scope**: The derived value; its staleness semantics; the asymmetry between the two ends.
- **Out of Scope**: What to do when it reaches zero, which is [`ring_overflow`](../../../ring_overflow/readme.md)'s; the cursors it is derived from (→ [Producer Cursor](001_producer_cursor.md)).

### Definition

The number of records the producer may publish before the ring is full:

```text
free_capacity = CAPACITY - ( producer_cursor - consumer_cursor )
```

A derived quantity — nothing stores it. It is a subtraction and a subtraction,
computed from a plain load of the producer's own cursor and an acquire load of
the consumer's.

**Its value is a lower bound that only ever improves.** The consumer can drain
concurrently, making more space; nothing can make less, because the producer
asking is the only thread that consumes space. So a reported value of *n* means
"at least *n*", and the true figure at any later instant is `>= n` until this
producer itself publishes.

**That one-sided staleness is the entire difference from `ring_mpsc`, and it
converts a hint into a guarantee:**

| | `ring_spsc` | `ring_mpsc` |
|---|---|---|
| Who can decrease free capacity | This producer only | Any of N producers |
| A reported `n` means | **At least `n`** — the next `n` publishes will succeed | **At most `n`** — another producer may take it first |
| Usable for | Deciding to publish a batch of `n` without re-checking | A heuristic only; every publish must still handle `Full` |
| Cost of trusting it | None | A lost race, surfacing as an unexpected `Full` |

**Same expression, same two loads, opposite direction of error.** A caller that
learns the SPSC contract and carries it to an MPSC ring has written code that
is wrong in a way no signature reveals
(→ [SPSC Correctness Does Not Transfer](../pitfall/001_spsc_correctness_does_not_transfer.md)).

**The consumer's mirror quantity does not gain the same strength**, and the
asymmetry is exact. `available = producer - consumer` is a lower bound for the
consumer too — the producer can only add — so `available >= n` stays true. But
`available == 0` is *not* stable: the producer may publish immediately after.
So "at least `n` are readable" is reliable while "there is nothing to read" is
not (→ [Ring Occupancy Between the Cursors](../lifecycle/004_ring_occupancy.md)'s
invariants 3 and 4).

### Validation

| # | Rule | Enforced by | Status |
|---|------|-------------|--------|
| V1 | Always in `0..=CAPACITY` | Follows from `producer >= consumer` and `producer - consumer <= CAPACITY` | Structural (→ [Ring Occupancy Between the Cursors](../lifecycle/004_ring_occupancy.md) invariant 1) |
| V2 | Computed by subtraction on never-wrapping cursors, never by comparing derived slot indices | The cursors are `u64` sequences, not indices | Structural — comparing masked indices instead is the classic ring bug, since it cannot distinguish empty from full |
| V3 | The reported value is a lower bound valid until this producer publishes | Single-producer cardinality | **Depends on an unenforceable precondition** (→ [Exactly One Producer, Exactly One Consumer](../invariant/001_exactly_one_producer_one_consumer.md)) |
| V4 | Reading it costs one acquire load and one subtraction — the same as the publish path's own check | The definition | Structural, and the reason polling it before publishing is wasted work (→ [Producer Surface](../api/001_producer_surface.md)) |
| V5 | Zero means Full, and Full is actionable — not a race | V3 | Holds here; **does not hold in `ring_mpsc`** |

**V2 is the rule that prevents the oldest ring bug there is.** If free capacity
were computed from masked slot indices rather than raw sequences, `producer`
and `consumer` would compare equal both when the ring is empty and when it is
exactly one lap full — indistinguishable states with opposite meanings. Keeping
the cursors as unbounded sequences and masking only at the point of addressing
is what makes the two cases distinct — the sequence itself must never wrap,
while the derived index does, by design.

**V3 and V5 are the rules a reader is most likely to carry out of this crate
incorrectly**, because they are stated as properties of an expression that
looks identical in both rings.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_uncontended_claim_and_publish.md](../algorithm/001_uncontended_claim_and_publish.md) | Its step 3 computes this; V3 is why the answer is still true when acted on |
| [../algorithm/002_single_consumer_drain.md](../algorithm/002_single_consumer_drain.md) | Computes the complement in its step 3 |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_producer_surface.md](../api/001_producer_surface.md) | Exposes it as `free_capacity()`; V4 is why `is_full()` is the cheap poll |
| [../api/002_consumer_surface.md](../api/002_consumer_surface.md) | The mirror quantity, and the asymmetry that does not carry over |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_two_cursor_ring.md](../data_structure/001_two_cursor_ring.md) | Lists it among the derived-not-stored quantities |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_exactly_one_producer_one_consumer.md](../invariant/001_exactly_one_producer_one_consumer.md) | V3's unenforceable precondition |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_spsc_correctness_does_not_transfer.md](../pitfall/001_spsc_correctness_does_not_transfer.md) | V3 and V5 are its central worked example |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/004_ring_occupancy.md](../lifecycle/004_ring_occupancy.md) | V1's bound and V5's Full state |

### Types

| File | Relationship |
|------|--------------|
| [001_producer_cursor.md](001_producer_cursor.md) | One of the two values this is derived from |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_types/readme.md`](../../../ring_types/readme.md) | The sequence/slot-index split V2 depends on |

### Tests

| File | Relationship |
|------|--------------|
| `tests/spsc_test.rs` | `free_capacity_is_actionable_rather_than_advisory` — V3 and V5, the properties that distinguish this crate from `ring_mpsc`. The `n + 1`-th push failing is asserted too; a bound that is merely *sufficient* would pass the first half alone |
| `tests/spsc_test.rs` | `available_and_is_empty_agree_at_every_point_of_a_lap` — V2, the bug this rule exists to prevent. Monotonic `Seq` rather than masked indices is what makes the two distinguishable, and the test walks a full lap to show it |

### SP49 — Here `free_capacity` Is Exact, and the Family's Only Caller Reads It as Advisory

`free_capacity_is_actionable_rather_than_advisory` is this crate's test and its
name is the contract: with a single producer nothing can consume the headroom
between the read and the push.

`ring_core` exposes one `free_capacity` over both backends
(→ `ring_core`'s `decisions/002`), and at MPSC the same number is advisory —
another producer may take the slot first. Its four callers therefore treat every
result as advisory, which is correct for the union and wastes the guarantee here.

**The stronger contract exists, is tested, and has no consumer.** That is the
cost of the uniform signature, paid entirely by this crate.

**Disposition:** declined — this instance's own text traces the advisory
treatment to `ring_core`'s uniform `free_capacity` signature over both
backends (→ `ring_core`'s `decisions/002`); the fix is a design decision in
`ring_core`, not a defect in this crate's own source or
`type/002_free_capacity.md`.
