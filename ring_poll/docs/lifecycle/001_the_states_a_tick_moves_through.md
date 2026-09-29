# Lifecycle: The States A Tick Moves Through

### Scope

- **Purpose**: Record the states a `Tick` occupies from construction to disuse, and which of the transitions between them the type represents.
- **Responsibility**: The one constructor, the accumulating middle, the read, the return to zero, and the end that still does not exist.
- **In Scope**: `Tick::new`, the four mutating methods, `progress`, `reset`, and the absence of any *terminal* operation.
- **Out of Scope**: What `Copy` does to a tick in flight (→ [`../data_structure/002`](../data_structure/002_a_copy_accumulator.md)); a record's own path through the crate (→ [`002`](002_where_a_record_can_end_up.md)).

### The states

A tick is a per-frame object with three states and two transitions, one of which
is not represented in the type:

```text
                 new( budget )
                       │
                       ▼
                  ┌─────────┐
     ┌───────────▶│  fresh  │  moved = 0, lost = 0, progress() = None
     │            └────┬────┘
     │                 │  push / push_batch / recv / drain
     │                 ▼
     │            ┌─────────┐
     │ ┌─────────▶│ working │  moved > 0 possible, budget fixed
     │ │          └────┬────┘
     │ └ any operation ┘   │  progress()  — &self, repeatable
     │                     ▼
     │            ┌─────────┐
     │            │  read   │  …and back to `working` on the next call
     │            └────┬────┘
     └───── reset() ───┘
```

The loop back from *read* to *working* is the point. `progress()` takes `&self`,
so reading the tick does not end it. `reset()` returns it to *fresh* with the
budget intact, which is a re-entry rather than an exit — there is still nothing
that ends a tick → PL29.

### What is fixed and what accumulates

| Field | Set by | Changed by | Read by |
|---|---|---|---|
| `budget` | `Tick::new`, once | nothing — no setter exists | `budget()`, and every forwarding method |
| `moved` | starts at 0 | four `+=` sites, and `reset()` | `progress()` |
| `lost` | starts at 0 | `push_batch` only, and `reset()` | `lost()` |

The asymmetry is deliberate for `budget` — a tick's cost ceiling should not move
under it mid-frame. What follows is that a caller who wants the *budget* changed
must build a whole new tick, and the crate's own suite still does exactly that in
`a_default_tick_is_a_single_attempt` → PL30. The counters are the half that
became changeable: `reset()` zeroes both and leaves the budget alone.

### What the type does not have

| Operation a per-frame object often has | Present? |
|---|---|
| `impl Drop` | no |
| A consuming method (`fn finish( self )`) | no |
| `fn reset( &mut self )` | **yes** — added for PL29 |
| A setter for the budget | no |
| A flag for "already read" | no |

