# Data Structure: Sequence-Stamped Ring

### Scope

- **Purpose**: Fix the field-level shape a claim-and-publish ring needs, including two properties left unspecified elsewhere — the per-slot stamp and the cursor cache-line separation.
- **Responsibility**: Define the four field groups, state which of their details are decided and which the family still owns, and record the false-sharing contract as a contract rather than a tuning note.
- **In Scope**: Slot array, per-slot sequence stamp, producer cursor, consumer cursor, and the operation set the two algorithm instances execute against them.
- **Out of Scope**: The procedures themselves (→ [Claim-Then-Publish Slot Acquisition](../algorithm/001_claim_then_publish.md), [Batch Drain by Single Cursor Swap](../algorithm/002_batch_drain_by_cursor_swap.md)); the atomic orderings the fields are accessed under (→ [Publication Ordering](../invariant/002_publication_ordering.md)); what a slot's payload bytes mean, which is the consumer's (→ [`ring_tls`'s POD payload requirement](../../../ring_tls/docs/non_functional_requirement/002_pod_pointer_free_payloads.md), the same payload-agnosticism stated for the sibling crate).

### Abstract

A sequence-stamped ring is a fixed-capacity array of payload slots, each
carrying its own sequence stamp, addressed by two cursors that live on
separate cache lines. Capacity is fixed at construction and never grows: the
structure allocates once and publishes forever without allocating again,
which is what makes its footprint a function of capacity rather than of
arrival rate.

The design this structure grew out of specifies it in three words — the
deleted predecessor crate's prospective backing store was
"`crossbeam::queue::SegQueue` or a **cache-aligned ring buffer**", with no
field list, no stamp, and no statement of what is aligned to what; no source
specifies ring sizing, wraparound/overwrite handling, or multi-consumer
coordination either. This instance is where those three words become a
structure. Two of its claims are
therefore originated here rather than transcribed: that the per-slot stamp
carries the *sequence number* rather than a readiness flag, and that "cache
aligned" means the two cursors specifically — not the payload cells, which
must go the opposite way.

### Structure

| Field | Type | Meaning |
|-------|------|---------|
| `slots` | Fixed-capacity array of payload cells — `UnsafeCell<[ MaybeUninit< T >; CAPACITY ]>`-shaped, densely packed with no per-cell padding | The claimed-and-written region; a producer owns exactly one cell for exactly one lap (→ [Claim-Then-Publish Slot Acquisition](../algorithm/001_claim_then_publish.md) Steps 2–3) |
| `stamps` | One atomic sequence stamp per slot — `AtomicSeq`, deliberately **unpadded** | The publication token *and* the lap discriminator; a slot's stamp equals the sequence number published into it, never a boolean. Padding it would cost 64× the payload array |
| `producer_cursor` | `AtomicSeq`, cache-line padded, alone on its line | The claim counter every producer exchanges against; monotonic, never decreasing, never reset. Owned by `ring_claim`'s `Claimer` rather than by `Ring` directly |
| `consumer_cursor` | `AtomicUsize`, cache-line padded, on a **different** line from `producer_cursor` | How far the single consumer has drained; also the signal that releases slots for reuse |

#### Why the stamp is a sequence number, not a ready flag

A slot has three states the consumer must tell apart, and a boolean tells
apart only two:

| Slot state | Stamp value | How the consumer decides |
|---|---|---|
| Published, this lap | Exactly the sequence expected for this slot on this lap — `slot_index + lap × CAPACITY` | Stamp equals the expected sequence → readable |
| Claimed, not yet published | Whatever the previous lap left, or `UNSTAMPED` | Stamp ≠ the expected sequence → not readable; the drain stops here |
| Empty or stale — never written, or holding the previous lap's element | `UNSTAMPED`, or the previous lap's sequence | Same test as above; ≠ the expected sequence → not readable |

The second and third rows collapsing into one test is correct rather than
lossy: not-yet-published and not-written are the same thing to a consumer,
and neither is drainable.

**The test is equality, not ordering, and that is stronger than this section
originally stated.** The rows above read "*below* the expected sequence" until
a later revision, which is an ordering comparison and invites a sentinel that must be
ordered against every slot's expectation
(→ [Ring Construction and Teardown](../lifecycle/001_ring_construction_and_teardown.md),
where that framing cost an initialization scheme). Under equality, a stale
stamp fails for the same reason a never-written one does — it is not the value
being looked for — so no ordering relation between sentinel and expectation is
needed at all. `contiguous_end` is `if self.stamp( end ).load( OBSERVE ) != end
{ break; }`.

A separate `is_ready: bool` flag fails on the third row specifically. After
one full lap, a stale slot's flag still reads `true` from the previous lap's
publication, and the consumer cannot distinguish "published just now" from
"published one capacity ago" without a second field carrying — a sequence
number. This is the same reuse-confusion shape that makes a bare pointer
comparison unsafe in a lock-free structure; the stamp closes it by
construction, because a sequence number is unique across all laps by
[Claim-Then-Publish Slot Acquisition](../algorithm/001_claim_then_publish.md)
Step 1 and a flag is unique across none. An earlier worked example is
written in exactly the flag form (`ring[head].data = 500; ring[head].is_ready
= true;`), which makes the flag the shape a reader is most likely to reach
for; naming why it is insufficient is this section's reason to exist.

**Resolved: 64 bits, and a separate parallel array.**

The width question was `usize`-versus-`u32`: 64 bits never wraps in any
realistic runtime — at one claim per nanosecond it lasts about 585 years — but
doubles the stamp array's memory traffic against a `u32`, which wraps after
~4.3 × 10⁹ claims and needs explicit wraparound handling no source specifies.
It is settled one crate down: `ring_types::Seq` is a `u64` newtype, so the
choice was made for the family rather than per-crate — and `u64` rather than
`usize` also means the 585-year argument holds on a 32-bit target instead of
collapsing to ~4 seconds there, which the original framing would have missed.
The `u32` saving remains available to a future revision and remains
unmeasured; what is no longer available is making it here alone.

The layout question — separate parallel array versus inline per-slot header —
is implemented as **separate**: `stamps : Box< [ AtomicSeq ] >` beside
`slots : Buffer< UnsafeCell< S > >` — the cell on each slot rather than around
the whole buffer, because the outer form made every producer's write claim the
entire allocation and so alias every other producer's (see `ring_spsc`'s
[Two-Cursor Ring](../../../ring_spsc/docs/data_structure/001_two_cursor_ring.md)
for the Miri diagnosis that shape produces). This is genuinely the unmeasured half,
kept as a live question rather than presented as a finding. Separate keeps
payload cells dense for the consumer's sequential read; inline would put stamp
and payload on one cache line so the drain's stamp probe warms the payload it
is about to read. Separate was taken because `Buffer< S >` already exists and
already owns dense slot storage, so it is the arrangement that reuses a tested
crate rather than the arrangement that won an argument — an implementation
convenience, and named as one; the family's own harness still has the
measurement to make.

#### Cursor cache-line separation is a contract, not an optimization

`producer_cursor` and `consumer_cursor` are 8 bytes each. Declared adjacently
they land in the same 64-byte cache line, and the structure still passes
every functional test — which is exactly why this belongs in the field
contract rather than in a performance note. The mechanism:

1. A producer's claim is a read-modify-write, so it must take the containing
   cache line into **exclusive** state on its core.
2. That invalidates every other core's copy of the line — including the
   consumer's, which is reading `consumer_cursor` on the same line for
   reasons entirely unrelated to the producer.
3. The consumer's next access misses and must re-fetch across the
   interconnect. So does the next producer's.

Under 16 producers this is 16 forced invalidations per lap of pure
bookkeeping, on a line neither party actually shares data through. Earlier
design discussion names this precisely, and locates it precisely: the false
sharing a Disruptor-shaped design fights does not happen on the data array —
it happens on the counters (sequence numbers), which is what actually needs
to be spread apart to 64 bytes. The padding is what makes the two cursors
independent variables in hardware and not merely in source.

**And the inverse, which is the part that gets applied backwards.** Padding
each *payload cell* to a cache line is the natural-seeming symmetric move and
is wrong: an 8-byte element padded to 64 bytes inflates a 1 M-slot ring from
8 MB to 64 MB, and makes the consumer pull 64 bytes from RAM for every 8
useful ones — destroying the sequential-read bandwidth the batch drain
depends on (same source, "Hell Circle 1"). Cells stay packed.

**One honest gap in that reasoning, which this crate must not inherit
silently.** The reasoning above justifies dense cells by observing that in a
*thread-local* buffer there is no false sharing on the data cells themselves,
because only one core writes to that ring. That premise does not hold here.
This is a shared multi-producer ring: sequences `n` and `n + 1` resolve to
adjacent slots, so two different producers can be writing two cells on the
same cache line at the same instant — genuine false sharing on the data
array, which the thread-local case structurally cannot have. Dense packing is
therefore a deliberate trade for this crate (RAM bandwidth on the read side
against write-side coherence traffic), not the free lunch it is for
[`ring_tls`](../../../ring_tls/docs/data_structure/001_thread_local_append_log.md).
Where the trade lands depends on element size relative to a cache line and on
producer count, so it is left **open** and named as a case the family's own
harness should reproduce rather than assume away.

### Operations

| Operation | Contract | Cost discipline |
|-----------|----------|------------------|
| `claim` | Compare-exchange loop on `producer_cursor`, fused with the gating headroom check; returns a sequence no other caller receives, ascending in hardware-serialization order, or `RingError::Full` | One hardware RMW per attempt, ~20 ns uncontended, **lock-free** — the fused capacity check makes the loop unavoidable, a conclusion already ruled on at the family level |
| `write` | Store the payload into `slots[ sequence % CAPACITY ]`; no atomic, no synchronization — the caller is the slot's sole writer for this lap | ~500 ns and fully parallel across producers; the whole reason `claim` is the only serialized step |
| `publish` | Store `sequence` into `stamps[ sequence % CAPACITY ]`; the slot becomes drainable at this instant and the producer's ownership ends | One atomic store, `Release` (→ [Publication Ordering](../invariant/002_publication_ordering.md)) |
| `drain` | Advance `consumer_cursor` to the published watermark, walk the claimed range ascending, release the slots | Amortized ≪ 1 atomic per element; a plain store rather than a compare-exchange, because the cursor has exactly one writer |
| `free_capacity` | `CAPACITY - ( producer_cursor - consumer_cursor )`, the room a claim may safely consume | Read-only; the quantity a backpressure policy acts on (→ [Bounded Capacity and Backpressure Policy](../non_functional_requirement/002_bounded_capacity_backpressure.md), policy undecided) |

**No operation on this structure allocates.** The slot array, the stamp
array, and both cursors are allocated once at construction; a claim reuses a
slot rather than obtaining one. That is the structural difference from the
heap-node MPSC this crate would replace — the deleted predecessor crate's
own mechanism, where each `push` leaks one `Box::into_raw` node and each
drain reclaims it — and it is why capacity, and not arrival rate, bounds
this structure's footprint.

**Decided versus open, stated plainly.** Decided by the family
and by this instance: fixed capacity, per-slot sequence stamp, two
cursors, cursors on separate cache lines, cells packed. Open, and each one a
case for the harness rather than an argument: capacity itself, power-of-two
constraint, stamp width, stamps inline versus parallel, whether cells warrant
padding under this crate's genuinely-shared write pattern, and whether the
consumer cursor doubles as the reclamation signal or a second cursor trails
it (→ [Batch Drain by Single Cursor Swap](../algorithm/002_batch_drain_by_cursor_swap.md)
Step 5).

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_claim_then_publish.md](../algorithm/001_claim_then_publish.md) | Fetch-adds `producer_cursor`, writes one `slots` cell, stores one `stamps` entry |
| [../algorithm/002_batch_drain_by_cursor_swap.md](../algorithm/002_batch_drain_by_cursor_swap.md) | Advances `consumer_cursor` and scans `stamps` for the published watermark |
| [../../../ring_tls/docs/algorithm/001_tagged_record_bump_append.md](../../../ring_tls/docs/algorithm/001_tagged_record_bump_append.md) | The single-writer sibling's append procedure — the direct contrast for why this structure's slots carry a sequence stamp its region does not need |

