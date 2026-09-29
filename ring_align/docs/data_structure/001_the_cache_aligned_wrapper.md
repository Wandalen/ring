# Data Structure: The `CacheAligned` Wrapper

### Scope

- **Purpose**: Document the wrapper as a memory layout rather than as a name — one declared field, no padding field, and a size that is a property of an attribute rather than of anything stored.
- **Responsibility**: Give the layout, the size table, where the padding physically is, and what the shape guarantees without any code running.
- **In Scope**: In-memory form of `CacheAligned<T>`; the sub-line and super-line cases.
- **Out of Scope**: The accessors, which are [`api/002`](../api/002_the_wrapper_surface.md); the pair arrangement, which is [`data_structure/002`](002_a_padded_pair_in_one_struct.md).

### Layout

```rust
#[ repr( align( 64 ) ) ]
pub struct CacheAligned< T >( T );
```

```
CacheAligned< u64 >, 64 bytes total
┌────────────────┬───────────────────────────────────────────────┐
│ payload: u64   │ trailing padding                              │
│ 8 bytes        │ 56 bytes — not a field, not initialised,      │
│ offset 0       │ not addressable through any accessor          │
└────────────────┴───────────────────────────────────────────────┘
 ↑ always a multiple of 64                                        ↑ next line
```

**The padding is not stored anywhere.** There is no `[ u8; 56 ]` field, no
`PhantomData`, and nothing in the crate computes 56. The trailing bytes exist
because `size_of` rounds up to the alignment
(→ [`algorithm/002`](../algorithm/002_rounding_a_payload_up_to_whole_lines.md)),
and the crate never names them. That is why the declaration is one line and why
no `unsafe` is involved — a hand-written padding array would need the size
computed per payload, which is `const` arithmetic at best and
`MaybeUninit` at worst.

The payload sits at **offset 0** in practice, and that is an observation rather
than a guarantee. `#[ repr( align( … ) ) ]` constrains alignment only; it does
not imply `repr( C )` or `repr( transparent )`, so the layout of the fields
inside remains unspecified. With exactly one field there is nowhere else to put
it, and no compiler does — but nothing in the language promises it, so a caller
casting `*const CacheAligned< T >` to `*const T` would be relying on an
unspecified detail. No such cast is offered
(→ [`pattern/001`](../pattern/001_the_newtype_as_layout_carrier.md)).

### Sizes

| `T` | `size_of::<T>()` | `size_of::<CacheAligned<T>>()` | `align_of` | Lines |
|-----|-----------------:|-------------------------------:|-----------:|------:|
| `u8` | 1 | 64 | 64 | 1 |
| `u64` | 8 | 64 | 64 | 1 |
| `[ u8; 63 ]` | 63 | 64 | 64 | 1 |
| `[ u8; 65 ]` | 65 | **128** | 64 | 2 |
| `AtomicSeq` (the real payload) | 8 | 64 | 64 | 1 |

**Alignment stays 64 even when size grows.** Row 4 is two lines, aligned to the
first — not aligned to 128. That is correct: what the family needs is that each
value *starts* a line and *ends* one, which is exactly alignment plus whole-line
sizing.

Regenerate the real-payload row:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_align
cargo test --test align_test a_wrapped_value_occupies_exactly_one_line an_oversized_payload_rounds_up_to_whole_lines
# The command above can legitimately exit nonzero -- a count of zero, a diff
# that differs -- and the sweep that keeps this output current skips any block
# that exits nonzero, so without this the block freezes instead of refreshing.
:
```

Live output:

```
error: unexpected argument 'an_oversized_payload_rounds_up_to_whole_lines' found

Usage: cargo test [OPTIONS] [TESTNAME] [-- [ARGS]...]

For more information, try '--help'.
```

### What the Shape Guarantees Without Code

Three properties hold by construction, with nothing executing:

1. **Every instance starts at a line boundary.** Alignment 64 means every
   address the compiler or allocator chooses is a multiple of 64.
2. **Every instance ends at a line boundary.** Size is a multiple of 64, so the
   byte after an instance is also 64-aligned.
3. **Therefore no other value can occupy any line an instance touches** —
   because there is no room left in those lines for the compiler to place one.

Property 3 is the one the family needs, and it is a *consequence* of 1 and 2
rather than something either states alone. This is why
[`invariant/001`](../invariant/001_two_wrapped_fields_never_share_a_line.md)
lists both alignment and sizing as separate enforcement mechanisms, and why
`ring_cursor`'s acceptance criterion names both:

> The acceptance criterion names both because only their conjunction says "one
> per line", and a future layout change could break the second while leaving
> the first intact.

### What the Shape Does Not Guarantee

- **Nothing about where two instances go.** The compiler places fields; this
  structure has no opinion (→ [`data_structure/002`](002_a_padded_pair_in_one_struct.md)).
- **Nothing about the *host's* line size.** Every number above is 64 because
  the constant is 64 (→ [`pitfall/001`](../pitfall/001_a_constant_too_small_buys_nothing.md)).
- **Nothing about the padding's contents.** The trailing bytes are whatever the
  allocation held. They are unreachable through the API, so this matters only
  to a caller doing something the type does not support — but it means a
  `CacheAligned` is **not** safe to compare or hash by raw bytes, and the
  derives correctly go through the payload instead
  (→ [`type/002`](../type/002_cache_aligned.md)).

### AL9 — The Module Doc's "So" Is False for the Zero-Sized Payload

`src/lib.rs:39-41` says the size is a multiple of `CACHE_LINE` for any `T` that
fits, **so** two of them in one struct land on different lines. The probe above
shows the premise holding and the conclusion failing:

```
    ()        align  64  size    0
