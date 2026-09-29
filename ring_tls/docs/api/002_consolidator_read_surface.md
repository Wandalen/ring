# API: Consolidator Read Surface

### Scope

- **Purpose**: State the operations that read a buffer from a thread other than its owner, and establish why they are exposed as three separate primitives rather than one consolidate call.
- **Responsibility**: Fix the operation set, the safety precondition each carries, the error behaviour, and the boundary against `ring_flush`.
- **In Scope**: Seal, drain, and reset as caller-facing operations; the registry iteration they run over.
- **Out of Scope**: When to call them, which is `ring_flush`'s (→ [Consolidation Cycle](../lifecycle/002_consolidation_cycle.md)); the epoch mechanism that makes sealing possible (→ [Buffer Epoch Cycle](../lifecycle/003_buffer_epoch_cycle.md)); the writer's side (→ [Writer Append Surface](001_writer_append_surface.md)).

### Abstract

This surface is called from a thread that does **not** own the buffer it is
reading, which inverts every property the writer surface has. The writer is
synchronization-free because it is alone; this reader is not alone, and every
operation here carries a precondition about the buffer's state that the
operation itself cannot check.

The surface is deliberately three primitives rather than one
`consolidate_all()`. That is a boundary decision, not an ergonomic one:
`ring_flush` exists in the family to sequence these, and is exported
alongside this crate. A fused call here would duplicate that crate's job
inside this one and remove the seam at which the sealing mechanism — still
undecided — is chosen.

### Operations

| Operation | Signature shape | Precondition | Cost |
|-----------|-----------------|--------------|------|
| `seal` | `&self -> Watermark` | Owner is not mid-append, per the chosen E2 mechanism | Mechanism-dependent: a pointer flip, or a barrier wait |
| `drain` | `&self, watermark -> Records<'_>` | Buffer is Sealed and the watermark is the one `seal` returned | One sequential read of the occupied prefix |
| `reset` | `&self, watermark` | `drain`'s borrow has ended | One store |

**Three calls, and each one's precondition is the previous one's
postcondition.** That chain is what a fused call would hide, and hiding it
would be wrong here specifically because the middle link — `drain`'s borrow —
is what makes the read zero-copy. `Records<'_>` borrows the region; the
records are read in place, not copied out. `reset` cannot run while that
borrow lives, and Rust's own lifetimes enforce it, which is a real guarantee
rather than a documented obligation. A `consolidate_all() -> Vec<Record>`
would copy every record on every cycle, and copying is precisely what a bump
allocator is chosen to avoid.

**`seal` returns a watermark rather than mutating state, and that matters
for re-entrancy.** The watermark is the value `drain` and `reset` both take,
so a caller cannot accidentally drain to a different point than it sealed at.
It also means two concurrent consolidation passes would each get their own
watermark and both proceed — which is exactly
[Consolidation Cycle](../lifecycle/002_consolidation_cycle.md)'s Cleanup
Requirement 4, unenforced. Making the surface non-re-entrant would need
`seal` to take `&mut` on something, and there is no obvious single object to
take it on when the buffers are per-thread. **Undecided**, and it is
`ring_flush`'s question as much as this crate's.

**Registry iteration is conspicuously absent from this table.** These
operations act on *one* buffer. Walking all registered buffers is the
consolidator's loop, and whether this crate provides that loop depends on the
unresolved registry-ownership question
(→ [Registration State](../lifecycle/004_registration_state.md)). If the
registry is `ring_registry`'s, this surface stays per-buffer and `ring_flush`
does the walking. That is the shape this instance assumes, and it is an
assumption rather than a decision.

### Error Handling

