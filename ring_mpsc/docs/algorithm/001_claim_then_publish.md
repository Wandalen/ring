# Algorithm: Claim-Then-Publish Slot Acquisition

### Scope

- **Purpose**: Specify the producer-side acquisition procedure that buys a privately-owned slot for the cost of one hardware atomic, so the expensive half of publishing never serializes against another producer.
- **Responsibility**: State the four producer steps — claim, locate, write, publish — the cost split that motivates their order, and why no retry loop appears on this path.
- **In Scope**: `ring_mpsc`'s producer side: the sequence claim, the slot the claim resolves to, and the stamp that ends the producer's ownership of it.
- **Out of Scope**: The consumer's side of the same slot (→ [Batch Drain by Single Cursor Swap](002_batch_drain_by_cursor_swap.md)); the memory-ordering annotations every step below deliberately omits (→ [Publication Ordering](../invariant/002_publication_ordering.md)); the fields claimed against (→ [Sequence-Stamped Ring](../data_structure/001_sequence_stamped_ring.md)); what happens when the claim lands on a slot the consumer has not drained (→ [Bounded Capacity and Backpressure Policy](../non_functional_requirement/002_bounded_capacity_backpressure.md) — `Fail`, as of a later revision).

### Abstract

A producer takes exclusive write access to one ring slot by atomically
advancing a shared sequence counter and keeping the value that was there
before. That read-modify-write costs about **20 ns**, and it is the only part
of publishing that any two producers serialize against each other. Everything
after it — the payload write, about **500 ns**, roughly 25× the ticket's cost
— runs fully in parallel, because distinct tickets resolve to distinct slots
and distinct slots never collide.

That ratio is the whole design. The pattern deliberately serializes the
cheap instruction and parallelizes the expensive one, rather than the
reverse shape a lock produces, where the ~500 ns write sits inside the
critical section and every additional producer adds a full write's worth of
waiting. Under 16 producers the serialized component is ~320 ns of ticket
issuance per lap regardless of payload size; under a lock protecting the
write itself it would be ~8 µs and would grow with the payload.

**This instance originally specified `fetch_add` and claimed wait-freedom;
both are corrected here.** The original text read: "The claim also needs no
compare-and-swap retry loop … `fetch_add` always succeeds and always returns a
value no other caller received, so a producer completes its claim in a bounded
number of steps no matter how many other producers are contending. The
vocabulary for this is **wait-free**."

The reasoning is sound and the conclusion does not survive **bounded**
capacity, which the Disruptor material it drew on does not impose at the claim:

> **A `fetch_add` claim hands out sequences past the consumer's tail, and
> there is no wait-free undo.**

So the free-capacity check must be fused into the claim, and a fused
check-and-claim is a `compare_exchange` loop by construction. `ring_claim`
implements exactly that; the crate is **lock-free**, not wait-free — the
measurement and the ruling, already made at the family level — and
[Ring Occupancy Between the Cursors](../lifecycle/004_ring_occupancy_between_cursors.md)'s
Behavioral Invariant 3 is the same fact seen as a state-observability problem.

**The original material is not wrong; it is describing a different claim.**
Its distinction stands as written — wait-free means every thread finishes in
a finite, predictable number of steps, as against lock-free, where the system
progresses but an individual thread may lose its compare-exchange race
arbitrarily many times. What was imported along with the vocabulary was an
unstated premise: that the claim is unbounded. The deleted predecessor
crate's working mailbox mechanism sits on the lock-free side of that line (a
`compare_exchange_weak` retry loop) — and so, it turns out, does this crate.
The advantage over the mailbox is real but narrower than claimed: this
crate's CAS loop contends on one cursor with an early `Full` exit, while the
mailbox's contends on a stack head with unbounded retry.

**The 25× ratio, which is the actual design argument, is untouched by any of
this.** It concerns what sits inside the serialized region versus outside it,
not how the serialization is achieved. A CAS loop that retries under
contention costs more than a `fetch_add` that does not, and both are still
~20 ns against a ~500 ns payload write.

### Algorithm

**Input** — `&self` (a shared reference; callable from any thread with no
exclusive access and no coordination with any other producer), the payload
to publish.

