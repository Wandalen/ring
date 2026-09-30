# Non-Functional Requirement: Two Atomic Loads for Every Look

### Scope

- **Purpose**: Account for the memory traffic a wait generates, which is entirely the caller's predicate and not this crate's, and give the multiplier a full budget applies to it.
- **Responsibility**: Show where the loads happen, at what ordering, how many a full `for_data` costs, and what the crate contributes on top.
- **In Scope**: Memory operations attributable to a `wait_until` call.
- **Out of Scope**: Time cost per attempt — see [`002`](002_what_each_strategy_costs_per_attempt.md).

### The Crate Issues No Loads

```sh
cd "$(git rev-parse --show-toplevel)"
grep -vE "^[[:space:]]*(//|///|//!)" ring_wait/src/lib.rs \
  | grep -E "Ordering|atomic|load|store|fence" \
  || echo '(no atomic operation in this crate)'
# control: the identical expression over ring_cursor, which owns the loads
grep -vE "^[[:space:]]*(//|///|//!)" ring_cursor/src/lib.rs \
  | grep -cE "Ordering|atomic|load|store|fence"
```

Live output:

```
(no atomic operation in this crate)
16
```

**No match**, against a control — the identical expression over `ring_cursor` —
that returns sixteen. `ring_wait` names no ordering, performs no atomic operation, and
emits no fence. The only synchronisation instruction anywhere in it is
`core::hint::spin_loop()` at `:123`, which is a scheduling hint rather than a
memory operation.

The comment filter is not optional here: the unstripped grep returns five hits
(`:30`, `:227`, `:236`, `:252`, `:261`) and every one of them is documentation —
the module note, and the two doctests that build a `CursorPair` and `store` into
it to set up a scenario. Doctests are compiled and run, so those `Ordering`
mentions are real code in the *test* sense while being absent from the crate's
compiled surface.

Everything a wait reads, it reads through the caller's closure. That is what
makes `wait_until` generic over `FnMut() -> bool` rather than over a cursor
([`api/002`](../api/002_the_predicate_is_the_parameter.md)), and it means the
memory cost of a wait is a property of the predicate, not of the loop.

### WT16 — Two Acquire Loads per Look, for Both Named Predicates

The two wrappers fix predicates that live in `ring_cursor`, and both are the same
shape:

```rust
// ring_cursor/src/lib.rs — CursorPair::may_claim
pub fn may_claim( &self ) -> bool
{
  ring_seqno::may_claim( self.producer.load( GATING ), self.consumer.load( GATING ), self.capacity )
}

// ring_cursor/src/lib.rs — CursorPair::pending
pub fn pending( &self ) -> u64
{
  ring_seqno::pending( self.producer.load( GATING ), self.consumer.load( GATING ) )
}
```

```sh
cd "$(git rev-parse --show-toplevel)"
# No line numbers anywhere, and `command grep` rather than the shell's ugrep
# shim.  Every claim below is about which functions perform a gated load and
# how many each performs -- never about where in the file they sit.  The
# earlier form printed `grep -n` output from a crate this one does not own, so
# any edit to ring_cursor, by anyone and for any reason, made this block stale
# without a single thing about waiting having changed.
command grep -o 'pub const GATING : Ordering = Ordering::[A-Za-z]*' ring_cursor/src/lib.rs
printf 'gated loads in the crate: %s\n' \
  "$( command grep -o 'load( GATING )' ring_cursor/src/lib.rs | wc -l )"
echo '  -- gated loads per function --'
awk '/^[[:space:]]*(pub )?(const )?fn /{ if ( match( $0, /fn [a-z_]+/ ) ) f = substr( $0, RSTART + 3, RLENGTH - 3 ) }
     /load\( GATING \)/{ print "    " f, gsub( /load\( GATING \)/, "&" ) }' ring_cursor/src/lib.rs
```

Live output:

```
pub const GATING : Ordering = Ordering::Acquire
gated loads in the crate: 7
  -- gated loads per function --
    slowest 1
    free_slots 2
    pending 2
    may_claim 2
```

`GATING` is `Ordering::Acquire`, asserted by a doctest on the constant itself.
The whole crate performs **seven** gated loads across four lines: one inside a
`map` over a cursor slice in `slowest`, and two each in `CursorPair`'s
`free_slots`, `pending` and `may_claim`.

Both of this crate's predicates read **both** cursors, because neither question
can be answered from one: room depends on where the consumer got to, and pending
depends on where the producer got to.

| | Loads per look | Ordering | Cursors read |
|--|---------------:|----------|--------------|
| `for_space` → `may_claim()` | 2 | `Acquire` | producer, consumer |
| `for_data` → `pending()` | 2 | `Acquire` | producer, consumer |
| `wait_until` with any other predicate | whatever the closure does | the closure's | the closure's |