| Condition | Behaviour | Is it an error? |
|-----------|-----------|-----------------|
| Buffer is empty | `drain` yields no records | **No.** The common case for an idle thread |
| `drain` called without `seal` | Reads a concurrently-mutating region — torn records, garbage lengths | **Yes, and undetectable.** The precondition has no runtime check |
| `reset` called with a stale watermark | Discards records appended since the seal | Yes, silently |
| Owner thread died mid-cycle | Region may be freed — use-after-free | Yes (→ [Thread Registration and Teardown](../lifecycle/001_thread_registration_and_teardown.md)'s N3) |
| Two concurrent passes | Both drain the same records | Yes, undetected |

**Rows two and five are unenforced preconditions, and that is the honest
characterization of this surface's safety story.** Neither is checkable
cheaply: verifying the owner is not appending requires the synchronization
the design exists to avoid, and detecting a concurrent pass requires shared
state the per-thread structure does not have. So both are contracts on the
caller — which is tolerable precisely because the caller is `ring_flush`, one
crate, rather than arbitrary consumer code. **If this surface were reachable
by ordinary consumers, that would not be tolerable**, and the fact that
`ring_tls` is externally visible means it currently is reachable. Narrowing
the reader half behind `ring_flush` while leaving the writer half public is
an option this instance names and does not choose.

**The fourth row is the one no discipline fixes.** A buffer whose owning
thread has exited may be freed memory. No precondition on the consolidator
prevents dereferencing it; the fix has to be in teardown — the orphan-list
option in
[Thread Registration and Teardown](../lifecycle/001_thread_registration_and_teardown.md)
— rather than here. Listing it here anyway keeps the surface's real failure
set complete rather than the subset it can address.

### Compatibility Guarantees

1. **Zero-copy reads are the guarantee.** `drain` yields a borrow, not owned
   data. A future revision returning owned records would forfeit the design's
   main advantage while preserving its costs, and would be a breaking change
   regardless of how the signature was arranged to look compatible.
2. **The three-primitive shape is stable.** Seal, drain, reset stay
   separate, because the seam between them is where `ring_flush` and the
   undecided E2 mechanism both attach.
3. **Empty is not an error and never will be.** Same reasoning as the ring's
   drain surface — an idle thread is normal.
4. **Preconditions may become type-enforced, which would be breaking.** The
   seal-before-drain chain is currently prose; expressing it as a token type
   (`Sealed` proving the seal happened) would be a strict improvement and a
   signature change. A caller should not assume today's shape is final.
5. **Not stable overall**, for the same reason the writer surface is not
   (→ [Measured Before Adopted](../non_functional_requirement/001_measured_before_adopted.md)).

### APIs

| File | Relationship |
|------|--------------|
| [001_writer_append_surface.md](001_writer_append_surface.md) | The counterpart; every property here inverts one there, and its `remaining` is actionable where this surface's preconditions are not checkable |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_thread_local_append_log.md](../data_structure/001_thread_local_append_log.md) | The region `drain` borrows and `reset` rewinds |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_family_dependency_seam.md](../integration/001_family_dependency_seam.md) | Draws the `ring_flush` boundary this surface's three-primitive shape exists to preserve |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_single_writer_append.md](../invariant/001_single_writer_append.md) | The invariant `seal` exists to make compatible with a cross-thread read |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_consolidation_cycle.md](../lifecycle/002_consolidation_cycle.md) | Composes these three into C2–C4; its Cleanup Requirement 4 is the unenforced re-entrancy precondition |
| [../lifecycle/001_thread_registration_and_teardown.md](../lifecycle/001_thread_registration_and_teardown.md) | Owns the dead-owner failure this surface cannot defend against |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_pod_pointer_free_payloads.md](../non_functional_requirement/002_pod_pointer_free_payloads.md) | What makes reading records in place from another thread sound at all |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/003_buffer_epoch_cycle.md](../lifecycle/003_buffer_epoch_cycle.md) | `seal` effects E2, `drain` runs in Draining, `reset` effects E4 |
| [../lifecycle/004_registration_state.md](../lifecycle/004_registration_state.md) | Its Behavioral Invariant 3 is what the absent registry-iteration operation would have to satisfy |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_record_tag.md](../type/001_record_tag.md) | What `drain` decodes each record by |
| [../type/002_epoch.md](../type/002_epoch.md) | The cycle identifier a watermark belongs to, and what a stale-watermark check would compare |

### Sources

| File | Relationship |
|------|--------------|
| `src/lib.rs` | Crate root; skeleton — no operations declared yet |

### Tests

| File | Relationship |
|------|--------------|
| `tests/consolidator_api_test.rs` (to create) | `drain`'s records point into the buffer's own region — zero-copy asserted by address comparison, not by signature inspection |
| `tests/consolidator_api_test.rs` (to create) | `reset` does not compile while a `drain` borrow is live — Compatibility Guarantee 1's lifetime enforcement, as a `compile_fail` doc test |

### TL9 — Three Primitives Became One Call, and a Consumer Needed the Third Back

```sh
cd "$(git rev-parse --show-toplevel)"
grep -A 5 'pre-implementation surface specified' ring_flush/src/lib.rs
```

Live output:

```
//! `ring_tls`'s pre-implementation surface specified `seal`/`drain`/`reset` as
//! three calls, and what was built is `flush_into` — claim and drain fused,
//! emptying the buffer whether or not the records land. That shape cannot
//! satisfy this crate's O3/O4: a rejected batch would already be gone.
//! `TlsBuffer::drain` was added there so the check can happen before the buffer
//! is touched (→ `docs/algorithm/002_sequencing_seal_drain_reset.md`).
```

A consumer crate's module documentation states the divergence, names both
shapes, and records that `TlsBuffer::drain` was added to this crate so its own
requirement could be met. That paragraph is the only place in the repository
where the two designs are compared — and it is not in this crate.

**Disposition:** declined — the three-primitive `seal`/`drain`/`reset` surface
this instance specifies was never built; the crate exports `flush_into` plus a
later `drain`. One of the nineteen instances
`../decisions/001_the_corpus_specifies_an_api_the_crate_did_not_build.md`
already catalogs, whose rewrite-vs-supersede-vs-relocate resolution is
a future pass's call, not a per-file text edit here.

### TL10 — The Consumer Kept the Unbuilt Vocabulary in Its Own Comments

```sh
cd "$(git rev-parse --show-toplevel)"
grep 'seal, drain' ring_flush/src/lib.rs
ls ring_flush/docs/algorithm/
```

Live output:

```
    // Steps 1, 3 and 4 — seal, drain, reset. `TlsBuffer::drain` empties the
001_evaluating_a_policy_at_an_append.md
002_sequencing_seal_drain_reset.md
readme.md
```

`ring_flush` carries a whole algorithm instance named for the three-call
sequence, and its own source narrates the steps in those terms. The crate that
was supposed to provide them exports one fused call and one late addition.

The vocabulary outlived the design in the consumer, and the producer's corpus
never recorded that it had changed — which is how a term survives with nothing
behind it.
