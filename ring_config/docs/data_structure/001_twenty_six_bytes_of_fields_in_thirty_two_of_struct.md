# Data Structure: Twenty-Six Bytes of Fields in Thirty-Two of Struct

### Scope

**Purpose:** Record the record's measured layout — its size, alignment, the six
bytes it does not use, and the one property those unused bytes buy back.

**Responsibility:** The five field types, the padding, and the niche that makes
`Option< RingConfig >` free.

**In Scope:** `ring_config/src/lib.rs:41-49`;
`ring_types/src/capacity.rs:22-23`; `ring_types/src/policy.rs:21-22`,
`:104-105`.

**Out of Scope:** What passing the record costs is
[`data_structure/002`](002_four_times_a_reference_and_nobody_pays_it.md). Which
derives are absent, and why, is
[`type/001`](../type/001_five_derives_and_the_one_that_is_free_and_absent.md).

---

## The Five Fields and Their Types

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the record as declared --'
command grep -m1 -A8 -F '#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]' ring_config/src/lib.rs
echo '  -- and the field types it is built from --'
command grep 'derive\|pub struct Capacity\|pub enum WaitKind\|pub enum OverflowPolicy' ring_types/src/capacity.rs ring_types/src/policy.rs | sed 's|^ring/||'
```

Live output:

```
  -- the record as declared --
#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
pub struct RingConfig
{
  capacity : Capacity,
  wait : WaitKind,
  overflow : OverflowPolicy,
  producers : usize,
  batch : usize,
}
  -- and the field types it is built from --
ring_types/src/capacity.rs:#[ derive( Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash ) ]
ring_types/src/capacity.rs:pub struct Capacity( usize );
ring_types/src/policy.rs:#[ derive( Debug, Clone, Copy, PartialEq, Eq, Hash, Default ) ]
ring_types/src/policy.rs:pub enum WaitKind
ring_types/src/policy.rs:#[ derive( Debug, Clone, Copy, PartialEq, Eq, Hash, Default ) ]
ring_types/src/policy.rs:pub enum OverflowPolicy
```

## The Measured Layout

A probe linking `ring_config` and `ring_types` and reporting `size_of` and
`align_of` for the record and each of its field types. It ran twice, unchanged,
on `aarch64-unknown-linux-gnu` with `--release`:

```rust
println!( "RingConfig         size {:>3}  align {}", size_of::< RingConfig >(), align_of::< RingConfig >() );
println!( "  Capacity         size {:>3}  align {}", size_of::< Capacity >(), align_of::< Capacity >() );
println!( "  WaitKind         size {:>3}  align {}", size_of::< WaitKind >(), align_of::< WaitKind >() );
println!( "  OverflowPolicy   size {:>3}  align {}", size_of::< OverflowPolicy >(), align_of::< OverflowPolicy >() );
println!( "  usize            size {:>3}  align {}", size_of::< usize >(), align_of::< usize >() );

let sum = size_of::< Capacity >()
  + size_of::< WaitKind >()
  + size_of::< OverflowPolicy >()
  + size_of::< usize >() * 2;
println!( "field bytes {}   struct bytes {}   padding {}", sum, size_of::< RingConfig >(), size_of::< RingConfig >() - sum );
println!( "&RingConfig        size {:>3}", size_of::< &RingConfig >() );
println!( "Option<RingConfig> size {:>3}", size_of::< Option< RingConfig > >() );
```

```
RingConfig         size  32  align 8
  Capacity         size   8  align 8
  WaitKind         size   1  align 1
  OverflowPolicy   size   1  align 1
  usize            size   8  align 8
field bytes 26   struct bytes 32   padding 6
&RingConfig        size   8
Option<RingConfig> size  32
```

---

### RC9 — Six of Thirty-Two Bytes Are Padding, and No Field Order Removes Them

Three fields are eight bytes each — `Capacity`, which is a `usize` newtype, and
the two raw `usize` counts — and two are one byte, since `WaitKind` and
`OverflowPolicy` are fieldless enums. Twenty-six bytes of field in a
thirty-two-byte struct at alignment eight.

Reordering does not help. `repr(Rust)` already reorders freely, and any
arrangement of three eight-byte fields and two one-byte fields rounds up to
thirty-two under alignment eight. The six bytes are a consequence of the field
*types*, not of their declaration order.

**Finding.** What would remove them is narrowing the two counts. `producers` is a
thread count and `batch` is bounded above by `capacity`, so neither needs
sixty-four bits; `u32` for both gives eighteen bytes of field in twenty-four bytes
of struct — a quarter smaller, with three bytes of padding instead of six.

Recording it rather than recommending it, because the case for the change is weak
on its own terms: the record is built once per ring and read through a reference
by every low-level consumer
([`data_structure/002`](002_four_times_a_reference_and_nobody_pays_it.md)), so
eight bytes saved is eight bytes nobody was moving. It is worth knowing that six
of the record's thirty-two bytes hold nothing, and worth knowing the reason is two
`usize` fields that could each be half the width.

---

### RC10 — The Two One-Byte Enums Make `Option< RingConfig >` Free

`size_of::< Option< RingConfig > >()` is thirty-two — identical to
`size_of::< RingConfig >()`. The compiler finds a niche and stores the `None`
discriminant inside the record rather than beside it.

The niche is in the enums. `WaitKind` and `OverflowPolicy` occupy one byte each
and have far fewer than 256 variants, so both carry unused bit patterns the
optimizer can claim. Neither `Capacity` — a transparent-in-practice `usize`
newtype with no `NonZero` inside it — nor either raw `usize` offers one.

**Finding.** So an optional configuration costs nothing over a mandatory one, and
nothing in the crate says so. That is exactly the property a caller needs when
deciding between `Option< RingConfig >` and a sentinel value, or between storing a
configuration and storing a builder — and it is the sort of property that is
invisible unless measured and silently lost if a field type ever changes.

It also connects to RC9 in a way worth stating: narrowing `producers` and `batch`
to `u32` would shrink the record to twenty-four bytes and leave the niche intact,
since the niche comes from the enums and not the integers. The two changes do not
compete.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`data_structure/002`](002_four_times_a_reference_and_nobody_pays_it.md) | What moving these thirty-two bytes costs, and who moves them |
| [`type/001`](../type/001_five_derives_and_the_one_that_is_free_and_absent.md) | The derive list on `:41`, and the one derive every field type has that it does not |
| [`non_functional_requirement/001`](../non_functional_requirement/001_a_record_that_allocates_nothing_and_is_read_once.md) | The record's cost profile as a requirement |

### Sources

| Fact | Where |
|------|-------|
| The five fields as declared | `ring_config/src/lib.rs:41-49` |
| `Capacity` is a `usize` newtype with no niche | `ring_types/src/capacity.rs:22-23` |
| Both policy enums are fieldless and one byte | `ring_types/src/policy.rs:21-22`, `:104-105` |
| 32 bytes, align 8, 26 of field, 6 of padding | Probe above, two runs |
| `Option< RingConfig >` is also 32 bytes | Probe above, two runs |

### Tests

| Test | Covers |
|------|--------|
| `every_named_field_is_carried` | That all five fields are present and reported |
| `the_record_is_copy_and_compares_by_value` | The `Copy` and `PartialEq` derives the layout supports |
| `defaults_are_the_documented_ones` | The four fields `new` fills without an argument |
