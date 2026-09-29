# Invariant: No Lock in the Path

### Scope

- **Purpose**: State the cost floor this crate is measured against, and pin down what "no lock" is actually asserting — which is stronger than the absence of a `Mutex` and weaker than the absence of all atomics.
- **Responsibility**: The statement, its precise boundary, how it is enforced and checked, and what its violation costs.
- **In Scope**: The synchronization budget of publish and drain; the progress guarantee; the measurable assertion.
- **Out of Scope**: Wait strategies invoked *after* a full or empty result, which are [`ring_wait`](../../../ring_wait/readme.md)'s; the payload's own cost.

### Invariant Statement

**No publish and no drain acquires a lock, blocks, or executes a
read-modify-write. Both are wait-free: bounded steps, independent of any other
thread's progress.**

Three claims of decreasing obviousness, and the third is the one that carries
the crate:

1. **No lock.** No `Mutex`, `RwLock`, or futex in either path. This is the
   claim this crate's own reached-test names — "no lock in the path."
2. **No blocking.** Neither path parks, sleeps, or spins waiting on the other
   thread. A full ring returns `Full`; an empty ring returns an empty batch.
   Whether to then wait is the *caller's* decision, delegated to `ring_wait`,
   and `WaitKind::None` declines to wait at all.
3. **No read-modify-write.** Not one `fetch_add`, `compare_exchange`, or
   `swap` in either path. Only plain loads, acquire loads, and release stores.

**Claim 3 is not implied by claims 1 and 2, and it is where this crate differs
from its sibling.** `ring_mpsc` satisfies 1 and 2 — it is lock-free and never
blocks — while performing a `fetch_add` per claim under contention from every
producer. That RMW takes exclusive ownership of the cursor's cache line, so N
producers serialize on line transfers even though none of them ever waits. A
crate can be lock-free and still contended; this one is neither.

**Wait-free, not merely lock-free.** Lock-free guarantees *someone* makes
progress; wait-free guarantees *this thread* completes in bounded steps. With
one producer there is no CAS to retry, so every operation here has a fixed
instruction count — the stronger guarantee, and it comes free from the
cardinality invariant rather than from extra machinery
(→ [Exactly One Producer, Exactly One Consumer](001_exactly_one_producer_one_consumer.md)).

### Enforcement Mechanism

| Level | Mechanism | Catches |
|-------|-----------|---------|
| Structural | The dependency list contains no synchronization crate, and `ring_gating`/`ring_barrier` are deliberately absent (→ [Family Dependency Seam](../integration/001_family_dependency_seam.md)) | A lock arriving as a transitive dependency |
| Type | The cursors are `PaddedCursor` values whose only operations are load and store | An RMW added by hand |
| Test | A counting ordering shim records every atomic operation performed across a full run and asserts zero RMWs | Regression, which is the case the other two miss |
| Bounded time | The reached-test's run completes in bounded time on a full ring rather than hanging | An accidental spin loop |

**The third row is the only one that survives refactoring**, and it is the one
that matters. Structure and types constrain what is easy to write; only a
measurement catches a `fetch_update` someone adds for a good local reason two
years from now. `ring_tls` uses the same technique for its own zero-atomic
claim — asserted by a counting allocator/atomic shim — so the shim is family
machinery rather than a per-crate invention:
[`ring_testkit`](../../../ring_testkit/readme.md) is its natural home.

**What is deliberately *not* enforced:** the absence of a lock in the
*caller's* code. A consumer that wraps this ring in a `Mutex` has not violated
this invariant; it has discarded the property the invariant exists to provide.
That distinction is why the measurement is stated over this crate's own path
rather than over the program.

### Violation Consequences

| Violation | Effect | Detection |
|-----------|--------|-----------|
| A lock enters the path | The family's correctness floor becomes slower than the mutex-guarded-queue candidate it is benchmarked against (`ring_bench`) | The benchmark, and the counting shim |
| An RMW enters the path | Wait-freedom degrades to lock-freedom; cost becomes contention-dependent, so the SPSC/MPSC comparison stops isolating the variable it exists to isolate | The counting shim only — timing alone will not show it at one producer |
| A blocking wait enters the path | The family's try-only requirement is violated for everything downstream, and a tick can stall on a full ring | The bounded-time test |

