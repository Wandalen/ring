# API: Consumer Drain Surface

### Scope

- **Purpose**: State the operations the single consumer thread calls, and establish why this surface is batch-shaped by necessity rather than by convenience.
- **Responsibility**: Fix the operation set, the borrow-versus-copy question that decides whether the ring can be zero-copy at all, the error surface, and what a caller may rely on.
- **In Scope**: Drain, the readable-count query, and the empty result, from the caller's side.
- **Out of Scope**: The watermark scan inside the drain (→ [Batch Drain](../algorithm/002_batch_drain_by_cursor_swap.md)); when to call it, which is the consumer's own scheduling problem (→ [The Spinning Consumer Owns a Core](../pitfall/001_spinning_consumer_owns_a_core.md)); the producer's side (→ [Producer Publish Surface](001_producer_publish_surface.md)).

### Abstract

The consumer surface has exactly one legitimate caller, and every property
that distinguishes it from the producer surface follows from that. Because
there is one caller, `consumer_cursor` has one writer, so advancing it is a
plain store rather than a compare-exchange — which is what makes the drain's
amortized cost *less than one atomic per element* rather than one per
element. The single-consumer constraint is not a limitation this surface
works around; it is the source of its performance.

**"Exactly one caller" was recorded as a contract this surface cannot enforce.
It is enforced.** The crate name is no longer the only place it is stated:
`Consumer` is not `Clone`, and carries a `PhantomData< Cell< () > >` that makes
it `!Sync`. Two threads cannot call `drain` concurrently because two threads
cannot both hold it, and `&Consumer` cannot cross a thread boundary either.
Both refusals are asserted by `compile_fail` doc tests, which is the only way
to test a negative — a test that compiles proves nothing about what does not.

That was the first of the two open questions below, and it is closed: the
surface *should* make it unrepresentable, and does. It cost one `PhantomData`
field and no runtime anything.
[Single-Consumer Total Order](../invariant/001_single_consumer_total_order.md)
still owns the invariant; what changed is that it is now held by the type
system rather than by the crate's name.

### Operations

Signatures are the real ones, from `src/lib.rs`:

| Operation | Signature | Contract | Cost |
|-----------|-----------|----------|------|
| `drain` | `&mut self -> Batch< '_, S >` | Yields every record published contiguously from the cursor, in sequence order | One scan of the published run |
| `drain_up_to` | `&mut self, usize -> Batch< '_, S >` | The same, capped — the cap is also clamped to capacity, so a caller cannot make the scan walk past the ring | The same scan, bounded |
| *(commit)* | `impl Drop for Batch` | Not a method. The cursor advance is the batch's destructor — one plain store, which is what the single-consumer constraint buys | One plain store |
| `available` | `&self -> usize` | Count readable *now* | The same scan, without the store |
| `is_empty` | `&self -> bool` | Whether the cursor has caught the published run | One scan step, no walk |
| `position` | `&self -> Seq` | The consumer cursor's current value | One relaxed load |

**The batch shape is forced, not chosen.** A per-element `pop() -> Option<T>`
would need to re-establish the watermark on every call, because a single
element's readability cannot be known without the scan that finds where the
published run ends. That turns an amortized-sub-atomic drain into a
per-element scan and discards the crate's main cost advantage. So the surface
is `drain -> Batch` and a per-element iterator is a *view over* the batch, not
an alternative to it.

**The borrow-versus-copy question is the one that decides zero-copy.** A
`Batch<'_, T>` that borrows the ring lets the consumer read payloads in place
— no copy, which is the whole point of a ring over a queue of boxed nodes.
But the borrow must end before the slots are released for reuse, so the
cursor advance has to happen when the batch is dropped rather than when the
drain is called, and the consumer cannot hold a batch across a call that
would need room. The alternative, `drain -> Vec<T>`, is trivially safe and
copies every element — reintroducing exactly the memcpy the dense slot
packing exists to avoid
(→ [Sequence-Stamped Ring](../data_structure/001_sequence_stamped_ring.md)'s
packing argument). **Decided: borrowing.** `Batch< '_, S >` borrows the ring,
`get`/`get_mut`/`iter` read the slots in place, and the cursor advance is the
batch's `Drop` — so the slots are released for reuse exactly when the borrow
ends, enforced by the borrow checker rather than by a rule the consumer must
remember. `a_live_batch_still_holds_its_slots_against_reuse` asserts the
consequence the instance predicted: a producer cannot reclaim those slots
while the batch is alive.

The predicted cost is real and unmitigated: a consumer genuinely cannot hold a
batch across a call that needs room, which is a constraint on the consumer's
own structure. That is the correct trade — the copy the alternative imposes is
paid by *every* element on *every* drain, while this constraint is paid only
by a consumer that wanted to hold a batch open, which is a consumer that
should be draining more often anyway.

