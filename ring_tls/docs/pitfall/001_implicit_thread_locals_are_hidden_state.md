# Pitfall: Implicit Thread-Locals Are Hidden Global State

### Scope

- **Purpose**: Intercept the reading of "per-thread" that reaches for `thread_local!`, before a hidden singleton makes multiple independent instances in one process structurally impossible.
- **Responsibility**: State the trap, the four failure shapes it produces, and the owned-handle mitigation plus the two corollaries that follow from it.
- **In Scope**: Where a region's storage lives and who names it — thread-keyed static versus owner-held handle.
- **Out of Scope**: How many writers may touch one region at a time, which is a separate contract holding under either storage choice (→ [Single-Writer Append](../invariant/001_single_writer_append.md)); what the appended bytes may contain (→ [POD, Pointer-Free, Page-Aligned Payloads](../non_functional_requirement/002_pod_pointer_free_payloads.md)).

### Trap

The crate's own one-line description says *per-thread*, and Rust has a
first-class construct spelled exactly that:

```rust
// The shape that looks obviously right and is not:
thread_local! {
  static LOG : RefCell< BumpLog > = RefCell::new( BumpLog::new( 64 * 1024 ) );
}
```

Everything about it is inviting. There is no handle to thread through call
stacks, no lifetime parameter to propagate, no `&mut self` receiver in every
signature that touches the log. Deeply nested code reaches the buffer without
any intermediate function needing to know it exists. It satisfies the phrase
"per-thread" literally, and on a fixed-size pool with one owner it even
behaves correctly.

It is also the hidden-singleton shape: "global singletons, `static`
variables, **and thread-local stashes**" belong together as a single
prohibition, because all three are reachable mutable paths that no signature
mentions. `thread_local!` reads as the local, scoped one of the three.
Structurally it is not: it is a process-wide static keyed by thread id, and
thread id is not a name any owner in this system chose.

### Failure

**Two independent instances silently share one region.** Two owners running on
the same worker thread — two instances, two test cases, two units of work
scheduled onto the same pool thread — both resolve `LOG` to the *same* region.
Their records interleave into one byte stream with no tag distinguishing whose
is whose, and the consolidator attributes all of them to whichever owner it
happens to drain for. Note what this is *not*: it is not a data race. One
thread is appending, so [Single-Writer
Append](../invariant/001_single_writer_append.md) technically still holds, the
borrow checker is satisfied, and a thread sanitizer reports nothing. It is a
correctness defect wearing the shape of a correct program — strictly worse to
diagnose than the data race the discipline was designed to make
unrepresentable.

**State outlives the owner, in both directions.** Thread-local storage is
keyed by thread, not by owner, and a work-stealing pool hands a thread to a
different owner at will. An owner that finishes leaves its un-consolidated
bytes in the region, and the region's capacity is retained across cycles *by
design* (→ [Zero Allocations in Steady
State](../invariant/002_zero_allocations_in_steady_state.md)), so nothing
clears at the boundary a reader would assume clears it — the next owner
inherits a partially-filled buffer. The mirror case is worse: an owner
migrated to a fresh thread mid-cycle finds an empty region and has silently
lost everything it appended before the migration. Neither direction produces
an error; both produce plausible output.

**The stated scale target becomes unreachable.** This crate's own worked
figure is 50 independent instances coexisting in one process. Under
thread-keyed storage the number of independent regions is bounded by *thread
count*, not owner count — on a 16-thread pool that is 16 regions serving 50
owners, a 3:1 aliasing the code never mentions and no type expresses. The
target is not merely missed; it is not expressible in the design.

**It is invisible until the second instance appears.** Every test with one
instance passes. Every single-threaded benchmark passes. Surfacing the defect
requires two owners *and* thread reuse between them *and* a consolidation
boundary in between — which is the configuration that first materializes in
production-shaped concurrent load, not in a test suite, and long after the
storage choice has been built on. A defect cheap to prevent and expensive to
find is exactly what warrants a standing pitfall entry rather than a review
habit.

### Mitigation

**Hand out owned, `Send` handles; never reach into thread-local storage.** The
log is per-thread by *discipline* — one owner appends at a time, per
[Single-Writer Append](../invariant/001_single_writer_append.md) — not by
*storage location*. The `&mut self` receiver is the enforcement, and its cost
is exactly the thing that made TLS attractive: ownership becomes visible in
every signature that touches the log instead of hiding in a static. That
visibility is the feature.

**`Send` yes, `Sync` deliberately no.** `Send` is required unconditionally: a
scheduler may place an owner on any worker thread, so a handle that cannot
cross threads reintroduces thread-keyed coupling from the opposite direction —
pinning owners to threads instead of aliasing threads across owners. `Sync` is
deliberately *not* required and should not be added: a shared-reference append
means two concurrent appenders, which [Single-Writer
Append](../invariant/001_single_writer_append.md) forbids outright. The pair
`Send + !Sync` is the type-level statement of "movable, not shareable," and it
is worth asserting rather than leaving to inference.

