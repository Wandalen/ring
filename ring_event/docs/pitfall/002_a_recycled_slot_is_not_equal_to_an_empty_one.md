# Pitfall: A Recycled Slot Is Not Equal to an Empty One

> **Closed.** The trap this document records no longer exists. `BytesSlot`'s
> `Debug`, `PartialEq` and `Eq` were rewritten from a `derive` over the fields
> to hand-written impls over `read()`, so equality now agrees with every other
> public observation and a recycled slot *is* equal to an empty one. The
> narrative below is preserved as the record of what was found; the measured
> values it quotes were taken before that rewrite and are marked where they
> appear. Both findings' Dispositions carry the current state.

### Scope

**Purpose:** Record that `BytesSlot`'s equality once distinguished values no
public observation could tell apart, that it did so for only one of the two
shapes `recycle` serves, and that the obvious use for it was the one place it
failed — and that a later rewrite of that equality closed both.

**Responsibility:** The one surviving slot `derive` and the three hand-written
impls that replaced the other, both `Slot::clear` bodies, the five public
observations a `BytesSlot` offers, the family's uses of slot equality, and what
the crate's own recommended assertion does when written.

**In Scope:** `ring_slot/src/lib.rs:83-84`, `:183-186`, `:243-247`,
`:248-254`, `:256-262`, `:264-267`, `:290`, `:304`, `:323`, `:367`, `:392-395`;
`ring_slot/tests/slot_test.rs:258-260`.

**Out of Scope:** The `len`-only reset itself, and the contract it contradicts,
are [`data_structure/002`](../data_structure/002_what_clear_leaves_behind.md).
The other trap the same field creates is
[`pitfall/001`](001_a_zero_length_payload_reads_as_nothing.md).

---

## The Derive, the Two Resets, and What the Family Does Instead

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the one surviving derive --'
command grep -A 1 '^#\[ derive( Debug, Clone, PartialEq, Eq ) \]' ring_slot/src/lib.rs
echo '  -- and the three hand-written impls that replaced the other --'
command grep -E '^impl< const N : usize > (core::fmt::Debug|PartialEq|Eq) for BytesSlot' ring_slot/src/lib.rs
echo '  -- and clear still leaves them in different conditions --'
command grep -m1 -A6 -F '    self.0.is_none()' ring_slot/src/lib.rs | tail -n 4
command grep -m1 -A6 -F '    Self::is_empty( self )' ring_slot/src/lib.rs | tail -n 4
echo '  -- equality assertions against a slot anywhere in the family --'
command grep -rc 'assert_eq!( *[a-z_]*, *[A-Za-z]*Slot' --include=*.rs | awk -F: '$2 > 0' | wc -l
echo '  -- what the slot crates own suite does instead --'
command grep -m1 -A5 -F '  assert!( slot.write( b"abcde" ).is_err() );' ring_slot/tests/slot_test.rs | tail -n 4
echo '  -- is_empty assertions across both suites --'
command grep -rc 'is_empty()' --include=*.rs ring_event/tests ring_slot/tests | awk -F: '{ s += $2 } END { print "  " s }'
```

Live output:

```
  -- the one surviving derive --
#[ derive( Debug, Clone, PartialEq, Eq ) ]
pub struct TypedSlot< T >( Option< T > );
  -- and the three hand-written impls that replaced the other --
impl< const N : usize > core::fmt::Debug for BytesSlot< N >
impl< const N : usize > PartialEq for BytesSlot< N >
impl< const N : usize > Eq for BytesSlot< N > {}
  -- and clear still leaves them in different conditions --
  fn clear( &mut self )
  {
    self.0 = None;
  }
  fn clear( &mut self )
  {
    self.len = 0;
  }
  -- equality assertions against a slot anywhere in the family --
2
  -- what the slot crates own suite does instead --
  assert_eq!( slot.read(), b"abcd" );
  assert_eq!( slot.len(), 4 );
  assert!( !slot.is_empty() );
}
  -- is_empty assertions across both suites --
  38
```

## Every Observation Agrees; Equality Does Not

*Recorded before the rewrite.* The one line that has since changed is marked
below; every other reading is unchanged, because every other reading already
agreed.

```rust
// -ev_probe/src/bin/observational_equality.rs
// `used` is published to and then recycled; `fresh` was never touched.
let mut used = BytesSlot::< 4 >::empty();
publish_into( &mut used, &b"ab"[ .. ] ).unwrap();
recycle( &mut used );
let fresh = BytesSlot::< 4 >::empty();
// …every public observation compared, then `used == fresh`.
// The same sequence is then run at `TypedSlot< u32 >`.
```

```
    bytes  capacity   4 vs 4  agree true
    bytes  len        0 vs 0  agree true
    bytes  is_empty   true vs true  agree true
    bytes  read       [] vs []  agree true
    bytes  drain_from None vs None  agree true
    bytes  ==         false
    typed  is_empty   true vs true  agree true
    typed  drain_from None vs None  agree true
    typed  ==         true
