# Pattern: Every Write Is a Borrow

### Scope

**Purpose:** Record that every operation on a constructed buffer takes or returns
a borrow, that no by-value form exists in any direction, and that the two
iterator methods name `core::slice::Iter` in their public types — pinning a
private field's representation into the public API.

**Responsibility:** The borrow discipline as a shape: what it buys, what corner
it leaves empty, and what it fixes in place.

**In Scope:** `ring_store/src/lib.rs:88-301`.

**Out of Scope:** Which functions carry `#[ must_use ]` is
[`api/001`](../api/001_twelve_functions_three_const_seven_must_use.md). The
three-word layout is
[`data_structure/001`](../data_structure/001_three_words_whatever_the_slot_costs.md).

---

### BF36 — Twelve Functions, One By-Value Parameter, and No Way Out

The whole public surface, with its associated types:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -E 'pub (const )?fn |^  type ' ring_store/src/lib.rs
```

Live output:

```
  pub fn new( capacity : Capacity ) -> Self
  pub fn clear( &mut self )
  pub fn all_empty( &self ) -> bool
  pub const fn capacity( &self ) -> Capacity
  pub const fn len( &self ) -> usize
  pub const fn is_empty( &self ) -> bool
  pub fn get( &self, index : SlotIndex ) -> &S
  pub fn get_mut( &mut self, index : SlotIndex ) -> &mut S
  pub fn at( &self, seq : Seq ) -> &S
  pub fn at_mut( &mut self, seq : Seq ) -> &mut S
  pub fn iter( &self ) -> core::slice::Iter< '_, S >
  pub fn iter_mut( &mut self ) -> core::slice::IterMut< '_, S >
  type Item = &'a S;
  type IntoIter = core::slice::Iter< 'a, S >;
  type Item = &'a mut S;
  type IntoIter = core::slice::IterMut< 'a, S >;
```

`new` is the only function taking a value by value, and it takes a `Capacity` —
never an `S`. Everything after construction is `&self` or `&mut self`, and the
four accessors return `&S` or `&mut S`. And there is no by-value exit:

```sh
cd "$(git rev-parse --show-toplevel)"
grep 'fn into_inner\|IntoIterator for Buffer<\|IntoIterator for Buffer <\|fn into_vec\|fn into_slots' ring_store/src/lib.rs \
  || echo '  none — a Buffer cannot be taken apart'
```

Live output:

```
  none — a Buffer cannot be taken apart
```

**Finding.** A buffer is created, borrowed, and dropped. There is no
`into_inner`, no `IntoIterator for Buffer< S >`, no way to recover the
`Box< [ S ] >` or a `Vec< S >` of the payloads. The lattice of accessors is
complete on the borrow axis — two address forms times two borrow strengths, plus
matched iterator pairs and matched `IntoIterator` impls for `&` and `&mut` — and
empty on the by-value axis.

That is the right shape for the consumer that exists. Both rings hold their
buffer inside an `UnsafeCell` for the ring's whole life and reach slots through
raw pointers; a by-value teardown would be an operation neither can perform and a
second way to move payloads out, competing with the `set`-returns-displaced route
the family already settled on. The write-as-borrow discipline is also what keeps
`Buffer` from ever naming a payload type, which is what lets one definition serve
both slot shapes ([`item/001`](../item/001_the_slot_as_the_buffer_sees_it.md)).

The corner it leaves empty has one visible cost, in tests rather than in
production: asserting on a buffer's final contents means walking `iter` and
copying, because there is no way to take the slots and compare them as a
collection. That is a small price and the right trade, and it is nowhere stated
— a reader looking for `into_inner` finds absence, not a reason.

---

### BF37 — A Private Field's Type Is Pinned by Four Public Signatures

`slots` is private, and its representation is nonetheless fixed by the API:

```sh
cd "$(git rev-parse --show-toplevel)"
grep 'core::slice::' ring_store/src/lib.rs
```

Live output:

```
  pub fn iter( &self ) -> core::slice::Iter< '_, S >
  pub fn iter_mut( &mut self ) -> core::slice::IterMut< '_, S >
  type IntoIter = core::slice::Iter< 'a, S >;
  type IntoIter = core::slice::IterMut< 'a, S >;
```

Four occurrences, all public. `iter` and `iter_mut` name the concrete slice
iterator in their return types rather than `impl Iterator< Item = &S >`, and the
two `IntoIterator` impls name it again as an associated type.

**Finding.** Encapsulation here is one-sided. The field is private, so no caller
can reach the box — and the box's type is still a published part of the contract,
because four signatures say `core::slice::Iter`. Changing storage to anything not
producing a `slice::Iter` — two half-slices, a chunked layout, a padded
per-element wrapper — would break every downstream crate, even though nothing
downstream can name the field.

The inherent methods could return `impl Iterator< Item = &S >` and lose nothing;
`slice::Iter` carries its own `#[ must_use ]`, which is why neither method needs
one of its own
([`api/001`](../api/001_twelve_functions_three_const_seven_must_use.md) BF14),
and `impl Iterator` inherits that. The `IntoIterator` impls genuinely need a named
type, which a newtype would supply.

Whether that is worth doing is a judgement about how likely the layout is to
change, and given `data_structure/001`'s three-word pin and the reached-test's
allocate-once clause, the honest answer is *not very*. What is worth recording is
that the constraint exists and is invisible: a maintainer reading `slots :
Box< [ S ] >` behind a private marker will reasonably believe they can change it,
and the four signatures that say otherwise are ninety lines away.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`pattern/001`](001_delegate_the_fold_own_the_storage.md) | The other shape this crate commits to |
| [`api/001`](../api/001_twelve_functions_three_const_seven_must_use.md) | The same twelve functions, counted by attribute |
| [`api/002`](../api/002_six_ways_to_reach_a_slot.md) | The accessor lattice as reachability |
| [`item/001`](../item/001_the_slot_as_the_buffer_sees_it.md) | Why the write must be a borrow |
| [`data_structure/001`](../data_structure/001_three_words_whatever_the_slot_costs.md) | The layout these four signatures pin |

### Sources

| Fact | Where |
|------|-------|
| The full public surface | `ring_store/src/lib.rs:88-301` |
| No by-value form | `ring_store/src/lib.rs` — no occurrence |
| The four slice-typed signatures | `ring_store/src/lib.rs:269, 275, 284, 295` |
| `slice::Iter`'s own `#[ must_use ]` | Census in `api/001` |
| The rings' `UnsafeCell` ownership | `ring_mpsc/src/lib.rs:314-320`; `ring_spsc/src/lib.rs` |

### Tests

| Test | Covers |
|------|--------|
| `indexed_get_and_set_round_trip` | The write expressed as an exclusive borrow |
| `iteration_visits_slots_in_index_order` | The shared iterator |
| `mutable_iteration_reaches_every_slot` | The exclusive iterator |
| `a_borrowed_buffer_iterates_directly` | Both `IntoIterator` impls |
| `the_same_buffer_type_serves_both_slot_shapes` | What never naming a payload type buys |
| *(to create)* | A compile-fail assertion that a `Buffer` cannot be consumed into its slots |
