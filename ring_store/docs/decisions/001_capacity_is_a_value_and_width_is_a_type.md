# Ring capacity is a runtime `Capacity` over one heap allocation, while slot width stays a type parameter

Status: Accepted

## Context

`ring_store::Buffer<S>` holds a ring's slots. `ring_spsc::Ring::new` and `ring_mpsc::Ring::new` build their storage
through `Buffer::new(capacity)`. Two sizes describe a ring's storage. One is how many slots it has. The other, for
`ring_slot::BytesSlot<N>`, is how wide each slot is.

A ring buffer is often chosen to avoid heap allocation, and Rust can express a fixed-size array whose length is a
const generic. Against that, `ring_config::RingConfig::new(slots: usize) -> Result<Self, RingError>` decides the slot
count at runtime and may reject it, through `ring_types::Capacity::new`.

## Decision

`Buffer<S>` stores a `Box<[S]>` and the `Capacity` it was built with. `Buffer::new(capacity: Capacity)` allocates
exactly that many `Default` slots on the heap, once. The capacity is not part of the type.

Slot width goes the other way. `BytesSlot<N>` keeps `N` as a const generic, so the width is part of the slot's type.

The rule the family follows is this. A size that changes a type's layout is a type parameter. A size that only
changes how many of a fixed-size thing exist is a value. Both appear in one line of `Buffer::new`'s doc example,
`Buffer<BytesSlot<8>>` built with `Capacity::new(16)`, and the `8` in angle brackets beside the `16` in parentheses is
this rule, not an inconsistency.

## Alternatives considered

- **A const-generic capacity, `Buffer<S, const CAP: usize>` over `[S; CAP]`.** It allocates nothing, lives wherever
  it is declared, and lets `Capacity::mask` fold to a compile-time constant. The price is that ring size becomes part
  of the type. Every function taking a `&Buffer` grows a const parameter, two rings of different sizes become
  different types, and a capacity read from a config file cannot be represented. `RingConfig::new` taking a
  `usize` and returning a `Result` could not be written.
- **A runtime width for `BytesSlot`.** Width decides the slot's layout, so the type cannot be sized without it. A
  runtime width would move the payload behind a heap pointer and give up the inline payload that is the shape's
  purpose.

## Consequences

- Each ring makes exactly one heap allocation, at construction. `Buffer::clear` empties slots in place and keeps it.
- The index fold in `ring_index` masks with a runtime `Capacity` value rather than a constant.
- Capacity needs runtime validation, so `Capacity::new` is fallible and every caller handles a `Result` a const
  parameter would not need.
- No source file states the rule above. A reader asking "why not an array?" finds the answer only here.
- Revisit if the family drops runtime-configured capacity, that is, if `RingConfig::new` no longer takes the slot
  count as a `usize`. The const-generic option then reopens.
- Revisit when a ring must allocate nothing on the heap, for example a ring placed in a `static`. A const-generic
  buffer beside `Buffer` would then be the candidate, with `RingConfig` kept for the runtime path.

See [`src/lib.rs`](../../src/lib.rs) and [`ring_config`](../../../ring_config/readme.md).
