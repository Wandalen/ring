# Pitfall: The Spinning Consumer Owns a Core

### Scope

- **Purpose**: Name the cost that arrives with the ring's canonical consumer shape, before a tick-bounded workload spends a full core polling a queue that receives work once per frame.
- **Responsibility**: State the trap, quantify the waste with the source's own tick figures, give the mitigation, and name the opposite regime where the mitigation is itself the mistake.
- **In Scope**: When the consumer's drain is triggered, and what the choice costs in each of the two workload regimes this crate's prospective consumers actually span.
- **Out of Scope**: What the drain does once triggered (→ [Batch Drain by Single Cursor Swap](../algorithm/002_batch_drain_by_cursor_swap.md)); why one consumer rather than several (→ [Single-Consumer Total Order](../invariant/001_single_consumer_total_order.md)); what a full ring does while the consumer is not draining (→ [Bounded Capacity and Backpressure Policy](../non_functional_requirement/002_bounded_capacity_backpressure.md)).

### Trap

**The ring has no triggering event.** A producer's publish is a store into a
slot; nothing wakes anybody. So the consumer's canonical shape — the one that
arrives with the pattern, in every description of it — is a `while( true )`
poll:

```rust
loop
{
  let w = published_watermark();          // Step 1 of the drain
  if w == consumer_cursor { continue; }   // Step 2's short-circuit, ~always
  drain_to( w );
}
```

This is a defining property of the pattern rather than an implementation
choice: unlike a thread-local-buffering approach, which wakes once per frame
to batch-walk every thread's own queue, this pattern's consumer has no
equivalent triggering event and needs a continuous `while(true)` polling
loop to notice newly claimed slots as they land, occupying a full core for
the pattern's entire runtime.

The trap is not that the loop is wrong. It is that it comes *with* the
pattern, so it gets adopted along with the pattern, into a workload that
never needed it — and its cost is invisible in a throughput benchmark, which
measures elements per second on a machine where the extra core was free.

### Failure

**On a tick-bounded workload, that core is pure waste, and the arithmetic is
not close.** A representative single-machine tick: a main compute phase of
roughly **10 ms** followed by a merge of roughly **0.1 ms**. A spinning
consumer is busy for 10.1 ms out of every 10.1 ms in order to do 0.1 ms of
work — a duty cycle of about **1%**, with roughly **99% of one core burned**
on the short-circuit branch. On a representative hardware budget of 16 to 32
cores with no GPU, that is between 3% and 6% of the machine's total compute,
spent observing that nothing has changed.

**And it is worse than an idle core, which is the part that surprises.** The
poll's repeated load keeps the producer cursor's cache line in shared state
on the consumer's core. Every producer claim is a read-modify-write, so it
must take that line exclusive — invalidating the poller's copy — and the
poller's next iteration pulls it back. A hot-polled cursor is therefore
*more expensive to claim against* than an unwatched one — and more so than
this section assumed, since each claim is a compare-exchange loop whose
losing attempts each re-take the line, a correction already made at the
family level, rather than one unconditional read-modify-write: the
consumer's observation is itself coherence traffic on the exact line the
producers contend for, by the same mechanism that makes cursor padding a
contract
(→ [Sequence-Stamped Ring](../data_structure/001_sequence_stamped_ring.md),
whose padding separates producer from consumer — not producer from poller,
which no padding can). No source states this for this case; it is recorded
as a derivation rather than a citation.

Two second-order costs compound it on shared hardware: a spinning core holds
a thermal and boost budget the compute cores would otherwise draw on, and on
a scheduler that cannot tell a spin from useful work, the consumer competes
with the very producers it is waiting for.

### Mitigation

**Drive the drain from the tick boundary.** Where the workload already has a
frame or tick, call the drain once per tick — exactly the shape a
thread-local-buffering approach uses, and the property this pattern is
contrasted against above. The consumer occupies a core for the 0.1 ms it is
merging and releases it for the other 10 ms; the batch is larger, so the
drain's amortized atomic cost per element falls rather than rises
(→ [Batch Drain by Single Cursor Swap](../algorithm/002_batch_drain_by_cursor_swap.md)).

