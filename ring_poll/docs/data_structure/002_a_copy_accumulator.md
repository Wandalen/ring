# Data Structure: A Copy Accumulator

### Scope

- **Purpose**: Record what it meant for `Tick` to be `Copy` while carrying a running total, why the derive was dropped, and what duplication still costs now that it is explicit.
- **Responsibility**: The `moved` and `lost` counters, their mutation sites, the aliasing the `Copy` derive permitted, and the arithmetic they use.
- **In Scope**: `#[ derive( Debug, Clone ) ]` on `Tick`, the four `self.moved +=` sites, and what a duplicated tick reports.
- **Out of Scope**: The three shapes considered together (→ [`001`](001_three_values_three_shapes.md)); what `Progress` means as a returned value (→ [`../type/002`](../type/002_progress_has_no_zero_made.md)).

### Abstract

`Tick` is three `usize`-sized fields, two of which are counters that its methods
increment. It used to derive `Copy`. A type that is both trivially duplicated and
internally mutable is a shape worth writing down: every copy is an independent
accumulator, and nothing in the type said so. The derive is gone; `Clone` is not,
so the shape still exists — it just costs a visible `.clone()` now.

### The declaration

```
#[ derive( Debug, Clone ) ]
pub struct Tick
{
  budget : Budget,
  moved : usize,
  lost : usize,
}
```

`budget` is written once, at construction, and only read afterwards.
`moved` starts at zero and is incremented by all four operations:

| Method | Increment |
|---|---|
| `push` | `+= 1` on `Ok` |
| `push_batch` | `+= moved`, the published count |
| `recv` | `+= 1` on `Some` |
| `drain` | `+= moved`, the drained count |

`lost` starts at zero too, and only `push_batch` writes it — see
[`../algorithm/002`](../algorithm/002_what_an_attempt_costs.md). Each method takes
`&mut self`, so accumulating through a single binding behaves exactly as it reads.

### What `Copy` added, and what `Clone` still allows

`Copy` made duplication implicit. Every one of these produced a second,
independent accumulator with the same starting count, with nothing at the call
site to say so:

| Expression | Result under `Copy` | Result now |
|---|---|---|
| `let t2 = tick;` | `tick` still usable — a copy, not a move | `tick` is moved; using it afterwards fails to compile |
| `fn f( t : Tick )` called as `f( tick )` | `f` mutates its own copy; the caller's is untouched | the tick is moved into `f` and does not come back |
| `vec.push( tick )` | the vector holds a snapshot | the vector takes ownership |
| `[ tick; 4 ]` | four accumulators, all at the current count | rejected — the element type is no longer `Copy` |

None of it was *wrong*, and for `Budget` — a value written once and read many
times — `Copy` is exactly right. `Tick` inherited it from sitting in the same
file with the same shape, and `Tick` is the type where duplication has a meaning:
the copies drift → PL11. `Clone` is retained, so a caller who genuinely wants a
second accumulator writes `tick.clone()` and gets exactly the old behaviour, at a
call site that names it.

### The arithmetic

`moved` and `lost` are both `usize` and every site uses `+=`. There is no
saturating or checked form, and the workspace sets `overflow-checks` nowhere, so
a release build wraps rather than panicking. Reaching it requires moving
`usize::MAX` records through one tick, which no scheduler does, and the point of
recording it is the asymmetry rather than the risk: `Budget` is a newtype
specifically so a degenerate value cannot be constructed, and the two counters
next to it have no such guard → PL12.

### Evidence

| # | Claim | Test |
|---|---|---|
| C1 | A tick accumulates across operations of different kinds | `a_tick_accumulates_across_its_operations` |
| C2 | A tick that moved nothing reports `Progress::None` | `a_tick_counts_nothing_when_nothing_moved` |
| C3 | `Tick` is not `Copy`, and cloning it forks the accumulator | `a_cloned_tick_accumulates_separately` |
| C4 | A tick can be zeroed without losing its budget | `a_reset_tick_reports_no_progress_and_keeps_its_budget` |

C3 was the row worth noting, and it is why this document has a finding. It used
to read *"`Tick` is `Copy` | the derive; no test asserts a consequence of it"* —
the derive was checked by the compiler, and what the derive *permitted* was
checked by nothing. The consequence is now the thing the test asserts, and the
implicit form that made it reachable by accident is gone.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_poll
printf 'Tick derives:             %s\n' "$( awk '/^pub struct Tick/{ print d; exit } /^#\[ derive\(/{ d = $0 }' src/lib.rs | sed 's/^#\[ derive( //; s/ ) \]$//' )"
printf 'Tick doc on the omission: %s\n' "$( command grep -m1 -oE '# Deliberately Not .Copy.' src/lib.rs | tr -d '\140' )"
printf 'Tick fields:              %s\n' "$( awk '/^pub struct Tick/{f=1} f && /^}$/{exit} f' src/lib.rs | command grep -oE '^  [a-z_]+ :' | tr -d ' :' | tr '\n' ' ' )"
printf 'sites mutating moved:     %s\n' "$( command grep -c 'self.moved +=' src/lib.rs || true )"
printf 'checked or saturating:    %s\n' "$( command grep -cE 'checked_add|saturating_add|wrapping_add' src/lib.rs || true )"
printf 'methods taking &mut self: %s\n' "$( command grep -c '&mut self' src/lib.rs || true )"
printf 'overflow-checks in repo:  %s\n' "$( cd ../.. && command grep -rc 'overflow-checks' Cargo.toml 2>/dev/null || true )"
printf 'tests naming Tick:        %s\n' "$( command grep -c 'Tick::' tests/poll_test.rs || true )"
printf 'tests naming Copy:        %s\n' "$( command grep -c 'Copy' tests/poll_test.rs || true )"
printf 'types deriving Copy:      %s\n' "$( command grep -cE '^#\[ derive\(.*Copy' src/lib.rs || true )"
```

Live output:

```
Tick derives:             Debug, Clone
Tick doc on the omission: # Deliberately Not Copy
Tick fields:              budget moved lost 
sites mutating moved:     4
checked or saturating:    1
methods taking &mut self: 6
overflow-checks in repo:  0
tests naming Tick:        15
tests naming Copy:        2
types deriving Copy:      2
```

### Data Structures

| File | Relationship |
|------|--------------|
| [001_three_values_three_shapes.md](001_three_values_three_shapes.md) | The other two types, and the trait table this one sits in |

### Types

| File | Relationship |
|------|--------------|
| [`../type/002_progress_has_no_zero_made.md`](../type/002_progress_has_no_zero_made.md) | What `progress()` returns from the counter this file is about |

### Algorithms

| File | Relationship |
|------|--------------|
| [`../algorithm/002_what_an_attempt_costs.md`](../algorithm/002_what_an_attempt_costs.md) | The other thing the counter does not count |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | The declaration and all four increment sites |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/poll_test.rs`](../../tests/poll_test.rs) | C1, C2 |

