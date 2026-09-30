# Invariant: Zero Allocations in Steady State

### Scope

- **Purpose**: Make "no per-append allocation" an exactly-countable property — zero, not low — so a reintroduced allocator call is a test failure rather than a latency mystery found under load.
- **Responsibility**: State the count, define when steady state begins, name the rewind-not-deallocate mechanism enforcing it, and price the lock an allocator call would smuggle back onto a lock-free path.
- **In Scope**: Allocator and deallocator call counts on the append and reset paths, across cycles, per region.
- **Out of Scope**: The growth policy governing the warm-up allocations this invariant excludes (→ [Thread-Local Append Log](../data_structure/001_thread_local_append_log.md), still undecided); the measured latency the property is supposed to buy (→ [Measured Before Adopted](../non_functional_requirement/001_measured_before_adopted.md)).

### Invariant Statement

After warm-up, a cycle that appends any number of records to a region performs
**exactly zero allocator calls and exactly zero deallocator calls**. Buffers
are cleared and their capacity retained across cycles, never dropped.

**Warm-up** is the finite prefix of cycles during which any region still
grows. Steady state begins at the first cycle whose peak byte count does not
exceed the capacity the region has already reached; it is a property of the
workload's high-water mark, not of a cycle number. Naming the boundary matters
because it is what makes the count exact: the claim is `0`, and a harness that
observes `1` has found a defect, not noise.

Restated as a budget: one allocation per region *lifetime* under a
never-exceeded high-water mark, N allocations under a workload that keeps
growing — never one per append, and never one per cycle. This is a
clear-and-retain-capacity recycling shape: the region is never dropped
between cycles, only emptied in place, and the following cycle writes into
the capacity already reached — the same recycling shape this invariant
states as an exact zero-per-tick count rather than a qualitative habit.

### Enforcement Mechanism

**Reset is a rewind, not a release.** Resetting sets the cursor to the region
base — one integer store — and returns the allocation untouched. Nothing is
freed, nothing is zero-filled, and no `Drop` walk runs, because the region
holds plain bytes with no destructors
(→ [POD, Pointer-Free, Page-Aligned
Payloads](../non_functional_requirement/002_pod_pointer_free_payloads.md)).
`drop` is never called on the region between cycles; the region outlives every
cycle it serves.

**Half of this is free from the language, and half is discipline — the split
is worth stating rather than blurring.** `Vec::clear` gives exactly the
required behavior for byte elements: it drops nothing and retains capacity, so
the correct implementation is also the idiomatic one. What no compiler
prevents is a consumer or a later maintainer writing `*region = Vec::new()`,
letting the region drop at cycle end and rebuilding it, or reaching for a
per-record `Box`. Those all compile cleanly and all violate this invariant.
The type system enforces the single-writer discipline
(→ [Single-Writer Append](../invariant/001_single_writer_append.md)); it does
not enforce this one, and claiming otherwise would overreach the same way a
no-globals clause would if it claimed compiler enforcement it does not have:
no globals, statics, or thread-local stashes is a discipline the type system
cannot check for either.

**What is checkable, mechanically.** A counting global allocator in a test
harness, snapshotted around the steady-state window:

```rust
// tests/zero_alloc_test.rs (to create)
// 1. run one warm-up cycle at the workload's peak record count
// 2. snapshot ( allocs, deallocs, region.capacity() )
// 3. run 1_000 further cycles of append-then-reset
// 4. assert allocs_delta == 0 && deallocs_delta == 0
// 5. assert region.capacity() == snapshotted capacity
```

Step 5 is not redundant with step 4, and both halves are required. A zero
allocation count paired with a *shrunk* capacity would mean the region is
being rebuilt through some path the counter does not see; a retained capacity
paired with a nonzero count would mean something else on the append path is
allocating. Only the conjunction says what this invariant claims.

The complementary check — that the growth path exists and works — belongs in
the same file rather than being left implicit: a region driven past its
high-water mark must allocate exactly once and preserve every previously
appended byte across the growth. An invariant asserting "0 in steady state"
is falsifiable only if warm-up is separately shown to be non-zero.

### Violation Consequences

**An allocator call on the append path reintroduces a lock into a path whose
entire purpose is being lock-free.** Most global allocators synchronize:
glibc's `malloc` guards each arena with a mutex, and thread-caching allocators
(jemalloc, mimalloc, tcmalloc) are lock-free only on the thread-cache hit path
— a cache miss, a refill, or a size class the cache does not carry falls back
to a locked central or arena path. So the cost is not "an allocation"; it is a
lock, acquired inside the one procedure the crate exists to keep
synchronization-free. Every argument this crate makes — appends need no
atomics, contention is designed out rather than synchronized away — is priced
on that procedure containing no lock at all
(→ [Tagged-Record Bump
Append](../algorithm/001_tagged_record_bump_append.md)).

**The fix for that lock is never a rewritten global allocator.** A
specialized `#[global_allocator]` built to make this crate's own rare
growth-path lock disappear would run underneath every *other* allocation in
the process too — logging, error paths, any dependency this crate does not
control — trading one crate's already-priced, rare lock for a process-wide
fragmentation and syscall cost paid by code that never asked for it. What
this invariant actually claims is narrower and already sufficient without
one: zero allocator calls on *this crate's own append path*, against
whichever global allocator the binary already has.

