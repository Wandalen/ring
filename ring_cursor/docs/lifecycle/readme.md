# lifecycle

How a cursor and a pair evolve — one from construction to destruction, the other
around a lap and back to a state it has already been in.

### Overview Table

| ID | Name | Subject | Stages |
|----|------|---------|-------:|
| 001 | [A Cursor From `new` to Shared](001_a_cursor_from_new_to_shared.md) | One `PaddedCursor` | 6 |
| 002 | [A Pair Across a Full Lap](002_a_pair_across_a_full_lap.md) | A `CursorPair`'s three readings | 6 rows, cyclic |

### The Two Are Different Kinds of Lifecycle

001 is a **linear** progression with a terminal stage: construct, place, share,
operate, move, drop. It ends.

002 is **cyclic and unbounded**: the readings are functions of the difference
between two cursors, so a pair returns to states it has already occupied while
both cursors climb forever. Nothing about a `CursorPair` ends.

The interesting result in each is negative:

| | The finding |
|---|---|
| 001 | There is no seventh stage — no closed, poisoned, or exhausted state. `ring_shutdown` exists as a separate crate because a cursor cannot express termination |
| 002 | One impossible state (consumer ahead of producer) is indistinguishable from a fresh ring in **all three readings at once**, because the arithmetic saturates. `ring_debug` exists to bypass that arithmetic |

**Both negatives explain a whole crate's existence.** That is the argument for
documenting lifecycles here rather than only where the types are defined: what a
type *cannot* represent is what the neighbouring crates are for.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the constructors, one pair per build configuration --'
command grep -E '^  #\[ cfg\( (not\( )?loom' ring_cursor/src/lib.rs
echo '  -- and the destructor this crate does not have --'
command grep -cE 'impl.*Drop for' ring_cursor/src/lib.rs
echo '  -- control: the same expression where the family does write them --'
command grep -rE 'impl.*Drop for' --include=*.rs ring_*/src/ | sed 's|ring/||'
```

Live output:

```
  -- the constructors, one pair per build configuration --
  #[ cfg( not( loom ) ) ]
  #[ cfg( loom ) ]
  #[ cfg( not( loom ) ) ]
  #[ cfg( loom ) ]
  -- and the destructor this crate does not have --
0
  -- control: the same expression where the family does write them --
ring_mpsc/src/lib.rs:impl< S > Drop for Reserved< '_, S >
ring_mpsc/src/lib.rs:impl< S > Drop for Batch< '_, S >
ring_spsc/src/lib.rs:impl< S > Drop for Reservation< '_, S >
ring_spsc/src/lib.rs:impl< S > Drop for Batch< '_, S >
```

**Constructor attributes, no destructor, and the four the family does write.** The
zero is paired with the control because a zero proves nothing on its own — the
first version of this recipe searched for `impl Drop for`, which the family never
writes, and so it printed zero for `ring_mpsc` too. The expression is now
identical in both arms, and every hit it finds is a *guard* — `Reserved`,
`Reservation`, `Batch`, each with a lifetime parameter. Not one is on a type that
owns a ring.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CU29 | The four constructor bodies | n/a — duplication | Two types × two `cfg` arms, with identical bodies in each pair. Only the `not( loom )` copy is compiled by any routine verification level, so the `loom` copy can drift from it silently until someone runs `RUSTFLAGS="--cfg loom"` by hand |
| CU30 | The absent `Drop` | n/a — observation | This crate declares none, so a cursor's end of life is whatever owns it. The family's four `Drop` impls are all on borrow guards with a lifetime — `Reserved`, `Reservation`, `Batch` — and none on a type that owns a ring, so the absence here is not "positions need no cleanup" but "this crate hands out no guards" |
| CU31 | The lap boundary | n/a — coverage | Whether a producer exactly `capacity` ahead may claim is decided in `ring_seqno` and asserted here by `exactly_one_lap_ahead_is_full_and_one_less_is_not`. This crate contributes two loads and owns none of the arithmetic, so a change to the boundary rule reaches these tests as a failure rather than as a compile error |
| CU32 | `pending` versus `free_slots` | n/a — inconsistency | Across a full lap the two readings answer complementary questions in different types — `u64` and `usize` — so a caller checking that they sum to the capacity casts one of them. The widths come from `ring_seqno`; this crate forwards them and adds no note |
