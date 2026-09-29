# Algorithm: One Allocation, N Defaults

### Scope

**Purpose:** Record the three-step construction body, that it costs exactly one
allocation at any capacity and calls `Default::default` once per slot, and that
those `N` calls are the crate's only per-slot work and the whole of what it asks
a slot shape for.

**Responsibility:** How a buffer comes into existence — the allocation, the
initialisation, and what each costs.

**In Scope:** `ring_store/src/lib.rs:67-93`; the counting-allocator
measurements.

**Out of Scope:** That `Default` is thereby a bound on the whole type is
[`data_structure/002`](../data_structure/002_the_default_bound_is_a_bound_on_the_type.md).
Addressing is [`algorithm/001`](001_one_mask_no_modulo.md).

---

## The Three Steps

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A26 -F '  /// Allocate exactly `capacity` empty slots, once.' ring_store/src/lib.rs
```

Live output:

```
  /// Allocate exactly `capacity` empty slots, once.
  ///
  /// Bounded on `Default` alone rather than on `Slot`, because allocation needs
  /// nothing a slot offers beyond an empty value. The two crates that assemble a
  /// ring rely on that: they store `Buffer< UnsafeCell< S > >`, putting the cell
  /// on each slot rather than around the whole buffer, and `UnsafeCell< S >` is
  /// `Default` without being a `Slot`. Wrapping the buffer instead would mean a
  /// producer forming `&mut Buffer` — an exclusive claim over the *entire*
  /// allocation — to write one slot, which two producers writing different slots
  /// violate; per-slot cells make each write claim exactly the slot it touches.
  /// See [`Buffer::clear`] for the operations that do still need `Slot`.
  ///
  /// ```
  /// use ring_store::Buffer;
  /// use ring_slot::BytesSlot;
  /// use ring_types::Capacity;
  ///
  /// let buffer : Buffer< BytesSlot< 8 > > = Buffer::new( Capacity::new( 16 ).unwrap() );
  /// assert_eq!( buffer.len(), 16 );
  /// ```
  #[ must_use ]
  pub fn new( capacity : Capacity ) -> Self
  {
    let mut slots = Vec::with_capacity( capacity.get() );
    slots.resize_with( capacity.get(), S::default );
    Self { slots : slots.into_boxed_slice(), capacity }
  }
```

Reserve, fill, freeze. The doc claims *exactly `capacity` empty slots, once* —
the reached-test's first clause — and the three steps are chosen so that claim holds
literally rather than approximately. The bound paragraph in the middle is why the
signature reads `S : Default` and not `S : Slot + Default`; what that costs a
shape author is
[`data_structure/002`](../data_structure/002_the_default_bound_is_a_bound_on_the_type.md)
and not this instance's subject.

---

### BF12 — One Allocation at Any Capacity, and None Afterwards

The body has three plausible places to allocate. Measured with a counting global
allocator:

```
--- (7) what one construction costs ---
  capacity    1: allocations = 1, bytes =      8, Default::default calls = 1
  capacity   16: allocations = 1, bytes =    128, Default::default calls = 16
  capacity 1024: allocations = 1, bytes =   8192, Default::default calls = 1024
```

One, at every scale. `Vec::with_capacity` takes the whole block up front;
`resize_with` fills to exactly that reservation so it never grows; and
`into_boxed_slice` on a `Vec` whose length equals its capacity is a no-op rather
than a shrinking copy. Each step is load-bearing — reserving with `Vec::new()`
instead would reallocate on the way up, and a `resize_with` past the reservation
would too.

And nothing after construction allocates at all:

```
--- (8) what everything after construction costs ---
  10000 at_mut + at + all_empty + iter + clear: allocations = 0
```

Ten thousand writes, a read, a full fold, a full iteration and a whole-buffer
`clear` — zero. The one case that does allocate is the payload's own business,
not the buffer's:

```
--- (9) a payload that allocates does so on its own account ---
  one String published into a slot: allocations = 1