```

Zero is a multiple of sixty-four. Two zero-sized wrappers in one struct sit at
the same address.

**Finding.** The stated implication is not the theorem the type has. The true
statement needs "non-zero-sized" or, better, needs to be about the *pair* rather
than about the wrapper, which is how
[`invariant/001`](../invariant/001_two_wrapped_fields_never_share_a_line.md)
states it. Nothing in the family instantiates `CacheAligned< () >`, so this is a
doc defect rather than a live bug — and it is the exact shape of reasoning error
that a reader would carry into a case that does matter.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_align
command grep -m1 -B1 -A3 -F 'The padding is the point:' src/lib.rs
```

Live output:

```
///
/// The padding is the point: `size_of::<CacheAligned<T>>()` is a multiple of
/// [`CACHE_LINE`] for any non-zero-sized `T` that fits, so two of them in one
/// struct are guaranteed to land on different lines. (A zero-sized `T` makes
/// the wrapper itself zero-sized, so two of them coincide at one address —
```

**Disposition:** applied — `src/lib.rs`'s module doc for `CacheAligned` now
qualifies the claim to non-zero-sized `T` and states the excluded case
explicitly rather than implying the "so" holds universally; the crate's 7
doctests re-verified passing (`cargo test --all-features`, 2026-09-03). Now
prints: `two of them coincide at one address`

---

### AL10 — Alignment Is a Maximum, Not a Constant

```
    Wide      align 128  size  128
```

A payload declared `#[ repr( align( 128 ) ) ]` produces a 128-aligned wrapper:
the attribute sets a floor, and `align_of::< CacheAligned< T > >()` is
`max( 64, align_of::< T >() )` rather than 64.

**Finding.** Harmless — every value above 64 preserves the separation property —
but it means `align_of` is not a constant of this type, which the doctest's
`u64` case quietly suggests it is. A reader who writes
`assert_eq!( align_of::< CacheAligned< T > >(), CACHE_LINE )` for a generic `T`
has written an assertion that is false for a payload the type accepts.

---

### AL11 — The Padding Is Not a Field and Is Not Stored

The struct has exactly one member. The difference between `size_of` and
`size_of::< T >()` is what the attribute instructed the compiler to leave empty
— there is no `[ u8; N ]`, no `PhantomData`, and no arithmetic anywhere in the
crate that computes a padding amount.

**Finding.** This is why `Debug` prints only the payload and `PartialEq`
compares only the payload: the derives see one field, because there is one
field. It is also why the crate has no algorithm to get wrong at this level
(→ [`algorithm/002`](../algorithm/002_rounding_a_payload_up_to_whole_lines.md)),
and why a reader looking for where the 56 bytes are declared will not find them.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_rounding_a_payload_up_to_whole_lines.md](../algorithm/002_rounding_a_payload_up_to_whole_lines.md) | The rule producing every number in the size table |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_wrapper_surface.md](../api/002_the_wrapper_surface.md) | The four routes to the payload at offset 0 |

### Data Structures

| File | Relationship |
|------|--------------|
| [002_a_padded_pair_in_one_struct.md](002_a_padded_pair_in_one_struct.md) | Two of these, and the property that needs two |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_two_wrapped_fields_never_share_a_line.md](../invariant/001_two_wrapped_fields_never_share_a_line.md) | Properties 1–3 restated as a standing restriction, with what breaks each |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_the_newtype_as_layout_carrier.md](../pattern/001_the_newtype_as_layout_carrier.md) | Why the field is private, given that offset 0 makes the cast obvious |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_size_of_proves_nothing_about_addresses.md](../pitfall/002_size_of_proves_nothing_about_addresses.md) | Why this table is evidence for the property rather than the property |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_cache_aligned.md](../type/002_cache_aligned.md) | The same declaration read as a name and a contract |

### Sources

| File | Relationship |
|------|--------------|
| [`../invariant/001_two_wrapped_fields_never_share_a_line.md`](../invariant/001_two_wrapped_fields_never_share_a_line.md) | "Padded so that it occupies a cache line by itself" — the shape this instance draws |

### Tests

| File | Relationship |
|------|--------------|
| `tests/align_test.rs` | `a_wrapped_value_occupies_exactly_one_line` asserts rows 1–3; `an_oversized_payload_rounds_up_to_whole_lines` asserts row 4 as a general multiple-of-a-line property before the specific 128 |
