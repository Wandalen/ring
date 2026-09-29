# Pattern: Staging Then Merge

### Scope

- **Purpose**: State how this crate composes with a ring into a complete write path, and correct the reading that `ring_tls` and `ring_mpsc` are competing candidates rather than two halves of one.
- **Responsibility**: Give the composition its shape, when it applies, when a bare ring is the better answer, and what the two-stage structure costs.
- **In Scope**: The staging-plus-merge composition; the division of labour between this crate and a ring; the alternatives it is chosen against.
- **Out of Scope**: The ring's own mechanism (→ [`ring_mpsc`](../../../ring_mpsc/docs/readme.md)); which pattern wins the benchmark, which is a future benchmark's verdict; the consolidation trigger, which is `ring_flush`'s.

### Problem

A write path taking mutations from many threads to one ordered stream has two
costs that pull in opposite directions:

- **Per-record synchronization.** Every record crossing a thread boundary
  individually pays an atomic at minimum. At millions of records per second
  across sixteen threads, that atomic is contended and its cache line
  bounces between cores.
- **Ordering.** Something must produce a single sequence the consumer reads
  in order, and per-thread buffers deliberately provide no cross-thread
  order at all
  (→ [Buffer Epoch Cycle](../lifecycle/003_buffer_epoch_cycle.md)'s
  Behavioral Invariant 3).

A design that optimizes the first — per-thread buffers, no atomics —
sacrifices the second. A design that optimizes the second — one shared
ordered structure — pays the first on every record.

Reading these as two candidates to choose between is the mistake this pattern
exists to correct, and it is an easy one to make from the crate list alone:
`ring_tls` and `ring_mpsc` sit side by side in a 33-crate family whose stated
purpose includes benchmarking candidate patterns against each other.

### Solution

**Stage per-thread with no synchronization; merge once per cycle into an
ordered structure.** The record count crossing the synchronized boundary
drops from *one atomic per record* to *one handoff per thread per cycle*.

| Stage | Crate | Cost per record | Provides |
|-------|-------|-----------------|----------|
| **Stage** | `ring_tls` | Bounds check + write + bump. **No atomic** | Per-thread order, zero contention |
| **Merge** | A ring, or the consumer's own merge | Amortized over a whole buffer | Cross-thread order |

The arithmetic is the argument. Sixteen threads at one million records per
second each, consolidating at 60 Hz: a bare ring pays 16 million contended
atomics per second; this composition pays 960 handoffs per second — sixteen
threads times sixty cycles. Four orders of magnitude fewer synchronized
operations, for the price of records arriving in cycle-sized batches rather
than individually.

**The cost argument above is the ranking, not an appeal to authority.**
Thread-local buffering with single-threaded merge wins against the
alternatives on exactly the arithmetic already shown: four orders of
magnitude fewer synchronized operations. This crate is the staging half of
that composition, which is what `src/lib.rs` means by "the thread-local-buffer
half of a mechanism."

**So the two crates are not competitors.** `ring_mpsc` is a *merge target*
for this pattern and also a *standalone* write path for consumers that do not
stage. The benchmark compares thread-local-buffering-plus-merge against a
bare ring against the other candidates — the composition against the
alternatives, not this crate against its own merge stage.

### Applicability

