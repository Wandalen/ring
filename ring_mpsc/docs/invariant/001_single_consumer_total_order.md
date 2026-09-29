# Invariant: Single-Consumer Total Order

### Scope

- **Purpose**: Fix the property this crate exists to provide — many uncoordinated producers, one merged order the consumer can trust and a replay can reproduce.
- **Responsibility**: State the contract-grain property, what any winning mechanism must supply to enforce it, and what a violation costs each prospective consumer.
- **In Scope**: Publication visibility, exactly-once delivery, the total order, and reproducibility of that order.
- **Out of Scope**: The concrete claim/publish mechanism and its memory-ordering discipline (owned at the family level, undecided); whether and when consumers migrate onto this crate (also undecided).

### Invariant Statement

For any set of producer threads publishing elements concurrently, the single
consumer drains every published element **exactly once** — none lost, none
duplicated — in **one total order** consistent with each element's sequence
number, and two drains over the same publication history yield the **same
order**.

Three clauses, each independently load-bearing:

- **Publication atomicity.** An element is either fully published — visible
  to the consumer together with its sequence number — or not visible at all.
  No partially-written element is ever drained.
- **Exactly-once delivery.** The consumer observes each published element
  once. A ring slot is never handed to the consumer twice, and a claimed slot
  whose publication completed is never skipped.
- **Reproducible order.** The drained order is a pure function of the
  publication history (the sequence numbers), never of scheduler timing,
  drain batching, or how many drain calls the history was consumed across.

### Enforcement Mechanism

The mechanism splits into what the family already fixes and
what its own verdict still owns — this doc states the boundary rather than
pretending the open half is settled:

**Decided (the family's own definition of the crate):**

- Every element carries a **sequence number claimed atomically before
  publication** — "a sequence-numbered ring merging many producer threads
  into one consumer-side total order" is the crate's identity, not a
  candidate.
- The drained order **is** sequence order. No post-drain sort, no
  per-producer sub-queues surfacing in a nondeterministic interleave.
- **One consumer.** The single-consumer restriction is what makes the drain
  side lock-free without consumer-side coordination; a second consumer is out
  of contract, not a degraded mode.

**Resolved in a later revision** (this section previously read "Open (TBD — supplied by the
winning pattern, benched under the family's own harness)"; the two items below are now
implemented, and what remains open is whether the implemented mechanism *wins*
its comparison, not what it is):

- The claim mechanism — **closed: a CAS loop, and not by preference.**
  [Claim-Then-Publish Slot Acquisition](../algorithm/001_claim_then_publish.md)
  specified the `fetch_add` variant because that is what the Disruptor
  material it drew on describes. A `fetch_add` claim on a *bounded*
  ring hands out sequences past the consumer's tail with no wait-free undo, so
  the free-capacity check must be fused into the claim, and a fused
  check-and-claim is a `compare_exchange` loop, measured against this crate's
  own assumption and ruled on at the family level. The original material was
  not wrong; it describes an unbounded-claim Disruptor, and the boundedness is
  this crate's own requirement.
- Capacity and backpressure policy — **closed: bounded capacity, `Fail`
  policy.** A claim on a full ring returns `RingError::Full`.
  [Bounded Capacity and Backpressure Policy](../non_functional_requirement/002_bounded_capacity_backpressure.md)
  required a bounded footprint, a policy declared per priority class, and no
  overwriting of any element this invariant's exactly-once clause covers.
  `Fail` satisfies all three, and is the only one of the four candidates that
  can be *built on* — Block and Drop-newest are three lines in a caller over
  a failing claim, while the reverse recovery is impossible. Per-class policy
  stays where the class is known, above this crate.
- The exact atomic memory-ordering annotations proving publication
  atomicity — **no longer open, and no longer the family's own verdict to supply.**
  [Publication Ordering](002_publication_ordering.md) states them as
  contract in both directions, because an ordering discipline is a
  correctness requirement every candidate must satisfy, not a performance
  variant a candidate gets to choose.

### Violation Consequences

Each prospective consumer turns a violated clause into a distinct
silent failure:

- **A lost element is a lost intent.** The prospective consumer's use is
  intent submission, where the log is the source of truth — an intent that
  was published but never drained diverges world state with nothing erroring
  anywhere.
- **A duplicated element is a double-applied intent.** A despawn applied
  twice, a balance delta summed twice — the same lost-update/double-apply
  class the deleted predecessor's own pitfall named, reproduced at intake
  grain.
- **A non-reproducible order breaks deterministic replay.** The prospective
  consumer's correctness thesis requires the merged intent order to be a pure
  function of what was submitted; a merge order that varies run-to-run
  defeats that guarantee without any single element being wrong.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_claim_then_publish.md](../algorithm/001_claim_then_publish.md) | Supplies the unique monotonic sequence number the total order is stated over, narrowing the claim-mechanism item above |
