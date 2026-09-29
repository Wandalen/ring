# Algorithm: The Dispatch Happens Before the Program Runs

### Scope

**Purpose:** Record where the choice between the two slot shapes is actually
made, and establish that it cannot be deferred to runtime even deliberately.

**Responsibility:** Both traits' dyn-compatibility, the two different reasons
they refuse it, and the diagnostic a caller who tries gets back.

**In Scope:** `ring_event/src/lib.rs:53-63`, `:66`, `:103-124`; probes
below.

**Out of Scope:** What the bodies do once dispatched is
[`algorithm/001`](001_two_executable_statements_and_one_branch.md). The
associated type's shape is
[`type/001`](../type/001_an_associated_type_with_a_lifetime.md).

---

## Nothing in the Crate Can Be Erased

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- dyn, Box, unsafe and inline anywhere in the crate --'
for w in 'dyn ' 'Box<' '#\[ inline' 'unsafe'; do printf '%-12s %s\n' "$w" "$( command grep -c "$w" ring_event/src/lib.rs || true )"; done
echo '  -- every public item, and its generic parameters --'
command grep 'pub trait\|pub fn\|^impl' ring_event/src/lib.rs
echo '  -- the two declarations that decide dyn-compatibility --'
awk '/^  fn fill\( self, slot : &mut S \) -> Result< \(\), RingError >;$/{ print } /^  \/\/\/ What a reader is handed when the slot holds something\.$/{ n2 = NR } n2 && NR == n2 + 1 { print }' ring_event/src/lib.rs
```

Live output:

```
  -- dyn, Box, unsafe and inline anywhere in the crate --
dyn          0
Box<         0
#\[ inline   0
unsafe       0
  -- every public item, and its generic parameters --
pub trait Fill< S >
impl< T > Fill< TypedSlot< T > > for T
impl< const N : usize > Fill< BytesSlot< N > > for &[ u8 ]
pub trait Peek
impl< T > Peek for TypedSlot< T >
impl< const N : usize > Peek for BytesSlot< N >
pub fn publish_into< S, P >( slot : &mut S, payload : P ) -> Result< (), RingError >
pub fn drain_from< S >( slot : &S ) -> Option< S::Out< '_ > >
pub fn recycle< S >( slot : &mut S )
  -- the two declarations that decide dyn-compatibility --
  fn fill( self, slot : &mut S ) -> Result< (), RingError >;
  type Out< 'a > where Self : 'a;
```

---

## What the Compiler Says to Four Attempts at Erasure

*The four diagnostics below are frozen at authoring time: the probe binaries
under `-ev_probe/src/bin/` were swept per this project's convention for
temporary files, and rustc's exact error wording, span formatting and E-code
text are not a stability guarantee across compiler versions. The declarations
they exercise — `fn fill( self, slot : &mut S )` and
`type Out< 'a > where Self : 'a` at `lib.rs:63`, `:106` — are unchanged from
what's quoted here, so the properties demonstrated (`Peek` failing
dyn-compatibility on its GAT, `Fill`'s blanket impl misdirecting the
diagnostic through the wrong resolution) should still hold; only the literal
compiler text is not re-verified.*

Four probe binaries against the real crate. Two ask for a `Peek` trait object;
two ask to call `Fill` through one.

```rust
// -ev_probe/src/bin/dyn_peek.rs — the read half, erased
let erased : &dyn Peek< Out = &u32 > = &slot;

// -ev_probe/src/bin/dyn_fill.rs — the write half, named and constructed
let erased : &dyn Fill< TypedSlot< u32 > > = &payload;

// -ev_probe/src/bin/dyn_method_syntax.rs — and called, through method syntax
erased.fill( &mut slot ).unwrap();

// -ev_probe/src/bin/dyn_call.rs — and called, fully qualified
< dyn Fill< TypedSlot< u32 > > as Fill< TypedSlot< u32 > > >::fill( *erased, &mut slot )
```

```
  -- Peek, erased --
error[E0038]: the trait `Peek` is not dyn compatible
  --> src/bin/dyn_peek.rs:8:21
   |
 8 |   let erased : &dyn Peek< Out = &u32 > = &slot;
   |                     ^^^^^^^^^^^^^^^^^^ `Peek` is not dyn compatible
  -- Fill, erased and constructed --
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.01s
  -- Fill, called through method syntax --
error[E0308]: mismatched types
  --> src/bin/dyn_method_syntax.rs:10:16
   |
10 |   erased.fill( &mut slot ).unwrap();
   |          ----  ^^^^^^^^^ expected `&mut TypedSlot<&dyn Fill<...>>`, found `&mut TypedSlot<u32>`
   |          |
   |          arguments to this method are incorrect
  -- Fill, called fully qualified --
error[E0277]: the size for values of type `dyn Fill<TypedSlot<u32>>` cannot be known at compilation time
  --> src/bin/dyn_call.rs:11:71
   |
