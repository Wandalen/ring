# Invariant: The Frontier Never Exceeds a Dependency

### Scope

- **Purpose**: State the crate's one safety property precisely enough to say what it does *not* claim, and show what actually enforces it.
- **Responsibility**: Give the property, the non-atomic read that complicates it, the direction the staleness runs, and the test that can fail.
- **In Scope**: What `frontier` and `available` guarantee about their answers.
- **Out of Scope**: What they cost — see [`non_functional_requirement/001`](../non_functional_requirement/001_every_frontier_read_allocates_nothing.md).

### The Property

> Whatever `frontier()` reports, every dependency had at least reached it.

If it were ever violated, a consumer would read a slot the producing dependency
has not finished writing — a data race, and the failure this whole family exists
to make impossible.

### What the Property Is Not

The read is not atomic across dependencies. `ring_cursor::slowest` loads each
cursor in turn:

```rust
// ring_cursor::slowest in ring_cursor/src/lib.rs
cursors.iter().map( | c | c.load( GATING ) ).min()
```

`min()` short-circuits nothing — it drives the iterator to exhaustion — so for
a barrier over three dependencies the answer is a minimum over three positions
read at three different instants. There is **no instant at which the
set was collectively in the reported state.** A stronger-sounding property —
"the frontier is the minimum of the dependencies' positions *at one moment*" —
is false, and no amount of ordering strength would make it true.

The correct statement is per-dependency and one-directional:

| For each dependency `i` | Holds |
|-------------------------|-------|
| There was an instant during the read at which `i` was at `p_i` | ✔ |
| `frontier = min p_i` | ✔ |
| Cursors only advance | ✔ (nothing in the family ever stores backwards) |
| Therefore at any later instant, `i >= p_i >= frontier` | ✔ |
| Therefore `frontier` is behind or level with every dependency, never ahead | **the property** |

The staleness runs in the safe direction, and that is the whole guarantee: a
barrier can under-report progress, never over-report it. Under-reporting costs
throughput and nothing else — the consumer comes back and asks again.

This is also why `wait_for`'s second read is sound
([`algorithm/002`](../algorithm/002_wait_for_asks_twice.md)): re-reading after a
successful wait can only return a value at or ahead of the one that satisfied
the predicate, never behind it.

### What Enforces It

| Enforcer | Where | Would break it |
|----------|-------|----------------|
| `Ordering::Acquire` on every load | `ring_cursor::GATING` | `Relaxed` — reports a position the consumer has no happens-before edge to |
| A minimum, not a first element | `ring_seqno::slowest` → `.min()` | `cursors[ 0 ]` — reports the fastest when it happens to be first |
| `Option`, not an identity | `slowest` returning `None` | `Seq::MAX` for an empty set — reports the entire sequence space |
| No caching | this crate stores no frontier | A cached value outlives its evidence |

None of the first three is in this crate. The whole property is delegated, and
the crate's contribution is to not undo it — which is what makes the manual
check on delegation worth having
([`workaround/001`](../workaround/001_the_check_that_capacity_stays_out.md)).

### BR15 — The Test Can Actually Fail

```rust
// tests/barrier_test.rs:372-398
const TOTAL : u64 = 2_000;
let deps = deps_zeroed( 2 );
let barrier = Barrier::over( &deps );

// one thread drives deps[ 0 ] from 1 to 2_000; deps[ 1 ] never moves
for _ in 0..20_000
{
  let frontier = barrier.frontier().expect( "two dependencies" );
  assert_eq!( frontier, Seq::ZERO, "cursor 1 never moved, so it is always the minimum" );
}
```

`assert_eq!( frontier, Seq::ZERO )` — an *exact* equality, checked twenty
thousand times against a cursor being driven concurrently through two thousand
positions.

| Defect | This test |
|--------|-----------|
| `slowest` reads `cursors[ 0 ]` | Fails — reports a moving position |
| `slowest` takes a maximum | Fails — reports up to 2000 |
| An off-by-one anywhere in the fold | Fails |
| A cached frontier | Fails, or passes by luck on the first read |
| `slowest` reads the *last* element | **Passes** — with two cursors the minimum is at index 1 |
| `Relaxed` instead of `Acquire` | **Not caught** — see below |

