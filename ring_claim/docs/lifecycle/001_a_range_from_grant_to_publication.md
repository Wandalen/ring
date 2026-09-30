# Lifecycle: A Range from Grant to Publication

### Scope

- **Purpose**: Trace a sequence through its whole life around the ring, isolate the one transition this crate owns, and record what the next crate's code assumes about how that transition ends.
- **Responsibility**: Name the states, show that this crate's window has exactly two exits, and show that a sibling crate's unbounded spin loop is kept terminating by this crate's `must_use` and nothing else.
- **In Scope**: The life of a sequence from free to free again, and the grant→publication window in particular.
- **Out of Scope**: The `Claimer`'s own life — see [`lifecycle/002`](002_the_claimer_over_a_rings_life.md).

### The States

A sequence goes round exactly once and passes through six states. Each
transition is a cursor moving, and each cursor belongs to a different crate:

| # | State | Left by | Cursor that moves | Owner |
|--:|-------|---------|-------------------|-------|
| 1 | **Free** | a producer claiming it | claimed | **`ring_claim`** |
| 2 | **Claimed** | the producer writing the slot | *none* | the caller |
| 3 | **Written** | the producer publishing | published | `ring_publish` |
| 4 | **Published** | a consumer reading it | *none* | the caller |
| 5 | **Consumed** | that consumer advancing | its own | `ring_consume` |
| 6 | **Released** | every consumer having passed it | *none* — the gate recomputes | `ring_gating` |

Three of the six transitions move no cursor at all. Those are the ones nothing
can observe, and state 2 — claimed but not written — is the one
the claim/publish handshake is built around: a consumer must never see it
(`ring_publish:20-23`).

This crate owns exactly one arrow: **1 → 2**. It is irreversible, and it is the
only transition in the ring that cannot be undone by any later action
([`decisions/001`](../decisions/001_compare_exchange_rather_than_fetch_add.md)).

### CL31 — `ring_publish`'s Unbounded Spin Terminates Because of a Lint in This Crate

`Publisher::publish` (`ring_publish/src/lib.rs:200-210`) is a bare loop
with no budget, no wait strategy and no exit but success:

```rust
pub fn publish( &self, start : Seq, len : usize ) -> Seq
{
  loop
  {
    if let Ok( end ) = self.try_publish( start, len )
    {
      return end;
    }
    core::hint::spin_loop();
  }
}
```

Everywhere else in the family that shape would be a defect, and the crate says
so itself (`:42-53`) before explaining why it is correct here:

> Waiting for space is unbounded: it depends on a consumer that may be slow,
> stalled, or gone, so it needs a strategy and a give-up. Waiting for your
> predecessor to publish is bounded by that producer finishing a slot write it
> has already started **and cannot abandon** — it is not blocked on anything
> itself.

And its `# Panics` section stated the termination argument outright — as that
section read when this finding was raised:

> **Never.** The loop exits when the predecessor publishes, **which it is
> committed to doing**; a caller that publishes a range it never claimed
> deadlocks here instead, which is a caller bug this crate cannot detect

Both sentences rested on the same premise: a producer holding a claim *cannot
abandon it* and *is committed to publishing*. Nothing enforces that. A `Claim`
is two integers with no destructor, and dropping one is a compile **warning**
that `let _ = …` silences
([`decisions/002`](../decisions/002_must_use_without_drop.md)).

So the chain is:

| Link | Strength |
|------|----------|
| `publish` spins forever unless the predecessor publishes | code |
| the predecessor publishes because it is "committed" | **an assumption** |
| the predecessor is committed because it must not drop its `Claim` | a doc comment |
| it must not drop its `Claim` because of `#[ must_use ]` | **a lint** |

The family's one deliberately budget-free spin loop is kept terminating by a
compiler warning in a different crate, two tiers down, that the compiler will
not emit if the caller writes `let _ =`.