**Why one sequential consumer is nevertheless the right shape — the Amdahl
figure, which is the argument people skip.** The reflex objection to a single
consumer is that it is a serialization point and therefore the bottleneck.
Measuring it settles the question rather than arguing it. Amdahl's Law does
bound the system by its sequential part, and the merge's cost scales with
*total* system-wide output rather than per-core output — 16 cores generating
100,000 events each leaves one core to process 1.6 M sequentially. But at
the representative tick above, that sequential part is 0.1 ms against 10 ms
of parallel compute: about **1% overhead** — short enough to be considered
negligible, which is why an engine can accept a single-threaded merge rather
than building a more complex parallel one. The single consumer is not the
bottleneck it is assumed to be — which is what makes the spin, not the
serialism, the cost actually worth removing.

**Intermediate positions exist, and none is decided.** Spin-then-park (a
bounded spin, then a futex or condvar wait) trades wake-up latency for CPU;
`core::hint::spin_loop()` keeps the shape but cuts the spin's issue rate and
power draw; core pinning bounds which core is sacrificed without reducing the
sacrifice. These sit on a latency-versus-CPU continuum, no source names any
of them, and where the point belongs is a measurement the family's own
harness would make.

#### The opposite regime, where the mitigation is the mistake

**On a continuous, frame-less pipeline the spin is correct and a
tick-triggered drain is wrong.** There is no tick to trigger on, and
introducing one imposes exactly the latency the workload exists to avoid: a
drain at 10 ms cadence adds up to a full tick of delay per element, about
5 ms on average, entirely inside the ring. For a frame-discrete workload that
delay is free, because the result is not consumed until the next frame
anyway. For a continuous pipeline it is the dominant cost, and it is
self-inflicted.

Earlier design discussion draws exactly this line and draws it as the
pattern's real boundary — the domain split is streaming-continuous versus
frame-discrete, not simply "which is faster" in the abstract — naming
continuous order flow as the case where this pattern is still fastest, since
the alternative's merge phase pays an Amdahl-bounded cost this pattern never
incurs at all. That domain's own ruling is not a decision this crate has
adopted; it is cited here only for the general principle.

**Both regimes appear among this crate's own prospective consumers**, which
is why the choice cannot be made once here:

| Regime | Correct trigger | Wrong trigger costs |
|---|---|---|
| Frame/tick-discrete — a fixed-cadence simulation or actor tick | Drain once per tick boundary | Spinning burns ~99% of a core at a ~1% duty cycle |
| Continuous / frame-less — an unbounded event pipeline with no natural boundary | Continuous poll | Tick-triggering adds ~½ tick of latency per element, in the one workload where latency is the product |

**The API consequence, which is this pitfall's actual output.** `ring_mpsc`
must not bake a drain trigger into the ring. A non-blocking drain that
returns immediately when the watermark has not moved serves both regimes,
because the caller supplies the loop or the tick.

**Held, and under its own name rather than `try_drain`'s.** `Consumer::drain`
*is* the non-blocking drain this section asks for — it scans, swaps the
cursor, and returns, yielding an empty `Batch` when nothing is published. The
`try_` prefix was dropped because there is no non-`try` sibling to distinguish
it from: a prefix that contrasts with nothing is noise, and its absence is
what tells a reader there is no blocking variant to reach for by accident.
`draining_an_empty_ring_yields_an_empty_batch_and_moves_nothing` is the
assertion, and it checks both halves — the empty result *and* that the cursor
did not move.

