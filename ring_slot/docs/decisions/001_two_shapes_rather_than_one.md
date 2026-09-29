# Decision: Two Shapes Rather Than One

### Scope

**Purpose:** Record the ruling that the ring carries two slot shapes instead of
one, the cost asymmetry argued for it, and what the argument still owes.

**Responsibility:** The founding decision of the crate — why `TypedSlot` and
`BytesSlot` both exist rather than one of them.

**In Scope:** `ring_slot/src/lib.rs:7-17`; the `Slot` trait as the
mechanism that keeps the two from forking the ring.

**Out of Scope:** The bench that would price the asymmetry, recorded as SL4 in
[`integration/002`](../integration/002_where_the_second_shape_stops.md). The
length-versus-flag ruling inside `BytesSlot` is
[`decisions/002`](002_a_length_rather_than_a_flag.md).

---

## The Ruling as Stated

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A10 -F '//! `docs/feature/182_typed_slot_and_bytes_slot.md` asks for two slot shapes over' ring_slot/src/lib.rs
```

Live output:

```
//! `docs/feature/182_typed_slot_and_bytes_slot.md` asks for two slot shapes over
//! one ring: a typed slot for traffic whose shape is known at compile time, and
//! a bytes slot for traffic that arrives from outside and is decoded later.
//! The reason for both is a cost asymmetry — force everything through bytes and
//! in-process command traffic pays an encoding cost for nothing; type the ring
//! and opaque host traffic has nowhere to go without a second ring built for it.
//! That asymmetry is argued here, not measured: the family's own bench crate
//! times `TypedSlot` on both ring implementations and has never instantiated
//! `BytesSlot`, so the difference this paragraph claims has no number behind it.
//!
//! The feature's constraint is that "both use the same claim, gating, and drain
```

The argument is symmetric and complete on its face. Collapse to bytes and typed
traffic pays for encoding it does not need; collapse to typed and untyped traffic
needs a second ring. Two shapes is the only option that serves both without
forking the implementation, and the trait is what stops the fork.

---

### SL5 — The Constraint Is Enforced by a Trait, Not by Convention

This crate's own constraint — "the difference is confined to what a slot contains" —
could have been a rule people follow. It is instead a type:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A7 -F 'pub trait Slot' ring_slot/src/lib.rs
```

Live output:

```
pub trait Slot
{
  /// Whether this slot currently holds nothing.
  fn is_empty( &self ) -> bool;

  /// Return the slot to its empty state.
  ///
  /// **Not a promise to overwrite.** For a shape that owns what it stores
```

Two methods, both about occupancy, neither about content. A downstream crate
bounded by `S : Slot` *cannot* branch on the shape, because the trait exposes
nothing that would let it — there is no `as_bytes`, no associated type, no
downcast. The constraint is not enforced by review; it is enforced by there
being no way to violate it.

**Finding.** The ruling's mechanism is stronger than the ruling required. The
ruling requires that both shapes use the same claim, gating, and drain; the trait
delivers that by making the shapes *indistinguishable* to everything above them,
which is a strictly stronger property and the reason four crates could stay
generic across three layers without coordination
([`integration/001`](../integration/001_seven_dependents_and_four_that_stay_generic.md)
SL1).

The cost of that strength shows up in
[`api/002`](../api/002_a_trait_with_two_methods.md): a caller who wants content
must leave the trait entirely, which is why `ring_event` had to invent `Fill` and
`Peek` rather than extending `Slot`.

---

### SL6 — The Asymmetry Is Argued, Not Measured

The decision rests on a claim about relative cost: encoding in-process command
traffic is waste, and a second ring for host traffic is worse. Both halves are
plausible and neither is quantified anywhere in the family.

```sh
cd "$(git rev-parse --show-toplevel)"
# anything that measures the two shapes against each other
grep -r 'BytesSlot' ring_bench/ 2>/dev/null || echo "  ring_bench: no BytesSlot anywhere"
grep -rl 'copy cost\|encoding cost' ring_*/src/*.rs || echo "  no crate discusses the cost in code"
```

Live output:

```
ring_bench/Cargo.toml:#                benchmark exercises (the other, `BytesSlot< N >`, round-trips
ring_bench/docs/integration/002_the_only_consumer_of_two_contract_names.md:#                benchmark exercises (the other, `BytesSlot< N >`, round-trips
ring_bench/docs/integration/001_declared_edges_and_the_three_that_were_missing.md:`TypedSlot< T >` at `ring_slot/src/lib.rs:176` and `BytesSlot< N >` at
ring_slot/src/lib.rs
```

The second line is the whole result: the only file in the family that discusses
the cost is the module comment arguing from it.

**Finding.** The founding ruling of the crate is supported by an argument that
reads correctly and by no measurement at all. This is a reasonable state for a
decision this shape — the argument is about *avoiding* a cost, and the cheapest
proof is that nobody pays it — but it leaves the ruling unfalsifiable in its own
repository. This crate's own argument calls for exactly this measurement, and
the family's bench crate does not perform it (SL4).

The consequence is narrow and worth stating plainly: if the encoding cost for a
typical `Record` turned out to be negligible, the correct ruling would be one
shape, and nothing currently in the repository could tell you.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`decisions/002`](002_a_length_rather_than_a_flag.md) | The second ruling, inside `BytesSlot`, and the distinction it gives up |
| [`pattern/001`](../pattern/001_one_trait_two_shapes.md) | The shape this ruling produces, and where else the family uses it |
| [`api/002`](../api/002_a_trait_with_two_methods.md) | What the trait deliberately does not offer, and what that costs downstream |
| [`integration/002`](../integration/002_where_the_second_shape_stops.md) | The unmeasured asymmetry, as a coverage gap rather than a ruling |

### Sources

| Fact | Where |
|------|-------|
| The ruling and its argument | `ring_slot/src/lib.rs:7-17` |
| The trait that enforces it | `ring_slot/src/lib.rs:38-45` |
| No measurement anywhere | `ring_bench/` |

### Tests

| Test | Covers |
|------|--------|
| `both_shapes_drive_through_the_trait_alone` | That the trait alone drives either shape — the constraint, directly |
| `the_trait_reports_the_same_cycle_for_both_shapes` | That the two are indistinguishable through the trait across a full cycle |
| *(to create)* | A bench pricing the encoding cost the ruling avoids |
