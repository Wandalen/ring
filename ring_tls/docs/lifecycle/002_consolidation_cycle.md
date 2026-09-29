# Lifecycle: Consolidation Cycle

### Scope

- **Purpose**: State the repeating phase nested inside a buffer's live period, and establish that its period is a correctness parameter rather than a tuning knob.
- **Responsibility**: Name the cycle's phases, who triggers each, what each must complete before the next begins, and the three independent constraints that jointly bound the period.
- **In Scope**: One consolidation cycle across all registered buffers; its trigger, its deadline, and what an overrun costs.
- **Out of Scope**: The per-buffer states within a cycle (→ [Buffer Epoch Cycle](../lifecycle/003_buffer_epoch_cycle.md)); the enclosing thread lifetime (→ [Thread Registration and Teardown](001_thread_registration_and_teardown.md)); what the consumer does with consolidated records.

### Lifecycle Phases

| Phase | Entered by | While in it | Left via |
|-------|------------|-------------|----------|
| **C1 Accumulating** | Previous cycle's reset completing | Owners append freely; nothing is read | The consumer triggering a consolidation |
| **C2 Sealing** | Consumer triggers | Each buffer transitions Appending → Sealed by the chosen mechanism | Every registered buffer sealed |
| **C3 Draining** | All buffers sealed | The consolidator walks the registry and reads each buffer base-to-watermark | Last buffer read |
| **C4 Resetting** | Drain complete | Each buffer's bump pointer returns to base | All reset — returns to C1 |

**C1 is where essentially all wall-clock time is spent**, and the cost of the
other three is what the consolidation *period* trades against. The whole
design is: make C1 free (no atomics, no locks, one bump per record) and pay
for it in C2–C4, infrequently.

**C2's cost depends entirely on the undecided E2 mechanism.** Under the
double-buffer it is a per-thread pointer flip, effectively free and requiring
no thread to stop. Under the stop-the-world barrier every appending thread
blocks for the whole of C2–C4
(→ [Buffer Epoch Cycle](../lifecycle/003_buffer_epoch_cycle.md)'s E2).
The two make this phase's cost differ by orders of magnitude, which is why
the period's lower bound below is stated per-mechanism rather than absolutely.

### Phase Transitions

| # | From → To | Trigger | Deadline |
|---|-----------|---------|----------|
| K1 | C1 → C2 | **Consumer's decision** — this crate never self-triggers | None; but see the three constraints below |
| K2 | C2 → C3 | All buffers observed Sealed | Under the barrier mechanism, every producer is blocked until this completes |
| K3 | C3 → C4 | Last buffer drained | **Must complete before any owner flips again** (double-buffer) or before the barrier releases |
| K4 | C4 → C1 | All bump pointers reset | — |

**K1 is not this crate's, and that is the most important structural fact
here.** Nothing in `ring_tls` decides when to consolidate. It exposes the
capability and the consumer picks the period — per frame, per N milliseconds,
on a size watermark, or never. The crate therefore cannot guarantee any
bound on data latency or on the loss window; both are consequences of a
decision made above it.

**Three independent constraints bound the period, and they push in different
directions:**

1. **Upper bound from capacity.** A buffer is a fixed region. Appending past
   its end must either grow it — the one allocation the crate's steady-state
   invariant permits and would rather avoid
   (→ [Zero Allocations in Steady State](../invariant/002_zero_allocations_in_steady_state.md))
   — or fail. So the period must be short enough that no thread fills its
   region between consolidations, which makes the period a function of the
   *fastest* appending thread's rate, not the average.
2. **Upper bound from the loss window.** Every record appended since the last
   consolidation is what a thread exit destroys
   (→ [Thread Registration and Teardown](001_thread_registration_and_teardown.md)'s
   N3). A longer period is a proportionally larger loss on any thread exit.
3. **Lower bound from cost.** C2–C4 have a fixed cost per cycle — registry
   iteration, per-buffer sealing, and under the barrier mechanism, a global
   stall. Consolidating too often pays it repeatedly for little data.

**This is why the period is a correctness parameter, not a tuning knob.**
Constraints 1 and 2 are correctness — exceed them and you allocate
unexpectedly or lose data. Only constraint 3 is performance. A consumer that
treats the period as "how often do I want to pay the consolidation cost"
has read only the third and will discover the first two as bugs.

### Dependencies

| Dependency | Phase | Why |
|------------|-------|-----|
| Consumer's own scheduler | K1 | Owns the trigger entirely; this crate provides no timer, no watermark, and no self-trigger |
| [`ring_registry`](../../../ring_registry/readme.md) | C3 | Supplies the buffer set C3 iterates — **not currently a dependency** (→ [Registration State](../lifecycle/004_registration_state.md)) |
| [`ring_flush`](../../../ring_flush/readme.md) | C2–C4 | The family's crate named for exactly this cycle, and one of the five externally-visible crates alongside this one — a strong signal that K1's trigger and this cycle's orchestration belong there rather than here |
| [`ring_types`](../../../ring_types/readme.md) | C3 | Record tagging the drain decodes |

**`ring_flush` existing is the reason this instance does not specify the
trigger.** A family crate named for flushing, exported alongside this one,
almost certainly owns the cycle's orchestration; documenting a trigger
mechanism here would duplicate it into the wrong crate. The seam is
[Family Dependency Seam](../integration/001_family_dependency_seam.md)'s, and
this crate's half is: provide seal, drain, and reset as operations, and let
`ring_flush` sequence them.

### Cleanup Requirements

1. **Every registered buffer is drained in C3, or the cycle is incomplete.**
   A skipped buffer's records survive to the next cycle under the
   double-buffer — where the owner has moved on and the region is
   untouched — but are *overwritten* if the owner flips again first
   (→ [Buffer Epoch Cycle](../lifecycle/003_buffer_epoch_cycle.md)'s E4
   deadline). So a partial drain is not a delayed drain; it is conditional
   data loss.
2. **C4 resets only buffers C3 actually drained.** Resetting an undrained
   buffer discards its records outright, converting requirement 1's
   conditional loss into certain loss.
3. **A cycle with no registered buffers completes normally.** Zero threads is
   not an error state, and a consolidator that treats an empty registry as a
   failure would break every startup sequence.
4. **The cycle is not re-entrant.** Two concurrent consolidation passes would
   both seal, both drain, and both reset — double-reading records and racing
   on the reset. Nothing in this crate prevents it; whoever owns K1 must.

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_consolidator_read_surface.md](../api/002_consolidator_read_surface.md) | The operations C2–C4 are composed of; its non-re-entrancy is Cleanup Requirement 4 |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_thread_local_append_log.md](../data_structure/001_thread_local_append_log.md) | The fixed region whose capacity supplies constraint 1's upper bound |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_family_dependency_seam.md](../integration/001_family_dependency_seam.md) | Where the `ring_flush` boundary is drawn |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_zero_allocations_in_steady_state.md](../invariant/002_zero_allocations_in_steady_state.md) | Constraint 1 is this invariant expressed as a period bound — overrun the period and the region grows |

