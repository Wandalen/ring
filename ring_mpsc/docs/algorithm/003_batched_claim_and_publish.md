# Algorithm: Batched Claim-and-Publish

### Scope

- **Purpose**: Give the producer a grant of k contiguous sequences per gate check and per exchange, so the contended step — the consumer-cursor read and the compare-exchange — is amortised over the group instead of paid per record.
- **Responsibility**: The batched producer procedure — claim up to `max`, write by offset, publish the whole grant on drop — and the cost shape it trades.
- **In Scope**: `Producer::claim_batch`, `Producer::push_batch`, `ReservedBatch`, and the per-sequence publish in its drop.
- **Out of Scope**: The claim loop itself, which is unchanged and stays in `ring_claim` (→ [Claim-Then-Publish](001_claim_then_publish.md)); the consumer's drain, which this procedure does not touch (→ [Batch Drain by Single Cursor Swap](002_batch_drain_by_cursor_swap.md)); a per-slot release protocol that would remove the consumer-cursor read from the claim's fast path entirely — a redesign of the stamp-equality rule (→ [`pitfall/002`](../pitfall/002_a_stale_stamp_reads_as_unpublished_not_as_wrong.md)), recorded as open rather than shipped speculatively.

### The procedure

1. **Claim up to `max`.** `ring_claim::Claimer::claim_up_to( max )` grants `1..=max` contiguous sequences with one headroom read of the consumer cursor and one compare-exchange — the same contended step `claim( 1 )` pays, once per group instead of once per record. The grant is *adaptive*: whatever headroom the gate allows at the value the exchange runs against, never more, and `Full` when nothing. A producer under pressure therefore keeps making progress at whatever width the ring allows instead of spinning until its full ask appears.
2. **Write by offset.** `ReservedBatch::slot_mut( offset )` reaches the slot of `start + offset`. Sequence → slot folds through `ring_index` per offset, so a grant crossing a lap addresses wrapped slots in issue order — the fold `ring_batch::drain_order` performs on the read side, run here on the write side. The writes carry no synchronization: the grant owns its range exclusively (MP53).
3. **Publish on drop.** The drop stores `stamp( seq ) = seq` with `PUBLISH` for **every** sequence of the grant, in issue order — k `Release` stores. A grant could not stop at its first unwritten sequence: that would wedge the drain behind it forever, the exact failure the guard shape exists to make impossible. An unwritten offset publishes an empty slot — the single guard's defined outcome, one slot at a time.

### The cost shape

| step | `claim( 1 )` per record | `claim_batch( k )` per grant |
|------|--------------------------|------------------------------|
| gate read (consumer cursor, Acquire) | one per record | **one per grant** |
| claim exchange (AcqRel CAS) | one per record | **one per grant** |
| publish (`Release` stamp store) | one per record | one per record |

The consumer's side is untouched. The equality scan already tolerates arbitrary per-producer publication order, and a within-grant publish is contiguous in issue order, so no new gap class appears — cross-producer gaps remain, and their cost is what the amortisation buys headroom against: fewer gate reads and exchanges per delivered record means a producer that does stall the consumer stalls it against fewer atomic operations, not more.

### What this procedure deliberately does not do

It does not remove the consumer-cursor read from the claim — the gate still needs the consumer's position, once per grant. Making the claim gate itself off the slot stamps (the way a producer discovers "full" without asking the consumer) requires the consumer to mark slots on commit, which rewrites `contiguous_end`'s equality rule and every pitfall built on it. Recorded as open; not shipped speculatively.

### MP53 — the batched grant is a write group, not a reservation

A held `ReservedBatch` parks the consumer behind its whole range: the drain stops at the grant's first unpublished sequence, which while the guard is alive is its first one. The single [`Reserved`](001_claim_then_publish.md) has the same property with k = 1; the batch multiplies the width of the window a slow writer can hold open. The contract is therefore claim, write, drop — and the drop publishes every sequence of the grant whether written or not, so the window always closes at the drop rather than at the writer's leisure.
