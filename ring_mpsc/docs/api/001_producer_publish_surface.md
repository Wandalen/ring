# API: Producer Publish Surface

### Scope

- **Purpose**: State the operations a producer thread calls, their contracts, and the single condition under which any of them can fail to make progress.
- **Responsibility**: Fix the operation set, the shape question (three calls versus one guard), the error surface, and what a caller may rely on across the family's still-settling export boundary.
- **In Scope**: Claim, write, publish, and the capacity query, from the caller's side.
- **Out of Scope**: The steps inside each operation (→ [Claim-Then-Publish](../algorithm/001_claim_then_publish.md)); the handle type that owns these methods (→ [`ring_handle`](../../../ring_handle/readme.md)); the consumer's side (→ [Consumer Drain Surface](002_consumer_drain_surface.md)).

### Abstract

The producer surface is called by many threads at once and is **lock-free**:
some producer always makes progress, and no producer can block another
indefinitely. `Producer` is `Copy + Send + Sync`, which is what "multi-producer"
means concretely — the handle is shared by copying it into each thread, not by
wrapping it.

**This instance originally claimed wait-freedom, and that claim was wrong.**
It is corrected here rather than deleted, because the reason it is unavailable
is a structural fact worth stating once:

> **Wait-freedom and bounded capacity are exclusive at the claim.**

A wait-free claim is a `fetch_add`: unconditional, bounded steps, no retry.
On a *bounded* ring that hands out sequences past the consumer's tail, and
undoing them requires either a CAS loop or blocking — there is no wait-free
undo. So the free-capacity check must be fused into the claim, and a fused
check-and-claim is a `compare_exchange` loop by construction. `ring_claim`
implements exactly that, measured against this instance's own assumption and
ruled on at the family level.

The loop is not a weakness, and lock-free is not a consolation prize: every
producer's failure to progress is another producer's success. What is lost is
the *bounded step count per call*, which matters only to a hard-real-time
caller — and a hard-real-time caller cannot use a bounded ring's backpressure
either, so it was never in the audience.

The surface's asymmetry with the consumer's is total and worth stating
plainly: many callers here versus exactly one there; contended here versus
uncontended there; per-element here versus per-batch there. Documenting them
as one API would obscure every one of those differences.

### Operations

Signatures are the real ones, from `src/lib.rs`:

| Operation | Signature | Contract | Cost |
|-----------|-----------|----------|------|
| `claim` | `&self -> Result< Reserved< '_, S >, RingError >` | Returns a guard owning a sequence no other caller receives, or `RingError::Full`. Ownership of the slot ends when the guard drops | One CAS loop, contended |
| *(write)* | `Reserved: DerefMut< Target = S >` | Not a method — the guard *is* the slot. `*reserved = …` or `reserved.set( v )` writes in place | Fully parallel; no synchronization at all |
| *(publish)* | `impl Drop for Reserved` | Not a method either. Publication is the guard's destructor: one `Release` store of the sequence into the slot's stamp | One `Release` store |
| `push` | `&self, T -> Result< Seq, RingError >` | The fused convenience form, for `TypedSlot< T >` only | Claim + move + drop |
| `claim_batch` | `&self, max -> Result< ReservedBatch< '_, S >, RingError >` | Up to `max` contiguous sequences — whatever headroom allows — for one gate check and one exchange. The guard reaches its slots by offset and publishes the whole grant when it drops | One CAS loop per grant, contended |
| *(batch write)* | `ReservedBatch::slot_mut( offset ) -> Option< &mut S >` | The grant's slots by offset — the same unsynchronized in-place write as `Reserved`, range-checked | Fully parallel; no synchronization at all |
| *(batch publish)* | `impl Drop for ReservedBatch` | One `Release` stamp store per sequence of the grant, in issue order — every sequence, written or not; an unwritten offset publishes an empty record | k `Release` stores |
| `push_batch` | `&self, &mut Vec< T > -> Result< usize, RingError >` | The batched fused convenience: drains exactly the granted prefix, publishes it, returns the count; `Full` leaves the vec untouched | One grant + k moves + drop |
| `free_capacity` | `&self -> usize` | Room a claim may consume — **advisory** | One gating scan |
| `claimed` | `&self -> Seq` | The claim cursor's current value | One relaxed load |

