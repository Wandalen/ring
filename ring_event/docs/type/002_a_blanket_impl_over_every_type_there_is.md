# Type: A Blanket Impl Over Every Type There Is

### Scope

**Purpose:** Establish what `impl< T > Fill< TypedSlot< T > > for T` actually
ranges over — an unbounded `T`, so every type in the language — and what follows
at the type level from `Fill` and `Slot` therefore overlapping.

**Responsibility:** The two `Fill` impls and their bounds, the types each
admits, the consequences of a slot being a legal payload, and what the crate's
one lint can and cannot see.

**In Scope:** `ring_event/src/lib.rs:34`, `:40-43`, `:66-87`, `:173-178`.

**Out of Scope:** Coherence, the orphan rule and downstream extension are
[`item/002`](../item/002_four_impls_and_what_the_blanket_one_does_not_claim.md).
The read half's type-level story is
[`type/001`](001_an_associated_type_with_a_lifetime.md).

---

## Both Impls, Their Bounds, and What Documents Them

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the trait doc that describes the write half extension story --'
command grep -m1 -A4 -F '/// A payload that knows how to enter a slot of shape `S`.' ring_event/src/lib.rs | tail -n 4
echo '  -- and both Fill impls, in full, with every doc line they carry --'
command grep -m1 -A16 -F 'impl< T > Fill< TypedSlot< T > > for T' ring_event/src/lib.rs
echo '  -- the crate lint, and every impl it does not reach --'
command grep -m1 -F '#![ deny( missing_docs ) ]' ring_event/src/lib.rs
awk '/^impl/{ printf "  line %-4d doc above: %-4s %s\n", NR, ( p ~ /^ *\/\/\// ? "yes" : "no" ), $0 } { p = $0 }' ring_event/src/lib.rs
echo '  -- the one bound on the write half, at its only call site --'
command grep -m1 -A5 -F 'pub fn publish_into< S, P >( slot : &mut S, payload : P ) -> Result< (), RingError >' ring_event/src/lib.rs
```

Live output:

```
  -- the trait doc that describes the write half extension story --
///
/// Implemented on the *payload*, not the slot, so adding a payload kind never
/// touches the slot types — and so `publish_into` needs no match, no downcast
/// and no enum of shapes.
  -- and both Fill impls, in full, with every doc line they carry --
impl< T > Fill< TypedSlot< T > > for T
{
  fn fill( self, slot : &mut TypedSlot< T > ) -> Result< (), RingError >
  {
    // Discarding the displaced value is `fill`'s documented contract — "write
    // `self` into `slot`, replacing whatever it held" — not an oversight. A
    // caller that needs the old record calls `TypedSlot::set` directly and
    // binds it; this trait exists to give both slot shapes one signature, and
    // `BytesSlot` has nothing to hand back.
    slot.set( self );
    Ok( () )
  }
}

