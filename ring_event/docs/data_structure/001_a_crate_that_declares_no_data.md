# Data Structure: A Crate That Declares No Data

### Scope

**Purpose:** Record that this crate declares no data at all, place that among the
33, and establish what follows from it — every representational property of a
round trip belongs to `ring_slot`, including the ones the two shapes do not share.

**Responsibility:** The zero-type census, the four crates in the same position,
the absent representation vocabulary, and the two derives this crate inherits
without being able to say anything about them.

**In Scope:** `ring_event/src/lib.rs:36-37`, `:43`;
`ring_slot/src/lib.rs:83-84`, `:236-241`.

**Out of Scope:** What that inheritance costs at the one operation where the two
shapes diverge is [`data_structure/002`](002_what_clear_leaves_behind.md). The
one type-level declaration the crate does make is
[`type/001`](../type/001_an_associated_type_with_a_lifetime.md).

---

## Nothing Declared, and What Is Borrowed Instead

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- structs, enums, unions and type aliases declared by this crate --'
command grep -r '^pub struct\|^pub enum\|^pub union\|^pub type\|^struct \|^enum ' --include=*.rs ring_event/src | wc -l
echo '  -- ring crates declaring no data type of their own, out of how many --'
for c in ring_*/; do
  n=$( command grep -rc '^pub struct\|^pub enum\|^pub union\|^pub type\|^struct \|^enum ' --include=*.rs $c/src 2>/dev/null | awk -F: '{ s += $2 } END{ print s+0 }' )
  [ "$n" = 0 ] && basename $c
done
ls -d ring_*/ | wc -l
echo '  -- what each of the four declares instead --'
for c in ring_event ring_index ring_seqno ring_wait; do
  printf '%-12s traits %s  fns %s\n' "$c" "$( command grep -c '^pub trait' $c/src/lib.rs || true )" "$( command grep -c '^pub \(const \)\?fn' $c/src/lib.rs || true )"
done
echo '  -- representation vocabulary anywhere in the crate --'
for w in 'repr' 'size_of' 'align_of' 'transmute' 'as_ptr'; do printf '%-10s %s\n' "$w" "$( command grep -c "$w" ring_event/src/lib.rs || true )"; done
echo '  -- the only line in the crate naming a data shape at all --'
command grep 'struct \|enum \|union ' ring_event/src/lib.rs
echo '  -- what the two shapes it unifies derive, and where they come from --'
command grep -m1 -A1 -F 'use ring_slot::{ BytesSlot, Slot, TypedSlot };' ring_event/src/lib.rs
awk '/^\/\/\/ assert_eq!\( slot\.take\(\), Some\( 42 \) \);$/{ n1 = NR } n1 && NR >= n1 + 3 && NR <= n1 + 4 { print } /^\/\/\/ assert!\( !slot\.is_empty\(\) \);$/{ n2 = NR } n2 && NR >= n2 + 2 && NR <= n2 + 7 { print }' ring_slot/src/lib.rs
```

Live output:

```
  -- structs, enums, unions and type aliases declared by this crate --
0
  -- ring crates declaring no data type of their own, out of how many --
ring_event
ring_index
ring_seqno
ring_wait
33
  -- what each of the four declares instead --
ring_event   traits 2  fns 3
ring_index   traits 0  fns 3
ring_seqno     traits 0  fns 5
ring_wait    traits 0  fns 6
  -- representation vocabulary anywhere in the crate --
repr       0
size_of    0
align_of   0
transmute  0
as_ptr     0
  -- the only line in the crate naming a data shape at all --
/// and no enum of shapes.
  -- what the two shapes it unifies derive, and where they come from --
