# Lifecycle: Ring Liveness Through a Handle

### Scope

- **Purpose**: Enumerate the liveness states a handle observes — open, closed, closed-and-drained — and mark the two that are indistinguishable by return value alone, which is where callers go wrong.
- **Responsibility**: The states, their transitions, and the invariants each end may rely on.
- **In Scope**: Liveness as seen from a handle; what each state does to a publish and to a drain.
- **Out of Scope**: Ownership of the handles, an orthogonal axis (→ [Handle Ownership](003_handle_ownership.md)); the `close`/`reset`/`drain_all` operations themselves, which are [`ring_shutdown`](../../../ring_shutdown/readme.md)'s.

### States

**Liveness is a separate axis from ownership.** A ring in any ownership state
may be open or closed; closing does not consume handles and dropping a handle is
not closing. Conflating them produces a `close()` that takes ownership of the
pair, which is more restrictive than what `ring_shutdown` specifies.

**Where `is_closed()` is read, as implemented.** This model was written
expecting the check on the handle itself. It is not there: reading the flag
requires depending on `ring_shutdown`, which depends on `ring_wait`, which would
put a parking operation within reach of the tick path and break the non-parking
restriction (→ [`decisions/002`](../decisions/002_why_is_closed_is_absent.md)). Every state
and transition below is a property of the **ring**, and remains exactly as
described; what moved is the observation point. A caller that needs it wraps the
producer in `ring_shutdown::Guarded`, whose refusal carries the same
distinction. Read `is_closed()` throughout this instance as "the ring's
liveness flag, wherever it is read from."

| # | State | `try_push` | `try_recv` | `is_closed()` |
|---|-------|-----------|-----------|---------------|
| G0 | **Open, empty** | `Ok` (or `Err( Full )` at capacity 0) | `None` | `false` |
| G1 | **Open, has items** | `Ok` until full | The item | `false` |
| G2 | **Open, full** | `Err( Full( item ) )` | The item | `false` |
| G3 | **Closed, has items** | `Err( Closed( item ) )` | The item | `true` |
| G4 | **Closed, drained** | `Err( Closed( item ) )` | `None` | `true` |

