# Workaround: A Unit Error Variant Cannot Carry the Refused Item

### Scope

- **Purpose**: Record the constraint that makes `push` destroy a refused item, and the cost of absorbing it rather than working around it.
- **Responsibility**: The constraint's origin in `ring_types`, the workaround this crate did not take, and the deletion condition.
- **In Scope**: `RingError::Full` as a unit variant, and `push`'s `Result< (), RingError >`.
- **Out of Scope**: Whether to change it, which is an open ruling (→ [`../decisions/002`](../decisions/002_a_refused_push_destroys_the_item_its_doc_promises_to_return.md)).

### The Constraint

`RingError` is `ring_types`' shared error enum for the whole family. `Full` is a
unit variant, because for a ring the caller's item is already in a slot or was
never moved — nothing needs handing back. A staging buffer that takes `T` by
value is the one shape in the family where that is false.

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'RingError variants that carry data: '
grep -cE '^  [A-Z][A-Za-z]*(\(|\s*\{)' ring_types/src/error.rs
printf 'and Full is not one of them:\n'
grep '^  Full,' ring_types/src/error.rs
```

Live output:

```
RingError variants that carry data: 1
and Full is not one of them:
  Full,
```

Exactly one variant carries data, so the constraint is not "this error type
cannot hold anything" — it is that `Full` specifically does not, and `Full` is
the variant a bounded buffer returns.

### The Workaround Not Taken

`ring_registry` faced the same shape and put the payload in the `Err` tuple
rather than in the variant: `Result< (), ( RegistryError, Split< T > ) >`. That
requires no change to the shared enum and is available to this crate unchanged.

### Cost

Every refused `push` drops the item. For the three current consumers, all
staging `Copy` types, the cost is zero. For any `T` owning a resource it is
silent data loss on the path the API expects callers to hit — `is_full` exists
precisely because refusal is routine.

The documentation states the opposite, which converts a bounded cost into a
trap (→ [`../decisions/002`](../decisions/002_a_refused_push_destroys_the_item_its_doc_promises_to_return.md)).

### Deletion Condition

`push` returns `Result< (), ( RingError, T ) >`. Breaking for all three
consumers, none of which currently inspects the error's payload, so the
migration is a `.map_err( |( e, _ )| e )` at three call sites.

### Sources

| File | Relationship |
|------|-----------------|
| `../../../ring_types/src/error.rs` | Declares `Full` as a unit variant |
| `../../../ring_registry/src/lib.rs` | The workaround available and not taken |
| `src/lib.rs` | The signature that absorbs the constraint |

### TL52 — Two of Nine `RingError` Variants Carry Data and Neither Is `Full`

```sh
cd "$(git rev-parse --show-toplevel)"
# A variant carries data when `(` follows its name on the same line, or when a
# `{` opens on the next one. `BatchTooLarge` wraps, so a single-line pattern
# reports one data-carrying variant where the enum has two.
awk '
  /^pub enum RingError$/ { inside = 1; next }
  inside && /^\}$/       { exit }
  inside && /^  [A-Z][A-Za-z]*\(/ { data += 1; next }
  inside && /^  \{$/     { data += 1; unit -= 1; next }
  inside && /^  [A-Z][A-Za-z]*,?$/ { unit += 1 }
  END { printf "unit variants: %d\nvariants carrying data: %d\n", unit, data }
' ring_types/src/error.rs
printf 'and Full is declared among the unit ones:\n'
grep '^  Full,$' ring_types/src/error.rs
```

Live output:

```
unit variants: 7
variants carrying data: 2
and Full is declared among the unit ones:
  Full,
```

`Full` is a unit variant because for a ring the caller's item is already in a
slot or was never moved. A staging buffer taking `T` by value is the one shape
in the family where that reasoning does not hold.

The two variants that do carry data — `CapacityNotPowerOfTwo( usize )` and
`BatchTooLarge { requested, capacity }` — both carry a *number the caller
passed in*, never a value the caller surrendered ownership of. So the enum
already has the shape for returning a rejected item and has never once been
used that way; the constraint is a habit of the enum, not a limit of it.
