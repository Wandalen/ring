# Lifecycle: Open and Closed

### Scope

- **Purpose**: Give the two states a ring's liveness has, the edges between them, and which operations each state permits.
- **Responsibility**: The diagram, the per-state operation table, and which edges are idempotent.
- **In Scope**: `Shutdown`'s two states and the `close` / `reopen` / `reset` edges.
- **Out of Scope**: Slot-level or occupancy states, which are `ring_core`'s (→ [`ring_core/docs/lifecycle/002`](../../../ring_core/docs/lifecycle/003_occupancy_across_backends.md)).

### States

```
            close                 close (again)
    ┌──────────────────────►  ┌──────┐
    │                         │      ▼
 ┌──────┐                   ┌──────────┐
 │ Open │                   │  Closed  │
 └──────┘                   └──────────┘
    ▲                         │      ▲
    └──────────────────────────      │
             reopen            └──────┘
                                reopen is not idempotent —
                                the token is consumed

    reset : Open|Closed ──close──► Closed ──discard──► Closed ──reopen──► Open
```

Two states, and the machine is that small because the flag is one `AtomicBool`.
Everything else this crate does is a function of which state it is in.

#### What each state permits

| Operation | Open | Closed |
|---|---|---|
| `is_closed` | `false` | `true` |
| `admit` | `Ok( () )` | `Err( RingError::Closed )` |
| `Guarded::try_push` | delegates to the ring | `Err( Refusal::Closed( record ) )` |
| `Guarded::try_push_batch` | delegates | `0`, **consuming nothing** |
| `Guarded::is_blocked` | `is_full()` | `true` |
| `Stopped::drain_all` | **no new token** — none can be minted here | permitted |
| `Stopped::discard_all` | **no new token** | permitted |
| `wait_for_close` | spins to budget exhaustion, `Err( Empty )` | returns immediately, `Ok` |
| `for_space_or_close` | `Ok( Wake::Ready )` or `Err( Full )` | `Ok( Wake::Closed )`, whatever the occupancy |

The two **no new token** rows are the interesting ones: no runtime check
forbids them, and a `Stopped` obtained earlier and still in scope drains or
discards exactly as it would in `Closed` — what's absent from the Open state
is the *minting* of a fresh [`Stopped`](../type/001_stopped_proof_token.md),
not the ability to act through one already held (→ SD31 below).

### Transitions

| Edge | Idempotent | Note |
|---|---|---|
| `close` | **yes** | Closing a closed shutdown returns a fresh, usable token |
| `reopen` | no — consumes the token | A second call needs a second `close` |
| `reset` | **yes** | Ends Open from either start state; a reset of an empty open ring returns `0` |

`close`'s idempotence is load-bearing. Teardown is reached from the normal end
of a run and from an unwind through a guard, and neither path should have to
discover whether it is first.

`reset`'s idempotence is the same argument at the harness level: a test
fixture calling `reset` in setup and teardown must not care which ran last.

### Behavioral Invariants

Three properties hold across every path through this machine, and each is the
kind a caller would otherwise have to infer from the tables above:

1. **The state is total.** There is no third state, no "closing", no
   partially-closed. `is_closed` answers `true` or `false` and nothing observes
   an in-between, because the flag is one `AtomicBool` and one store.
2. **Only `Closed` can produce a `Stopped`.** This is a claim about minting,
   not about use: the two **no new token** rows in the permission table are
   structural, not conditional — there is no expression that mints a fresh
   `Stopped` while the flag reads Open. A `Stopped` obtained before a later
   `reopen` remains live afterward, and a drain through *that* token compiles
   regardless of what the flag currently reads (→ SD31 below).
3. **Neither state is a sink.** Every state is reachable from every other in one
   edge, which is the next paragraph's subject.

#### `Closed` is not terminal

