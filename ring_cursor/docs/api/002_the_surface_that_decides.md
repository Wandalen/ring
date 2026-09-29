# API: The Surface That Decides

### Scope

- **Purpose**: List the half of the surface where the crate chooses the memory ordering on the caller's behalf, and separate it from the items that name no ordering at all.
- **Responsibility**: Give each item's signature, mark which fix `GATING` and which are ordering-free, and state the obligation the fixing creates.
- **In Scope**: `GATING`; `slowest`; `CursorPair`'s eight associated functions.
- **Out of Scope**: The half that forwards, which is [`api/001`](001_the_surface_that_forwards.md); why the ordering is fixed, which is [`decisions/001`](../decisions/001_gating_is_fixed_not_a_parameter.md).

### The Items

| Item | Signature | Ordering |
|------|-----------|----------|
| `GATING` | `pub const GATING : Ordering` | **is** the decision |
| `slowest` | `pub fn slowest( cursors : &[ PaddedCursor ] ) -> Option< Seq >` | fixes `GATING`, once per cursor |
| `CursorPair::free_slots` | `pub fn free_slots( &self ) -> usize` | fixes `GATING`, twice |
| `CursorPair::pending` | `pub fn pending( &self ) -> u64` | fixes `GATING`, twice |
| `CursorPair::may_claim` | `pub fn may_claim( &self ) -> bool` | fixes `GATING`, twice |
| `CursorPair::new` | `pub const fn new( capacity : Capacity ) -> Self` | none — construction |
| `CursorPair::producer` | `pub const fn producer( &self ) -> &PaddedCursor` | none — hands back a cell the caller then chooses for |
| `CursorPair::consumer` | `pub const fn consumer( &self ) -> &PaddedCursor` | none — same |
| `CursorPair::capacity` | `pub const fn capacity( &self ) -> Capacity` | none — a field read |
| `CursorPair::on_distinct_lines` | `pub fn on_distinct_lines( &self ) -> bool` | none — reads addresses, not values |

**Four items fix an ordering; six name none.** The heading is about where the
decision *can* be made, not about how many items make it.

### The Accessors Reopen the Choice

`producer()` and `consumer()` hand back `&PaddedCursor`, and a `PaddedCursor` is
[the forwarding surface](001_the_surface_that_forwards.md). So a caller holding a
`CursorPair` can bypass the fixed ordering entirely:

```rust
pair.free_slots()                                  // GATING, fixed
pair.producer().load( Ordering::Relaxed )          // whatever the caller likes
```

This is not a leak — it is the design. Every consumer in the family uses it:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -r 'cursors\.\(producer\|consumer\)()\.load(' --include=*.rs ring_spsc/src/ \
  | sed -E 's/:/: /' | LC_ALL=C sort
```

Live output:

```
ring_spsc/src/lib.rs:       .field( "consumed", &self.cursors.consumer().load( GATING ) )
ring_spsc/src/lib.rs:       .field( "produced", &self.cursors.producer().load( GATING ) )
ring_spsc/src/lib.rs:     let consumed = self.ring.cursors.consumer().load( GATING );
ring_spsc/src/lib.rs:     let consumed = self.ring.cursors.consumer().load( OWN );
ring_spsc/src/lib.rs:     let produced = self.ring.cursors.producer().load( GATING );
ring_spsc/src/lib.rs:     let produced = self.ring.cursors.producer().load( GATING );
ring_spsc/src/lib.rs:     let produced = self.ring.cursors.producer().load( GATING );
ring_spsc/src/lib.rs:     let produced = self.ring.cursors.producer().load( OWN );
ring_spsc/src/lib.rs:     let seq = self.ring.cursors.producer().load( OWN );
ring_spsc/src/lib.rs:     let start = self.ring.cursors.consumer().load( OWN );
ring_spsc/src/lib.rs:     let start = self.ring.cursors.consumer().load( OWN );
ring_spsc/src/lib.rs:     self.ring.cursors.consumer().load( OWN )
ring_spsc/src/lib.rs:     self.ring.cursors.consumer().load( OWN ) == self.ring.cursors.producer().load( GATING )
ring_spsc/src/lib.rs:     self.ring.cursors.producer().load( OWN )
```

```rust
self.ring.cursors.consumer().load( OWN ) == self.ring.cursors.producer().load( GATING )
```

**Two orderings on two cursors of one pair, in one expression.** `ring_spsc`
reads its *own* cursor at `OWN` (`Relaxed`, sound because it is that cursor's
sole writer) and the *peer's* at `GATING` (`Acquire`, load-bearing). That line is
only expressible because the accessors impose nothing. The pair fixes the
ordering for the questions it answers itself, and declines to fix it for the
cursors it lends out.

**The fixed/free boundary is therefore about questions, not about types.** A
question whose answer authorises overwriting a slot is fixed; handing back a cell
is not a question.

### The Obligation Fixing Creates

Once `free_slots()` promises `Acquire`, three things follow:

| # | Obligation | Held by |
|---|-----------|---------|
| S1 | The ordering must be documented where a caller will find it | `src/lib.rs:38-51`, and each reading's own doc comment refers to it |
| S2 | It must be one ordering, named once — not `Acquire` written three times | `tests/manual/readme.md` M3, which reads the source and expects exactly one non-doc `Ordering::Acquire` |
| S3 | It must not be silently upgraded later | Nothing. No test would notice `GATING` becoming `SeqCst` — the four external value assertions would fail, which is the only alarm |

S3 is the weakest, and it is weak in the safe direction: an upgrade is sound and
slow, and the cost would surface in `ring_bench` rather than in a correctness
failure. The dangerous direction — a *downgrade* — trips the same four
assertions.

### What the Empty Cases Answer

Two items on this surface have a degenerate input, and they answer differently:

| Item | Degenerate input | Answers |
|------|------------------|---------|
| `slowest` | an empty slice | `None` — because its two callers resolve "no cursors" to opposite values |
| `CursorPair::free_slots` | a fresh pair | the full capacity — there is no empty case; a pair always has two cursors |

`slowest`'s `None` is the more consequential of the two and is argued in
[`algorithm/001`](../algorithm/001_the_slowest_fold.md) § `None` Rather Than
`Seq::ZERO`.

### CU7 — The Deciding Surface Hands Back the Choice It Made

```rust
pub const fn producer( &self ) -> &PaddedCursor
pub const fn consumer( &self ) -> &PaddedCursor
```

`CursorPair` fixes `GATING` for its own three readings — that is
[`decisions/001`](../decisions/001_gating_is_fixed_not_a_parameter.md)'s whole
subject. Both accessors then return a `&PaddedCursor`, whose `SeqCell` impl
forwards whatever ordering the caller names.

**Finding.** `ring_spsc` uses exactly that route to read a cursor at an ordering
other than `GATING`, through this crate's own accessor. The decision holds for
the three readings and is undone for anyone who asks for the cursor itself, which
is not a defect so much as an unstated boundary: the pair decides for its
questions, not for its fields.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A3 -F '  /// `GATING` is fixed only for the pair'"'"'s own three readings' ring_cursor/src/lib.rs
```

