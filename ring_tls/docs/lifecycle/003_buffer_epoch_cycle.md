# Lifecycle: Buffer Epoch Cycle

### Scope

- **Purpose**: State the conditions a per-thread buffer holds across one consolidation cycle, and identify the single transition that cannot be effected by the owning thread alone.
- **Responsibility**: Name each state, what it permits the owner and the consolidator to do, the coordination each transition requires, and which coordination mechanism is undecided.
- **In Scope**: One buffer, one owning thread, one consolidation cycle; the double-buffer and barrier alternatives for the contested transition.
- **Out of Scope**: How the consolidator finds the buffer at all (→ [Registration State](004_registration_state.md)); the append steps within Appending (→ [Tagged Record Bump Append](../algorithm/001_tagged_record_bump_append.md)); what happens when the owning thread dies mid-cycle (→ [Thread Registration and Teardown](../lifecycle/001_thread_registration_and_teardown.md)).

### States

| State | Owner thread may | Consolidator may | Held while |
|-------|------------------|------------------|------------|
| **Appending** | Append freely — no atomics, no lock, no coordination | **Nothing.** Reading here races the bump pointer | Normal operation, the overwhelming majority of the time |
| **Sealed** | Nothing to this buffer | Not yet — must observe the seal first | The window between the owner stopping and the consolidator starting |
| **Draining** | Nothing to this buffer | Read every record from base to the sealed watermark | The consolidation pass |
| **Reset** | Resume appending from the base | Nothing | Instantaneously, before returning to Appending |

**Appending is the state the whole crate exists to make cheap, and it is
exactly the state in which the buffer is unreadable.** The append path has no
atomics because the owner is the sole accessor — that is the entire
performance argument (→ [Single-Writer Append](../invariant/001_single_writer_append.md)).
The consolidator reading a buffer in Appending would observe a bump pointer
mid-advance and a record mid-write, with no ordering guarantee of any kind,
because there is no synchronization to provide one. So the states are not a
formality over a "mostly safe" read: Appending and Draining are mutually
exclusive by construction, and everything below is about how that exclusion
is achieved without paying for it on the append path.

**Sealed exists as a distinct state because the exclusion is not
instantaneous.** The owner stopping and the consolidator starting are two
events on two threads; the interval between them is real, and naming it keeps
the transition from being read as atomic when it is not.

### Transitions

| # | From → To | Effected by | Coordination required |
|---|-----------|-------------|------------------------|
| E1 | Reset → Appending | Owner, first append | None |
| E2 | Appending → Sealed | **Contested — see below** | The transition this instance is about |
| E3 | Sealed → Draining | Consolidator, on observing the seal | A `Release`/`Acquire` pair on the seal itself |
| E4 | Draining → Reset | Consolidator, on finishing the read | The owner must not resume before this completes |
| E5 | Draining → Reset (empty) | Consolidator, buffer had no records | Same; a no-op read is still a cycle |

**E2 is the crate's central unsolved coordination problem, and it has two
shapes with materially different costs.**

- **Double-buffer.** The owner holds two regions and an index. E2 is the
  owner flipping the index — a single store on its own thread, no waiting,
  no cross-thread agreement at the moment of the flip. The consolidator then
  drains the region the owner just left. Cost: **2× the memory**, permanently,
  for every thread, whether or not consolidation is frequent. Benefit: the
  owner never stops. E4 needs no coordination either, because the owner is
  already writing elsewhere.
- **Stop-the-world barrier.** All owners reach a barrier, every buffer
  becomes Sealed together, the consolidator drains them all, and the barrier
  releases. Cost: **every producer thread stalls** for the whole
  consolidation — a known multithreading-barrier failure mode, worth
  measuring rather than arguing about. Benefit: one region per thread, and a
  globally consistent snapshot for free.

**Undecided, and it is not a small decision.** It changes the memory
footprint by a factor of two, changes whether producers ever block, and
changes whether the consolidated view is a consistent global snapshot — the
barrier gives one, the double-buffer does not, since different threads flip
at different instants. A consumer needing "all intents up to time T" gets it
free from the barrier and cannot get it from the double-buffer without an
additional epoch number carried per record. That last point is why
[Epoch](../type/002_epoch.md) exists as a type at all.