**Output** — the payload occupies exactly one slot, that slot's stamp names
the sequence number the payload was published under, and no other producer
wrote to that slot between the claim and the stamp.

**Step 1 — claim.** `claimer.claim( 1, &consumers )?` — a `compare_exchange`
loop that re-reads the gating headroom on each attempt and returns
`RingError::Full` when there is none. The winning exchange's *expected* value
is the sequence handed out, so two concurrent callers receive `n` and `n + 1`,
never the same number. Two properties follow directly from the exchange's
atomicity, and neither needs any further mechanism: sequence numbers are
**unique**, and they are **monotonically increasing** in the order the
hardware serialized the successful exchanges. That serialization order *is*
this crate's arrival order — there is no shared clock among producers and no
other candidate definition (the same linearization-point argument the
deleted predecessor crate made for its own CAS-win order).

This step read `producer_cursor.fetch_add( 1, … )` until a later revision. The substitution
changes the progress class and nothing else: a losing exchange retries against
a cursor another producer has already advanced, so it re-reads and tries the
next sequence rather than corrupting anything. Both properties above are
consequences of atomicity, not of which atomic. See the Abstract for why the
substitution is forced rather than chosen.

**Step 2 — locate.** `let slot = sequence % capacity;` — a mask
(`sequence & ( capacity - 1 )`) where capacity is a power of two. Step 1's
uniqueness survives this reduction only **within one lap of the ring**:
sequences `n` and `n + capacity` resolve to the same slot. Slot ownership is
therefore unique per lap, not unconditionally, and the entire question of
what happens on the second lap is the backpressure policy's
(→ [Bounded Capacity and Backpressure Policy](../non_functional_requirement/002_bounded_capacity_backpressure.md)).
Capacity **is** constrained to a power of two, so the mask is what runs:
`ring_types::Capacity` refuses anything else at construction. The trade the
constraint forecloses — non-power-of-two sizing that a measured burst depth
might want — is real and is argued in full by
[Capacity](../type/002_capacity.md), which is where a relaxation would happen,
one crate down and for the whole family at once.

**Step 3 — write the payload.** The producer writes into the located slot
with no atomic operation and no synchronization of any kind — it is the sole
writer of that slot for this lap, which Step 1 established. This is the
~500 ns half, and it is the reason the ordering above is what it is: the
claim is short enough to serialize and the write is long enough that
serializing it would dominate.

**Step 4 — publish.** Store `sequence` into the slot's own stamp. The stamp
store *is* the publication event: before it, the slot is indistinguishable
from stale to the consumer; after it, the slot is drainable
(→ [Sequence-Stamped Ring](../data_structure/001_sequence_stamped_ring.md)
for why the stamp carries the sequence rather than a boolean flag). Nothing
else marks completion, and the producer's ownership of the slot ends here.

**On orderings.** Steps 1–4 above are stated without their `Ordering`
arguments on purpose. Getting them wrong compiles, passes tests on x86, and
fails on weakly-ordered hardware — measured at 14 failures in 60 runs on this
workspace's own aarch64 host, which is not a future target — so the
annotations are contract, stated once, in
[Publication Ordering](../invariant/002_publication_ordering.md), rather
than restated per step here where a reader could take them for incidental
detail.

#### Why the ring beats a CAS-prepend mailbox on this path

The deleted predecessor crate's mailbox mechanism solves the same
many-producers/one-consumer problem with a Treiber-style stack: each `push`
heap-allocates a node and compare-exchanges it onto the head. The comparison
is not "one atomic versus none" — both pay one atomic per message on the
producer side. Three things differ:

| | CAS-prepend stack | This algorithm |
|---|---|---|
| Producer atomic | `compare_exchange_weak` in a loop, contending on a stack head | `compare_exchange` in a loop, contending on one cursor with an early `Full` exit |
| Progress class | Lock-free | Lock-free — this row read "Wait-free; bounded steps per producer" until a later revision; see the Abstract |
| Resulting order | LIFO — newest CAS-win first | Sequence order — arrival order directly |
| Consumer fix-up | An in-place chain reversal before the walk | None; the reversal step does not exist |
| Per-message allocation | One `Box::into_raw` node, unbounded in aggregate | None; the slot array is allocated once |

