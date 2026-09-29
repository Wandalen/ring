# Lifecycle: The States A Script Moves Through

### Scope

- **Purpose**: Enumerate the shutdown states a running script occupies, and what each admits.
- **Responsibility**: States, transitions, and the one transition that passes through a state it did not intend to.
- **In Scope**: `Shutdown`'s open/closed flag as a script drives it.
- **Out of Scope**: The ring's own fullness, which is not a state of this machine — a full ring is a condition, not a mode.

### States

| State | `closed_at_end` | A `Push`/`Flush` is | A `Stage` is | A receive is |
|---|---|---|---|---|
| **Open** | `false` | Admitted, then subject to the ring's own capacity | Admitted into the staging buffer regardless of shutdown state | Permitted |
| **Closed** | `true` | Refused `Refusal::Closed`, whatever the ring's capacity | Admitted into the staging buffer regardless of shutdown state | Permitted |

Only two, and the asymmetry between the columns is the point: **closing stops
publication and not consumption.** That is what makes `DrainAll` sound —
`Stopped::drain_all` loops until a batch comes back empty, and it terminates
because nothing can be added while it runs.

A script begins **Open**. It has no third state: `Shutdown` holds one boolean.

### Transitions

| From | Step | To | Notes |
|---|---|---|---|
| Open | `Close` | Closed | — |
| Closed | `Close` | Closed | Idempotent; `ring_shutdown` guarantees this so that teardown reachable from two paths need not know which is first |
| Closed | `Reopen` | Open | The intended use |
| Open | `Reopen` | **Open, via Closed** | The token authorising a reopen is only minted by a close — see [`pitfall/002`](../pitfall/002_reopening_closes_first.md) |
| Open | `DrainAll` | Closed | Closes first, then drains |
| Closed | `DrainAll` | Closed | — |
| either | `Push`/`Flush`/`Recv` | unchanged | These read the state; none writes it |
| either | `Stage` | unchanged | Neither reads nor writes it — the staging buffer is independent of shutdown |

**Row four is the whole reason this file exists.** Every other transition is
what its name says. `Reopen` from Open passes through Closed and back, and in a
single-threaded script that is unobservable — `reopening_an_open_ring_is_observably_a_no_op`
pins that it changes no field of the `Outcome`. In a concurrent one it is a
window in which another thread's push is refused on a ring nobody closed.

### Behavioral Invariants

Four properties hold over every script, whatever sequence of steps it contains:

1. **A script ends in a determinate state, and `Outcome::closed_at_end` reports
   it.** There is no "unknown" — the flag is read once at teardown, after the last
   step, from the same `Shutdown` every step used.
2. **Consumption is never blocked by state.** Both rows of the state table permit
   a receive. This is what makes `DrainAll` terminate, and it is the reason the
   two columns are asymmetric rather than a simple on/off.
3. **Every transition is caller-driven.** No step transitions the machine as a
   side effect of a *ring* condition — a full ring refuses a push without moving
   the state, so a `refused_full` and a `refused_closed` never come from the same
   cause.
4. **`Reopen` and `DrainAll` both pass through Closed.** Neither has an
   Open→Open or Open→Closed shortcut, so a reader reasoning about "was it ever
   closed?" must treat both as a close (→ [`pitfall/002`](../pitfall/002_reopening_closes_first.md)).

#### What has no state here

The ring's fullness is not modelled as a state. A full ring refuses a push, and
so does a closed one, but they are different kinds of thing: fullness is a
function of what the consumer has drained and clears on its own; closure is a
flag and clears only when someone flips it. `ring_shutdown` encodes the
difference in `Refusal`'s two arms, and this crate keeps it as two counters
rather than collapsing them into one refusal count.
→ [`type/001`](../type/001_outcome_and_anomaly.md).

### Evidence

| # | Claim | Test |
|---|---|---|
| N1 | Closed refuses pushes with a reason distinct from fullness | `a_closed_ring_refuses_with_a_reason_of_its_own` |
| N2 | `Reopen` from Closed admits publications again | `reopening_admits_publications_again` |
| N3 | `Reopen` from Open changes no observable | `reopening_an_open_ring_is_observably_a_no_op` |
| N4 | `DrainAll` leaves the ring Closed | `drain_all_recovers_every_record_and_leaves_the_ring_closed` |
| N5 | Closing does not discard what is already in the ring | `closing_leaves_published_records_where_they_are` |
| N6 | A flush into Closed is refused as closed | `a_flush_into_a_closed_ring_is_refused_as_closed` |