**`available` costs the same as `drain` and callers will assume otherwise.**
Both perform the watermark scan; only `drain` also advances the cursor. A
consumer that calls `available` to decide whether to call `drain` has paid the
scan twice. `is_empty` is the cheap check — it tests one stamp and stops,
never walking the run — and is the one a polling loop should use. The naming
does not communicate this. It was recorded as "a defect in the surface worth
recording before it is implemented rather than after"; it was implemented
anyway, with the asymmetry documented on both methods, because renaming
`available` to something that advertises its cost (`scan_available`) would
make the *common* call site uglier to warn about the *uncommon* misuse. The
warning belongs in the docs, which is where it is.

### Error Handling

**No operation returns an error, and here — unlike the producer surface —
that is close to a real guarantee rather than deferred policy.**

| Condition | Behaviour | Is it an error? |
|-----------|-----------|-----------------|
| Nothing published | `drain` returns an empty batch | **No.** Empty is the normal state of a working ring; treating it as an error would make the common case exceptional |
| Producers all stopped | Empty batch, forever | No — and indistinguishable from idle producers (→ [Ring Construction and Teardown](../lifecycle/001_ring_construction_and_teardown.md)'s L2) |
| A producer holds a claim | Empty batch **despite later sequences being published** — but only until that claim is dropped | Not an error, and no longer permanent |
| Two threads call `drain` | **Unrepresentable.** `Consumer` is neither `Clone` nor `Sync` | Not applicable — it does not compile |

**The third row was the one that should alarm a reader, and the guard decision
declawed it.** It read: "Empty batch, forever, despite later elements being
published — the most serious defect this surface can exhibit." That was true
of the three-call producer shape, where a claim could be abandoned and never
published. With
[the producer surface](001_producer_publish_surface.md)'s guard, a claim
cannot be abandoned: dropping the guard publishes, and a producer that panics
or returns early drops the guard. The head-of-line block is now bounded by the
*lifetime of a stack value*, not by a caller remembering to publish.
`an_unpublished_claim_blocks_every_later_sequence_while_it_is_held` asserts
both halves — that it blocks while held, and that it clears when dropped.

What survives is real but ordinary: a producer that holds a claim for a long
time delays every later sequence, because a stamped ring drains contiguously
by construction (→ [Batch Drain](../algorithm/002_batch_drain_by_cursor_swap.md)).
That is head-of-line blocking, it is inherent to total ordering, and the only
way to not have it is to not promise order. From the consumer's side it is
still indistinguishable from idleness, so the diagnostic still belongs
outside the drain — a stalled-watermark warning that fires when `claimed()`
exceeds `published_through()` for longer than some interval, which is
[`ring_debug`](../../../ring_debug/readme.md)'s or
[`ring_stats`](../../../ring_stats/readme.md)'s to provide, not this
surface's. Both accessors exist on `Ring` for exactly that.

### Compatibility Guarantees

1. **Order is the guarantee.** Elements arrive in sequence order across every
   drain, with no gaps and no duplicates, for the ring's lifetime — not just
   within one batch. This is
   [Single-Consumer Total Order](../invariant/001_single_consumer_total_order.md),
   and it is the one property a caller may build on. Any future change that
   weakened it — a skip-the-stuck-slot recovery, a multi-consumer mode —
   would be a contract change and not an optimization, however it was
   packaged.
2. **Batch boundaries are not stable and must not be relied on.** The same
   published sequence of elements may arrive as one batch of 100 or ten of
   10, depending entirely on producer timing. A consumer that treats a batch
   as a semantic unit — a transaction, a frame — has read structure into
   scheduling noise. If a semantic grouping is needed it belongs in the
   payload (→ [`ring_batch`](../../../ring_batch/readme.md), which owns
   batch-as-a-concept for the family), never in the drain's return shape.
3. **Empty is stable and permanent.** `drain` returning nothing will never
   become an error or a blocking call. A blocking variant, if wanted, is an
   additional operation over [`ring_wait`](../../../ring_wait/readme.md),
   not a change to this one.
4. **Not stable overall, same as the producer side.** The mechanism is ungated
   until [Measured Before Adopted](../non_functional_requirement/001_measured_before_adopted.md)
   clears. The borrow-versus-copy question no longer threatens the return
   type — it is decided (borrowing) — so what remains unstable is the
   mechanism under the surface, not the surface's shape.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_batch_drain_by_cursor_swap.md](../algorithm/002_batch_drain_by_cursor_swap.md) | The procedure `drain` is the surface of; its Step 3 is the scan `available` also pays for |

### APIs

| File | Relationship |
|------|--------------|
| [001_producer_publish_surface.md](001_producer_publish_surface.md) | The asymmetric counterpart; many callers, lock-free, per-element |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_sequence_stamped_ring.md](../data_structure/001_sequence_stamped_ring.md) | Its dense packing is what the copy-on-drain option would waste, and its `drain` row states the sub-atomic cost claim |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_single_consumer_total_order.md](../invariant/001_single_consumer_total_order.md) | The one compatibility guarantee this surface makes; the contract it was said to be unable to enforce is enforced by `Consumer` being neither `Clone` nor `Sync` |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_ring_construction_and_teardown.md](../lifecycle/001_ring_construction_and_teardown.md) | Its P3 → P4 final drain is this surface's last call, and its L3 leak is what an unread batch becomes |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_measured_before_adopted.md](../non_functional_requirement/001_measured_before_adopted.md) | Why this surface is not stable |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_spinning_consumer_owns_a_core.md](../pitfall/001_spinning_consumer_owns_a_core.md) | What a caller does between drains, and why `is_empty` rather than `available` is the poll |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/004_ring_occupancy_between_cursors.md](../lifecycle/004_ring_occupancy_between_cursors.md) | Its Behavioral Invariant 4 is why `is_empty` is safely actionable here and `free_capacity` is not on the producer side |

