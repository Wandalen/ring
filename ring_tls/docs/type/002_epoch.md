# Type: Epoch

### Scope

- **Purpose**: Define the consolidation-cycle counter, and establish that it is the only mechanism by which per-thread buffers could yield a globally consistent snapshot.
- **Responsibility**: Fix what an epoch identifies, its validation rules, where it must be stored under each sealing mechanism, and the cost that placement carries.
- **In Scope**: The epoch as a value type; its per-buffer and per-record placements; its relationship to snapshot consistency.
- **Out of Scope**: The cycle it counts (→ [Consolidation Cycle](../lifecycle/002_consolidation_cycle.md)); the sealing mechanism whose choice determines whether per-record storage is needed (→ [Buffer Epoch Cycle](../lifecycle/003_buffer_epoch_cycle.md)); the global ordering it does *not* provide.

### Definition

An **epoch** is a monotonically increasing counter identifying one
consolidation cycle. Every record appended between two consolidations belongs
to the same epoch; the counter increments once per cycle, never per record
and never per thread.

Its purpose is narrow and specific: it is the only way to answer *"were these
two records written during the same cycle?"* across two different threads'
buffers. Nothing else in this crate can answer that, because per-thread
buffers carry no cross-thread ordering of any kind
(→ [Buffer Epoch Cycle](../lifecycle/003_buffer_epoch_cycle.md)'s
Behavioral Invariant 3).

**Whether it must be stored per record depends entirely on the sealing
mechanism, and the two answers differ sharply:**

| Sealing mechanism | Epoch storage | Snapshot consistency |
|-------------------|---------------|----------------------|
| Stop-the-world barrier | **One counter, global.** Every thread is stopped at the same instant, so every buffer's contents provably belong to one epoch | Free — the barrier *is* the snapshot |
| Double-buffer | **Per record, or nothing.** Threads flip at different instants, so buffer A's tail and buffer B's head may straddle a cycle boundary | Only recoverable by tagging each record with its epoch |

**This is the hidden cost of the double-buffer option, and it is not
mentioned in the mechanism's own trade table.**
[Buffer Epoch Cycle](../lifecycle/003_buffer_epoch_cycle.md)'s E2 states
the double-buffer's cost as 2× memory and its benefit as never stopping a
producer. That accounting is incomplete: if the consumer needs a consistent
snapshot, the double-buffer *additionally* costs an epoch field on every
record — compounding with the record tag's own overhead
(→ [Record Tag](001_record_tag.md)) to make per-record framing 3–8 bytes
rather than 1–2. On small records that is a substantial fraction of the
payload.

Whether the consumer needs a consistent snapshot is therefore upstream of the
sealing decision, not downstream of it. A consumer that only needs "every
record eventually, in no particular cross-thread order" pays nothing and the
double-buffer is clearly better. A consumer needing "all intents up to time
T" pays per record, and the barrier may win despite stalling producers. **The
question is unanswered because it is the consumer's**, and naming which
consumer property decides it is this instance's contribution.

### Validation

| Rule | Statement | Consequence if violated |
|------|-----------|-------------------------|
| V1 | Strictly increasing, one increment per completed cycle | An epoch reused across cycles makes two different snapshots indistinguishable |
| V2 | Incremented exactly once per cycle, by exactly one party | Two incrementers produce gaps or duplicates; the counter's meaning collapses |
| V3 | A record's epoch never exceeds the current epoch | Would mean a record was tagged for a cycle that has not begun — corruption |
| V4 | Increment happens at cycle *completion*, not at trigger | Incrementing at C1 → C2 means records appended during C2–C4 under the double-buffer carry an ambiguous epoch |
| V5 | Width does not wrap within the process's lifetime | See below |

**V4 is subtle and easy to get backwards.** Under the barrier, the increment
point does not matter — no appends happen during C2–C4. Under the
double-buffer, appends continue throughout, so the increment must land at a
point where "which epoch is current" is unambiguous for an appending thread.
Placing it at completion means a record appended mid-cycle is tagged with the
cycle still in flight, which is the correct reading; placing it at trigger
means records appended during a long drain get the *next* epoch while the
previous one is still being consolidated. The second is not wrong so much as
harder to reason about, and picking the wrong one produces off-by-one-cycle
bugs that only appear when consolidation is slow.

**V5 is genuinely non-binding here, unlike its counterpart in the sibling
crate.** [`ring_mpsc`](../../../ring_mpsc/docs/type/001_sequence_number.md)'s
sequence increments once per *record* — millions per second — so its width is
a real trade. An epoch increments once per *cycle*: at one consolidation per
frame and 60 frames per second, a `u32` lasts about two years of continuous
runtime and a `u64` outlasts the hardware. So `u32` is safe and `u64` is free
insurance, and there is no measurement to run. Stating that explicitly keeps
this from being copied as an open question by analogy with the sequence
number, which is exactly the kind of false symmetry a family of 33 similar
crates invites.

**V2's "exactly one party" is unenforced**, same as
[Consolidation Cycle](../lifecycle/002_consolidation_cycle.md)'s
non-re-entrancy requirement — and it is the same underlying gap seen from the
counter's side rather than the cycle's.

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_consolidator_read_surface.md](../api/002_consolidator_read_surface.md) | Its watermark belongs to an epoch; a stale-watermark check would compare epochs |
| [../api/001_writer_append_surface.md](../api/001_writer_append_surface.md) | Would carry the epoch into each record under the double-buffer plus consistent-snapshot combination |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_thread_local_append_log.md](../data_structure/001_thread_local_append_log.md) | Where a per-buffer epoch would live; a per-record epoch instead becomes part of its undecided framing |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_single_writer_append.md](../invariant/001_single_writer_append.md) | Why cross-thread ordering is absent, which is why an epoch is the only snapshot mechanism available |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_consolidation_cycle.md](../lifecycle/002_consolidation_cycle.md) | The cycle this counter counts; V4's increment point sits at its K4 |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/003_buffer_epoch_cycle.md](../lifecycle/003_buffer_epoch_cycle.md) | Its E2 mechanism choice decides whether per-record storage is needed at all — the cost its own trade table omits |

### Types

| File | Relationship |
|------|--------------|
| [001_record_tag.md](001_record_tag.md) | Sits alongside the epoch in per-record framing; the two overheads compound |

### Sources

| File | Relationship |
|------|--------------|
| `src/lib.rs` | Crate root; skeleton — no epoch counter yet |

### Tests

| File | Relationship |
|------|--------------|
| `tests/epoch_test.rs` (to create) | Across many cycles with continuous appends under the double-buffer, no record is tagged with an epoch later than the cycle in which it was appended — V3 and V4 together |
| `tests/epoch_test.rs` (to create) | Under the barrier mechanism, every record drained in one cycle carries the same epoch with no per-record field present — the free-snapshot claim, asserted structurally |

### TL49 — The Epoch Type Was Never Declared Either

`ring_tls` imports four items — `Ordering`, `SeqCell`, `claim`/`BatchClaim`,
and `RingError`/`Seq`. `Seq` is the family's sequence number and is not an
epoch: it identifies a slot in a ring, not a round of consolidation.

**Disposition:** declined — the Epoch counter, its per-record/per-buffer
storage trade-off, and V1-V5 validation rules this instance specifies govern a
consolidation-cycle concept the crate never built; `ring_tls` imports `Seq` (a
ring slot identifier) but declares no epoch counter anywhere. One of the
nineteen pre-implementation instances
`../decisions/001_the_corpus_specifies_an_api_the_crate_did_not_build.md`
catalogs; rewriting it is a future pass's call, not this one's.
