# Item: The SlotIndex the Buffer Trusts

### Scope

**Purpose:** Record the three `ring_types` values that cross this boundary, that
the buffer reaches `SlotIndex` through its accessor rather than its public field,
and that `Seq` enters and is consumed within one line — so the buffer can address
a sequence and can never answer which sequence a slot holds.

**Responsibility:** The addressing values as items: where each appears, how long
it lives, and what the buffer retains of it.

**In Scope:** `ring_store/src/lib.rs:62-257`;
`ring_mpsc/src/lib.rs:330-335`.

**Out of Scope:** That an out-of-range `SlotIndex` is constructible is
[`decisions/001`](../decisions/001_panic_rather_than_option.md). The fold is
[`algorithm/001`](../algorithm/001_one_mask_no_modulo.md).

---

### BF28 — Three Types Cross, and the Buffer Uses the Accessor Rather Than the Public Field

Every signature and field naming a `ring_types` type:

```sh
cd "$(git rev-parse --show-toplevel)"
grep 'SlotIndex\|Seq\|Capacity' ring_store/src/lib.rs | grep -E 'pub (const )?fn |^  (slots|capacity) :' | grep -vE ':[[:space:]]*(///|//!|//)'
```

Live output:

```
  capacity : Capacity,
  pub fn new( capacity : Capacity ) -> Self
  pub const fn capacity( &self ) -> Capacity
  pub fn get( &self, index : SlotIndex ) -> &S
  pub fn get_mut( &mut self, index : SlotIndex ) -> &mut S
  pub fn at( &self, seq : Seq ) -> &S
  pub fn at_mut( &mut self, seq : Seq ) -> &mut S
```

`Capacity` is stored; `SlotIndex` and `Seq` are parameters only, never fields.
And the buffer unwraps a `SlotIndex` through its method, not its tuple field:

```sh
cd "$(git rev-parse --show-toplevel)"
grep 'index\.get()\|index\.0' ring_store/src/lib.rs
```

Live output:

```
    &self.slots[ index.get() ]
    &mut self.slots[ index.get() ]
```

**Finding.** `SlotIndex( pub usize )` exposes its field, and this crate declines
to use it — both unwrap sites go through `SlotIndex::get`. That is the right
habit and it is worth naming, because the field's publicness exists for
`ring_index`'s benefit and is a permission this crate has no need of
([`decisions/001`](../decisions/001_panic_rather_than_option.md) BF7).

The discipline is one-sided, though. The crate's *source* never touches `.0`, and
the crate's *doctests and tests* construct twenty-five indices by tuple syntax,
which is the same permission used the other way round. So the habit holds where a
reader is least likely to look and is abandoned where they are most likely to
copy from — a test file is the first place someone writing against this API will
crib an example, and every example there builds an index by hand.

---

### BF29 — A `Seq` Lives for One Line, So the Buffer Cannot Say What a Slot Holds

`at` and `at_mut` take a `Seq`, fold it, and discard it — the sequence is never
stored, and no field, method or return value carries one. That is the direct
consequence of this crate's own *holds no cursor and no ordering state* claim: a buffer
that remembered which sequence occupied a slot would be holding ordering state.

The consumers need that information, and keep it themselves:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A6 -F '  /// One stamp per slot, holding the sequence whose payload currently occupies' ring_mpsc/src/lib.rs
```

Live output:

```
  /// One stamp per slot, holding the sequence whose payload currently occupies
  /// it. Unpadded on purpose: [`PaddedCursor`] would make this array 64 times
  /// the size of the payload array for a small `S`, to prevent a false-sharing
  /// contention that does not arise — two producers writing adjacent stamps are
  /// two producers that claimed adjacent sequences, which is a handful of
  /// stores on one line rather than a contended loop.
  stamps : Box< [ AtomicSeq ] >,
```

A second array, the same length as the slots, holding exactly the mapping the
buffer refuses to hold.

**Finding.** The split is the design working: `Ring` owns the sequence-to-slot
correspondence and pays for it in a parallel array of atomics, while `Buffer`
stays a plain container that can be indexed by anyone. Every ring gets to choose
its own stamping arrangement — `ring_mpsc` unpadded per slot, `ring_spsc`
something else — precisely because storage has no opinion.

The cost is a shape a reader has to reconstruct. `Buffer::at( seq )` looks like a
sequence-addressed container, and it is not one: it is an index-addressed
container with a fold in front, and the returned `&S` carries no evidence about
which sequence it belongs to. A caller holding a `&S` from `at( Seq( 5 ) )` and a
`&S` from `at( Seq( 5 + capacity ) )` holds two references to the same slot with
nothing to distinguish the laps — which is exactly why the stamps array exists,
and is stated only in the crate that has one.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`item/001`](001_the_slot_as_the_buffer_sees_it.md) | The other item this container holds |
| [`decisions/001`](../decisions/001_panic_rather_than_option.md) | The publicness this crate declines to use, and the panic that rests on it |
| [`algorithm/001`](../algorithm/001_one_mask_no_modulo.md) | The fold a `Seq` passes through |
| [`type/002`](../type/002_the_types_that_cross_the_boundary.md) | The same three types, as types rather than as items |
| [`data_structure/001`](../data_structure/001_three_words_whatever_the_slot_costs.md) | Why no fourth field holds a sequence |

### Sources

| Fact | Where |
|------|-------|
| Every signature naming a `ring_types` type | `ring_store/src/lib.rs:62-257` |
| Both unwraps through `SlotIndex::get` | `ring_store/src/lib.rs:206, 217` |
| The stamps array | `ring_mpsc/src/lib.rs:330-335` |
| This crate's no-ordering-state clause | `ring_store/tests/buffer_test.rs:5` |
| Twenty-five hand-built indices in this crate | Census in `decisions/001` |

### Tests

| Test | Covers |
|------|--------|
| `a_sequence_addresses_the_slot_ring_index_says_it_does` | `Seq` in, `SlotIndex` out, forty times |
| `a_full_lap_overwrites_and_a_partial_one_does_not` | Two sequences a lap apart reaching one slot |
| `indexed_get_and_set_round_trip` | `SlotIndex` as the direct address |
| `ring_mpsc` — the stamp assertions | The mapping the buffer refuses to hold |
| *(to create)* | An assertion that `at( s )` and `at( s + capacity )` return the same address |
