# Lifecycle: Buffer State Through a Flush

### Scope

- **Purpose**: Present the seal/drain/reset sequence as states rather than steps, so that the window in which a buffer is neither writable nor drained becomes a named condition rather than an implementation detail.
- **Responsibility**: Enumerate the states, the transitions, and the invariants preserved across them.
- **In Scope**: Buffer condition as this crate drives it; the sealed-and-unreset window.
- **Out of Scope**: The epoch mechanism producing these states (→ [`ring_tls`'s Buffer Epoch Cycle](../../../ring_tls/docs/lifecycle/003_buffer_epoch_cycle.md)); the policy's own state (→ [Policy Arming and Firing](004_policy_arming_and_firing.md)).

### States

| # | State | Writable? | Records reachable? | Reached from |
|---|-------|-----------|-------------------|--------------|
| B0 | **Empty** | Yes | Nothing staged | Binding; a completed flush |
| B1 | **Accumulating** | Yes | Staged, unpublished | An append from B0 or B1 |
| B2 | **Sealed** | **No** | Staged, owned by the flusher | A firing trigger from B1 |
| B3 | **Claimed** | No | Staged; ring space reserved | A successful claim from B2 |
| B4 | **Published** | No | **In the ring**, visible to the consumer | A completed drain from B3 |
| B5 | **Stranded** | **No** | Staged, claim failed, not reset | A failed claim from B2 |

**B5 is the state this instance exists to name.** It is correct — the records
are safe and the caller has been told `Rejected` — and it is a state in which
the buffer accepts no appends while the program continues running. Nothing
about the step-wise description in
[`algorithm/002`](../algorithm/002_sequencing_seal_drain_reset.md) makes it
obvious that this is a *resting* state rather than a transient one, which is
the argument for describing the sequence twice.

**B2 through B4 are transient within one drive call.** B5 is not: it persists
until the next drive call succeeds, and a writer appending in the meantime has
nowhere to put a record.

> **B5 does not exist in the implementation, and naming it here is what removed
> it.**
>
> This section's whole argument is that the sequence deserves describing twice
> because the step-wise form hides a resting state. That argument was correct
> and it had one more consequence than it drew: a resting state nobody wants is
> a reason to reorder the steps, not only a reason to document the state.
>
> `Flusher::run` checks the ring's free capacity **before touching the buffer
> at all** (→ [`algorithm/002`](../algorithm/002_sequencing_seal_drain_reset.md)'s
> reconciliation). A rejection therefore returns from B1, never entering B2, so
> there is no failed claim to strand anything. The states as built are B0, B1,
> and a single expression carrying B2→B4; B3 and B5 have no representation.
>
> | State | As built |
> |-------|----------|
> | B0 Empty | Yes — `buffer.len() == 0` |
> | B1 Accumulating | Yes |
> | B2 Sealed | Collapsed — the `&mut self.buffer` borrow, not an epoch swap |
> | B3 Claimed | Collapsed into `try_push_batch` |
> | B4 Published | Collapsed — same expression |
> | B5 **Stranded** | **Unreachable.** The claim is checked before the seal |
>
> What a rejection leaves is B1 with the same records it had a moment earlier —
> writable, appendable, and indistinguishable from never having driven. That is
> what `a_rejected_flush_leaves_the_policy_armed` observes: two refusals, the
> records still staged and still countable through `staged()`, then the same
> four records landing in order once the consumer frees room.

### Transitions

| # | From | To | Trigger | Notes |
|---|------|----|---------|-------|
| T1 | B0 | B1 | An append | |
| T2 | B1 | B1 | An append | The common case; policy consulted each time |
| T3 | B1 | B2 | A firing trigger | Seal |
| T4 | B2 | B3 | Claim succeeds | |
| T5 | B2 | B5 | **Claim fails** | Reset deliberately skipped (O3/O4) |
| T6 | B3 | B4 | Drain completes | Cannot fail |
| T7 | B4 | B0 | Reset | |
| T8 | B5 | B3 | A later drive; claim now succeeds | **The recovery path** |
| T9 | B2 | B0 | Seal found nothing | `TriggeredEmpty` |
| T10 | B1 | B1 | An append when full, policy is `OnBarrier` | **Unresolved** — see below |
| T11 | B5 | — | An append while stranded | **Unresolved** |

**T8 is the recovery and it depends on the caller driving again.** Nothing here
retries; a consumer that stops driving after a rejection leaves the buffer in
B5 permanently and every subsequent append hits T11.

**T10 and T11 are the two unresolved transitions and they are the same
question at different points:** what happens to an append with nowhere to go.
[`algorithm/001`](../algorithm/001_evaluating_a_policy_at_an_append.md) records
T10's three options — flush anyway, reject the append, defer to
`ring_overflow` — and declines to choose. T11 is narrower and more urgent,
because B5 is reachable by ordinary backpressure rather than by
misconfiguration.

**Both belong to `ring_overflow`** (which owns `OverflowPolicy`'s handlers,
with the enum itself living in `ring_types`), which this crate does not
depend on. Recorded in
[`decisions/`](../decisions/readme.md).

**As built, T5, T8 and T11 are all unreachable and T10 is the only question
left.** With B5 gone the "more urgent" of the two disappears, and what remains
is the one the reordering could not touch: an `OnBarrier` buffer that fills
between announcements. That is answered — option 2, refuse the append visibly —
and `append` returns `RingError::Full` for it, with `buffer_capacity()`
available so a caller can see it coming. `ring_overflow` is still the right
owner of a *policy* over that refusal; it is no longer needed to decide whether
a refusal happens at all.

**T2's "policy consulted each time" is also wrong as built.** `append` is one
`push` and consults nothing — the policy is read on `drive`, at the caller's
cadence. That was a cost decision
(→ [`nfr/002`](../non_functional_requirement/002_the_decision_costs_nothing_on_the_append_path.md)),
and it does not change any transition in this table; T2 is still an append from
B1 to B1. Only the annotation was wrong.

### Behavioral Invariants

| # | Invariant | Holds because |
|---|-----------|---------------|
| I1 | Records are never in two places at once | Seal transfers ownership; the drain copies into claimed space before publish |
| I2 | No record is lost across any transition | **As built:** a rejection never enters B2, so no record is ever in a state from which it could be lost — a stronger proof than "T5 skips reset; T8 recovers from B5," which held before the reordering (→ `FL31` below) |
| I3 | The writer never observes B2–B5 as writable | Seal swaps the region before the flusher touches it — `ring_tls`'s epoch mechanism |
| I4 | B4 → B0 is unconditional | Reset cannot fail |
| I5 | Exactly one state at a time per buffer | One flusher per buffer ([`type/001`](../type/001_flush_policy.md)'s N3) |
| I6 | No transition parks or blocks | The family's no-parking-on-the-tick-path rule |
| I7 | Every transition into B4 appends exactly one flush-log entry | The log derives from the outcome ([`type/002`](../type/002_flush_outcome.md)'s M5) |

**I2 is the invariant the whole state machine is arranged around** and T5 is
the transition that would break it under the obvious simplification. "Always
reset after seal" is tidier, symmetric, and loses every record in the rejected
case. The asymmetry is the correctness.

**I5 deserves a note about what it does not say.** One flusher per buffer means
one state machine per buffer — but a thread may own several buffers for several
rings, each with its own policy — the same buffer can serve rings with
different latency needs. Whether that means one buffer with
several policies or several buffers is a shape question this crate's docs do
not settle, and I5 is stated per-buffer so that it remains true either way.

**I3 is inherited, not enforced here.** This crate calls seal and trusts the
swap. If `ring_tls`'s epoch mechanism has a window, every state above B1 is
racy and nothing in this crate's tests would show it.

**I3 as built is enforced by ownership and inherits nothing.** No epoch swap is
called; `Flusher::new` takes the buffer by value, so there is no writer outside
the flusher to observe any state at all. The invariant holds because its
subject does not exist — the strongest form available, and not the one this
section anticipated. `I1` and `I4` are vacuous for the same reason.

**Three of the seven invariants therefore lost their content and none lost
their truth** — I1, I3 and I4, each vacuous because its subject (a second
writer observing B2 through B5) no longer exists — which is worth stating
plainly rather than deleting the rows: an invariant that became
unrepresentable is a design outcome, and a later change that reintroduces a
shared buffer reintroduces every one of them. **I2 is a different case, not
this one:** not vacuous, but justified by transitions this document elsewhere
calls unreachable — see `FL31` below for the proof that survives without them.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_sequencing_seal_drain_reset.md](../algorithm/002_sequencing_seal_drain_reset.md) | The same sequence as steps; its O3/O4 produce T5 |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_two_dependencies_and_the_barrier_it_cannot_see.md](../integration/001_two_dependencies_and_the_barrier_it_cannot_see.md) | Where `ring_overflow`'s absence leaves T10 and T11 unowned |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_the_consolidation_cycle.md](../lifecycle/001_the_consolidation_cycle.md) | The same cycle as phases; B5 is its C3 → K5 skip |

### State Machines

| File | Relationship |
|------|--------------|
| [004_policy_arming_and_firing.md](004_policy_arming_and_firing.md) | The orthogonal axis — policy state, independent of buffer state |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_flush_outcome.md](../type/002_flush_outcome.md) | B4 → `Flushed`; B5 → `Rejected`; B0 via T9 → `TriggeredEmpty` |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_tls/docs/lifecycle/003_buffer_epoch_cycle.md`](../../../ring_tls/docs/lifecycle/003_buffer_epoch_cycle.md) | I3's mechanism, which this machine assumes |
| [`ring_overflow/readme.md`](../../../ring_overflow/readme.md) | Owns T10 and T11 |

### Tests

| File | Relationship |
|------|--------------|
| `tests/flush_test.rs` | B5 and T8 — `a_rejected_flush_leaves_the_policy_armed` is exactly the scenario this row specifies, step for step, and additionally asserts the log distinguishes the two refusals from the success that followed |

### FL31 — The Invariant Tally Counts Two, Names Three, and Misses the One Justified by Two Unreachable Transitions

The correction block audits its own invariants and the audit does not close:

```sh
cd "$(git rev-parse --show-toplevel)"
F=ring_flush/docs/lifecycle/003_buffer_state_through_a_flush.md
echo '  -- the tally, and the invariants it names as emptied --'
awk '/^### FL/{ exit } /vacuous for the same reason|Two of the seven|I3 as built/{ printf "    %d: %s\n", NR, substr( $0, 1, 92 ) }' "$F"
echo '  -- what the same document says about the transitions I2 rests on --'
awk '/^### FL/{ exit } /^\| I2 \|/{ printf "    %s\n", substr( $0, 1, 92 ) }' "$F"
awk '/^### FL/{ exit } /T5, T8 and T11 are all unreachable|^> \| B5 \*\*Stranded\*\*/{ printf "    %d: %s\n", NR, substr( $0, 1, 92 ) }' "$F"
```

Live output:

```
  -- the tally, and the invariants it names as emptied --
    140: **I3 as built is enforced by ownership and inherits nothing.** No epoch swap is
    144: section anticipated. `I1` and `I4` are vacuous for the same reason.
  -- what the same document says about the transitions I2 rests on --
    | I2 | No record is lost across any transition | **As built:** a rejection never enters B2, 
    54: > | B5 **Stranded** | **Unreachable.** The claim is checked before the seal |
    95: **As built, T5, T8 and T11 are all unreachable and T10 is the only question
```

I2 — "No record is lost across any transition" — holds, the table says, "because
T5 skips reset; T8 recovers from B5." The same document states that T5, T8 and
T11 are unreachable and that B5 has no representation. **The invariant's entire
stated justification is a pair of transitions the instance elsewhere says cannot
occur.**

I2 is nonetheless true, and more strongly than the row claims: a rejection never
enters B2, so no record is ever in a state from which it could be lost. The row
records the old proof of a claim whose new proof is better.

**The audit that caught I1, I3 and I4 stopped one row short, and miscounted the
rows it did catch.** "Two of the seven invariants therefore lost their content"
sits immediately after a sentence naming three — I3 explicitly, then "`I1` and
`I4` are vacuous for the same reason." The count was written for I3 and one
other, then a second reason was appended to the sentence above it and the tally
below was not re-read.

**The shape is specific to corrective passes.** An ordinary error is one claim
being wrong. This is a *correction* being incomplete in two directions at once —
it examined the invariants whose subject disappeared (B2's writability) and not
the invariant whose *evidence* disappeared, and it summarised its own findings
without recounting them. A reader trusts a paragraph that says "two of seven"
more than one that says "some," because a number reads as the product of
counting.

```sh
# -mN and no -n: this file is its own subject, so an unbounded
# match also finds this command line and every copy of its own
# output below, and -n re-prefixes a fresh line number onto each
# earlier pass's output -- the stacked addresses this block carried
cd "$(git rev-parse --show-toplevel)"
F=ring_flush/docs/lifecycle/003_buffer_state_through_a_flush.md
echo '  -- I2 row now reads --'
command grep -m2 'As built:.*rejection never enters B2' "$F"
echo '  -- tally paragraph now reads --'
command grep -m1 'Three of the seven' "$F"
```

Live output:

```
  -- I2 row now reads --
| I2 | No record is lost across any transition | **As built:** a rejection never enters B2, so no record is ever in a state from which it could be lost — a stronger proof than "T5 skips reset; T8 recovers from B5," which held before the reordering (→ `FL31` below) |
    | I2 | No record is lost across any transition | **As built:** a rejection never enters B2, 
  -- tally paragraph now reads --
**Three of the seven invariants therefore lost their content and none lost
```

**Disposition:** applied — I2's row now states the as-built proof (a
rejection never reaches B2, so nothing is ever in a losable state) instead of
citing T5/T8/B5, which the same document already calls unreachable, and the
tally paragraph now reads "Three" — I1, I3, I4 — with I2 pulled out into its
own sentence explaining it is a stale-justification case, not a vacuous-content
case, so the two defects the finding names (miscount, and I2 omitted from
either bucket) are both corrected in the same edit. Now prints: `Three of the seven`
