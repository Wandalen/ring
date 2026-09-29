# Lifecycle: Producer and Consumer Pairing

### Scope

- **Purpose**: Give the two ends their own arc inside the ring's Live phase, and establish that the pairing is where this crate's central invariant is actually made true rather than merely stated.
- **Responsibility**: The phases each end passes through, the transitions, the ordering obligations, and the cleanup each end owes.
- **In Scope**: A `Producer` and a `Consumer` from creation to drop; the thread-affinity question.
- **Out of Scope**: The ring's own allocation and teardown (→ [Ring Construction and Teardown](001_ring_construction_and_teardown.md)); the handle types themselves, which are [`ring_handle`](../../../ring_handle/readme.md)'s.

### Lifecycle Phases

The two ends are created together and die independently. Each passes through
the same four phases.

| # | Phase | Begins | Ends | The end may |
|---|-------|--------|------|-------------|
| Q1 | **Paired** | `Ring::split( &mut self )` returns the pair | The end is moved to its thread | Nothing yet — it has no thread |
| Q2 | **Bound** | The end is moved onto a thread | That thread's last operation | Everything its surface offers |
| Q3 | **Quiescent** | The thread stops operating on it | The end is dropped | Nothing; the other end is unaffected |
| Q4 | **Dropped** | The end goes out of scope | — | Nothing. If it is the second, the ring enters P4 |

**Q1 → Q2 is a *move*, and that is what enforces cardinality.** The end is not
`Clone`, so moving it onto a thread leaves none behind. There is no moment at
which two threads both hold a `Producer`, because there is only ever one
`Producer` value in existence. This crate's own reached-test asserts exactly
this shape — both ends `Send`, able to move the pair to two threads without a
shared mutable reference.