### Data Structures

| File | Relationship |
|------|--------------|
| ~~the deleted predecessor crate's mailbox structure~~ | The heap-node MPSC this structure would replace — same producer/consumer roles, footprint bounded by arrival rate rather than by capacity. **Deleted 2026-08-26**, and it has no successor. |
| [../../../ring_tls/docs/data_structure/001_thread_local_append_log.md](../../../ring_tls/docs/data_structure/001_thread_local_append_log.md) | The sibling crate's uncontended per-thread region — the single-writer case whose dense-packing justification this structure explicitly cannot borrow |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_single_consumer_total_order.md](../invariant/001_single_consumer_total_order.md) | The contract the stamp discriminator and the single consumer cursor exist to enforce |
| [../invariant/002_publication_ordering.md](../invariant/002_publication_ordering.md) | Governs how `stamps` and both cursors are accessed; a correct field layout with wrong orderings is still unsound |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../../../ring_tls/docs/non_functional_requirement/002_pod_pointer_free_payloads.md](../../../ring_tls/docs/non_functional_requirement/002_pod_pointer_free_payloads.md) | The payload constraint stated for the sibling crate; a slot cell here is subject to the same question, unresolved for this crate |
| [../non_functional_requirement/001_measured_before_adopted.md](../non_functional_requirement/001_measured_before_adopted.md) | The gate every "open at this grain" item above is deferred to |
| [../non_functional_requirement/002_bounded_capacity_backpressure.md](../non_functional_requirement/002_bounded_capacity_backpressure.md) | Owns what `free_capacity == 0` does, and why fixed capacity is a requirement rather than a sizing choice |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_channel_to_ring_binding.md](../pattern/001_channel_to_ring_binding.md) | Its Rule 2 binds a declared channel's slot type to this structure's `slots` field — already fixed per ring instance, which is why the shared-T path costs this structure nothing today |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_spinning_consumer_owns_a_core.md](../pitfall/001_spinning_consumer_owns_a_core.md) | A polled `producer_cursor` is a contended cache line by the same mechanism the padding contract above describes — the padding separates producer from consumer, not producer from poller |

