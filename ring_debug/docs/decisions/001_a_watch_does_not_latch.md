# A `Watch` reports only what it can observe now, and does not latch a violation it has reported

Status: Deferred

## Context

`ring_debug::Watch` keeps the last valid observation of a cursor pair and compares each new one against it.
`Watch::observe` adopts a reading only when it passes, so a fault that persists reports on every call.

The open case is a fault that clears. A corrupting write followed by a legitimate producer advance restores the
ordering the checks test for, and the next observation is valid again. The watch has to either keep reporting the
earlier violation or report the valid reading.

Two forces pull against each other:

- An investigator usually wants memory. A fault that appeared once and cleared is the hardest kind to catch and
  the one most worth knowing about.
- The crate cannot tell a real recovery from a cursor that was corrupted and then overwritten with a plausible
  value. That ring has not recovered. It has lost records silently, and both cases produce the same readings.

## Decision

Keep the current behaviour until an investigation shows which way it should go. Both positions are coherent, and
no-latch is the default because no code was written either way, not because it won.

A `Watch` does not latch. `Watch::observe` returns `Ok` for any reading that is valid against the last accepted
baseline, whatever it returned before, and that reading becomes the new baseline. The test
`a_watch_that_faulted_reports_ok_once_the_ring_recovers` pins the transition: a violation, then a valid reading,
then `Ok`, then ordinary steps that still pass.

Not latching is the stricter reading of what a checker is. The watch reports what it can observe and nothing else. A latch
would make it claim knowledge of the ring's history that it does not have, and it would need a definition of
"recovered" that this crate cannot supply.

A caller that wants a history can keep one by recording each `Err` that `observe` returns.

## Alternatives considered

- **Latch on the first violation.** Every later observation would report the fault. This needs a fourth field
  beside `producer`, `consumer` and `capacity`, and a `Watch` could no longer be reused across phases of a run
  without a reset operation.
- **Latch, with an explicit `reset`.** The same costs, plus a decision about what `reset` means when the ring is
  still broken at the moment it is called.

## Consequences

- An investigator polling at intervals can miss a fault that appeared and cleared between two polls. That cost
  falls on whoever hits the case and leaves no trace, so nothing in the crate can measure it.
- Reversing the decision is cheap in code: one `bool` field and one branch in `observe`. The test above is the one
  that has to change.
- Revisit when a real investigation loses a fault that mattered because the watch stopped reporting it.