11 |   < dyn Fill< TypedSlot< u32 > > as Fill< TypedSlot< u32 > > >::fill( *erased, &mut slot ).unwrap();
   |                                                                       ^^^^^^^ doesn't have a size known at compile-time
```

---

### EV3 — Both Halves Refuse Dynamic Dispatch, for Two Different Reasons, and Neither Says So

Every public item in the crate is generic, and the words `dyn`, `Box`, `unsafe`
and `#[ inline ]` appear zero times. So the choice between `TypedSlot< T >` and
`BytesSlot< N >` is made at monomorphisation and there is no runtime component to
it — the module documentation's "neither function can tell the two slot shapes
apart" (`:21-22`) is true in the strong sense that after codegen there is no
single function left that could.

That much is design. What the probes add is that the property is not merely
unused but unavailable, and the two halves are unavailable differently.

`Peek` fails immediately: `E0038`, not dyn compatible, because `Out< 'a >` is a
generic associated type and a vtable cannot carry one. The error names the trait
and points at the declaration.

`Fill` does not fail there. `&dyn Fill< TypedSlot< u32 > >` names a legal type and
constructs from a `&u32` without complaint — the build finishes. It fails only at
the call, and only when the call is written in a form that actually reaches the
trait object: `E0277`, `dyn Fill< TypedSlot< u32 > >` is not `Sized`, because
`fill` takes `self` by value and a value cannot be moved out of a trait object.

**Finding.** Neither trait documents this. `Fill`'s doc comment explains why it is
implemented on the payload; `Peek`'s explains why `Out` carries a lifetime.
Neither says that the resulting trait cannot be erased, which is the property a
consumer would hit first if they tried to hold a heterogeneous collection of slot
shapes — a reasonable thing to want from a crate whose stated purpose is that
both shapes take one path.

---

### EV4 — The Blanket Impl Makes Every Type a `Fill`, so the Trait-Object Diagnostic Points at the Wrong Thing

`impl< T > Fill< TypedSlot< T > > for T` at `:66` has no bounds on `T`. Every type
in existence therefore implements `Fill< TypedSlot< Self > >` — including `u32`,
including `&[ u8 ]`, and including `&dyn Fill< TypedSlot< u32 > >`.

That last one is what produces the middle probe's diagnostic. Writing
`erased.fill( &mut slot )` where `erased : &dyn Fill< TypedSlot< u32 > >` does not
attempt dynamic dispatch at all: method resolution finds the blanket impl on the
reference type first, and the call is checked as
`< &dyn Fill< … > as Fill< TypedSlot< &dyn Fill< … > > > >::fill`. The reported
error is `E0308`, a type mismatch, and the type it names is
`&mut TypedSlot< &dyn Fill< … > >` — a type nobody wrote and nobody wants.

**Finding.** A caller who reaches for erasure gets a diagnostic that describes a
different problem than the one they have. The real obstruction is the by-value
`self` (`E0277`, visible only from the fully-qualified form); what they are shown
is a nested slot type generated by an impl they did not know applies to them.

The blanket impl is deliberate and its benefit is real — it is why a typed payload
needs no per-type implementation, and `fill_is_implemented_on_the_payload_so_a_new_payload_needs_no_slot_change`
is the test that pins the benefit. This is the cost side of the same decision, and
it is unrecorded: nothing in the crate notes that `Fill` is universally
implemented, which is the fact that turns one confusing error into another.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`algorithm/001`](001_two_executable_statements_and_one_branch.md) | What the dispatched bodies do |
| [`type/001`](../type/001_an_associated_type_with_a_lifetime.md) | The associated type that costs `Peek` its dyn-compatibility |
| [`type/002`](../type/002_a_blanket_impl_over_every_type_there_is.md) | The unbounded blanket impl, and what else it reaches |
| [`decisions/001`](../decisions/001_the_write_half_on_the_payload_the_read_half_on_the_slot.md) | Why `Fill` is on the payload at all |

### Sources

| Fact | Where |
|------|-------|
| Zero `dyn`, `Box`, `unsafe`, `#[ inline ]` | Census above |
| Every public item generic | Census above |
| `fill` taking `self` by value | `ring_event/src/lib.rs:63` |
| `Out< 'a >` as a generic associated type | `ring_event/src/lib.rs:106` |
| The unbounded blanket impl | `ring_event/src/lib.rs:66` |
| `Peek` not dyn compatible; `Fill` failing only at the call | Probes above |

### Tests

| Test | Covers |
|------|--------|
| `fill_is_implemented_on_the_payload_so_a_new_payload_needs_no_slot_change` | The blanket impl's intended benefit |
| `peek_is_implemented_on_the_slot_so_the_reader_needs_no_payload_type` | The read half's static dispatch |
| `both_shapes_land_in_storage_through_the_same_two_calls` | One body monomorphised twice |
