# Data Structure: The Niche `Option` Finds, and the Array That Has None

### Scope

**Purpose:** Record that `TypedSlot< T >` is byte-identical to `Option< T >` — the
newtype costs nothing and inherits `T`'s niches transitively — that
`BytesSlot< N >` has no niche at any `N`, and what that asymmetry costs a caller
who wraps a slot.

**Responsibility:** The layout consequences of the two shapes' representations:
a niche-carrying `Option` against a niche-free array-plus-length.

**In Scope:** `ring_slot/src/lib.rs:83-84, 237-241`; the measured sizes of
both shapes and of `Option` over each.

**Out of Scope:** The absolute size of `BytesSlot< N >` and whether the length
field should be narrower are
[`data_structure/001`](001_sixteen_bytes_to_carry_eight.md). What the two shapes
*mean* is [`decisions/001`](../decisions/001_two_shapes_rather_than_one.md).

---

## The Two Representations

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -B1 -F 'pub struct TypedSlot< T >( Option< T > );' ring_slot/src/lib.rs
command grep -m1 -A7 -F '/// assert!( !slot.is_empty() );' ring_slot/src/lib.rs | tail -n 6
```

Live output:

```
#[ derive( Debug, Clone, PartialEq, Eq ) ]
pub struct TypedSlot< T >( Option< T > );
/// slot.clear();
/// assert_eq!( slot, BytesSlot::< 16 >::empty(), "clear returns it to a fresh slot's value" );
/// assert_eq!( format!( "{slot:?}" ), "BytesSlot { payload: [] }" );
/// ```
#[ derive( Clone ) ]
pub struct BytesSlot< const N : usize >
```

One is a newtype over `Option`; the other is two plain fields. Everything below
follows from that.

---

### SL23 — `TypedSlot< T >` Is Exactly Its `Option`, at Every `T`

Measured, release:

```
--- TypedSlot: exactly its Option ---
TypedSlot< u32 >           size   8  align  4
Option< u32 >              size   8  align  4
TypedSlot< () >            size   1  align  1
TypedSlot< String >        size  24  align  8
Option< String >           size  24  align  8
```

Byte for byte, at both a niche-free payload (`u32` — 4 bytes plus a discriminant
word) and a niche-carrying one (`String` — 24 bytes, the discriminant folded into
`Vec`'s capacity field). The newtype wrapper is free; the slot is its `Option`
with a different name.

That extends to nesting:

```
--- nesting: does the outer Option find a niche? ---
TypedSlot< String >        size  24  align  8
Option< TypedSlot< String > > size  24  align  8
TypedSlot< u32 >           size   8  align  4
Option< TypedSlot< u32 > > size   8  align  4
```

Both free. `String`'s case is the interesting one — the inner `Option` already
consumed a discriminant value out of the niche, and the outer one finds another,
because the niche `Vec` exposes is a range rather than a single spare value.
`u32`'s case is free for the duller reason: `Option< u32 >` has three spare bytes
of padding next to its discriminant, and the outer `Option` reuses one.

**Finding.** `TypedSlot< T >` costs nothing over `Option< T >` and composes into
further `Option` layers for free. Two consequences follow, and only the first is
obvious.

The obvious one: `TypedSlot` is the cheap shape, and `TypedSlot< () >` at one
byte is the cheapest possible payload-free signal in the family — which is what
makes it the right answer to the zero-length conflation
([`decisions/002`](../decisions/002_a_length_rather_than_a_flag.md) SL8) rather
than merely an available one.

The less obvious one: because the slot *is* its `Option`, every layout property a
caller might reason about is `T`'s, not the crate's. `size_of::< TypedSlot< T > >()`
changes when `T` does and the crate has no say. That is correct for a transparent
container, and it means the crate's own cost documentation can only ever be about
the wrapper, which is zero.

---

### SL24 — `BytesSlot< N >` Has No Niche at Any `N`, So Every Wrapping Layer Costs a Full Word

`[ u8; N ]` has no invalid bit patterns — every one of the 2^(8N) values is a
legal array. `usize` has none either. So the struct has nowhere to hide a
discriminant:

```
BytesSlot< 8 >             size  16  align  8
Option< BytesSlot< 8 > >   size  24  align  8
```

Eight bytes for one bit, and it does not improve at larger `N` — the array grows,
the niche stays absent, and the cost stays a full word.

Nothing in the family pays it today:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r 'Option< *\(Typed\|Bytes\)Slot' ring_*/src/*.rs ring_*/tests/*.rs 2>/dev/null \
  || echo '  no slot is wrapped in an Option anywhere in the family'
```

