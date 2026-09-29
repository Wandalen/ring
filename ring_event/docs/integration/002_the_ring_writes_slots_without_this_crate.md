# Integration: The Ring Writes Slots Without This Crate

### Scope

**Purpose:** Record what the assembled ring actually calls to write and read a
slot, and establish that the read half of the "one path" could not serve it even
if the edge were declared.

**Responsibility:** `ring_core`'s manifest, its write call, its four read calls,
the operations this crate offers against the one `ring_core` needs, and where the
family's genericity over slot shape is lost.

**In Scope:** `ring_core/Cargo.toml`; `ring_core/src/lib.rs:389`,
`:558`, `:563`, `:598`, `:605`; `ring_event/src/lib.rs:7-9`, `:124`,
`:207`.

**Out of Scope:** The dependency census is
[`integration/001`](001_one_declarer_and_it_is_a_dev_dependency.md). What
`recycle` does differently per shape is
[`data_structure/002`](../data_structure/002_what_clear_leaves_behind.md).

---

## What `ring_core` Calls Instead

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- what ring_core declares, and what it does not --'
command grep -n 'ring_' ring_core/Cargo.toml | command grep -v '^2:' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
echo '  -- how it writes a slot --'
command grep -m1 -A4 -F '          Ok( mut reserved ) =>' ring_core/src/lib.rs
echo '  -- and how it reads one --'
command grep -rn 'TypedSlot::take' --include=*.rs ring_core/src | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
echo '  -- the read operations this crate offers --'
command grep -n 'fn peek\|pub fn drain_from\|pub fn recycle' ring_event/src/lib.rs | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
echo '  -- are the rings themselves generic over the slot shape? --'
command grep -n 'pub struct Ring<' ring_spsc/src/lib.rs ring_mpsc/src/lib.rs | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
echo '  -- and how many times ring_core fixes them, against BytesSlot mentions --'
command grep -c 'TypedSlot< T >' ring_core/src/lib.rs || true
command grep -c 'BytesSlot' ring_core/src/lib.rs || true
```

Live output:

```
  -- what ring_core declares, and what it does not --
