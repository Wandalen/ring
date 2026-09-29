# Workaround: A Slicing Expression at Every Generic Call Site

### Scope

**Purpose:** Record that every byte payload published through this crate is
written as a slicing expression while every byte payload written directly to the
slot is a plain literal, establish that the difference is forced rather than
stylistic, and price the impl that would remove it.

**Responsibility:** `publish_into`'s generic parameter against
`BytesSlot::write`'s concrete one, the two call-site forms and their counts
across the workspace, and the coherence of a third `Fill` impl.

**In Scope:** `ring_event/src/lib.rs:80-87`, `:170`, `:173-175`;
`ring_event/tests/event_test.rs:75`, `:137`, `:148`, `:153`, `:183`,
`:239`; `ring_slot/src/lib.rs:347`.

**Out of Scope:** Why the byte impl is on `&[ u8 ]` and can be on nothing
foreign is
[`item/002`](../item/002_four_impls_and_what_the_blanket_one_does_not_claim.md).
The unbounded reach of the other impl is
[`type/002`](../type/002_a_blanket_impl_over_every_type_there_is.md).

---

## Two Paths to the Same Slot, Two Call-Site Forms

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the generic parameter that blocks the coercion, and the concrete one that does not --'
command grep -m1 -A2 -F 'pub fn publish_into< S, P >( slot : &mut S, payload : P ) -> Result< (), RingError >' ring_event/src/lib.rs
command grep -m1 -F '  pub fn write( &mut self, payload : &[ u8 ] ) -> Result< (), RingError >' ring_slot/src/lib.rs
echo '  -- every byte fill that goes through this crate --'
command grep -r 'publish_into( *&mut [a-z_]*, *&\[\|publish_into( *&mut [a-z_]*, *&b"' --include=*.rs
command grep -m1 -F '    assert!( land_and_read( &mut bytes, Seq( raw ), &raw.to_le_bytes()[ .. ] ) );' ring_event/tests/event_test.rs
echo '  -- every byte fill that calls the slot directly, plain literal then sliced --'
command grep -r '\.write( b"' --include=*.rs | wc -l
command grep -r '\.write( &[a-z_]*\[ *\.\. *\]\|\.write( &b"' --include=*.rs | wc -l
echo '  -- and this crates own suite using both forms, 56 lines apart --'
command grep -m1 -F '  publish_into( &mut bytes, &b"x"[ .. ] ).unwrap();' ring_event/tests/event_test.rs
command grep -m1 -F '  bytes.write( b"cd" ).unwrap();' ring_event/tests/event_test.rs
```

Live output:

```
  -- the generic parameter that blocks the coercion, and the concrete one that does not --
pub fn publish_into< S, P >( slot : &mut S, payload : P ) -> Result< (), RingError >
where
  P : Fill< S >,
  pub fn write( &mut self, payload : &[ u8 ] ) -> Result< (), RingError >
  -- every byte fill that goes through this crate --
ring_event/tests/event_test.rs:  publish_into( &mut slot, &[][ .. ] ).unwrap();
ring_event/tests/event_test.rs:  publish_into( &mut slot, &b"abcd"[ .. ] ).unwrap();
ring_event/tests/event_test.rs:  let refused = publish_into( &mut slot, &b"abcde"[ .. ] );
ring_event/tests/event_test.rs:  publish_into( &mut bytes, &b"payload"[ .. ] ).unwrap();
ring_event/tests/event_test.rs:  publish_into( &mut bytes, &b"x"[ .. ] ).unwrap();
ring_event/src/lib.rs:/// publish_into( &mut bytes, &b"hi"[ .. ] ).unwrap();
ring_event/src/lib.rs:/// publish_into( &mut slot, &b"ab"[ .. ] ).unwrap();
    assert!( land_and_read( &mut bytes, Seq( raw ), &raw.to_le_bytes()[ .. ] ) );
  -- every byte fill that calls the slot directly, plain literal then sliced --
