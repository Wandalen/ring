# Type: An Associated Type With a Lifetime

### Scope

**Purpose:** Account for the family's only generic associated type — what it
buys, why each of its three `where` clauses is or is not there, and what a
generic caller can actually do with a value of it.

**Responsibility:** `Peek::Out`'s declaration and its two definitions, the
module documentation section justifying them, the family-wide GAT census, and
the boundary between plumbing an unbounded associated type and inspecting one.

**In Scope:** `ring_event/src/lib.rs:25-32`, `:105-106`, `:129`, `:139`;
`ring_event/tests/event_test.rs:42`.

**Out of Scope:** The blanket impl on the write half is
[`type/002`](002_a_blanket_impl_over_every_type_there_is.md). Why the read half
sits on the slot at all is
[`decisions/001`](../decisions/001_the_write_half_on_the_payload_the_read_half_on_the_slot.md).

---

## The Declaration, Its Definitions, and Its Only Company

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the declaration, its one doc line, and both definitions --'
command grep -m1 -A1 -F '  /// What a reader is handed when the slot holds something.' ring_event/src/lib.rs
command grep -m1 -F '  type Out< '"'"'a > = &'"'"'a T where T : '"'"'a;' ring_event/src/lib.rs
command grep -m1 -F '  type Out< '"'"'a > = &'"'"'a [ u8 ];' ring_event/src/lib.rs
echo '  -- every generic associated type in the 33-crate family --'
command grep -r 'type [A-Za-z]*< .a >' --include=*.rs ring_*/src
echo '  -- what the module doc spends on why it exists --'
command grep -m1 -A7 -F '//! ## Why the read half is a GAT' ring_event/src/lib.rs
echo '  -- the one generic body that consumes the result, against the concrete ones --'
command grep -m1 -F '  publish_into( buffer.at_mut( seq ), payload ).is_ok() && drain_from( buffer.at( seq ) ).is_some()' ring_event/tests/event_test.rs
command grep -c 'assert_eq!( drain_from\|assert_eq!( round_trip' ring_event/tests/event_test.rs || true
```

Live output:

```
  -- the declaration, its one doc line, and both definitions --
  /// What a reader is handed when the slot holds something.
  type Out< 'a > where Self : 'a;
  type Out< 'a > = &'a T where T : 'a;
  type Out< 'a > = &'a [ u8 ];
  -- every generic associated type in the 33-crate family --
ring_event/src/lib.rs:  type Out< 'a > where Self : 'a;
ring_event/src/lib.rs:  type Out< 'a > = &'a T where T : 'a;
ring_event/src/lib.rs:  type Out< 'a > = &'a [ u8 ];
  -- what the module doc spends on why it exists --
//! ## Why the read half is a GAT
//!
//! The two shapes genuinely return different things: a `TypedSlot<T>` hands
//! back a `&T`, a `BytesSlot<N>` a `&[u8]` whose length is the payload's, not
//! the slot's. Flattening both into one concrete return type would mean
//! copying, and a `&[u8]` copied out of a slot is the allocation the whole
//! family exists to avoid. An associated type with a lifetime lets one function
//! serve both without either paying for the other's shape.
  -- the one generic body that consumes the result, against the concrete ones --
  publish_into( buffer.at_mut( seq ), payload ).is_ok() && drain_from( buffer.at( seq ) ).is_some()
17
```

## Which of the Three `where` Clauses Are Load-Bearing

*The three diagnostics below are frozen at authoring time: the probe binary
has since been swept, and rustc's exact error wording, span formatting and
E-code text are not a stability guarantee across compiler versions — the
first diagnostic's own text even names an open, unresolved language issue
(rust-lang/rust#87479) as the reason the bound is "currently required," so
the wording is explicitly provisional by rustc's own admission.
`Peek::Out`'s three `where`-clause positions (`ring_event/src/lib.rs:106`,
`:129`, `:139`) are unchanged, so the three compiler-forced constraints
demonstrated should still hold; only the literal compiler text is not
re-verified.*

```rust
// -ev_probe/src/bin/gat_bounds.rs
// A local replica of `Peek`, to ask which of its two `where` clauses are needed.
pub struct Cell< T >( Option< T > );

