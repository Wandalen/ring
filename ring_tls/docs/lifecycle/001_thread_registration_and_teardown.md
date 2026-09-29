# Lifecycle: Thread Registration and Teardown

### Scope

- **Purpose**: State the phases binding a buffer's life to its owning thread's, and characterize the data-loss window that a thread exit opens — the pattern's defining hazard.
- **Responsibility**: Name each phase, its entry condition, the obligations teardown carries, and the three mitigations available with their costs.
- **In Scope**: One thread's buffer from creation through free; the exit path; what happens to records appended after the final consolidation.
- **Out of Scope**: The repeating consolidation phase's own mechanics (→ [Consolidation Cycle](002_consolidation_cycle.md)); the visibility states registration moves through (→ [Registration State](../lifecycle/004_registration_state.md)); the consumer's shutdown sequencing, which is above this crate.

### Lifecycle Phases

| Phase | Entered by | Guarantees | Left via |
|-------|------------|------------|----------|
| **T1 Created** | Thread-local initialization allocates the region | Region exists; bump pointer at base; **not yet reachable by the consolidator** | Registration |
| **T2 Live** | Registration completes | Appends land and are consolidated; the repeating cycle runs (→ [Consolidation Cycle](002_consolidation_cycle.md)) | Thread exit begins |
| **T3 Exiting** | Thread exit begins; TLS destructors scheduled | **Nothing.** The central hazard — see below | Deregistration and free |
| **T4 Freed** | Region deallocated, registry entry removed | Nothing; the buffer no longer exists | — |

**T1 is not a formality: appends are possible in it and are lost.** A
thread-local initializes on first access, so the natural implementation
allocates the region *during* the first append — which means that append
lands in an unregistered buffer. Everything written before registration
becomes visible is invisible to consolidation, with no error
(→ [Registration State](../lifecycle/004_registration_state.md)'s
Unregistered state). The obligation this creates —
[Register Before First Append](../pattern/001_register_before_first_append.md)
— is a usage rule, not something the append path can check without putting a
branch on the hot path to catch a setup error.

### Phase Transitions

| # | From → To | Trigger | Obligation |
|---|-----------|---------|------------|
| N1 | T1 → T2 | Registration insert completes | Must precede the first append |
| N2 | T2 → T3 | Thread exit begins | **External** — the buffer cannot detect it, and neither can the consolidator |
| N3 | T3 → T4 | TLS destructor runs; region freed | **The loss window — see below** |
| N4 | T1 → T4 | Thread exits without ever appending | None; nothing was written |

**N3 is where the pattern's defining hazard lives, and it is not a corner
case.** A thread appends records, and then exits. Between its last append and
its TLS destructor running, no consolidation is guaranteed to occur. The
destructor frees the region. Every record appended since the last
consolidation is gone — no error, no panic, no diagnostic, and no way for the
consumer to learn that anything was lost, because
[Registration State](../lifecycle/004_registration_state.md)'s Behavioral
Invariant 4 says a missing buffer is indistinguishable from an empty one.

The window's size is not bounded by anything this crate controls. It is the
interval between the last consolidation and the thread's exit, which is a
function of the consumer's consolidation period and the thread's own
lifetime — so a short-lived worker thread that appends and exits between two
consolidations loses **everything it ever wrote**. That is the case worth
stating plainly: the pattern's failure is not degraded throughput under
stress, it is total silent loss for a specific, entirely ordinary thread
shape.

Three mitigations, none free and none chosen:

| Mitigation | Mechanism | Cost |
|------------|-----------|------|
| **Drain on TLS drop** | The destructor consolidates its own buffer before freeing | The exiting thread must reach the consumer's consolidation structure from inside a TLS destructor — a context with well-known ordering hazards, where other thread-locals may already be destroyed. Also makes exit latency a function of buffer occupancy |
| **Orphan the region** | The destructor deregisters but *leaks* the region into a global orphan list for the next consolidation to drain and free | Correct and simple. Costs an unbounded-in-principle orphan list and moves the free off the exiting thread — and requires the consolidator to distinguish orphans from live buffers |
| **Join before shutdown** | The consumer guarantees all appending threads are joined before the final consolidation | Free in code, and pushes the entire obligation onto the consumer — who must know it exists. Works only for threads the consumer owns; a thread from a third-party pool is outside its reach |

**Undecided.** The orphan list is the one that handles the short-lived-worker
case without constraining the consumer's threading model, and it is the
direction this instance would lean if leaning were deciding. It is not: the
choice interacts with whether the registry is even this crate's
(→ [Family Dependency Seam](../integration/001_family_dependency_seam.md)),
since an orphan list is a second shared structure alongside the registry.