Every method takes `&self` or `&mut self`. Nothing takes `self` by value, so
nothing can mark a tick as spent — `reset` says *start again*, which is a
different statement from *this one is over*.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_poll
printf 'Tick constructors:         %s\n' "$( awk '/^impl Tick$/{f=1} f&&/^\}$/{exit} f' src/lib.rs | command grep -oE 'pub const fn new|pub fn new' | wc -l )"
printf 'Tick methods, in order:    %s\n' "$( awk '/^impl Tick$/{f=1} f&&/^\}$/{exit} f' src/lib.rs | command grep -oE '^  pub (const )?fn [a-z_]+' | sed 's/.*fn //' | tr '\n' ' ' )"
printf 'taking self by value:      %s\n' "$( awk '/^impl Tick$/{f=1} f&&/^\}$/{exit} f' src/lib.rs | command grep -cE 'fn [a-z_]+.*\( self[,)]' || true )"
printf 'taking &mut self:          %s\n' "$( awk '/^impl Tick$/{f=1} f&&/^\}$/{exit} f' src/lib.rs | command grep -c '&mut self' || true )"
printf 'setters for budget:        %s\n' "$( command grep -cE 'fn set_budget|fn with_budget|self\.budget = ' src/lib.rs || true )"
printf 'reset/finish/end methods:  %s\n' "$( command grep -cE 'fn (reset|finish|end|close|consume|into_)' src/lib.rs || true )"
printf 'impl Drop for anything:    %s\n' "$( command grep -c '^impl.*Drop' src/lib.rs || true )"
printf 'progress takes:            %s\n' "$( command grep -oE 'fn progress\( &?m?u?t? ?self \)' src/lib.rs )"
printf 'sites writing moved:       %s\n' "$( command grep -c 'self.moved +=' src/lib.rs || true )"
printf 'sites writing budget:      %s\n' "$( command grep -c 'self.budget =' src/lib.rs || true )"
printf 'tests building a Tick:     %s\n' "$( awk '/^fn /{n=$0} /Tick::(new|default)/{print n}' tests/poll_test.rs | sort -u | wc -l )"
printf 'reading then moving more:  %s\n' "$( awk '/^fn /{n=$0; s=0} /\.progress\(\)/{s=1} s && /\.(push|push_batch|recv|drain)\( &mut/{print n; s=0}' tests/poll_test.rs | sort -u | wc -l )"
printf 'a 2nd Tick for a 0 count:  %s\n' "$( command grep -c 'let mut empty = Tick::default()' tests/poll_test.rs || true )"
printf 'and where:                 %s\n' "$( command grep -n 'let mut empty = Tick::default()' tests/poll_test.rs | cut -d: -f1 | sed 's/^/tests\/poll_test.rs:/' )"
printf 'tests naming reset/frame:  %s\n' "$( command grep -ciE 'reset|reuse|frame' tests/poll_test.rs || true )"
```

Live output:

```
Tick constructors:         1
Tick methods, in order:    new reset budget progress lost push push_batch recv drain 
taking self by value:      0
taking &mut self:          5
setters for budget:        0
reset/finish/end methods:  1
impl Drop for anything:    0
progress takes:            fn progress( &self )
sites writing moved:       4
sites writing budget:      0
tests building a Tick:     9
reading then moving more:  2
a 2nd Tick for a 0 count:  1
and where:                 tests/poll_test.rs:654
tests naming reset/frame:  6
```

### Lifecycles

| File | Relationship |
|------|--------------|
| [002_where_a_record_can_end_up.md](002_where_a_record_can_end_up.md) | The other thing with a lifecycle here, and the one that can end |

### Data Structures

| File | Relationship |
|------|--------------|
| [`../data_structure/002_a_copy_accumulator.md`](../data_structure/002_a_copy_accumulator.md) | PL11 — what `Copy` does to the *working* state above |

### Items

| File | Relationship |
|------|--------------|
| [`../item/001_four_operations_eight_entry_points.md`](../item/001_four_operations_eight_entry_points.md) | The method inventory this file reads as a sequence |

### Types

| File | Relationship |
|------|--------------|
| [`../type/001_budget_clamps_to_one.md`](../type/001_budget_clamps_to_one.md) | The value fixed at construction and never changed after |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | `impl Tick` — every state transition above |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/poll_test.rs`](../../tests/poll_test.rs) | `a_tick_accumulates_across_its_operations`, `a_tick_counts_nothing_when_nothing_moved`, `a_default_tick_is_a_single_attempt` |

### PL29 — a per-frame object with no representation of the frame ending

`Tick` is documented as *"one system's turn on the ring"*. A turn ends. Nothing
in the type says when.

`progress()` takes `&self` and can be called any number of times. No method takes
`self` by value, so none can consume the tick. There was no `Drop`, no `finish`,
no `reset`. A tick that had been read was byte-identical to one that had not, and
a tick carried into a second frame kept accumulating into the same counter.

The result was that "this tick is over" was a fact held entirely by the caller's
control flow. A scheduler that builds a tick per frame is correct and nothing
enforced it; a scheduler that hoisted the tick out of the loop — an ordinary
optimisation, and one that looked harmless because `Tick` was `Copy` and cheap —
got a `Progress` that was the running total since program start rather than the
progress of this frame. Both compiled, both moved records correctly, and the
second reported a number that grew without bound while the frame it described did
not.

