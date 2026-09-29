# Non-Functional Requirement: POD, Pointer-Free, Page-Aligned Payloads

### Scope

- **Purpose**: Fix the payload and region constraints that are the shared precondition for zero-copy handoff and OS copy-on-write snapshotting, so neither capability is discovered to be unreachable after a layout is already frozen.
- **Responsibility**: State the POD/pointer-free/page-aligned requirement, price what it buys with numbers, name where it buys nothing (`wasm32`), and give a compile-time check plus a grep that must print nothing.
- **In Scope**: What an appended payload's type may contain, and the alignment and length granularity of the region holding it.
- **Out of Scope**: The append procedure copying those bytes (→ [Tagged-Record Bump Append](../algorithm/001_tagged_record_bump_append.md)); the OS mechanism consuming the precondition, which this crate satisfies but does not implement; the benchmark verdict gating adoption (→ [Measured Before Adopted](001_measured_before_adopted.md)).

### Quality Attribute

**Portability and zero-copy capability.** Both reduce to one question about
the bytes in the region: are they meaningful to a reader that is not the
writing thread, in an address space that is not the writer's, at a base
address that is not the one they were written at? Every capability layered
on this crate — handing a sealed region off by pointer swap, forking a
snapshot, casting the region to a byte slice for an analytical reader — is
that question answered yes.

### Statement

Appended payloads are `#[repr(C)]` plain-old-data containing **no pointers** —
neither into the log region itself nor into the heap — and the region is
**page-aligned**.

Three clauses, each load-bearing for a different reason:

- **`#[repr(C)]`, not `#[repr(Rust)]`.** Rust's default layout is
  deliberately unspecified: field order and padding may differ between
  compiler versions and between builds. A region written by one build and read
  by another — a persisted snapshot, two crates on different toolchains, an
  external analytical reader — needs a layout that is a contract, not an
  implementation detail.
- **No pointers, and no padding.** The pointer clause is the obvious one; the
  padding clause is the one usually missed. A `#[repr(C)]` struct with
  interior padding copied via a `size_of`-bounded slice cast writes
  *uninitialized* padding bytes into the region. That breaks byte-for-byte
  determinism between two runs of the same workload, and it is exactly what
  `bytemuck::Pod`'s no-padding requirement exists to reject. The POD bound
  must be the no-padding kind, not merely "contains no references."
- **Page-aligned region.** 4 KB is the page-table granularity the copy-on-write
  mechanism operates at; 2 MB huge pages are the named alternative
  granularity. Alignment of the *base* is necessary but not sufficient — see
  the length clause under Measurement Method.

**Why a pointer specifically cannot cross the boundary**, stated precisely
rather than as a slogan, because the three consumers fail differently:

- **Under `fork()`**, a pointer does resolve in the child — the address space
  is copied, so the numeric value still names something. What it names is the
  child's own copy-on-write copy of the pointee, which is correct only if the
  pointee is itself inside the snapshotted region. A pointer into the general
  heap drags heap pages into the CoW set and makes the snapshot's completeness
  depend on allocator state, which is precisely the coupling an isolated
  simulation arena exists to prevent.
- **Under a cross-process or relocated mapping** — `memfd_create`, a region
  mapped into a second process, a snapshot reloaded at a different base — the
  pointer is simply wrong. Its numeric value names nothing the reader's
  address space maps, and there is no fixup pass to run because the bytes
  carry no type information saying which of them were pointers.
- **Under an in-process pointer-swap handoff**, a pointer *into the region
  itself* is invalidated by the swap and by any growth that relocates the
  region — a self-referential offset would survive both, which is why offsets,
  not pointers, are the correct encoding where a payload must reference
  another part of the log.

The general rule underneath all three: a pointer's validity is
address-space-scoped and base-address-scoped, so it survives at most one of
this crate's three intended consumers, and only by accident.

**What the constraint buys, with numbers:**

- **`fork()` completes in under a microsecond regardless of region size**,
  because no physical memory is copied — every page-table entry is flagged
  read-only/copy-on-write instead. The incremental cost afterward is **one
  4 KB page copy per page actually written**, paid by whichever side writes
  first. At 2 MB huge-page granularity the page-table entry count drops by
  512×, and the per-touched-page copy cost rises by the same factor — the
  trade, not a strict improvement.
- **Zero-copy handoff**: a sealed region is handed off by swapping a pointer
  rather than copying its bytes, so the handoff cost is O(1) in the region's
  size instead of O(n). The source design corpus's own consolidation stage
  does exactly this — the consuming thread swaps each buffer's raw pointer,
  zero-copy, rather than draining it byte by byte.
- **Byte-slice view with no serialization pass**: a region of `#[repr(C)]` POD
  can be reinterpreted as `&[u8]` (and back) directly — a `bytemuck`-class
  cast — with no encoder, no schema walk, and no intermediate buffer.

**Where this buys less: `wasm32`.** Wasm runs inside a single isolated linear
memory with no `fork()`, no `mmap()`, and no OS system calls at all, so the
copy-on-write path **does not exist there** — not as a slower variant, as an
absent one. The POD discipline still buys the byte-slice view and the
in-linear-memory pointer-swap handoff on that target; a snapshot costs a real
`memcpy` of the whole region — the flat-copy fallback shape any page backend
without `fork()` must take for the same reason. This is a stated platform
asymmetry, not a defect to close: nothing this crate can do makes `fork()`
exist on `wasm32`.

### Measurement Method

