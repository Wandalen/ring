# Lifecycle: Slot State Across One Lap

### Scope

- **Purpose**: State the four conditions a single slot passes through between two consecutive claims of the same index, and fix which field write effects each transition.
- **Responsibility**: Make the slot's state machine explicit, including the transition that has no writer at all, and record which states are distinguishable by an observer and which are deliberately not.
- **In Scope**: One slot at one index, across one lap and over the lap boundary into the next; the observer's decision procedure; concurrency between slots.
- **Out of Scope**: The stamp's width and storage layout (→ [Sequence-Stamped Ring](../data_structure/001_sequence_stamped_ring.md)); the memory orderings each transition's write carries (→ [Publication Ordering](../invariant/002_publication_ordering.md)); the ring-wide occupancy view (→ [Ring Occupancy Between the Cursors](004_ring_occupancy_between_cursors.md)).

### States

A slot at index `i` is in exactly one of four states with respect to a given
lap `L`, whose expected sequence is `S = i + L × CAPACITY`. The state is not
stored anywhere as a state: it is *computed* by comparing `stamps[i]` against
`S`, which is what makes the stamp a state encoding rather than a readiness
flag.

| State | Condition on `stamps[i]` | Who may touch the slot | Observable as itself? |
|-------|--------------------------|------------------------|-----------------------|
| **Reusable** | `< S`, and `consumer_cursor > S - CAPACITY` — the previous lap's element has been drained | Any producer, by claiming `S` | No — collapses with Written below |
| **Written** | `< S`, and one producer holds sequence `S` and is mid-payload-write | Exactly the one producer that claimed `S` | No — collapses with Reusable above |
| **Published** | `== S` | The consumer, read-only, until it advances past `S` | Yes — this is the only positive test |
| **Drained** | `== S`, and `consumer_cursor > S` | Any producer, by claiming `S + CAPACITY` | Not from the stamp alone; needs the cursor |

**Three of the four states are stamp-indistinguishable, and that is the
design rather than a limitation.** Reusable and Written both read `< S`, and
the consumer's drain treats both identically: stop here. It has no reason to
tell them apart, because neither is drainable and no action differs between
them. Drained and Published both read `== S`; the difference is only visible
by also reading `consumer_cursor`, and only the producer side ever needs to,
when deciding whether claiming `S + CAPACITY` would overwrite live data. So
the stamp answers exactly one question — *may the consumer read this slot on
this lap?* — and the cursor answers the other — *may a producer overwrite
it?*. Two questions, two fields, no third state word.

**Why a boolean cannot encode this.** A `ready: bool` distinguishes
Published from Written within one lap and then fails at the lap boundary: a
Drained slot's flag still reads `true` from lap `L` when the consumer arrives
expecting lap `L + 1`, so stale-from-last-lap is indistinguishable from
published-just-now. Recovering the difference needs a monotonic value — a
sequence number — at which point the flag is redundant. The full argument,
including an earlier worked example written in flag form, is
[Sequence-Stamped Ring](../data_structure/001_sequence_stamped_ring.md)'s.

### Transitions

| # | From → To | Effected by | Field write | Concurrent with |
|---|-----------|-------------|-------------|-----------------|
| T1 | Reusable → Written | Producer, [Claim-Then-Publish](../algorithm/001_claim_then_publish.md) Step 1 | `producer_cursor` compare-exchange — **not** a write to this slot | Every other slot's T1/T2/T3 |
| T2 | Written → Written | Producer, Step 2 | `slots[i]` payload store, non-atomic | Every other slot; nothing else touches slot `i` |
| T3 | Written → Published | Producer, Step 3 | `stamps[i] = S`, `Release` | Every other slot; the consumer's T4 on *lower* sequences |
| T4 | Published → Drained | Consumer, [Batch Drain](../algorithm/002_batch_drain_by_cursor_swap.md) Step 5 | `consumer_cursor` advance past `S` — **not** a write to this slot | Every producer's T1/T2/T3 on other slots |
| T5 | Drained → Reusable | **Nobody** | None | — |

