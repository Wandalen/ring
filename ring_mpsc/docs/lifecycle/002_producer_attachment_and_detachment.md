# Lifecycle: Producer Attachment and Detachment

### Scope

- **Purpose**: State the cycle an individual producer runs against a longer-lived ring, and isolate the one window — claim held, payload not yet published — that makes detachment more than dropping a handle.
- **Responsibility**: Name the producer's phases, what each permits, and why the ring cannot observe this cycle at all.
- **In Scope**: One producer's attach, publish loop, and detach; the outstanding-claim window; the handle's relationship to the ring's own lifetime.
- **Out of Scope**: The ring's own phases (→ [Ring Construction and Teardown](001_ring_construction_and_teardown.md)); the consumer, which has no equivalent cycle because there is exactly one of it; the claim procedure's steps (→ [Claim-Then-Publish](../algorithm/001_claim_then_publish.md)).

### Lifecycle Phases

| Phase | Established by | May do | Leaves via |
|-------|----------------|--------|------------|
| **Q1 Detached** | Not yet attached, or already detached | Nothing against this ring | Acquiring a handle |
| **Q2 Attached, idle** | Holding a producer handle, no claim outstanding | Claim; detach freely | Claiming, or detaching |
| **Q3 Claim outstanding** | Fetch-add returned a sequence, payload not yet published | Write the payload; publish | Publishing |
| **Q4 Detaching** | Handle release requested while in Q2 | Nothing | Immediate — Q2 → Q4 → Q1 is instantaneous |

**Q3 is the whole content of this instance.** Q1, Q2, and Q4 are bookkeeping;
Q3 is a window in which one thread has taken exclusive ownership of a slot
the consumer is waiting behind, and during which the ring is not in a state
any observer can safely tear down or fully drain.

**The ring cannot observe any of this.** There is no producer registry, no
attach counter, no per-producer state in the ring. A "producer handle" is a
capability to call claim — not a registration. That is deliberate (a registry
would be a contended structure serving no operation on the hot path), but it
means every guarantee below is the caller's to uphold rather than the ring's
to enforce. [`ring_registry`](../../../ring_registry/readme.md) exists in the
family for cases that genuinely need the count; this crate does not take that
dependency, and the omission is a design position rather than an oversight.

### Phase Transitions

| # | From → To | Trigger | Safe? |
|---|-----------|---------|-------|
| M1 | Q1 → Q2 | Acquire a producer handle | Yes, unconditionally |
| M2 | Q2 → Q3 | Claim (CAS loop) | Yes — the sequence is uniquely owned from this instant |
| M3 | Q3 → Q2 | Publish (stamp store, on guard drop) | Yes; ownership of the slot ends here |
| M4 | Q2 → Q1 | Release the handle | Yes — nothing is outstanding |
| M5 | Q3 → Q1 | Release the handle **with a claim outstanding** | **Unrepresentable.** The defect this instance exists to name, and the reason the guard was chosen |

**M5 was unrecoverable, and calling it "abandoning a claim" undersold it.**
A producer that left Q3 without publishing — by returning early, by panicking
between the claim and the stamp store, or by dropping a handle mid-operation —
left a slot permanently Written. The consumer's drain stopped at that sequence
and never advanced past it. Every element published *after* it, however many,
became permanently unreachable even though each was correctly published and
sitting in memory. One dropped claim silently converted the ring from a
working channel into a channel that delivered nothing further, with no error,
no panic, and no diagnostic.

Three mitigations were named. **The first was chosen**, and its stated cost
turned out not to apply:

| Mitigation | Mechanism | Cost, as predicted → as found |
|------------|-----------|-------------------------------|
| **Make M5 unrepresentable** ✅ | `claim` returns `Reserved`, whose `Drop` stores the stamp. Leaving Q3 without publishing is impossible: leaving Q3 *is* dropping the guard | **Predicted:** a tombstone the consumer must handle, so the payload type grows a "skip me" case, plus a completion flag to avoid publishing a partial write on panic. **Found:** neither is needed — see below |
| Detect and skip | The consumer times out on a stalled watermark and skips the stuck sequence | Breaks [Single-Consumer Total Order](../invariant/001_single_consumer_total_order.md) outright — a skipped sequence is a gap in the total order the invariant promises, so this is not a mitigation but a different contract |
| Accept and document | State that a producer must publish every claim, and that failing to do so wedges the ring | Free, honest, and offers the caller no help whatsoever |

**Neither predicted cost materialized, and the reason is worth stating because
it is the load-bearing part of the decision.** Both costs assumed a slot may
contain arbitrary bytes, so an unwritten publication would be *garbage* the
consumer must be warned about. It cannot: the consumer leaves every slot
`Slot::clear`-ed, i.e. `Default`. A guard dropped without a write therefore
publishes an **empty record** — well-formed, safe, and already representable
in the payload type (`TypedSlot::get` returns `None`). No tombstone variant,
no fifth slot state, no completion flag on the drop path.
`a_claim_dropped_without_a_write_publishes_an_empty_record` asserts exactly
this.

**What remains, stated rather than hidden:** a producer that panics *after* a
partial write publishes that partial write — a well-formed record with wrong
contents. Detecting it needs the completion flag, which is a per-publication
cost paid by every correct producer to improve the diagnosis of an incorrect
one. Not taken. A payload that must be all-or-nothing should be built and then
moved in, which is what `Producer::push` does.

