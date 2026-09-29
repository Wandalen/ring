# Lifecycle: A Cursor From `new` to Shared

### Scope

- **Purpose**: Trace a `PaddedCursor` through every stage it has, and show that the interesting property is how few stages there are.
- **Responsibility**: Give each stage with the code that performs it and the test that covers it, and account for the traits the type deliberately does not have.
- **In Scope**: Construction, placement, sharing, operation, movement, destruction.
- **Out of Scope**: How a *pair* evolves as a ring runs, which is [`lifecycle/002`](002_a_pair_across_a_full_lap.md).

### The Stages

| # | Stage | Code | Covered by |
|---|-------|------|------------|
| 1 | **Construct** | `PaddedCursor::new( Seq( 7 ) )` or `::default()` | `a_cursor_holds_the_sequence_it_was_built_with` |
| 2 | **Place** | A `CursorPair` field, a `Vec` element, or a standalone binding | `an_array_of_cursors_gives_each_its_own_line` |
| 3 | **Share** | `&PaddedCursor` handed to any number of readers | `a_cursor_is_shared_by_reference_not_by_copy` |
| 4 | **Operate** | The four `SeqCell` methods, any number of times, from any number of threads | `padding_does_not_change_what_the_cell_does`, `many_producers_on_one_cursor_lose_nothing` |
| 5 | **Move** | Boxed, returned, pushed into a `Vec` — before sharing | `the_gap_survives_the_pair_being_moved` |
| 6 | **Drop** | Nothing happens | — |

**There is no seventh stage.** A cursor has no closed, poisoned, or exhausted
state; nothing invalidates it; nothing needs to be released. It is an integer
that moved.

### Stage 1 Has Two Doors and They Agree

```rust
assert_eq!( PaddedCursor::new( Seq( 7 ) ).load( Ordering::Acquire ), Seq( 7 ) );
assert_eq!( PaddedCursor::default().load( Ordering::Acquire ), Seq::ZERO );
```

`Default` is derived, so it reaches `AtomicSeq`'s and ultimately `AtomicU64`'s —
`0`. The second assertion pins that the derived path and `Seq::ZERO` are the same
value, which is worth an assertion precisely because it is a chain of three
derives nobody wrote.

Both doors exist because the pair uses one and the slice-shaped consumers use the
other: `CursorPair::new` calls `PaddedCursor::new( Seq::ZERO )` explicitly, and
`ring_gating` builds a `Vec` by `resize_with( n, PaddedCursor::default )`.

### Stage 3 Is Where the Missing Traits Matter

```rust
#[ derive( Debug, Default ) ]
pub struct PaddedCursor( CacheAligned< AtomicSeq > );
```

Two derives, and the four it does **not** have are the design:

| Absent | If it were present |
|--------|--------------------|
| `Clone` | A caller could clone a cursor and hold two positions believing it held one |
| `Copy` | The same, silently, on every pass by value |
| `PartialEq` | Comparing two cursors would read both non-atomically and compare a state that never existed |
| `Drop` | Nothing to release — adding one would be the only reason the type could not be `const`-constructed |

The test says the first two out loud:

> Two `&PaddedCursor` to one cursor must see each other's writes. If the padding
> wrapper had made the type `Copy`, a caller could hold two independent cursors
> while believing it held one.

**Note the counterfactual is about the wrapper.** `AtomicU64` is not `Copy`, so
`PaddedCursor` could not have become `Copy` by accident — but `CacheAligned< T >`
is a generic wrapper, and a `#[ derive( Clone, Copy ) ]` added to *it* for some
other `T`'s benefit would be the way this happens.

### Stage 5 Happens Before Stage 3, Always

A cursor may be moved freely while it is owned, and not at all once it is shared —
that is Rust's borrow checker, not a rule this crate states. The ordering matters
for one reason:

```rust
let pair = CursorPair::new( cap( 2 ) );
let boxed = Box::new( pair );
assert!( boxed.on_distinct_lines(), "still separated after a move onto the heap" );
```

The test's own comment concedes it cannot fail:

> A struct's field offsets are fixed at compile time, so this cannot fail for a
> live `CursorPair` — which is the point. The assertion exists to catch a future
> layout where the two cursors stop being separate fields.

**An assertion that cannot fail today, kept for a change that would make it able
to.** That is a defensible test and an unusual one, and it is only defensible
because the comment says so — the same assertion without the comment reads as a
misunderstanding of `repr` guarantees.

### Stage 6 Is Empty, and That Is a Property