Worth being precise about what this is and is not. It is **not** a defect in
`ring_publish`: given a correct producer the argument is sound, and the
alternative — a budget — has, as the same passage says, no correct handling on
exhaustion. It is a case of the strongest guarantee in one crate resting on the
weakest enforcement in another. What was missing was any statement of that
dependency at the place a caller reads: `# Panics` named one deadlock
(publishing an unclaimed range) and omitted the other (a predecessor that
dropped its claim), which is the reachable one.

```sh
cd "$(git rev-parse --show-toplevel)"
# `publish`'s panic contract as it now ships. Anchored on the heading rather
# than on a line address, so an edit above it cannot silently re-target this
# recipe; the file has exactly one `# Panics`, so `-m1` is unambiguous.
command grep -m1 -A16 -F '/// # Panics' ring_publish/src/lib.rs
```

Live output:

```
    /// # Panics
    ///
    /// Never. It deadlocks instead, and there are two ways in.
    ///
    /// A caller that publishes a range it never claimed waits for a turn that
    /// cannot arrive. That is a caller bug this crate cannot detect, and
    /// `try_publish` is the variant for a caller that wants to decide for itself.
    ///
    /// A caller whose *predecessor* dropped its claim without publishing waits
    /// just as long, and that one is not the waiting caller's bug at all. The
    /// module documentation's termination argument — a predecessor "cannot
    /// abandon" a slot write it has already started — describes correct
    /// producers rather than a property the types enforce:
    /// `ring_claim::Claim` has no destructor, so an abandoned claim is a
    /// `#[ must_use ]` warning and nothing more, and `let _ = …` silences even
    /// that. Of the two deadlocks this is the reachable one, and the only
    /// defence against it is that every producer publishes what it claims.
```

**Disposition:** applied — the omission was in `ring_publish`, and that is where
it was fixed: `publish`'s `# Panics` section now opens "Never. It deadlocks
instead, and there are two ways in," and gives both. The unclaimed-range case
keeps its old wording and its pointer to `try_publish` for a caller that wants to
decide for itself. The second case is new, and it is the one this finding is
about — it names the predecessor that dropped its claim, states that the waiting
caller is not the one at fault, and then does the part the old text left implicit:
it says the module documentation's "cannot abandon" is a description of correct
producers rather than a property the types enforce, because `ring_claim::Claim`
has no destructor, so an abandoned claim is a `#[ must_use ]` warning and nothing
more and `let _ = …` silences even that. The spin loop is unchanged and still
correct. Now prints: `Of the two deadlocks this is the reachable one`

### CL32 — One Dropped Claim of Width One Stops the Whole Ring, Not Its Own Slots

Publication is strictly ordered. `try_publish` advances *only* when the
published cursor is exactly at the claim's start (`ring_publish:31-33`):

> [`Publisher::try_publish`] advances only when the published cursor is
> *exactly* at the claim's start. A producer whose predecessor has not finished
> is told so and must try again.

That is a deliberate refusal of the alternatives — highest-contiguous-point, or
a per-slot availability bitmap — which the crate defers to `ring_mpsc`
(`:35-40`). It has a consequence for the abandoned case that neither crate
states:

| If a claim at sequence *N* is dropped | Effect |
|---------------------------------------|--------|
| slots *N .. N+len* | never written, never published |
| the published cursor | **stops at *N*, permanently** |
| every claim granted after *N* | granted successfully, written, and unpublishable forever |
| every consumer | stalls at *N*, so the gate never releases |
| producers | claim until headroom is exhausted, then get `Full` forever |

The damage is not proportional to the claim's width. A dropped claim of **one**
sequence ends the ring exactly as thoroughly as a dropped claim of a thousand,
because the loss is not the slots — it is the ordering token. Everything behind
it is fine, written, and unreachable.

That is what makes `Claim`'s `must_use` message accurate rather than
alarmist:

> a claimed range that is never published strands its slots **and stalls every
> consumer**