use ring_slot::{ BytesSlot, Slot, TypedSlot };
use ring_types::RingError;
#[ derive( Debug, Clone, PartialEq, Eq ) ]
pub struct TypedSlot< T >( Option< T > );
/// slot.clear();
/// assert_eq!( slot, BytesSlot::< 16 >::empty(), "clear returns it to a fresh slot's value" );
/// assert_eq!( format!( "{slot:?}" ), "BytesSlot { payload: [] }" );
/// ```
#[ derive( Clone ) ]
pub struct BytesSlot< const N : usize >
```

---

### EV9 — Four of Thirty-Three Crates Declare No Data, and This Is the Only One That Declares Traits Instead

The census returns zero: no struct, no enum, no union, no type alias anywhere
under `src/`. Four of the family's 33 crates are in that position —
`ring_event`, `ring_index`, `ring_seqno`, `ring_wait` — so declaring nothing is
unusual but not unique.

What separates this one from the other three is what fills the gap. `ring_index`,
`ring_seqno` and `ring_wait` declare zero traits and between three and six free
functions each, and every one of those functions names the family's concrete
vocabulary in its signature: `Seq`, `Capacity`, `SlotIndex`, `WaitKind`,
`CursorPair`. They are function crates over types someone else declared.
`ring_event` declares two traits and three functions, and the functions name no
concrete type in any parameter position at all — only `S` and `P`, bound by the
two traits above them.

The representation vocabulary is correspondingly empty: `repr`, `size_of`,
`align_of`, `transmute` and `as_ptr` each appear zero times. The crate never
reads a byte, never asks how large anything is, and never assumes a layout.

**Finding.** The absence is deliberate and the crate half-says so: the only line
in the crate carrying the word `struct`, `enum` or `union` is a doc comment at
`:43` explaining that implementing `Fill` on the payload is what lets
`publish_into` need "no match, no downcast and no enum of shapes."

That sentence is scoped to one function's implementation. Read as a statement
about the crate it would be the design in one line — the alternative to two
traits was one enum with two variants and a match in every function, and the
enum is what would have to change every time a shape is added. Nothing states it
at that scope, so the fact that this crate is the family's one *shape-agnostic*
crate rather than merely another type-less one has to be reconstructed from the
census above.

---

### EV10 — Declaring No Data Means Inheriting Every Property of the Data, Including the Ones the Two Shapes Do Not Share

*Written before `PartialEq` was narrowed.* The reasoning below is the record
of what was found; the Disposition at the end of this section says what the
rewrite did to each step.

Both shapes derive the same four traits — `Debug`, `Clone`, `PartialEq`, `Eq` —
and on that evidence they look interchangeable. Their fields say otherwise.
`TypedSlot< T >` is `Option< T >`: one field, and the derived `PartialEq`
compares exactly the payload. `BytesSlot< N >` is `[ u8; N ]` plus a `len`, and
the derived `PartialEq` compares all `N` bytes, including whatever sits past
`len` — bytes that `read()` deliberately never returns, since its doc at
`ring_slot:358` promises "the bytes written, and only those — never the unused
tail."

So the two shapes disagree about what equality means. For one, equal values are
equal slots. For the other, equal values may sit in unequal slots, because the
slot carries state the value does not.

**Finding.** This crate is where a caller meets both shapes through one
signature, and it is structurally unable to say anything about it. It declares no
data, so it has no declaration to hang the caveat on; it never touches
representation, so it cannot detect the difference; and its own read path masks
it, because `drain_from` forwards to `read()` and `read()` is exactly the
function that hides the tail.

The module documentation states the property it does guarantee — "neither
function can tell the two slot shapes apart" (`:21-22`) — and that is true of the
functions. It is not true of the values, and the distinction is recorded nowhere.
A caller who reads the guarantee as *these two shapes behave alike* has read one
step further than the sentence supports, and the crate offers nothing that would
stop them. What that costs at the one operation where the divergence becomes
observable is [`data_structure/002`](002_what_clear_leaves_behind.md).

**Disposition:** applied. `BytesSlot`'s `PartialEq` is no longer derived. It is
hand-written over `read()`, along with `Debug` and `Eq`, at
`ring_slot/src/lib.rs:248-267`, leaving `#[ derive( Clone ) ]` alone on
the shape at `:236`. Three steps of the reasoning fall with it: the two shapes
no longer derive the same four traits, no derived `PartialEq` compares the
bytes past `len`, and the two no longer disagree about what equality means —
both now compare the value `read()` returns. The census block above was
re-recorded after that change, and its last two lines show the surviving
derive. "The distinction is recorded nowhere" has expired too: it is recorded
in [`data_structure/002`](002_what_clear_leaves_behind.md) and in `ring_slot`'s
[`non_functional_requirement/002`](../../../ring_slot/docs/non_functional_requirement/002_four_traits_and_what_they_compare.md).

What this section is about survives all of that. The divergence was narrowed,
not removed: `clear()` still drops the payload on one shape and only zeroes
`len` on the other, so the bytes past `len` still outlive a clear — equality
simply stopped being the place that shows it. This crate still declares no
data, still never names a representation, and still has nowhere to hang the
caveat. That is the finding, and it is unchanged.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`data_structure/002`](002_what_clear_leaves_behind.md) | The inherited divergence, demonstrated |
| [`type/001`](../type/001_an_associated_type_with_a_lifetime.md) | The crate's one type-level declaration |
| [`type/002`](../type/002_a_blanket_impl_over_every_type_there_is.md) | The impl that makes the payload side need no declaration either |
| [`algorithm/001`](../algorithm/001_two_executable_statements_and_one_branch.md) | Two statements, matching two declared types of zero |

### Sources

| Fact | Where |
|------|-------|
| Zero types declared under `src/` | Census above |
| Four of 33 ring crates declaring none | Census above |
| The other three naming concrete family types; this one naming none | Census above, `ring_index/src/lib.rs:49`, `ring_seqno/src/lib.rs:50`, `ring_wait/src/lib.rs:112` |
| Zero `repr`, `size_of`, `align_of`, `transmute`, `as_ptr` | Census above |
| "no match, no downcast and no enum of shapes" | `ring_event/src/lib.rs:41-43` |
| The two shapes' fields, and the derive each carries | `ring_slot/src/lib.rs:83-84`, `:236-241` |
| `BytesSlot`'s hand-written `Debug`, `PartialEq` and `Eq`, which replaced three of its derives | `ring_slot/src/lib.rs:248-267` |
| `read()` returning the written bytes and not the tail | `ring_slot/src/lib.rs:358` |
| "Neither function can tell the two slot shapes apart" | `ring_event/src/lib.rs:21-22` |

### Tests

| Test | Covers |
|------|--------|
| `both_shapes_land_in_storage_through_the_same_two_calls` | One generic body over both declared-elsewhere shapes |
| `a_byte_payload_of_every_length_up_to_slot_size_survives_storage` | `read()` masking the tail at every length |
| `peek_is_implemented_on_the_slot_so_the_reader_needs_no_payload_type` | The read half reaching representation it never names |