### Lifecycles

| File | Relationship |
|------|--------------|
| [001_thread_registration_and_teardown.md](001_thread_registration_and_teardown.md) | The enclosing phase; its N3 loss window is exactly one cycle's accumulation, which is constraint 2 |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/003_buffer_epoch_cycle.md](../lifecycle/003_buffer_epoch_cycle.md) | Each buffer's own states within C2–C4; its E2 mechanism decides C2's cost |
| [../lifecycle/004_registration_state.md](../lifecycle/004_registration_state.md) | Its Behavioral Invariant 3 is what lets C3 iterate while threads churn |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_epoch.md](../type/002_epoch.md) | Increments once per cycle; the identifier a record carries if a consistent global snapshot is wanted |

### Sources

| File | Relationship |
|------|--------------|
| `src/lib.rs` | Crate root; skeleton — no consolidation operations yet |

### Tests

| File | Relationship |
|------|--------------|
| `tests/consolidation_cycle_test.rs` (to create) | Across many cycles with continuously appending threads, every appended record appears in exactly one consolidated output — no loss, no duplication |
| `tests/consolidation_cycle_test.rs` (to create) | A thread appending faster than the period allows grows its region exactly once and reports it, rather than growing silently — constraint 1 made observable |

### TL38 — The Consolidation Cycle Has No Consolidator

What replaced it is a policy in another crate: `ring_flush` decides when to
flush and this crate performs it. The cycle is real; its driver is a consumer,
not a function here (→ [`../pattern/002`](../pattern/002_staging_then_merge.md),
which describes the two-stage composition accurately for both designs).

**Disposition:** declined — the C1-C4 phase table and its `ring_registry`-
iterating consolidator have no built counterpart; the cycle's actual driver is
`ring_flush`'s own policy, calling into this crate's `flush_into`/`drain`, per
`../pattern/002_staging_then_merge.md`'s already-accurate reading. One of the
nineteen instances
`../decisions/001_the_corpus_specifies_an_api_the_crate_did_not_build.md`
catalogs; rewriting it is a future pass's call, not this one's.
