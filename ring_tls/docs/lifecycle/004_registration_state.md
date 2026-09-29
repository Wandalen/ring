# Lifecycle: Registration State

### Scope

- **Purpose**: State the conditions between a thread having a buffer and the consolidator being able to reach it, since a buffer the consolidator cannot find is functionally a buffer whose contents are lost.
- **Responsibility**: Name the visibility states, the transitions that change them, and the shared structure that makes this machine the crate's one genuinely contended point.
- **In Scope**: A buffer's presence in the global buffer set; the registration and deregistration transitions; the contention this introduces into an otherwise lock-free design.
- **Out of Scope**: The buffer's own epoch states (→ [Buffer Epoch Cycle](003_buffer_epoch_cycle.md)); the thread lifecycle driving these transitions (→ [Thread Registration and Teardown](../lifecycle/001_thread_registration_and_teardown.md)); the registry's implementation, which is not this crate's.

### States

| State | Buffer exists? | Consolidator can reach it? | Data appended here is |
|-------|----------------|----------------------------|-----------------------|
| **Absent** | No | — | — |
| **Unregistered** | Yes | **No** | **Silently lost** at the next consolidation |
| **Registered** | Yes | Yes | Consolidated normally |
| **Deregistering** | Yes | Racing — may or may not be reached | Lost or consolidated, non-deterministically |

**Unregistered is a real state with a real failure, not a transient
formality.** A thread that creates its buffer lazily on first append — the
natural implementation, since a thread-local initializes on first access —
appends into a buffer no consolidator knows about. Every record written
between the first append and the registration becoming visible is invisible
to consolidation, and there is no error: the append succeeds, the data is in
memory, and it is simply never read. **Registration must therefore precede
the first append, not accompany it**, and stating that here is the point of
naming the state at all.

**Deregistering is worse because it is non-deterministic.** A thread
removing itself from the registry while a consolidation pass is walking the
registry produces either a consolidated buffer or a missed one depending on
iteration timing. Whether that matters depends entirely on whether the
buffer's remaining records mattered, which the crate cannot know.

### Transitions

| # | From → To | Effected by | Contention |
|---|-----------|-------------|------------|
| R1 | Absent → Unregistered | Thread-local initialization | None — thread-local |
| R2 | Unregistered → Registered | Insert into the global buffer set | **Contended** — the one shared mutable structure in this design |
| R3 | Registered → Deregistering | Thread exit begins | Contended |
| R4 | Deregistering → Absent | Removal completes; buffer freed | Contended |
| R5 | Registered → Registered | Steady state | None |

**R2 through R4 are the crate's only contended operations, and that is worth
stating loudly in a crate whose entire pitch is "no atomics on the append
path."** The pitch is true and remains true — R5 is where a thread spends
essentially all of its life, and R5 is free. But the design is not
lock-free end to end, and a reader who takes "zero-lock append log" as
covering the whole crate has been misled by the name. The contention is
real, and it is *bounded*: R2 happens once per thread, R3/R4 once per thread,
and both are off the hot path entirely.

**This is the same shape as the pitfall this crate already documents.**
[Implicit Thread-Locals Are Hidden State](../pitfall/001_implicit_thread_locals_are_hidden_state.md)
names the readings this crate's vocabulary invites and misleads with;
"zero-lock" covering registration is one more instance of exactly that
pattern, found on a different axis. Recording it here rather than as a new
pitfall keeps one trap in one place.

**Whether the registry belongs to this crate at all is undecided.** The
family has [`ring_registry`](../../../ring_registry/readme.md) for precisely
this job, and this crate does not currently depend on it — `Cargo.toml`
declares `ring_types`, `ring_atomic` and `ring_batch`. Two readings are open: the
registry is `ring_registry`'s and this crate takes the dependency, or
registration is the *consumer's* problem and this crate provides only the
per-thread buffer with the consumer wiring discovery itself. The second is
consistent with the crate's stated payload-agnosticism and with its very thin
dependency list; the first is consistent with the family having a crate named
for the job. **Nothing read here settles it**, and the choice determines
whether R2–R4 are this crate's states at all or merely states it must
document because its users will hit them.

