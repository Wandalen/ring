# impl Capacity

## Representation

The inherent implementation block on [`Capacity`](../struct/001_capacity.md),
holding all three of its associated functions and no associated constants.
Together they are the type's entire behaviour: one constructor that can fail
([`new`](../associated_function/001_capacity_new.md)) and two total accessors
([`get`](../associated_function/002_capacity_get.md),
[`mask`](../associated_function/003_capacity_mask.md)).

**The block's shape is the pattern's shape.** With a private field and no
`Default`, this `impl` is the only way to obtain a `Capacity`, which is what makes
`mask()` total and what removes the power-of-two check from fifteen consumer
crates (→ [`../../pattern/002`](../../pattern/002_a_newtype_that_makes_a_check_unnecessary.md)).
Adding a public constructor here — or a `From< usize >` — would undo it silently.

All three functions are `const fn` and all three take `self` by value.

## Kind

Implementation (§ Item Kind Taxonomy : Stable Item Kinds #12)

## Definition

`ring_types/src/capacity.rs:25`

```rust
impl Capacity
{
  pub const fn new( slots : usize ) -> Result< Self, RingError >   // :40
  pub const fn get( self ) -> usize                                // :60
  pub const fn mask( self ) -> usize                               // :75
}
```

Inherent, not a trait impl. The crate declares no trait
(→ [`../readme.md`](../readme.md) § What the crate does not declare), so every
inherent block here is the type's own surface rather than a contract with
anything.

## File Usage

| File | Line(s) | Context |
|------|---------|---------|
| `ring_types/src/capacity.rs` | 25, 40, 60, 75, 79 | **Block header (25)**; the three associated functions it contains (40, 60, 75); closing brace (79) |

The block spans lines 25-79 — 55 of the file's 79 lines, most of which are doc
comments and their examples.

Test-only references: `ring_types` — `tests/types_test.rs` exercises all three
functions across three tests, but an `impl` block is not a nameable item, so no
test references it directly.

## Crate Usage

| Crate | Via File | Purpose |
|-------|----------|---------|
| `ring_types` | `src/capacity.rs` | Declares the block; its three functions are the type's only surface |
| *(fifteen consumers)* | `src/lib.rs` each | Call `new`, `get` or `mask` through it — an `impl` block is reached by method resolution, never by name |

**An `impl` block's Crate Usage is its members' usage, unioned.** Fifteen crates
call at least one of the three; only two — `ring_index` and `ring_mpsc` — call
`mask`, which is the narrowest of the three and the one the whole design exists
for (→ [`../associated_function/003_capacity_mask.md`](../associated_function/003_capacity_mask.md)).