ring_config = { path = "../ring_config" }
ring_mpsc = { path = "../ring_mpsc" }
ring_overflow = { path = "../ring_overflow" }
ring_slot = { path = "../ring_slot" }
ring_spsc = { path = "../ring_spsc" }
ring_types = { path = "../ring_types" }
  -- how it writes a slot --
          Ok( mut reserved ) =>
          {
            let displaced = reserved.set( record );
            debug_assert!( displaced.is_none(), "a claimed slot held a record" );
            Ok( () )
  -- and how it reads one --
ring_core/src/lib.rs:        batch.get_mut( 0 ).and_then( TypedSlot::take )
ring_core/src/lib.rs:        batch.get_mut( 0 ).and_then( TypedSlot::take )
ring_core/src/lib.rs:        out.extend( ( 0..len ).filter_map( | offset | batch.get_mut( offset ).and_then( TypedSlot::take ) ) );
ring_core/src/lib.rs:        out.extend( ( 0..len ).filter_map( | offset | batch.get_mut( offset ).and_then( TypedSlot::take ) ) );
  -- the read operations this crate offers --
  fn peek( &self ) -> Option< Self::Out< '_ > >;
  fn peek( &self ) -> Option< &T >
  fn peek( &self ) -> Option< &[ u8 ] >
pub fn drain_from< S >( slot : &S ) -> Option< S::Out< '_ > >
pub fn recycle< S >( slot : &mut S )
  -- are the rings themselves generic over the slot shape? --
ring_spsc/src/lib.rs:pub struct Ring< S >
ring_mpsc/src/lib.rs:pub struct Ring< S >
  -- and how many times ring_core fixes them, against BytesSlot mentions --
8
0
```

## Composing the Missing Operation Out of the Two That Exist

*The `E0502` diagnostic below is frozen at authoring time: the probe binary
has since been swept, and rustc's exact borrow-checker wording and span
formatting are not a stability guarantee across compiler versions.
`drain_from`'s `&S` borrow and `recycle`'s `&mut S` parameter
(`ring_event/src/lib.rs:207`, `:229`) are unchanged, so the underlying
conflict — the immutable borrow `drain_from` returns is still live when
`recycle` asks for `&mut` — should still hold; only the literal compiler text
is not re-verified.*

A probe against the real crate: read the payload, then empty the slot, and keep
what was read — which is what a drain does.

```rust
// -ev_probe/src/bin/compose_a_take.rs
struct Owned( String );

let mut slot = TypedSlot::empty();
publish_into( &mut slot, Owned( "payload".into() ) ).unwrap();

let taken = drain_from( &slot ).unwrap();
recycle( &mut slot );
println!( "{}", taken.0 );
```

```
error[E0502]: cannot borrow `slot` as mutable because it is also borrowed as immutable
  --> src/bin/compose_a_take.rs:14:12
   |
13 |   let taken = drain_from( &slot ).unwrap();
   |                           ----- immutable borrow occurs here
14 |   recycle( &mut slot );
   |            ^^^^^^^^^ mutable borrow occurs here
15 |   println!( "{}", taken.0 );
   |                   ------- immutable borrow later used here
```

---

### EV19 — The Ring's Drain Cannot Use `drain_from`, Because This Crate Has No Way to Take a Value Out

The module documentation states the wiring as fact: `publish_into` and
`drain_from` "are then written once, generically, and the ring's publish and
drain call those."

`ring_core` is the crate that assembles a ring, and its manifest names
`ring_slot` and not this one. It writes with `reserved.set( record )` at `:389`,
which is `TypedSlot::set` reached directly. It reads with `TypedSlot::take`, four
times — `:558`, `:563`, `:598`, `:605`.

The write side is a wiring difference: `publish_into( &mut reserved, record )`
would do the same thing, and adopting it is a manifest line and a call-site edit.
The read side is not. `take` moves the payload out of the slot, which is what a
drain must do — the consumer is handed an owned `T`, and the slot has to be empty
afterwards for the next lap. Every read operation this crate offers borrows:
`peek` returns `Option< Self::Out< '_ > >`, `drain_from` returns
`Option< S::Out< '_ > >`, and both impls' `Out` is a reference. `recycle` empties
the slot but returns nothing, so the two together cannot be composed into a take
either: the probe above is that composition, and it does not compile — `E0502`,
the borrow from `drain_from` is still live when `recycle` asks for `&mut`. A
`Clone` bound would buy a copy rather than a move, which is the cost the family
exists to avoid.

**Finding.** The stated fact is false, and the read half of it is not false for
want of a manifest line. This crate has no operation that removes a payload from a
slot, and a ring's drain is exactly an operation that removes a payload from a
slot. Adopting `drain_from` is not a wiring change; it is a design change —
either a fourth function taking `&mut S` and returning an owned value, or a
`Peek::Out` that may own, which the module documentation at `:25-32` rules out
for the byte shape because owning there would mean copying.

That leaves a real question nothing records: whether the one path was ever meant
to cover the read side at all, or whether `Peek` is a peek in the strict sense —
for an observer, a debugger, a `ring_trace` — and the consumer's take was always
going to be `ring_slot`'s.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -B2 -A6 -F 'not yet the one production rings execute' ring_event/src/lib.rs
```

Live output:

```
//! the crate that assembles a ring, calls neither today** — it reaches
//! `TypedSlot::set`/`TypedSlot::take` directly, so this crate's one path is
//! declared but not yet the one production rings execute. Neither function
//! can tell the two slot shapes apart, which is exactly the property feature
//! 182 is asking for, once something calls them.
//!
//! ## Why the read half is a GAT
//!
//! The two shapes genuinely return different things: a `TypedSlot<T>` hands
```

**Disposition:** applied — the module documentation in
`ring_event/src/lib.rs` no longer states as fact that "the ring's
publish and drain call those"; it now states the weaker, accurate claim this
instance establishes — the generic path exists and *could* be called through
a single body — and names the current reality directly: `ring_core` calls
neither `publish_into` nor `drain_from` today, reaching `TypedSlot::set` and
`TypedSlot::take` instead. The read-side design question this instance leaves
open (its own closing paragraph) stays open, since no single resolution is
named. Now prints: `not yet the one production rings execute`

---

### EV20 — The Identical-Path Property Is Unreachable Because `ring_core` Never Admits the Second Shape

This crate's own module documentation asks that "`TypedSlot<T>` and `BytesSlot` both round-trip through the
**identical** claim/publish/drain path", and this crate exists to supply the
identical path.

The rings can carry either. `ring_spsc::Ring< S >` and `ring_mpsc::Ring< S >` are
both generic in the slot shape, and `ring_store::Buffer< S >` is too. The
narrowing happens in exactly one place: `ring_core` writes `TypedSlot< T >` eight
times — in its ring enum, its ends, its producer and its consumer — and mentions
`BytesSlot` zero times. Every ring a user assembles through `ring_core` is a
typed-slot ring, and there is no parameter, feature or constructor that changes
that.

**Finding.** So the feature is unrealized twice over, and the two failures are
independent. Even if `ring_core` adopted `publish_into` and `drain_from`
tomorrow, it would exercise them at exactly one of the two shapes, and "both
round-trip through the identical path" would still be untrue — proven only for
the half that already worked.

This is worth recording precisely because it is invisible from here. This crate's
suite genuinely runs both shapes through one generic body, which is the strongest
evidence available *inside* the crate and reads like the feature is satisfied.
The gap is one crate away, in a monomorphisation `ring_core` never explains and
that nothing in the family flags as temporary.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`integration/001`](001_one_declarer_and_it_is_a_dev_dependency.md) | The manifest census this builds on |
| [`api/002`](../api/002_a_result_one_impl_can_never_return.md) | The displaced value `ring_core:389` captures and this crate drops |
| [`data_structure/002`](../data_structure/002_what_clear_leaves_behind.md) | Why the second shape's absence keeps another hazard latent |
| [`decisions/002`](../decisions/002_three_free_functions_instead_of_methods.md) | The sentence naming a caller that does not call |

### Sources

| Fact | Where |
|------|-------|
| "a ring's publish and drain *could* call those" | `ring_event/src/lib.rs:17-18` |
| `ring_core` declaring `ring_slot` and not this crate | `ring_core/Cargo.toml:15-20` |
| The write, reached directly | `ring_core/src/lib.rs:389` |
| Four reads through `TypedSlot::take` | `ring_core/src/lib.rs:557`, `:562`, `:598`, `:605` |
| Every read operation here returning a borrow | `ring_event/src/lib.rs:124`, `:207` |
| `drain_from` and `recycle` not composing into a take | Probe above |
| The rings being generic in the slot shape | `ring_spsc/src/lib.rs:239`, `ring_mpsc/src/lib.rs:308` |
| `ring_core` fixing `TypedSlot< T >` eight times, `BytesSlot` zero | Census above |
| This crate's "identical path" requirement | `ring_event/src/lib.rs:7-9` |

### Tests

| Test | Covers |
|------|--------|
| `both_shapes_land_in_storage_through_the_same_two_calls` | The property, proven at the level below `ring_core` |
| `a_recycled_storage_slot_stops_returning_the_previous_lap` | A ring-level property asserted through `ring_store` instead |
| `a_typed_payload_survives_storage_byte_identically` | The one shape `ring_core` would actually exercise |