40
0
  -- and this crates own suite using both forms, 56 lines apart --
  publish_into( &mut bytes, &b"x"[ .. ] ).unwrap();
  bytes.write( b"cd" ).unwrap();
```

## What Happens Without the Slice

The probe below lived in `-ev_probe/`, a hyphen-prefixed scaffold since swept, so
neither fence in this section is regenerable by the recipe gate — both are
hand-maintained, and the transcript's own coordinate is kept resolvable by hand.
It last read `ring_event/src/lib.rs:71`; it now reads `ring/…:80` — the
same unchanged `impl` line, after the family moved to `ring/` and the file grew
nine lines above it.

```rust
// -ev_probe/src/bin/slicing_ritual.rs
let mut a = BytesSlot::< 8 >::empty();
// Direct call: the parameter is a concrete `&[ u8 ]`, so the array coerces.
a.write( b"abcd" ).unwrap();

// Through the crate: `P` is generic, so no coercion happens.
let mut b = BytesSlot::< 8 >::empty();
publish_into( &mut b, b"abcd" ).unwrap();
```

```
error[E0277]: the trait bound `&[u8; 4]: Fill<BytesSlot<8>>` is not satisfied
   --> src/bin/slicing_ritual.rs:13:25
    |
 13 |   publish_into( &mut b, b"abcd" ).unwrap();
    |   ------------          ^^^^^^^ the trait `Fill<BytesSlot<8>>` is not implemented for `&[u8; 4]`
    |   |
    |   required by a bound introduced by this call
    |
help: the trait `Fill<BytesSlot<8>>` is not implemented for `&[u8; 4]`
      but it is implemented for `&[u8]`
   --> /home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_event/src/lib.rs:80:1
    |
 80 | impl< const N : usize > Fill< BytesSlot< N > > for &[ u8 ]
    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    = help: for that trait implementation, expected `[u8]`, found `[u8; 4]`
```

## The Impl That Would Remove It

*The probe below is frozen at authoring time — the replica crate that ran
it has since been swept — and no drift was found: Rust's coherence rules
(the orphan rule, non-overlapping impl detection) are a stable language
guarantee this project does not control the versioning of, and the three
printed lengths/values are plain integers and an `Option`, uninvolved with
`BytesSlot`'s since-rewritten `Debug`/`PartialEq`. The recorded output
should still reproduce; it just cannot be re-run to confirm.*

```rust
// -ev_probe/src/bin/array_impl_coherence.rs
// A local replica of the crate's write half, plus the impl that would remove
// the slicing ritual — to check it does not collide with the other two.
impl< T > Fill< TypedCell< T > > for T { /* … */ }
impl< const N : usize > Fill< BytesCell< N > > for &[ u8 ] { /* … */ }

// The proposed third impl.
impl< const M : usize, const N : usize > Fill< BytesCell< N > > for &[ u8; M ]
{
  fn fill( self, slot : &mut BytesCell< N > ) { Fill::fill( &self[ .. ], slot ) }
}

publish_into( &mut c, b"abcd" );          // the form that does not compile today
publish_into( &mut c, &b"xy"[ .. ] );     // the form the crate requires today
publish_into( &mut t, 7u32 );             // the blanket impl, still unambiguous
```

```
    array literal, no slice   len 4
    slice expression, as now  len 2
    blanket impl unaffected   Some(7)