**N2 is undetectable from inside this crate, same as its sibling's.** Nothing
here observes thread exit; the TLS destructor is the only hook, and it fires
at N3, after the decision about N3 has already had to be made.
[`ring_mpsc`](../../../ring_mpsc/docs/lifecycle/001_ring_construction_and_teardown.md)'s
L2 is the identical shape on the other write path — neither crate can detect
that its producers are done — which suggests the answer is a family-level
one rather than two independent solutions.

### Dependencies

| Dependency | Phase | Why |
|------------|-------|-----|
| [`ring_types`](../../../ring_types/readme.md) | T1–T4 | The shared vocabulary the region's records are tagged in; one of the family's five externally-visible crates, as this crate is |
| [`ring_atomic`](../../../ring_atomic/readme.md) | T2–T4 | The ordering primitives the epoch swap at seal is built from |
| [`ring_batch`](../../../ring_batch/readme.md) | T3 | The contiguous claim a drained region lands as |
| [`ring_registry`](../../../ring_registry/readme.md) | N1, N3 | **Not currently a dependency.** Whether registration and the orphan list belong to it or to the consumer is open (→ [Registration State](../lifecycle/004_registration_state.md)) |

### Cleanup Requirements

1. **The buffer is consolidated after its last append and before its region
   is freed.** The single requirement N3 exists to highlight, and the one
   this crate currently has no mechanism to meet.
2. **Deregistration precedes the free, and follows the final
   consolidation.** Freeing while registered leaves the consolidator a
   dangling pointer — a use-after-free rather than the merely-silent loss of
   requirement 1. Ordering these two correctly is what makes T3 a phase
   rather than an instant.
3. **The region is not zeroed on free.** Same reasoning as Reset
   (→ [Buffer Epoch Cycle](../lifecycle/003_buffer_epoch_cycle.md)'s
   Behavioral Invariant 4): the bytes are unreachable by construction, and
   zeroing is per-teardown traffic for no gain. If the payload is ever
   sensitive this changes, which is a payload concern this crate is
   deliberately agnostic about.
4. **A thread that never appended still deregisters.** N4 skips the loss
   question but not the registry hygiene; a registry accumulating entries for
   dead threads grows without bound and slows every consolidation pass.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_writer_append_surface.md](../api/001_writer_append_surface.md) | Callable only in T2; its unbounded-`T` question interacts with whether a TLS destructor can drop payloads correctly |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_thread_local_append_log.md](../data_structure/001_thread_local_append_log.md) | The region allocated at T1 and freed at T4 |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_family_dependency_seam.md](../integration/001_family_dependency_seam.md) | Where the `ring_registry` question and the family-level thread-exit-detection question both land |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_zero_allocations_in_steady_state.md](../invariant/002_zero_allocations_in_steady_state.md) | T2 is the steady state; T1's allocation and T4's free are outside it by construction |

### Lifecycles

| File | Relationship |
|------|--------------|
| [002_consolidation_cycle.md](002_consolidation_cycle.md) | The repeating cycle nested inside T2; its period is what sizes N3's loss window |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_register_before_first_append.md](../pattern/001_register_before_first_append.md) | The usage rule N1 requires |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/004_registration_state.md](../lifecycle/004_registration_state.md) | The visibility axis of the same transitions; its Behavioral Invariant 4 is why N3's loss is silent |

### Sources

| File | Relationship |
|------|--------------|
| `Cargo.toml` | Declares two dependencies; `ring_registry`'s absence is what leaves the registration question open |
| `src/lib.rs` | Crate root; skeleton — no thread-local declared yet |

### Tests

| File | Relationship |
|------|--------------|
| `tests/thread_teardown_test.rs` (to create) | A thread that appends N records and exits before any consolidation contributes N records to the next consolidated output — the N3 loss window, asserted as the requirement rather than reproduced as the defect |
| `tests/thread_teardown_test.rs` (to create) | Spawning and exiting 10 000 short-lived appending threads leaves the registry size bounded — Cleanup Requirement 4 |

### TL37 — There Is No Registration and No Teardown

This instance specifies a thread registering before its first append and
deregistering at teardown so a consolidator can find it. The crate has no
registry, and the built lifecycle is a plain owned value with a derived `Drop`.

`ring_bench` and `ring_testkit` each construct one per run and let it fall out
of scope; `ring_flush` holds one as a struct field for the flusher's own
lifetime.

**Disposition:** declined — the T1-T4 phase table, registry-based
registration, and the orphan-list mitigation this instance specifies have no
built counterpart; the crate's actual lifecycle is a plain owned value with a
derived `Drop`. One of the nineteen pre-implementation instances
`../decisions/001_the_corpus_specifies_an_api_the_crate_did_not_build.md`
catalogs, whose rewrite-vs-supersede-vs-relocate resolution remains an open
follow-up.
