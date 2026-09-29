# Data Structure: One Byte, and 253 Spare Niches

### Scope

**Purpose:** Record `Resolution` as a layout — three fieldless variants in one
byte — and what that layout gives a caller for free.

**Responsibility:** The enum's declaration, its measured size and alignment, and
the niche that makes `Option< Resolution >` cost nothing.

**In Scope:** `ring_overflow/src/lib.rs:55-76`.

**Out of Scope:** The 24-byte shape `resolve` returns it inside is
[`data_structure/002`](002_a_one_byte_outcome_in_a_twenty_four_byte_result.md).
What the derives enable is
[`type/001`](../type/001_six_derives_on_a_fieldless_enum.md).

---

## The Declaration, and What It Measures

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the whole data structure --'
command grep -m1 -A10 -F '#[ derive( Debug, Clone, Copy, PartialEq, Eq, Hash ) ]' ring_overflow/src/lib.rs | command grep -v '^  ///'
echo '  -- every field in the crate --'
command grep -c '  [a-z_]* : ' ring_overflow/src/lib.rs || true
echo '  -- and the constant that now pins its count --'
command grep -m1 -F '  pub const ALL : [ Self; 3 ] = [ Self::DroppedIncoming, Self::EvictedOldest, Self::Refused ];' ring_overflow/src/lib.rs
echo '  -- against the enum it is indexed by --'
command grep -m1 -A7 -F '  /// Discard the item being published; the ring'"'"'s contents are untouched.' ring_types/src/policy.rs | command grep -v '^  ///'
```

Live output:

```
  -- the whole data structure --
#[ derive( Debug, Clone, Copy, PartialEq, Eq, Hash ) ]
pub enum Resolution
{
  DroppedIncoming,
  -- every field in the crate --
0
  -- and the constant that now pins its count --
  pub const ALL : [ Self; 3 ] = [ Self::DroppedIncoming, Self::EvictedOldest, Self::Refused ];
  -- against the enum it is indexed by --
  #[ default ]
  DropNewest,
  DropOldest,
  Fail,
}
```

Measured, on `aarch64-unknown-linux-gnu`:

```
  -- what each shape costs to return, in bytes --

    type                                 size  align
    OverflowPolicy                          1      1
    Resolution                              1      1
    Option< Resolution >                    1      1
    RingError                              24      8
    Result< Resolution, RingError >        24      8
    Result< Resolution, () >                1      1
```

---

### OV9 — Three Variants, No Fields, One Byte, and No Cost to Wrap

`Resolution` carries no data at all. Its three variants are discriminants, it
occupies one byte with alignment one, and the crate declares zero struct fields
anywhere — the census above finds none.

That is the cheapest a named outcome can be, and it is what makes the type's
design argument affordable: `Resolution` exists to replace a boolean, and it
costs exactly what a boolean costs.

**Finding.** The layout also comes with 253 unused discriminant values, and the
compiler spends one of them: `Option< Resolution >` measures one byte, the same
as `Resolution` itself. So a caller who wants "a resolution, or none yet" pays
nothing for the option, and `Result< Resolution, () >` is likewise one byte.

Neither shape is used anywhere in the crate or its consumer, and the free niche
is not recorded as available. It matters because the one shape the crate does
return is 24 bytes
([`data_structure/002`](002_a_one_byte_outcome_in_a_twenty_four_byte_result.md)) —
so the type is designed to be cheap and is delivered inside something that is
not, and the measurement showing both sits nowhere in the source.

---

### OV10 — The Outcome Enum and the Policy Enum Have the Same Shape and No Relation the Compiler Knows

`OverflowPolicy` is three fieldless variants in one byte. `Resolution` is three
fieldless variants in one byte. The mapping between them is total, injective, and
written out twice
([`algorithm/001`](../algorithm/001_two_mappings_over_three_policies.md)).

The two declarations differ by exactly one derive, and it is the right one.
`OverflowPolicy` derives `Default` and marks `DropNewest` with `#[ default ]`,
because a ring built without a stated policy needs one. `Resolution` derives the
same six traits and not `Default`, because there is no default outcome — every
resolution is caused by an event, and a zero-argument constructor for one would
be a value nothing produced.

What the two share is that there is no `#[ repr ]` on either, no discriminant
assignment, and nothing that would let a reader assume
`DropNewest as u8 == DroppedIncoming as u8`. Both enums are ordered the same way
in their declarations, and the correspondence between position 0 and position 0
is coincidence maintained by hand.

**Finding.** The near-identical shape is a small trap for exactly the
optimisation someone will eventually propose: `Resolution` could be derived from
`OverflowPolicy` by transmute or by `as` cast, and it must not be, because
nothing pins the discriminants. `would_resolve`'s three-arm `match` is what makes
the relation explicit, and the compiler will preserve it — a fourth policy is a
compile error at both `match` sites.

Recording it because the two declarations are eight lines apart in two crates,
look interchangeable, and are related only by code that a future author might
read as redundant. `ring_types` publishes `OverflowPolicy::ALL` as a constant and
asserts its length; `Resolution` had no equivalent, so a fourth *resolution*
variant compiled against everything except this crate's own two matches.

`Resolution::ALL` now exists and is asserted the same way, so the asymmetry in how
the two counts are guarded is gone. The rest of this finding is unchanged and
deliberately so: there is still no `#[ repr ]` and no assigned discriminant on
either enum, because pinning them is exactly what would make the transmute this
finding warns about look legitimate. The correspondence stays maintained by two
`match` blocks, which is the point.

**Disposition:** applied — `Resolution::ALL` added to
`ring_overflow/src/lib.rs` mirroring `OverflowPolicy::ALL`, and pinned by
`resolution_has_exactly_three_variants_and_no_overwrite`, which now asserts the
length, that every member is named by the exhaustive helper, and that no member
repeats. The `matches!` half of the same asymmetry was closed in the same pass —
see [`invariant/001`](../invariant/001_the_mapping_is_total_and_injective.md)
§ OV22. The discriminant half is declined by design, per the paragraph above. Now prints: `  pub const ALL : [ Self; 3 ] = [ Self::DroppedIncoming, Self::EvictedOldest, Self::Refused ];`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`data_structure/002`](002_a_one_byte_outcome_in_a_twenty_four_byte_result.md) | The shape this byte is returned inside |
| [`type/001`](../type/001_six_derives_on_a_fieldless_enum.md) | What the derives make of a fieldless enum |
| [`algorithm/001`](../algorithm/001_two_mappings_over_three_policies.md) | The mapping that relates the two enums |
| [`decisions/001`](../decisions/001_the_fourth_variant_that_is_not_there.md) | The variant deliberately absent from this declaration |

### Sources

| Fact | Where |
|------|-------|
| The enum declaration and its derives | `ring_overflow/src/lib.rs:55-76` |
| Zero struct fields in the crate | Census above |
| `OverflowPolicy`'s three variants and its `Default` | `ring_types/src/policy.rs:104`, `:106-113` |
| Sizes and alignments | Probe, quoted above |
| `OverflowPolicy::ALL` and its length assertion | `ring_types/src/policy.rs:131`; `ring_types/tests/types_test.rs:200` |

### Tests

| Test | Covers |
|------|--------|
| `a_resolution_is_a_plain_comparable_value` | `Copy`, `PartialEq` and `Hash` on the layout |
| `resolution_has_exactly_three_variants_and_no_overwrite` | The variant count, structurally |
| `distinct_policies_give_distinct_resolutions` | That the two enums stay in bijection |
| *(to create)* | Nothing asserts a size, so a payload added to a variant would pass |