The reversal is the sharpest of these, because it is invisible in a
throughput microbenchmark and structural in a correctness argument: the
stack cannot produce arrival order at all without a full pass over the
drained chain, and that pass is on the single consumer's critical path
(→ [Batch Drain by Single Cursor Swap](002_batch_drain_by_cursor_swap.md)).
The per-message allocation is the second sharpest — it is what makes the
mailbox's memory footprint a function of arrival rate rather than of
capacity
(→ [Bounded Capacity and Backpressure Policy](../non_functional_requirement/002_bounded_capacity_backpressure.md)).

Neither observation decides anything. Whether the ring actually beats the
mechanism it would replace is
[Measured Before Adopted](../non_functional_requirement/001_measured_before_adopted.md)'s
question and the family's own harness to answer; this section states what the
benchmark is expected to be measuring, not its result.

**This instance narrowed an open item to the wrong candidate, and the
narrowing is what made that visible.**
[Single-Consumer Total Order](../invariant/001_single_consumer_total_order.md)
listed the claim mechanism as open among three — CAS loop, `fetch_add` cursor,
Disruptor-style sequence barriers. This doc specified the second in full,
because it is the one the Disruptor material it drew on describes down to
the instruction.

**Implemented: the first.** Specifying `fetch_add` "in full" is exactly what
surfaced the incompatibility — a vaguer commitment ("some atomic claim") would
have been quietly satisfied by the CAS loop and nobody would have noticed the
wait-free guarantee downstream in
[the producer surface](../api/001_producer_publish_surface.md) had become
false. Being specific enough to be wrong is the mechanism that worked here.

The third candidate, Disruptor-style sequence barriers, remains genuinely
unmeasured and is what the family's own harness compares
against. A verdict favouring it is still a verdict against this instance
rather than a detail below it.

### Algorithms

| File | Relationship |
|------|--------------|
| ~~the deleted predecessor crate's push/drain mechanism~~ | The CAS-prepend producer path this one is contrasted against and would replace. **Deleted 2026-08-26**, and it has no successor. |
| [../../../ring_tls/docs/algorithm/001_tagged_record_bump_append.md](../../../ring_tls/docs/algorithm/001_tagged_record_bump_append.md) | The sibling crate's append path — the uncontended per-thread case that needs no claim step at all, because it has no second producer to be unique against |
| [002_batch_drain_by_cursor_swap.md](002_batch_drain_by_cursor_swap.md) | The consumer counterpart — reads the slots this procedure stamps, and inherits the hole problem Step 1's out-of-order completion creates |

### Data Structures

| File | Relationship |
|------|--------------|
| ~~the deleted predecessor crate's mailbox structure~~ | The heap-node MPSC structure whose producer path this algorithm's cost table compares against. **Deleted 2026-08-26**, and it has no successor. |
| [../data_structure/001_sequence_stamped_ring.md](../data_structure/001_sequence_stamped_ring.md) | The cursor Step 1 exchanges against, the slot Step 2 resolves to, and the stamp Step 4 writes |

### Invariants

| File | Relationship |
|------|--------------|
| ~~the deleted predecessor crate's own invariant~~ | The non-blocking property the mechanism being replaced holds; this algorithm was believed to strengthen it from lock-free to wait-free on the producer path; it does not — both are lock-free (see the Abstract). **Deleted 2026-08-26**, and it has no successor. |
| [../invariant/001_single_consumer_total_order.md](../invariant/001_single_consumer_total_order.md) | Step 1's unique monotonic sequence is what the total-order clause is stated over |
| [../invariant/002_publication_ordering.md](../invariant/002_publication_ordering.md) | Supplies the `Ordering` arguments Steps 1–4 deliberately omit, and states why they are contract rather than detail |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_measured_before_adopted.md](../non_functional_requirement/001_measured_before_adopted.md) | The gate the ~20 ns / ~500 ns cost argument must survive as a measurement rather than an argument |
| [../non_functional_requirement/002_bounded_capacity_backpressure.md](../non_functional_requirement/002_bounded_capacity_backpressure.md) | Owns what Step 2's second lap does when the claim lands on an undrained slot |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_spinning_consumer_owns_a_core.md](../pitfall/001_spinning_consumer_owns_a_core.md) | A continuously-polling consumer keeps Step 1's cursor line contended, making every exchange here more expensive than an unwatched one |