Live output:

```
  no slot is wrapped in an Option anywhere in the family
```

**Finding.** The asymmetry is real, measured, and currently latent. No crate
wraps a slot in an `Option`, so nobody pays the eight bytes — but the shape that
would pay them is also the shape that is already half overhead at the sizes in
use ([`data_structure/001`](001_sixteen_bytes_to_carry_eight.md) SL21), so the
two costs compound rather than trade off: a `Option< BytesSlot< 8 > >` would be
24 bytes to carry 8.

The reason it stays latent is worth naming, because it is not luck. Slots live in
a `Buffer< S >` as a boxed slice, indexed rather than owned individually, and
emptiness is asked with `is_empty` rather than represented as `None` — the crate
built its own emptiness signal instead of leaning on `Option`, so no caller ever
needs a second one. `TypedSlot` reaching for `Option` internally and `BytesSlot`
not is exactly what makes the two shapes' external behaviour identical.

What would break it is an API returning `Option< BytesSlot< N > >` by value — a
`try_take` or a fallible constructor. Neither exists, and if one is added the
right shape is `Result< BytesSlot< N >, _ >` with an error type carrying its own
niche, or a `&mut` out-parameter. That is a design constraint the crate has not
had to state, and it is the kind that gets discovered by measurement after the
fact rather than by reading the struct.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_slot
command grep -F 'no spare bit to hide a discriminant in' src/lib.rs
```

Live output:

```
/// no spare bit to hide a discriminant in — wrapping it in `Option` costs a
```

**Disposition:** applied — `BytesSlot`'s own doc comment now states the design
constraint this finding says the crate has not had to state: no bit pattern is
spare, so wrapping it in `Option` costs a full word at any `N`, and a future
fallible API over this type should prefer `Result` with a niche-carrying error
instead of `Option< BytesSlot< N > >`.
Now prints: `no spare bit to hide a discriminant in`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`data_structure/001`](001_sixteen_bytes_to_carry_eight.md) | What a `BytesSlot` costs before anything wraps it |
| [`decisions/001`](../decisions/001_two_shapes_rather_than_one.md) | Why there are two shapes for these two representations to differ over |
| [`decisions/002`](../decisions/002_a_length_rather_than_a_flag.md) | The distinction `TypedSlot`'s discriminant carries and the length cannot |
| [`type/001`](../type/001_two_shapes_one_trait_no_copy.md) | What the two representations admit as payload |

### Sources

| Fact | Where |
|------|-------|
| The newtype over `Option` | `ring_slot/src/lib.rs:83-84` |
| The array plus length | `ring_slot/src/lib.rs:237-241` |
| No slot wrapped in an `Option` | `ring_*/src`, `ring_*/tests` — no occurrence |
| Every size and alignment figure | Release probe, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `a_typed_slot_holds_non_copy_payloads` | A `String` payload — the niche-carrying case measured above |
| `a_slot_holding_a_default_value_is_still_occupied` | That the discriminant, not the payload, answers emptiness |
| `a_fresh_bytes_slot_is_empty` | `BytesSlot`'s emptiness signal, built rather than borrowed from `Option` |
| *(to create)* | A layout assertion that `size_of::< TypedSlot< T > >() == size_of::< Option< T > >()` |