**Resolved, and neither horn of the dilemma was taken.** The open question was
whether the crate *also* ships a blocking or parking consumer: shipping it
makes the easy path the parked one, wrong for continuous workloads; not
shipping it makes every continuous consumer hand-roll a spin, which is where
this document's mistakes get made. Both horns assume the choice is
`ring_mpsc`'s to make. It is not — the family already has
[`ring_wait`](../../../ring_wait/readme.md) ("wait strategies for space and
data availability") and [`ring_poll`](../../../ring_poll/readme.md)
("non-blocking progress helpers") as separate crates, so a parking consumer is
a composition a caller assembles rather than a variant this crate ships or
withholds.

That is a better answer than either horn, and the reason is worth stating
because it generalizes: **the dilemma was real only while the unit of
decision was assumed to be one crate.** Shipping-versus-withholding is a
forced choice inside a single API; across a family it is a placement question,
and placement has a third answer. This instance could not have found it — the
crates it names did not exist when it was written — which is the same lesson
[Family Dependency Seam](../integration/001_family_dependency_seam.md)
records from the other direction.

What survives unchanged is the mitigation table above: this crate still bakes
in no trigger, and the caller still supplies the loop or the tick.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_claim_then_publish.md](../algorithm/001_claim_then_publish.md) | The publish that wakes nobody — the absence this trap starts from |
| [../algorithm/002_batch_drain_by_cursor_swap.md](../algorithm/002_batch_drain_by_cursor_swap.md) | The drain being triggered; its Step 2 short-circuit is the branch a spinning consumer takes on nearly every iteration |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_sequence_stamped_ring.md](../data_structure/001_sequence_stamped_ring.md) | The producer cursor a poll keeps contended — padding separates producer from consumer, and cannot separate producer from poller |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_single_consumer_total_order.md](../invariant/001_single_consumer_total_order.md) | The one-consumer restriction the Amdahl figure defends as adequate rather than merely mandated |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_bounded_capacity_backpressure.md](../non_functional_requirement/002_bounded_capacity_backpressure.md) | Drain cadence sets the reclamation rate — a tick-triggered drain makes the full-ring policy reachable in normal operation rather than only under pathology |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../../../ring_tls/docs/pitfall/001_implicit_thread_locals_are_hidden_state.md](../../../ring_tls/docs/pitfall/001_implicit_thread_locals_are_hidden_state.md) | Same category: a mechanism that quietly claims a process-wide resource |

### Sources

| File | Relationship |
|------|--------------|
| `src/lib.rs` | `Consumer::drain` and `Consumer::drain_up_to` return immediately in every case; there is no loop, no park, and no trigger anywhere in the crate — the absence is the design |
| `../../../ring_wait/readme.md` | Where a parking strategy belongs, if a caller wants one — the third answer to this pitfall's open question |
| `../../../ring_poll/readme.md` | The non-blocking-progress sibling of the same split |

### Tests

| File | Relationship |
|------|--------------|
| `tests/mpsc_test.rs::draining_an_empty_ring_yields_an_empty_batch_and_moves_nothing` | The non-blocking drain, in the case a blocking one would wait: nothing published, so an empty batch and an unmoved cursor rather than a park |
| `tests/mpsc_test.rs::a_second_drain_sees_what_was_published_after_the_first` | That repeated drains are how a caller supplies its own trigger — the loop is the caller's, and it composes |
| `tests/mpsc_test.rs::drain_up_to_zero_takes_nothing_and_leaves_the_records_drainable` | The degenerate trigger: a caller that decides this tick is not the tick pays nothing and loses nothing |

### MP44 — The Crate Offers No Wait, Which Is Why the Pitfall Is Reachable

The absence is deliberate — `ring_wait` owns waiting — but the consequence lands
on whoever composes them. A caller reaching for this crate directly gets a
non-blocking drain and no hint that spinning on it is the documented trap.

**The pitfall is a composition hazard documented in the crate that cannot fix
it.** That is the right place for the description and the wrong place for the
remedy, which is why `ring_wait` exists.

### MP45 — Nothing Measures the Spinning Consumer's Cost

A pitfall stated as "owns a core" is a quantitative claim. `ring_bench` measures
throughput and latency; neither reports consumer CPU occupancy, so the figure
supporting the trap's severity is an argument rather than a measurement.

Recorded rather than fixed: adding the measurement is a `ring_bench` change, and
the claim is uncontroversial enough that its absence has cost nothing yet.
