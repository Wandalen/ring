# Pattern: The Quantity, the Predicate, and the Wait

### Scope

- **Purpose**: Show the three-rung ladder every gating type in the family climbs, where this crate's third rung diverges from its sibling's, and what the census says about whether any third rung is used.
- **Responsibility**: Give the ladder, the four third rungs the family actually shipped, the naming spread across the two shared rungs, and the caller counts for both.
- **In Scope**: The API shape of asking whether one may proceed.
- **Out of Scope**: `ring_gating`'s side of the divergence as it argues it — see [`ring_gating`'s pattern/002](../../../ring_gating/docs/pattern/002_the_predicate_the_quantity_and_the_reason.md).

### The Ladder

The rationale is written down once, on the method that first needed a second
rung:

> Exactly [`free_slots`] being non-zero, expressed as the question a caller
> actually asks. Kept as its own method because the two readings answer
> different questions — "how much room" is a batch-sizing input, "may I" is a
> branch — and a caller that only needs the branch should not have to know that
> zero is the boundary.
>
> — `ring_cursor/src/lib.rs:395-399`, on `CursorPair::may_claim`

| Rung | Question | Returns | For |
|------|----------|---------|-----|
| **Quantity** | *How many?* | a count | Sizing a batch |
| **Predicate** | *May I take this many?* | `bool` | A branch |
| **Third** | *…and if not, then what?* | — | Control flow |

The first two rungs are stable across the family. The third is where the three
gating types stop agreeing — and it is the whole of the `ring_gating`/`ring_barrier` divergence.

### The First Two Rungs, Three Times

| Type | Quantity | Predicate |
|------|----------|-----------|
| `CursorPair` | `free_slots() -> usize` (`ring_cursor:368`)<br>`pending() -> u64` (`:387`) | `may_claim() -> bool` (`:417`) |
| `GatingSet` | `headroom( producer ) -> usize` (`ring_gating:222`) | `admits( producer, count ) -> bool` (`:242`) |
| **`Barrier`** | **`available( from ) -> u64`** (`:216`) | **`admits( from, count ) -> bool`** (`:238`) |

Same shape, three vocabularies. Which brings the naming census.

