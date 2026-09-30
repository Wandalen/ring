# Type: The Types That Cross the Boundary

### Scope

**Purpose:** Record that of the three `ring_types` types in this crate's
signatures exactly one is validated — the one whose field is private — that the
panic in `get` rests on the two that are not, and that `Buffer` derives one trait
where its own capacity derives eight.

**Responsibility:** The three crossing types as types: their fields, their
constructors, their derives, and what each guarantees at the boundary.

**In Scope:** `ring_types/src/capacity.rs:22-52`;
`ring_types/src/id.rs:24-25, 98-99`.

**Out of Scope:** Where each value appears and how long it lives is
[`item/002`](../item/002_the_slotindex_the_buffer_trusts.md). The panic itself is
[`decisions/001`](../decisions/001_panic_rather_than_option.md).

---

### BF48 — One of Three Is Validated, and It Is the One With a Private Field

The three declarations, with their derives:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A1 -F '#[ derive( Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash ) ]' ring_types/src/capacity.rs
awk '/^\/\/\/ assert_eq!\( Seq\( 3 \)\.next\(\), Seq\( 4 \) \);$/{ n1 = NR } n1 && NR >= n1 + 2 && NR <= n1 + 3 { print } /^\/\/\/ assert_eq!\( SlotIndex\( 3 \)\.get\(\), 3 \);$/{ n2 = NR } n2 && NR >= n2 + 2 && NR <= n2 + 3 { print }' ring_types/src/id.rs
```

Live output:

```
#[ derive( Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash ) ]
pub struct Capacity( usize );
#[ derive( Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default ) ]
pub struct Seq( pub u64 );
#[ derive( Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default ) ]
pub struct SlotIndex( pub usize );
```

`Capacity`'s field is private; the other two are public. And `Capacity` is the
one with a fallible constructor:

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/pub const fn new/,/^  }/p' ring_types/src/capacity.rs
```

Live output:

```
  pub const fn new( slots : usize ) -> Result< Self, RingError >
  {
    if slots == 0
    {
      return Err( RingError::CapacityZero );
    }
    if !slots.is_power_of_two()
    {
      return Err( RingError::CapacityNotPowerOfTwo( slots ) );
    }
    Ok( Self( slots ) )
  }
```

Exercised at the boundary:

```
--- (2) the three crossing types ---
  Capacity::new( 0 )  = Err(CapacityZero)
  Capacity::new( 6 )  = Err(CapacityNotPowerOfTwo(6))
  Capacity::new( 8 )  = Ok(Capacity(8))
  SlotIndex( 999 )    = SlotIndex(999)  (no validation, no fallible constructor)
  ring_types::Seq( u64::MAX ) constructs unconditionally
```

**Finding.** The split is coherent and it explains most of this crate's shape.
`Capacity` is a *proposition* — non-zero and a power of two — so it hides its
field and admits only values that satisfy it, which is what lets `mask()` exist,
what lets `new` size a slice without checking, and what makes `is_empty` a
constant. `Seq` and `SlotIndex` are *coordinates* — any value is meaningful, so
there is nothing to validate and nothing to hide.

The consequence lands here. Two of the three types the buffer accepts carry no
guarantee at all, and `get`'s bounds check is the only thing standing between an
arbitrary `SlotIndex` and a slice index. The panic doc argues that an
out-of-range index means two rings' capacities were mixed
([`decisions/001`](../decisions/001_panic_rather_than_option.md) BF7) — which is
a claim about provenance that the type does not make and cannot make, because
`SlotIndex( 999 )` is one expression away from anywhere.

One detail is worth naming for how neatly it encodes all of this: `Capacity`'s
derive list is the other two's minus `Default`. There is no default capacity,
because `Capacity( 0 )` is the value the constructor exists to reject, so the
derive that would manufacture it is absent. The proposition is enforced in the
derives as well as in the constructor.

---

### BF49 — `Buffer` Derives One Trait Where Its Own Capacity Derives Eight

```sh
cd "$(git rev-parse --show-toplevel)"
grep 'derive' ring_store/src/lib.rs
```

Live output:

```
/// # The derived `Debug` renders every slot
/// embeds a `Buffer` and derives `Debug` in turn inherits this cost and this
#[derive(Debug)]
```

**Finding.** A `Buffer` cannot be cloned, compared, hashed, ordered or defaulted.
Each absence is right and each is unstated:

- **No `Clone`.** Cloning storage would produce a second owner of payloads that a
  ring believes it alone holds, and for `BytesSlot< N >` would copy `N` bytes per
  slot regardless of occupancy (`ring_slot`'s SL35). A ring is not a value.
- **No `PartialEq`.** Two buffers cannot be compared, so a test asserting on
  contents walks `iter` and compares slots — which is the same borrow-only shape
  [`pattern/002`](../pattern/002_every_write_is_a_borrow.md) records, arriving
  from the derive side rather than the method side.
- **No `Default`.** `Buffer::default()` would need a capacity, and there is no
  default capacity — the same reason `Capacity` itself does not derive it. That
  the bound is `S : Default` while `Buffer` is not `Default` reads as an
  inconsistency until you notice the capacity is where it breaks.

The one derive present is the one the workspace's lint requires, and it is the
one with the sharpest edge ([`type/001`](001_one_parameter_three_impl_blocks_one_derive.md)
BF47). So the trait list is: everything useful omitted for good reasons nobody
wrote down, and the one thing that had to be there rendering three million
characters.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`type/001`](001_one_parameter_three_impl_blocks_one_derive.md) | The parameterised type these three sit beside |
| [`item/002`](../item/002_the_slotindex_the_buffer_trusts.md) | The same three types, as values with lifetimes |
| [`decisions/001`](../decisions/001_panic_rather_than_option.md) | The panic that rests on the two unvalidated types |
| [`algorithm/001`](../algorithm/001_one_mask_no_modulo.md) | The `mask()` that `Capacity`'s proposition makes possible |
| [`pattern/002`](../pattern/002_every_write_is_a_borrow.md) | The borrow-only shape the missing `PartialEq` reinforces |

### Sources

| Fact | Where |
|------|-------|
| The three declarations and their derives | `ring_types/src/capacity.rs:22-23`; `ring_types/src/id.rs:24-25, 98-99` |
| `Capacity`'s validation | `ring_types/src/capacity.rs:42-49` |
| The boundary behaviour of all three | Release probe, quoted above |
| `Buffer`'s single derive | `ring_store/src/lib.rs:58` |
| `BytesSlot`'s unconditional `Clone` cost | `ring_slot` — SL35 |

### Tests

| Test | Covers |
|------|--------|
| `a_buffer_is_exactly_its_slots_and_its_capacity` | Both fields, through the one derive |
| `a_buffer_is_never_empty_because_a_capacity_is_never_zero` | `Capacity`'s non-zero proposition, reaching the buffer |
| `an_index_past_the_capacity_panics_rather_than_wrapping` | An unvalidated `SlotIndex` meeting the bounds check |
| `ring_types` — `types_test.rs` | Both rejection paths, deliberately |
