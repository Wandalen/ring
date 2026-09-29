# Struct Items

### Scope

- **Purpose**: Catalog the three structs `ring_types` defines, all of them single-field newtypes over a primitive.
- **Responsibility**: Give each one's definition, derives, field visibility, and measured usage.
- **In Scope**: `Capacity`, `Seq`, `SlotIndex`.
- **Out of Scope**: The rules `Capacity` enforces, which are [`../../type/001`](../../type/001_capacity.md)'s; the reason `Seq` and `SlotIndex` are two types, which is [`../../data_structure/001`](../../data_structure/001_two_position_types_and_the_fold_between_them.md)'s.

### Overview Table

| ID | Name | Kind | Defined In | Status |
|----|------|------|-----------|--------|
| 001 | [Capacity](001_capacity.md) | Struct (#6) | `src/capacity.rs:23` | 🔄 |
| 002 | [Seq](002_seq.md) | Struct (#6) | `src/id.rs:25` | 🔄 |
| 003 | [SlotIndex](003_slot_index.md) | Struct (#6) | `src/id.rs:99` | 🔄 |

### The three differ on exactly two axes

| | Field | `Default` | Derives |
|--|-------|-----------|---------|
| `Capacity` | **private** `usize` | **no** | `Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash` |
| `Seq` | `pub u64` | yes | the same eight plus `Default` |
| `SlotIndex` | `pub usize` | yes | the same eight plus `Default` |

**One is not like the other two, and it is the same one on both axes.**
`Capacity`'s field is private because a public one would let a caller construct
an invalid capacity and skip the constructor entirely; `Default` is absent for
the same reason, since `Capacity( 0 )` is precisely the value the type exists to
forbid. `Seq` and `SlotIndex` have no invariant to protect — every `u64` is a
valid position and every `usize` a valid offset — so the field is public and the
zero value is meaningful.

The consequence is that `Capacity` is the only one of the three whose
construction can fail, which is why it is also the only one that pulls
`RingError` into its module (→ [`../use_declaration/005_use_crate_ring_error.md`](../use_declaration/005_use_crate_ring_error.md))
and the only one with a `type/` instance.

### All three are `Copy`

Sixteen bytes at most, no heap, no drop glue. This is not incidental — it is the
measured half of [`../../non_functional_requirement/001`](../../non_functional_requirement/001_errors_and_positions_do_not_allocate.md),
and it is what lets every consumer take these by value rather than by reference,
which in turn is why not one of the thirteen associated functions in this crate
takes `&self`:

```sh
cd "$(git rev-parse --show-toplevel)"
for f in ring_types/src/capacity.rs ring_types/src/id.rs; do
  printf '%s:%s\n' "$f" "$( command grep -c '&mut self\|&self' "$f" )"
done
```

Live output:

```
ring_types/src/capacity.rs:0
ring_types/src/id.rs:0
```

Both files return `0`. The single `&self` in the crate is `Display::fmt`'s, which
the trait's signature imposes.
