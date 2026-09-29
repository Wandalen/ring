# Lifecycle: The Barrier Holds the Consumer

### Scope

- **Purpose**: Work out the arrangement this crate makes possible and does not itself achieve — the consume point being structural rather than a rule to remember — and state exactly which part is missing and where it would live.
- **Responsibility**: The phases of the arrangement, its transitions, what it depends on above this family, and its cleanup obligations.
- **In Scope**: The `Consumer`'s placement across a tick; what enforcement would require.
- **Out of Scope**: The handle's own arc (→ [Split, Move and Drop](001_split_move_and_drop.md)); the scheduler that would enforce placement, which is outside the 33-crate family entirely.

### Lifecycle Phases

The design requires that "the consumer runs at a defined point — end of stage,
end of tick — and nowhere else," and identifies this crate's contribution
precisely: the split "separates the consuming end into its own handle, so the
barrier **can** hold the only thing capable of draining."

**The word is *can*.** These phases trace what has to happen for that
possibility to become a fact.

| # | Phase | State of the `Consumer` | Determinism |
|---|-------|------------------------|-------------|
| K1 | **Created** | Returned from the split, held by whoever called it | Undetermined — it is a value in a scope |
| K2 | **Placed** | Moved into the barrier / end-of-tick structure | **Established**, if this happens |
| K3 | **Systems run** | Held by the barrier; unreachable from any system | Held. No system has a value that can drain |
| K4 | **Barrier runs** | The barrier drains | Held. Every run sees the same published prefix for the same inputs |
| K5 | **Next tick** | Unchanged — the handle stays put | Held, as long as K2 is not undone |
| K6 | **Misplaced** | Moved into a system, a task, or a background thread | **Lost**, silently and permanently |

**K2 is the load-bearing phase and nothing in this family performs it.** The
split returns a value; where it goes is decided by the code that called the
split. If that code is a scheduler that installs it in the barrier, K2 happens.
If it is a system's constructor, K6 happens instead, and every type-level
guarantee in this crate remains satisfied.

**K3 is where the crate's contribution is actually visible.** During the phase
when systems run, no system holds anything that can drain — not because a rule
forbids it, but because the only value with that capability is elsewhere. That
is the "structural rather than a rule to remember" property, and it is real. It
is a property of K3 given K2, not of the type given nothing.

**K6 produces no error, no warning, and no test failure.** The system drains,
gets a well-formed prefix of whatever had been published at that moment, and
proceeds. Two runs of identical inputs get different prefixes.
This carries the consequence exactly: "replay diverges and a test that passes proves
nothing about the next run."

### Phase Transitions

| # | Transition | Trigger | Enforced by |
|---|-----------|---------|-------------|
| J1 | K1 → K2 | The scheduler takes ownership of the `Consumer` | **Nothing in this family.** A convention at the call site |
| J2 | K2 → K3 | The tick begins | The scheduler's own structure |
| J3 | K3 → K4 | Systems finish; the barrier is reached | The scheduler |
| J4 | K4 → K5 | The drain completes | — |
| J5 | K1 → K6, or K2 → K6 | The handle is moved somewhere else | **Nothing.** A move is a move |
| J6 | K6 → K2 | Someone notices and moves it back | Review, or an incident |

**J1 and J5 are the same operation as far as this crate is concerned** — a
`Send` owned value being moved. There is no signature difference between moving
a `Consumer` into a barrier and moving it into a system, which is precisely why
K6 cannot be prevented here.

**What would actually enforce J1, for the record:**

| Mechanism | Where it would live | Cost |
|-----------|--------------------|------|
| The barrier's constructor is the only public route to a `Consumer` — the split returns it wrapped in a type only the barrier can unwrap | Above this family; requires this crate to know about barriers | Couples a general-purpose ring family to one scheduler design. Rejected for that reason, not because it would not work |
| The scheduler owns all rings and hands systems only `Producer`s | The scheduler | **The realistic answer.** Costs nothing here and moves the guarantee to where placement is already decided |
| A runtime assertion that the draining thread is the barrier thread | Here, with a thread-id field | A field, a comparison per drain, and it fails only after the wrong drain already happened (→ [Two Handles Over One Backend](../data_structure/001_two_handles_over_one_backend.md)'s absent-state table) |

**The second row is where this obligation belongs**, and naming it is this
instance's main purpose: the guarantee is achievable, it is simply not
achievable *here*, and the crate that can achieve it is the one that already
decides which values a system receives.

### Dependencies

| # | Depends on | For |
|---|-----------|-----|
| E1 | A scheduler with a defined barrier point | K2's destination existing at all |
| E2 | [`ring_flush`](../../../ring_flush/readme.md) | The producer side aligning to the same barrier, so publication and consumption meet at one point rather than two — this is `ring_flush`'s contribution |
| E3 | The `Consumer` being non-`Clone` (→ [Consumer](../type/002_consumer.md)) | "The only thing capable of draining" being literally true. With `Clone`, the barrier holds *a* draining handle rather than *the* one |
| E4 | [`ring_spsc`](../../../ring_spsc/docs/readme.md) or an MPSC backend with a single consumer | The cardinality this arrangement assumes |

**E2 is a genuine two-sided requirement and half of it is in another crate.**
A barrier that drains at tick end, fed by producers that publish whenever they
like, still has a defined consume point — determinism holds. But
`ring_flush`'s `OnBarrier` policy
exists so that publication is aligned too, which is what makes the *contents* of
each drain deterministic rather than only its timing. This instance depends on
E2 for the stronger property and not for the weaker one, and the distinction is
easy to blur.

**E3 restates a type rule as a lifecycle dependency deliberately.** The
non-`Clone` property is documented as a type constraint; its *reason* is this
arrangement. Neither location alone explains it — a reader of the type sees a
missing derive, and a reader of this instance sees why it must stay missing.

### Cleanup Requirements

| # | Requirement | Rationale |
|---|-------------|-----------|
| Y1 | **The barrier drops the `Consumer` when the ring is retired, and not before** | An early drop takes the ring's drain capability with it; the producer then fills and refuses forever (→ [Split, Move and Drop](001_split_move_and_drop.md)'s L5) |
| Y2 | **A final drain runs before the handle is dropped** | Items published between the last barrier and shutdown are otherwise dropped silently. `drain_all()`, `ring_shutdown`'s operation, is what exists for that |
| Y3 | **The handle is not moved out of the barrier for shutdown convenience** | A shutdown path that relocates the `Consumer` is K6 arriving at the end of the run, where it is least likely to be noticed |
| Y4 | **Nothing in this crate is required to clean up** | The arrangement is entirely in the holder's structure; this crate has no registration to undo — which is also why it has no hook to enforce anything |