```

**`bytes ==` now reads `true`.** Re-run 2026-09-07 against the hand-written
`PartialEq`: every line above is reproduced unchanged except that one, which
has flipped. `BytesSlot`'s five public observations and its equality now give
the same answer, which is the whole of what this section reported missing.

## How Many Empty Slots There Are

*Recorded before the rewrite.* Every line below has since changed; the current
readings follow the block.

```rust
// -ev_probe/src/bin/no_canonical_empty.rs
fn emptied_after( payload : &[ u8 ] ) -> BytesSlot< 4 >
{
  let mut slot = BytesSlot::< 4 >::empty();
  publish_into( &mut slot, payload ).unwrap();
  recycle( &mut slot );
  slot
}

let a = emptied_after( &b"ab"[ .. ] );
let b = emptied_after( &b"cd"[ .. ] );
let c = BytesSlot::< 4 >::empty();
```

```
    after ab    BytesSlot { bytes: [97, 98, 0, 0], len: 0 }  is_empty true
    after cd    BytesSlot { bytes: [99, 100, 0, 0], len: 0 }  is_empty true
    never used  BytesSlot { bytes: [0, 0, 0, 0], len: 0 }  is_empty true
    a == b false   a == c false   b == c false
    distinct values a BytesSlot< 4 > can hold that are all observationally empty: 4294967296
```

**All four lines now read differently.** Re-run 2026-09-07: all three slots
print `BytesSlot { payload: [] }`, and `a == b true   a == c true   b == c
true`. `Debug` reports what `read()` returns rather than the backing array, so
the residue is no longer printed; `PartialEq` compares the same thing, so the
three are no longer distinguishable. The class this section counted has one
member, not 4,294,967,296 — the count was never a property of the type's values
but of the representations mapping onto them, and equality no longer separates
representations.

---

### EV43 — The Derived Equality Is Finer Than Anything the Public API Can Observe

*Written before the rewrite.* The reasoning below is the record of what was
found; `PartialEq` has since been narrowed to `read()`, and the Disposition at
the end of this section says what that did to each step.

`BytesSlot< N >` offers five ways to look at itself: `capacity`, `len`,
`is_empty`, `read`, and `Peek::peek` through `drain_from`. The probe runs all
five against a recycled slot and a slot that was never used, and all five agree.
There is no sequence of public calls that separates the two values.

`==` separates them, because `PartialEq` is derived over the fields and `clear`
resets one of the two. That is not a bug in the derive — it is doing exactly
what a derive does — but it means the type's equality is strictly finer than
observational equivalence, and `Eq` is derived on top of it, which is the trait
that says the relation is a genuine equivalence on the type's values. Two values
a caller cannot tell apart are not equal.

The asymmetry makes it sharper. `TypedSlot`'s `clear` writes `self.0 = None`,
which restores the exact representation `empty()` produces, so for the typed
shape a recycled slot *is* equal to a fresh one and every intuition holds. Both
shapes derive the same four traits, on the same line, in the same file; `recycle`
is one generic function over both; and the answer differs. That is the crate's
whole thesis — one path, two behaviours — arriving somewhere nobody looked for
it.

**Finding.** There is no canonical representation of "empty" for `BytesSlot`:
the probe's arithmetic puts 4,294,967,296 distinct `BytesSlot< 4 >` values in
the observationally-empty class for `N = 4` alone, and three of them are printed
above, mutually unequal. A caller who reaches for `==` expecting it to mean
"the same state" is reaching for a relation the type does not implement, and
nothing at the declaration says so. One comment above `:160` — that equality
compares the whole backing array and so distinguishes slots that recycling has
made observationally identical — costs a line and removes the surprise. Making
`clear` zero the tail would remove the class instead, which is the fix
[`data_structure/002`](../data_structure/002_what_clear_leaves_behind.md)
already argues for on its own grounds; this is the second reason for it.

**Disposition:** declined — both proposed fixes (a comment above `:160`, or
zeroing the tail in `clear`) target `ring_slot/src/lib.rs`, outside
this pass's assigned crates. The premise is also no longer current: `:160`'s
derive no longer exists — `BytesSlot`'s `Debug`/`PartialEq` were rewritten
(concurrent, out-of-scope change) to compare via `read()` rather than the
raw fields, so `==` now agrees with every other public observation instead
of distinguishing an observationally-empty class; re-verified live by adding
this finding's own proposed assertion,
`assert_eq!( bytes, BytesSlot::< 8 >::empty() )`, to
`ring_event/tests/event_test.rs` and confirming it now passes rather than
fails (`cargo test -p ring_event --all-features`, 2026-09-04) — see
[`data_structure/002`](../data_structure/002_what_clear_leaves_behind.md)
EV12, where that assertion now lives permanently.

---

### EV44 — The Derive's One Obvious Use Is the One That Fails

Nobody had hit this. When this was written there were zero equality assertions
against a slot value anywhere in the thirty-three crates, and the family checked
state the long way — `ring_slot`'s own suite verifies an unchanged slot with
three separate assertions on `read()`, `len()` and `is_empty()` rather than one
comparison against an expected value, and the two suites carried thirty-one
`is_empty()` assertions between them. The trap was entirely latent. Both counts
have since moved; the recipe above reads them live, and the Disposition below
says what moved them.

It stops being latent the moment anyone writes the assertion the derive exists
for. `assert_eq!( slot, BytesSlot::< 8 >::empty() )` after a `recycle` is the
natural way to say "the slot is back to its initial state", it is the assertion
[`data_structure/002`](../data_structure/002_what_clear_leaves_behind.md)
recommends adding to `recycling_empties_either_shape_through_the_same_call`, and
it fails. It fails for a true reason — the residue is real and worth catching —
but the failure message names byte arrays, so a reader meets the `clear`
contract gap disguised as a broken test.

The nearby form is safe for a reason worth stating. The refusal tests that
[`invariant/002`](../invariant/002_a_refused_fill_changes_nothing.md) recommends
tightening compare against `before`, a clone taken from the slot itself, so the
residue is on both sides of the comparison and cancels. Comparing against a
clone works; comparing against a constructor does not; and the difference is
invisible unless you already know what `clear` leaves behind.

**Finding.** The derive is simultaneously the sharpest detector of the `clear`
gap in the codebase and the likeliest way to be confused by it, and which one it
is depends entirely on whether the reader already knows. That is the definition
of a trap worth documenting rather than removing: the assertion should be
written, it should fail, and the failure should be labelled — a comment on the
line saying it fails because `clear` resets `len` and not the array, so the
first person to add it reads a diagnosis instead of a puzzle. Between that and
the declaration comment above, the hazard costs two lines to defuse and neither
changes behaviour.

**Disposition:** moot — there is no longer a failure to label. Both of the two
equality assertions the recipe now counts *are* the assertion this finding
predicted nobody would write, and both pass:
`ring_event/tests/event_test.rs:228`, which EV43's Disposition records
adding, and `ring_slot/src/lib.rs:233`, a doctest asserting `clear
returns it to a fresh slot's value` — the exact inverse of this document's
title. `BytesSlot`'s `PartialEq` is now hand-written over `read()`
(`ring_slot/src/lib.rs:256-262`), so it no longer separates a recycled
slot from a fresh one, and the comment this finding asked for would be
describing behaviour the type does not have. The `is_empty()` census moved
thirty-one → thirty-eight over the same period. Re-measured 2026-09-07.