No `Drop`, no `unsafe`, no allocation. `grep -c unsafe ring_cursor/src/lib.rs`
is `0`, and the workspace denies `unsafe-code` anyway.

The consequence worth stating: **a `PaddedCursor` in a `static` is legal**, because
`new` is `const` in the ordinary build. That is what makes a statically-allocated
ring possible, and it is why the `loom` build's loss of `const` is filed as a
workaround rather than shrugged off — see
[`workaround/001`](../workaround/001_the_loom_constructor_cannot_be_const.md).

### What No Stage Covers

| # | Gap | Where it is handled |
|---|-----|---------------------|
| L1 | Shutting down — telling readers to stop | `ring_shutdown`, with a separate `AtomicBool` flag. A cursor has no way to say "no more" |
| L2 | Overflow — a cursor reaching `u64::MAX` | `ring_types::Seq::next` panics in debug, wraps to zero in release. Nothing in this crate checks |
| L3 | Reuse — resetting a cursor for a second run | `store( Seq::ZERO, … )`, with no help from the type. Nothing prevents doing it while readers are live |

**L1 is the one that shapes the family.** Because a cursor cannot express
termination, `ring_shutdown` exists as a separate crate holding a separate
atomic — `src/lib.rs:82` reads `self.closed.load( Ordering::Acquire )`. A design
that gave `Seq` a sentinel would have removed that crate and made every
comparison in `ring_seqno` conditional. The split is the better trade and it is
invisible unless both halves are read together.

### CU29 — Half the Constructors Are Never Compiled

```
158:  #[ cfg( not( loom ) ) ]
166:  #[ cfg( loom ) ]
268:  #[ cfg( not( loom ) ) ]
281:  #[ cfg( loom ) ]
```

Two types, two `cfg` arms each, identical bodies within each pair.

**Finding.** Only the `not( loom )` copy is compiled by any verification level
this project runs. The `loom` copy is built when someone types
`RUSTFLAGS="--cfg loom"` by hand, which no level does — so the two can drift
apart silently, and the first symptom would be a `loom` run failing to compile
long after the change that broke it.

---

### CU30 — The Family's Four Destructors Are All on Guards

```
ring_mpsc/src/lib.rs:985:impl< S > Drop for Reserved< '_, S >
ring_mpsc/src/lib.rs:1257:impl< S > Drop for Batch< '_, S >
ring_spsc/src/lib.rs:788:impl< S > Drop for Reservation< '_, S >
ring_spsc/src/lib.rs:1130:impl< S > Drop for Batch< '_, S >
```

Every one carries a lifetime parameter. Not one is on a type that owns a ring.

**Finding.** So the absence of `Drop` here is not "a position needs no cleanup" —
it is "this crate hands out no guards". A cursor's end of life is its owner's
business, and the crates that do write destructors write them for borrows, which
is a different lifecycle question from the one this instance traces.

---

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_padded_cursor.md](../data_structure/001_the_padded_cursor.md) | The layout that stages 2 and 5 preserve |

### Lifecycles

| File | Relationship |
|------|--------------|
| [002_a_pair_across_a_full_lap.md](002_a_pair_across_a_full_lap.md) | The same stages for a pair, and the arithmetic they run under |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_a_reading_that_consults_one_cursor.md](../pitfall/002_a_reading_that_consults_one_cursor.md) | Why stage 1's `Seq::ZERO` hides a defect in the readings |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_padded_cursor.md](../type/002_padded_cursor.md) | The two derives, and the four absent ones as promises |

### Workarounds

| File | Relationship |
|------|--------------|
| [../workaround/001_the_loom_constructor_cannot_be_const.md](../workaround/001_the_loom_constructor_cannot_be_const.md) | Stage 1 in the build where it is not `const` |

### Sources

| File | Relationship |
|------|--------------|
| `ring_cursor/src/lib.rs:143-171` | The derives and both constructors |
| `ring_cursor/src/lib.rs:199-221` | Stage 4 — the four operations |
| `ring_shutdown/src/lib.rs:82` | L1 — the flag a cursor cannot be |
| `ring_types/src/id.rs:34-46` | L2 — `Seq::next`'s deliberate non-wrapping |

### Tests

| File | Relationship |
|------|--------------|
| `tests/cursor_test.rs:130-135` | Stage 1, both doors |
| `tests/cursor_test.rs:137-160` | Stage 4, all four methods |
| `tests/cursor_test.rs:162-173` | Stage 3, and the `Copy` counterfactual |
| `tests/cursor_test.rs:90-100` | Stage 5, and its unfailable assertion |
| `tests/cursor_test.rs:370-395` | Stage 4 under contention |