### Sources

| File | Relationship |
|------|--------------|
| `src/lib.rs` | Defines `Ring`'s three fields (`slots`, `stamps`, `consumers`) and the `unsafe impl Sync` that makes the shared-slot arrangement sound |

### Tests

| File | Relationship |
|------|--------------|
| `tests/mpsc_test.rs::a_stale_stamp_from_the_previous_lap_does_not_read_as_published` | The three-state discrimination, in the case a boolean flag would get wrong |
| `tests/mpsc_test.rs::stamps_start_unstamped_and_there_is_exactly_one_per_slot` | The stamp array's shape and initial value — one per slot, `UNSTAMPED` |
| `tests/mpsc_test.rs::the_claim_cursor_and_the_consumer_cursor_are_on_distinct_cache_lines` | The separation contract, asserted by address arithmetic rather than trusted to `ring_cursor` |
| `tests/mpsc_test.rs::every_slot_is_reused_across_many_laps_without_loss_or_duplication` | The lap discriminator working across wraparound, which is the whole reason the stamp is a sequence |
| `tests/mpsc_test.rs::the_debug_rendering_names_the_ring_state_a_reader_would_want` | That the fields above are observable for debugging without exposing the unsafe interior |

### MP9 — The `UnsafeCell` Is Per Slot Because the Outer Form Was Unsound

The field documentation records the history: reaching a slot through
`( *cell.get() ).at_mut( seq )` claims exclusive access to the *whole* buffer,
so two producers writing two different slots aliased everything. Miri reported
it against `Buffer< S >` rather than against any slot — the producers never
touched the same record.

**The wrong version passes every behavioural test.** No record is lost, no
ordering violated; the defect is a claim about aliasing that no assertion can
observe. That is the argument for running Miri on this crate at all, and it is
the strongest available evidence that the unsafe here needs a tool rather than
a review.

### MP10 — The Outer Form Became Constructible Only by Loosening a Bound

The unsound outer form was not chosen — it was the only one that compiled,
because `Buffer::new`'s `Slot` bound excluded `UnsafeCell< S >`. Relaxing that
bound to `Default` is what made the sound per-slot form expressible.

**A type bound in a sibling crate was holding this crate's storage in an unsound
shape**, and nothing connected the two. Recorded because the dependency graph
shows `ring_store` below `ring_mpsc` and gives no hint that a bound there
constrains soundness here.