Live output:

```
  /// `GATING` is fixed only for the pair's own three readings
  /// (`free_slots`, `pending`, `may_claim`) — this accessor hands back the
  /// raw cursor itself, whose `SeqCell` impl forwards whatever ordering the
  /// caller names. The pair decides for the questions it answers, not for
```

**Disposition:** applied — `producer()`'s and `consumer()`'s doc comments
now state the boundary by name: `GATING` is fixed only for the pair's own
three readings, not for a direct load or store through the returned cursor.
The crate's 23 tests (1 `allocation_test.rs` + 22 `cursor_test.rs`) plus 14
doctests re-verified passing (`cargo test --all-features`, 2026-09-04). Now
prints:
`is fixed only for the pair's own three readings`

---

### CU8 — Two Readings Are Not `const`, and the Reason Is Not Design

| Item | `const` |
|------|---------|
| `producer`, `consumer`, `capacity` | yes |
| `addr` | no |
| `on_distinct_lines` | no |

`addr` reads a runtime address through `core::ptr::from_ref`, which no `const`
context can evaluate. `on_distinct_lines` calls `addr` twice and inherits the
restriction.

**Finding.** The split is forced by one call, not chosen, and no document says
so — so a reader sees five accessors of which three are `const` and is left to
guess whether the other two were an oversight. They were not, and the constraint
will not lift by editing this crate.

---

### APIs

| File | Relationship |
|------|--------------|
| [001_the_surface_that_forwards.md](001_the_surface_that_forwards.md) | The half the accessors hand back into |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_the_slowest_fold.md](../algorithm/001_the_slowest_fold.md) | What `slowest` computes, and what it stopped allocating |
| [../algorithm/002_three_readings_of_two_cursors.md](../algorithm/002_three_readings_of_two_cursors.md) | What the three readings compute |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_gating_is_fixed_not_a_parameter.md](../decisions/001_gating_is_fixed_not_a_parameter.md) | Why these items fix rather than forward |
| [../decisions/002_the_capacity_is_held_by_the_pair.md](../decisions/002_the_capacity_is_held_by_the_pair.md) | Why `capacity()` is an accessor rather than a parameter |

### Items

| File | Relationship |
|------|--------------|
| [../item/002_cursor_pair_and_its_readings.md](../item/002_cursor_pair_and_its_readings.md) | Attributes and caller trees for each item here |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_gating.md](../type/001_gating.md) | The constant this surface is organised around |

### Sources

| File | Relationship |
|------|--------------|
| `ring_cursor/src/lib.rs:71-123` | `GATING` and `slowest` |
| `ring_cursor/src/lib.rs:247-442` | `CursorPair`'s eight associated functions |
| `ring_spsc/src/lib.rs:198, 213` | `OWN` and `HANDOFF`, the consumer's own ordering constants |
| `ring_spsc/src/lib.rs:887` | Both orderings on one pair, in one expression |

### Tests

| File | Relationship |
|------|--------------|
| `tests/cursor_test.rs:177-187` | A fresh pair — every reading at once |
| `tests/cursor_test.rs:231-253` | `may_claim` and `free_slots` never disagree |
| `tests/cursor_test.rs:285-299` | A capacity of one — the smallest legal ring |
| `src/lib.rs:106-118` | `slowest`'s doctest, including the empty case |
| `tests/manual/readme.md` M3 | S2, read from source |
