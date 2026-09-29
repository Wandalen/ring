# Algorithm: Batch Drain by Single Cursor Swap

### Scope

- **Purpose**: Specify the consumer-side procedure that claims an entire published run in one atomic operation instead of one atomic per element, and name the published-watermark problem that decides whether that claim is sound.
- **Responsibility**: State the drain steps, why no order fix-up step exists, and which of the three candidate watermark detections is used — the last resolved in a later revision in favour of the per-slot stamp scan.
- **In Scope**: `ring_mpsc`'s single consumer: how far it may safely read, how it claims that range, and the order it walks it in.
- **Out of Scope**: How elements got into the range (→ [Claim-Then-Publish Slot Acquisition](001_claim_then_publish.md)); the `Ordering` arguments the steps below omit (→ [Publication Ordering](../invariant/002_publication_ordering.md)); when a drain is triggered at all, which is the caller's and not this crate's (→ [The Spinning Consumer Owns a Core](../pitfall/001_spinning_consumer_owns_a_core.md)); multi-consumer or broadcast fan-out, which this crate's contract excludes outright (→ [Single-Consumer Total Order](../invariant/001_single_consumer_total_order.md)).

### Abstract

The consumer does not walk the ring element by element, testing each one. It
reads how far publication has reached, advances its own cursor to that point
in a single store, and then walks the range it just claimed with no further
atomics on the hot path. Draining message-by-message via an iterator would
cost one atomic operation per message; the consumer instead grabs the entire
pending batch in a single operation. Over a run of `N` published elements the
amortized atomic cost per element is `1/N` rather than `1`.

