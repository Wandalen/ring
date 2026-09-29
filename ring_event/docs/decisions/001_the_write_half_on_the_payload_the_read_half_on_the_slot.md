# Decisions: The Write Half on the Payload, the Read Half on the Slot

### Scope

**Purpose:** Record the crate's central design decision — that its two traits sit
on opposite sides of the same operation — establish what each side buys, and
measure how much of it anything actually uses.

**Responsibility:** `Fill< S >`'s placement on the payload, `Peek`'s placement on
the slot, the extension points that follow from each, and the census of
implementors outside the crate.

**In Scope:** `ring_event/src/lib.rs:39-43`, `:53`, `:66`, `:80`,
`:88-92`, `:103`, `:127`, `:137`; probes below.

**Out of Scope:** Why the functions are free rather than methods is
[`decisions/002`](002_three_free_functions_instead_of_methods.md). The associated
type's own cost is [`type/001`](../type/001_an_associated_type_with_a_lifetime.md).

---

## Two Traits, Facing Opposite Ways

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the two traits, and where each is implemented --'
command grep 'pub trait\|^impl' ring_event/src/lib.rs
echo '  -- the stated rationale for putting the write half on the payload --'
command grep -m1 -A4 -F '/// A payload that knows how to enter a slot of shape `S`.' ring_event/src/lib.rs
echo '  -- and everything the read half says about sitting on the slot --'
command grep -m1 -A4 -F '/// A slot that knows what a reader gets back from it.' ring_event/src/lib.rs
echo '  -- other implementors found: 0 (expected) --'
command grep -r 'impl.*Fill<\|impl.*Peek for' --include=*.rs . | command grep -v '^ring_event/src' || true
printf '    hits: %s\n' "$( command grep -r 'impl.*Fill<\|impl.*Peek for' --include=*.rs . | command grep -v '^ring_event/src' | wc -l )"
echo '  -- how the suite covers the payload direction --'
command grep -m1 -A4 -F 'fn fill_is_implemented_on_the_payload_so_a_new_payload_needs_no_slot_change()' ring_event/tests/event_test.rs
```

Live output:

```
  -- the two traits, and where each is implemented --
pub trait Fill< S >
impl< T > Fill< TypedSlot< T > > for T
impl< const N : usize > Fill< BytesSlot< N > > for &[ u8 ]
pub trait Peek
impl< T > Peek for TypedSlot< T >
impl< const N : usize > Peek for BytesSlot< N >
  -- the stated rationale for putting the write half on the payload --
/// A payload that knows how to enter a slot of shape `S`.
///
/// Implemented on the *payload*, not the slot, so adding a payload kind never
/// touches the slot types — and so `publish_into` needs no match, no downcast
/// and no enum of shapes.
  -- and everything the read half says about sitting on the slot --
/// A slot that knows what a reader gets back from it.
///
/// The read half of the shared path. `Out` is a lifetime-parameterised
/// associated type because the two shapes return borrows of different things —
/// see the module documentation for why neither is flattened into the other.
  -- other implementors found: 0 (expected) --
    hits: 0
  -- how the suite covers the payload direction --