### PL11 — an accumulator that duplicates silently

`Tick` derived `Copy`, so `let snapshot = tick;` was a copy and both bindings
stayed live. From that point the two accumulated separately: operations on `tick`
did not appear in `snapshot.progress()`, and the compiler said nothing, because
nothing was wrong — that is what `Copy` means.

The hazard is that `Tick` reads like a handle. It is passed to methods that
mutate it, it has an accessor for its accumulated state, and the name describes
a unit of work rather than a value. A caller who wrote
`fn run_systems( tick : Tick, … )` instead of `&mut Tick` got a program that
compiled, ran, moved records correctly, and reported zero progress — the moves
landed in a copy that was dropped at the end of the call.

That failure is silent in the specific way that matters here: the records really
do move, the ring really does drain, and only the *reporting* is lost. A
scheduler using `Progress` to decide whether to keep spinning would see `None`
after a productive tick and back off exactly when it should not.

Nothing in the crate mentioned it. `Tick`'s doc comment described the accounting
it adds and never the shape it had, and the suite had two `Tick` tests, both
using one binding throughout. The derive was three words in a line the compiler
checks; its consequence for a mutable accumulator was unchecked and undocumented.

`&mut Tick` in a signature is the whole fix on the caller's side. What the caller
could not do was get the compiler to insist on it, because the type consented to
being copied. Dropping `Copy` moves that insistence into the type, at the cost of
making `Tick` move where it used to duplicate — a real cost, paid deliberately,
and the reason the derive line is now the subject of a doc section rather than
three words nobody reads.

**Disposition:** applied — `Copy` was dropped from `Tick`'s derive in
`src/lib.rs`, leaving `#[ derive( Debug, Clone ) ]`, so `fn f( t : Tick )` now
moves the tick and a caller who passes it by value cannot go on using it. A
`# Deliberately Not Copy` section on the type states why, names `Clone` as the
explicit escape hatch for a caller who really does want a second accumulator, and
records that `Budget` keeps `Copy` because a write-once value has no drift to
suffer. `a_cloned_tick_accumulates_separately` in `tests/poll_test.rs` pins the
remaining behaviour: a cloned tick and its origin count independently, which is
correct and, unlike before, only reachable by writing `.clone()`. Now prints: `# Deliberately Not Copy`

### PL12 — the guarded field and the unguarded one sit in the same struct

`Budget` exists as a newtype so that a nonsensical value cannot be held: the
field is private, `new` clamps zero to one, and there is no other constructor.
The type spends a whole declaration on making one `usize` trustworthy.

`moved` is the other `usize` in the same struct and has none of that. It is
incremented with bare `+=` at four sites, with no checked, saturating or
wrapping form anywhere in the file, and the workspace manifest sets
`overflow-checks` nowhere — so a debug build panics on overflow and a release
build wraps.

The practical risk is nil. Overflowing a `usize` counter requires moving about
1.8 × 10¹⁹ records through a single tick, and a tick is a per-frame object. This
is recorded for the shape rather than the number: the crate demonstrably knows
how to make a `usize` field safe by construction, applies that treatment to the
input, and leaves the output as a raw counter. If a future change makes `Tick`
long-lived — a per-connection or per-session accumulator rather than a per-frame
one — the reasoning that makes the bare `+=` fine today stops holding, and
nothing in the type records that the reasoning was ever load-bearing.

`saturating_add` would cost nothing and change no behaviour reachable today.
Recording why it is absent is the cheaper half, and is what this finding does.

**Correction (2026-09-28):** "no checked, saturating or wrapping form anywhere
in the file" is no longer accurate as a whole-file claim. `Progress::then` now
routes through `Self::of( self.count().saturating_add( other.count() ) )`
([`../type/002`](../type/002_progress_has_no_zero_made.md)), so the census this
finding's Regenerate block runs — a file-wide grep for
`checked_add|saturating_add|wrapping_add` — reads 1, not 0. That addition is
unrelated to the field this finding is about: `moved` and `lost` are still
incremented with bare `+=` at every site (`self.moved += 1`, `self.moved +=
moved`, `self.lost += offered - moved`), so the guarded-field-versus-unguarded-
field contrast the finding draws, and the risk it records, are both unchanged.