### Sources

| File | Relationship |
|------|--------------|
| `src/lib.rs` | Declares `Consumer` and `Batch`; the `compile_fail` doc tests on the module are where the two refusals above are asserted |

### Tests

| File | Relationship |
|------|--------------|
| `tests/mpsc_test.rs::four_producers_exchange_one_hundred_thousand_items_with_byte_parity` | Compatibility Guarantee 1 under real producer timing — the concatenation of every batch equals the published multiset exactly, with batch boundaries varying run to run, which is Guarantee 2's evidence as a side effect |
| `tests/mpsc_test.rs::a_second_drain_sees_what_was_published_after_the_first` | That the cursor advances across drains and nothing is redelivered |
| `tests/mpsc_test.rs::a_live_batch_still_holds_its_slots_against_reuse` | The borrow decision's consequence — the batch's lifetime, not the drain call, is what releases the slots |
| `tests/mpsc_test.rs::drain_up_to_zero_takes_nothing_and_leaves_the_records_drainable` | That a capped drain of zero is a no-op rather than a commit of nothing |
| `tests/mpsc_test.rs::drain_up_to_more_than_capacity_is_capped_rather_than_scanning_past_the_ring` | The second clamp in `drain_up_to`, which the signature does not advertise |
| `tests/mpsc_test.rs::draining_an_empty_ring_yields_an_empty_batch_and_moves_nothing` | Compatibility Guarantee 3 — empty is a value, not an error, and costs no cursor movement |
| `tests/mpsc_test.rs::an_unpublished_claim_blocks_every_later_sequence_while_it_is_held` | The Error Handling table's third row, in both directions: blocked while held, clear once dropped |

### MP7 — A Live `Batch` Holds Its Slots Against Reuse

`a_live_batch_still_holds_its_slots_against_reuse` pins it. The batch borrows
`&mut Consumer`, and the `COMMIT` store happens in its `Drop` — so a caller
holding a batch across a long computation holds that many slots out of
circulation.

**This is backpressure applied by lexical scope**, which is unusual enough to
state: the producers' headroom check reads the consumer cursor, so a batch kept
alive is a batch stalling every producer. Dropping it promptly is a performance
obligation the type does not express.

### MP8 — Two `is_empty` Methods on Two Types Answer Different Questions

```sh
cd "$(git rev-parse --show-toplevel)"/ring_mpsc
grep -E '^  pub (const )?fn is_empty' src/lib.rs
grep -B2 '^  pub (const )?fn is_empty' src/lib.rs | grep -E '^[0-9]+-  ///' | head -4
```

Live output:

```
  pub fn is_empty( &self ) -> bool
  pub const fn is_empty( &self ) -> bool
```

Two methods, two receivers, two meanings. `Consumer::is_empty` is a live
question about the ring; `Batch::is_empty` is a settled fact about a drain that
already happened.

A caller writing `if consumer.drain().is_empty()` and a caller writing
`if consumer.is_empty()` are asking different things — the first has already
committed to a drain and released its slots, the second has not touched the
ring. The names give no hint of that.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_mpsc
echo '  -- Consumer::is_empty, non-destructive contrast --'
command grep -A5 '/// Whether nothing is drainable' src/lib.rs | tail -3
echo '  -- Batch::is_empty, the destructive-drain contrast --'
command grep -A5 '/// Whether the batch is empty' src/lib.rs | tail -3
```

Live output:

```
  -- Consumer::is_empty, non-destructive contrast --
    /// nothing. Contrast [`Batch::is_empty`]: `consumer.drain().is_empty()`
    /// commits the whole drain as a side effect of taking it, discarding every
    /// currently published record along the way, whereas `consumer.is_empty()`
  -- Batch::is_empty, the destructive-drain contrast --
    /// settled fact about a drain that already committed, not a live question
    /// about the ring. `consumer.drain().is_empty()` discards every currently
    /// published record as a side effect of the `drain()` call alone — for a
```

**Disposition:** applied — both doc comments in `src/lib.rs` now cross-link
and state the hazard directly: `Consumer::is_empty` names itself
non-destructive and points to `Batch::is_empty`'s side effect, and
`Batch::is_empty` states that `consumer.drain().is_empty()` discards every
currently published record and points back to `Consumer::is_empty` for a
non-destructive check. The crate's 31 integration tests plus 33 doctests (28
regular, 5 `compile_fail`) re-verified passing
(`cargo test --all-features -p ring_mpsc`, 2026-09-04). Now prints:
`commits the whole drain as a side effect of taking it, discarding every`