| [../algorithm/002_batch_drain_by_cursor_swap.md](../algorithm/002_batch_drain_by_cursor_swap.md) | Enforces all three clauses on the drain side — the watermark for publication atomicity, cursor arithmetic for exactly-once, the ascending walk for reproducibility |

### Data Structures

| File | Relationship |
|------|--------------|
| ~~the deleted predecessor crate's mailbox structure~~ | The working hand-rolled CAS-stack MPSC this crate's mechanism would replace internally — its atomic-batch-drain requirement is this invariant's drain side. **Deleted 2026-08-26**, and it has no successor. |
| [../data_structure/001_sequence_stamped_ring.md](../data_structure/001_sequence_stamped_ring.md) | The per-slot stamp and single consumer cursor this contract's clauses are enforced through |

### Invariants

| File | Relationship |
|------|--------------|
| ~~the deleted predecessor crate's own invariant~~ | The non-blocking property the predecessor mechanism holds; any winning ring must keep it while adding the sequence-order clause. **Deleted 2026-08-26**, and it has no successor. |
| [002_publication_ordering.md](002_publication_ordering.md) | The mechanism this contract's publication-atomicity and exactly-once clauses actually rest on; closes the memory-ordering item above |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_measured_before_adopted.md](../non_functional_requirement/001_measured_before_adopted.md) | The adoption gate — a mechanism enforcing this invariant still doesn't ship until it wins the benchmark |
| [../non_functional_requirement/002_bounded_capacity_backpressure.md](../non_functional_requirement/002_bounded_capacity_backpressure.md) | Constrains the capacity/backpressure item above — an overwrite policy applied to a contract-bearing element is a violation of the exactly-once clause, not a configuration |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_channel_to_ring_binding.md](../pattern/001_channel_to_ring_binding.md) | Its Rule 1 holds a Disruptor ring open to as many readers as needed; this invariant's single-consumer clause is why that half never activates for this crate regardless |

### Pitfalls

| File | Relationship |
|------|--------------|
| ~~the deleted predecessor crate's own pitfall~~ | The lost-write class a violated exactly-once clause reproduces one layer down. **Deleted 2026-08-26**, and it has no successor. |
| [../pitfall/001_spinning_consumer_owns_a_core.md](../pitfall/001_spinning_consumer_owns_a_core.md) | Defends the single-consumer clause with the measured Amdahl figure rather than leaving it as an unexplained restriction |

### Sources

| File | Relationship |
|------|--------------|
| `src/lib.rs` | Implements this contract; `Consumer`'s `!Clone`/`!Sync` shape is where the one-consumer clause stops being a convention and becomes a compile error |

### Tests

| File | Relationship |
|------|--------------|
| `tests/mpsc_test.rs::four_producers_exchange_one_hundred_thousand_items_with_byte_parity` | All three clauses at once under real contention: every element drained exactly once (multiset parity), no sequence granted twice, and each producer's own items in issue order |
| `tests/mpsc_test.rs::the_drain_stops_at_the_first_unpublished_sequence_not_the_highest_published` | The total-order clause's load-bearing mechanism — drained order *is* sequence order because the drain refuses to skip a gap |
| `tests/mpsc_test.rs::every_slot_is_reused_across_many_laps_without_loss_or_duplication` | Exactly-once across wraparound, where a lap-confusion bug would show as totals that still reconcile |
| `tests/mpsc_test.rs::every_record_written_is_destroyed_exactly_once` | Exactly-once in the destructor sense, with a drop-counting payload |
| `src/lib.rs` `compile_fail` doc tests | The one-consumer clause: `Consumer` is neither `Clone` nor `Sync`, asserted the only way a negative can be |

### MP22 — Single-Consumer Is Enforced by Borrowing, Not by a Type

There is no `ConsumerToken`, no runtime flag, no `Once`. The uniqueness is the
borrow checker's, and it is load-bearing for the `unsafe impl Sync`
(→ [`../workaround/002`](../workaround/002_an_unsafe_impl_sync_the_compiler_cannot_derive.md)),
whose first safety clause cites exactly these two receivers.

**A soundness argument resting on two function signatures** is worth recording as
such: changing `ends` to `&self` would compile, would look like a convenience,
and would invalidate the `Sync` impl.

### MP23 — `both_ends_and_the_handles_they_split_into_name_one_ring`

A pairing bug — handles onto two rings — produces no error and no lost record in
a single-threaded test; each half works. The named test compares the `ring()`
pointers, which is the only way the defect is observable.

`ring()` is `pub const fn` on three types for this reason, and it is one of the
few observation methods that does have a caller: this test.
