# type

Two concrete types, and every interesting thing about them is something they
refuse. Neither carries `Copy` while thirty-seven sibling types across the 33
crates do. `TypedSlot< T >` places no bound on `T` anywhere — not on the struct,
not on the inherent impl, not on `Default` — so any Rust type at all can be a
payload. `BytesSlot< N >` places no bound on `N` either, and has no `where`
clause in 288 lines, so both degenerate ends of the range are legal types.

Each refusal is the right call and none of them is written down. The absent
`Copy` is what forces the family's borrow-only slot access, and a third
implementor of `Slot` that *was* `Copy` would compile and quietly undo it. The
absent bound on `N` makes a slot's width part of its type rather than part of its
value — the opposite of how the family handles a ring's slot *count*, one tier
away, under the same word.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Two Shapes, One Trait, No `Copy`](001_two_shapes_one_trait_no_copy.md) | SL45, SL46 — a derive omitted for two different reasons, and two payload channels different in kind |
| 002 | [The Const Parameter as Capacity](002_the_const_parameter_as_capacity.md) | SL47, SL48 — width fixed at compile time, and an unbounded `N` whose far end is named but untested |

### The Two Declarations

```rust
#[ derive( Debug, Clone, PartialEq, Eq ) ]
pub struct TypedSlot< T >( Option< T > );

#[ derive( Clone ) ]
pub struct BytesSlot< const N : usize >
{
  bytes : [ u8; N ],
  len : usize,
}
```

One parameterised by a type, the other by a value. Both have private fields, so
the only way in or out is the inherent surface. `BytesSlot` derives only
`Clone` — `Debug`, `PartialEq`, and `Eq` are hand-written over `read()` instead,
so none of the three sees a byte past `len`
([`non_functional_requirement/002`](../non_functional_requirement/002_four_traits_and_what_they_compare.md)).

### The `Copy` Two of Them Do Not Have

| | Could be `Copy`? | Why not |
|---|---|---|
| `TypedSlot< T >` | No | `Copy` forbids a destructor, and the type exists to hold payloads that have one |
| `BytesSlot< N >` | **Yes** — a mirror with the derive compiles | Every move would become an implicit `N`-byte memcpy, unspelled at the call site |

Thirty-seven derived `Copy` across the family and zero here. The omission is
right and unrecorded, and the surrounding evidence points the other way — every
sibling small value type has it, `BytesSlot`'s fields all permit it, and no
comment says no. A maintainer adding it for symmetry would find every test
passing and every bench slower, with the regression spread across every binding
rather than concentrated where a profile would name it.

The consequence reaches `ring_store`: `get`, `get_mut`, `at`, and `at_mut` all
return references, no by-value form. That is what forces the drain idiom to reach
*through* a borrow and move the payload rather than the slot. A third `Slot`
implementor that was `Copy` would satisfy every bound in the family and silently
reintroduce by-value access — nothing states that not being `Copy` is a
requirement rather than an accident of the two types that exist.

### One Word, Two Mechanisms

| | Slot width | Ring slot count |
|---|---|---|
| Named | `BytesSlot< N >` | `Capacity( usize )` |
| Fixed at | Compile time, in the type | Run time, in a constructor argument |
| Held as | Nothing — `N` has no storage | A field, `capacity : Capacity` |
| Where | `ring_slot/src/lib.rs:237` | `ring_types/src/capacity.rs:23`, `ring_store/src/lib.rs:62` |

Both are right — a slot count comes from configuration, a byte width must be
static or the array cannot be inline — and the split is stated nowhere. A reader
who met `Capacity` first and reaches for a runtime slot width gets a type error
with no pointer to the reason.

The sharper consequence: `BytesSlot< 8 >` and `BytesSlot< 16 >` are unrelated
types sharing no supertype but `Slot`, so no array, `Vec`, or ring can hold both.
A process handling two wire-frame widths needs two rings. That is an
architectural constraint imposed by a type parameter, and the module comment —
which does explain why a slot cannot grow — does not mention it.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# the two declarations
awk '/^\/\/\/ assert_eq!\( slot\.take\(\), Some\( 42 \) \);$/{ n1 = NR } n1 && NR >= n1 + 3 && NR <= n1 + 4 { print } /^\/\/\/ assert!\( !slot\.is_empty\(\) \);$/{ n2 = NR } n2 && NR >= n2 + 2 && NR <= n2 + 7 { print }' ring_slot/src/lib.rs

# 37 derived Copy in the family, 0 here
printf '  derive( .. Copy .. ) across the 33 crates : %d\n' \
  "$( grep -rhE '^#\[ derive\(.*Copy' ring_*/src/*.rs | wc -l )"
printf '  derive( .. Copy .. ) on a ring_slot type  : %d\n' \
  "$( grep -cE '^#\[ derive\(.*Copy' ring_slot/src/lib.rs )"

# the four unbounded N sites, and no where clause anywhere
grep -n 'const N : usize' ring_slot/src/lib.rs
printf '  where clauses in the crate : %d\n' \
  "$( grep -cE '^[[:space:]]*where' ring_slot/src/lib.rs )"

# four accessors, four references, no by-value form
grep -nE '^[[:space:]]*pub fn (get|get_mut|at|at_mut)' ring_store/src/lib.rs

# the same word, one tier away, as a runtime value
grep -n 'pub struct Capacity\|pub const fn get' ring_types/src/capacity.rs
grep -n 'capacity : Capacity' ring_store/src/lib.rs
```

Layout figures, the compiling `Copy` mirror, and the unbounded-`T` end-to-end run
come from a release probe; they are quoted in the instances.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SL45 | `ring_slot` | n/a — doc gap | Thirty-seven types in the family derive `Copy` and neither slot does; `BytesSlot` could, and the reason it must not — an implicit `N`-byte memcpy per move — is recorded nowhere |
| SL46 | `ring_slot` | n/a — doc gap | The absent `Copy` is what makes `ring_store`'s four reference-only accessors enforceable; nothing states that a `Copy` implementor of `Slot` would undo it |
| SL47 | `ring_slot` | n/a — doc gap | Width is part of the type and slot count is part of the value, one tier apart under the same word, with no note on either — and no container can hold two widths |
| SL48 | `ring_slot` | n/a — doc gap | `N` carries no bound and no `where` clause; the far end is instantiated once to assert `capacity()` and never exercised, and the guidance on where the design stops working lives in four other instances rather than at `N` |