Two steps that a linked-list MPSC needs are absent here, and their absence is
the point rather than an optimization. There is **no reversal**: the
sequence-numbered claim already produced arrival order
(→ [Claim-Then-Publish Slot Acquisition](001_claim_then_publish.md) Step 1),
where a CAS-prepend stack produces LIFO and must be walked once to undo it
(the deleted predecessor crate's own push/drain mechanism, Step 2). And
there is **no per-node deallocation**: slots are reused in
place, where the stack's drain reconstitutes and drops one `Box` per message.

The load-bearing difficulty is not in any of the steps. It is that the
producer cursor is a *claim* cursor and not a *publication* watermark, and
the two are the same variable only when there is exactly one producer.

### Algorithm

**Input** — `&mut self` or an owning consumer handle (single-consumer, so
exclusivity is available and no compare-exchange is needed on the consumer
cursor at all), a caller-owned sink for the drained elements.

**Output** — every element published before this call's watermark read is
delivered to the sink exactly once, in sequence order; the consumer cursor
names the first sequence not yet delivered.

**Step 1 — read the drainable watermark.** Determine the highest sequence
`w` such that every sequence in `[ consumer_cursor, w )` is published. This
is the step the whole algorithm turns on, and it is **not** a plain read of
the producer cursor — see the next section.

**Step 2 — short-circuit.** If `w == consumer_cursor`, nothing is drainable;
return without touching any shared state. This is the common case on a
continuously-polled ring and the reason a poll must be cheap
(→ [The Spinning Consumer Owns a Core](../pitfall/001_spinning_consumer_owns_a_core.md)).

**Step 3 — claim the range in one store.** Advance `consumer_cursor` to `w`.
One store, not a compare-exchange: the single-consumer restriction means this
variable has exactly one writer, so there is no race to lose
(→ [Single-Consumer Total Order](../invariant/001_single_consumer_total_order.md),
where the restriction is contract rather than a degraded mode). The range
`[ old_cursor, w )` now belongs to this drain, and no subsequent drain will
see it — which is the exactly-once clause, enforced by cursor arithmetic
rather than by per-element bookkeeping.

**Step 4 — walk the claimed range in sequence order.** For each sequence `s`
in `[ old_cursor, w )` ascending, read the payload at `s % capacity` and hand
it to the sink. No atomic is touched inside this loop. Reading ascending is
what makes the delivered order the sequence order, and therefore what makes
it reproducible across a different number of drain calls over the same
publication history — the third clause of
[Single-Consumer Total Order](../invariant/001_single_consumer_total_order.md),
which a batch drain could otherwise silently break by letting batch
boundaries leak into the observed order.

**Step 5 — release the slots.** Publish the advanced `consumer_cursor` so
producers may reuse the range's slots on the next lap. This is a second
handoff in the opposite direction, and it carries its own ordering
requirement, distinct from publication's
(→ [Publication Ordering](../invariant/002_publication_ordering.md)'s
reclamation pair). Whether Step 3's store and this step are one store or
two — i.e. whether the consumer cursor doubles as the reclamation signal, or
a separate `read_watermark` trails it — was **open**: one variable is simpler
and one fewer cache line, two variables let the consumer claim a range without
immediately conceding its slots, which matters if the sink borrows from the
ring rather than copying out of it.

**Resolved: one variable, and the second variable's benefit is obtained
without it.** The concern was real — the sink *does* borrow from the ring
(→ [Consumer Drain Surface](../api/002_consumer_drain_surface.md)'s borrow
decision) — but it is answered by *when* the single store happens rather than
by adding a second cursor. Steps 3 and 5 are not two moments in one call:
Step 3 computes the range and hands back a `Batch`, and Step 5 is that
`Batch`'s `Drop`. So the consumer holds its range without conceding the slots
for exactly as long as the borrow lives, and the borrow checker is what
enforces the gap that a second cursor would have had to represent explicitly.
`a_live_batch_still_holds_its_slots_against_reuse` asserts it.

#### The published-watermark problem

Step 1 cannot read the producer cursor. Producers claim in a serialized
order and then publish in an *arbitrary* one, because everything between
claim and publish — the ~500 ns payload write — is deliberately not
serialized. So:

1. Producer A claims sequence 7. Producer B claims sequence 8.
2. B is scheduled, finishes its write, and stamps slot 8 published.
3. A is descheduled mid-write. Slot 7 holds nothing valid.
4. The producer cursor now reads 9.

Draining to the producer cursor would deliver slot 7 — which contains either
uninitialized memory or, worse, the *previous lap's* fully-formed element,
one capacity older and entirely plausible. The drainable prefix therefore
ends at **the first unpublished slot**, not at the claim cursor. A single
in-flight producer is enough to make the two diverge, and nothing bounds how
long the gap persists: a producer preempted by the OS between Steps 3 and 4
of the claim path stalls the watermark for a full scheduling quantum, no
matter how many later sequences publish behind it.

This is precisely the problem a single-producer ring does not have. With one
producer there is no way to leave a hole behind, so the claim cursor *is* the
watermark and one `Acquire` load of it suffices — which is exactly the shape
an earlier single-producer worked example uses. No source specifies a ring
at all for the multi-producer case, so the detection below is this crate's
own question rather than a choice among documented options.

**How the watermark is detected — resolved in a later revision: the per-slot stamp scan.**
Three candidates were named, each trading against a different property this
crate claims:

| Candidate | Mechanism | What it costs |
|---|---|---|
| Per-slot stamp scan | Walk forward from `consumer_cursor`, loading each slot's stamp, stop at the first slot whose stamp is not the sequence expected for it on this lap | One atomic load per candidate slot at the boundary, so the "single atomic operation" claim above holds only for the *claim*, not the *detection*; the scan is `O(run length)` and re-reads the stopped-at slot every poll |
| Producer-advanced watermark | A second cursor producers advance after stamping, but only once their own predecessor has published | Restores a true one-atomic drain, but makes a producer's publication **block on a peer producer's progress** — which is the one thing a contended-claim ring exists to avoid. This is exactly what [`ring_publish`](../../../ring_publish/readme.md)'s `Publisher` does, and why it is not used here |
| Availability array | A separate per-slot lap/availability entry the consumer probes, the classical LMAX arrangement | A second memory stream the consumer touches per slot, competing for cache with the payload it is about to read |

**The first won, and it was named here as "the default to beat."** It is what
[Sequence-Stamped Ring](../data_structure/001_sequence_stamped_ring.md)'s
per-slot stamp already makes possible with no extra field, and `contiguous_end`
in `src/lib.rs` is that walk. Its stated cost is paid as described: the "single
atomic operation" claim holds for the *claim*, not the *detection*, the scan is
`O(run length)`, and the stopped-at slot is re-read on every poll.

The second was ruled out on grounds stronger than measurement, which is why
this closed without waiting for the family's own harness: it
reintroduces peer-blocking on the publish path, and an earlier ruling
records that `ring_publish` — the crate that implements it — declines this
crate's case in writing, calling it "`ring_mpsc`'s problem at S5." The third
remains genuinely unmeasured and is a live alternative for a future revision;
it trades a cache stream against a scan, which is the kind of question only a
benchmark settles.

The requirement this doc fixes is unchanged and is what the resolution had to
satisfy: the drain must stop at the first unpublished slot by *some*
mechanism, never at the claim cursor.

#### What a batch drain must not become

Batching interacts with two constraints that are easy to lose:

- **Batch boundaries must not be observable.** Draining `[0, 5)` then
  `[5, 9)` must deliver the same order as draining `[0, 9)` once. Step 4's
  ascending walk is what guarantees this; any per-batch grouping, sorting, or
  parallel sub-drain would break the reproducibility clause without any
  single element being wrong
  (→ [Single-Consumer Total Order](../invariant/001_single_consumer_total_order.md)).
- **A second consumer is not a faster consumer.** Adding a reader that also
  advances the consumer cursor does not double drain throughput; it splits
  the element stream between the two, each seeing a fraction. Earlier design
  discussion names this failure and its fix — per-reader cursors, a
  broadcast ring, with the producer pacing against the slowest reader.
  That shape is deliberately **out of contract** here: this crate
  fixes one consumer, and a broadcast variant would change the backpressure
  question from "is the ring full" to "has the slowest reader fallen a lap
  behind", which no invariant in this crate currently covers.

### Algorithms

| File | Relationship |
|------|--------------|
| ~~the deleted predecessor crate's push/drain mechanism~~ | The single-swap drain this one generalizes — same one-atomic batch claim, minus the reversal pass and the per-node `Box::from_raw`. **Deleted 2026-08-26**, and it has no successor. |
| [001_claim_then_publish.md](001_claim_then_publish.md) | The producer counterpart whose deliberately-unserialized payload write is what creates the hole this algorithm must stop at |
| [../../../ring_tls/docs/algorithm/001_tagged_record_bump_append.md](../../../ring_tls/docs/algorithm/001_tagged_record_bump_append.md) | The producer-side append this drain's watermark eventually reads past |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_sequence_stamped_ring.md](../data_structure/001_sequence_stamped_ring.md) | The consumer cursor Step 3 advances and the per-slot stamps the first watermark candidate scans |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_single_consumer_total_order.md](../invariant/001_single_consumer_total_order.md) | The three clauses this procedure enforces: publication atomicity via the watermark, exactly-once via cursor arithmetic, reproducibility via the ascending walk |
| [../invariant/002_publication_ordering.md](../invariant/002_publication_ordering.md) | Supplies Step 1's `Acquire` and Step 5's `Release`, the two halves of the handoff this algorithm sits between |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_bounded_capacity_backpressure.md](../non_functional_requirement/002_bounded_capacity_backpressure.md) | Consumes this algorithm's output rate — how far the watermark trails the claim cursor is exactly the fill level a backpressure policy acts on |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_channel_to_ring_binding.md](../pattern/001_channel_to_ring_binding.md) | Names this drain's unbounded per-call range as a difference from the channel language's `batch` cap, and cites this file's own multi-reader exclusion as the reason Rule 1's reader-count half does not apply here |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_spinning_consumer_owns_a_core.md](../pitfall/001_spinning_consumer_owns_a_core.md) | Step 2's short-circuit is the branch a spinning consumer takes on nearly every iteration; the pitfall is what to do instead of iterating |