The finding's *reasoning* survives its subject. It argued the derive was
simultaneously the sharpest detector of the `clear` gap and the likeliest way
to be confused by it — and the resolution went the other way than either branch
it offered: rather than labelling the failure or zeroing the tail, `PartialEq`
was narrowed to `read()`, which removes the detector along with the confusion.
What still holds is that `clear` leaves the residue; the recipe above shows both
`clear` bodies unchanged. That gap is
[`data_structure/002`](../data_structure/002_what_clear_leaves_behind.md)'s, and
equality is no longer evidence for or against it.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`data_structure/002`](../data_structure/002_what_clear_leaves_behind.md) | The residue that makes both traps possible |
| [`pitfall/001`](001_a_zero_length_payload_reads_as_nothing.md) | The other trap the same `len` field creates |
| [`invariant/002`](../invariant/002_a_refused_fill_changes_nothing.md) | The comparison form that is safe, and why |
| [`pattern/002`](../pattern/002_proof_by_monomorphisation.md) | One generic function, two shapes, two answers |

### Sources

| Fact | Where |
|------|-------|
| `TypedSlot` deriving `PartialEq, Eq` — the one derive left | `ring_slot/src/lib.rs:83-84` |
| `BytesSlot`'s hand-written `Debug`, `PartialEq` and `Eq`, and why | `ring_slot/src/lib.rs:243-247`, `:248-254`, `:256-262`, `:264-267` |
| `TypedSlot::clear` restoring the canonical representation | `ring_slot/src/lib.rs:183-186` |
| `BytesSlot::clear` resetting only the length — still true | `ring_slot/src/lib.rs:392-395` |
| The five public observations `BytesSlot` offers | `ring_slot/src/lib.rs:290`, `:304`, `:323`, `:367`, and `Peek::peek` |
| All five agreeing where `==` once did not | Probe above, and the re-run beneath it |
| The observationally-empty class, then 2³² and now 1 | Probe above, and the re-run beneath it |
| The two slot-equality assertions that now exist, both passing | `ring_event/tests/event_test.rs:228`, `ring_slot/src/lib.rs:233` |
| The three-assertion form the slot suite uses instead | `ring_slot/tests/slot_test.rs:258-260` |

### Tests

| Test | Covers |
|------|--------|
| `recycling_empties_either_shape_through_the_same_call` | The reset both shapes share, and the equality assertion now inside it at `:228` |
| `a_recycled_storage_slot_stops_returning_the_previous_lap` | The property the residue does not violate |
| `ring_slot::a_failed_write_leaves_the_previous_contents_intact` | The slot suite's field-by-field alternative to `==` — declared in `ring_slot/tests/`, not here |
