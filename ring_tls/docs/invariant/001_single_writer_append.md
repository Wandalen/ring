# Invariant: Single-Writer Append

### Scope

- **Purpose**: Guarantee that a buffer's contents are always one thread's sequential writes, never an interleaving — the property that lets the append path carry zero synchronization.
- **Responsibility**: State the epoch discipline, the ownership mechanism enforcing it, and what an interleaved buffer costs.
- **In Scope**: Which thread may append to a buffer during a write epoch, and when the consolidating reader may begin.
- **Out of Scope**: The buffer's internal layout (→ [Thread-Local Append Log](../data_structure/001_thread_local_append_log.md)); the order in which consolidated buffers are merged (→ [`ring_mpsc`](../../../ring_mpsc/docs/invariant/001_single_consumer_total_order.md), a separate crate's contract).

### Invariant Statement

During a write epoch, **exactly one thread appends to a given buffer**, and
**no reader observes any buffer until every writer's epoch has ended**.

Equivalently: at no instant does any buffer have two live `&mut`-grade
accessors, and the write→read transition is a strict barrier, never a
concurrent overlap. This is a single-writer, epoch-bounded discipline stated
in its own right; the soundness argument rests on the discipline itself, not
on any particular encoding built on top of it.

### Enforcement Mechanism

**Ownership, not synchronization.** Append operations take the buffer by
`&mut self`, so the borrow checker rejects two concurrent appenders at
compile time; consolidation takes each buffer by exclusive borrow or by move
only after the epoch barrier. The zero-atomics property of the append path is
*derived* from this invariant — the discipline is what makes lock-free
appends sound, not any cleverness in the write itself.

The epoch barrier itself is the consumer's to place — a tick pipeline's
phase boundary, in a typical consumer; this crate's contract is that its API
makes placing it natural and skipping it a compile error, not a runtime
race.

### Violation Consequences

Two threads appending to one buffer is a data race — undefined behavior in
Rust terms, before any observable symptom. Where symptoms do surface, they
surface late and far from the racing writer: for bytecode-shaped payloads, a
tag byte from one record followed by another record's payload bytes, which
the consolidating decoder either misparses silently — applying a wrong
operation to a wrong target — or panics on, one full epoch after the race
that caused it. A reader overlapping a writer sees a torn record the same
way. Neither failure names its cause; the invariant exists so the cause is
unrepresentable rather than diagnosable.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_tagged_record_bump_append.md](../algorithm/001_tagged_record_bump_append.md) | Step 2's `&mut self` receiver is the compile-time mechanism this invariant is enforced by |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_thread_local_append_log.md](../data_structure/001_thread_local_append_log.md) | The structure this discipline governs |

### Invariants

| File | Relationship |
|------|--------------|
| [002_zero_allocations_in_steady_state.md](002_zero_allocations_in_steady_state.md) | The sibling contract; this one the borrow checker enforces, that one it does not |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_measured_before_adopted.md](../non_functional_requirement/001_measured_before_adopted.md) | The speed comparison this invariant is never traded away for, regardless of the numbers |
| [../non_functional_requirement/002_pod_pointer_free_payloads.md](../non_functional_requirement/002_pod_pointer_free_payloads.md) | What an appended payload may contain — the constraint this invariant's zero-atomics appends write into |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_implicit_thread_locals_are_hidden_state.md](../pitfall/001_implicit_thread_locals_are_hidden_state.md) | This pitfall's whole point — the discipline stated here is satisfied by ownership, not by storage location, and TLS silently swaps one for the other |

### Sources

| File | Relationship |
|------|--------------|
| `src/lib.rs` | Crate root; enforces this invariant structurally — `push` takes `&mut self`, so a second appender is a borrow error rather than a race |

### Tests

| File | Relationship |
|------|--------------|
| `tests/single_writer_test.rs` (to create) | Compile-fail case: two live mutable accessors rejected; runtime case: consolidate-read after quiesce returns each thread's bytes verbatim and unmixed |

### TL28 — The Single-Writer Discipline Is Enforced by `&mut self` and Nothing Else

```sh
cd "$(git rev-parse --show-toplevel)"/ring_tls
printf 'mutating methods, all &mut self:\n'
grep -oE 'pub fn [a-z_]+\( &mut self' src/lib.rs
printf 'Sync or Send impls:  %s\n' \
  "$( grep -vE '^\\s*(//|///|//!)' src/lib.rs | grep -cE 'unsafe impl (Send|Sync)' )"
```

Live output:

```
mutating methods, all &mut self:
pub fn push( &mut self
pub fn discard( &mut self
pub fn drain( &mut self
Sync or Send impls:  0
```

No hand-written `Send`/`Sync`, so `TlsBuffer< T >` is auto-`Send` when `T` is
and never `Sync` in a way that permits two concurrent writers. The invariant is
a type-system consequence, not a rule the caller must remember.

### TL29 — The Epoch Discipline This Instance Names Has No Referent

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'Epoch in non-comment code, whole family: %s\n' \
  "$( find ring_*/src ring_*/tests -name '*.rs' -exec cat {} + 2>/dev/null \
      | grep -vE '^\\s*(//|///|//!)' | grep -c 'Epoch' )"
printf 'consolidate:                             %s\n' \
  "$( find ring_*/src ring_*/tests -name '*.rs' -exec cat {} + 2>/dev/null \
      | grep -vE '^\\s*(//|///|//!)' | grep -c 'consolidate' )"
```

Live output:

```
Epoch in non-comment code, whole family: 0
consolidate:                             0
```

Zero and zero. The argument is valid about the design it was written for and has
nothing to attach to here (→ [`../type/002`](../type/002_epoch.md), which
defines the type that does not exist).

**Disposition:** declined — the epoch discipline this instance's Invariant
Statement opens with ("During a write epoch...") has no referent (confirmed
above: zero `Epoch`/`consolidate` hits family-wide); the invariant's actual
substance — single-writer via `&mut self` — does hold in the built crate, per
TL28 in this same file, and needs no correction. Only the epoch-framing
portion is one of the nineteen pre-implementation instances
`../decisions/001_the_corpus_specifies_an_api_the_crate_did_not_build.md`
catalogs, and rewriting it is a future pass's call.
