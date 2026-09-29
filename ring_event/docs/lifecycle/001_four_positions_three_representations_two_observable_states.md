# Lifecycle: Four Positions, Three Representations, Two Observable States

### Scope

**Purpose:** Trace a slot through the three operations this crate defines, and
establish how much of the resulting lifecycle either observation method can
actually see.

**Responsibility:** The transitions `publish_into`, `drain_from` and `recycle`
perform, the two-answer vocabulary `Slot` offers, and the gap between the
positions a byte slot occupies and the states its observers can report.

**In Scope:** `ring_event/src/lib.rs:173-178`, `:207-212`, `:229-234`;
`ring_slot/src/lib.rs:41-67`, `:84`, `:183-186`, `:238-241`, `:392-395`;
probe below.

**Out of Scope:** What the recycled residue means for a caller is
[`data_structure/002`](../data_structure/002_what_clear_leaves_behind.md). The
zero-length reading in isolation is
[`pitfall/001`](../pitfall/001_a_zero_length_payload_reads_as_nothing.md).

---

## The Three Operations and the Two Words

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the three operations, whole --'
awk '/^pub fn publish_into< S, P >\( slot : &mut S, payload : P \) -> Result< \(\), RingError >$/{ n1 = NR } n1 && NR >= n1 && NR <= n1 + 5 { print } /^\/\/\/ let slot = BytesSlot::< 4 >::empty\(\);$/{ n2 = NR } n2 && NR >= n2 + 3 && NR <= n2 + 8 { print } /^pub fn recycle< S >\( slot : &mut S \)$/{ n3 = NR } n3 && NR >= n3 && NR <= n3 + 5 { print }' ring_event/src/lib.rs
echo '  -- the entire state vocabulary the Slot trait offers --'
command grep -m1 -A7 -F 'pub trait Slot' ring_slot/src/lib.rs
echo '  -- what each shape actually stores --'
awk '/^pub struct TypedSlot< T >\( Option< T > \);$/{ print } /^pub struct BytesSlot< const N : usize >$/{ n2 = NR } n2 && NR >= n2 + 1 && NR <= n2 + 4 { print }' ring_slot/src/lib.rs
echo '  -- and what clear writes, per shape --'
awk '/^    self\.0\.is_none\(\)$/{ n1 = NR } n1 && NR >= n1 + 3 && NR <= n1 + 6 { print } /^    Self::is_empty\( self \)$/{ n2 = NR } n2 && NR >= n2 + 3 && NR <= n2 + 6 { print }' ring_slot/src/lib.rs
```

Live output:

```
  -- the three operations, whole --
pub fn publish_into< S, P >( slot : &mut S, payload : P ) -> Result< (), RingError >
where
  P : Fill< S >,
{
  payload.fill( slot )
}
/// let mut slot = BytesSlot::< 4 >::empty();
/// publish_into( &mut slot, &b"ab"[ .. ] ).unwrap();
/// assert_eq!( drain_from( &slot ), Some( &b"ab"[ .. ] ) );
/// assert!( !slot.is_empty(), "reading it did not free it" );
///
/// recycle( &mut slot );
pub fn recycle< S >( slot : &mut S )
where
  S : Slot,
{
  slot.clear();
}
  -- the entire state vocabulary the Slot trait offers --
pub trait Slot
{
  /// Whether this slot currently holds nothing.
  fn is_empty( &self ) -> bool;

