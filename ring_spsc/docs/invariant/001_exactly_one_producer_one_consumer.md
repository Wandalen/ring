# Invariant: Exactly One Producer, Exactly One Consumer

### Scope

- **Purpose**: State the precondition every cost saving in this crate is purchased with, and establish that it cannot be checked at runtime — only made unrepresentable.
- **Responsibility**: The statement, where enforcement can and cannot live, and what a violation actually does.
- **In Scope**: The cardinality constraint on each end; the enforcement points; the failure mode.
- **Out of Scope**: The multi-producer relaxation, which is a different crate (→ [`ring_mpsc`](../../../ring_mpsc/docs/readme.md)); the handle types that enforce it, which are [`ring_handle`](../../../ring_handle/readme.md)'s.

### Invariant Statement

**At most one thread ever calls a producer operation on a given ring, and at
most one thread ever calls a consumer operation on it, for the entire lifetime
of that ring.**

Not "one at a time" — *one*, for the ring's whole life. A handoff between two
threads that never overlap still violates the invariant as stated, because the
plain cursor loads carry no synchronization that would make the first thread's
writes visible to the second. A correct handoff needs an explicit
happens-before edge (a join, a channel send, a mutex) that this crate does not
provide and does not know about.

**Everything this crate claims rests on it:**

| Claim | Depends on |
|-------|-----------|
| Plain (non-atomic) load of one's own cursor | That cursor having exactly one writer |
| No `fetch_add` on the producer cursor | One producer |
| No gating set, no minimum computation | One consumer |
| No per-slot publication stamp | One producer, so publication cannot be overtaken |
| `free_capacity()` being binding rather than advisory | One producer, so nothing consumes the reported space |

Remove the invariant and every row becomes false at once. This is not a ring
that degrades under a second producer — it is a different data structure, and
that one is `ring_mpsc`.

### Enforcement Mechanism

**Nothing in this crate can enforce it, and that has to be said plainly.**
There is no runtime check that distinguishes "the producer thread" from "a
second producer thread": both call the same method on the same value, and any
check capable of telling them apart (a stored `ThreadId` compared on every
call) would put a load and a branch on the hot path in order to catch a
structural error that is the same on every call — paying per record for a
per-program mistake.

Enforcement is therefore structural, and lives above this crate:

| Point | Mechanism | Owner |
|-------|-----------|-------|
| Handle split | `Producer` exposes no drain method; `Consumer` no publish method | [`ring_handle`](../../../ring_handle/readme.md) |
| Non-duplicable ends | Each end is not `Clone`, so a second producer cannot be conjured from the first | `ring_handle` |
| Move semantics | Both ends are `Send`; moving a `Producer` to a thread leaves none behind | `ring_handle` |
| Construction | Exactly one `( Producer, Consumer )` pair is returned per ring | [`ring_factory`](../../../ring_factory/readme.md) |

This crate's own `trybuild` compile-fail cases assert the first three
mechanically — one per direction, plus a `Send` assertion and a two-thread
move without a shared mutable reference. **That is the invariant's real
enforcement, and it is a compile-time one.**

**This crate's own contribution is to state the requirement so that the crates
above know what they are enforcing.** A `ring_handle` implemented without
knowing this invariant would have no reason to withhold `Clone`.

### Violation Consequences

| Violation | Immediate effect | How it surfaces |
|-----------|------------------|-----------------|
| Two producers | Both read the same producer cursor value, both write the same slot, both advance the cursor by one | **Silent.** One record is lost and one slot holds a torn mixture of two payloads. The counts still look right |
| Two consumers | Both read the same range, both advance the consumer cursor past it | **Silent.** Every record is processed twice, and the cursor over-advances, freeing slots the producer then overwrites |
| Sequential handoff without a happens-before edge | The second thread may observe a stale cursor | **Silent, and load-dependent.** Works on x86-64 far more often than on AArch64, so it passes local testing and fails in the field |

**All three are silent, and the third is worse than the first two** because it
is the one that passes tests. Two simultaneous producers corrupt data quickly
and visibly under load; a handoff between non-overlapping threads produces
correct results almost always, on the machine it was developed on.

**No consequence is a panic, an error return, or a debug assertion in this
crate.** [`ring_debug`](../../../ring_debug/readme.md) owns runtime invariant
checks over a live ring and is the only place a violation could be caught
after the fact — and even there, only some violations leave a detectable
cursor state.