`RingError::is_transient()` returns `false` for `Closed`, which reads as
terminal and is not: it means *retrying the same operation* cannot clear it.
`reopen` can, and `reset` does. The distinction matters for a producer's retry
loop — it should stop retrying, not conclude the ring is dead.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_shutdown
printf 'what the edge table says of close: %s\n' "$( awk '/^### Regenerate/{ exit } /^\| .close. \|/{ sub( /^\| .close. \| /, "" ); print }' docs/lifecycle/002_open_and_closed.md )"
printf 'what invariant 2 says:         %s\n' "$( awk '/^### Regenerate/{ exit } { print }' docs/lifecycle/002_open_and_closed.md | tr '\n' ' ' | sed 's/  */ /g' | command grep -o 'no expression that mints a fresh .Stopped. while the flag reads Open' )"
printf 'how a token is minted:         %s\n' "$( command grep -o 'pub fn close.*' src/lib.rs )"
printf 'how one is spent:              %s\n' "$( command grep -o 'pub fn reopen.*' src/lib.rs )"
printf 'rows in the permission table:  %s\n' "$( awk '/^#### What each state permits/{f=1} f&&/^### /{exit} f&&/^\| .[a-z_A-Z:]/{ n++ } END{ print n-1 }' docs/lifecycle/002_open_and_closed.md )"
printf 'of those, marked no-new-token: %s\n' "$( awk '/^#### What each state permits/{f=1} f&&/^### /{exit} f&&/^\|/&&/no new token/{ n++ } END{ print n+0 }' docs/lifecycle/002_open_and_closed.md )"
printf 'edges in the transitions table: %s\n' "$( awk '/^### Transitions/{f=1} f&&/^`close/{exit} f&&/^\| .[a-z]/{ n++ } END{ print n-1 }' docs/lifecycle/002_open_and_closed.md )"
printf 'of those, methods on the flag: %s\n' "$( command grep -cE '^  pub fn (close|reopen)\(' src/lib.rs )"
printf 'and free fns needing a ring:   %s\n' "$( command grep -c '^pub fn reset<' src/lib.rs )"
printf 'what reset needs to be called: %s\n' "$( command grep -o 'pub fn reset< T : Send >.*' src/lib.rs )"
printf 'what a bare Shutdown needs:    %s\n' "$( command grep -o 'pub const fn new() -> Self' src/lib.rs )"
printf 'tests holding a flag, no ring: %s\n' "$( awk '/^fn [a-z_]*\(\)/{ name=$2; body="" } /^\}$/{ if( name != "" && body !~ /ring\( / && body ~ /Shutdown::new|Shutdown::default/ ) printf "%s ", name; name="" } { body = body "\n" $0 }' tests/shutdown_test.rs )"
```

Live output:

```
what the edge table says of close: **yes** | Closing a closed shutdown returns a fresh, usable token |
what invariant 2 says:         no expression that mints a fresh `Stopped` while the flag reads Open
how a token is minted:         pub fn close( &self ) -> Stopped< '_ >
how one is spent:              pub fn reopen( self )
rows in the permission table:  9
of those, marked no-new-token: 2
edges in the transitions table: 3
of those, methods on the flag: 2
and free fns needing a ring:   1
what reset needs to be called: pub fn reset< T : Send >( shutdown : &Shutdown, consumer : &mut Consumer< '_, T > ) -> usize
what a bare Shutdown needs:    pub const fn new() -> Self
tests holding a flag, no ring: close_is_idempotent_and_admit_reports_it() default_is_an_open_shutdown() wait_for_close_reports_the_budget_running_out() for_space_or_close_names_the_exit_it_took() for_space_or_close_reports_back_pressure_as_full() 
```

### Types

| File | Relationship |
|------|--------------|
| [../type/001_stopped_proof_token.md](../type/001_stopped_proof_token.md) | Why two rows of the operation table are unreachable rather than erroring |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_teardown_and_reuse.md](../lifecycle/001_teardown_and_reuse.md) | The `reset` path traversed end to end, with what it promises about the ring afterwards |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_shutdown_surface.md](../api/001_shutdown_surface.md) | The same operations as a surface rather than as a state table |

### Tests

| File | Relationship |
|------|--------------|
| `tests/shutdown_test.rs` | `close_is_idempotent_and_admit_reports_it` — the edge table's first row |
| `tests/shutdown_test.rs` | `reset_discards_and_leaves_the_ring_open` — the third row, from both start states |
| `tests/shutdown_test.rs` | `a_closed_batch_push_consumes_nothing` — the operation table's fourth row |
| `tests/shutdown_test.rs` | `a_close_is_not_something_retrying_clears` — the not-terminal reading |

### SD31 — The Transitions Table Supplies the Counterexample to a Behavioral Invariant Twenty Lines Below It

Behavioral Invariant 2 above reads: *"a drain in the Open state is not an error
a caller could hit, it is an expression that does not compile."* The Transitions
table, in the same document, says of `close`: **yes**, idempotent — *"Closing a
closed shutdown returns a fresh, usable token."*

Both are accurate descriptions of the code, and together they refute the first.
`close( &self )` takes a shared borrow and mints without limit; `reopen( self )`
consumes only the receiver it was called on. Mint two, spend one, and the
machine is in the Open state with a live `Stopped` in scope — where
`stopped.drain_all( … )` compiles. The permission table's two **not reachable**
rows are reachable, and the paragraph directly under them explains they are
absent *"not by a runtime check but because `Stopped` cannot be obtained there"*,
which is true of the current state and not of the current scope.

The idempotence is deliberate and the reason given for it is good — teardown is
reached from a normal end and from an unwind, *"and neither path should have to
discover whether it is first"*. The defect is not that `close` is idempotent. It
is that the same document states a structural guarantee that idempotence
removes, and neither statement was read against the other.

This is the third sighting of one fact and it is now in every stance a document
can take: [`decisions/002`](../decisions/002_should_a_stopped_token_be_unique.md)
treats it as an **open question**, [`../invariant/002`](../invariant/002_drain_terminates_because_close_preceded_it.md)
asserts its **impossibility**, and this table presents it as an intended
**feature**. All three are current, none cites the others on this point, and no
corpus checker compares two statements inside one file, let alone across three.

**Disposition:** applied — the permission table's two rows, the paragraph
under it, and Behavioral Invariant 2 now all state the narrower claim (no
*new* token can be minted while Open) instead of the broader one the
Transitions table's own idempotence row contradicts. Now prints: `none can
be minted here`

Scoped to lines 1-88 (the substantive prose, before `### Regenerate`'s own
verification commands, which would otherwise self-match their own quoted
search text):

