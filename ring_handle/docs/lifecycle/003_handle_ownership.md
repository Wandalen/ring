# Lifecycle: Handle Ownership

### Scope

- **Purpose**: Enumerate the ownership states a ring passes through as it becomes a handle pair and back to nothing, and mark which transitions the type system permits, forbids, and cannot see.
- **Responsibility**: The states, the transitions, and the invariants each state holds.
- **In Scope**: Ownership and reachability of the ring across the pair's life.
- **Out of Scope**: Whether the ring is open or closed, an orthogonal axis (→ [Ring Liveness Through a Handle](004_ring_liveness_through_a_handle.md)); the drain's own states, which are the backends'.

### States

| # | State | Reachable capabilities | Holder |
|---|-------|------------------------|--------|
| H0 | **Unsplit** | Publish **and** drain, through one value | Whoever built the ring |
| H1 | **Split, co-located** | Publish and drain, through two values in one scope | One scope |
| H2 | **Split, separated** | Publish here, drain there | Two scopes, typically two threads |
| H3 | **Producer only** | Publish. Drain is unreachable — permanently | One scope |
| H4 | **Consumer only** | Drain. Publish is unreachable — permanently | One scope |
| H5 | **Neither** | None. The ring is dropped | — |

**H0 is a state the design wants to be transient**, and it is the only state in
which the capability partition does not exist. A `Factory::build` that returns
the pair directly makes H0 exist only inside the factory
(→ [Split, Move and Drop](../lifecycle/001_split_move_and_drop.md)'s L1).

**H1 and H2 differ in no observable property of this crate.** Both have two
handles with disjoint capabilities; the difference is where they are, which is
not something a type records. The distinction is drawn because H2 is the state
the crate exists to make safe, and because the transition into it is the one
[Send Without Sync](../non_functional_requirement/002_send_without_sync.md)
measures.

**H3 and H4 are asymmetric in consequence, not in structure.** In H3 a producer
publishes into a ring nobody will drain — it fills, then refuses forever. In H4
a consumer drains a ring nobody will fill — it empties, then returns `None`
forever, which is a benign terminal state. The structural mirror image is not a
behavioural one, and H3's refusal shape is the open question named in
[Split, Move and Drop](../lifecycle/001_split_move_and_drop.md)'s L5.

### Transitions

| # | From | To | Trigger | Permitted by |
|---|------|----|---------|--------------|
| M1 | H0 | H1 | The split consumes the ring | The API. **Irreversible** — there is no unsplit |
| M2 | H1 | H2 | A handle is moved to another scope or thread | `Send`; requires no support |
| M3 | H2 | H1 | Both handles return to one scope | Ordinary moves. Legal, and unusual |
| M4 | H1 or H2 | H3 | The `Consumer` drops | `Drop` |
| M5 | H1 or H2 | H4 | The `Producer` drops | `Drop` |
| M6 | H3 or H4 | H5 | The remaining handle drops | `Drop`; the ring drops with it under the `Arc` shape |
| M7 | H1 | H0 | — | **Forbidden.** No operation rejoins the pair |
| M8 | H3 | H1 | — | **Forbidden.** A dropped capability does not come back |
| M9 | any | H1 by cloning | — | **Forbidden**, and this is the one forbidden transition with no compile-time detector: one `#[derive(Clone)]` and M9 becomes legal |

**M7's absence is a design choice worth stating rather than an oversight.** A
`fn rejoin( p: Producer<T>, c: Consumer<T> ) -> Ring<T>` is implementable and is
not offered: it would recreate H0, in which the capability partition does not
hold, at an arbitrary point in the program. The cost of not having it is that a
ring cannot be reconfigured in place — which is what `reset()`, `ring_shutdown`'s
operation, is for, and it operates without leaving H1.

**M9 is the state machine's real vulnerability.** Every other forbidden
transition is forbidden by the absence of an operation, which a compile-fail
case can pin. M9 is forbidden by the absence of a *derive*, and no case in the
acceptance criterion covers it
(→ [A Convenience Method Undoes the Crate](../pitfall/001_a_convenience_method_undoes_the_crate.md)'s F1).

**M3 is legal and worth a note.** Moving both handles back into one scope is
sound — nothing about H1 is unsafe — but a program that does it is usually one
that wanted a single-threaded queue and got a concurrency primitive. It is not
an error; it is a signal.

### Behavioral Invariants

1. **The capability partition holds in every state from H1 onward.** No state
   after M1 has one value that can both publish and drain, and there is no
   transition back to one.

2. **The union of reachable capabilities never grows.** H1 has both; H3 and H4
   have one; H5 has none. Every transition either preserves or shrinks the set —
   which is what makes M9 the violation it is, since cloning grows the number of
   *holders* while the type-level set stays the same.

3. **No state is distinguishable at runtime from within a handle.** A
   `Producer` in H1, H2 and H3 is byte-identical. Behaviour differs (in H3 the
   ring fills and stays full) but nothing in the handle records the state — which
   is why L5's refusal shape has to come from a flag the `Consumer`'s drop sets,
   not from the handle's own knowledge.

4. **H2 is the only state where both capabilities are exercised concurrently**,
   and it is therefore the only state where the ring's own memory ordering is
   under test. A test that never leaves H1 exercises the API and none of the
   concurrency (→ [Send Without Sync](../non_functional_requirement/002_send_without_sync.md)'s
   measurement 2).

5. **State is determined by ownership alone, never by a field.** This is what
   makes the machine enforceable at compile time and also what makes invariant 3
   true — there is no state variable, so there is nothing to read and nothing to
   get out of sync.

**Invariant 4 is the one that has practical consequences for the test suite.**
It is entirely possible to write a thorough-looking handle test that constructs
a pair, publishes, drains, and asserts correctness — all in H1, on one thread,
proving nothing about the property the crate exists for.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_splitting_a_ring_into_two_ends.md](../algorithm/001_splitting_a_ring_into_two_ends.md) | M1 in full |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_two_handles_over_one_backend.md](../data_structure/001_two_handles_over_one_backend.md) | Why invariant 5 holds — there is no state field to hold a state in |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_capability_follows_the_handle.md](../invariant/001_capability_follows_the_handle.md) | Invariant 1, stated at crate grain with its violation costs |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_split_move_and_drop.md](../lifecycle/001_split_move_and_drop.md) | These states as phases, with their ordering dependencies and cleanup obligations |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_send_without_sync.md](../non_functional_requirement/002_send_without_sync.md) | M2, and invariant 4's requirement that tests actually reach H2 |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_a_convenience_method_undoes_the_crate.md](../pitfall/001_a_convenience_method_undoes_the_crate.md) | M9's undetected status, as the edit that enables it |

