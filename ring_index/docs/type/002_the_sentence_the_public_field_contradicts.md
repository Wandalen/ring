# Type: The Sentence the Public Field Contradicts

### Scope

**Purpose:** Record that `SlotIndex`'s doc comment forbids a construction its own
type signature permits and its own doctest performs, and that the forbidden
construction is nevertheless absent from every line of non-test code in the
family except one — the body of this crate's `of` function.

**Responsibility:** The gap between what `SlotIndex`'s prose asks of callers and
what its declaration enforces, measured across all 33 crates.

**In Scope:** `ring_types/src/id.rs:88-99`; every `SlotIndex( .. )` in
`any crate root`.

**Out of Scope:** the opacity split across the three types is
[`type/001`](001_three_types_borrowed_none_owned.md). The one-owner rule for the
*fold* rather than the *type* is
[`pattern/001`](../pattern/001_one_owner_for_one_arithmetic_fact.md).

---

## The Sentence, and the Doctest Two Lines Below It

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A11 -F '/// A position within a ring'"'"'s storage, always in `0..capacity`.' ring_types/src/id.rs
```

Live output:

```
/// A position within a ring's storage, always in `0..capacity`.
///
/// Derived from a [`Seq`] by `ring_index`, never constructed by counting.
/// Two sequences a full lap apart produce the identical `SlotIndex`, which is
/// the whole reason [`Seq`] exists as a separate type.
///
/// ```
/// use ring_types::SlotIndex;
/// assert_eq!( SlotIndex( 3 ).get(), 3 );
/// ```
#[ derive( Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default ) ]
pub struct SlotIndex( pub usize );
```

Three lines apart: "never constructed by counting," then a doctest constructing
one by counting, then a `pub` field making it always possible.

---

### IX39 — The Prohibition Is Unenforced, Self-Contradicted, and Almost Universally Obeyed

**Finding.** "Derived from a [`Seq`] by `ring_index`, never constructed by
counting" is a rule with no mechanism. `pub struct SlotIndex( pub usize )` grants
every crate in the workspace the ability to write `SlotIndex( 4096 )` at a
capacity of 8, and the first two lines of the type's own doctest do essentially
that with a smaller number.

The census across all 33 crates, excluding `ring_types`' own module and excluding
doctests:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -r 'SlotIndex( ' --include=*.rs . | command grep -v 'ring_types/src' \
  | command grep -v ': *///' | cut -d: -f1 | sort | uniq -c | sort -rn
echo '  -- of which are src, not tests --'
command grep -r 'SlotIndex( ' --include=*.rs . | command grep -v 'ring_types/src' \
  | command grep -v ': *///' | command grep '/src/'
```

Live output:

```
     20 ring_store/tests/buffer_test.rs
      6 ring_index/tests/index_test.rs
      6 ring_batch/tests/batch_test.rs
      3 ring_types/tests/types_test.rs
      1 ring_index/src/lib.rs
  -- of which are src, not tests --
ring_index/src/lib.rs:  SlotIndex( ( seq.0 as usize ) & capacity.mask() )
```

Thirty-five constructions. Thirty-four of them are in test files, and the
thirty-fifth is `of`'s body — which is the only one the sentence sanctions, and
is precisely the one the sentence describes.

So the rule holds in the sense that matters and is enforced by nothing at all. No
line of running production code in any of the 33 crates counts a slot index into
existence; every one that exists at runtime came out of line 51. That outcome was
achieved by convention, by review, and by the fact that `Buffer::at` makes the
convenient path the correct one — never by the compiler.

The distinction worth recording is what a `pub` field costs here versus what it
costs on `Seq`. A wrong `Seq` is a wrong sequence number, which the gating
machinery is built to notice. A wrong `SlotIndex` is an index, and `Buffer`
hands it to a slice:

- `Seq( pub u64 )` — forging one produces a claim that the gate will reject or
  admit on its merits. The type is a label and the checking is elsewhere.
- `SlotIndex( pub usize )` — forging one produces a subscript. Nothing between
  the forge site and the storage re-derives it.

---

### IX40 — One Line Converts Between the Two Domains, and It Does So by Unwrapping

`of` is the only conversion from sequence space to slot space in the family. Its
body is the only place `ring_index` reaches through a public field:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep '\.0\b' ring_index/src/lib.rs
```

Live output:

```
  SlotIndex( ( seq.0 as usize ) & capacity.mask() )
/// In a debug build, if `start.0 + ( count - 1 ) as u64` overflows `u64` for
```

**Finding.** The expression unwraps one newtype through its public field, casts,
masks with an accessor call on the opaque one, and wraps the result in the other
newtype's public field. Both transparent types are used transparently, exactly
once, in the one function whose job is to cross between them.

That is the newtype pattern working as designed rather than a defect: a wrapper
that never gets unwrapped is a wrapper that cannot be converted, and a conversion
has to happen somewhere. What is worth recording is the consequence — the
protection `Seq` and `SlotIndex` provide is *not* provided by their fields, which
are open. It is provided by there being one function, and by that function
appearing in `use` statements the reviewer can count
([`api/001`](../api/001_three_functions_three_must_use_one_reached.md) IX6).

The contrast with `Capacity` is the whole design in one comparison. `Capacity`'s
invariant is enforced by privacy, so a violation is a compile error.
`SlotIndex`'s invariant — "always in `0..capacity`", the first line of its doc —
is enforced by a convention that a single line of code happens to be the only
place that could break, so a violation is a code review someone did not do.

Neither type documents which kind of enforcement it has. `SlotIndex`'s comment
states its range as a fact ("always in `0..capacity`") in the same register
`Capacity` could state its own, and only one of the two is true by construction.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`type/001`](001_three_types_borrowed_none_owned.md) | The opacity split, and what `Capacity`'s privacy buys |
| [`pattern/001`](../pattern/001_one_owner_for_one_arithmetic_fact.md) | One owner for the fold, which is the same convention seen from the other side |
| [`pitfall/002`](../pitfall/002_the_second_fold_nobody_noticed.md) | The place the convention did break, and what it produced |
| [`api/001`](../api/001_three_functions_three_must_use_one_reached.md) | The reach census the convention is auditable through |

### Sources

| Fact | Where |
|------|-------|
| The prohibition and the doctest that breaks it | `ring_types/src/id.rs:88-99` |
| Thirty-five constructions, one of them non-test | Census above |
| The single conversion site | `ring_index/src/lib.rs`'s `of` function |

### Tests

| Test | Covers |
|------|--------|
| `mask_equals_modulo_over_four_laps_of_every_capacity` | The one sanctioned construction, against a counted one |
| `derivation_is_a_mask` | The same, asserted structurally rather than numerically |
| *(to create)* | Nothing testable — the rule the sentence states is not expressible against a `pub` field |