**Compile-time, for the payload clause.** A trait bound on the typed append
entry point — `bytemuck::Pod`-shaped, or an equivalent local unsafe marker
trait, since this crate carries an empty `[dependencies]` and adding one is
itself a decision — so a non-POD payload is a *compile error*. The bound is
the whole enforcement: a runtime POD check would mean the bound was not
applied, and would catch the violation on the machine that already paid for
it.

**Grep, for this crate's own source.** Runnable today against the skeleton,
and meaningful as it grows:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_tls
command grep -rnE '\*(const|mut) |NonNull|Box<|Rc<|Arc<|Cell<|RefCell<' src/ --include='*.rs' || true
```

Live output:

```
```

Two honest caveats on that command, stated rather than left for the reader to
discover. It checks *this crate's* types, not a caller's payloads — the
compile-time bound is what covers those, and nothing else can. And if a
future revision adopts a chunked bump arena instead of one contiguous region,
the region's own internal cursor may legitimately become a raw pointer, at
which point this grep narrows to the payload surface only and the bound
becomes the entirety of the enforcement. `Vec<u8>` is deliberately absent from
the pattern: it is a candidate for the region itself, not a payload.

**Runtime assertion, for the alignment clause — both halves.** Base alignment
alone is insufficient:

```rust
// tests/page_alignment_test.rs (to create)
assert_eq!( region.as_ptr() as usize % PAGE, 0 );   // base
assert_eq!( region.capacity() % PAGE, 0 );          // length granularity
```

The second assertion is the one that is easy to omit and expensive to omit. A
page-aligned base with a capacity that is not a whole multiple of the page
size leaves the region's final page shared with whatever the allocator placed
after it — so an unrelated write to that neighbour triggers a copy of a page
the region owns, silently coupling the snapshot's cost to code that has
nothing to do with this crate.

### Acceptance Threshold

- **Payload POD-ness: 100% compile-time.** Every appended payload type
  satisfies the bound, and the count of *runtime* POD checks is **0** — a
  nonzero count means the bound is not doing the work.
- **No padding.** Every payload type is padding-free, verified by the
  no-padding form of the bound rather than by inspection.
- **Alignment: base ≡ 0 and capacity ≡ 0 (mod page size)** on every native
  target, for whichever granularity is chosen. On `wasm32` the alignment
  requirement is retained — it costs essentially nothing and keeps one code
  path — but its copy-on-write justification does not apply there, so the
  acceptance case on that target is the byte-slice view round-tripping, never
  a fork.
- **The grep above prints nothing.**
- **Explicitly *not* a threshold: fork latency.** The sub-microsecond figure
  is a property of the kernel's page-table manipulation, not of this crate.
  Writing it into an acceptance bar would mean this crate's gate passes or
  fails on someone else's implementation. The obligation here is to satisfy
  the *precondition*; measuring the mechanism belongs to whichever consumer
  adopts it.

**Open (TBD):** whether the granularity is 4 KB or 2 MB. The
trade is stated above — 512× fewer page-table entries against a 512× larger
copy per touched page — and it cannot be settled independently of the region
layout question [Tagged-Record Bump
Append](../algorithm/001_tagged_record_bump_append.md) already leaves open,
because a chunked arena's chunk size and the page granularity are the same
number if chunks are page-backed. Also open: whether the log guarantees any
*record-level* alignment inside the region, which is what decides whether a
`#[repr(C)]` payload can be read back by reference or only by copy-out.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_tagged_record_bump_append.md](../algorithm/001_tagged_record_bump_append.md) | Step 3's operand copy is where the POD and no-padding constraints bind |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_thread_local_append_log.md](../data_structure/001_thread_local_append_log.md) | The region these alignment and layout constraints apply to |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_single_writer_append.md](../invariant/001_single_writer_append.md) | The exclusive-ownership discipline that makes a pointer-swap handoff expressible at all |
| [../invariant/002_zero_allocations_in_steady_state.md](../invariant/002_zero_allocations_in_steady_state.md) | Why reset needs no `Drop` walk (POD has no destructors), and why a rebuilt region would forfeit the alignment guaranteed here |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [001_measured_before_adopted.md](001_measured_before_adopted.md) | The sibling gate — that one is comparative against a baseline, this one is structural and has no baseline to beat |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_implicit_thread_locals_are_hidden_state.md](../pitfall/001_implicit_thread_locals_are_hidden_state.md) | The pointer-swap handoff this requirement enables is precisely what a thread-local-stashed region forecloses |

### Sources

| File | Relationship |
|------|--------------|
| `src/lib.rs` | Crate root; skeleton — no payload bound or alignment guarantee expressed yet |

### Tests

| File | Relationship |
|------|--------------|
| `tests/page_alignment_test.rs` (to create) | Region base and capacity are both whole page multiples; a region round-trips through a byte-slice view unchanged |
| `tests/pod_bound_test.rs` (to create) | Compile-fail cases: a payload containing a reference, a `Box`, and a padded `#[repr(C)]` struct are each rejected by the bound |

### TL42 — The POD and Alignment Preconditions Are Unenforceable on a Typed `Vec`

```sh
cd "$(git rev-parse --show-toplevel)"/ring_tls
grep 'pub struct TlsBuffer' src/lib.rs
grep 'impl< T > TlsBuffer< T >' src/lib.rs
```

Live output:

```
pub struct TlsBuffer< T >
impl< T > TlsBuffer< T >
```

No `T : Copy`, no `T : Pod`, no alignment bound. The requirement made sense for
a byte region a consolidator walks; for a typed `Vec` the compiler already
guarantees layout and the requirement has nothing left to constrain — except
that nothing now stops a `T` whose `Drop` is the very thing
[`../decisions/002`](../decisions/002_a_refused_push_destroys_the_item_its_doc_promises_to_return.md)
makes dangerous.
