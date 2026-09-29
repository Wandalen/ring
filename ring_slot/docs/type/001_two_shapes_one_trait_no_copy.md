# Type: Two Shapes, One Trait, No `Copy`

### Scope

**Purpose:** Record what each of the two types commits to as a payload channel,
that neither carries `Copy` while thirty-seven sibling types in the family do,
and that only one of the two could carry it.

**Responsibility:** The two concrete types as type-system commitments: what they
admit, what they refuse, and what their trait set forces on every consumer.

**In Scope:** `ring_slot/src/lib.rs:83-84, 237-241`; the family-wide
`Copy` census; `ring_store/src/lib.rs:204, 215, 246, 253`.

**Out of Scope:** `N` as a type parameter is
[`type/002`](002_the_const_parameter_as_capacity.md). What the derives *cost* is
[`non_functional_requirement/002`](../non_functional_requirement/002_four_traits_and_what_they_compare.md);
what `PartialEq` *compares* is
[`pitfall/001`](../pitfall/001_the_test_that_names_a_property_the_type_lacks.md).
Memory layout is
[`data_structure/001`](../data_structure/001_sixteen_bytes_to_carry_eight.md).

---

## The Two Declarations

```sh
cd "$(git rev-parse --show-toplevel)"
awk '/^\/\/\/ assert_eq!\( slot\.take\(\), Some\( 42 \) \);$/{ n1 = NR } n1 && NR >= n1 + 3 && NR <= n1 + 4 { print } /^\/\/\/ assert!\( !slot\.is_empty\(\) \);$/{ n2 = NR } n2 && NR >= n2 + 2 && NR <= n2 + 7 { print }' ring_slot/src/lib.rs
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

A newtype over `Option< T >` and a two-field struct over `[ u8; N ]`. One is
parameterised by a type, the other by a value. Both have private fields, so the
only way in or out is the inherent surface
([`api/001`](../api/001_ten_functions_six_const_seven_must_use.md)).

---

### SL45 — Thirty-Seven Types in the Family Carry `Copy`; Neither Slot Does

`Copy` is the family's default posture for small value types:

```sh
cd "$(git rev-parse --show-toplevel)"
printf '  derive( .. Copy .. ) across the 33 crates : %d\n' "$( grep -rhE '^#\[ derive\(.*Copy' ring_*/src/*.rs | wc -l )"
printf '  derive( .. Copy .. ) on a ring_slot type  : %d\n' "$( grep -cE '^#\[ derive\(.*Copy' ring_slot/src/lib.rs )"
```

Live output:

```
  derive( .. Copy .. ) across the 33 crates : 37
  derive( .. Copy .. ) on a ring_slot type  : 0
```

Sequence numbers, slot indices, capacities, policies, statistics snapshots — all
`Copy`. The two slot shapes are the exception, and the two are exceptions for
opposite reasons.

`TypedSlot< T >` cannot be `Copy` at all. `Copy` forbids a destructor, and the
type exists precisely to hold payloads that have one — a `TypedSlot< String >`
is exercised by the crate's own suite. A conditional `impl< T : Copy > Copy`
would be possible and would apply only to payloads that need no drop, which is
the narrow half the ring path does not use.

`BytesSlot< N >` could be. Its fields are `[ u8; N ]` and `usize`, both `Copy`,
and a mirror of it with the derive added compiles:

```
--- could BytesSlot be Copy? ---
  a Copy mirror compiles; both bindings live: 0 0
  size 24 bytes, copied implicitly on every move
```

**Finding.** The omission is right and unstated. `Copy` on a `BytesSlot< N >`
would make every move an implicit `N`-byte memcpy — the exact cost the shape is
built to avoid, applied silently at every binding, with no syntax at the call
site to show it happening
([`non_functional_requirement/002`](../non_functional_requirement/002_four_traits_and_what_they_compare.md)
SL35 measures the same copy under `Clone`, where at least it is spelled out).

But nothing records the reasoning, and the surrounding evidence points the other
way: thirty-seven siblings carry the derive — thirty-six when this was written,
and the one added since is `ring_stats::StatsCounts`, made `Copy` precisely so a
caller could take a counter set by value — `BytesSlot`'s fields all permit it,
and the crate offers no comment. A maintainer adding `Copy` for symmetry would
find every test passing and every bench slower, with the regression spread across
every binding rather than concentrated anywhere a profile would name.

---

### SL46 — The Two Payload Channels Are Different in Kind, and Neither Type Says So

What each shape admits is decided by its parameter, and the two admit
incomparable things:

```
--- what TypedSlot admits ---
  T with no Clone, no Copy, no Default, no PartialEq
  default()   -> empty: true
  set/take    -> Some(1)

