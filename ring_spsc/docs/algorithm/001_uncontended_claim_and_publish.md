# Algorithm: Uncontended Claim and Publish

### Scope

- **Purpose**: Specify the single producer's claim-then-publish procedure, and account for why every atomic read-modify-write the multi-producer path needs disappears here.
- **Responsibility**: The step sequence, the ordering each step requires, the one bound check, and the cost profile that makes this the family's cheapest correct write.
- **In Scope**: The producer side of one record; the ordering constraints; the free-space check.
- **Out of Scope**: The consumer's drain (→ [Single-Consumer Drain to the Published Bound](002_single_consumer_drain.md)); the contended variant (→ [`ring_mpsc`](../../../ring_mpsc/docs/algorithm/001_claim_then_publish.md)); the slot's byte layout, which is `ring_slot`'s.

### Abstract

One producer claims the next sequence, writes the slot, and publishes. With
exactly one producer the claim needs no atomic read-modify-write at all: the
producer cursor has a single writer, so a plain load of a value nobody else
can change is sufficient, and the store that advances it is a release store
rather than a `fetch_add`.

This crate is "the cheapest correct ring — no contention on the claim, no
minimum across cursors." Both halves of that phrase are structural, not
incidental:

| What MPSC needs | Why SPSC does not | Saved |
|-----------------|-------------------|-------|
| Atomic `fetch_add` on the producer cursor | Only one thread writes it | The RMW and its cache-line ownership transfer |
| A gating set and a minimum across it | There is one consumer, so the "minimum" is that one cursor's value | The whole [`ring_barrier`](../../../ring_barrier/readme.md)/[`ring_gating`](../../../ring_gating/readme.md) machinery |
| Gating fused *into* the claim atomically | Nothing else can move the producer cursor between the check and the claim | The fusion constraint — check and claim may be separate steps |
| Publication with holes | Publication is strictly sequential (→ [Slot State Without Holes](../lifecycle/003_slot_state_without_holes.md)) | Per-slot publication stamps as the readiness signal |

**The third row is the one that is easy to state wrongly.** It is not that
SPSC skips the bound check — a single producer that laps the consumer
overwrites unread data exactly as a multi-producer one would. It is that the
check need not be *atomic with respect to* the claim, because the only thread
that could invalidate it between the two steps is the producer itself.

### Algorithm

Publishing one record:

1. **Read the producer cursor** — a plain (non-atomic) load. This thread is
   its only writer, so no other thread can have changed it.
2. **Read the consumer cursor** — an `Acquire` load. This one *is* written by
   another thread, and the acquire pairs with the consumer's release in step 3
   of the drain, making that thread's slot reads visible before this thread
   reuses the slot.
3. **Check free space.** `producer - consumer < CAPACITY`. If false the ring
   is full; hand off to the configured [`ring_overflow`](../../../ring_overflow/readme.md)
   policy and stop. Unlike the multi-producer case this answer is still true
   when acted on (→ [Free Capacity](../type/002_free_capacity.md)).
4. **Derive the slot index** — `producer & (CAPACITY - 1)`, a mask rather than
   a division, which is what `Capacity`'s power-of-two validation buys
   (`ring_types`; this crate's own reached-test asserts the mask).
5. **Write the slot.** The full payload, through `ring_slot`'s typed or bytes
   translator. No other thread may observe this slot until step 6.
6. **Publish** — a `Release` store advancing the producer cursor by one. The
   release is what makes step 5's writes visible to a consumer that acquires
   the same cursor.

**Steps 1–4 are pure local computation on values this thread owns**, which is
why the cost of a publish is a load, a compare, a memcpy, and a release store
— no read-modify-write anywhere in the path. That is the concrete content of
[No Lock in the Path](../invariant/002_no_lock_in_the_path.md), and it is what
this crate's own reached-test means by "no lock in the path."

**Step 6 is the only synchronizing operation**, and it is the cheapest kind: a
release store, not a fence and not an RMW. On x86-64 a release store is an
ordinary `mov`; on AArch64 it is `stlr`. Neither takes exclusive ownership of
a cache line the way `fetch_add` does.

**Open at this grain, and closed at the next one.** Whether steps 5 and 6 are
exposed as two calls, a guard value, or one fused `push` is the surface
question (→ [Producer Surface](../api/001_producer_surface.md)), not an
algorithm question — the step sequence is identical under all three shapes,
which is why this instance did not need to wait for it. The answer, for a
reader following the chain: a guard (`claim()` returning a `Reservation` that
publishes on drop), with a fused `try_push`/`push_with` layered over it.
Whether this crate's numbers justify the family at all is a judgment made
elsewhere; this instance specifies what will be measured, not whether it wins.

### Algorithms

| File | Relationship |
|------|--------------|
| [002_single_consumer_drain.md](002_single_consumer_drain.md) | The other half of the pair; its release in step 3 is what step 2's acquire pairs with |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_producer_surface.md](../api/001_producer_surface.md) | The shape this procedure is exposed through, and the three candidate shapes it is neutral between |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_two_cursor_ring.md](../data_structure/001_two_cursor_ring.md) | The two cursors steps 1, 2 and 6 touch, and why they are on separate cache lines |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_family_dependency_seam.md](../integration/001_family_dependency_seam.md) | Accounts for `ring_gating`'s absence from the dependency list, which the table above explains |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_exactly_one_producer_one_consumer.md](../invariant/001_exactly_one_producer_one_consumer.md) | The precondition step 1's plain load depends on; violating it makes step 1 a data race |
| [../invariant/002_no_lock_in_the_path.md](../invariant/002_no_lock_in_the_path.md) | The cost claim steps 1–6 establish |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_byte_parity_over_one_hundred_thousand.md](../non_functional_requirement/002_byte_parity_over_one_hundred_thousand.md) | The reached-test this procedure must satisfy |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/003_slot_state_without_holes.md](../lifecycle/003_slot_state_without_holes.md) | Why step 6 alone is a sufficient readiness signal, with no per-slot stamp |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_producer_cursor.md](../type/001_producer_cursor.md) | The value steps 1 and 6 read and write |
| [../type/002_free_capacity.md](../type/002_free_capacity.md) | Step 3's computation, and why its answer stays true here |