fn fill_is_implemented_on_the_payload_so_a_new_payload_needs_no_slot_change()
{
  // Demonstrated by using the trait directly rather than through the free
  // function: this is what a caller adding a payload kind would write.
  let mut slot = TypedSlot::empty();
```

---

## What a Downstream Crate Can Actually Add

*Both probes below are frozen at authoring time — the scratch crate that ran
them has since been swept — and no drift was found: `Fill`, `Peek` and
`Slot`'s signatures at `lib.rs:53`, `:80`, `:103`, `:127`, `:137` are
unchanged from what these probes exercise, and neither recorded line depends
on `BytesSlot`'s `Debug`/`PartialEq` — the one part of this crate's
dependency surface since rewritten (see
[`invariant/002`](../invariant/002_a_refused_fill_changes_nothing.md)). The
recorded output should still reproduce; it just cannot be re-run to confirm.*

Two probes, each writing the impl a consumer would write. The first adds a payload
kind; the second adds a slot shape the family does not have.

```rust
// -ev_probe/src/bin/extend_payload.rs — a new payload, no slot changed
struct MyId( u32 );

impl Fill< BytesSlot< 8 > > for MyId
{
  fn fill( self, slot : &mut BytesSlot< 8 > ) -> Result< (), RingError >
  {
    slot.write( &self.0.to_le_bytes() )
  }
}
```

```rust
// -ev_probe/src/bin/extend_slot.rs — a third shape, no payload changed
#[ derive( Debug, Default ) ]
struct CountingSlot { held : Option< u32 >, writes : usize }

impl Slot for CountingSlot
{
  fn is_empty( &self ) -> bool { self.held.is_none() }
  fn clear( &mut self ) { self.held = None; }
}

impl Peek for CountingSlot
{
  type Out< 'a > = &'a u32;
  fn peek( &self ) -> Option< &u32 > { self.held.as_ref() }
}

impl Fill< CountingSlot > for u32
{
  fn fill( self, slot : &mut CountingSlot ) -> Result< (), RingError >
  {
    slot.held = Some( self );
    slot.writes += 1;
    Ok( () )
  }
}
```

```
  -- a new payload kind, added downstream --
    into a byte slot   Some([7, 0, 0, 0])
    into a typed slot  Some(9)
  -- a new slot shape, added downstream --
    published          Some(5)
    recycled           None
    third shape        CountingSlot { held: None, writes: 1 }
```

---

### EV13 — Each Half Is Open in the Direction the Other Half Is Closed, and Only One Half Says So

`Fill< S >` takes the slot as a type parameter and is implemented on the payload.
`Peek` takes no parameter at all and is implemented on the slot, with the payload
side expressed as an associated type. The impl headers are the whole design in
four lines: `Fill< TypedSlot< T > > for T` and `Fill< BytesSlot< N > > for &[ u8 ]`
against `Peek for TypedSlot< T >` and `Peek for BytesSlot< N >`.

The consequence is that each trait is extensible along a different axis. A new
payload kind is a new `Fill` impl and no slot changes. A new slot shape is a new
`Peek` impl and no payload changes. Neither direction requires editing anything in
this crate, which is what makes "one path" survive contact with a family that will
grow more shapes than the two it has.

**Finding.** Only half of that is documented. `Fill`'s doc comment states its side
outright — "Implemented on the *payload*, not the slot, so adding a payload kind
never touches the slot types" — and it is the clearest sentence in the crate.
`Peek`'s doc comment opens with "A slot that knows what a reader gets back from
it" and then spends its remaining lines on why `Out` carries a lifetime. It never
says that the trait sits on the slot for the mirror-image reason, or that the pair
is a pair.

A reader who studies `Fill` learns the design principle and then, reaching `Peek`,
finds the same principle applied without being named. The symmetry is the reason
the crate is two traits rather than one, and it is visible only by holding the two
declarations side by side.

---

### EV14 — Both Extension Points Work and Neither Has a User

The census is unambiguous: outside `ring_event/src`, the workspace contains zero
`Fill` impls and zero `Peek` impls. Not in another crate, not in a test, not in a
doctest. Every implementor of either trait is one of the four the crate ships.

The suite's `fill_is_implemented_on_the_payload_so_a_new_payload_needs_no_slot_change`
is the closest thing to coverage, and its own comment is precise about what it
does: it uses the trait directly rather than through the free function, "what a
caller adding a payload kind would write." What it does not do is add a payload
kind — it calls `.fill()` on a `u32`, which the blanket impl already covers. The
extension point named in the test's name is not exercised by the test.

Both probes above show it would work, including in the direction nothing
documents. A local payload type takes its own `Fill< BytesSlot< 8 > >` impl while
keeping the blanket impl's `Fill< TypedSlot< MyId > >`, and both round-trip. A
local slot type takes `Slot`, `Peek` and an inbound `Fill< CountingSlot > for u32`
— a foreign trait parameterised by a local type, implemented for a foreign type —
and passes through all three free functions unmodified, including `recycle`.

**Finding.** The crate's stated purpose is that two shapes take one path, and what
it has actually built is a path any number of shapes can take. That is a better
property than the one claimed, and it is recorded nowhere and pinned by nothing.
A third shape added tomorrow would be the first thing ever to test whether the
design does what its author intended, at exactly the moment that answer stops
being cheap to change.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`decisions/002`](002_three_free_functions_instead_of_methods.md) | The other half of the same shape decision |
| [`type/002`](../type/002_a_blanket_impl_over_every_type_there_is.md) | The blanket impl a new payload has to coexist with |
| [`api/001`](../api/001_two_traits_three_functions_one_associated_type.md) | The three bounds a caller pays for the split |
| [`integration/002`](../integration/002_the_ring_writes_slots_without_this_crate.md) | Why no shape has been added yet |

### Sources

| Fact | Where |
|------|-------|
| `Fill` on the payload, parameterised by the slot | `ring_event/src/lib.rs:53`, `:66`, `:80` |
| `Peek` on the slot, payload as an associated type | `ring_event/src/lib.rs:103`, `:127`, `:137` |
| The stated rationale, write half only | `ring_event/src/lib.rs:41-43` |
| `Peek`'s doc saying nothing about its own placement | `ring_event/src/lib.rs:88-92` |
| Zero implementors of either trait outside the crate | Census above |
| The payload-direction test not adding a payload | `ring_event/tests/event_test.rs:218-221` |
| A downstream payload and a downstream slot shape both working | Probes above |

### Tests

| Test | Covers |
|------|--------|
| `fill_is_implemented_on_the_payload_so_a_new_payload_needs_no_slot_change` | The write half reached directly |
| `peek_is_implemented_on_the_slot_so_the_reader_needs_no_payload_type` | The read half reached directly |
| `both_shapes_land_in_storage_through_the_same_two_calls` | One body over both shipped shapes |