**N5 is worth naming.** Closing is not teardown — the records stay where they
are, and `in_ring_at_end` reports them. Recovering them takes a `DrainAll`, and
that separation is what lets a script close, inspect, and then decide.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit
printf 'what Shutdown holds:        %s\n' "$( awk '/^pub struct Shutdown/{f=1} f && /: /{ sub( /^ */, "" ); print; exit }' ../ring_shutdown/src/lib.rs )"
printf 'close and reopen:           %s\n' "$( command grep -hoE 'pub fn (close|reopen)\( [^)]*\)( -> [A-Za-z<> _'"'"']+)?' ../ring_shutdown/src/lib.rs | tr '\n' ' ' )"
printf 'what try_push checks first: %s\n' "$( awk '/pub fn try_push\( &mut self, record/{f=1} f && /if self\./{ sub( /^ */, "" ); print; exit }' ../ring_shutdown/src/lib.rs )"
printf 'shutdown in the Stage arm:  %s\n' "$( awk '/Step::Stage \| Step::StageMany/{f=1} f && /^        },$/{exit} f' src/lib.rs | command grep -c 'shutdown\|guard' || true )"
printf 'the whole Stage write:      %s\n' "$( awk '/staging.push\( record \)/{ sub( /^ */, "" ); print; exit }' src/lib.rs )"
printf 'what fills closed_at_end:   %s\n' "$( awk '/closed_at_end : shutdown/{ sub( /^ */, "" ); print; exit }' src/lib.rs )"
printf 'thread spawns inside run:   %s\n' "$( awk '/pub fn run\( &self/{f=1} f && /^  }$/{exit} f' src/lib.rs | command grep -c 'spawn' || true )"
printf 'N-rows above:               %s\n' "$( command grep -c '^| N[0-9]' docs/lifecycle/001_the_states_a_script_moves_through.md )"
printf 'of those, tests that exist: %s\n' "$( awk -F'|' '/^\| N[0-9]/ { gsub( /[^a-z_]/, "", $4 ); print $4 }' docs/lifecycle/001_the_states_a_script_moves_through.md | while read -r n ; do command grep -q "fn $n" tests/testkit_test.rs && echo x ; done | wc -l )"
```

Live output:

```
what Shutdown holds:        closed : AtomicBool,
close and reopen:           pub fn close( &self ) -> Stopped< '_ > pub fn reopen( self ) 
what try_push checks first: if self.shutdown.is_closed()
shutdown in the Stage arm:  0
the whole Stage write:      if staging.push( record ).is_err() { refused_staging += 1; }
what fills closed_at_end:   closed_at_end : shutdown.is_closed(),
thread spawns inside run:   0
N-rows above:               6
of those, tests that exist: 6
```

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_from_a_step_to_an_outcome.md](../algorithm/001_from_a_step_to_an_outcome.md) | Where each transition sits in the step loop |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_reopening_closes_first.md](../pitfall/002_reopening_closes_first.md) | Row four, in full |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_outcome_and_anomaly.md](../type/001_outcome_and_anomaly.md) | `refused_full` and `refused_closed` as the two refusal kinds |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_shutdown/src/lib.rs`](../../../ring_shutdown/src/lib.rs) | `Shutdown`, `Stopped`, `Guarded`, `Refusal` |

### Tests

| File | Relationship |
|------|--------------|
| `tests/testkit_test.rs` | N1–N6 |

### TK29 — the state table has two write columns and the crate has three write paths

The States table gives one answer per state for *"A push is"*. The crate has
three steps that put a record somewhere — `Push`, `Stage` and `Flush` — and they
do not share that answer.

`Step::Stage`'s whole body is
`if staging.push( record ).is_err() { refused_staging += 1; }`. The word
`shutdown` does not appear in the arm, and neither does `guard`: **zero**
references to the state the table is about. So on a **Closed** ring a `Stage`
step mints a record, stages it successfully, and is counted in `staged_at_end` —
while the row a reader consults says a push is *"Refused `Refusal::Closed`,
whatever the ring's capacity"*.

The transitions table repeats the error in the other direction. Its last row
groups `Push`/`Stage`/`Flush`/`Recv` and says *"These read the state; none writes
it"*. Half of that is right for all four. The reading half is wrong for exactly
one: `Stage` neither reads nor writes it.

Nothing is broken. `Flush` is refused as closed, which is what
`a_flush_into_a_closed_ring_is_refused_as_closed` pins, and that is the correct
design — staging is a buffer in front of the ring, and buffering into it while
closed is what lets a caller close, stage, reopen and flush. What is wrong is
the document: a reader who maps the table's push row onto `Step::Stage` gets the
opposite of what runs, and the crate's most reachable statement of its own state
machine is the place they will look.

The fix is a column, not a sentence. Two states × three write paths is six
cells, and the table currently prints two.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit
command grep -m1 -F 'Neither reads nor writes it — the staging buffer is independent of shutdown' docs/lifecycle/001_the_states_a_script_moves_through.md
```

Live output:

```
| either | `Stage` | unchanged | Neither reads nor writes it — the staging buffer is independent of shutdown |
```

**Disposition:** applied — the States table now carries a separate `Stage`
column stating it is admitted into the staging buffer regardless of shutdown
state, and the Transitions table splits `Stage` out of the four-step row into
its own row noting it neither reads nor writes the state.
Now prints: `Neither reads nor writes it — the staging buffer is independent of shutdown`

### TK30 — the termination argument names a property that is doing no work here

Behavioral invariant 2 says consumption being permitted in both states *"is what
makes `DrainAll` terminate"*, and the States section says the same: `drain_all`
*"terminates because nothing can be added while it runs."*

`Script::run` spawns **zero** threads. Every step runs on the caller's thread,
in order, and `Step::DrainAll` is one of them. Nothing can be added while
`drain_all` runs because there is no other thread to add anything — not because
the ring is closed. Under this crate's scripted fixture the argument would hold
identically on an open ring.

The property is real and it is `ring_shutdown`'s, where a producer thread and a
draining consumer genuinely race and closing is what bounds the loop. Imported
here it reads as a fact about the fixture, and the fixture is the one setting in
which it is unfalsifiable: no script can construct the interleaving the argument
rules out.

This matters for what the invariant *licenses*. A reader who takes invariant 2
as established by the scripted suite has a termination guarantee backed by tests
that never test it. The crate does have a setting where the guarantee is load
bearing — `tests/exhaustive_test.rs`, two threads over a leaked ring — and that
file calls `drain_all` under a bound of one push, which terminates for the same
uninteresting reason a one-element loop always does
(→ [`../invariant/002`](../invariant/002_the_shape_a_delivered_list_must_have.md) TK24).

So the strongest statement this crate can currently make about `drain_all` is
that it terminates in every case it runs. The reason offered is a stronger claim
than the evidence behind it, and it is inherited rather than measured.