**G0 and G4 return `None` from `try_recv` and mean opposite things.** In G0 more
may arrive; in G4 nothing ever will. The return value does not distinguish them
— `is_closed()` is the only thing that does
(→ [Consumer Surface](../api/002_consumer_surface.md)'s Error Handling).

**A consumer that does not check `is_closed()` cannot terminate correctly.** It
either exits early on a transient `None` in G0, losing items still to come, or
polls forever in G4 against a ring that will never fill again. Both are the same
missing check and they fail in opposite directions, which is what makes the bug
hard to characterize from a single observed symptom.

**G3 is the state that makes close non-destructive.** Closing refuses new
publishes and preserves what is already there — `ring_shutdown`'s reached-test
requires that `drain_all()` "returns exactly the items outstanding at close." A
close that discarded them would collapse G3 into G4 and lose data at exactly the
moment a system is trying to shut down cleanly.

**G2 and G3 both refuse a publish and must remain distinguishable.** `Full` is
transient — drain and retry. `Closed` is terminal. A caller that cannot tell
them apart retries against a closed ring indefinitely, which is
[Nothing Reachable From a Handle Can Park](../invariant/002_no_parking_operation_is_reachable.md)'s
W3 reached by way of an error-type simplification
(→ [Producer Surface](../api/001_producer_surface.md)'s guarantee 4).

### Transitions

| # | From | To | Trigger | Observable by |
|---|------|----|---------|---------------|
| X1 | G0 | G1 | A successful publish | The consumer, via `len()` or a successful drain |
| X2 | G1 | G2 | Publishing fills the last slot | The producer, via `Err( Full )` |
| X3 | G2 | G1 | A drain frees a slot | The producer, on its next attempt — **not by notification** |
| X4 | G1 | G0 | The last item is drained | The consumer |
| X5 | G0/G1/G2 | G3/G4 | `close()` | Both ends, via `is_closed()` |
| X6 | G3 | G4 | The last outstanding item is drained | The consumer |
| X7 | G4 | G0 | `reset()` | Both ends. `ring_shutdown`'s criterion requires the result be "byte-identical to a freshly constructed ring" |
| X8 | G3/G4 | — | A second `close()` | Idempotent; not a transition |
| X9 | G4 → G3 | — | — | **Forbidden.** Nothing re-fills a closed ring |

**X3 has no notification and that is the design.** A producer learns the ring
drained by trying again — there is no wakeup, because a wakeup is a parking
mechanism and nothing reachable from a handle may park. This is why the flush
policy in `ring_flush` and the poll loop in `ring_poll`
exist above this crate: someone has to decide *when* to try again, and it is not
the handle.

**X5 is observable from both ends simultaneously and is the only transition
that is.** Every other transition here is caused by one end and noticed by the
other on its next call.

**X7 is the only transition that moves backwards** and it is not this crate's
operation. `reset()` on a closed, drained ring is `ring_shutdown`'s, and the
"byte-identical to a freshly constructed one" requirement is what makes X7 a
genuine return to G0 rather than a state that merely behaves like it.

**X9's impossibility is what makes G4 terminal**, and it is worth naming because
the natural mental model — "closed means the producer stopped" — suggests a
reopen. There is none; `reset()` goes to G0, not to G3.

### Behavioral Invariants

1. **The producer may rely on `Err( Full )` being transient in G2.** Some drain
   can change it. It may not rely on any bound on when.

2. **The producer may rely on `Err( Closed )` being permanent** until an X7 it
   does not perform. This is the only terminal refusal.

3. **The consumer may not conclude "done" from `None`.** `None` holds in both G0
   and G4; only `is_closed()` separates them, and the check must be *after* the
   drain attempt — checking first and draining second loses items published
   between the two calls.

4. **Neither handle caches liveness.** Both read the authoritative flag on each
   call. A cached copy can disagree with `ring_shutdown`'s, and a stale `false`
   means publishing into a closed ring
   (→ [Two Handles Over One Backend](../data_structure/001_two_handles_over_one_backend.md)'s
   absent-state table).

5. **Closing never discards.** G3 exists precisely so that close and drain are
   separable, and `ring_shutdown`'s `drain_all()` depends on it.

6. **These states are orthogonal to ownership.** A ring may be in G3 while in
   ownership state H3 — closed, with items, and no consumer left to drain them.
   That combination is a data-loss configuration reachable through two
   individually reasonable steps, and nothing flags it.

**Invariant 3's ordering requirement is the subtlest rule on this page.** The
correct shutdown loop drains first, then checks closed, then drains once more —
because a publish can land between the drain returning `None` and the closed
check returning `true`. Checking closed first is the natural way to write it and
it drops those items.

**Invariant 6 names a real hole and does not close it.** G3 + H3 is unreachable
by any single mistake: it needs a close and a consumer drop. Both are legitimate
operations, neither is wrong alone, and their combination silently strands
whatever was outstanding.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_producer_surface.md](../api/001_producer_surface.md) | G2 and G3's refusals, and why they stay distinguishable |
| [../api/002_consumer_surface.md](../api/002_consumer_surface.md) | G0 and G4's identical `None`, and the ordering invariant 3 requires |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_two_handles_over_one_backend.md](../data_structure/001_two_handles_over_one_backend.md) | Invariant 4's mechanism — no cached flag, because there is no field to cache it in |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_one_dependency_and_the_backends_beneath.md](../integration/001_one_dependency_and_the_backends_beneath.md) | The `ring_shutdown` seam X5 and X7 cross |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_no_parking_operation_is_reachable.md](../invariant/002_no_parking_operation_is_reachable.md) | Why X3 carries no notification |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_split_move_and_drop.md](../lifecycle/001_split_move_and_drop.md) | Its D3 — the independence invariant 6 formalizes, and the L5 question G3+H3 is a variant of |

### State Machines