**Y2 is the requirement that gets skipped**, because at shutdown the remaining
items usually do not matter — until the ring carries something that does, like
the last frame's intents in a system that persists them.

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_consumer_surface.md](../api/002_consumer_surface.md) | The drain K4 performs, and why its bound is fixed at call time |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_capability_follows_the_handle.md](../invariant/001_capability_follows_the_handle.md) | The property that makes K3 structural, and its explicit statement that placement is not enforced here |

### Lifecycles

| File | Relationship |
|------|--------------|
| [001_split_move_and_drop.md](001_split_move_and_drop.md) | The handle's own arc; its T4 is this instance's J5 |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_enforce_by_withholding.md](../pattern/001_enforce_by_withholding.md) | Its applicability row three — where the practice enables enforcement without performing it |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_consumer.md](../type/002_consumer.md) | C8, which this instance is the long form of |

### Sources

| File | Relationship |
|------|--------------|
| [../invariant/001_capability_follows_the_handle.md](../invariant/001_capability_follows_the_handle.md) | The requirement, K6's consequence, and this crate's own contribution — "*can* hold the only thing capable of draining"; the split this arrangement is built on |
| [`ring_flush/readme.md`](../../../ring_flush/readme.md) | E2's producer-side alignment |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/handle_test.rs`](../../tests/handle_test.rs) · `draining_at_the_same_point_is_deterministic` | Given a fixed publication sequence, two runs draining at the same point yield identical sequences — K4's determinism, which is the only part of this arrangement this crate can test |
| [`ring_flush/tests/flush_test.rs`](../../../ring_flush/readme.md) — **not yet written** | E2's half — `OnBarrier` fires at exactly its stated trigger and no other point. `ring_flush` had not been implemented yet when this was written; until it is implemented, E2 is claimed by nothing and this row is a debt, not a citation |

### HD30 — E2's Debt Row Names a Test That Exists and Covers Exactly What the Row Asked For

The Tests table's second row calls `ring_flush/tests/flush_test.rs` "**not yet
written**" and E2 "claimed by nothing":

```sh
cd "$(git rev-parse --show-toplevel)"
printf '  the file the row calls not yet written: %s\n' \
  "$( [ -f ring_flush/tests/flush_test.rs ] && echo exists || echo absent )"
printf '  #[ test ] in it:                        %s\n' \
  "$( command grep -c '^#\[ test \]' ring_flush/tests/flush_test.rs )"
echo '  -- test fns whose body names OnBarrier --'
awk '/^fn /{f=$0; shown=0} /OnBarrier/ && f!="" && !shown {print "    " f; shown=1}' \
  ring_flush/tests/flush_test.rs | head -4
```

Live output:

```
  the file the row calls not yet written: exists
  #[ test ] in it:                        32
  -- test fns whose body names OnBarrier --
    fn on_full_ignores_barriers_and_counts()
    fn on_barrier_fires_only_when_a_barrier_is_announced()
    fn on_barrier_never_fires_without_an_announcement()
    fn dropping_a_driver_with_records_staged_publishes_nothing()
```

Thirty-one tests, fifteen of them touching `OnBarrier`, and two of those match
the row's wording almost clause for clause: `on_barrier_fires_only_when_a_barrier_is_announced`
covers "fires at exactly its stated trigger", `on_barrier_never_fires_without_an_announcement`
covers "and no other point."

**Correction (2026-09-28):** this paragraph read "Thirty-one tests".
`ring_flush/tests/flush_test.rs` gained one more test since this finding was
written, unrelated to `OnBarrier` or to the row this finding is about. The
count is thirty-two now; the fifteen touching `OnBarrier` and the two matching
the row's wording almost clause for clause are both unaffected and still hold.

**The debt was paid and the ledger was never updated.**
`ring_flush` was implemented after this instance was written, so the row's own
reasoning — "`ring_flush` had not been implemented yet when this was written;
until it is implemented, E2 is claimed by nothing" — expired as soon as
`ring_flush` shipped its tests.

The shape is worth naming because a debt row is the one kind of documentation
that is *supposed* to become wrong, and nothing reads it back. A missing test
gets found by a gate; a stale claim that a test is missing gets found by
someone re-reading the sentence. `citations.py` checks that a named test exists
in the crate's own `tests/`, which a row saying **not yet written** never
triggers — the row is invisible to the only checker that would notice.
