# Non-Functional Requirement: Measured Before Adopted

### Scope

- **Purpose**: Make this crate's own adoption gate a checkable requirement rather than an informal expectation.
- **Responsibility**: State the quality attribute, the measurement procedure, and the threshold adoption requires.
- **In Scope**: The evidence required before a prospective consumer adopts this crate's append mechanism in place of its own.
- **Out of Scope**: The harness's candidate set and workload design, which belong to whichever benchmark ultimately runs it; migration mechanics after a winning verdict (future work).

### Quality Attribute

Append latency and allocation cost on the hot write path — the two costs the
bump discipline exists to eliminate, and therefore the two a generalized
implementation must not silently reintroduce.

### Statement

The buffer this crate ships must hold the discipline's cost profile — no
atomics, no locks, no per-append allocation — **and measurably not regress**
the mechanism a prospective consumer already has: a `Vec<u8>`-based append
path, benched against this crate's implementation alongside other
write-path candidates. Adoption, if it happens, factors out the append
mechanism only — a consumer's own vocabulary and public API stay exactly as
it already documents them, so the comparison is mechanism-to-mechanism, not
API-to-API.

**The write-path candidate set.** This crate's benchmark measures its own
thread-local-buffer-plus-merge shape against several alternative write-path
designs: MVCC double-buffering, the Disruptor ring, a shared exclusive-access
buffer, declared read/write DAG scheduling, and atomics. Three of these are
alternatives this NFR's "measurably not regress" clause is measured against
directly: MVCC's whole-buffer-copy-on-merge shape is exactly the per-append
allocation and copy cost this crate's bump-then-consolidate discipline is
built to avoid — and it sidesteps a second, independent cost by
construction: MVCC's order-blind parallel writers can each finish with a
different idea of what one shared value should hold, and the mechanism
itself has no built-in way to say which result is right — settling it needs
bespoke, pair-by-pair arbitration code, which is exactly the hand-written
special-casing a shared crate is meant to replace; this crate's
single-writer-per-region discipline
(→ [Single-Writer Append](../invariant/001_single_writer_append.md))
forecloses the hazard outright — there is never a second writer in the same
region to disagree with in the first place, so no arbitration call is ever
needed. A shared exclusive-access buffer removes the append phase's
per-thread isolation entirely, trading it for instant cross-thread
visibility this crate's own contract does not require. Atomics is a
direct-write alternative to the buffer-and-merge shape this crate
implements, with contention cost under sustained multi-producer load still
to be measured rather than assumed.

A losing or inconclusive verdict leaves the consumer exactly as it is
today — this crate stays a skeleton, and nothing regresses.

### Measurement Method

This requirement's contract: a criterion harness over synthetic
archetype-table workloads, producing per-pattern reports plus a written
verdict. The harness does not exist yet; the concrete bench command is
recorded here when it lands (TBD). Until then the checkable artifact is the
absence of a `ring_tls` edge in any consumer's manifest — the same scan
[`integration/002`'s TL26](../integration/002_prospective_consumer_adoption.md)
runs across every `ring_*` crate and keeps current.

### Acceptance Threshold

No regression against the replaced append path on the benchmark workload,
with the discipline's structural costs verifiably absent (no atomics or
locks on the append path; one allocation per buffer lifetime). Comparative
by this requirement's own construction — the bar is the working mechanism
being replaced, never a pre-committed number. Speed never trades against
[Single-Writer Append](../invariant/001_single_writer_append.md): a faster
buffer that weakens the epoch discipline fails regardless of its numbers.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_tagged_record_bump_append.md](../algorithm/001_tagged_record_bump_append.md) | The procedure whose measured cost this requirement's benchmark verdict gates adoption of |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_thread_local_append_log.md](../data_structure/001_thread_local_append_log.md) | The structure under measurement |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_single_writer_append.md](../invariant/001_single_writer_append.md) | The contract that holds regardless of measured speed |
| [../invariant/002_zero_allocations_in_steady_state.md](../invariant/002_zero_allocations_in_steady_state.md) | The structural clause this benchmark cannot trade away for speed, regardless of measured numbers |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [002_pod_pointer_free_payloads.md](002_pod_pointer_free_payloads.md) | The sibling gate — this one is comparative against a baseline, that one is structural and has no baseline to beat |

### Sources

| File | Relationship |
|------|--------------|
| `src/lib.rs` | Crate root; skeleton — no implementation to measure yet |

### Tests

| File | Relationship |
|------|--------------|
| `/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/` benchmark harness (to create) | The criterion harness whose reports are this requirement's evidence |

### TL41 — The Adoption Gate Was Met by a Crate the Requirement Was Not Written For

The requirement is stated as a measured cost — amortised operations per item
— rather than as a property of a region. That is why it survived a total change
of structure, and it is the second of the nineteen instances to do so (with
[`../invariant/002`](../invariant/002_zero_allocations_in_steady_state.md)).

Both survivors are the ones written as numbers.