**The "two producers" row understates its own range, and one path used to
break the claim above it.** `tests/spsc_test.rs`'s
`exhaustive::free_capacity_degrades_safely_even_when_a_precondition_violation_reaches_d2`
found, by exhaustive `loom` search rather than by reasoning, an interleaving
worse than "both advance the cursor by one": `Producer::claim`'s `is_full`
check and its own subsequent re-read of the producer cursor are two
independent, unfenced loads, so one racer's re-read can observe a *second*
racer's already-landed publish and compute its own `seq` from that newer
value. Both racers' publishes then land, and the cursor advances by **two**,
not one — for a capacity-1 ring, straight past capacity, the D2 state
`ring_debug`'s `docs/invariant/002` names. Until `Producer::free_capacity` was
hardened to `saturating_sub`, this specific path made the paragraph above
false: `capacity - occupancy` underflowed and panicked in a dev build. It is
true now because that one call site no longer can, not because the violation
became unreachable — it did not, and the loom test above keeps proving it
reachable on every run rather than letting the claim go stale again.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_uncontended_claim_and_publish.md](../algorithm/001_uncontended_claim_and_publish.md) | Its step 1 plain load is unsound without this invariant |
| [../algorithm/002_single_consumer_drain.md](../algorithm/002_single_consumer_drain.md) | Its step 1 and step 5 likewise |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_producer_surface.md](../api/001_producer_surface.md) | Its Error Handling table's second row is this violation |
| [../api/002_consumer_surface.md](../api/002_consumer_surface.md) | Same, for the consumer end |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_two_cursor_ring.md](../data_structure/001_two_cursor_ring.md) | The single-writer-per-cursor property this invariant guarantees |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_reached_through_the_export_surface.md](../integration/002_reached_through_the_export_surface.md) | Why enforcement lands on `ring_handle` rather than here |

### Invariants

| File | Relationship |
|------|--------------|
| [002_no_lock_in_the_path.md](002_no_lock_in_the_path.md) | The cost claim this invariant pays for |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_producer_consumer_pairing.md](../lifecycle/002_producer_consumer_pairing.md) | Where the pairing is established and where a handoff would have to synchronize |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_spsc_correctness_does_not_transfer.md](../pitfall/001_spsc_correctness_does_not_transfer.md) | The reverse hazard — code proven correct here relied on this invariant silently |

### Sources

| File | Relationship |
|------|--------------|
| [`bench_harness/docs/acceptance/001_feature_reached_tests.md`](../../../bench_harness/docs/acceptance/001_feature_reached_tests.md) | This crate's `trybuild` compile-fail assertions — the enforcement, made binary |

### Tests

| File | Relationship |
|------|--------------|
| `tests/spsc_test.rs` | `src/lib.rs`'s `compile_fail` blocks — non-`Clone`, non-`Sync`, and no second `split` while the first pair lives. In the library rather than here, because rustdoc collects doc tests from the library target only |
| `tests/spsc_test.rs` | `one_producer_and_one_consumer_exchange_one_hundred_thousand_items` and `both_ends_are_send_and_neither_is_sync` — this invariant's positive half, and the bound it rests on stated as a trait bound rather than inferred from a passing run |
| `tests/spsc_test.rs` | `exhaustive::free_capacity_degrades_safely_even_when_a_precondition_violation_reaches_d2` — the negative half this page's own text used to understate: exhaustive `loom` proof that violating this invariant reaches D2 through nothing but this crate's own API, and that `free_capacity` degrades safely once it does |

### SP22 — The Cardinality Invariant Is Enforced by `&mut` and by Compile-Fail Tests

`a_second_pair_may_be_split_once_the_first_is_gone` establishes the positive
half: the restriction is lifetime-scoped, not once-per-ring.

The negatives — no second live split, no `Clone` on either end, no `Sync` on
either end, no drained record outliving its commit — are all things that must
*fail to compile*, so the five `compile_fail` doc tests are the only mechanism
available. **A crate whose central invariant is a negative needs a test form
that most crates never use.**

### SP23 — Nothing Prevents the Two Ends Living on the Same Thread

`split` hands back two values with no thread affinity. A caller may keep both on
one thread and alternate — which is sound, since the disjointness argument is
about slot ranges rather than about threads.

Recorded because the `unsafe impl Sync`'s SAFETY comment and the module
documentation both reason in terms of "the producer's thread" and "the
consumer's thread", which is the common case rather than the contract. Nothing
in the suite covers the single-threaded pairing, and nothing needs to — but a
future change that *did* assume two threads would find no test in its way.
