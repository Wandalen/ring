# data_structure

This crate declares no data. No struct, no enum, no union, no type alias — four of
the family's 33 crates are in that position, and this is the only one of the four
that fills the gap with traits rather than with functions over somebody else's
concrete types. Its signatures name `S` and `P`; the shapes are `ring_slot`'s.

That makes this definition a study of what a shape-agnostic crate inherits. Both
instances follow the same thread: the two slot shapes agree exactly as far as
this crate's own API can see, and disagreed one field below it. The first
instance establishes the inheritance and found equality meaning two things —
`BytesSlot`'s `PartialEq` is now hand-written over `read()`, so that half is
closed. The second follows it to `recycle`, the one operation with no return
value, and found a trait contract true for one implementor and false for the
other, since rewritten to name the split. What survives both rewrites is the
test: written to cover both shapes, and unable to tell them apart.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_a_crate_that_declares_no_data.md) | A Crate That Declares No Data | Zero types, four of 33, and the properties that arrive with the shapes |
| [002](002_what_clear_leaves_behind.md) | What `clear` Leaves Behind | One contract, two impls, and eight bytes that outlive the reset |

## What the Crate Owns and What It Borrows

Owned: two traits, three free functions, one associated type. Borrowed:
`TypedSlot`, `BytesSlot` and `Slot` from `ring_slot`, `RingError` from
`ring_types`. The representation vocabulary — `repr`, `size_of`, `align_of`,
`transmute`, `as_ptr` — appears zero times in 234 lines, so nothing here depends
on how either shape is laid out.

The single line in the crate carrying the word `struct`, `enum` or `union` is a
doc comment at `:43` noting that implementing `Fill` on the payload is what lets
`publish_into` need "no enum of shapes." That is the design stated once, about one
function, and never at the scope where it is the crate's whole reason to exist.

## Where the Two Shapes Stop Agreeing

`TypedSlot< T >` is `Option< T >`. `BytesSlot< N >` is `[ u8; N ]` plus a `len`.
`TypedSlot` derives `Debug`, `Clone`, `PartialEq` and `Eq`; `BytesSlot` derives
only `Clone` and writes the other three by hand over `read()`. On the derives
alone the two once looked interchangeable.

They still are not, but the seam moved. Equality no longer splits them: both
compare the bytes `read()` returns, so two slots this crate reads identically
now compare equal too. `clear` is where the split survives — the typed impl
assigns `None` and runs the destructor, the byte impl writes `len = 0` and
leaves every byte in place. The contract used to say the payload was dropped,
which described one of the two; it now says outright that clearing is not a
promise to overwrite, and names which shape does which.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- types declared here, and the crates in the same position --'
command grep -rn '^pub struct\|^pub enum\|^pub union\|^pub type' --include=*.rs ring_event/src | wc -l
for c in ring_*/; do
  n=$( command grep -rc '^pub struct\|^pub enum\|^pub union\|^pub type\|^struct \|^enum ' --include=*.rs $c/src 2>/dev/null | awk -F: '{ s += $2 } END{ print s+0 }' )
  [ "$n" = 0 ] && basename $c
done
echo '  -- the shapes it borrows, and what they are made of --'
command grep -m1 -A1 -F 'use ring_slot::{ BytesSlot, Slot, TypedSlot };' ring_event/src/lib.rs
awk '/^\/\/\/ assert_eq!\( slot\.take\(\), Some\( 42 \) \);$/{ n1 = NR } n1 && NR >= n1 + 3 && NR <= n1 + 4 { print } /^\/\/\/ assert!\( !slot\.is_empty\(\) \);$/{ n2 = NR } n2 && NR >= n2 + 2 && NR <= n2 + 7 { print }' ring_slot/src/lib.rs
echo '  -- the contract, and the two impls of it --'
command grep -m1 -A3 -F '  /// Return the slot to its empty state.' ring_slot/src/lib.rs
awk '/^    self\.0\.is_none\(\)$/{ n1 = NR } n1 && NR >= n1 + 3 && NR <= n1 + 6 { print } /^    Self::is_empty\( self \)$/{ n2 = NR } n2 && NR >= n2 + 3 && NR <= n2 + 6 { print }' ring_slot/src/lib.rs
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| EV9 | `ring_event` | n/a — observation | The crate declares no struct, enum, union or type alias anywhere under `src/`, one of four such crates among the family's 33, and the only one of the four that declares traits rather than free functions over the family's concrete vocabulary — `ring_index`, `ring_seqno` and `ring_wait` all name `Seq`, `Capacity`, `SlotIndex`, `WaitKind` or `CursorPair` in their signatures while this crate names no concrete type in any parameter position at all; the representation vocabulary is correspondingly empty and the design is stated only once, in a doc comment at `:43` scoped to one function's implementation rather than to the crate it explains |
| EV10 | `ring_event` | n/a — doc gap | Declaring no data means inheriting every property of `ring_slot`'s two shapes, including the ones they do not share: both derive `Debug`, `Clone`, `PartialEq`, `Eq`, but `TypedSlot< T >` is `Option< T >` so equal payloads are equal slots, while `BytesSlot< N >` derives equality over all `N` bytes including the tail past `len` that `read()` promises never to return — so two byte slots this crate reads identically can compare unequal, a divergence the crate is structurally unable to state (no declaration to hang it on), detect (it never touches representation) or expose (`drain_from` forwards to the very function that masks it), and which the module doc's "neither function can tell the two slot shapes apart" invites a reader to read one step past |
| EV11 | `ring_slot` | **wrong doc** | `Slot::clear`'s contract, then at `:43` — "Return the slot to its empty state, dropping whatever it held" — is true of `TypedSlot`, whose impl assigns `None` and runs the destructor, and false of `BytesSlot`, whose impl writes `self.len = 0` and leaves the array untouched: a probe recycling `b"secret!!"` finds all eight bytes still resident in a slot that reports `is_empty()`, observable through the derived `Debug` and through the derived `PartialEq`, which makes a recycled slot rewritten with `b"hi"` compare unequal to a fresh slot written with the same `b"hi"` while both read identically — and "dropping whatever it held" is exactly the sentence a reader checks when asking whether a recycled slot still holds the previous payload |
| EV12 | `ring_event` | **latent hazard** | `recycle` is the one operation of three with no return value, so nothing in its signature can show the shapes diverging, and its stated rationale at `:216-218` — a shape-specific reset "would be a fourth path the identical-path claim does not cover" — nominates it as the place the claim matters most; `recycling_empties_either_shape_through_the_same_call` recycles both shapes and then asserts only `drain_from( … ) == None`, which for the byte shape consults `is_empty()`, which reads `len`, the one field `clear` writes, so the assertion cannot fail whatever the array holds, while `a_recycled_storage_slot_stops_returning_the_previous_lap` states the property `ring_shutdown` depends on and then instantiates `Buffer< TypedSlot< u32 > >` — `Buffer< BytesSlot< 8 > >` being a configuration `ring_store` constructs in its own doctest and suite, in which the previous world's payload survives the reset with no test anywhere looking, and `assert_eq!( bytes, BytesSlot::< 8 >::empty() )` is one line that fails today |
