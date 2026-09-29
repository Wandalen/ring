# Workaround: Three Steps to Avoid Two Bounds

### Scope

**Purpose:** Record that `new`'s three-line construction is the one route to a
`Box< [ S ] >` that needs neither `unsafe` nor a `Clone` bound, that neither
constraint is named where the three lines are, and that the two arguments which
must stay equal are the same expression written twice with nothing asserting it.

**Responsibility:** The construction sequence in `Buffer::new` — what each of the
three steps buys, and what it costs if one of them drifts.

**In Scope:** `ring_store/src/lib.rs:87-93`; the two constraints it
routes around, at `Cargo.toml:227` and `ring_slot/src/lib.rs:41`.

**Out of Scope:** What the allocation costs once made is
[`non_functional_requirement/001`](../non_functional_requirement/001_allocate_once_then_never_again.md).
Why the `Default` bound is there at all is
[`data_structure/002`](../data_structure/002_the_default_bound_is_a_bound_on_the_type.md).

---

### BF50 — Three Lines That Route Around Two Constraints, Neither Named

The construction, whole:

```sh
cd "$(git rev-parse --show-toplevel)"

# the three steps
command grep -m1 -B1 -A5 -F '  pub fn new( capacity : Capacity ) -> Self' ring_store/src/lib.rs

# the first constraint: no unsafe anywhere in the workspace
grep 'unsafe-code' Cargo.toml   # unanchored: the members list above it grows

# the second: the trait declares no supertraits, so Clone is not already there
grep 'pub trait Slot' ring_slot/src/lib.rs
```

Live output:

```
  #[ must_use ]
  pub fn new( capacity : Capacity ) -> Self
  {
    let mut slots = Vec::with_capacity( capacity.get() );
    slots.resize_with( capacity.get(), S::default );
    Self { slots : slots.into_boxed_slice(), capacity }
  }
unsafe-code = "deny"
pub trait Slot
```

The two obvious alternatives, measured and compiled:

```
--- the three steps, and two ways to get them wrong ---
  with_capacity(16) + resize_with(16)              1 alloc, 0 realloc,  128 bytes
  with_capacity(16) + resize_with(8)   [mismatch]  1 alloc, 1 realloc,  128 bytes
  Vec::new() + resize_with(16)         [no reserve] 1 alloc, 0 realloc,  128 bytes
  vec![ default; 16 ]                  [needs Clone] 1 alloc, 0 realloc,  128 bytes
```

The last row allocates identically — and does not compile here:

```
error[E0277]: the trait bound `S: Clone` is not satisfied
 --> /tmp/-cl/probe.rs:9:26
  |
9 |     Self { slots : vec![ S::default(); n ].into_boxed_slice() }
  |                    ------^^^^^^^^^^^^-----
  |                    |     |
  |                    |     the trait `Clone` is not implemented for `S`
  |                    required by a bound introduced by this call
  |
note: required by a bound in `std::vec::from_elem`
help: consider further restricting type parameter `S` with trait `Clone`
```

**Finding.** The three lines are not a style choice; they are the intersection of
two constraints that meet nowhere else in the crate.

The first is `unsafe-code = "deny"`, set once at the workspace and re-allowed by
exactly two crates ([`integration/002`](../integration/002_every_unsafe_block_in_the_family.md)
BF4). The direct route to N default-initialised slots — allocate, write each in
place, assume initialised — is closed here, so the array has to be built through
a safe `Vec` and handed over.

The second is subtler. `vec![ S::default(); n ]` is the idiomatic one-liner and
it produces byte-identical results, but `from_elem` clones its element, so it
requires `S : Clone`. The `Slot` trait declares no supertraits, so the bound is
not already present — it would have to be added to `impl< S : Default >`, where
`new` lives, and from there to every signature that mentions a `Buffer`. For
`TypedSlot< T >`, whose `Clone` is conditional on `T`, that propagates all the
way to the payload: a ring carrying a non-cloneable handle would stop compiling.
The one-line version would buy nothing and cost the ability to move
non-`Clone` payloads through the ring, which is most of what a ring is for.