**T5 has no writer, and naming that is the point of this section.** A
Drained slot becomes Reusable for lap `L + 1` by the passage of the producer
cursor alone — nothing is reset, no stamp is cleared, no memory is touched.
The slot is simply reinterpreted against a larger `S`, and the same stamp
value that meant *Published on lap L* now means *below expected, not
readable* on lap `L + 1`. A cleanup pass that zeroed drained stamps would be
pure cost, and worse, would introduce a write to a slot the reclaiming
producer already believes it owns — a data race manufactured to service a
state machine that never needed the write.

**T1 and T4 write a cursor, not the slot.** Both transitions of the slot's
state are effected by writes to a *different* field, which is why the slot
array itself carries exactly one atomic write per lap (T3) and one non-atomic
write per lap (T2). That ratio — one atomic per element published, zero per
element drained — is the cost claim the two algorithm instances rest on.

**Ordering of T2 before T3 is the load-bearing edge.** T2 is a plain store
and T3 is a `Release` store; a reordering that lets T3 become visible first
publishes a slot whose payload has not landed, and the consumer reads
uninitialized memory with the stamp telling it the data is good. This is the
single unsoundness this state machine can express, and its enforcement is
[Publication Ordering](../invariant/002_publication_ordering.md)'s, not this
instance's.

### Behavioral Invariants

1. **One writer per slot per lap.** Between T1 and T3 for sequence `S`,
   exactly one thread may touch `slots[i]` — the producer whose
   compare-exchange won `S`. `claim` hands out each sequence once
   (→ [Single-Consumer Total Order](../invariant/001_single_consumer_total_order.md)),
   so this follows from T1's uniqueness rather than from any lock.
2. **The stamp is monotonic per slot.** Successive values of `stamps[i]`
   ascend by exactly `CAPACITY` across laps. Any observed decrease means a
   sequence was published twice or the cursor wrapped, both of which are
   corruption rather than a state.
3. **No state is skipped, and no state is re-entered within a lap.** The
   cycle is strictly T1 → T2 → T3 → T4 → T5, once per lap. This instance then
   said a producer that claims and never publishes leaves the slot Written
   forever and stalls the drain at `S` permanently, "stated here rather than
   defended: a panicking producer between T1 and T3 wedges the consumer."

   **T3 is now a destructor, so a slot cannot stay Written past the claiming
   scope.** `Producer::claim` returns a `Reserved` guard whose `Drop` performs
   T3, and a panic between T1 and T3 unwinds through that drop. The failure
   mode is not defended because it is no longer reachable: leaving the
   Written state is not something a producer can forget to do. What remains is
   that a slot is Written for as long as the guard is *held*, which stalls the
   drain for that duration — bounded by a stack value's lifetime rather than
   forever. `an_unpublished_claim_blocks_every_later_sequence_while_it_is_held`
   asserts the bound in both directions.
4. **A slot may be Published while lower-sequenced slots are Written.** T3 on
   sequence `S + 3` may complete before T3 on `S`. The drain's watermark scan
   is what turns this into a total order, by refusing to advance past the
   first non-Published slot even though later ones are readable
   (→ [Batch Drain](../algorithm/002_batch_drain_by_cursor_swap.md) Step 3).
   Publication is out of order; consumption is not.

**Closed: there is no fifth state.** The open question was whether *Poisoned* —
entered when a producer panics between T1 and T3 — is worth having at all,
noting it would need a distinct stamp value below `S`, a consumer policy for
skipping it, and a justification for why the resulting gap does not break the
total-order invariant. The last of those three was the real obstacle, and it
has no answer: a skipped sequence *is* a gap in the total order, so
*Poisoned* was never a fifth state of this machine but a different contract
wearing one.

