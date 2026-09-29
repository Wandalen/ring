# Lifecycle: A Sequence From Published to Committed

### Scope

**Purpose:** Trace one sequence number through the states this crate can observe
or cause, and establish which transitions are guarded and which are not.

**Responsibility:** The states a `Seq` occupies from the consumer's point of
view, the operation causing each transition, and what enforces it.

**In Scope:** The four states `ring_consume` can distinguish; the two it causes;
the window between them.

**Out of Scope:** The producer-side states before publication — `ring_claim` and
`ring_publish` own those. The consumer's own lifecycle, which is
[`002`](002_the_consumer_over_a_rings_life.md).

---

## The States

From this crate's vantage point a sequence is in exactly one of four states, and
the crate can observe the boundary between each pair:

| State | Test | Caused by |
|-------|------|-----------|
| **Unpublished** | `seq >= barrier.frontier()` | — the producer has not reached it |
| **Available** | `position <= seq < frontier` | the producer advancing its cursor |
| **Read** | *not observable* | the caller, between `available` and `commit` |
| **Committed** | `seq < position` | `commit` or `commit_available` |

```
  unpublished ──producer publishes──▶ available ──caller reads──▶ read
                                                                    │
                                          reusable ◀──commit────────┘
```

### CN30 — Three of the Four Transitions Are Observable and the Load-Bearing One Is Not

The crate can see a sequence become available: the frontier moved.
It can see one become committed: its own cursor moved.
It cannot see one become **read**, because reading happens in the caller's
buffer and this crate never sees the buffer.

That single unobservable transition is where every hazard in the crate lives:

| Hazard | Which unobservable fact it depends on |
|--------|---------------------------------------|
| committing before reading ([`decisions/001`](../decisions/001_two_calls_not_one.md) CN7) | whether the read happened |
| `commit_available` after a partial read ([`api/002`](../api/002_the_two_commits.md) CN20) | how much was read |
| never committing at all | whether the caller is still reading or has stopped |

All three are the same gap wearing different clothes. The crate's entire
contract is "here is what you may read" and "tell me when you have", and it has
no way to check the second claim against reality. A caller that lies — by
commission, by omission, or by arithmetic error — is believed.

That is not a fixable defect. A ring buffer's consumer is *defined* by being the
thing that reads the buffer; a primitive that could verify the read would have
to own the buffer, which would make it a different crate. `ring_gating` and
`ring_barrier` are similarly blind by construction.

What is recordable is that the module documentation frames this correctly and
only once. Its opening argument — *everything between the two calls is a read of
borrowed slots* — is exactly the statement that the read is unobservable and the
caller is trusted. Nothing in the API documentation of `commit` or
`commit_available` repeats it, so a reader who arrives at either function
without having read the module header sees a range check and no mention that the
check verifies range and not provenance.

**Cost:** reachable, and irreducible. The gap cannot be closed; the finding is
that it is stated in one place and the two functions it applies to do not
reference it.

---

### CN31 — "Committed" and "Reusable" Are the Same Moment, Seen From Two Sides

The diagram above shows `commit` producing a *reusable* state, and that is the
producer's word for it. From the consumer's side the same store means "I am done
with this." From the producer's side, reading the same cursor through its gating
set, it means "you may overwrite this."

There is no delay, no acknowledgement, and no intermediate state. The instant
`self.cursor.store( through, COMMIT )` retires, a producer that loads that cursor
may write the slot.

Two consequences that the crate's documentation states separately and never
together:

**The `Release` ordering is what makes the two readings agree.** The store is
`Release` and the producer's load is `Acquire`
([`non_functional_requirement/002`](../non_functional_requirement/002_eleven_constants_and_the_one_that_is_shared.md)),
so everything the consumer did before the commit — every read of the buffer —
happens-before the producer's subsequent write. Drop the ordering and the two
sides disagree about what "committed" means at exactly the moment it matters.

**There is no way to un-commit.** `commit`'s lower bound refuses backwards
movement ([`invariant/002`](../invariant/002_the_cursor_only_moves_forward.md)),
which is correct, and means a consumer that commits and then discovers it
mis-read has no recovery. The data is gone the moment the producer laps.

That second point is the one worth surfacing, because it changes how the
`commit` / `commit_available` choice should be made. The two functions look like
a convenience decision — do I want to name a sequence or not — and are actually
an irreversibility decision: `commit_available` gives up everything currently
available, permanently, in one call, with no argument to get wrong and no error
to catch. A caller uncertain how much it read should call neither until it is
certain.

Nothing in either function's documentation says the operation is irreversible.

**Cost:** reachable. The irreversibility is real, correct, and unstated; the
easier of the two calls is the one that discards more.

---

## What Is Correctly Absent

| Not present | Correctly so |
|-------------|--------------|
| a "read" state in the type system | the crate never sees the buffer; the state is the caller's |
| an un-commit or rollback | the producer may already have overwritten; there is nothing to roll back to |
| a per-sequence status query | the states are ranges, not per-item; a query would be `position` and `frontier` |
| an acknowledgement from the producer | the cursor *is* the acknowledgement, in both directions |

---

## Cross-References

| Definition | Instance | Relationship |
|------------|----------|--------------|
| lifecycle | [002](002_the_consumer_over_a_rings_life.md) | the consumer's own states |
| decisions | [001](../decisions/001_two_calls_not_one.md) | the window the unobservable transition sits in |
| api | [002](../api/002_the_two_commits.md) | the choice CN31 reframes as irreversibility |
| invariant | [002](../invariant/002_the_cursor_only_moves_forward.md) | why there is no way back |
| non_functional_requirement | [002](../non_functional_requirement/002_eleven_constants_and_the_one_that_is_shared.md) | the ordering that makes both readings agree |

### Sources

| What | Where |
|------|-------|
| The available bound | `ring_consume/src/lib.rs:93-96` |
| The commit store | `ring_consume/src/lib.rs:433,475` |
| The window argument | `ring_consume/src/lib.rs`, module doc |
| The producer's read of the same cursor | `ring_gating/src/lib.rs` |

### Tests

| Claim | Verified by |
|-------|-------------|
| Availability grows as publication advances | `consume_test.rs:97` |
| Availability shrinks as the consumer commits | `consume_test.rs:112` |
| The cursor is what the producer gates on | `consume_test.rs`, `the_consumer_cursor_is_what_the_producer_would_gate_on` |
| Every sequence is offered exactly once | `consume_test.rs`, `every_sequence_is_offered_exactly_once_across_a_full_drain` |
