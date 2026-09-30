# Workaround: Capacity Is a Value, Width Is a Type

### Scope

**Purpose:** Record that the heap allocation this crate exists to make exactly
once is itself a workaround — the price of keeping capacity a runtime value —
that a downstream builder is the concrete reason it must stay one, and that the
same family made the opposite choice for the other size parameter without
marking the asymmetry anywhere.

**Responsibility:** The two size parameters a ring carries — slot count and slot
width — and where each one lives.

**In Scope:** `ring_store/src/lib.rs:84`; the two library construction
sites at `ring_mpsc/src/lib.rs:378` and `ring_spsc/src/lib.rs:301`;
`ring_config/src/lib.rs:65-72`.

**Out of Scope:** How the allocation is made is
[`workaround/001`](001_three_steps_to_avoid_two_bounds.md). What it costs once
made is
[`non_functional_requirement/001`](../non_functional_requirement/001_allocate_once_then_never_again.md).

---

### BF52 — The Heap Is the Price of a Runtime Capacity, and a Builder Is Why

Every construction of a `Buffer` in library code, and the crate that decides what
its argument will be:

```sh
cd "$(git rev-parse --show-toplevel)"

# every library construction of a Buffer
grep -r 'Buffer::new' ring_*/src/*.rs | grep -vE ':[[:space:]]*(///|//!|//)' | sort

# where a capacity is computed at runtime, fallibly
command grep -m1 -A7 -F '  pub fn new( slots : usize ) -> Result< Self, RingError >' ring_config/src/lib.rs
```

Live output:

```
ring_mpsc/src/lib.rs:      slots : Buffer::new( capacity ),
ring_spsc/src/lib.rs:      slots : Buffer::new( capacity ),
  pub fn new( slots : usize ) -> Result< Self, RingError >
  {
    Ok
    (
      Self
      {
        capacity : Capacity::new( slots )?,
        wait : WaitKind::default(),
```

**Finding.** `Box< [ S ] >` is a heap allocation, and a heap allocation is
exactly what a ring buffer is usually chosen to avoid. The crate's own module
comment presents "capacity slots allocated once" as the achievement; the
achievement is really that there is one allocation rather than none, and the
reason there is one at all is that `capacity` is a value.

The alternative is available in this language and used elsewhere in this very
family: `Buffer< S, const CAP : usize >` holding `[ S; CAP ]` allocates nothing,
lives wherever it is declared, and would let `Capacity::mask()` fold to a
compile-time constant. What it costs is that the ring's size becomes part of its
type — every function taking a `&Buffer` grows a const parameter, two rings of
different sizes stop being the same type, and a capacity read from a config file
becomes unrepresentable.

That last one is not hypothetical. `ring_config::new` takes `slots : usize` and
returns a `Result`, so the family already has a path where the number of slots is
decided at runtime and may be rejected. A const-generic capacity would make that
crate's central function impossible to write. The heap allocation is what buys
it.

None of this is written down. `new` presents itself as the obvious way to get N
slots; nothing says that a whole design was traded here, or that
`ring_config`'s signature is the thing holding the trade in place. A reader
reasonably asks "why not an array?" and the crate has no answer to offer, even
though a good one exists two crates away.

---

### BF53 — The Other Size Parameter Went the Other Way, on the Same Line

Both parameters meet in this crate's own doc example:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -F '    /// let buffer: Buffer<BytesSlot<8>> = Buffer::new(Capacity::new(16).unwrap());' ring_store/src/lib.rs
```

Live output:

```
    /// let buffer: Buffer<BytesSlot<8>> = Buffer::new(Capacity::new(16).unwrap());
```

**Finding.** Two sizes, one line, opposite treatments. The slot's width — `8` —
is a const generic, checked at compile time, part of the type, and unable to come
from a config file. The slot count — `16` — is a runtime value, validated by a
fallible constructor, absent from the type, and free to come from anywhere.

Both choices are defensible and they are defensible for *different* reasons,
which is precisely what makes the asymmetry worth stating. A slot's width
determines its layout, so it has to be known to size the type at all; making it
a value would push `BytesSlot` onto the heap and give up the inline payload that
is the shape's entire purpose
([`non_functional_requirement/001`](../non_functional_requirement/001_allocate_once_then_never_again.md)
BF43). A ring's capacity determines nothing about layout — only how many of a
known-size thing to make — so it can be a value, and being a value is what lets
it be configured.

So the rule the family actually follows is: *a size that changes the type is a
type parameter; a size that only changes the count is a value.* That is a good
rule. It is stated in neither crate. What a reader meets instead is one line
where `8` is written inside angle brackets and `16` inside parentheses, with
nothing marking the difference as a decision rather than an inconsistency — and
the two library call sites, where `capacity` arrives as an opaque parameter,
give no hint either.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`workaround/001`](001_three_steps_to_avoid_two_bounds.md) | How the allocation this one justifies is actually made |
| [`non_functional_requirement/001`](../non_functional_requirement/001_allocate_once_then_never_again.md) | The one allocation, measured, and the shape that stops the count there |
| [`type/002`](../type/002_the_types_that_cross_the_boundary.md) | `Capacity` as a proposition — why it is fallible at all |
| [`algorithm/001`](../algorithm/001_one_mask_no_modulo.md) | The `mask()` a const capacity would fold at compile time |
| [`integration/001`](../integration/001_four_dependents_two_that_build_on_it.md) | The two dependents that hold the only library call sites |

### Sources

| Fact | Where |
|------|-------|
| Both size parameters on one line | `ring_store/src/lib.rs:84` |
| The two library construction sites | `ring_mpsc/src/lib.rs:378`; `ring_spsc/src/lib.rs:301` |
| A capacity computed at runtime, fallibly | `ring_config/src/lib.rs:65-72` |
| The const-generic width | `ring_slot/src/lib.rs:237` |
| `Capacity`'s fallible constructor | `ring_types/src/capacity.rs:40-51` |

### Tests

| Test | Covers |
|------|--------|
| `the_same_buffer_type_serves_both_slot_shapes` | Both shapes under one capacity — the asymmetry, exercised without being named |
| `exactly_n_slots_are_allocated_for_capacity_n` | A capacity supplied as a value, at four sizes |
| `a_buffer_is_never_empty_because_a_capacity_is_never_zero` | The validation a const parameter would not have needed |
| `ring_config` — `capacity_is_validated_at_construction` | The runtime path that keeps capacity a value |