### Sources

| File | Relationship |
|------|--------------|
| `src/lib.rs` | Implements Steps 1–4: `Producer::claim` is Step 1, `Reserved`'s `DerefMut` is Step 2–3, and its `Drop` is Step 4 |

### Tests

| File | Relationship |
|------|--------------|
| `tests/mpsc_test.rs::four_producers_exchange_one_hundred_thousand_items_with_byte_parity` | Step 1's uniqueness under sustained contention — four concurrent claimers each receive a unique, contiguous sequence, checked as a set rather than a count |
| `tests/mpsc_test.rs::the_claim_advances_before_the_publish_and_the_two_are_separate_cursors` | The Step 1 / Step 4 split, which is the whole shape of this algorithm — the claim cursor moves at Step 1 and the drainable frontier only at Step 4 |
| `tests/mpsc_test.rs::a_claim_past_capacity_reports_full_rather_than_overwriting` | Step 1's `Full` exit, which is what the fused headroom check buys and what a `fetch_add` could not have provided |
| `tests/mpsc_test.rs::a_claimed_slot_is_readable_through_the_guard_before_it_is_published` | Step 3's unsynchronized write into a solely-owned slot, observed from the owner |
| `tests/mpsc_test.rs::exhaustive::a_published_record_is_never_observed_before_the_write_that_preceded_it` | The Step 3 → Step 4 edge under `loom`, where the ordering this section defers to the invariant is actually checked |

### MP1 — The Claim Cursor Is Advanced by `ring_claim`, Not Here

The algorithm reads as one procedure and is split across three crates. What is
in `src/lib.rs` is the stamp store, the slot write, and the guard type; the
claim's own compare-and-advance and the headroom check gating it against the
consumer cursor are `ring_claim::Claimer::claim`'s.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_mpsc
printf 'compare_exchange here:  '; grep -c 'compare_exchange' src/lib.rs
printf 'fetch_ in code:         '; grep -vE '^\s*(//|///|//!)' src/lib.rs | grep -c 'fetch_'
printf 'ring_claim uses:        '; grep -c 'ring_claim\|Claimer' src/lib.rs
```

Live output:

```
compare_exchange here:  0
fetch_ in code:         0
ring_claim uses:        12
```

**Zero read-modify-writes in this file.** A reader looking here for the atomic
that makes multi-producer claiming work will not find one — which is the
dependency seam working, and worth stating because the crate's name suggests
otherwise.

### MP2 — A Claim Dropped Without a Write Still Publishes

`a_claim_dropped_without_a_write_publishes_an_empty_record` is the test, and its
name states the contract: the guard publishes on drop regardless of whether the
caller wrote anything.

**That is the right design and it is a sharp edge.** The alternative — not
publishing on an unwritten drop — would leave a permanent hole in the sequence
space that every later consumer blocks behind, because the drain stops at the
first unpublished sequence. Publishing a default record keeps the stream
contiguous.

The cost is that a producer which claims and then takes an early return path
silently emits a record. Nothing in the type marks the drop as significant —
`Reserved` is not `#[ must_use ]`, and being a guard it could not usefully
be.

**Disposition:** declined — `Reserved`'s own doc comment
(`ring_mpsc/src/lib.rs:927-931`) already states this exact tradeoff in
these words: "A guard dropped without a write publishes an empty slot, not a
torn one... That is why no completion flag is tracked: there is nothing for
it to prevent." The three plausible repairs are each ruled out by the crate's
own existing reasoning: a `written`-flag warning on drop would fight the
design rather than fix it (an intentional zero-payload publish is a valid use,
not a bug); requiring an explicit `.publish()` call instead of publish-on-drop
is the exact regression the type comment says the guard shape exists to
prevent ("would otherwise wedge the ring permanently"); and `#[ must_use ]`
cannot express "used but not written to," which the finding's own last
sentence already concludes. Nothing left to change without reopening a
tradeoff the crate already made deliberately and documented in full.