### Sources

| File | Relationship |
|------|--------------|
| `src/lib.rs` | `Consumer::drain`/`drain_up_to` are Steps 1–3, `Batch`'s accessors are Step 4, and its `Drop` is Step 5; `contiguous_end` is the watermark scan |

### Tests

| File | Relationship |
|------|--------------|
| `tests/mpsc_test.rs::the_drain_stops_at_the_first_unpublished_sequence_not_the_highest_published` | The published-watermark problem's worked example, asserted: sequence 8 published while 7 is held delivers nothing |
| `tests/mpsc_test.rs::a_second_drain_sees_what_was_published_after_the_first` | That draining the same history in several calls yields the same order as one — no batch-boundary leakage, and nothing delivered twice |
| `tests/mpsc_test.rs::a_live_batch_still_holds_its_slots_against_reuse` | Step 5 as a destructor: the slots are conceded when the borrow ends, not when `drain` returns |
| `tests/mpsc_test.rs::a_commit_restores_exactly_the_capacity_it_released` | Step 5's cursor advance, checked by exact accounting rather than by the drain appearing to work |
| `tests/mpsc_test.rs::exhaustive::the_consumer_never_drains_further_than_the_producers_published` | The watermark bound under `loom`, exhaustively rather than at whatever interleaving the scheduler picked |

### MP3 — The Drain Stops at the First Gap, Not the Highest Published

`the_drain_stops_at_the_first_unpublished_sequence_not_the_highest_published`
pins it, and `an_unpublished_claim_blocks_every_later_sequence_while_it_is_held`
states the consequence.

This is what "total order" costs (→ [`../invariant/001`](../invariant/001_single_consumer_total_order.md)):
records leave in claim order, so a producer holding a claim holds the whole
stream. Under a frame budget that is a latency cliff rather than a throughput
one — every other producer's records are already written and still undelivered.

**The published-but-undrained window is invisible from the consumer side.**
`available()` reports what is drainable, not what is written, so a consumer
cannot distinguish "nothing published" from "one producer is slow".

### MP4 — `drain_up_to` Is Capped Rather Than Scanning Past the Ring

`drain_up_to_more_than_capacity_is_capped_rather_than_scanning_past_the_ring`
and `drain_up_to_zero_takes_nothing_and_leaves_the_records_drainable` are the
two boundary tests, and together they make the parameter total: every `usize`
is a legal argument.

Worth recording because the obvious implementation is not total — a scan of
`max` sequences from the read position walks past the ring's own bound for
large `max`, and the stamps would then be read for sequences no producer has
reached.