### Behavioral Invariants

1. **Registration strictly precedes the first append.** Otherwise
   Unregistered's silent-loss window is open. This is an ordering obligation
   on whoever owns registration, and it cannot be enforced from inside the
   append path without a check on every append — which would put a branch on
   the hot path that exists solely to catch a setup error.
2. **Deregistration strictly follows the last consolidation of that
   buffer.** Otherwise the buffer's final records are lost. This is the same
   obligation as
   [Thread Registration and Teardown](../lifecycle/001_thread_registration_and_teardown.md)'s
   central hazard, seen from the visibility side rather than the memory side.
3. **The registry's iteration must tolerate concurrent R2 and R4.** A
   consolidation pass runs while threads are starting and stopping. An
   iteration that invalidates on mutation would make consolidation fail
   whenever a thread happened to start during it — a failure whose frequency
   scales with thread churn and which would look like a flaky test rather
   than a design gap.
4. **A buffer's absence from the registry is indistinguishable from a buffer
   with no records.** Both consolidate to nothing. This is why Unregistered's
   loss is silent, and it cannot be fixed inside the consolidator — it has
   nothing to compare against.

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_consolidator_read_surface.md](../api/002_consolidator_read_surface.md) | Iterates the registry; Behavioral Invariant 3 is a requirement on how |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_thread_local_append_log.md](../data_structure/001_thread_local_append_log.md) | The buffer whose visibility these states track |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_family_dependency_seam.md](../integration/001_family_dependency_seam.md) | Where the `ring_registry`-or-consumer question lands as a dependency decision |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_zero_allocations_in_steady_state.md](../invariant/002_zero_allocations_in_steady_state.md) | R5 is the steady state it describes; R1–R4 are outside it, which is why registration's allocation does not violate it |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_thread_registration_and_teardown.md](../lifecycle/001_thread_registration_and_teardown.md) | The thread-side driver of every transition here |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_register_before_first_append.md](../pattern/001_register_before_first_append.md) | The usage rule Behavioral Invariant 1 requires, stated as a pattern a consumer can follow |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_implicit_thread_locals_are_hidden_state.md](../pitfall/001_implicit_thread_locals_are_hidden_state.md) | The same misleading-vocabulary trap on a different axis — "zero-lock" reading as covering registration |

### State Machines

| File | Relationship |
|------|--------------|
| [003_buffer_epoch_cycle.md](003_buffer_epoch_cycle.md) | The orthogonal axis; it assumes Registered throughout |

### Sources

| File | Relationship |
|------|--------------|
| `Cargo.toml` | Declares `ring_types`, `ring_atomic` and `ring_batch` — the absence of `ring_registry` is what leaves the ownership question open |
| `src/lib.rs` | Crate root; skeleton — no registration mechanism yet |

### Tests

| File | Relationship |
|------|--------------|
| `tests/registration_test.rs` (to create) | Records appended before registration completes are absent from the consolidated output — Unregistered's silent loss, made visible as a test rather than discovered in production |
| `tests/registration_test.rs` (to create) | A consolidation pass completes correctly while threads concurrently register and deregister — Behavioral Invariant 3, under churn |

### TL40 — The Registration State Machine Has One State in the Built Crate

A `TlsBuffer` is usable the instant `with_capacity` returns and until it is
dropped. There is no intermediate state a caller can observe, and therefore no
window in which the ordering rule of
[`../pattern/001`](../pattern/001_register_before_first_append.md) could be
violated.

**Disposition:** declined — the Absent/Unregistered/Registered/Deregistering
visibility states and R1-R5 transitions this instance specifies have no built
counterpart; a `TlsBuffer` is usable from `with_capacity` to drop with no
intermediate state at all. One of the nineteen pre-implementation instances
`../decisions/001_the_corpus_specifies_an_api_the_crate_did_not_build.md`
catalogs; rewriting or superseding it is a future pass's decision.