**It surfaces at the worst possible time.** The thread-cache fast path is
uncontended, so a single-producer microbenchmark shows nothing and a
single-threaded test suite passes. The lock only bites under exactly the
multi-producer contention this crate was factored out to remove — appearing as
a p99 latency spike under the load profile the mechanism was adopted for, long
after adoption, at the moment the numbers are being relied on. A defect that
is invisible precisely where it is cheap to find and visible only where it is
expensive is worth a standing invariant rather than a code-review habit.

**It invalidates the adoption comparison.** The baseline
[Measured Before Adopted](../non_functional_requirement/001_measured_before_adopted.md)
requires this crate not to regress against the `Vec<u8>`-based mechanism a
prospective consumer already has — which already holds this property,
allocating once via `Vec::with_capacity` and never again. A generalization
that loses it does not merely score worse; it fails the requirement's
structural clause outright, regardless of its measured numbers.

**A dropped-and-rebuilt region breaks the two properties layered on top of
it.** A fresh allocation is not the same pages: it forfeits the page alignment
[POD, Pointer-Free, Page-Aligned
Payloads](../non_functional_requirement/002_pod_pointer_free_payloads.md)
requires unless deliberately re-obtained with the same alignment, and any
consumer holding a byte-slice view or a swapped-out pointer to the previous
region holds a dangling one. Growth *during* a cycle has the same shape in
milder form — a reallocating region moves its bytes — which is why confining
growth to warm-up is a correctness convenience and not only a performance one
(→ [Tagged-Record Bump
Append](../algorithm/001_tagged_record_bump_append.md)'s **Open** section,
where whether growth relocates at all is still undecided).

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_tagged_record_bump_append.md](../algorithm/001_tagged_record_bump_append.md) | The procedure whose Step 1 growth branch this invariant asserts is never taken after warm-up |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_thread_local_append_log.md](../data_structure/001_thread_local_append_log.md) | The region this count is measured over; its `reset` row states the capacity-retention rule this invariant makes exact |

### Invariants

| File | Relationship |
|------|--------------|
| [001_single_writer_append.md](001_single_writer_append.md) | The sibling contract; that one the borrow checker enforces, this one it does not |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_measured_before_adopted.md](../non_functional_requirement/001_measured_before_adopted.md) | Names this property as a structural clause the benchmark cannot trade away for speed |
| [../non_functional_requirement/002_pod_pointer_free_payloads.md](../non_functional_requirement/002_pod_pointer_free_payloads.md) | Why reset needs no `Drop` walk, and what a rebuilt region would cost the page-alignment guarantee |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_implicit_thread_locals_are_hidden_state.md](../pitfall/001_implicit_thread_locals_are_hidden_state.md) | Capacity retention across cycles is by design here — why a thread-keyed region is not cleared at an owner boundary, the trap this pitfall names |

### Sources

| File | Relationship |
|------|--------------|
| `src/lib.rs` | Crate root; `push` refuses past `limit` rather than growing, with a `debug_assert!` pinning `items.capacity()` unchanged, so no allocation occurs after `with_capacity` returns (→ TL30) |

### Tests

| File | Relationship |
|------|--------------|
| `tests/zero_alloc_test.rs` (to create) | Counting global allocator: 0 alloc and 0 dealloc deltas across 1000 steady-state cycles, with region capacity unchanged; separately, exactly one allocation when driven past the high-water mark, with all prior bytes preserved |

### TL30 — This Is the One Pre-Implementation Invariant the Built Crate Satisfies

The instance argues from a region sized once at registration. The crate
reserves once at construction and refuses past `limit`, so no allocation occurs
after `with_capacity` returns.

Same guarantee, different structure — and the reason it survived the rewrite is
that it was stated as a *count* rather than as a property of a bump pointer
(→ [`../non_functional_requirement/001`](../non_functional_requirement/001_measured_before_adopted.md)
on why the counts were chosen that way).

### TL31 — The Reached-Test Names a Counting Allocator This Crate Lacks

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'the criterion the test file quotes:\n'
command grep -m1 -A2 -F '//! reached-test reads: "A `TlsBuffer` accumulates `N` items with zero atomic' ring_tls/tests/tls_test.rs
printf 'global_allocator declarations, whole family: %s\n' \
  "$( grep -rl 'global_allocator' --include='*.rs' ring_*/ | wc -l )"
printf 'what both halves are actually asserted against:\n'
grep 'CountingSeq' ring_tls/tests/tls_test.rs | head -2
```

Live output:

```
the criterion the test file quotes:
//! reached-test reads: "A `TlsBuffer` accumulates `N` items with zero atomic
//! operations (asserted by a counting allocator/atomic shim), and one
//! `flush_into` moves all `N` into the ring as a single contiguous claim."
global_allocator declarations, whole family: 4
what both halves are actually asserted against:
//! Both halves are asserted against `ring_atomic::CountingSeq`, the counting
use ring_atomic::{AtomicSeq, CountingSeq, SeqCell};
```

Both halves of the criterion are asserted against `ring_atomic::CountingSeq` —
the *atomic* shim. The allocator half of "a counting allocator/atomic shim" has
no implementation in this crate, and the test file quotes the phrase verbatim
while satisfying only one side of the slash.

The count above is four, not zero. `ring_barrier`, `ring_claim`, `ring_consume`
and `ring_cursor` each carry a `#[ global_allocator ]` in their own
`tests/allocation_test.rs`. So the allocator half is not unimplementable here —
the instrument exists inside the family, and the crate that states the criterion
is one of the twenty-nine that have not adopted it.

Zero-allocation still holds (→ TL30), by `push` refusing before `Vec` needs to
grow. Nothing observes it: the suite would stay green if TL14's single
comparison stopped being the only insertion path.