`resize_with` is what dodges it — it takes `FnMut() -> T`, calls `S::default`
per slot, and never clones. So the sequence reads: reserve exactly, fill by
calling, freeze. Each step is load-bearing and none of the three says so. A
reader arriving at `new` sees three lines that could plausibly be one, and the
`git blame` for a "simplification" to `vec![ S::default(); n ]` would be
rejected by the compiler — but the equally plausible one to `Vec::new()` would
not be.

---

### BF51 — The Two Arguments That Must Agree Are the Same Expression, Twice

`capacity.get()` appears on both line 90 and line 91. They are not independently
derived — they are one value read twice — so today they cannot disagree. The
measurement above says what happens if they ever do:

| Construction | Allocations | Reallocations |
|--------------|-------------|---------------|
| `with_capacity( 16 )` + `resize_with( 16 )` | 1 | 0 |
| `with_capacity( 16 )` + `resize_with( 8 )` | 1 | **1** |
| `Vec::new()` + `resize_with( 16 )` | 1 | 0 |

**Finding.** Two things sit in that table, and they point opposite ways.

The mismatch row is the hazard. `into_boxed_slice` must produce a slice whose
allocation is exactly its length, so a `Vec` with spare capacity is shrunk — and
shrinking is a reallocation. A buffer built with disagreeing arguments still
works, still holds the right number of slots, and still passes every test in the
file; it just allocates twice instead of once, silently, in the one crate whose
entire reason for existing is that it allocates once
([`non_functional_requirement/001`](../non_functional_requirement/001_allocate_once_then_never_again.md)
BF42). Nothing asserts the property, so nothing would catch it.

The `Vec::new()` row cuts the other way and is worth stating precisely, because
it looks like the first step is redundant. On this allocator, at this capacity,
`resize_with` on an empty `Vec` also lands one allocation — the growth path
reaches the exact size in a single step. That is a measurement, not a guarantee:
`Vec` documents `reserve` as securing space for *at least* the requested count
and says nothing about how growth reaches a given length. Dropping
`with_capacity` would leave the allocate-once property resting on allocator
behaviour rather than on the API, which is the difference between a property and
a coincidence. The first step earns its line — and, like the other two, does not
explain that it does.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`workaround/002`](002_capacity_is_a_value_width_is_a_type.md) | The other constraint the construction accepts: a runtime capacity forces the heap |
| [`integration/002`](../integration/002_every_unsafe_block_in_the_family.md) | The `unsafe` denial these steps route around |
| [`non_functional_requirement/001`](../non_functional_requirement/001_allocate_once_then_never_again.md) | The property the mismatch would break |
| [`data_structure/002`](../data_structure/002_the_default_bound_is_a_bound_on_the_type.md) | The `Default` bound `resize_with` consumes |
| [`lifecycle/001`](../lifecycle/001_allocate_once_borrow_forever_drop_plainly.md) | Where this construction sits in the buffer's life |

### Sources

| Fact | Where |
|------|-------|
| The three steps | `ring_store/src/lib.rs:87-93` |
| The `unsafe` denial | `Cargo.toml:227` |
| `Slot` declares no supertraits | `ring_slot/src/lib.rs:41` |
| Allocation and reallocation counts for all four constructions | Release probe under a counting global allocator, quoted above |
| That `vec!` requires `Clone` under this bound | `rustc` compile probe, quoted above |
| `TypedSlot`'s conditional `Clone` | `ring_slot/src/lib.rs:83-84` |

### Tests

| Test | Covers |
|------|--------|
| `exactly_n_slots_are_allocated_for_capacity_n` | The length the two arguments produce — not the allocation count |
| `every_slot_starts_empty` | That `resize_with` called `S::default` for each |
| `the_same_buffer_type_serves_both_slot_shapes` | Construction under both shapes |
| *(to create)* | An allocation-counting assertion that construction costs exactly one — which is what would catch a drifting second argument |