**Cardinality is enforced one level earlier too, and by the same mechanism.**
`split` takes `&mut self`, so the pair borrows the ring exclusively: a second
`split` while the first pair is alive does not compile. That is the
`slice::split_at_mut` pattern, and it costs no `Arc`, no allocation and no
runtime check — the two ends are two `&Ring` reborrows of one exclusive
borrow. `tests/spsc_test.rs`'s `a_second_pair_may_be_split_once_the_first_is
_gone` asserts the permissive half (a second pair *is* obtainable once the
first is dropped), and a `compile_fail` block in the crate's module
documentation asserts the restrictive half.

**This is where [Exactly One Producer, Exactly One Consumer](../invariant/001_exactly_one_producer_one_consumer.md)
stops being a documented precondition and becomes a fact about the program.**
The invariant instance can only state the requirement; the pairing is the
mechanism.

#### The thread-affinity question

**Decided: the second, with its precondition stated rather than hidden.** Both
ends are `Send` and neither is `Sync` — each carries a
`PhantomData< Cell< () > >` to remove the auto-`Sync` it would otherwise get
from holding only a shared reference — so handoff between threads is permitted
by the type system, and `tests/spsc_test.rs`'s
`both_ends_are_send_and_neither_is_sync` asserts exactly that pair of facts.
What the crate does *not* do is supply the happens-before edge such a handoff
needs; that remains the caller's, as spelled out below.

The two answers, and why the more permissive one was chosen:

- **Bound once.** The end is moved to a thread and stays there for its whole
  life. Simplest, and it is what the invariant literally requires.
- **Movable between threads, one at a time.** The end may be handed off, so
  long as no two threads hold it at once — which `Send` without `Sync` already
  expresses in the type system.

The second is strictly more permissive and *is* sound, but only because the
handoff mechanism (a join, a channel send, a mutex release) supplies the
happens-before edge that makes the first thread's writes visible to the second.
**This crate supplies no such edge**, which is exactly the third violation row
in the invariant instance — a sequential handoff with no synchronization is
silent, load-dependent breakage that works on x86-64 far more often than on
AArch64, so it survives local testing and fails in the field. Note that this
workspace's own host is aarch64, so the local run is the *less* forgiving of
the two — which makes a green local suite evidence of something, though still
not of the ordering (→ `tests/manual/readme.md` S9).

So: `Send` and `!Sync` is the right bound and it permits handoff, but the
handoff's correctness rests on machinery outside this crate. Stating the
permission without stating what it rests on would be the dangerous half of the
truth — which is why the module documentation says it too, at the point where
a reader meets `split`, and not only here.

### Phase Transitions

| # | Transition | Trigger | Ordering obligation |
|---|-----------|---------|---------------------|
| M1 | Q1 → Q2 | The end is moved onto a thread | Must happen before that thread's first operation. The move itself supplies the edge |
| M2 | Q2 → Q2 | Publishes, or drains | The whole of [`algorithm/`](../algorithm/readme.md) |
| M3 | Q2 → Q3 | The thread performs its last operation | None |
| M4 | Q3 → Q4 | Drop | If this is the second end, triggers the ring's L4 |
| M5 | Q2 → Q2 (handoff) | The end moves to another thread | **Requires an external happens-before edge.** Unenforced here |

**M5 is the only transition with an obligation this crate cannot check**, and
it is the one worth flagging. M1 is safe because a move onto a fresh thread
carries its own synchronization; M5 is a move between two threads that already
exist, and whether the edge is present depends entirely on the mechanism the
caller used.

#### The abandoned claim

**Unreachable as built** — shape 3 was chosen (→ [Producer Surface](../api/001_producer_surface.md)),
so this section describes the hazard the shape removes rather than one the
crate carries. It is kept because the hazard is what *justifies* the shape, and
because `ring_mpsc` faces the same question with a worse answer available.

Under the three-call producer shape (shape 2), a producer that claims a slot
and then returns — an early exit, a `?`, a panic caught upstream — without
publishing leaves the ring permanently stuck: its cursor never advances, so
the consumer never sees the record and every subsequent publish writes past a
slot nobody will read.

**The damage is narrower here than in the multi-producer ring**, and that is
worth being precise about. In `ring_mpsc` an abandoned claim blocks the
contiguous prefix, so *every other producer's* published records behind the
gap become undrainable — one producer's early return loses the whole tail.
Here the abandoning producer is the only producer, so it wedges only its own
ring. Fatal either way; not equally expensive.

Shape 3 — a guard that publishes on drop — makes the case unreachable, and is
the only remedy that survives a panic. It is what `claim()` returns.
`tests/spsc_test.rs`'s `a_reservation_publishes_on_drop_even_unwritten` pins
the behaviour that makes it work: a `Reservation` dropped without a write still
advances the cursor, publishing whatever the slot already held. That is a
deliberate choice of a stale record over a wedged ring, and it is the direct
consequence of Cleanup 2 below.

### Dependencies

| Depends on | For | Phase |
|-----------|-----|-------|
| [`ring_handle`](../../../ring_handle/readme.md) | The *exported* `Producer`/`Consumer` types — this crate defines its own pair, which `ring_handle` wraps or re-exports | Q1 |
| [`ring_factory`](../../../ring_factory/readme.md) | Choosing this backend over `ring_mpsc`, and rejecting `producer_count != 1` before a ring is built at all | Q1 |
| [`ring_shutdown`](../../../ring_shutdown/readme.md) | Waking a parked counterpart at close, so Q3 is reachable rather than blocked | Q2 → Q3 |

**All three are above this crate in the dependency order.** This crate defines
what the ends must guarantee *and now enforces the pairing itself* — `split`'s
`&mut self` is the enforcement, and it needs nothing from above. What the
crates above add is the rest: choosing between backends, rejecting a
multi-producer config, and waking a parked counterpart. That is the same
inversion the ring's own construction has, and it is what ruling 4's five-crate
export surface costs and buys.

### Cleanup Requirements

1. **Neither end must outlive a borrow into the slots.** A drained batch
   borrowed from the consumer cannot outlive the consumer, and the guard shape
   is what expresses that in the type system rather than in prose.

2. **A producer must not enter Q3 holding an unpublished claim.** Under shape 2
   this is an unenforceable rule; under shape 3 it is unrepresentable, and shape
   3 is what was built. There is no third option that catches it at runtime,
   because "claimed and not yet published" is indistinguishable from "claimed
   and about to be published."

3. **Dropping one end does not invalidate the other.** A consumer whose
   producer is gone can still drain what was published; a producer whose
   consumer is gone can still publish until the ring fills, and then reports
   `Full` forever. Neither case is an error, and neither should panic —
   detecting counterpart death and reporting it as `Closed` is
   [`ring_shutdown`](../../../ring_shutdown/readme.md)'s job, not a
   destructor's.

4. **Drop order between the ends is not specified and both orders must work.**
   Cleanup 3 is what makes that true.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_uncontended_claim_and_publish.md](../algorithm/001_uncontended_claim_and_publish.md) | The M2 operation whose shape 2 creates the abandoned-claim case |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_producer_surface.md](../api/001_producer_surface.md) | The three shapes, and the abandonment hazard shape 3 removes |
| [../api/002_consumer_surface.md](../api/002_consumer_surface.md) | Cleanup 1's borrow lifetime |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_family_dependency_seam.md](../integration/001_family_dependency_seam.md) | Why all three Dependencies sit above this crate |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_exactly_one_producer_one_consumer.md](../invariant/001_exactly_one_producer_one_consumer.md) | The invariant Q1 → Q2's move makes true, and whose third violation row is M5 |

### Lifecycles

| File | Relationship |
|------|--------------|
| [001_ring_construction_and_teardown.md](001_ring_construction_and_teardown.md) | The outer phase this cycle nests inside; its L4 is M4's consequence |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_spsc_correctness_does_not_transfer.md](../pitfall/001_spsc_correctness_does_not_transfer.md) | Why the abandoned claim's narrower blast radius here does not carry to MPSC |

### Sources

| File | Relationship |
|------|--------------|
| [`bench_harness/docs/acceptance/001_feature_reached_tests.md`](../../../bench_harness/docs/acceptance/001_feature_reached_tests.md) | This crate's `trybuild` compile-fail cases — Q1 → Q2's enforcement, made binary |

### Tests

| File | Relationship |
|------|--------------|
| `tests/spsc_test.rs` | `one_producer_and_one_consumer_exchange_one_hundred_thousand_items` — the pair moves to two threads and exchanges 100 000 items with no shared mutable reference. This crate's own positive half, at this crate's pairing |
| `tests/spsc_test.rs` | `both_ends_are_send_and_neither_is_sync` — the bound the thread-affinity answer rests on, asserted as a trait bound rather than inferred from a passing run |
| `tests/spsc_test.rs` | `a_departed_producer_leaves_the_published_tail_drainable` and `a_departed_consumer_leaves_the_producer_reporting_full_forever` — Cleanup 3 and 4, both drop orders, neither panicking |
| `tests/spsc_test.rs` | `a_second_pair_may_be_split_once_the_first_is_gone` — Q1 is re-enterable after Q4, which is what makes `&mut self` a cardinality bound rather than a one-shot |
| `tests/spsc_test.rs` | `a_reservation_publishes_on_drop_even_unwritten` — the abandoned claim made unreachable, Cleanup 2 |
| `src/lib.rs` | The `compile_fail` blocks: a second `split` while the first pair lives, and a drained record outliving its commit (Cleanup 1) |

### SP33 — Re-Splitting Is Legal and the Restriction Is Lifetime-Scoped

A design that took `self` would make the pairing permanent and would be simpler
to argue about. Taking `&mut self` means the cardinality invariant is enforced
per-lifetime rather than per-ring, and the cursors survive the re-split — a
second pair starts where the first stopped, not at zero.

That continuity is the useful part and is not obvious from the signature.
