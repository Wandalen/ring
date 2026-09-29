# API: Writer Append Surface

### Scope

- **Purpose**: State the operations an appending thread calls, and establish the two properties the signature must preserve — no synchronization, and construction in place.
- **Responsibility**: Fix the operation set, the in-place-construction requirement, the capacity failure mode, and what external visibility means for this surface's stability.
- **In Scope**: Append and the capacity query, from the owning thread's side.
- **Out of Scope**: The steps inside append (→ [Tagged Record Bump Append](../algorithm/001_tagged_record_bump_append.md)); registration, which must have already happened (→ [Register Before First Append](../pattern/001_register_before_first_append.md)); the consolidator's side (→ [Consolidator Read Surface](002_consolidator_read_surface.md)).

### Abstract

The writer surface is called by exactly one thread — its owner — and performs
**no synchronization of any kind**: no atomics, no locks, no fences. Not
"lock-free" in the technical sense that admits atomics; genuinely
synchronization-free, because the buffer has one accessor
(→ [Single-Writer Append](../invariant/001_single_writer_append.md)). An
append is a bounds check, a payload write, and a pointer bump.

That makes this the cheapest write path the family offers, and the comparison
worth holding in mind is that
[`ring_mpsc`](../../../ring_mpsc/docs/api/001_producer_publish_surface.md)'s
producer surface — which is itself wait-free and fast — still pays one atomic
fetch-add per record. This surface pays zero. The price is paid elsewhere: no
ordering across threads, and a consolidation step that has no counterpart in
the ring.

**This crate is externally visible**, one of `ring_factory`, `ring_handle`,
`ring_tls`, `ring_flush`, `ring_types` — the family's five exported crates. So
unlike an internal family crate, changes here are visible to consumers
directly, and the Compatibility Guarantees below are a real contract rather
than an internal note.

### Operations

| Operation | Signature shape | Contract | Cost |
|-----------|-----------------|----------|------|
| `append` | `&mut self, tag, payload` | Writes a tagged record at the bump pointer and advances it; caller must be the owning thread | Bounds check + write + bump. No atomic |
| `append_with` | `&mut self, tag, len, f: impl FnOnce(&mut [u8])` | Reserves `len` bytes and lets the caller fill them **in place** | Same, minus a copy |
| `remaining` | `&self -> usize` | Bytes left before the region is exhausted | One load; **not advisory** — see below |

**`append_with` is not a convenience variant; it is the operation that makes
the design worth having.** A plain `append(payload)` requires the caller to
have already materialized the payload somewhere — on the stack, typically —
and then copies it into the region. For a large record that is a full extra
copy of every byte, on the hot path, defeating the point of a bump allocator.
`append_with` hands the caller the destination and lets it construct there.
If only one of the two operations survives, it should be this one.

**`remaining` is genuinely actionable here, unlike its ring counterpart.**
[`ring_mpsc`](../../../ring_mpsc/docs/lifecycle/004_ring_occupancy_between_cursors.md)'s
`free_capacity` is advisory because other producers can consume the room
between the read and the claim. Here there are no other producers — the owner
is the sole writer, so a value it reads cannot be invalidated by anyone else.
Check-then-append is sound on this surface and unsound on that one, for the
same-looking code. That asymmetry is exactly the kind of thing a consumer
using both crates will get wrong, and it is why the two surfaces are
documented separately rather than as one "write path API."

**`&mut self` is doing real work in these signatures.** It encodes the
single-writer invariant in the type system rather than in prose: a `&mut`
cannot be aliased, so two threads cannot hold appending access simultaneously
in safe code. Whether the thread-local storage actually yields `&mut` — or a
`&` with interior mutability, which would silently drop the guarantee — is a
decision that determines whether the crate's central invariant is enforced or
merely documented. **Undecided**, and it is the single most consequential
signature question on this surface.

### Error Handling

| Condition | Behaviour | Is it an error? |
|-----------|-----------|-----------------|
| Region exhausted | **Undecided** — grow, or fail | The one genuine failure mode |
| Called from a non-owning thread | Unsound; a data race | Prevented by `&mut self`, *if* the storage yields `&mut` |
| Called before registration | Succeeds, and the data is silently lost | Not detectable here (→ [Registration State](../lifecycle/004_registration_state.md)) |
| Called during consolidation | Unsound under the barrier mechanism; fine under double-buffer | Depends on the undecided E2 mechanism |

**The exhaustion case decides this signature's return type, exactly as
backpressure decides the ring's.** Growing keeps `append` infallible and
breaks
[Zero Allocations in Steady State](../invariant/002_zero_allocations_in_steady_state.md)
— which that invariant permits as a bounded exception rather than forbidding,
but every growth is a real allocation on the hot path at the worst moment.
Failing makes it `append(...) -> Result<(), Full>` and pushes a decision to
every call site, most of which will have nothing useful to do. A third option
— grow, but report it — keeps the signature infallible while making
constraint 1 of
[Consolidation Cycle](../lifecycle/002_consolidation_cycle.md) observable,
which is the property that would let a consumer tune its period correctly
instead of guessing.

**The third row deserves emphasis because it is the one with no defense.**
Appending before registration succeeds and silently loses the data. There is
no return value that could signal it, because from the buffer's perspective
nothing is wrong. The only remedies are structural: make the append surface
unreachable until registration completes — a builder or a token — or accept
the usage rule and document it
(→ [Register Before First Append](../pattern/001_register_before_first_append.md)).

### Compatibility Guarantees

