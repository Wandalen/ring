# Algorithm: Tagged-Record Bump Append

### Scope

- **Purpose**: Specify the exact write sequence one append performs — tag byte, operand memcpy, pointer bump — so its cost profile is reviewable as a procedure rather than asserted as a property.
- **Responsibility**: State the steps, the single branch, the one growth allocation, and the boundary at which record meaning stops being this crate's concern.
- **In Scope**: `append`'s own procedure and `reset`'s rewind, over one region owned by one writer.
- **Out of Scope**: The region's internal layout and growth policy (→ [Thread-Local Append Log](../data_structure/001_thread_local_append_log.md), still undecided); what any tag byte means, which is the consumer's vocabulary and never this crate's; the order many regions merge in (→ [`ring_mpsc`'s Batch Drain by Cursor Swap](../../../ring_mpsc/docs/algorithm/002_batch_drain_by_cursor_swap.md), a separate crate's procedure).

### Abstract

A record is a flat byte stream, not a typed value: **one tag byte followed by
raw little-endian operand bytes**, appended by advancing a monotonic bump
pointer into one contiguous region. This is an encoding and
allocation-strategy decision independent of concurrency — it applies equally
to a single uncontended writer, which is one independent reason Step 4 below
needs no ordering argument. The whole of an append is a bounds check, a byte
store, a `memcpy`, and an integer add.

Three costs are absent by construction rather than optimized away. There is
**no per-record allocator call** — the region is obtained once and reused, so
appending N records costs 0 allocations, not N (→ [Zero Allocations in Steady
State](../invariant/002_zero_allocations_in_steady_state.md)). There is **no
free list** — nothing is ever individually released, so there is no metadata
to maintain, no fragmentation to manage, and no coalescing pass. There is **no
destructor to run** — the region holds plain bytes, so reset is a rewind of
one integer rather than a walk over N live values.

The contrast that gives this its shape is a `Vec<Command>` of typed enum
values: that costs one heap allocation per boxed payload, a `Drop` walk over
every element at clear time, and a per-element size equal to the widest
variant. The byte stream costs one growth allocation for the whole region, no
`Drop` walk at all, and exactly the bytes each record actually needs.

### Algorithm

**Input** — `&mut self` (an exclusive borrow; this is the enforcement point of
[Single-Writer Append](../invariant/001_single_writer_append.md), not a
stylistic choice), `tag: u8`, `operands: &[u8]`.

**Output** — the region's logical length grows by exactly `1 +
operands.len()`; the appended bytes are readable back verbatim, in append
order, after the write epoch ends.

**Step 1 — bounds check.** Compute `need = 1 + operands.len()`. If `len + need
> capacity`, grow the region once. This is the procedure's only branch and its
only allocation; in steady state it is never taken, which is precisely what
[Zero Allocations in Steady
State](../invariant/002_zero_allocations_in_steady_state.md) asserts as a
measurable count rather than a hope. The growth factor and whether growth
relocates the region are both open (→ **Open** below).

**Step 2 — write the tag.** One byte store at `base + len`. The value is the
caller's; this crate neither reads nor validates it.

**Step 3 — copy the operands.** `memcpy` of `operands.len()` bytes at `base +
len + 1`. For a fixed-width scalar operand the length is a compile-time
constant and the copy compiles to a single store. The little-endian byte
encoding is produced by the caller (`to_le_bytes` at the call site, or an
unsafe `size_of`-bounded slice cast for a `#[repr(C)]` struct — see
[POD, Pointer-Free, Page-Aligned
Payloads](../non_functional_requirement/002_pod_pointer_free_payloads.md)), so
the log itself never interprets a field, never byte-swaps, and never has an
endianness of its own.