**E4's obligation is easy to miss in the double-buffer shape.** The owner
resuming into a region the consolidator has not finished draining is a
use-after-read — the owner overwrites records the consolidator is still
reading. With two regions the owner will not naturally return to the drained
one until the *next* flip, so the constraint is "consolidation must finish
within one epoch," which is a real deadline rather than a free property. If
the owner flips twice before the consolidator drains once, it overwrites
undrained data silently. Nothing in this crate currently detects that.

### Behavioral Invariants

1. **A buffer is never simultaneously Appending and Draining.** The whole
   soundness argument. Enforced by E2's mechanism, whichever is chosen —
   which means the invariant is currently stated without an enforcement
   mechanism, and that is the honest position rather than a gap being
   papered over.
2. **The owner is the only thread that ever appends.** Not merely
   *currently* — ever, for the buffer's whole life
   (→ [Single-Writer Append](../invariant/001_single_writer_append.md)). No
   state above relaxes this; the consolidator reads and resets, never
   appends.
3. **Records are consumed in append order within a buffer, and in no defined
   order across buffers.** The bump pointer gives per-thread order for free.
   Cross-thread order is whatever the consolidator's iteration produces,
   which is the buffer-set iteration order, which is not meaningful. **A
   consumer needing a global order must impose one** — this crate does not
   provide it, and that is the structural difference from
   [`ring_mpsc`](../../../ring_mpsc/docs/invariant/001_single_consumer_total_order.md),
   whose whole contract is a total order. Two crates, two orderings, and a
   consumer choosing between them is choosing exactly this.
4. **Reset does not zero the region.** It sets the bump pointer back to the
   base. Stale bytes from the previous epoch remain and are unreachable by
   construction, because reads are bounded by the current bump pointer. A
   zeroing reset would be per-epoch memory traffic proportional to the
   region size for no correctness gain.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_tagged_record_bump_append.md](../algorithm/001_tagged_record_bump_append.md) | Runs entirely within the Appending state; its lack of atomics is what makes the states mutually exclusive rather than merely ordered |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_writer_append_surface.md](../api/001_writer_append_surface.md) | The owner's surface, callable only in Appending |
| [../api/002_consolidator_read_surface.md](../api/002_consolidator_read_surface.md) | The consolidator's surface, callable only in Draining; owns E3 and E4 |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_thread_local_append_log.md](../data_structure/001_thread_local_append_log.md) | The region and bump pointer these states describe; the double-buffer option doubles its footprint |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_single_writer_append.md](../invariant/001_single_writer_append.md) | Behavioral Invariant 2 restated as a contract; this machine's states are how it survives a concurrent reader |
| [../invariant/002_zero_allocations_in_steady_state.md](../invariant/002_zero_allocations_in_steady_state.md) | Reset's no-zeroing property and the double-buffer's fixed 2× are both what keep steady state allocation-free |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_thread_registration_and_teardown.md](../lifecycle/001_thread_registration_and_teardown.md) | What happens when the owning thread dies in any of these states — the loss window this machine assumes away |

### State Machines

| File | Relationship |
|------|--------------|
| [004_registration_state.md](004_registration_state.md) | Orthogonal axis: this machine assumes the consolidator can find the buffer, which is that machine's subject |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_epoch.md](../type/002_epoch.md) | The counter that would recover a consistent global snapshot under the double-buffer option |

### Sources

| File | Relationship |
|------|--------------|
| `src/lib.rs` | Crate root; skeleton — no state encoding yet |

### Tests

| File | Relationship |
|------|--------------|
| `tests/epoch_cycle_test.rs` (to create) | Under the chosen E2 mechanism, a consolidator read concurrent with continuous appends observes a self-consistent record set with no torn record — the mutual exclusion, asserted under contention rather than reasoned about |
| `tests/epoch_cycle_test.rs` (to create) | An owner flipping twice before one drain completes is detected rather than silently overwriting — E4's deadline, which currently has no mechanism |

### TL39 — The Epoch Cycle Describes a Field That Does Not Exist

The state machine is coherent and has no referent. Its closest built analogue
is the empty/partial/full progression of `items` against `limit`, which is
three states rather than an unbounded counter and needs no dating.

**Disposition:** declined — the Appending/Sealed/Draining/Reset state machine
and its E1-E5 transitions describe a coordination protocol the crate never
built; the closest built analogue is `items` against `limit`, a three-state
progression needing no epoch at all. One of the nineteen pre-implementation
instances
`../decisions/001_the_corpus_specifies_an_api_the_crate_did_not_build.md`
catalogs, whose resolution is still open.