The fifth row is a real gap, covered elsewhere:
`every_set_size_from_one_to_eight_folds_to_the_minimum` places the minimum at
every index of every size from one to eight, so a last-element fold fails 28 of
its 36 cases — sequentially. Under concurrency, only index 1 is exercised.

The sixth is the honest limit: the ordering cannot be falsified by a
single-machine test on x86, where the hardware supplies the acquire semantics
the code failed to ask for. The comment in the test says so, and the family's
answer to it is `loom` in `ring_publish/tests/handshake_test.rs`, not another
assertion here.

**Every row above depends on the spawn**, which is what manual check B6 exists
to protect. Delete `scope.spawn` and `deps[ 0 ]` never moves either — both
cursors sit at `Seq::ZERO`, every candidate fold returns `Seq::ZERO`, and the
assertion passes against all of them. The test's whole discriminating power is
that one dependency is being driven while the other is not; without the writer
it is a tautology with a green tick, which is the same failure mode as the
producer half's `headroom <= CAPACITY`.

**Why BR15 is worth naming:** the producer half's equivalent test asserts
`headroom <= CAPACITY`, which is true by construction for every input —
`capacity.saturating_sub( … )` cannot exceed `capacity` — so it cannot fail
([`ring_gating`'s NFR 002](../../../ring_gating/docs/non_functional_requirement/002_the_gate_must_never_over_report.md) § G12).
The two crates guard the same class of property, and only one of them does it
with an instrument that can register a defect.

The same file contains the weaker shape too, one test earlier:

```rust
// tests/barrier_test.rs:363
assert!( frontier.0 <= TOTAL, "read past what was ever published" );
```

That catches invention — a frontier ahead of anything ever stored — and cannot
catch lag, a wrong minimum, or a stale cache. It is not wrong to have it; it is
worth knowing which of the two assertions is carrying the property.

### BR35 — The Saturation This Crate Depends On Is Documented as Something the Caller Has Already Ruled Out

`Seq::distance_to` saturates, and `ring_types` explains why in a sentence that
describes a caller this crate is not: *"the caller that needs the direction has
already compared the two, and every caller that does not wants a count."*

The sentence describes two kinds of caller, and every one of the family's eleven
call sites is the second kind. Not one compares the two sequences before calling
— `ring_seqno::has_room` comes closest and compares the *result* against a
capacity, which establishes nothing about direction. So the saturation's
justification rests entirely on the first category being empty.

`Barrier::available` is what makes that load-bearing. It hands both sequences to
`distance_to` and returns whatever comes back, so a `from` past the frontier
yields `0` — the same answer as a `from` exactly at it, in the one crate where
the difference between those two states is the whole point.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A10 -F '  /// How many publications separate this sequence from a later one, or `0`' ring_types/src/id.rs
# every call site, and the one that compares before calling
command grep -r "distance_to" --include=*.rs ring_*/src/ | grep -v ring_types | sed 's|ring/||'
```

Live output:

```
    /// How many publications separate this sequence from a later one, or `0`
    /// when `later` is not actually later.
    ///
    /// Saturating rather than signed: the caller that needs the direction has
    /// already compared the two, and every caller that does not wants a count.
    ///
    /// ```
    /// use ring_types::Seq;
    /// assert_eq!(Seq(4).distance_to(Seq(10)), 6);
    /// assert_eq!(Seq(10).distance_to(Seq(4)), 0);
    /// ```
ring_barrier/src/lib.rs:        self.frontier().map_or(0, |frontier| from.distance_to(frontier))
ring_mpsc/src/lib.rs:        from.distance_to(end) as usize
ring_mpsc/src/lib.rs:        Batch { ring: self.ring, start, len: start.distance_to(end) as usize }
ring_seqno/src/lib.rs:    earlier.distance_to(later) / capacity.get() as u64
ring_seqno/src/lib.rs:    consumer.distance_to(producer) < capacity.get() as u64
ring_seqno/src/lib.rs:    let in_flight = consumer.distance_to(producer);
ring_seqno/src/lib.rs:    consumer.distance_to(producer)
ring_spsc/src/lib.rs:    /// cannot itself underflow (`Seq::distance_to` is `saturating_sub`), but if
ring_spsc/src/lib.rs:        consumed.distance_to(produced)
ring_spsc/src/lib.rs:        consumed.distance_to(produced) as usize
ring_spsc/src/lib.rs:        Batch { ring: self.ring, start, len: start.distance_to(produced) as usize }
ring_spsc/src/lib.rs:        Batch { ring: self.ring, start, len: max.min(start.distance_to(produced) as usize) }
```

### BR36 — A Consumer Past Its Barrier Is Indistinguishable From One Exactly At It

Both answer `available == 0`. One is a consumer that has read everything its
dependencies published; the other has read past them, which is the precise
condition this crate exists to prevent and the one state a barrier should be
able to name.

The doctest asserts both — `available( Seq( 4 ) ) == 0, "caught up"` and
`available( Seq( 9 ) ) == 0, "and never negative"` — on a frontier of 4, side by
side, with the second labelled as a property rather than as a state that should
never occur. The invariant is enforced upstream, by consumers only ever
advancing to a frontier they were given, and this crate cannot check it: the
saturation in `distance_to` (BR35) erases the evidence before `available`
returns.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A15 -F '  /// for why that is not the same answer `ring_gating` gives an empty set.' ring_barrier/src/lib.rs | tail -n 8
# the test asserts the same collapse
grep -A4 "fn available_is_zero_at_and_past_the_frontier" ring_barrier/tests/barrier_test.rs
```

Live output:

```
  /// let cursors = [ PaddedCursor::default() ];
  /// cursors[ 0 ].store( Seq( 4 ), Ordering::Release );
  ///
  /// let barrier = Barrier::over( &cursors );
  /// assert_eq!( barrier.available( Seq::ZERO ), 4 );
  /// assert_eq!( barrier.available( Seq( 4 ) ), 0, "caught up" );
  /// assert_eq!( barrier.available( Seq( 9 ) ), 0, "and never negative" );
  /// ```
fn available_is_zero_at_and_past_the_frontier()
{
  // Past the frontier is a state a correct consumer never reaches, but the
  // arithmetic must saturate rather than wrap into a huge count — a consumer
  // acting on a wrapped count reads slots that were never published.
```

### Invariants

| File | Relationship |
|------|--------------|
| [002_capacity_never_enters_the_arithmetic.md](002_capacity_never_enters_the_arithmetic.md) | The crate's other property, stated as an absence |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_the_frontier_in_two_delegations.md](../algorithm/001_the_frontier_in_two_delegations.md) | The chain the enforcers live along |
| [../algorithm/002_wait_for_asks_twice.md](../algorithm/002_wait_for_asks_twice.md) | Why the second read is sound |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_three_barrier_readings.md](../item/001_the_three_barrier_readings.md) | The three methods this property covers |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_a_non_blocking_wait_must_look_exactly_once.md](../non_functional_requirement/002_a_non_blocking_wait_must_look_exactly_once.md) | The correctness-under-concurrency requirement this satisfies |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_the_two_empty_answers_look_like_a_bug.md](../pitfall/001_the_two_empty_answers_look_like_a_bug.md) | The one case where "behind, never ahead" is not the useful reading |

### Workarounds

| File | Relationship |
|------|--------------|
| [../workaround/001_the_check_that_capacity_stays_out.md](../workaround/001_the_check_that_capacity_stays_out.md) | The instrument guarding what the compiler cannot |

### Sources

| File | Relationship |
|------|--------------|
| `ring_cursor/src/lib.rs:120-123` | The non-atomic per-cursor read |
| `ring_cursor/src/lib.rs:89` | `GATING = Ordering::Acquire` |
| `ring_seqno/src/lib.rs` | `slowest` — `.min()`, and the `None` |

### Tests

| File | Relationship |
|------|--------------|
| `tests/manual/readme.md` § B6 | The check that keeps the spawn, without which the safety test is a tautology |
| `tests/barrier_test.rs:372-398` | The exact-equality safety test |
| `tests/barrier_test.rs:341-370` | The weaker `<=` bound, alongside it |
| `tests/barrier_test.rs:97-117` | The fold at every size from one to eight, minimum at every index |
| `tests/barrier_test.rs:119-128` | A single lagging dependency holding the whole barrier |