| File | Relationship |
|------|--------------|
| [003_handle_ownership.md](003_handle_ownership.md) | The orthogonal axis invariant 6 crosses with |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_shutdown/readme.md`](../../../ring_shutdown/readme.md) | X5, X7, and invariant 5 — "`drain_all()` returns exactly the items outstanding at close" |
| [`ring_poll/readme.md`](../../../ring_poll/readme.md) | Why X3 has no wakeup |
| [`ring_flush/readme.md`](../../../ring_flush/readme.md) | Who decides when to retry after X2 |

### Tests

| File | Relationship |
|------|--------------|
| [`ring_shutdown/tests/shutdown_test.rs`](../../../ring_shutdown/readme.md) | After `close()` with items outstanding, `try_recv` yields exactly those items and then `None` — G3 → G4, and invariant 5. Tested there and not here, because `close()` is not on this crate's surface: `ring_shutdown::Guarded` owns it |
| **not written, and not writable here** | `try_push` returns `Result< (), T >`, not an error enum — there are no `Full` and `Closed` variants on this crate's surface to discriminate. The distinction exists in `ring_shutdown`, which is where the row would have to move to become a test |
| [`ring_shutdown/tests/shutdown_test.rs`](../../../ring_shutdown/readme.md) | A drain-then-check shutdown loop loses no item published concurrently with the close — invariant 3's ordering requirement. Same reason: the loop is written against `Guarded`, which wraps `ring_core::Producer` rather than this crate's, so a caller cannot currently hold both this crate's narrowings and close-awareness |

### HD32 — The Preamble Relocates One Observation and the Whole Transition Set Moved With It

The preamble concedes that `is_closed()` is read elsewhere and rules that
everything else "remains exactly as described." Check what the transitions
actually take:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the three close/reset/drain operations, with the handle type each takes --'
for op in close reset drain_all; do
  command grep -hoE "^ *pub (const )?fn $op<?[^{]*" ring_shutdown/src/lib.rs \
    | sed 's/^ *pub /    /' | cut -c1-100
done
echo '  -- and which crate those Producer/Consumer names come from --'
command grep -E '^use ring_' ring_shutdown/src/lib.rs | sed 's|^|    |'
```

Live output:

```
  -- ring_shutdown's three operations, with the handle type each takes --
    fn close( &self ) -> Stopped< '_ >
    fn reset< T : Send >( shutdown : &Shutdown, consumer : &mut Consumer< '_, T > ) -> usize
    fn drain_all< T : Send >( &self, consumer : &mut Consumer< '_, T >, out : &mut Vec< T > ) -> usi
    fn drain_all_bounded< T : Send >
  -- and which crate those Producer/Consumer names come from --
    use ring_core::{ Consumer, Producer };
    use ring_cursor::CursorPair;
    use ring_types::{ RingError, WaitKind };
```

`reset` and `drain_all` take `&mut Consumer< '_, T >`, and line 38 says which
`Consumer`: `ring_core`'s. This crate's `Consumer` is a different type with no
conversion in either direction.

**So X5, X6, X7, X8 and invariant 5 are not observable from a handle either —
the preamble moved one cell of the states table and the entire Transitions
section went with it.** A holder of `ring_handle::Consumer` cannot close, cannot
reset, cannot `drain_all`, and cannot read the flag; a holder of
`ring_core::Consumer` can do all four and has none of this crate's narrowings
(→ [`decisions/002`](../decisions/002_why_is_closed_is_absent.md)'s HD15).

The instance's title is *Ring Liveness Through a Handle*, and the honest scope
is liveness of the ring — accurate as a model of the ring, and reachable through
no handle this crate hands out. The three Tests rows already point exclusively
at `ring_shutdown/tests/shutdown_test.rs`, which is the same fact arriving from
the direction that gets checked.

One smaller thing the notation hides: `reset()` is written method-style
throughout, and it is a free function taking the `Shutdown` and the consumer as
arguments. `close()` is a method — on `Shutdown`, not on a handle. The parens
make all three read as operations on something this crate produces.