### Sources

| File | Relationship |
|------|--------------|
| `Cargo.toml` | Declares five dependencies and none of `ring_gating`, `ring_claim`, `ring_publish` or `ring_consume`, which the Abstract's table explains |

### Tests

| File | Relationship |
|------|--------------|
| `tests/spsc_test.rs` | `one_producer_and_one_consumer_exchange_one_hundred_thousand_items` — 100 000 items with byte-parity, in order, zero loss. The fourth clause is not behavioural and is checked at the source in `tests/manual/readme.md` S2 |
| `tests/spsc_test.rs` | `tests/manual/readme.md` S2 — the Abstract's central claim, established by reading the crate's compiled surface rather than by instrumenting one run. Zero `fetch_`, zero `compare_exchange`, exactly two `.store(` sites |

### SP1 — The Claim Is a Plain Store, and That Is the Whole Saving

```sh
cd "$(git rev-parse --show-toplevel)"/ring_spsc
printf 'read-modify-writes here:  '; grep -vE '^\s*(//|///|//!)' src/lib.rs | grep -cE 'compare_exchange|fetch_'
printf 'in the mpsc sibling:      '; grep -vE '^\s*(//|///|//!)' ../ring_mpsc/src/lib.rs | grep -cE 'compare_exchange|fetch_'
# The command above can legitimately exit nonzero -- a count of zero, a diff
# that differs -- and the sweep that keeps this output current skips any block
# that exits nonzero, so without this the block freezes instead of refreshing.
:
```

Live output:

```
read-modify-writes here:  0
in the mpsc sibling:      0
```

Zero against a non-zero count is the crate's central performance claim reduced
to a number a reader can re-derive. Everything else about the claim step —
uncontended, no spin, no backoff — follows from it.

The absence is enforced by cardinality rather than by discipline: with one
producer there is no second writer to lose a race to, so there is nothing for a
compare-exchange to protect against.

### SP2 — A Reservation Publishes on Drop Even Unwritten

`a_reservation_publishes_on_drop_even_unwritten` is the test, and its name is
the contract.

**The reason is the same here as in the sibling and the cost is lower.** In
`ring_mpsc` a hole in the sequence space blocks every later record because the
drain stops at the first unpublished sequence. Here there is one producer, so a
hole cannot form behind a completed claim — publication is in claim order by
construction. What the drop-publishes rule buys in this crate is not
contiguity but simplicity: no second code path for the abandoned-claim case.

The edge remains. A producer that claims and takes an early return emits a
record, and `Reservation` is a guard so it cannot usefully be `#[ must_use ]`.

**Disposition:** declined — `a_reservation_publishes_on_drop_even_unwritten`
(`ring_spsc/tests/spsc_test.rs`) already pins the drop-publish as deliberate,
and both slot kinds this crate ships default to a record a consumer reads as
empty rather than as garbage: `TypedSlot::default()` is `TypedSlot( None )`
and `BytesSlot::default()` is `Self { bytes : [ 0; N ], len : 0 }`
(`ring_slot/src/lib.rs`). `Reservation` already carries a
`#[ must_use = "..." ]` naming this exact accident, which catches a claim
discarded as a bare statement. The one case that remains — an early return via
`?` between claim and write — is a control-flow gap no lint construct closes,
and the ring stays internally consistent through it either way.