### State Machines

| File | Relationship |
|------|--------------|
| [004_ring_liveness_through_a_handle.md](004_ring_liveness_through_a_handle.md) | The orthogonal axis — a ring in any of H1–H4 may be open or closed independently |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_producer.md](../type/001_producer.md) | The value whose `!Clone` status forbids M9 |
| [../type/002_consumer.md](../type/002_consumer.md) | Same, draining |

### Sources

| File | Relationship |
|------|--------------|
| [../invariant/001_capability_follows_the_handle.md](../invariant/001_capability_follows_the_handle.md) | The partition invariant 1 states |
| [`ring_shutdown/readme.md`](../../../ring_shutdown/readme.md) | `reset()` — the operation M7's absence makes necessary |

### Tests

| File | Relationship |
|------|--------------|
| `tests/handle_test.rs` | `the_two_ends_travel_to_separate_threads` — the pair exchanges items in H2 on two real OS threads, which a single-threaded test silently fails to cover (invariant 4) |
| `tests/handle_test.rs` | `the_wrapper_costs_nothing` — each handle is the size of the `ring_core` handle it wraps, so a state field would show up (invariant 5; the "one pointer" figure this instance predicted was wrong, the equality is not) |
| `tests/ui/producer_clones.rs` | M9 — no longer without a detector; the case is outside this crate's stated criterion and is written anyway |

### HD31 — M9 Is Ruled Undetectable in the Transition Table and Detected in the Tests Table

Two rows of this file answer the same question and disagree. Each grep below
is capped `-m1`: unscoped, it would also match the copies of these same two
rows quoted in this very block, and every regeneration would then re-embed
however many copies happened to be live at capture time — a self-referential
count with no fixed point. `-m1` always lands on the one real row, wherever it
sits earlier in the file:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the two rows, matched as table rows so this finding is not its own evidence --'
command grep -m1 -nE '^\| M9 \|' ring_handle/docs/lifecycle/003_handle_ownership.md | cut -c1-120 | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
command grep -m1 -nE '^\| `tests/ui/producer_clones\.rs` \|' ring_handle/docs/lifecycle/003_handle_ownership.md | cut -c1-120 | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
echo '  -- and the cases that actually target a clone --'
ls ring_handle/tests/ui/*clone*.rs | sed 's|.*/|    |'
echo '  -- one of them, in full --'
command grep -vE '^\s*(//|$)' ring_handle/tests/ui/producer_clones.rs | tail -4 | sed 's|^|    |'
```

Live output:

```
  -- the two rows, matched as table rows so this finding is not its own evidence --
| M9 | any | H1 by cloning | — | **Forbidden**, and this is the one forbidden transition with no compile-time detec
| `tests/ui/producer_clones.rs` | M9 — no longer without a detector; the case is outside this crate's stated crit
  -- and the cases that actually target a clone --
    consumer_clones.rs
    producer_clones.rs
    producer_try_clones.rs
  -- one of them, in full --
      let mut ends = split.ends();
      let ( producer, _consumer ) = ends.split();
      let _second = producer.clone();
    }
```

M9's own row calls it "the one forbidden transition with no compile-time
detector." The Tests table, eleven rows later in the same file, cites
`producer_clones.rs` as "M9 — no longer without a detector." Three cases target
the clone axis, not one, and `producer_clones.rs` is four lines of setup plus
`producer.clone()`.

**The prose between them is the part that is right.** "M9 is forbidden by the
absence of a *derive*, and no case in the acceptance criterion covers it" is
still true — [`non_functional_requirement/001`](../non_functional_requirement/001_proven_by_code_that_must_not_compile.md)'s
criterion enumerates cases from this crate's own row, and these three are outside it.
Somebody wrote the cases, updated the Tests row to say so, and left the
transition table's stronger claim — *no detector at all* — untouched.

The two claims are one word apart and a reader takes the table's, because a
state machine's transition table is where a reader goes to learn what is
enforced. [`pattern/001`](../pattern/001_enforce_by_withholding.md)'s HD20
records the same drift from the other end: its consequence list names the Clone
axis as uncovered while three cases cover it. Same suite, two instances, one
undetected update.