**The guard shape was chosen, and the three-call shape rejected.** This
instance recorded the choice as Undecided and shared with
[Producer Attachment and Detachment](../lifecycle/002_producer_attachment_and_detachment.md)'s
M5 and
[Slot State Across One Lap](../lifecycle/003_slot_state_across_one_lap.md)'s
*Poisoned* question — "three documents, one choice." The choice is the guard,
and all three are closed together.

The argument for the three-call shape was that it makes the write provably
outside the serialized region — a caller can *see* that only `claim` is
contended. The guard keeps that property entirely: `Reserved` derefs to the
slot, so the write is still an ordinary unsynchronized store to memory the
caller exclusively owns, and it is still visibly not inside anything. What the
guard removes is M5: a caller that claims and returns early no longer wedges
the ring, because returning early *is* dropping the guard, which publishes.

**The objection that blocked this decision has a concrete answer.** The
recorded cost was that `Drop` cannot fail, so a panic mid-write would publish
a partially-written slot unless the guard tracked completion with a flag. That
is true of a slot holding *arbitrary* bytes. It is not true here: the consumer
leaves each slot `Slot::clear`-ed, i.e. `Default`, so a guard dropped without
a write publishes an **empty record** — a defined, observable, safe outcome,
not a torn one. No completion flag is needed, and none exists.
`a_claim_dropped_without_a_write_publishes_an_empty_record` asserts it.

That leaves the honest residue, stated rather than hidden: a producer that
panics *after* a partial write publishes that partial write. The record is
well-formed and safe; it is simply wrong. Detecting it needs a completion flag
on the drop path, which is a per-publication cost paid by every correct
producer to improve the diagnosis of an incorrect one. Not taken. A payload
that must be all-or-nothing should be built and then moved in — which is what
`push` does.

**A single fused `push(value)` is a fourth option, and it exists — as a
convenience, not as the surface.** With claim/write/publish fused, a caller
cannot construct the payload *in place*, so a large `T` is built on the stack
and memcpy'd, reintroducing a copy the ring's design avoids. So `push` is
offered for `TypedSlot< T >` where that copy is a move of one value and the
ergonomics win, while `claim` remains the primitive everything else is built
on. The fused form being a thin wrapper *over* the guard rather than an
alternative *to* it is the whole reason both can exist without the surface
forking.

**`free_capacity` is advisory, and now for a sharper reason.** A caller
reading room and then claiming performs two operations with a gap; other
producers may consume the room between them. The original text called this "a
structural property of a wait-free claim — resolving it requires a claim that
can fail, i.e. a compare-exchange loop, i.e. not wait-free." The claim *is*
a compare-exchange loop that can fail, and the value is *still* advisory —
because the gap is between the two calls, not inside either. The right
conclusion is the opposite of the original one: **do not read it and then
claim; just claim, and handle `RingError::Full`.** The claim's own check is
atomic with the claim, which is the entire point of fusing them.
[Ring Occupancy Between the Cursors](../lifecycle/004_ring_occupancy_between_cursors.md)'s
Behavioral Invariant 3 carries the full argument. Its open question — expose
the value at all, or name it `_hint` — is answered here: exposed, under its
plain name, because it is genuinely useful for *sizing* (a batch producer
asking "roughly how many can I push before I should yield") and the misuse it
invites is documented on the method itself.

### Error Handling

**One operation returns an error, and the guard shape made three of the four
conditions below unrepresentable rather than merely undocumented.**