impl< const N : usize > Fill< BytesSlot< N > > for &[ u8 ]
{
  fn fill( self, slot : &mut BytesSlot< N > ) -> Result< (), RingError >
  -- the crate lint, and every impl it does not reach --
#![ deny( missing_docs ) ]
  line 66   doc above: no   impl< T > Fill< TypedSlot< T > > for T
  line 80   doc above: no   impl< const N : usize > Fill< BytesSlot< N > > for &[ u8 ]
  line 127  doc above: no   impl< T > Peek for TypedSlot< T >
  line 137  doc above: no   impl< const N : usize > Peek for BytesSlot< N >
  -- the one bound on the write half, at its only call site --
pub fn publish_into< S, P >( slot : &mut S, payload : P ) -> Result< (), RingError >
where
  P : Fill< S >,
{
  payload.fill( slot )
}
```

## What an Unbounded `T` Admits

*The `a slot` row below predates `BytesSlot`'s `Debug` rewrite (see EV12's
Disposition in
[`data_structure/002`](../data_structure/002_what_clear_leaves_behind.md)):
today's `Debug` prints only `BytesSlot { payload: [] }`, not the raw
`bytes`/`len` fields shown here. The other rows in this probe print
`Option< () >`, `Option< RingError >` and plain byte slices, none of which
depend on `BytesSlot`'s `Debug`, so only that one line is affected.*

```rust
// -ev_probe/src/bin/blanket_reach.rs
// The blanket impl has no bounds, so this compiles for absolutely any `T`.
fn accepts_anything< T >( value : T ) -> Option< T >
{
  let mut slot = TypedSlot::< T >::empty();
  publish_into( &mut slot, value ).unwrap();
  slot.take()
}

accepts_anything( () );
accepts_anything( RingError::BatchTooLarge { requested : 9, capacity : 4 } );
accepts_anything( BytesSlot::< 2 >::empty() );

// `&[ u8 ]` implements `Fill` twice, at two different slot types.
publish_into( &mut b, &b"ab"[ .. ] ).unwrap();  // b : BytesSlot< 4 >
publish_into( &mut t, &b"ab"[ .. ] ).unwrap();  // t : TypedSlot< &[ u8 ] >

// So `Fill` and `Slot` are not disjoint: a slot nests inside a slot.
let mut outer = TypedSlot::< TypedSlot< u32 > >::empty();
publish_into( &mut outer, inner ).unwrap();
let once = drain_from( &outer ).expect( "outer holds something" );
recycle( &mut outer );

// A refusal cannot cross a level: the inner slot's state is irrelevant.
Fill::fill( full, &mut wrapper )  // full : BytesSlot< 2 >, already occupied
```

```
    unit            Some(())
    error type      Some(BatchTooLarge { requested: 9, capacity: 4 })
    a slot          Some(BytesSlot { bytes: [0, 0], len: 0 })
    same payload    into BytesSlot Some([97, 98])  into TypedSlot Some([97, 98])
    printed alike   but typed core::option::Option<&[u8]>  and core::option::Option<&&[u8]>
    one drain gives TypedSlot(Some(7))  and a second gives Some(7)
    after recycle   outer empty true  drain None
    wrapping a full slot: Ok(())
```

---

### EV47 — The Widest Declaration in the Crate Carries No Documentation, and the Lint Cannot Ask for Any

`impl< T > Fill< TypedSlot< T > > for T` has no bounds. Not `T : Sized` beyond
the implicit one, not `T : Debug`, nothing — so the set of types implementing
`Fill` is the set of types. The probe lands `()`, the crate's own
`RingError`, and a `BytesSlot< 2 >` through the same generic function without a
single annotation, because there is no `T` for which the impl does not apply.

Two consequences follow immediately at the type level. `&[ u8 ]` implements
`Fill` **twice** — once by the blanket impl at `TypedSlot< &[ u8 ] >`, once by
the explicit impl at `BytesSlot< N >` — which is legal because the `S` parameter
differs, and which the probe shows producing results that print identically
while typed `Option< &[u8] >` and `Option< &&[u8] >`. And `Fill` and `Slot` are
not disjoint sets: every slot is a payload, so `TypedSlot< TypedSlot< u32 > >`
is a real type that `publish_into` fills.

Meanwhile `#![ deny( missing_docs ) ]` at `:34` is the crate's only lint, and
the census shows all four impl blocks carrying no doc line above them — because
`missing_docs` does not apply to impl blocks. The one mechanism in the crate that
enforces documentation structurally cannot see the four declarations that decide
what the traits mean.

**Finding.** The trait's own doc explains the *policy* — "Implemented on the
payload, not the slot, so adding a payload kind never touches the slot types" —
which is the right thing to say about `Fill`. It says nothing about the *reach*
of the implementation that policy is delivered by, and the reach is total.
Whether that is intended is a real question: a bound like `T : Send` or
`T : 'static` would be a normal thing to want on a payload crossing a ring, and
the current declaration rules it out for every future caller by having already
claimed every type. One line above `:66` recording that the impl is deliberately
unbounded, and that narrowing it later would be a breaking change for every
downstream payload, is the note the lint cannot demand.

---

### EV48 — The Three Operations Are Per-Level and the Structure They Admit Is Not

Because a slot is a payload, the type system permits nesting to any depth, and
the probe walks one level of it. What comes back is worth reading carefully.

`drain_from( &outer )` returns `&TypedSlot< u32 >` — the inner slot, not its
contents — so reading a nested value takes one call per level: `one drain gives
TypedSlot(Some(7)) and a second gives Some(7)`. The crate's stated property is
that there is exactly one read path, and that remains true; what the nesting
shows is that "one path" means one path *per level*, and nothing says how many
levels there are.

`recycle( &mut outer )` reports the outer slot empty and drains to `None`,
having dropped the inner slot whole rather than recycling it. For `TypedSlot`
that is correct and total — the `Option` goes to `None` and the inner value's
`Drop` runs. It is worth stating anyway, because the same reasoning applied to
`TypedSlot< BytesSlot< N > >` is the only reason the residue documented in
[`pitfall/002`](../pitfall/002_a_recycled_slot_is_not_equal_to_an_empty_one.md)
does not compound: the inner slot is discarded rather than cleared, so its stale
array goes with it.

The last line is the sharpest. `Fill::fill` on a `BytesSlot< 2 >` that is
already occupied, into a `TypedSlot< BytesSlot< 2 > >`, returns `Ok( () )`.
The inner slot's state cannot make the outer publish fail, because the outer
publish is `slot.set( self )` and `set` cannot fail. A refusal does not
propagate across a level; it cannot, since only the byte impl can refuse and the
byte impl is never the outer one.

**Finding.** None of this is broken and none of it is written down. The three
functions are documented as the operations a ring performs on *a* slot, and the
type system quietly admits a tree of them where each function acts on exactly one
node. A caller who nests deliberately gets sensible behaviour; a caller who nests
by accident — passing a slot where its contents were meant — gets a type error at
the outer level, which is the good outcome and is not obvious in advance. One
sentence on `Fill` saying that a slot is itself a legal payload, so nesting
type-checks and each of the three operations acts on one level, converts a
discovery into a documented property and costs nothing to keep true.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`type/001`](001_an_associated_type_with_a_lifetime.md) | The read half's type-level counterpart |
| [`item/002`](../item/002_four_impls_and_what_the_blanket_one_does_not_claim.md) | What the same impl does *not* claim, and why |
| [`api/002`](../api/002_a_result_one_impl_can_never_return.md) | The refusal that cannot cross a level |
| [`pitfall/002`](../pitfall/002_a_recycled_slot_is_not_equal_to_an_empty_one.md) | The residue nesting happens not to compound |

### Sources

| Fact | Where |
|------|-------|
| The blanket impl and its absent bounds | `ring_event/src/lib.rs:66-78` |
| The explicit byte impl beside it | `ring_event/src/lib.rs:80-87` |
| The trait doc stating the extension policy | `ring_event/src/lib.rs:40-43` |
| The lint, and the four impls it does not reach | `ring_event/src/lib.rs:34`; census above |
| `()`, `RingError` and a slot all landing as payloads | Probe above |
| `&[ u8 ]` implementing `Fill` twice | Probe above |
| One drain per level, and a whole-inner discard | Probe above |
| A refusal not crossing a level | Probe above |

### Tests

| Test | Covers |
|------|--------|
| `fill_is_implemented_on_the_payload_so_a_new_payload_needs_no_slot_change` | The extension point the blanket impl provides |
| `a_typed_publish_cannot_fail` | Why the outer level of a nest never refuses |
| `recycling_empties_either_shape_through_the_same_call` | The reset that discards an inner slot whole |