This closes the decision shared with
[Slot State Across One Lap](../lifecycle/003_slot_state_across_one_lap.md)'s
*Poisoned* question and
[the producer surface](../api/001_producer_publish_surface.md)'s guard-versus-
three-calls question — "three documents, one choice." The choice is the guard,
and the fifth slot state the tombstone would have required does not exist.

### Dependencies

| Dependency | Phase | Why |
|------------|-------|-----|
| [`ring_claim`](../../../ring_claim/readme.md) | M2 | Owns the compare-exchange loop that enters Q3. The guard type did not land here — `Reserved` lives in `ring_mpsc`, because it borrows this crate's `Ring` and would otherwise force `ring_claim` to know the ring's slot layout |
| [`ring_publish`](../../../ring_publish/readme.md) | M3 | Owns the stamp store that leaves Q3 |
| [`ring_handle`](../../../ring_handle/readme.md) | M1, M4 | Owns the handle type itself; one of the family's five externally-visible crates, so the handle's shape is an export-surface decision and not merely an internal one |

**The handle's own lifetime relationship to the ring decides L5 in the sibling
instance, and the borrowing option was taken.** `Producer< 'a, S >` borrows
`&'a Ring< S >`, so the compiler forbids dropping the ring while producers
live and
[Ring Construction and Teardown](001_ring_construction_and_teardown.md)'s
forbidden L5 transition is unrepresentable rather than merely prohibited. The
`Arc` option — which would have made L5 possible and pushed it to runtime —
is not taken here.

That decision is this crate's own, for its own `Producer`. It does not
pre-empt [`ring_handle`](../../../ring_handle/readme.md), which owns the
*exported* handle shape and may still need `Arc` for a handle that outlives a
stack frame. What this instance can now say is that borrowing works at this
layer, so `ring_handle` is choosing between two working options rather than
resolving an open question for both.

### Cleanup Requirements

1. **Publish before detaching.** Every claim taken in Q3 is published before
   M4. This was "the caller's obligation, unenforced"; it is now automatic —
   a `Reserved` borrows the ring and publishes on drop, so a claim cannot
   survive its producer's scope, let alone its detachment.
2. **No cleanup on the ring's side.** Detaching writes nothing, touches no
   cursor, and leaves no trace — a correctly-detached producer is
   indistinguishable from one that never attached. That is what makes M4
   free, and it is why the crate takes no `ring_registry` dependency.
3. **Detachment is not a barrier.** M4 does not wait for the consumer to
   drain what this producer published. Published elements outlive their
   producer; the handle's lifetime and the payload's are unrelated.
4. **A panic between M2 and M3 is handled by the same mechanism as a normal
   return.** Rust's unwinding runs destructors, so the guard fires — which is
   what made the guard attractive. The tombstone semantics that made it look
   hard are not needed, since an unwritten publication is an empty record
   rather than garbage. A panic *after* a partial write still publishes that
   partial write; see M5 above for why that residue is accepted rather than
   flagged.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_claim_then_publish.md](../algorithm/001_claim_then_publish.md) | Its Steps 1–3 are exactly M2, Q3's body, and M3 |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_family_dependency_seam.md](../integration/001_family_dependency_seam.md) | Names `ring_handle`, `ring_claim`, and `ring_publish` as the seams this cycle runs across |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_single_consumer_total_order.md](../invariant/001_single_consumer_total_order.md) | The contract the detect-and-skip mitigation would break, which is why it is listed as a different contract rather than an option |

### Lifecycles

| File | Relationship |
|------|--------------|
| [001_ring_construction_and_teardown.md](001_ring_construction_and_teardown.md) | The enclosing cycle; its Cleanup Requirement 2 is M5 seen at teardown, and its forbidden L5 is decided by this instance's handle-shape question |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/003_slot_state_across_one_lap.md](../lifecycle/003_slot_state_across_one_lap.md) | Q3 is exactly its Written state; M5 is what would have made that state permanent, and the guard is what bounds it to the guard's own scope |

### Sources

| File | Relationship |
|------|--------------|
| `src/lib.rs` | `Producer` is the handle (M1/M4 are `Copy` and drop, both free); `Reserved` is Q3, and its `Drop` is M3 |

### Tests

| File | Relationship |
|------|--------------|
| `tests/mpsc_test.rs::an_unpublished_claim_blocks_every_later_sequence_while_it_is_held` | M5's tail-loss consequence, asserted in both directions — later elements are undrainable while the claim is held, and drainable the moment it drops. The second half is what the guard bought |
| `tests/mpsc_test.rs::a_claim_dropped_without_a_write_publishes_an_empty_record` | The guard's chosen semantics: an unwritten publication is empty, not a tombstone and not garbage |
| `tests/mpsc_test.rs::a_cloned_producer_shares_the_claim_cursor_rather_than_starting_a_new_one` | M1 — acquiring another handle attaches to the same ring rather than forking state |
| `tests/mpsc_test.rs::four_producers_exchange_one_hundred_thousand_items_with_byte_parity` | Cleanup Requirement 3 at scale: the consumer keeps draining after every producer thread has ended, and the totals still reconcile |

### MP33 — Detachment Is a Drop With No Bookkeeping

`Producer` is `Copy` over a shared reference (→ [`../api/001`](../api/001_producer_publish_surface.md), MP5),
so there is no count to decrement and no slot to reclaim. Attachment and
detachment are both free.

The consequence is that the ring cannot know its producers are gone. There is no
"all producers dropped" signal, so a consumer cannot distinguish "no records
yet" from "no records ever again" — which is what `ring_shutdown` exists to
supply one layer up.