**Corollary 1 — the crate must never spawn its own threads.** It accepts a
caller-supplied executor, or more precisely for a true leaf with an empty
`[dependencies]`, it touches no scheduler at all and knows nothing about
threading beyond the ownership rules in its signatures. A crate that spawns is
a crate that decides thread identity, which is the same hidden-global mistake
one level up: the caller can no longer choose the mapping from owners to
threads, so it can no longer guarantee the isolation it was promised.

**Corollary 2 — a handoff must be a pointer swap, not a copy.** The
consolidation shape this crate is factored out to serve swaps each region's
pointer out zero-copy rather than draining it byte by byte. A design that
reaches into thread-local storage to read another thread's buffer *cannot* do
this — it holds no owned handle to swap, only a scoped borrow inside a
`with()` closure, so its only available move is to copy the bytes out. The TLS
shape therefore does not merely risk correctness; it forecloses the
performance property, and it does so silently, because a copy is a working
implementation (→ [POD, Pointer-Free, Page-Aligned
Payloads](../non_functional_requirement/002_pod_pointer_free_payloads.md), and
[`ring_mpsc`'s Batch Drain by Cursor
Swap](../../../ring_mpsc/docs/algorithm/002_batch_drain_by_cursor_swap.md) for
the same swap-not-copy shape on the merge side).

**The honest gap.** This crate can make the correct thing ergonomic and the
incorrect thing explicit; it cannot make the incorrect thing unrepresentable.
Nothing stops a *consumer* from putting a perfectly well-designed owned handle
into its own `thread_local!` and reproducing every failure above one level
up. Composition is type-enforced, reachability is not — and that is why this
is filed as a pitfall rather than claimed as an invariant.

**Checks, both sides.** The storage side, runnable against this crate today:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_tls
command grep -rnE 'thread_local!|^\s*static\s' src/ --include='*.rs' || true
```

Live output:

```
```

The pattern deliberately catches `static` as well as `thread_local!`, since
both are process-wide addresses; `const` does not match and is the correct
choice for a genuine compile-time constant, because a `const` is inlined at
each use site and has no single memory location to share. The handle side, as
a compile-time assertion rather than a convention:

```rust
// tests/handle_traits_test.rs (to create)
const fn assert_send< T : Send >() {}
const _ : () = assert_send::< BumpLog >();
// plus a compile-fail case asserting BumpLog is NOT Sync
```

Asserting the negative half matters as much as the positive one: a future
change that adds interior synchronization to make the type `Sync` would
compile, pass every existing test, and quietly permit the two-appender case
the whole discipline exists to exclude.

### Algorithms

| File | Relationship |
|------|--------------|
| [../../../ring_mpsc/docs/algorithm/002_batch_drain_by_cursor_swap.md](../../../ring_mpsc/docs/algorithm/002_batch_drain_by_cursor_swap.md) | The swap-not-copy handoff shape Corollary 2 preserves and a thread-local stash forecloses |
| [../algorithm/001_tagged_record_bump_append.md](../algorithm/001_tagged_record_bump_append.md) | Where the `&mut self` receiver this pitfall argues for actually appears |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_thread_local_append_log.md](../data_structure/001_thread_local_append_log.md) | The structure whose name contains the word that sets this trap; its per-thread identity is ownership, not storage location |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_single_writer_append.md](../invariant/001_single_writer_append.md) | The discipline that makes the log per-thread; this pitfall's whole point is that it is satisfied by ownership, not by storage location |
| [../invariant/002_zero_allocations_in_steady_state.md](../invariant/002_zero_allocations_in_steady_state.md) | Capacity retention across cycles is by design — which is why a thread-keyed region is not cleared at an owner boundary |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_pod_pointer_free_payloads.md](../non_functional_requirement/002_pod_pointer_free_payloads.md) | The zero-copy handoff Corollary 2 preserves is exactly what that requirement's POD constraint exists to enable |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../../../ring_mpsc/docs/pitfall/001_spinning_consumer_owns_a_core.md](../../../ring_mpsc/docs/pitfall/001_spinning_consumer_owns_a_core.md) | The merge half's own trap — same category: a mechanism that quietly claims a process-wide resource no signature mentions |

### Sources

| File | Relationship |
|------|--------------|
| `src/lib.rs` | Crate root; skeleton — no handle type or storage choice expressed yet, which is why this is filed before the choice is made |

### Tests

| File | Relationship |
|------|--------------|
| `tests/handle_traits_test.rs` (to create) | `BumpLog: Send` asserted at compile time; a compile-fail case asserting it is not `Sync` |
| `tests/multi_instance_test.rs` (to create) | Two instances driven from one thread keep byte-disjoint regions; an instance moved between threads mid-cycle retains every byte appended before the move |

### TL45 — The Pitfall Was Avoided by Building No Thread-Local at All

The buffer is an ordinary owned value the caller places. `ring_flush` holds
one as a struct field, `ring_bench` and `ring_testkit` as locals.

So the crate named `ring_tls` provides no thread-local mechanism, and this
instance is the reason: it is the one pre-implementation argument the
implementation followed.