**Step 4 — bump.** `len += need`. One non-atomic integer add. No fence, no
atomic, no lock: the write's soundness comes entirely from the exclusive
borrow taken in Step 1's signature, never from memory ordering
(→ [Single-Writer Append](../invariant/001_single_writer_append.md)). This is
the property distinguishing this procedure from `ring_mpsc`'s cross-thread
publication, which does need an ordering argument
(→ [Claim-Then-Publish](../../../ring_mpsc/docs/algorithm/001_claim_then_publish.md)).

**Reset** — `len = 0`. A rewind of the cursor to the region base, not a
deallocation, not a zero-fill, and not a `Drop` walk. Bytes above the new
`len` are stale but unreadable through the log's own API, which never exposes
past its cursor.

#### What the procedure deliberately omits

- **No per-record length prefix.** A record is one tag byte plus operands; the
  operand width is derivable from the tag *by the caller's decoder*, which is
  why the tag vocabulary being the caller's is a structural fact and not an
  API-surface preference. A length prefix would cost 4 bytes per record and
  buy this crate an ability it has deliberately declined: skipping a record it
  cannot interpret.
- **No record index or offset table.** Records are found by sequential decode
  from the region base. Random access to record *k* is not an operation this
  structure has.
- **No validation.** A tag byte followed by the wrong operand width is a
  caller bug this crate is structurally unable to detect. It will faithfully
  store the malformed record and faithfully hand it back, and the misparse
  surfaces one full epoch later in the consolidator — the same late,
  displaced failure shape [Single-Writer
  Append](../invariant/001_single_writer_append.md)'s Violation Consequences
  describes for an interleaved buffer. That is the honest price of payload
  agnosticism, paid deliberately.

#### The tag vocabulary is the caller's, not this crate's

This crate defines no opcode enum, assigns no meaning to any tag value, and
never decodes. This crate's own payload-agnostic design draws exactly this
boundary: whatever operation vocabulary a consumer encodes into its tags
stays entirely that consumer's, and out of scope here.

Concretely, each prospective consumer keeps its own vocabulary of structural
mutations, independently maintained. Whether two such vocabularies overlap
enough to warrant a third shared crate is an open question this crate's own
design does not depend on the answer to, and is not decided here.

The practical test of the boundary: this crate's whole API surface can be
written without naming a single opcode, and a consumer can change its entire
vocabulary — renumber every tag, add a sixth op — without this crate's source
changing by one line. Where that stops being true, the payload has leaked in.

#### Open (TBD)

- **Region shape.** A single growable `Vec<u8>` or a chunked bump arena. The
  choice decides whether Step 1's growth *relocates* the region: a `Vec`
  realloc moves the bytes and invalidates any pointer or slice a consumer
  holds into them, where a chunked arena appends a fresh chunk and leaves
  existing bytes in place. Confining growth to warm-up (→ [Zero Allocations in
  Steady State](../invariant/002_zero_allocations_in_steady_state.md)) is what
  makes the difference tolerable either way, not a reason it does not matter.
- **Growth factor**, and whether the region is pre-sized from the previous
  cycle's observed peak instead of grown reactively at all.
- **Record alignment.** Step 3 copies at whatever byte offset the cursor
  happens to sit on, so a `#[repr(C)]` operand lands unaligned in general and
  is readable only via an unaligned load or a copy-out. Whether the log
  guarantees any alignment — and pays the padding bytes that costs — is
  undecided, and interacts directly with the page-alignment requirement in
  [POD, Pointer-Free, Page-Aligned
  Payloads](../non_functional_requirement/002_pod_pointer_free_payloads.md).

### Algorithms

| File | Relationship |
|------|--------------|
| [../../../ring_mpsc/docs/algorithm/001_claim_then_publish.md](../../../ring_mpsc/docs/algorithm/001_claim_then_publish.md) | The cross-thread counterpart needing a memory-ordering argument this procedure does not — the cost difference exclusive ownership buys |
| [../../../ring_mpsc/docs/algorithm/002_batch_drain_by_cursor_swap.md](../../../ring_mpsc/docs/algorithm/002_batch_drain_by_cursor_swap.md) | The consuming side; this procedure's output is what a drain reads, but the merge order across regions is that crate's contract |