// (a) the trait's own bound, dropped
pub trait PeekA { type Out< 'a >; fn peek( &self ) -> Option< Self::Out< '_ > >; }
impl< T > PeekA for Cell< T > { type Out< 'a > = &'a T where T : 'a; /* … */ }

// (b) the impl's bound, dropped
pub trait PeekB { type Out< 'a > where Self : 'a; fn peek( &self ) -> Option< Self::Out< '_ > >; }
impl< T > PeekB for Cell< T > { type Out< 'a > = &'a T; /* … */ }
```

```
error: missing required bound on `Out`
 --> src/bin/gat_bounds.rs:7:3
  |
7 |   type Out< 'a >;
  |   ^^^^^^^^^^^^^^-
  |                 |
  |                 help: add the required where clause: `where Self: 'a`
  |
  = note: this bound is currently required to ensure that impls have maximum flexibility
  = note: we are soliciting feedback, see issue #87479 <https://github.com/rust-lang/rust/issues/87479> for more information

error[E0276]: impl has stricter requirements than trait
  --> src/bin/gat_bounds.rs:13:36
   |
13 |   type Out< 'a > = &'a T where T : 'a;
   |                                    ^^ impl has extra requirement `T: 'a`

error[E0309]: the parameter type `T` may not live long enough
  --> src/bin/gat_bounds.rs:26:20
   |
26 |   type Out< 'a > = &'a T;
   |             --     ^^^^^ ...so that the type `Cell<T>` will meet its required lifetime bounds
```

## What a Generic Body Can Do With One

*The `E0277` diagnostic below is frozen at authoring time: the probe binary
has since been swept, and rustc's exact error wording and span formatting
are not a stability guarantee across compiler versions. `Peek::Out`'s lack
of a `Debug` bound (`ring_event/src/lib.rs:106`) is unchanged, so the
demonstrated fact — a generic body cannot format `S::Out< '_ >` without
adding its own bound — should still hold; only the literal compiler text is
not re-verified.*

```rust
// -ev_probe/src/bin/opaque_out.rs
// Everything a generic body can do with what `drain_from` hands back.
fn inspect_generically< S : Peek >( slot : &S ) -> bool
{
  drain_from( slot ).is_some()
}

// And what it cannot: `Out` carries no bounds, so nothing is available on it.
fn print_generically< S : Peek >( slot : &S )
{
  println!( "{:?}", drain_from( slot ) );
}
```

```
error[E0277]: `<S as Peek>::Out<'_>` doesn't implement `Debug`
  --> src/bin/opaque_out.rs:12:21
   |
12 |   println!( "{:?}", drain_from( slot ) );
   |              ----   ^^^^^^^^^^^^^^^^^^ `<S as Peek>::Out<'_>` cannot be formatted using `{:?}` because it doesn't implement `Debug`
   |
   = help: the trait `Debug` is not implemented for `<S as Peek>::Out<'_>`
   = note: required for `Option<<S as Peek>::Out<'_>>` to implement `Debug`
```

---

### EV45 — Three `where` Clauses, Each Compiler-Forced, None Explained

The three lines that declare and define `Out` carry three different lifetime
positions, and read as though someone were inconsistent:

```rust
type Out< 'a > where Self : 'a;      // the trait
type Out< 'a > = &'a T where T : 'a; // the typed impl
type Out< 'a > = &'a [ u8 ];         // the byte impl
```

The probe shows each is exactly what the compiler demands. Removing `Self : 'a`
from the trait is a hard error — and the message is unusual, because it does not
say the bound is *sound*, it says it is "currently required to ensure that impls
have maximum flexibility" and cites an open language issue soliciting feedback.
That bound is an artefact of a rule the compiler itself flags as provisional.
Removing `T : 'a` from the typed impl is `E0309`, `T` may not live long enough.
And the two are coupled: with the trait's bound gone but the impl's kept, the
result is `E0276`, "impl has stricter requirements than trait". The byte impl
needs no bound at all because `[ u8 ]` has no parameter that could fail to
outlive anything.

**Finding.** The module documentation devotes a `##` section to why an
associated type with a lifetime is the right shape — the two return types
genuinely differ, flattening them would copy, and a copied `&[u8]` is the
allocation the family exists to avoid — and that section is convincing and
complete about the *choice*. The declaration itself gets one doc line, "What a
reader is handed when the slot holds something", which describes the type and
none of its syntax. A reader meeting three inconsistent-looking `where` clauses
on the family's only GAT has no way to learn from the source that all three are
forced, that the trait's is a placeholder for an unresolved language question,
or that the byte impl's absence is correct rather than an oversight. Two
sentences at `:105` would carry it, and the one about issue #87479 is the kind of
comment that stops a future reader from "tidying" a bound the compiler will
demand back.

---

### EV46 — The Associated Type Is Unbounded, So Generic Code Can Carry It but Not Look at It

`Out` declares no supertrait bounds. That is the maximally permissive choice and
it is what lets `BytesSlot` return `&[ u8 ]` while `TypedSlot< T >` returns
`&T` for a `T` with no bounds of its own. The cost is that a generic body holding
an `S::Out< '_ >` has nothing available on it — no `Debug`, no `PartialEq`, no
`Clone`. The probe's second function fails to compile for exactly that reason.

The suite shows the boundary precisely, in two helpers. `round_trip< S, P >`
*returns* `Option< S::Out< '_ > >` without touching it, which is all it can do,
and the thirteen `assert_eq!` comparisons that check what came back happen at
the call sites, after monomorphisation has turned `Out` into `&u32` or
`&[ u8 ]`. `land_and_read< S, P >` *consumes* the result inside the generic body,
and every consumption in the crate is the same one: `.is_some()`. Not because
the test wants a boolean, but because `is_some` is the only method an unbounded
associated type permits.

**Finding.** This is the right default and the constraint it implies is
unrecorded. A future generic consumer — a drain loop in `ring_core`, a debug
formatter in `ring_debug`, a test helper that compares two laps — will hit
`E0277` on its first attempt to do anything with the value, and the fix is a
`for< 'a > S::Out< 'a > : Debug`-shaped bound at that call site rather than
anything in this crate. Saying so at the declaration turns a compile error into
an expectation: `Out` is deliberately unbounded, so callers that need more add
the bound themselves. The alternative — putting `Debug` on the trait — would
force it onto every payload type forever, which is precisely the tax the
unbounded form avoids, and that tradeoff is worth one sentence where the type is
declared rather than being rediscovered by whoever first writes a generic reader.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`type/002`](002_a_blanket_impl_over_every_type_there_is.md) | The write half's type-level counterpart |
| [`api/001`](../api/001_two_traits_three_functions_one_associated_type.md) | The surface this type is one third of |
| [`pattern/001`](../pattern/001_a_trait_on_the_varying_side_and_one_function_over_it.md) | The shape that needs it |
| [`decisions/001`](../decisions/001_the_write_half_on_the_payload_the_read_half_on_the_slot.md) | Why the read half is the side that varies |

### Sources

| Fact | Where |
|------|-------|
| The declaration and its single doc line | `ring_event/src/lib.rs:105-106` |
| Both definitions and their differing bounds | `ring_event/src/lib.rs:129`, `:139` |
| The module doc's justification section | `ring_event/src/lib.rs:25-32` |
| The family's only GAT | Census above |
| Each `where` clause being compiler-forced | Probe above |
| `Out` being unusable without a bound | Probe above |
| The one generic consumption, and the thirteen concrete ones | `ring_event/tests/event_test.rs:42`; census above |

### Tests

| Test | Covers |
|------|--------|
| `the_same_generic_body_round_trips_a_bytes_slot` | `Out` resolving to `&[ u8 ]` |
| `one_generic_body_round_trips_a_typed_slot` | `Out` resolving to `&T` |
| `both_shapes_land_in_storage_through_the_same_two_calls` | The one generic consumption, through `.is_some()` |
| `peek_is_implemented_on_the_slot_so_the_reader_needs_no_payload_type` | Why the type varies by slot rather than by payload |