| Condition | Surface behaviour | Where it is decided |
|-----------|-------------------|---------------------|
| Ring full at claim | `Err( RingError::Full )` — the **Fail** policy | [Bounded Capacity and Backpressure Policy](../non_functional_requirement/002_bounded_capacity_backpressure.md) |
| Caller writes a sequence it does not own | **Unrepresentable.** The only write path is `DerefMut` on a guard that owns exactly one sequence | The guard shape, chosen above |
| Caller publishes twice | **Unrepresentable.** `Drop` runs once; the guard is not `Clone` and cannot be published manually | Same |
| Producer panics mid-write | Publishes a well-formed record with partial contents — safe, not torn, and not a wedge | [Producer Attachment and Detachment](../lifecycle/002_producer_attachment_and_detachment.md) M5, closed |

**Fail was chosen from the policy instance's four, and the reasoning is the
signature argument this section originally made.** Whichever policy is chosen
*changes this signature*: Block keeps `claim -> Seq` and blocks a producer on
a peer, which is the one thing a contended-claim ring exists to avoid.
Overwrite keeps the signature and silently drops the oldest undrained element,
violating exactly-once delivery — a stated invariant, so it is not a policy
choice at all at this layer. Drop-newest keeps the signature and loses this
element without telling the caller. **Fail is the only one that keeps the
decision at the call site**, where the caller knows whether its payload is
droppable, and it is the only one of the four that can be *built on* — Block
is `while let Err( Full ) = claim()`, Drop-newest is `let _ = push( v )`, and
both are three lines in a caller. The reverse does not hold: a caller cannot
recover fail-fast semantics from a policy that already blocked or already
dropped.

So the surface implements Fail and not the other three, and that is a
deliberate reduction rather than an incomplete implementation. Per-priority-
class policy — which the NFR declines to resolve — belongs in the crate that
knows the class, above this one.

### Compatibility Guarantees

1. **Nothing on this surface is stable yet.** The surface is implemented and
   tested, but
   [Measured Before Adopted](../non_functional_requirement/001_measured_before_adopted.md)
   gates the *mechanism*; a surface over a mechanism not yet measured against
   its alternatives cannot promise stability. Implemented is not adopted.
2. **Lock-freedom is the guarantee worth holding stable.** It is the property
   both prospective consumers would be adopting this crate *for*, and the one
   that constrains every remaining question above. A future revision that
   quietly makes `claim` blocking — a Block backpressure policy compiled in,
   say — would be a breaking change with an unchanged signature, which is the
   case for stating it here rather than as an implementation note.

   The guarantee this instance *used* to state, wait-freedom, would have been
   violated by the crate's own first implementation. That is worth keeping
   visible: a compatibility guarantee asserted about a dependency's behaviour
   before that dependency exists is a guess wearing a guarantee's clothes.
3. **This crate is internal to the family.** Only `ring_factory`,
   `ring_handle`, `ring_tls`, `ring_flush`, and `ring_types` are visible
   outside it.
   So this surface's real external contract is whatever
   [`ring_handle`](../../../ring_handle/readme.md) re-exports, and a change
   here is breaking only insofar as it reaches that boundary. That indirection
   is deliberate freedom: the mechanism can change under a stable handle.
4. **Payload agnosticism is bounded by `Slot`, and `Send` arrives via the
   `Sync` impl rather than the payload.** `S : Slot` is the bound — the
   payload is a slot type, not a bare `T` — and `unsafe impl< S : Send > Sync
   for Ring< S >` is where `Send` is actually required, on the ring rather
   than on each operation. `Copy` is *not* required: the teardown contract
   (→ [Ring Construction and Teardown](../lifecycle/001_ring_construction_and_teardown.md)'s
   L3) is discharged by `Buffer`'s own `Drop`, asserted by
   `every_record_written_is_destroyed_exactly_once` with a drop-counting
   payload. A caller should still expect bounds to be *added* rather than
   relaxed, which is breaking in the ordinary Rust sense.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_claim_then_publish.md](../algorithm/001_claim_then_publish.md) | The procedure these three operations are the surface of |

### APIs