**Disposition:** declined — this file's own "Why This Is the Right Split"
section justifies the two-load cost as the deliberate price of keeping
`ring_wait` free of any `Ordering` of its own; the measurement records an
accepted design tradeoff, not a defect in
`non_functional_requirement/001_two_atomic_loads_for_every_look.md` or the
crate it describes.

### The Multiplier

| Call | Budget | Acquire loads, worst case |
|------|-------:|--------------------------:|
| `for_data( …, WaitKind::None, … )` | any | **2** |
| `for_data( …, kind, 1 )` | 1 | 2 |
| `for_data( …, kind, DEFAULT_SPINS )` | 1024 | **2048** |
| `for_data( …, kind, usize::MAX )` | `usize::MAX` | 2 × `usize::MAX` |

Two thousand `Acquire` loads on two cache lines that another thread is actively
writing is the cost of one default-budget wait that fails. That is not obviously
too much — an `Acquire` load of an uncontended line is a few nanoseconds, and the
measured `Spin` attempt is 55–77 ns total
([`002`](002_what_each_strategy_costs_per_attempt.md)) — but it is the cost, and
nothing in the crate reports or bounds it separately from the attempt count.

The two cursors are deliberately on separate cache lines, and `CursorPair` can
be asked whether they really are:

```rust
// ring_cursor/src/lib.rs:438-441
pub fn on_distinct_lines( &self ) -> bool
{
  on_distinct_lines( self.producer.addr(), self.consumer.addr() )
}
```

That is the mitigation which makes the multiplier tolerable. Without it, every
look would be two reads of one line another thread is writing, and 2048 of them
would be 2048 invalidations rather than 1024 reads of each of two lines.

### What the Crate Adds

Per attempt, on top of the predicate:

| Strategy | Memory operations | Other |
|----------|-------------------|-------|
| `None` | none | one branch, then `break` |
| `Spin` | none | 1–8 × `core::hint::spin_loop()` |
| `Yield` | none | one `sched_yield` syscall |
| `Park` | none | one `nanosleep` syscall |

None of the four touches memory the caller did not hand it. The crate's own
state is one `usize` loop counter in a register.

### Why This Is the Right Split

A waiting library that owned its own loads would have to own the ordering too,
and the family has exactly one answer for that already:

> Each writing `Ordering::Acquire` inline would be the same argument made
> independently in several places, which is how a family ends up with one crate
> relaxed and the rest not.
>
> — `ring_cursor/src/lib.rs:78-81`

`ring_wait` naming no ordering is that principle applied by omission. It has
nothing to get wrong, and a future change to `GATING` reaches every wait in the
family without touching this crate at all.

The cost of the split is the one already recorded: because the predicate is
opaque, the crate cannot bound what a look costs, and its non-blocking guarantee
stops at the closure boundary
([`invariant/002`](../invariant/002_none_looks_exactly_once.md)).

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [002_what_each_strategy_costs_per_attempt.md](002_what_each_strategy_costs_per_attempt.md) | The time cost this multiplies against |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_predicate_is_the_parameter.md](../api/002_the_predicate_is_the_parameter.md) | Why the loads are the caller's |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_one_loop_and_the_two_ways_out.md](../algorithm/001_one_loop_and_the_two_ways_out.md) | The loop that has no memory ordering of its own |
| [../algorithm/002_two_wrappers_over_a_predicate_they_fix.md](../algorithm/002_two_wrappers_over_a_predicate_they_fix.md) | The two predicates measured here |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_a_crate_with_no_type_of_its_own.md](../data_structure/001_a_crate_with_no_type_of_its_own.md) | `CursorPair` as borrowed state |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_none_looks_exactly_once.md](../invariant/002_none_looks_exactly_once.md) | Why the guarantee cannot cover the closure's cost |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_the_predicate_the_pause_and_the_budget.md](../pattern/001_the_predicate_the_pause_and_the_budget.md) | The shape that makes the predicate the caller's |

### Sources

| File | Relationship |
|------|--------------|
| `ring_cursor/src/lib.rs:78-87` | `GATING`, and the reason it is named once |
| `ring_cursor/src/lib.rs:387-390,417-420` | The two predicates, two loads each |
| `ring_cursor/src/lib.rs:438-441` | `on_distinct_lines`, the mitigation the multiplier depends on |

### Tests

| File | Relationship |
|------|--------------|
| `tests/wait_test.rs:280-291` | `for_data` reading both cursors to answer |
| `tests/wait_test.rs:293-302` | `for_space` reading both — a consumer store changes the answer |
| `tests/wait_test.rs:304-324` | A cross-thread wait, where the ordering actually matters |
| `tests/manual/readme.md` § W6 | `ring_cursor` is a used dependency, not a vestigial one |