It is not needed either way. The guard decision
(→ [Producer Attachment and Detachment](../lifecycle/002_producer_attachment_and_detachment.md)'s
M5, [Producer Publish Surface](../api/001_producer_publish_surface.md)'s
guard-versus-three-calls) means a panic between T1 and T3 performs T3 on the
way out, so the slot reaches Published like any other — carrying an **empty**
payload, since the consumer left it `Slot::clear`-ed at T5. An empty record is
an ordinary value of the payload type, not a state of the slot.

**This is the third of the "three documents, one choice" set**, and it closes
with them. The choice pays off best here: a fifth state would have added a
stamp value, a branch in the drain's hot loop, and a hole in the invariant,
and instead the machine keeps exactly four states.

`a_claim_dropped_without_a_write_publishes_an_empty_record` is the assertion.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_claim_then_publish.md](../algorithm/001_claim_then_publish.md) | Effects T1, T2, and T3 — the producer's whole share of this machine |
| [../algorithm/002_batch_drain_by_cursor_swap.md](../algorithm/002_batch_drain_by_cursor_swap.md) | Effects T4; its watermark scan is the Published test in the States table |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_sequence_stamped_ring.md](../data_structure/001_sequence_stamped_ring.md) | Owns the `stamps` and cursor fields this machine's states are computed from |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_single_consumer_total_order.md](../invariant/001_single_consumer_total_order.md) | Supplies T1's uniqueness, which is what makes Behavioral Invariant 1 hold without a lock |
| [../invariant/002_publication_ordering.md](../invariant/002_publication_ordering.md) | Enforces the T2-before-T3 edge; without it the Published state is a lie |

### State Machines

| File | Relationship |
|------|--------------|
| [004_ring_occupancy_between_cursors.md](004_ring_occupancy_between_cursors.md) | The ring-wide view; its Full state is what blocks T1 from starting a new lap on this slot |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_sequence_number.md](../type/001_sequence_number.md) | The value `S` is computed from; its lap-decomposition is what makes the four states distinguishable at all |

### Sources

| File | Relationship |
|------|--------------|
| `src/lib.rs` | The four states are computed rather than stored: `stamp( seq ) == seq` is Published, and `contiguous_end` is the scan that reads them. `Reserved`'s `Drop` is T3; `Batch`'s is T4 |

### Tests

| File | Relationship |
|------|--------------|
| `tests/mpsc_test.rs::a_stale_stamp_from_the_previous_lap_does_not_read_as_published` | The same slot index reports Published on lap `L` and not-readable on lap `L + 1` for the identical stamp value, with no write between — the T5-has-no-writer claim, asserted directly |
| `tests/mpsc_test.rs::the_drain_stops_at_the_first_unpublished_sequence_not_the_highest_published` | Publishing `S + 3` before `S` leaves the drain stopped at `S` — Behavioral Invariant 4 |
| `tests/mpsc_test.rs::a_claim_dropped_without_a_write_publishes_an_empty_record` | Why there is no fifth state: the panicking-producer case reaches Published carrying an empty payload |
| `tests/mpsc_test.rs::a_taken_record_leaves_its_slot_empty` | T5 — the consumer's `clear`, which is what makes the previous row's empty record empty |
| `tests/mpsc_test.rs::every_slot_is_reused_across_many_laps_without_loss_or_duplication` | The whole cycle, repeated past wraparound, where a lap-confusion bug would surface |

### MP34 — A Claimed Slot Is Readable Through Its Guard Before Publication

`a_claimed_slot_is_readable_through_the_guard_before_it_is_published` pins it.
The guard holds `&mut` to the slot, so read-back is sound and needs no
synchronization — no other participant may touch that slot until the stamp
lands.

Worth recording as a state in its own right: the lifecycle has a window in which
a record exists, is addressable, and is invisible to every reader, and the only
handle to it is the guard.

### MP35 — A Taken Record Leaves Its Slot Empty

The drain moves the record out rather than copying it, so the slot holds `S`'s
default afterwards. Without that, a slot past the read cursor would still own a
payload, and `every_record_written_is_destroyed_exactly_once` — the
exactly-once guarantee at the heart of the family — would depend on drop order
at teardown rather than on the drain.