| File | Relationship |
|------|--------------|
| [002_consumer_drain_surface.md](002_consumer_drain_surface.md) | The asymmetric counterpart; its single-caller constraint is what lets it be batch-shaped |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_sequence_stamped_ring.md](../data_structure/001_sequence_stamped_ring.md) | States the same four operations from the field side, with their cost discipline |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_producer_attachment_and_detachment.md](../lifecycle/002_producer_attachment_and_detachment.md) | Shared the guard-versus-three-calls decision; closed here in favour of the guard, which closes its M5 |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_measured_before_adopted.md](../non_functional_requirement/001_measured_before_adopted.md) | Why no part of this surface is stable |
| [../non_functional_requirement/002_bounded_capacity_backpressure.md](../non_functional_requirement/002_bounded_capacity_backpressure.md) | Owns the policy that decides `claim`'s return type |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_channel_to_ring_binding.md](../pattern/001_channel_to_ring_binding.md) | Binds a declared channel to a ring instance; its Rule 2 fixes the `T` this surface is generic over |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/004_ring_occupancy_between_cursors.md](../lifecycle/004_ring_occupancy_between_cursors.md) | Owns why `free_capacity` is advisory; its expose-or-not question is answered above (exposed, plain name) |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_sequence_number.md](../type/001_sequence_number.md) | `claim`'s return value; its newtype-versus-bare-integer question is this surface's transposition-safety question |

### Sources

| File | Relationship |
|------|--------------|
| `src/lib.rs` | Declares `Producer`, `Reserved`, and the four operations; the module docs carry the same guard argument from the implementer's side |

### Tests

| File | Relationship |
|------|--------------|
| `tests/mpsc_test.rs::four_producers_exchange_one_hundred_thousand_items_with_byte_parity` | Four threads claiming and publishing under sustained contention; every element delivered exactly once and each producer's own items in issue order |
| `tests/mpsc_test.rs::the_producer_is_send_and_sync_and_copy_which_is_what_multi_producer_means` | The `Copy + Send + Sync` claim in the Abstract, asserted by a bound-checking function rather than by inspection |
| `tests/mpsc_test.rs::a_claim_dropped_without_a_write_publishes_an_empty_record` | The answer to the objection that blocked the guard decision — an unwritten guard publishes empty, not torn |
| `tests/mpsc_test.rs::a_cloned_producer_shares_the_claim_cursor_rather_than_starting_a_new_one` | That copying the handle is what shares the ring, not what forks it |
| `tests/mpsc_test.rs::a_claimed_slot_is_readable_through_the_guard_before_it_is_published` | `Reserved`'s shared deref — a producer can read back its own in-place write before publishing |
| `tests/mpsc_test.rs::a_claim_past_capacity_reports_full_rather_than_overwriting` | The Fail policy, distinguished by name from the Overwrite alternative it was chosen over |
| `tests/mpsc_test.rs::every_record_written_is_destroyed_exactly_once` | Compatibility guarantee 4's no-`Copy`-bound claim, with a drop-counting payload |

### MP5 — The Producer Is `Copy`, Which Is What Multi-Producer Means Here

`the_producer_is_send_and_sync_and_copy_which_is_what_multi_producer_means` is
the test. A `Producer< 'a, S >` is a `&'a Ring< S >` in a newtype, so copying it
is copying a shared reference — no `Arc`, no clone method, no cardinality
question.

**Contrast `ring_core::Producer`, which needs `try_clone` returning an
`Option`** because it must also cover the SPSC backend, where a second producer
is not permissible. The uniform surface one layer up converts a `Copy` into a
fallible method (→ `ring_core`'s `type/002`).

### MP6 — A Cloned Producer Shares the Claim Cursor Rather Than Starting a New One

`a_cloned_producer_shares_the_claim_cursor_rather_than_starting_a_new_one` is
the assertion, and it is the one that matters: a `Copy` that duplicated cursor
state would let two producers claim the same sequence, which is the exact
failure the whole crate exists to prevent.

The property follows from `Producer` being a shared reference, so the test is
guarding against a future refactor that gives `Producer` a field of its own
rather than against a bug reachable today.