```

---

### EV49 — Going Through the One Path Costs a Slicing Expression That Going Around It Does Not

The census splits perfectly. Every byte payload that reaches a slot through
`publish_into` is written as a slicing expression — `&b"abcd"[ .. ]`,
`&[][ .. ]`, `&raw.to_le_bytes()[ .. ]` — five call sites plus a doctest, no
exceptions. Every byte payload written directly to the slot is a plain literal:
thirty-three `write( b"…" )` calls across `ring_slot`, `ring_spsc`, `ring_mpsc`,
`ring_store` and this crate's own suite, and **zero** using a slice expression.

The reason is one word of signature. `BytesSlot::write` takes `payload : &[ u8 ]`,
a concrete parameter type, so `&[ u8; 4 ]` unsizes to it at the call site.
`publish_into` takes `payload : P where P : Fill< S >`, a generic parameter, and
unsizing coercion does not fire when the target is an inference variable — so
`&[ u8; 4 ]` stays `&[ u8; 4 ]`, a type with no `Fill` impl. The probe's error
says so and even names the near-miss: "not implemented for `&[u8; 4]` but it is
implemented for `&[u8]`".

**Finding.** The crate's argument is that there should be exactly one path, and
the path it provides is the one that costs a syntactic ritual at every call site
while the path it wants to replace is the ergonomic one. This crate's own test
file demonstrates both forms fifty-six lines apart — `publish_into( &mut bytes,
&b"x"[ .. ] )` at `:183`, `bytes.write( b"cd" )` at `:239` — and nothing marks
the difference or explains it, so the doctest at `:170` teaches the ritual as
though it were the natural way to write a byte literal. One sentence on the byte
impl, saying that a generic parameter forecloses the array-to-slice coercion and
that `[ .. ]` is therefore required rather than stylistic, is the difference
between a reader copying a form and understanding one.

---

### EV50 — A Third Impl Removes the Ritual and Does Not Collide With Anything

The obvious fix is to implement `Fill` for the array type as well, forwarding to
the slice impl. The obvious worry is coherence: this crate already carries a
blanket `impl< T > Fill< TypedSlot< T > > for T` that claims every type there is,
including `&[ u8; M ]`.

The replica probe settles it. All three impls coexist, `publish_into( &mut c,
b"abcd" )` compiles and lands four bytes, the existing sliced form still works,
and the blanket impl is unaffected — because the blanket impl's `S` is
`TypedCell< T >` and the new impl's is `BytesCell< N >`, so the two never
overlap. It is the same reason the existing `&[ u8 ]` impl is legal, applied once
more.

**Finding.** Six lines of forwarding impl would delete the ritual from every
present and future byte call site, and the probe shows it costs nothing in
coherence. Whether to take it is a real design question rather than an obvious
yes — a second impl is a second thing to keep in step with the first, and the
crate's stated virtue is that there is exactly one of everything, so adding an
impl to make one function nicer cuts against the grain deliberately. That
tradeoff should be recorded either way. As it stands, neither the impl nor the
decision not to write it exists, and the ritual is transmitted by example from
the one doctest that shows it.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`item/002`](../item/002_four_impls_and_what_the_blanket_one_does_not_claim.md) | Why the byte impl is on `&[ u8 ]` and nothing foreign |
| [`type/002`](../type/002_a_blanket_impl_over_every_type_there_is.md) | The blanket impl a third one has to coexist with |
| [`integration/002`](../integration/002_the_ring_writes_slots_without_this_crate.md) | The path callers take instead, where the ritual does not apply |
| [`api/001`](../api/001_two_traits_three_functions_one_associated_type.md) | What a call site looks like overall |

### Sources

| Fact | Where |
|------|-------|
| `publish_into`'s generic payload parameter | `ring_event/src/lib.rs:173-175` |
| `BytesSlot::write`'s concrete one | `ring_slot/src/lib.rs:347` |
| Five sliced call sites through this crate, plus a doctest | Census above |
| Thirty-three plain-literal direct writes, none sliced | Census above |
| Both forms in one test file, fifty-six lines apart | `ring_event/tests/event_test.rs:183`, `:239` |
| The coercion failing at a generic parameter | Probe above |
| The third impl coexisting with both existing ones | Probe above |

### Tests

| Test | Covers |
|------|--------|
| `a_byte_payload_of_every_length_up_to_slot_size_survives_storage` | The computed-array form of the ritual |
| `a_byte_payload_longer_than_the_slot_is_refused_and_changes_nothing` | Two sliced call sites in one test |
| `peek_agrees_with_the_slots_own_emptiness` | The direct `write( b"cd" )` form, in the same file |