### BR11 — One Rung, Three Names; One Name, Three Rungs

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r 'pub fn available' ring_*/src/*.rs   # 5 (4, plus available_up_to)
grep -r 'fn admits'        ring_*/src/*.rs   # 3
grep -r 'pub fn frontier'  ring_*/src/*.rs   # 1
```

Live output:

```
ring_barrier/src/lib.rs:  pub fn available( &self, from : Seq ) -> u64
ring_consume/src/lib.rs:  pub fn available( &self ) -> Available
ring_consume/src/lib.rs:  pub fn available_up_to( &self, max : u64 ) -> Available
ring_mpsc/src/lib.rs:  pub fn available( &self ) -> usize
ring_spsc/src/lib.rs:  pub fn available( &self ) -> usize
ring_barrier/src/lib.rs:  pub fn admits( &self, from : Seq, count : u64 ) -> bool
ring_bench/src/lib.rs:  pub const fn admits( self, producers : usize ) -> bool
ring_gating/src/lib.rs:  pub fn admits( &self, producer : Seq, count : usize ) -> bool
ring_barrier/src/lib.rs:  pub fn frontier( &self ) -> Option< Seq >
```

| Name | Definitions | Return types |
|------|------------:|--------------|
| `available` | 4 | `u64` (this crate), `Available` (`ring_consume:336`), `usize` (`ring_mpsc:1051`, `ring_spsc:856`) |
| `admits` | 3 | `bool` ×3 — but `ring_bench:522`'s is `Candidate::admits( producers )`, an unrelated question wearing the same word |
| `frontier` | **1** | `Option< Seq >` — this crate's, and nowhere else's |

So the quantity rung is the family's most-overloaded word and the fold under it
is the family's least: `available` means four things, `frontier` means one. A
reader who has seen `consumer.available()` return an `Available` run and then
reads `barrier.available( from )` returning a bare `u64` has met the same word
twice with different arity, different types, and different meaning — which is
why [`item/001`](../item/001_the_three_barrier_readings.md) insists on naming
the three barrier readings rather than leaning on the words.

### The Third Rung: Four Shipped, Three Placements

| Type | Third rung | Adds | Signature | Lives |
|------|-----------|------|-----------|-------|
| `CursorPair` | `for_space` | a **wait** | `( &CursorPair, WaitKind, usize ) -> Result< usize, RingError >` | `ring_wait:239` — *another crate*, a free function |
| `CursorPair` | `for_data` | a **wait** | `( &CursorPair, u64, WaitKind, usize ) -> Result< usize, RingError >` | `ring_wait:265` — same |
| `GatingSet` | `check` | a **reason** | `( &self, Seq, usize ) -> Result< (), RingError >` | `ring_gating:265` — on the type |
| **`Barrier`** | **`wait_for`** | a **wait** | `( &self, Seq, u64, WaitKind, usize ) -> Result< Seq, RingError >` | `ring_barrier:282` — on the type |

Three things are worth reading off that table.

**The wait already existed.** `ring_wait::for_data( pair, count, kind, spins )`
is `Barrier::wait_for( from, count, kind, spins )` with the receiver moved into
the argument list. This crate did not invent the fourth-argument wait shape; it
re-sited it. The one substantive difference is the fold underneath —
`for_data` compares against a `CursorPair`'s single consumer, `wait_for` against
a slice's minimum.

**Three return types for one rung.** `Result< usize >` returns the attempt count;
`Result< () >` returns nothing but the error; `Result< Seq >` returns the
frontier and **throws the attempt count away** — this crate is the only
`wait_until` caller in the family that discards it
([`algorithm/002`](../algorithm/002_wait_for_asks_twice.md) § BR5). Each choice
is locally defensible and no two agree.

**Two placements for one idea.** `CursorPair`'s wait lives in a downstream crate
because `ring_cursor` does not depend on `ring_wait`; `Barrier`'s lives on the
type because `ring_barrier` does. The placement follows the dependency edge, not
the design — which is what [`integration/002`](../integration/002_the_dependency_that_is_not_ring_seqno.md)
is about from the other direction.

### BR20 — No Third Rung Has a Production Caller

Doctests and definition lines excluded; `src/` counts are real code:

```sh
cd "$(git rev-parse --show-toplevel)"
for m in for_space for_data check wait_for; do
  echo "-- $m --"
  for f in ring_*/src/lib.rs ring_*/tests/*.rs; do
    n=$( command grep -vE '^\s*(///|//!|//)' "$f" | command grep -vE '^\s*pub fn' | command grep -cE "\b$m\(" )
    [ "$n" != 0 ] && echo "   $n  $f" || true
  done
done
```

Live output:

```
-- for_space --
   4  ring_wait/tests/wait_test.rs
-- for_data --
   8  ring_wait/tests/wait_test.rs
-- check --
   16  ring_debug/tests/debug_test.rs
   8  ring_gating/tests/gating_test.rs
-- wait_for --
   4  ring_barrier/tests/allocation_test.rs
   8  ring_barrier/tests/barrier_test.rs
```

`check` is two functions, not one: the sixteen in `ring_debug` are
`ring_debug::check`, a free function over a `CursorPair` that has nothing to do
with `GatingSet`. Only the eight in `ring_gating` belong to the rung below. The
census reports per file rather than per name for exactly this reason — a bare
name is not a function, and a total that assumed it was would read 24 callers for
this rung's four names and be wrong about half of them.

| Third rung | Callers in any `src/` | Callers in tests |
|------------|----------------------:|------------------|
| `ring_wait::for_space` | **0** | 4 — all `ring_wait/tests/wait_test.rs` |
| `ring_wait::for_data` | **0** | 6 — all `ring_wait/tests/wait_test.rs` |
| `GatingSet::check` | **0** | 8 — all `ring_gating/tests/gating_test.rs` |
| `Barrier::wait_for` | **0** | 10 — `barrier_test.rs` ×6, `allocation_test.rs` ×4 |
| | **0** | **28** |

Four third rungs, twenty-eight callers, every one of them a test of the rung
itself. Nothing in the family climbs past the second rung in production. The
four newest are `allocation_test.rs`'s, which call the wait to weigh it rather
than to wait on anything — so the rung is now measured as well as exercised,
and still not reached from a library.

The first two rungs are not like that. These counts are by *name* — `grep` cannot
tell `GatingSet::headroom` from `ring_claim`'s own, or `Barrier::available` from
`Consumer::available` — so read them as "this word is reached from production
code", which is exactly the contrast being drawn:

| Rung | Name | `src/` call sites | Where |
|------|------|------------------:|-------|
| Quantity | `.headroom(` | 6 | `ring_claim` ×3, `ring_gating` ×2, `ring_mpsc` ×1 |
| Quantity | `.available(` | 7 | `ring_consume` ×3, `ring_core` ×2, `ring_mpsc` ×1, this crate ×1 |
| Quantity | `.pending(` | 1 | `ring_wait:241`, inside `for_data` |
| Quantity | `.free_slots(` | **0** | — |
| Predicate | `.may_claim(` | 2 | `ring_shutdown:620`, `ring_wait:241` |
| Predicate | `.admits(` | 1 | this crate, inside `wait_for` |

So the ladder in practice is: **quantity is load-bearing, predicate is thin, the
third rung is unreached.** And the one `.available(` site that is unambiguously
this crate's is `Barrier::admits`'s body — whose own caller is `wait_for`, whose
own callers are six tests and four measurements ([`item/001`](../item/001_the_three_barrier_readings.md)
§ BR3). The entire top of this crate's ladder hangs from the test suite.

That is a finding about the family, not a complaint about this crate. Four
independent designs each grew a third rung, each shipped it, and none of them
found the caller that would have settled its return type. When one does, the
three answers will have to be reconciled — and the version with a caller wins,
not the version that was written first.

### Why the Divergence Was Not Arbitrary

`ring_gating` is the producer's half and `ring_barrier` the consumer's, and the
two ends fail differently:

| | Producer, out of room | Consumer, out of data |
|--|----------------------|-----------------------|
| Error | `RingError::Full` | `RingError::Empty` |
| Useful response | back-pressure — drop, block, or grow | wait — the data is coming |
| So the third rung gives | a **reason** to route on | a **wait** to sit in |

A producer that waits for room is stalling the thing that produces the room's
consumer; a consumer that waits for data is doing exactly what it exists to do.
The asymmetry in `ring_wait` itself says the same thing out loud — `for_space`
remaps its error to `Full` while `for_data` leaves it `Empty`, with a doc comment
explaining that a producer handed `Empty` would read it as "nothing to do"
rather than "back-pressure" (`ring_wait:221-224`).

### BR45 — The Family Writes This Three-Step Shape Four Times and Names the Steps Differently Each Time

Quantity, predicate over that quantity, wait on that predicate. `ring_barrier`
spells it `available` / `admits` / `wait_for`. `ring_gating` spells it
`headroom` / `admits` / `check`. `ring_wait` supplies the third step generically
as `wait_until` and calls its own bounded form `wait`. `ring_consume` re-derives
the first two rather than importing them.

`admits` is the one name two crates agree on, and its two signatures differ:
`Barrier::admits( from, count )` and `GatingSet::admits` take different
parameters for different questions. Four instances of one shape, one shared
name, zero shared code above the fold.

```sh
cd "$(git rev-parse --show-toplevel)"
grep -rE "pub fn (available|headroom|admits|check|wait_for|wait_until)\b" \
  --include=*.rs ring_*/src/ | sed 's|ring/||; s|\.rs:|.rs:  |'
