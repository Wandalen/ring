# Workaround: The `usize`/`u64` Seam

### Scope

**Purpose:** Record the three `as` casts in the crate body, which external
constraint each absorbs, and which one absorbs nothing.

**Responsibility:** `Seq`'s `u64` against `Capacity::get()`'s and `count`'s
`usize`, at the three points where the crate crosses between them.

**In Scope:** `ring_batch/src/lib.rs:131`, `:223`, `:323`;
`ring_types/src/id.rs:25`; `ring_seqno/src/lib.rs:95`.

**Out of Scope:** That the `+` on line 113 overflows is
[`pitfall/002`](../pitfall/002_the_addition_with_no_panics_section.md). The
capture bounds are
[`workaround/001`](001_the_capture_bound_edition_2024_made_necessary.md).

---

## Two Representations for One Quantity

`Seq` carries a `u64` because a sequence counter must not wrap on a 32-bit
target. `Capacity` and `count` are `usize` because they measure a slice. Both
describe positions in the same ring, so every comparison between them crosses a
type boundary the family chose deliberately and never named.

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- every cast in the crate body --'
command grep ' as u64\| as usize' ring_batch/src/lib.rs | command grep -v '///'
echo '  -- the two sides --'
command grep -m1 -F 'pub struct Seq( pub u64 );' ring_types/src/id.rs
command grep -m1 -F 'pub fn free_slots( producer : Seq, consumer : Seq, capacity : Capacity ) -> usize' ring_seqno/src/lib.rs
echo '  -- and the one crossing that needs no cast at all --'
command grep -m1 -F '  if count > capacity.get()' ring_batch/src/lib.rs
```

Live output:

```
  -- every cast in the crate body --
    Seq( self.start.0 + self.count as u64 )
  BatchClaim::new( cursor.fetch_add( count as u64, order ), count )
  if ( free_slots( at, behind, capacity ) as usize ) < count
  -- the two sides --
pub struct Seq( pub u64 );
pub fn free_slots( producer : Seq, consumer : Seq, capacity : Capacity ) -> usize
  -- and the one crossing that needs no cast at all --
  if count > capacity.get()
```

---

### BA52 — The Crate Only Ever Widens, Which Is Why It Has No Fallible Conversion

Two of the three casts go `usize` → `u64`: a `count` turned into a sequence
offset on line 131, and the same `count` turned into a `fetch_add` argument on
line 223. Both are lossless on every target Rust supports, since `usize` is never
wider than 64 bits.

**Finding.** The crate never converts in the other direction, which is why
`try_from`, `TryInto`, and `expect` appear nowhere in it. That is a real property
and it is load-bearing: a `u64` → `usize` conversion would be fallible on a
32-bit target, would need an error path, and would give this crate a failure mode
it currently does not have — `claim` returns `BatchClaim`, not
`Result< BatchClaim, _ >`, and that is only sound because nothing in it can
narrow.

Line 316 shows the seam handled without any cast at all: `count > capacity.get()`
compares two `usize` values, because `Capacity` was designed to answer in the
same width as the thing it bounds. So the crate crosses the boundary three times
and stays on one side once, and the pattern in all four is the same — go to
`u64` when a `Seq` is involved, stay in `usize` when it is not.

None of this is written down. A reader wondering why line 316 has no cast and
line 323 has one has to work out the widths themselves.

---

### BA53 — One of the Three Casts Crosses Nothing

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the cast --'
command grep -m1 -F '  if ( free_slots( at, behind, capacity ) as usize ) < count' ring_batch/src/lib.rs
echo '  -- and what it is casting --'
command grep -m1 -F 'pub fn free_slots( producer : Seq, consumer : Seq, capacity : Capacity ) -> usize' ring_seqno/src/lib.rs
```

Live output:

```
  -- the cast --
  if ( free_slots( at, behind, capacity ) as usize ) < count
  -- and what it is casting --
pub fn free_slots( producer : Seq, consumer : Seq, capacity : Capacity ) -> usize
```

**Finding.** `free_slots` returns `usize` and the expression casts it to `usize`.
Removing the cast — and the parentheses that exist only to hold it — leaves
`if free_slots( at, behind, capacity ) < count`, which compiles clean on a
patched copy.

Nothing will catch it. `cargo clippy` on this crate is silent, and stays silent
with `-W clippy::unnecessary_cast -W clippy::cast_lossless` passed explicitly:
the lint fires on literals and simple expressions, not on a function call's
result. The workspace's clippy table has exactly one entry —
`undocumented_unsafe_blocks = "deny"` — which can never fire either, because
`unsafe-code = "deny"` two tables above already forbids the blocks it would
document.

The cost is not the instruction; the compiler emits nothing for it. The cost is
that the parenthesised `( … as usize )` reads as a deliberate width fix at
exactly the place a reader is trying to work out which side of the seam they are
on, in the body of the function that has both of the crate's real hazards
([`pitfall/001`](../pitfall/001_the_window_between_the_gate_and_the_advance.md),
[`type/001`](../type/001_the_ring_that_can_gate_against_itself.md)). It is a
signpost pointing at a boundary that is not there.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`workaround/001`](001_the_capture_bound_edition_2024_made_necessary.md) | The crate's other absorbed external constraint |
| [`pitfall/002`](../pitfall/002_the_addition_with_no_panics_section.md) | What the widened value on line 113 is added to |
| [`algorithm/002`](../algorithm/002_check_then_advance.md) | The function all three of these lines sit in or feed |
| [`data_structure/001`](../data_structure/001_sixteen_bytes_that_are_not_a_buffer.md) | The two fields with the two different widths |

### Sources

| Fact | Where |
|------|-------|
| The three casts | `ring_batch/src/lib.rs:131`, `:223`, `:323` |
| `Seq`'s representation | `ring_types/src/id.rs:25` |
| `free_slots`' return type | `ring_seqno/src/lib.rs:95` |
| The crossing that needs no cast | `ring_batch/src/lib.rs:316` |
| The redundant cast compiles away, and clippy is silent | Patched-copy build and forced-lint run |

### Tests

| Test | Covers |
|------|--------|
| `a_full_ring_refuses_with_full_and_advances_nothing` | The comparison the redundant cast sits inside |
| *(to create)* | Nothing exercises a `count` near `usize::MAX`, the only place the two widening casts could matter |