```

**Finding.** The allocation claim is exact, holds at every capacity, and survives
every operation the type offers. That is the substance of the reached-test's *"a fixed
allocation of slots"* and the reason `if Missing` names "a growable vector
guarded by a lock" as the alternative — a `Vec` that could grow would put an
allocation on the publish path.

Worth recording as an achieved property rather than an intention: none of it is
asserted by a test. `exactly_n_slots_are_allocated_for_capacity_n` checks that
`len()` is `n`, which a triple-reallocating constructor would also satisfy, and
`clear_empties_every_slot_and_keeps_the_allocation` compares one slot's address
before and after `clear`, which is the closest the suite comes. The single most
valuable assertion this crate could add is the one measured above: a counting
allocator around `Buffer::new`, pinning `allocations == 1`.

---

### BF13 — The Per-Slot Cost Is `Default::default`, Called Once for Every Slot

`Default::default` is invoked exactly `capacity` times per construction — 1, 16
and 1024 in the runs above, tracking capacity precisely. It is the only per-slot
work `new` does, and — since `new` is bounded on `Default` alone — the whole of
what constructing a `Buffer` asks a slot shape for; `Slot`'s own two methods are
required only by the sweep, one impl block further down.

For the two shipped shapes that is nearly free: `TypedSlot::default` is `None`
and `BytesSlot::default` is a zeroed array. The zeroed array is not nothing at
scale:

```
--- (10) N Defaults, whatever N costs ---
  256 x BytesSlot< 4096 >: allocations = 1, bytes = 1050624
```

One megabyte, one allocation, 256 array initialisations.

**Finding.** The construction cost is linear in capacity with a constant the slot
shape chooses, and `ring_store` neither knows nor bounds that constant. A shape
whose `default()` opened a file or took a lock would do so `capacity` times
inside a function documented as "allocate exactly `capacity` empty slots, once" —
the *allocation* is once; the initialisation is `N` times, and the doc's "once"
attaches to the wrong noun for a reader skimming it.

This is the cheapest possible arrangement for the shapes that exist and the
finding is not that it should change. What is missing is one sentence saying that
`S::default` runs per slot, so a shape author knows the function they are writing
is on a ring's construction path `capacity` times over. `ring_slot`, which
defines the trait, says nothing about `Default` at all — the requirement appears
only in this crate's module comment
([`data_structure/002`](../data_structure/002_the_default_bound_is_a_bound_on_the_type.md)).

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`algorithm/001`](001_one_mask_no_modulo.md) | The other algorithm — addressing |
| [`data_structure/002`](../data_structure/002_the_default_bound_is_a_bound_on_the_type.md) | `Default` as a requirement on every slot shape in the family |
| [`non_functional_requirement/001`](../non_functional_requirement/001_allocate_once_then_never_again.md) | The no-allocation-after-construction requirement these numbers satisfy |
| [`lifecycle/001`](../lifecycle/001_allocate_once_borrow_forever_drop_plainly.md) | The allocation's whole life, from `new` to drop |
| [`invariant/001`](../invariant/001_capacity_equals_length_always.md) | The property `new` establishes and nothing can break |

### Sources

| Fact | Where |
|------|-------|
| The three-step body | `ring_store/src/lib.rs:87-93` |
| The reached-test's fixed-allocation clause | `ring_store/tests/buffer_test.rs:3-6` |
| Allocation and `Default` counts | Counting-allocator probe, quoted above |
| `TypedSlot`/`BytesSlot` `Default` impls | `ring_slot/src/lib.rs:170, 379` |
| `Default` unmentioned by the trait's crate | `ring_slot/src/lib.rs` — no occurrence in any doc comment |

### Tests

| Test | Covers |
|------|--------|
| `exactly_n_slots_are_allocated_for_capacity_n` | The slot count, across six capacities |
| `every_slot_starts_empty` | That `S::default` produced empty slots |
| `clear_empties_every_slot_and_keeps_the_allocation` | Address identity across `clear` — the nearest thing to an allocation assertion |
| `the_same_buffer_type_serves_both_slot_shapes` | Both shipped `Default` impls, through one constructor |
| *(to create)* | A counting allocator around `Buffer::new`, asserting exactly one allocation |