```

Live output:

```
ring_barrier/src/lib.rs:    pub fn available( &self, from : Seq ) -> u64
ring_barrier/src/lib.rs:    pub fn admits( &self, from : Seq, count : u64 ) -> bool
ring_barrier/src/lib.rs:    pub fn wait_for( &self, from : Seq, count : u64, kind : WaitKind, spins : usize )
ring_claim/src/lib.rs:    pub fn headroom( &self ) -> usize
ring_consume/src/lib.rs:    pub fn available( &self ) -> Available
ring_debug/src/lib.rs:  pub fn check( pair : &CursorPair ) -> Result< (), Violation >
ring_gating/src/lib.rs:    pub fn headroom( &self, producer : Seq ) -> usize
ring_gating/src/lib.rs:    pub fn admits( &self, producer : Seq, count : usize ) -> bool
ring_gating/src/lib.rs:    pub fn check( &self, producer : Seq, count : usize ) -> Result< (), RingError >
ring_mpsc/src/lib.rs:    pub fn available( &self ) -> usize
ring_spsc/src/lib.rs:    pub fn available( &self ) -> usize
ring_wait/src/lib.rs:  pub fn wait_until< F >( kind : WaitKind, spins : usize, mut ready : F ) -> Result< usize, RingError >
```

### Patterns

| File | Relationship |
|------|--------------|
| [001_the_borrowed_view_and_the_owned_set.md](001_the_borrowed_view_and_the_owned_set.md) | The ownership shape these methods sit on |
| [`ring_gating`'s pattern/002](../../../ring_gating/docs/pattern/002_the_predicate_the_quantity_and_the_reason.md) | The reason-rung half of the divergence |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_wait_for_asks_twice.md](../algorithm/002_wait_for_asks_twice.md) | The third rung's two phases, and the count it discards |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_nine_methods_over_one_borrowed_slice.md](../api/001_nine_methods_over_one_borrowed_slice.md) | The nine methods, grouped by the tier they serve |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_the_dependency_that_is_not_ring_seqno.md](../integration/002_the_dependency_that_is_not_ring_seqno.md) | The edge that lets the wait live on the type |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_three_barrier_readings.md](../item/001_the_three_barrier_readings.md) | BR3 — the caller chain under `available` |
| [../item/002_the_five_accessors_and_the_wait.md](../item/002_the_five_accessors_and_the_wait.md) | BR4 — where `wait_for`'s six callers are |

### Sources

| File | Relationship |
|------|--------------|
| `ring_cursor/src/lib.rs:370,419,395-399` | The two base rungs, and the rationale |
| `ring_wait/src/lib.rs:217-269` | Both `CursorPair` waits, and the error asymmetry |
| `ring_gating/src/lib.rs:222,242,283` | The reason rung |
| `ring_barrier/src/lib.rs:216,238,282` | The wait rung |

### Tests

| File | Relationship |
|------|--------------|
| `tests/barrier_test.rs` | 3 `admits`, 8 `available`, 6 `wait_for` |
| `ring_wait/tests/wait_test.rs` | The only `for_space` / `for_data` callers |
| `ring_gating/tests/gating_test.rs` | The only `check` callers |
