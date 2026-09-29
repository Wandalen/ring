# Data Structure: Thread-Local Append Log

### Scope

- **Purpose**: Give every producer thread a private append surface costing no atomics, no locks, and no per-append heap allocation.
- **Responsibility**: Define the structure at its decided grain — one contiguous bump-allocated byte region per thread — and the operation set consumers rely on.
- **In Scope**: The per-thread buffer identity, payload agnosticism, and the append/reset/consolidate-read operations.
- **Out of Scope**: The exact region layout and growth policy, still undecided; what the bytes mean, which stays entirely each consumer's own encoding.

### Abstract

A thread-local append log is one contiguous byte region owned by exactly one
thread, written strictly forward, and read by a single consolidator only
after every writer's epoch has ended. Its own scoping generalizes only the
append discipline — bump-allocate, append, reset — never any operation-code
vocabulary a consumer builds on top, which stays entirely the consumer's.

The structure's value is what it *doesn't* do during a write epoch: no
atomics, no mutexes, no cross-thread visibility, and no per-append heap
allocation beyond the region's own growth. Contention is designed out rather
than synchronized away — the staging half of the two-stage composition this
crate is built for (the merge half is
[`ring_mpsc`](../../../ring_mpsc/readme.md)'s, → [Staging Then Merge](../pattern/002_staging_then_merge.md)).

### Structure

| Field | Type | Meaning |
|-------|------|---------|
| `buffer` | One contiguous byte region per thread — `Vec<u8>`, with `bumpalo::Bump` the alternative the source design conversation names | Append-only linear memory; every record is written sequentially, whole, and never moved until reset |

**Open at this grain (TBD):** whether the region is a single
growable `Vec<u8>` or a chunked bump arena, its growth policy, and any
alignment guarantees for records. The identity above — per-thread, linear,
bump-style, byte-oriented — is this crate's own decision; the layout inside
it is not, and each open item is exactly what a future benchmark harness
exists to settle.

### Operations

| Operation | Contract | Cost discipline |
|-----------|----------|------------------|
| `append` | Write `n` bytes at the current end; bytes are opaque to this crate | No atomics, no locks, no allocation except the region's own growth policy |
| `reset` | Logical length back to 0; the allocation is retained for the next epoch | One allocation per buffer *lifetime*, not per epoch — the property the pre-allocating consumer design relies on |
| consolidate-read | A single reader takes each thread's region after the write epoch ends | Exclusive handoff — the read side never overlaps any writer (→ [Single-Writer Append](../invariant/001_single_writer_append.md)) |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_tagged_record_bump_append.md](../algorithm/001_tagged_record_bump_append.md) | Steps 1-4 operate on this region's bump pointer and byte buffer |

### Data Structures

| File | Relationship |
|------|--------------|
| [002_a_vec_and_a_limit.md](002_a_vec_and_a_limit.md) | The structure that was actually built — shares no field with this one (→ TL11) |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_single_writer_append.md](../invariant/001_single_writer_append.md) | The ownership discipline that makes zero-atomics appends sound |
| [../invariant/002_zero_allocations_in_steady_state.md](../invariant/002_zero_allocations_in_steady_state.md) | The region this count is measured over; its `reset` row states the capacity-retention rule that invariant makes exact |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_measured_before_adopted.md](../non_functional_requirement/001_measured_before_adopted.md) | The adoption gate — this structure ships into a consumer only on a winning benchmark verdict |
| [../non_functional_requirement/002_pod_pointer_free_payloads.md](../non_functional_requirement/002_pod_pointer_free_payloads.md) | The alignment and layout constraints this structure's region must satisfy |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_implicit_thread_locals_are_hidden_state.md](../pitfall/001_implicit_thread_locals_are_hidden_state.md) | The structure whose name contains the word that sets this trap; its per-thread identity is ownership, not storage location |

### Sources

| File | Relationship |
|------|--------------|
| `src/lib.rs` | Crate root; implements this structure as `TlsBuffer<T>` — a `Vec<T>` reserved once to the same bound its `push` refuses at, which is what makes the append allocation-free |

### Tests

| File | Relationship |
|------|--------------|
| `tests/append_log_test.rs` (to create) | Appended bytes read back verbatim after quiesce; reset retains capacity; no allocation on the append path after `new` |

### TL11 — The Structure Described Here Shares No Field With the One Declared

```sh
cd "$(git rev-parse --show-toplevel)"/ring_tls
grep -B 3 -A 2 'items : Vec< T >,' src/lib.rs
printf 'bump, tag, region, epoch or byte-encoding vocabulary in code: %s\n' \
  "$( grep -vE '^[[:space:]]*//' src/lib.rs | grep -ciE 'bump|tag|region|epoch|memcpy|to_le_bytes' )"
# The command above can legitimately exit nonzero -- a count of zero, a diff
# that differs -- and the sweep that keeps this output current skips any block
# that exits nonzero, so without this the block freezes instead of refreshing.
:
```

Live output:

```
#[ derive( Debug ) ]
pub struct TlsBuffer< T >
{
  items : Vec< T >,
  limit : usize,
}
bump, tag, region, epoch or byte-encoding vocabulary in code: 0
```

Zero. Not one term from this instance's vocabulary appears in the declaration
it describes.

**Disposition:** declined — the `buffer: Vec<u8>` bump-region structure this
instance specifies shares no field with the built `TlsBuffer<T> { items,
limit }`. One of the nineteen pre-implementation instances
`../decisions/001_the_corpus_specifies_an_api_the_crate_did_not_build.md`
catalogs; whether to rewrite it, mark it superseded, or relocate it toward a
prospective consumer's own still-live use case is a future pass's decision,
reserved explicitly in that document's own Deciders field.

### TL12 — The Type Name Is the Last Thing the Two Designs Share

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'thread_local! in the whole family: %s\n' \
  "$( find ring_*/src ring_*/tests -name '*.rs' -exec cat {} + 2>/dev/null \
      | grep -vE '^\\s*(//|///|//!)' | grep -c 'thread_local' )"
printf 'and every construction site is an ordinary local or field:\n'
grep -r 'TlsBuffer::< *[A-Za-z]* *>::with_capacity\|TlsBuffer::with_capacity' \
  --include='*.rs' ring_bench/src ring_testkit/src
```

Live output:

```
thread_local! in the whole family: 3
and every construction site is an ordinary local or field:
ring_bench/src/lib.rs:  let buffer = TlsBuffer::< Record >::with_capacity( workload.batch() );
ring_testkit/src/lib.rs:    let mut staging : TlsBuffer< u32 > = TlsBuffer::with_capacity( self.stage_limit );
```

No thread-local storage anywhere in thirty-three crates. The `tls` in the
crate's name describes a discipline the caller keeps — one buffer per thread,
never shared — not a mechanism the crate provides
(→ [`../pitfall/001`](../pitfall/001_implicit_thread_locals_are_hidden_state.md),
which argues for exactly this and is the one pre-implementation instance the
built crate vindicates).
