# A Fresh `ends` Restarted the Claim Cursor

### Scope

- **Purpose**: Record the trap in building each ends-generation's claimer over a fresh cursor cell — pushes that answer `Ok` and are never delivered, with every atomic operation individually correct.
- **Responsibility**: The defect and its repro, why the type system's fence covered only half of it, and the ownership change that closed it.
- **In Scope**: `Ring::ends`' construction of its claimer, the claim cursor's ownership, and the delivery-contract violation a restart produces.
- **Out of Scope**: The gate arithmetic that lets a producer-behind-consumer state through (noted below; the saturating fold lives in `ring_seqno`); the ordering contract (→ [Publication Ordering](../invariant/002_publication_ordering.md)).

### The Trap

`ends()` built a fresh `Claimer` per call, and `Claimer::new` starts its cursor
at sequence zero — always. On a *fresh* ring that is invisible. On a **used**
ring whose consumer has moved on, the second generation's producers claim
sequences the first generation's consumer had already passed:

1. Three records go through the first `ends` — pushed, drained, committed. The
   consumer cursor stands at 3.
2. A second `ends()` builds a claimer whose cursor stands at 0.
3. The gate hands the claim out: `free_slots( 0, 3, capacity )` folds the
   producer-behind-consumer state through a saturating subtraction and reads it
   as *fully free*.
4. `push` answers `Ok(Seq(0))` and stamps slot 0 — a stamp the consumer,
   scanning forward from 3, never rescans.

Three records through the first `ends`, then `push` → `Ok(Seq(0))`,
`drain` → 0. Count, checksum and per-producer order are all individually
consistent; the records are simply gone. The suite's validation would have
caught it — the driver builds a fresh queue per run, which is exactly why the
benchmark suite never hit it and why the finding surfaced there as
"Found, not fixed" before it surfaced as a test.

### Why the fence did not hold

The type system fenced the *concurrent* half of the hazard: `ends` takes
`&mut self`, so two `Ends` can never coexist, and a `compile_fail` doc test
pins it. But generations are **sequential** — the first ends is dropped, then
the second is built — and nothing connected one generation's cursor to the
next. The doc comment's uniqueness claim ("there is no moment at which two
`Ends` name one ring") was true for aliasing and silent about continuation,
which is the half the defect lived in.

### The Fix

The claim cursor cell moved into `Ring` (`claim_cursor`, cache-line padded,
alone on its line next to the consumer cursor's). Each `ends()` builds its
claimer over that cell through `ring_claim::Claimer::borrowed`, so a claim
grant persists across generations: the second generation's first claim is the
sequence after the first generation's last, and a push through a second
`ends` lands where the consumer is looking for it. The cursor is still moved
only by the compare-exchange loops — ownership of the cell changes nothing
about the gate.

Regression tests: `a_second_ends_continues_the_claim_cursor_rather_than_
restarting_it` and `a_record_pushed_through_a_second_ends_is_delivered_
rather_than_lost`.

### MP54 — the gate's arithmetic trusts monotonicity the structure does not enforce

`free_slots` folds a producer-behind-consumer state through a saturating
subtraction and reports *every slot free* — the exact state a restarted claim
cursor creates. The saturating fold is correct for every state the family's
monotonic sequences can produce, and silently wrong for the one they cannot,
which is why the restart was silent rather than loud: no assertion, no error,
a checksum that balances. The structural fix (the cell in the ring) removes
the reachable path to that state rather than the fold that mishandled it;
`ring_seqno`'s documentation records the same trust on its side.