--- what BytesSlot admits ---
  write 4 bytes -> Ok(())
  write 5 bytes -> true
  the payload channel is &[ u8 ] and nothing else
```

`TypedSlot< T >` places no bound on `T` anywhere — not on the struct, not on the
inherent impl, not on `Default`
([`non_functional_requirement/002`](../non_functional_requirement/002_four_traits_and_what_they_compare.md)
SL36 records why `Default` is hand-written to keep it that way). Any Rust type
whatsoever can be a payload, and the slot's own derives simply become conditional
on it.

`BytesSlot< N >` admits exactly one thing — a byte slice no longer than `N` — and
refuses everything else at runtime with a `Result`
([`workaround/002`](../workaround/002_batchtoolarge_borrowed_for_a_different_shape.md)).
There is no type-level way to hand it a payload it will reject.

Because neither type is `Copy`, a slot can never leave its buffer by value:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -E '^[[:space:]]*pub fn (get|get_mut|at|at_mut)' ring_store/src/lib.rs
```

Live output:

```
  pub fn get( &self, index : SlotIndex ) -> &S
  pub fn get_mut( &mut self, index : SlotIndex ) -> &mut S
  pub fn at( &self, seq : Seq ) -> &S
  pub fn at_mut( &mut self, seq : Seq ) -> &mut S
```

Four accessors, four references, no by-value form. That is what forces the
family's drain idiom — `get_mut( i ).and_then( TypedSlot::take )` — to reach
*through* a borrow to move the payload rather than the slot
([`lifecycle/001`](../lifecycle/001_a_slot_across_one_publish.md) contrasts the
two models this produces).

**Finding.** The choice is coherent: slots are fixtures of a pre-allocated
buffer, so borrowing them is the only sensible access, and refusing `Copy` is
what makes the compiler enforce it. The undocumented part is the consequence for
someone writing a new shape. A third implementor of `Slot` that *was* `Copy`
would compile, would satisfy every bound in the family, and would quietly
reintroduce by-value slot access — nothing in the trait, the buffer, or this
crate's documentation states that not being `Copy` is a requirement rather than
an accident of the two types that exist.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`type/002`](002_the_const_parameter_as_capacity.md) | `N` as the second shape's parameter, and what fixing it at compile time costs |
| [`data_structure/002`](../data_structure/002_the_niche_option_finds_and_the_array_does_not.md) | The two representations these declarations produce |
| [`non_functional_requirement/002`](../non_functional_requirement/002_four_traits_and_what_they_compare.md) | The four traits, three now hand-written on `BytesSlot`, and who requires them |
| [`api/002`](../api/002_a_trait_with_two_methods.md) | The trait both types implement, and its lack of supertraits |
| [`pattern/001`](../pattern/001_one_trait_two_shapes.md) | How these two types are dispatched |

### Sources

| Fact | Where |
|------|-------|
| The two declarations | `ring_slot/src/lib.rs:83-84, 237-241` |
| 37 derived `Copy` in the family, 0 here | `ring_*/src/*.rs` |
| The four reference-only accessors | `ring_store/src/lib.rs:204, 215, 246, 253` |
| A `Copy` mirror of `BytesSlot` compiling | Release probe, quoted above |
| An unbounded `T` accepted end to end | Release probe, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `a_typed_slot_holds_non_copy_payloads` | A `String` payload — the case `Copy` would forbid |
| `both_shapes_drive_through_the_trait_alone` | Both types behind one bound |
| `slots_compare_by_payload_not_by_tail` | `PartialEq`, the one derive with coverage |
| `a_bytes_slot_round_trips_its_payload` | The `&[ u8 ]` channel, end to end |
| *(to create)* | A payload with no derives at all, pinning `TypedSlot< T >`'s unbounded `T` |