  /// Return the slot to its empty state.
  ///
  /// **Not a promise to overwrite.** For a shape that owns what it stores
  -- what each shape actually stores --
pub struct TypedSlot< T >( Option< T > );
{
  bytes : [ u8; N ],
  len : usize,
}
  -- and what clear writes, per shape --
  fn clear( &mut self )
  {
    self.0 = None;
  }
  fn clear( &mut self )
  {
    self.len = 0;
  }
```

## A Byte Slot Walked Through Every Position

*The four rows below predate `BytesSlot`'s `Debug`/`PartialEq` rewrite (see
EV12's Disposition in
[`data_structure/002`](../data_structure/002_what_clear_leaves_behind.md)):
today's `Debug` prints only the written prefix — e.g.
`BytesSlot { payload: [97, 98] }` for the second row — not the raw
`bytes`/`len` fields shown here. The `is_empty`/`peek` columns do not depend
on `Debug` and are unaffected; this file's own EV30 section below states the
current (post-rewrite) behavior as fact, which is what makes the frozen
`Debug` column here worth flagging rather than silently trusting.*

```rust
// -ev_probe/src/bin/slot_states.rs
let mut slot = BytesSlot::< 4 >::empty();
show( "never written", &slot );

publish_into( &mut slot, &b"ab"[ .. ] ).unwrap();
show( "written, two bytes", &slot );

recycle( &mut slot );
show( "recycled", &slot );

let mut fresh = BytesSlot::< 4 >::empty();
publish_into( &mut fresh, &b""[ .. ] ).unwrap();
show( "written, zero bytes", &fresh );
```

```
    never written          BytesSlot { bytes: [0, 0, 0, 0], len: 0 }  is_empty true   peek None
    written, two bytes     BytesSlot { bytes: [97, 98, 0, 0], len: 2 }  is_empty false  peek Some([97, 98])
    recycled               BytesSlot { bytes: [97, 98, 0, 0], len: 0 }  is_empty true   peek None
    written, zero bytes    BytesSlot { bytes: [0, 0, 0, 0], len: 0 }  is_empty true   peek None
```

---

### EV29 — Only One of the Three Operations Is a Transition

`publish_into` takes `&mut S` and moves a slot from empty to holding.
`recycle` takes `&mut S` and moves it back. `drain_from` takes `&S`, calls
`peek`, and moves nothing — the payload it returns is a borrow into a slot that
is in exactly the state it was before the call.

So the crate's three operations are two transitions and one observation, and
nothing in their names or documentation groups them that way. `recycle`'s own doc
calls them "the three operations a ring performs on a slot", which is true and
flattens the distinction that matters most: a ring that publishes and drains has
moved its slot exactly once, and the slot is still full afterwards. Emptying it
is a separate call the ring must remember to make.

The typed shape makes the consequence concrete. `TypedSlot< T >` is
`Option< T >`, `clear` writes `None`, and the payload drops there — so a drain
followed by no `recycle` holds the payload alive for the rest of the lap.
`ring_slot`'s own `take` collapses read-and-empty into one call and returns an
owned value, which is why `ring_core` uses it and not this crate
([`integration/002`](../integration/002_the_ring_writes_slots_without_this_crate.md)).

**Finding.** The lifecycle this crate expresses needs three calls where the
operation a ring actually performs needs two, and the third is the one with no
return value and no error, which is the one easiest to omit. The doc comments
describe each function in isolation and never draw the sequence, so the fact
that a drain leaves the slot full — and that omitting `recycle` is a leak on the
typed side and a stale read on the byte side — has to be assembled by the reader
from three separate signatures.

---

### EV30 — A Byte Slot Occupies Four Positions and Reports Two

`Slot` offers exactly two words: `is_empty`, which answers a boolean, and
`clear`, which drives toward the `true` side of it. `Peek` adds one more
observation, `peek`, which answers `Some` or `None`. Between them that is the
entire vocabulary a generic caller has for a slot's state.

`TypedSlot` fits it exactly. `Option< T >` has two inhabitants, `clear` writes
`None`, and both observers read the same field, so every position the type can
occupy has a distinct representation and a distinct answer.

`BytesSlot` does not. The probe walks it through four positions and the results
collapse twice over:

Never-written and written-with-zero-bytes are **byte-identical** — same array,
same `len`, same `Debug` rendering. Nothing anywhere can tell them apart, which
is the state [`pitfall/001`](../pitfall/001_a_zero_length_payload_reads_as_nothing.md)
covers from the caller's side.

Recycled is **distinct in memory but not in observation**. Its array still holds
`[ 97, 98, 0, 0 ]` while `len` is `0`, and no public surface shows it: `is_empty`
and `peek` read `len`, and `Debug` and `PartialEq` are written by hand over
`read()`, which stops at `len` too. The bytes are there and nothing reaches them.

Four positions, three representations, two observable states.

**Finding.** The lifecycle a generic caller can reason about is strictly coarser
than the lifecycle the byte shape actually has, and the coarsening is not uniform
across shapes — the typed slot's state machine is faithful and the byte slot's
is lossy in two different directions at once. That is defensible as a design:
`is_empty` answering from `len` is what makes recycling cheap, and collapsing
zero-length into empty is what `Peek`'s doc comment spends eleven lines
explaining. What is not recorded anywhere is that the two shapes differ in this
respect at all, so a reader who has understood the typed lifecycle has not
understood the byte one, and nothing tells them.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`lifecycle/002`](002_a_finished_crate_in_the_unverified_stage.md) | The crate's own lifecycle position, by the same measure |
| [`data_structure/002`](../data_structure/002_what_clear_leaves_behind.md) | The third position, read as a hazard |
| [`invariant/001`](../invariant/001_two_readings_of_one_emptiness.md) | The agreement between the two observers, across these positions |
| [`pitfall/001`](../pitfall/001_a_zero_length_payload_reads_as_nothing.md) | The fourth position, read as a surprise |

### Sources

| Fact | Where |
|------|-------|
| Two transitions and one observation | `ring_event/src/lib.rs:173`, `:207`, `:229` |
| "the three operations a ring performs on a slot" | `ring_event/src/lib.rs:216-218` |
| The two-word state vocabulary | `ring_slot/src/lib.rs:41-67` |
| `TypedSlot` being an `Option` and `clear` writing `None` | `ring_slot/src/lib.rs:84`, `:183-186` |
| `BytesSlot` holding an array beside a length, `clear` writing only the length | `ring_slot/src/lib.rs:238-241`, `:392-395` |
| Four positions collapsing to three representations and two answers | Probe above |

### Tests

| Test | Covers |
|------|--------|
| `recycling_empties_either_shape_through_the_same_call` | The second transition, at both shapes |
| `recycling_a_slot_that_was_never_published_is_harmless` | The transition applied from the position it targets |
| `a_zero_length_publish_is_indistinguishable_from_unpublished_and_says_so` | The first collapse |
| `a_recycled_storage_slot_stops_returning_the_previous_lap` | The second, as far as observation reaches |