**The second row is the quiet one and it damages the whole family rather
than this crate.** The benchmark's purpose is to attribute cost to producer
cardinality. If the SPSC path also performs an RMW, the SPSC and MPSC numbers
converge, the comparison shows nothing, and the family's own benchmark
verdict is taken on a measurement that no longer measures the difference it
was set up to measure. At one producer there is no contention, so the RMW
costs almost nothing and the timing looks fine — the corruption is in the
experiment, not the numbers.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_uncontended_claim_and_publish.md](../algorithm/001_uncontended_claim_and_publish.md) | The publish path this invariant bounds; its step 6 is the only synchronizing operation |
| [../algorithm/002_single_consumer_drain.md](../algorithm/002_single_consumer_drain.md) | The drain path; one release store per batch |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_producer_surface.md](../api/001_producer_surface.md) | Its Operations table's wait-free column is this invariant per operation |
| [../api/002_consumer_surface.md](../api/002_consumer_surface.md) | Its "blocks?" column likewise |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_two_cursor_ring.md](../data_structure/001_two_cursor_ring.md) | Its Operations table's synchronization column |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_family_dependency_seam.md](../integration/001_family_dependency_seam.md) | The structural enforcement row — what the dependency list does and does not admit |

### Invariants

| File | Relationship |
|------|--------------|
| [001_exactly_one_producer_one_consumer.md](001_exactly_one_producer_one_consumer.md) | The precondition wait-freedom is bought with |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_correctness_floor_for_the_family.md](../non_functional_requirement/001_correctness_floor_for_the_family.md) | Why an RMW here corrupts the family's comparison, not just this crate's numbers |
| [../non_functional_requirement/002_byte_parity_over_one_hundred_thousand.md](../non_functional_requirement/002_byte_parity_over_one_hundred_thousand.md) | The reached-test carrying the "no lock in the path" clause |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_spsc_correctness_does_not_transfer.md](../pitfall/001_spsc_correctness_does_not_transfer.md) | Why the wait-free guarantee specifically does not survive the move to MPSC |

### Sources

| File | Relationship |
|------|--------------|
| [`bench_harness/docs/acceptance/001_feature_reached_tests.md`](../../../bench_harness/docs/acceptance/001_feature_reached_tests.md) | `ring_tls`'s counting-shim technique, reused here for claim 3 |

### Tests

| File | Relationship |
|------|--------------|
| `tests/spsc_test.rs` | `tests/manual/readme.md` S2 — claim 3, established over the whole crate rather than over one run: no `fetch_`, no `compare_exchange`, and exactly two `.store(` sites. A counting shim would need `CursorPair` generic over its cell type, and would still only report what a single execution did |
| `tests/spsc_test.rs` | `a_full_ring_reports_rather_than_blocks` and `draining_an_empty_ring_yields_an_empty_batch_and_moves_nothing` — claim 2. Neither returns *within* a bounded time so much as returns at all without waiting: the full ring yields `Err( RingError::Full )` and the empty drain an empty `Batch`, with no park and no spin anywhere in either path |

### SP24 — The Ordering Check Is a Value Assertion Plus a Loom Model

```sh
cd "$(git rev-parse --show-toplevel)"/ring_spsc
printf 'exhaustive module gate:  '; grep -m1 -n 'cfg( loom )' tests/spsc_test.rs
printf 'ordinary body gate:      '; grep -m1 -n 'cfg( not( loom ) )' tests/spsc_test.rs
printf 'loom in the manifest:    '; grep -c 'loom' Cargo.toml
```

Live output:

```
exhaustive module gate:  45://! simply narrow enough that sampling it 100 000 times does not open it. The `#[ cfg( loom ) ] mod exhaustive` at the bottom
ordinary body gate:      62:#[ cfg( not( loom ) ) ]
loom in the manifest:    3
```

Same structure as the sibling, and the same consequence: an ordinary
`cargo test` checks that the constants hold the intended values, not that they
are used in the intended places.

**The host is `aarch64-unknown-linux-gnu` and therefore weakly ordered**, so the
behaviour is observable here — `ring_mpsc`'s `invariant/002` records a mutation
run on this same machine where weakening its publish ordering failed 14 of 60
runs. No equivalent mutation run is recorded for this crate.

### SP25 — A Third Ordering Is in Use and Is Not This Crate's

```sh
cd "$(git rev-parse --show-toplevel)"/ring_spsc
printf 'GATING loads here:    '; grep -vE '^\s*(//|///|//!)' src/lib.rs | grep -c 'GATING'
printf 'declared here:        '; grep -cE '^pub const GATING' src/lib.rs
printf 'declared upstream:    '; grep -cE 'pub const GATING' ../ring_cursor/src/lib.rs
```

Live output:

```
GATING loads here:    8
declared here:        0
declared upstream:    1
```

So the invariant's happens-before argument depends on three orderings, two of
which are asserted in this crate's tests and one of which is not asserted here at
all. Changing `GATING` in `ring_cursor` would silently change this crate's
publication edge.

The check exists — one crate down, in `ring_cursor`'s own suite. Recorded
because the argument is stated here and the guard is elsewhere.