### Data Structures

| File | Relationship |
|------|--------------|
| [../../../ring_mpsc/docs/data_structure/001_sequence_stamped_ring.md](../../../ring_mpsc/docs/data_structure/001_sequence_stamped_ring.md) | The merge half's structure — contrast: its slots are sequence-stamped because many threads write them, where this region needs no stamp at all |
| [../data_structure/001_thread_local_append_log.md](../data_structure/001_thread_local_append_log.md) | The region this procedure writes into; its `append`/`reset` operation rows are what these steps specify |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_single_writer_append.md](../invariant/001_single_writer_append.md) | Step 1's `&mut self` is this invariant's enforcement point; Step 4's lack of any fence is what it buys |
| [../invariant/002_zero_allocations_in_steady_state.md](../invariant/002_zero_allocations_in_steady_state.md) | Why Step 1's growth branch is never taken after warm-up, and what reappearing allocation would cost |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_measured_before_adopted.md](../non_functional_requirement/001_measured_before_adopted.md) | The adoption gate — this procedure's cost profile is what the benchmark measures |
| [../non_functional_requirement/002_pod_pointer_free_payloads.md](../non_functional_requirement/002_pod_pointer_free_payloads.md) | Constrains what Step 3 may copy — POD only, no pointers, no padding |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_implicit_thread_locals_are_hidden_state.md](../pitfall/001_implicit_thread_locals_are_hidden_state.md) | Where Step 1's `&mut self` receiver comes from — an owned handle, never a thread-local lookup |

### Sources

| File | Relationship |
|------|--------------|
| `src/lib.rs` | Crate root; skeleton — no implementation of this procedure yet |

### Tests

| File | Relationship |
|------|--------------|
| `tests/append_procedure_test.rs` (to create) | A record round-trips byte-for-byte; exactly `1 + operands.len()` bytes are consumed per append; reset leaves capacity unchanged and the next append starts at the region base |

### TL1 — The Procedure Specified Here Has No Entry Point in the Crate

The procedure's first step is `append`, its bounds check reads `remaining`,
and its rewind is `reset`. None of the three is declared.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_tls
for n in append append_with remaining reset; do
  printf '  %-12s declared: %s\n' "$n" \
    "$( grep -vE '^\\s*(//|///|//!)' src/lib.rs | grep -cE "fn $n\\b" )"
done
```

Live output:

```
  append       declared: 0
  append_with  declared: 0
  remaining    declared: 0
  reset        declared: 0
```

The built procedure is `push` for one item and `flush_into` for the batch
(→ [`../algorithm/002`](../algorithm/002_the_fused_claim_and_drain.md)). This
instance is not wrong about *a* design — it is accurate about the one described
in [`../decisions/001`](../decisions/001_the_corpus_specifies_an_api_the_crate_did_not_build.md)
and never built.

**Disposition:** declined — Steps 1-4 and Reset specify `append`/`append_with`/
`remaining`/`reset` over a byte region, none of which the crate declares; this
is one of the nineteen pre-implementation instances
`../decisions/001_the_corpus_specifies_an_api_the_crate_did_not_build.md`
already catalogs. That document's own Deciders field reserves the choice among
rewrite, supersede-in-place, or relocate to a prospective consumer's own crate
for a future pass — picking one unilaterally here, file by file, would
preempt a decision the corpus itself has already filed as open.

### TL2 — The One Growth Allocation Is the One Thing Both Designs Agree On

This instance specifies a region sized once and an append that refuses past
its end. The built crate reserves `Vec::with_capacity( limit )` and refuses at
`limit`, so `Vec`'s growth path is never taken.

Two structures, two mechanisms, one guarantee — which is why
[`../invariant/002`](../invariant/002_zero_allocations_in_steady_state.md) holds
for the crate that exists even though it argues from the crate that does not.
It is the only instance among the nineteen for which that is true.