```sh
cd "$(git rev-parse --show-toplevel)"/ring_shutdown/docs/lifecycle
command sed -n '1,88p' 002_open_and_closed.md | tr '\n' ' ' | command grep -o 'not reachable' | wc -l
command sed -n '40,41p' 002_open_and_closed.md
command sed -n '72,77p' 002_open_and_closed.md | command grep -c 'no expression that mints a fresh'
```

Live output:

```
0
| `Stopped::drain_all` | **no new token** — none can be minted here | permitted |
| `Stopped::discard_all` | **no new token** | permitted |
1
```

### SD32 — The State Machine Has Three Edges and Only Two Can Be Taken by the Type It Is Drawn Over

The diagram and both tables above describe the states of a `Shutdown`, which is
one `AtomicBool` and nothing else — the document says so: *"the machine is that
small because the flag is one `AtomicBool`."* Of the three edges in the
Transitions table, `close` and `reopen` are methods on that flag. The third is
not: `pub fn reset< T : Send >( shutdown : &Shutdown, consumer : &mut Consumer< '_, T > ) -> usize`
is a free function that also needs a `Consumer` borrowed from a ring this crate
does not own.

That matters because a `Shutdown` is freely constructible with no ring at all —
`pub const fn new() -> Self` takes no arguments — and **five** of the crate's
twenty tests do exactly that: `close_is_idempotent_and_admit_reports_it`,
`default_is_an_open_shutdown`, `wait_for_close_reports_the_budget_running_out`,
`for_space_or_close_names_the_exit_it_took`, and
`for_space_or_close_reports_back_pressure_as_full` all drive this machine with
no ring in scope.

For every one of them, Behavioral Invariant 3 — *"Neither state is a sink. Every
state is reachable from every other in one edge"* — is true only via `close` and
`reopen`, and the `reset` row of the table is not an available edge. The
document does not distinguish the two kinds. A reader takes the three rows as
three ways to move the same object, and one of them requires an argument the
object's own constructor never mentions.

The same conflation shows up in the permission table, whose nine rows mix
operations on the flag (`is_closed`, `admit`) with operations on values the flag
hands out (`Guarded::try_push`, `Stopped::drain_all`) and operations that need a
`CursorPair` from a third crate (`for_space_or_close`). It is a useful table.
What it is not is a state machine over `Shutdown`, which is what the section
above it says it is.