The second clause is doing the real work. The first clause alone would describe
a leak; the pair describes a permanent, silent, total stop, and every producer
after the first will keep succeeding for a while — which is the worst possible
diagnostic signature.

**Disposition:** declined — closed by two sibling fixes rather than by a third
edit, and what those two leave over does not belong in a doc comment. The
behaviour itself is no longer only asserted: `one_dropped_claim_pins_the_frontier_and_the_ring_dies_reporting_full`
in `ring_publish/tests/handshake_test.rs` walks this table row for row on
a real ring — the successor's `claim` still `Ok`, the published cursor pinned,
`try_publish` refused, and then the terminal `Err( RingError::Full )` that is
byte-identical to healthy back-pressure (→ CL44). The caller-facing statement of
it now sits in `Publisher::publish`'s own `# Panics` section (→ CL31). What is
left is this finding's distinctive claim — that a dropped claim of width one ends
the ring as thoroughly as one of width a thousand — and writing that into either
crate's API documentation would restate, for an unreachable-by-correct-code case,
a consequence that follows in one step from the refusal rule already stated at
`ring_publish/src/lib.rs:31-33`. The place for a derivation is a corpus
document, which is where it is.

### The Window's Two Exits

Between grant and publication a range is in this crate's blind spot: the
`Claimer` has already moved its cursor and holds no record of who took what.
There are exactly two ways out:

| Exit | Mechanism | Detectable |
|------|-----------|:----------:|
| **Publish** | `ring_publish::Publisher::publish` / `try_publish` | ✔ — the cursor moves |
| **Strand** | drop, early return, `?`, panic, `let _ =` | **✘ — nothing anywhere observes it** |

No third exit exists, and no timeout, watchdog, or reconciliation pass is
defined anywhere in the family. The window is bounded only by the producer's own
discipline — which is exactly the situation `ring_spsc:615-621` calls out as the
reason it adopted a guard type instead, and which this crate cannot adopt
because a guard must publish on drop and publishing needs a ring
([`decisions/002`](../decisions/002_must_use_without_drop.md)).

### Lifecycles

| File | Relationship |
|------|--------------|
| [002_the_claimer_over_a_rings_life.md](002_the_claimer_over_a_rings_life.md) | The `Claimer`'s own life, which has no such window |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_must_use_without_drop.md](../decisions/002_must_use_without_drop.md) | Why the strand exit cannot be closed here |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_two_dependents_that_split_one_feature.md](../integration/001_two_dependents_that_split_one_feature.md) | The two crates that own the transitions either side of this one |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_a_refused_claim_moves_nothing.md](../invariant/002_a_refused_claim_moves_nothing.md) | Why a refusal never enters the window at all |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_dropping_a_claim.md](../pitfall/001_dropping_a_claim.md) | The four routes to the strand exit |

### Sources

| File | Relationship |
|------|--------------|
| `ring_claim/src/lib.rs:37-50` | The stranding argument, stated as a decision |
| `ring_claim/src/lib.rs:94-100` | The message whose second clause names the ring-wide effect |
| `ring_publish/src/lib.rs:18-27` | States 2 and 3, and why conflating them fails only under load |
| `ring_publish/src/lib.rs:29-40` | Why publication is refused rather than reordered |
| `ring_publish/src/lib.rs:42-53` | Why the spin needs no budget — the assumption this crate cannot guarantee |
| `ring_publish/src/lib.rs:183-210` | "Never" panics, and the loop that argument protects |

### Tests

| File | Relationship |
|------|--------------|
| `ring_publish/tests/handshake_test.rs` | The whole 1→4 path under `loom`, the family's only model of it |
| `tests/claim_test.rs:137` — `successive_claims_are_contiguous…` | That the 1→2 transition leaves no gap for publication to trip on |
| `tests/manual/readme.md § C4` | The two halves — one messaged `must_use`, no `Drop` — that the window rests on |
