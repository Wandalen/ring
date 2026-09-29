# Pattern: The Newtype as Layout Carrier

### Scope

- **Purpose**: Name the shape `CacheAligned` is an instance of — a newtype that adds no behaviour and no value invariant, only a property of where the value sits — and separate it from the three other newtype uses in the same family.
- **Responsibility**: State the shape, contrast it against the family's other newtypes, and say what the private field is actually buying.
- **In Scope**: The pattern and its family instances.
- **Out of Scope**: The layout the pattern produces, which is [`data_structure/001`](../data_structure/001_the_cache_aligned_wrapper.md); the type's contract, which is [`type/002`](../type/002_cache_aligned.md).

### The Shape

```rust
#[ derive( … ) ]
#[ repr( align( 64 ) ) ]
pub struct CacheAligned< T >( T );
```

A newtype whose entire content is an **attribute**. It has no field of its own,
no method that does anything to the payload, no invariant on the payload's
value, and a constructor that cannot fail. Strip the attribute and the type is
a no-op wrapper. The attribute is the product.

`ring_cursor`'s module doc states the consequence for its own type
(`ring_cursor/src/lib.rs:20-21`):

> [`PaddedCursor`] is [`ring_atomic::AtomicSeq`] inside
> [`ring_align::CacheAligned`] and nothing else — no field of its own, no logic
> of its own.

**The pattern composes:** a layout carrier wrapped around a payload yields
another layout carrier, and `PaddedCursor` is that.

### Against the Family's Other Newtypes

Four newtypes in the write-path family, and they differ in exactly the two axes
that matter — field visibility and what the wrapper is *for*:

| Newtype | Declared | Field | Carries | Construction |
|---------|----------|:-----:|---------|--------------|
| `Seq( pub u64 )` | `ring_types/src/id.rs:25` | public | **a name.** Distinguishes a sequence number from any other `u64` at a call site | any `u64`, directly |
| `SlotIndex( pub usize )` | `ring_types/src/id.rs:99` | public | **a name**, same as `Seq` | any `usize`, directly |
| `Capacity( usize )` | `ring_types/src/capacity.rs:23` | private | **a value invariant** — non-zero *and* a power of two, checked once at the boundary | `const fn new( usize ) -> Result< Self, RingError >` |
| `CacheAligned< T >( T )` | `ring_align/src/lib.rs:69` | private | **a layout property** — where the value starts and how much room it occupies | `const fn new( T ) -> Self`, infallible |

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r 'pub struct' --include=*.rs ring_types/src/ ring_align/src/
```

Live output:

```
ring_types/src/capacity.rs:pub struct Capacity( usize );
ring_types/src/id.rs:pub struct Seq( pub u64 );
ring_types/src/id.rs:pub struct SlotIndex( pub usize );
ring_align/src/lib.rs:pub struct CacheAligned< T >( T );
```

**The third row is the one usually meant by "newtype pattern"** — privacy plus
a fallible constructor equals a value that cannot be wrong. The fourth row
borrows the syntax and none of the mechanism: `new` accepts every `T` and
returns `Self`, so nothing is being validated. What is being *made true* is not
about the value at all.

**Rows 1–2 show the axis is not privacy.** `Seq` and `SlotIndex` expose their
fields and are still doing real work — they make a wrong argument at a call
site a type error. That is a fourth thing a newtype can carry, and it happens
without any encapsulation at all.

### What the Private Field Actually Buys

Not restriction. `get`, `get_mut`, and `into_inner` between them reach the
payload by shared reference, by exclusive reference, and by value — every route
a public field would have offered
(→ [`api/002`](../api/002_the_wrapper_surface.md)). A caller is not held back
by the privacy in any way it would notice.

What it buys is **representational freedom for this crate**: the declaration
can gain a field, change to an array, or acquire a `PhantomData` without
breaking a caller, because no caller names `.0`. Given that the whole crate is
one attribute and the attribute is the thing most likely to change on a
platform port (→ [`lifecycle/002`](../lifecycle/002_the_constant_across_a_platform_port.md)),
that freedom is worth the four accessor definitions it costs.

Stating it plainly matters because the reflex reading of a private field —
"this is protecting an invariant" — is wrong here, and the reflex is what
produced the overclaim this instance corrects in
[`invariant/001`](../invariant/001_two_wrapped_fields_never_share_a_line.md)'s
P3.

### Where the Pattern Applies, and Where It Stops

**Applies** when a property of a value's *placement* — alignment, size,
section, contiguity — must travel with the value through code that neither
knows nor cares about it. The wrapper carries the property into every struct,
array, and allocation the payload reaches, with no cooperation from the code in
between.

**Stops** at three points, all of them live in this crate:

| # | Limit | Evidence |
|---|-------|----------|
| K1 | The property is lost the moment the payload is unwrapped | `into_inner` discards the padding with the wrapper (→ [`lifecycle/001`](../lifecycle/001_the_wrapped_values_arc.md)) |
| K2 | The wrapper cannot say anything about a *second* value | One padded field beside one unpadded field still contends; the pattern needs a type owning both (→ [`data_structure/002`](../data_structure/002_a_padded_pair_in_one_struct.md)) |
| K3 | The attribute takes a literal, so the carried property cannot be parameterised | `#[ repr( align( CACHE_LINE ) ) ]` is `E0693` (→ [`type/001`](../type/001_cache_line.md)). A generic `Aligned< T, const N : usize >` is not expressible |