| Situation | Use this composition? |
|-----------|-----------------------|
| High record rate, many producer threads, batch-tolerant consumer | **Yes** — the case it is built for, and where the four-orders-of-magnitude reduction is real |
| Low record rate | **No** — a bare ring is simpler and the contention it avoids does not exist at low rates. Two stages of machinery to save atomics nobody was paying is pure complexity |
| Consumer needs each record with minimum latency | **No.** A record waits until the next consolidation, so latency is bounded below by the cycle period, not by the append. A bare ring publishes immediately |
| Consumer needs a consistent global snapshot | Yes, but read [Epoch](../type/002_epoch.md) first — the snapshot is free under one sealing mechanism and per-record-expensive under the other |
| Short-lived threads | **Only with the teardown question settled** (→ [Thread Registration and Teardown](../lifecycle/001_thread_registration_and_teardown.md)'s N3). A bare ring has no equivalent loss window, which is a genuine advantage it holds over this composition |
| `async` tasks without stable thread affinity | **No** (→ [Register Before First Append](001_register_before_first_append.md)'s applicability table) |

**Rows three and five are where the bare ring legitimately wins**, and saying
so plainly matters more than defending the composition. Latency and thread
churn are both real workload properties, and on either of them the simpler
structure is better. A pattern instance that could not name the case against
itself would not be worth reading.

### Consequences

- **Records arrive in batches, not individually.** The consumer's merge step
  processes a cycle's worth at once. That is a structural change to the
  consumer, not a tuning parameter — code written against per-record arrival
  does not become batch code by changing a constant.
- **Two failure surfaces instead of one.** The staging half's silent-loss
  windows (unregistered append, thread death before consolidation) do not
  exist in a bare ring. The composition inherits both, and neither is
  detectable downstream.
- **Memory is per-thread rather than shared.** N threads times region size,
  possibly doubled under the double-buffer sealing mechanism, against a
  ring's single fixed allocation. At sixteen threads with double-buffering
  that is 32 regions where a ring has one — a real footprint difference that
  the atomic-count argument above does not mention.
- **The merge stage is still needed and still costs something.** This
  composition reduces the synchronized operation count; it does not
  eliminate the ordered structure. Whatever merges still has to exist, and
  if that merge target is `ring_mpsc`, every one of its own open questions
  applies unchanged.
- **The cycle period becomes a correctness parameter** rather than a tuning
  knob, because it bounds both region occupancy and the loss window
  (→ [Consolidation Cycle](../lifecycle/002_consolidation_cycle.md)'s three
  constraints). A bare ring has no such parameter.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_writer_append_surface.md](../api/001_writer_append_surface.md) | The staging half's surface; its zero-atomic property is the composition's whole advantage |
| [../api/002_consolidator_read_surface.md](../api/002_consolidator_read_surface.md) | The handoff point into the merge stage |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_thread_local_append_log.md](../data_structure/001_thread_local_append_log.md) | The staged region, N of them; the per-thread footprint consequence is its multiplication |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_prospective_consumer_adoption.md](../integration/002_prospective_consumer_adoption.md) | Where a consumer decides between this composition and a bare ring |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_single_writer_append.md](../invariant/001_single_writer_append.md) | What makes the staging half free, and what the `async` applicability row threatens |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_consolidation_cycle.md](../lifecycle/002_consolidation_cycle.md) | The merge cadence; its three constraints are this composition's correctness parameters |
| [../lifecycle/001_thread_registration_and_teardown.md](../lifecycle/001_thread_registration_and_teardown.md) | The loss window the fifth applicability row turns on |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_measured_before_adopted.md](../non_functional_requirement/001_measured_before_adopted.md) | The gate that decides whether the ranked-first composition actually wins when measured |

### Patterns

| File | Relationship |
|------|--------------|
| [001_register_before_first_append.md](001_register_before_first_append.md) | A precondition of the staging half; an unregistered stage contributes nothing to the merge |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/003_buffer_epoch_cycle.md](../lifecycle/003_buffer_epoch_cycle.md) | Its Behavioral Invariant 3 is the ordering gap the merge stage exists to close |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_epoch.md](../type/002_epoch.md) | What the fourth applicability row turns on |

### Sources

| File | Relationship |
|------|--------------|
| `src/lib.rs` | States this crate as "the thread-local-buffer half of a mechanism" — the composition this pattern names |

### Tests

| File | Relationship |
|------|--------------|
| `tests/staging_merge_test.rs` (to create) | Under 16 threads at high append rate, the total atomic operation count scales with cycles rather than with records — the composition's central claim, measured rather than argued |
| `tests/staging_merge_test.rs` (to create) | Every record appended across all staged buffers appears exactly once in the merged output, with per-thread order preserved within each thread's contribution |

### TL44 — The Two-Stage Composition Is the One Pattern That Transferred Intact

The pattern's content is the cost argument, not the mechanism: N items, one
synchronised operation. That is exactly what `flush_into` delivers, and
`ring_flush` is the crate that owns the "when".

Three of the nineteen survived the rewrite, and all three are the ones stated as
costs or shapes rather than as calls.
