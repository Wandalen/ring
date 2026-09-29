# Algorithm: Rounding a Payload Up to Whole Lines

### Scope

- **Purpose**: State how a payload of arbitrary size becomes a value occupying a whole number of cache lines, and why the guarantee that matters is about *pairs* rather than about any single wrapped value.
- **Responsibility**: Give the rule the compiler applies, the two cases it produces, and the reason this crate performs no arithmetic of its own.
- **In Scope**: The size and alignment consequences of `#[ repr( align( 64 ) ) ]`; the sub-line and super-line cases.
- **Out of Scope**: Observing the result on real addresses, which is [`algorithm/001`](001_deciding_line_membership_by_division.md); whether 64 is the right number, which is [`pitfall/001`](../pitfall/001_a_constant_too_small_buys_nothing.md).

### Inputs

| Input | Type | Source |
|-------|------|--------|
| `T` | any sized type | The caller's payload |
| The alignment attribute | `#[ repr( align( 64 ) ) ]` | This crate's declaration on [`CacheAligned`](../type/002_cache_aligned.md) |

**This crate runs no code to produce the layout.** The algorithm is the
compiler's, invoked by an attribute; the crate's contribution is choosing to
invoke it and choosing the number. That is worth stating plainly because it is
the reason the whole crate needs no `unsafe`
(→ [`non_functional_requirement/002`](../non_functional_requirement/002_the_crate_costs_nothing_at_runtime.md))
— `repr( align )` is a safe attribute, and the alternative implementations of
this idea (manual padding arrays sized by hand, `MaybeUninit` byte blocks) are
the ones that need it.

### Steps

The rule Rust applies to a type carrying an alignment attribute:

1. Set `align_of::< CacheAligned< T > >()` to `max( align_of::< T >(), 64 )` — for
   every payload smaller than a line, that is 64.
2. Set `size_of::< CacheAligned< T > >()` to `size_of::< T >()` rounded **up** to
   a multiple of the alignment.

Step 2 is the one that does the work, and it is why the guarantee holds for
payloads on both sides of a line boundary.

| Payload | `size_of::<T>()` | `size_of::<CacheAligned<T>>()` | Lines occupied |
|---------|-----------------:|-------------------------------:|---------------:|
| `u8` | 1 | 64 | 1 |
| `u64` | 8 | 64 | 1 |
| `[ u8; 63 ]` | 63 | 64 | 1 |
| `[ u8; 65 ]` | 65 | **128** | 2 |

**The `[ u8; 65 ]` row is the one that has to be asserted rather than assumed.**
A reader who has internalised "the wrapper makes things 64 bytes" will expect
it to be wrong, and a wrapper that *truncated* to one line would be a soundness
bug rather than a layout inefficiency. The test states it as a general property
before it states the specific number:

```rust
assert_eq!( size % CACHE_LINE, 0, "size {size} is not a whole number of lines" );
assert_eq!( size, 128 );
```

### What the Guarantee Actually Is

**Not** "a wrapped value occupies one cache line" — that is false for the last
row. The guarantee is:

> `size_of::< CacheAligned< T > >()` is a whole multiple of `CACHE_LINE`, and
> the type's alignment is `CACHE_LINE`.

And the consequence the family needs is a statement about **two** of them:

> Two `CacheAligned` fields in one struct never share a cache line.

The second follows from the first by a step worth making explicit. Field `a`
starts at a multiple of 64 (alignment) and occupies a whole number of lines
(size), so field `b` — placed after it, at the next multiple of 64 — starts at
a line boundary `a` does not touch. **Whole-line sizing is what makes the
alignment compose**: alignment alone would put both fields on line boundaries,
but a 64-byte-aligned type of size 8 would let the compiler pack a second value
into the remaining 56 bytes of the same line, and the separation would evaporate
for reasons no assertion about a single type would catch.

That is why [`invariant/001`](../invariant/001_two_wrapped_fields_never_share_a_line.md)
is stated over a pair rather than over the wrapper, and why the test suite
asserts it on real addresses with a negative control rather than on `size_of`
alone (→ [`pitfall/002`](../pitfall/002_size_of_proves_nothing_about_addresses.md)).

### The Cost, Stated

Padding a `u64` to 64 bytes is an 8× memory expansion of that field.
This crate's own design accepts that cost in one ruling — "it costs memory
that is irrelevant at these counts" — and the count is
what makes that true: a ring has two cursors, not two million. **The rule does
not survive a change of scale**, and nothing in this crate enforces the scale.
A caller wrapping every element of a large array would pay 64 bytes per element
and this crate would neither notice nor complain
(→ [`pitfall/001`](../pitfall/001_a_constant_too_small_buys_nothing.md) for the
opposite failure).

### Algorithms

| File | Relationship |
|------|--------------|
| [001_deciding_line_membership_by_division.md](001_deciding_line_membership_by_division.md) | Observes the separation this rounding produces |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_cache_aligned_wrapper.md](../data_structure/001_the_cache_aligned_wrapper.md) | The declaration this rule is applied to |
| [../data_structure/002_a_padded_pair_in_one_struct.md](../data_structure/002_a_padded_pair_in_one_struct.md) | The two-field arrangement the guarantee is actually about |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_two_wrapped_fields_never_share_a_line.md](../invariant/001_two_wrapped_fields_never_share_a_line.md) | The pair-level statement this rule establishes |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_the_crate_costs_nothing_at_runtime.md](../non_functional_requirement/002_the_crate_costs_nothing_at_runtime.md) | Why an attribute rather than hand-written padding, and what that buys under `unsafe-code = "deny"` |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_size_of_proves_nothing_about_addresses.md](../pitfall/002_size_of_proves_nothing_about_addresses.md) | Why the test does not stop at the table above |

### Sources

| File | Relationship |
|------|--------------|
| [`../decisions/001_the_constant_is_not_conditional.md`](../decisions/001_the_constant_is_not_conditional.md) | "The padding is the whole point… it costs memory that is irrelevant at these counts" — the cost ruling this instance quotes |

### Tests

| File | Relationship |
|------|--------------|
| `tests/align_test.rs` | `a_wrapped_value_occupies_exactly_one_line` covers the first three rows; `an_oversized_payload_rounds_up_to_whole_lines` covers the fourth, asserting the general multiple-of-a-line property before the specific 128 |