**K3 is the pattern's hard ceiling** and it is a language limit rather than a
design choice. It is why this crate has one wrapper at one alignment instead of
a family of them, and why a second alignment anywhere in the workspace would
mean a second literal rather than a second instantiation.

That the attribute in question is the **only `repr` in the entire family** is
worth recording:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r 'repr(' --include=*.rs ring_*/src/
```

Live output:

```
ring_align/src/lib.rs://! No `unsafe` is needed for any of it — `#[ repr( align( 64 ) ) ]` is a safe
ring_align/src/lib.rs:#[ repr( align( 64 ) ) ]
ring_cursor/src/lib.rs://! `#[ repr( align( 64 ) ) ]` happens to round the size up too, so both hold —
```

Three hits across 33 crates: the attribute at `ring_align/src/lib.rs:68`, and
two doc comments mentioning it. **The family's entire layout-control surface is
one line, in this crate**, which is the strongest available argument that the
pattern is being applied deliberately rather than habitually.

### AL37 — Two of Six Family Newtypes Expose the Field the Other Four Hide

```
pub struct AtomicSeq( AtomicU64 );
pub struct Budget( usize );
pub struct Capacity( usize );
pub struct PaddedCursor( CacheAligned< AtomicSeq > );
pub struct Seq( pub u64 );
pub struct SlotIndex( pub usize );
```

`Seq` and `SlotIndex` are `pub`; the rest are not.

**Finding.** Both exposed ones are plain integers over which no invariant is
claimed, so the split is defensible — and it is undeclared. Nothing in the
family says "a newtype exposes its field when it carries no invariant", so a
seventh newtype has six examples and no rule, and whichever way its author
guesses, half the existing corpus agrees with them.

---

### AL38 — The Pattern's Own Exemplar Is Invisible to a Census of Its Kind

The grep that finds all six newtypes above is written for `pub struct Name( … )`
and does not match `pub struct CacheAligned< T >( T );` — the only generic one,
and the type the pattern is named after. It has to be fetched by name:

```
51-#[ derive( Debug, Clone, Copy, Default, PartialEq, Eq ) ]
52-#[ repr( align( 64 ) ) ]
53:pub struct CacheAligned< T >( T );
```

**Finding.** Any future count of the family's newtypes written the obvious way
will report six and omit the one that motivated the pattern. That is worth
stating in the pattern document itself, because the omission is silent and the
number it produces looks complete.

---

### AL40 — The Only Newtype That Wraps Another Newtype Is Where the Pattern Composes

`PaddedCursor( CacheAligned< AtomicSeq > )` stacks three names to say "an atomic
sequence number on its own cache line". Every other family newtype wraps a
primitive.

**Finding.** The stacking works only because each layer is transparent: the
padding survives `AtomicSeq` being wrapped, and `AtomicSeq`'s atomicity survives
being padded, because neither layer adds a field or an invariant of its own.
Composition is therefore a property of this pattern rather than an accident, and
it is the one place in the family where that property is exercised.

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_wrapper_surface.md](../api/002_the_wrapper_surface.md) | The four accessors that make the privacy non-restrictive |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_cache_aligned_wrapper.md](../data_structure/001_the_cache_aligned_wrapper.md) | The layout the attribute produces |
| [../data_structure/002_a_padded_pair_in_one_struct.md](../data_structure/002_a_padded_pair_in_one_struct.md) | K2 — the arrangement the pattern cannot reach alone |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_two_wrapped_fields_never_share_a_line.md](../invariant/001_two_wrapped_fields_never_share_a_line.md) | P3, corrected here — privacy buys representational freedom, not enforcement |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_the_wrapped_values_arc.md](../lifecycle/001_the_wrapped_values_arc.md) | K1 — where the carried property is dropped |

### Patterns

| File | Relationship |
|------|--------------|
| [002_one_owner_for_a_magic_number.md](002_one_owner_for_a_magic_number.md) | The other pattern; K3 is why the two cannot be merged into a parameterised wrapper |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_cache_aligned.md](../type/002_cache_aligned.md) | The same declaration as a contract, including which derives go through the payload |

### Sources

| File | Relationship |
|------|--------------|
| `ring_cursor/src/lib.rs:20-21` | The pattern composing — a layout carrier around a payload is another layout carrier |
| `ring_types/src/id.rs`, `ring_types/src/capacity.rs` | The three contrasting newtypes measured above |

### Tests

| File | Relationship |
|------|--------------|
| `tests/align_test.rs` | `a_wrapped_value_occupies_exactly_one_line` tests the attribute's effect, which is the only thing the pattern contributes |
| `tests/manual/readme.md` | M2 covers the `unsafe`-free claim the pattern depends on — a hand-written padding field would not be |