1. **Synchronization-freedom is the guarantee.** Not a signature but a
   property, and the one this surface exists to provide. A future revision
   introducing an atomic into `append` — even one preserving every signature
   — would be a breaking change to the only contract that matters here.
2. **This surface is externally visible and therefore genuinely constrained.**
   Unlike `ring_mpsc`, which hides behind `ring_handle` and can change its
   surface freely, this crate is named in the family's export list. Its
   signatures reach consumers. The open questions above are consequently
   more expensive to leave open than the equivalent internal ones, and that
   is a reason to settle them earlier rather than a reason to pretend they
   are settled.
3. **In-place construction will remain available.** Whatever else changes,
   `append_with` or an equivalent survives; removing it would remove the
   reason to use a bump allocator.
4. **Payload agnosticism is currently unbounded and will narrow.** The POD
   and alignment preconditions
   (→ [POD and Pointer-Free Payloads](../non_functional_requirement/002_pod_pointer_free_payloads.md))
   are stated as requirements there and not yet expressed as bounds here.
   Expect them to become bounds, which is breaking in the ordinary Rust
   sense.
5. **Not stable overall.** The mechanism is ungated until
   [Measured Before Adopted](../non_functional_requirement/001_measured_before_adopted.md)
   clears, and the region layout is undecided.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_tagged_record_bump_append.md](../algorithm/001_tagged_record_bump_append.md) | The procedure this surface is the caller's view of; its one growth allocation is the exhaustion case above |

### APIs

| File | Relationship |
|------|--------------|
| [002_consolidator_read_surface.md](002_consolidator_read_surface.md) | The cross-thread counterpart; its operations are mutually exclusive with this one in time |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_thread_local_append_log.md](../data_structure/001_thread_local_append_log.md) | The region and bump pointer these operations act on |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_single_writer_append.md](../invariant/001_single_writer_append.md) | What `&mut self` would encode in the type system, and what interior mutability would silently discard |
| [../invariant/002_zero_allocations_in_steady_state.md](../invariant/002_zero_allocations_in_steady_state.md) | The exhaustion case is the one permitted exception to it |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_thread_registration_and_teardown.md](../lifecycle/001_thread_registration_and_teardown.md) | This surface is callable only in its T2; the before-registration error row is its T1 |
| [../lifecycle/002_consolidation_cycle.md](../lifecycle/002_consolidation_cycle.md) | Its constraint 1 is what the exhaustion case's "grow, but report it" option would make observable |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_measured_before_adopted.md](../non_functional_requirement/001_measured_before_adopted.md) | Why this surface is not stable despite being externally visible |
| [../non_functional_requirement/002_pod_pointer_free_payloads.md](../non_functional_requirement/002_pod_pointer_free_payloads.md) | The preconditions Compatibility Guarantee 4 expects to become bounds |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_register_before_first_append.md](../pattern/001_register_before_first_append.md) | The usage rule the third error row cannot enforce from inside |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_implicit_thread_locals_are_hidden_state.md](../pitfall/001_implicit_thread_locals_are_hidden_state.md) | An implicit thread-local accessor would let this surface be called with no visible receiver — the trap it names, reached through this API |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/003_buffer_epoch_cycle.md](../lifecycle/003_buffer_epoch_cycle.md) | This surface is callable only in Appending; the fourth error row is the E2-dependent case |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_record_tag.md](../type/001_record_tag.md) | `append`'s tag parameter; its width is what bounds the per-record overhead |

### Sources

| File | Relationship |
|------|--------------|
| `src/lib.rs` | Crate root; skeleton — no operations declared yet |

### Tests

| File | Relationship |
|------|--------------|
| `tests/writer_api_test.rs` (to create) | An append loop executes with zero atomic operations, asserted by instrumentation rather than by inspection — Compatibility Guarantee 1 |
| `tests/writer_api_test.rs` (to create) | `append_with` writes the payload directly into the region with no intermediate copy, asserted by comparing the destination address against the closure's slice |

### TL7 — The Writer Surface Specified Here Is Not the One Exported

This instance leaves the append signature open between three candidates.
The crate answered by exporting a fourth: `push( &mut self, item : T ) ->
Result< (), RingError >`, typed rather than byte-oriented, with no tag argument
and no closure variant.

A caller reading this instance writes `buffer.append( tag, &bytes )` and does
not compile. That is the difference between this finding and TL1's: an
algorithm a reader cannot invoke wastes their time, and a signature they can
type out wastes a compile.

**Disposition:** declined — Operations, Error Handling, and Compatibility
Guarantees all specify `append`/`append_with`/`remaining` against a
byte-region signature the crate never exported; the built surface is `push`.
One of the nineteen pre-implementation instances
`../decisions/001_the_corpus_specifies_an_api_the_crate_did_not_build.md`
already catalogs, whose resolution (rewrite/supersede/relocate) is explicitly
a future pass's to rule on, not this one's.

### TL8 — The Open Signature Question Was Closed in Code and Not Recorded

[`../decisions/readme.md`](../decisions/readme.md) names the buffer layout as
tracked at benchmark grain "because the answer is shared with a prospective
consumer's own append-path replacement and isn't this crate's alone to
decide".

It was nonetheless decided here, unilaterally, by building `Vec< T >`. The
sentence is still in the file and is still true about the *process* — what
changed is that the process was bypassed and nothing in the corpus said so
until [`../decisions/001`](../decisions/001_the_corpus_specifies_an_api_the_crate_did_not_build.md).