The failure lands where the value is *used*, not where the mistake is made. A
scheduler backing off when `progress()` is `None` sees `Made( n )` forever after
the first productive frame and never backs off again — the exact inverse of
PL11's failure, from the same missing boundary.

`fn finish( self ) -> Progress` would make the end a compile-time fact — reading
the progress consumes the tick, and using it again is a move error. It is not the
cheap addition it first looks like: both tests that construct a working tick read
`progress()` and then keep moving records through it, so a consuming read is a
second method rather than a replacement for `progress()`, and the mid-frame read
has to stay. That does not weaken the finding, it locates it — the type supports
an interrogable, non-terminal tick deliberately, and simply has nothing for the
other half.

What was missing was cheaper than the method: nothing in `Tick`'s documentation
said a tick is meant to live for exactly one frame, or that `moved` accumulates
for the tick's whole life rather than the frame's, and the suite never named a
frame at all. Until one of those was written down, hoisting a `Copy` value out of
a loop was an ordinary-looking optimisation with no stated reason not to.

**Disposition:** applied — `Tick` gained `pub const fn reset( &mut self )`, which
zeroes `moved` and `lost` and leaves `budget` untouched, plus a
`# One Frame, Then reset` doc section in `src/lib.rs` stating outright that a tick
measures one frame and that carrying one across frames without resetting
accumulates a running total rather than a per-frame one. This is deliberately not
the `finish( self ) -> Progress` the finding weighed and rejected: a consuming
read would break the mid-frame `progress()`-then-keep-moving pattern two existing
tests rely on, so the boundary is now expressible without being mandatory. The
same-round change that dropped `Copy` (→ [`../data_structure/002`](../data_structure/002_a_copy_accumulator.md))
removes the other half of the hazard — hoisting a tick out of a loop is no longer
an invisible copy. `a_reset_tick_reports_no_progress_and_keeps_its_budget` in
`tests/poll_test.rs` pins both halves: the counters go to zero, the budget does
not. The method order now reads: Now prints: `new reset budget progress lost push push_batch recv drain`

### PL30 — the budget and the counter share one constructor, and the suite already pays for it

`Tick::new( budget )` is the only way to make a tick with a *chosen* budget.
`budget` is never written again — no setter, and no `self.budget =` anywhere in
the file. That immutability is right for a ceiling. The consequence is that the
budget and the counters could not be varied apart: the only operation that set
either one set both. Half of that has since changed — `reset` zeroes the counters
and leaves the budget alone (→ PL29) — and the other half has not: there is still
no way to change a budget without discarding the count.

This is not hypothetical. `a_tick_counts_nothing_when_nothing_moved` needs a tick
whose count is zero after having already asserted on one that is zero for a
different reason, and gets it the only way available:

```rust
assert_eq!( tick.progress(), Progress::None );

assert_eq!( consumer.try_recv(), Some( 2 ) );
let mut empty = Tick::default();                    // the `a 2nd Tick` count above
assert_eq!( empty.recv( &mut consumer ), None, "and an empty read is not progress" );
```

That second tick existed because there was no `reset`. The test is correct and
reads naturally — which is the point: the workaround was cheap enough at test
scale to pass without comment, and the same move at frame scale discards a budget
along with the count. `reset` is now available for exactly this, and the test is
left as written because it is also the site the census above counts.

Both directions were blocked. A scheduler lowering its budget as a frame runs
long — spend three attempts early, one late, the obvious adaptive policy for this
crate's use case — must build a new tick and lose the frame's accumulated
progress. A scheduler wanting a fresh count for a new frame had to rebuild too,
and if the budget was computed rather than constant it had to be recomputed or
carried.

The second direction is open now; the first is not. `with_budget( self, Budget ) -> Self`
is still one line, and it is still absent because nothing has asked for it — but
the reason to keep recording it has narrowed. `Tick`'s documentation now states
that a tick measures one frame and that `reset` is how a frame ends, so a caller
who wants a fresh count is told where to look. A caller who wants a *different
ceiling* mid-frame is not, and still learns the restriction by looking for a
setter and not finding one.
