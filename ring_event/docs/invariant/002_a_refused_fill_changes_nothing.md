# Invariant: A Refused Fill Changes Nothing

### Scope

**Purpose:** Record the property a caller relies on after a failed publish — that
the slot is exactly as it was — establish where it is asserted, and measure how
much of the slot those assertions actually reach.

**Responsibility:** `BytesSlot::write`'s early return, `Fill::fill`'s `# Errors`
section, the crate's refusal test, and the geometry that makes its assertion
total.

**In Scope:** `ring_event/src/lib.rs:53-64`;
`ring_slot/src/lib.rs:347-354`;
`ring_event/tests/event_test.rs:144-158`; probe below.

**Out of Scope:** The impossible `Err` on the typed side is
[`api/002`](../api/002_a_result_one_impl_can_never_return.md). Why `read()` hides
part of the slot is
[`data_structure/001`](../data_structure/001_a_crate_that_declares_no_data.md).

---

## Where the Property Is Asserted, and What It Rests On

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- what Fill promises about a refused write --'
command grep -m1 -A11 -F 'pub trait Fill< S >' ring_event/src/lib.rs
echo '  -- the mechanism, in the only impl that can refuse --'
command grep -m1 -A7 -F '  pub fn write( &mut self, payload : &[ u8 ] ) -> Result< (), RingError >' ring_slot/src/lib.rs
echo '  -- the test that pins it, and the shape it picks --'
command grep -m1 -A13 -F 'fn a_byte_payload_longer_than_the_slot_is_refused_and_changes_nothing()' ring_event/tests/event_test.rs
echo '  -- every place the family states what a refused write preserved --'
command grep -r 'left the old one\|previous contents intact' --include=*.rs ring_*/
echo '  -- and the geometry each of the two tests picks --'
command grep -m1 -A1 -F '  let mut slot = BytesSlot::< 4 >::empty();' ring_event/tests/event_test.rs
command grep -m1 -A8 -F 'fn a_failed_write_leaves_the_previous_contents_intact()' ring_slot/tests/slot_test.rs | tail -n 8
```

Live output:

```
  -- what Fill promises about a refused write --
pub trait Fill< S >
{
  /// Write `self` into `slot`, replacing whatever it held.
  ///
  /// # Errors
  ///
  /// Whatever the slot shape refuses — [`RingError::BatchTooLarge`] for a byte
  /// payload longer than the slot, which is a configuration error rather than
  /// back-pressure: no amount of draining makes the payload fit. A typed
  /// payload cannot fail, and says so by never returning `Err`.
  fn fill( self, slot : &mut S ) -> Result< (), RingError >;
}
  -- the mechanism, in the only impl that can refuse --
  pub fn write( &mut self, payload : &[ u8 ] ) -> Result< (), RingError >
  {
    if payload.len() > N
    {
      return Err( RingError::BatchTooLarge { requested : payload.len(), capacity : N } );
    }
    self.bytes[ ..payload.len() ].copy_from_slice( payload );
    self.len = payload.len();
  -- the test that pins it, and the shape it picks --
fn a_byte_payload_longer_than_the_slot_is_refused_and_changes_nothing()
{
  let mut slot = BytesSlot::< 4 >::empty();
  publish_into( &mut slot, &b"abcd"[ .. ] ).unwrap();

  // Not `Full`: a payload that does not fit is a configuration error, and no
  // amount of draining shrinks it. The same distinction `ring_batch` draws
  // between `BatchTooLarge` and `Full`, applied one level down.
  let refused = publish_into( &mut slot, &b"abcde"[ .. ] );
  assert_eq!( refused, Err( RingError::BatchTooLarge { requested : 5, capacity : 4 } ) );
  assert!( refused.unwrap_err().is_configuration(), "a caller must not retry this" );

  assert_eq!( drain_from( &slot ), Some( &b"abcd"[ .. ] ), "the refused write left the old one" );
}
  -- every place the family states what a refused write preserved --
ring_event/tests/event_test.rs:  assert_eq!( drain_from( &slot ), Some( &b"abcd"[ .. ] ), "the refused write left the old one" );
ring_slot/tests/slot_test.rs:/// A failed write leaves the previous contents intact — the slot is not
ring_slot/src/lib.rs:  /// // A failed write leaves the previous contents intact.
  -- and the geometry each of the two tests picks --
  let mut slot = BytesSlot::< 4 >::empty();
  publish_into( &mut slot, &b"abcd"[ .. ] ).unwrap();
{
  let mut slot = BytesSlot::< 4 >::empty();
  slot.write( b"abcd" ).unwrap();
  assert!( slot.write( b"abcde" ).is_err() );

  assert_eq!( slot.read(), b"abcd" );
  assert_eq!( slot.len(), 4 );
  assert!( !slot.is_empty() );
```

---

## The Same Refusal at a Geometry Neither Suite Uses

*The `before`/`after` lines below predate `BytesSlot`'s `Debug`/`PartialEq`
rewrite (see EV12's Disposition in
[`data_structure/002`](../data_structure/002_what_clear_leaves_behind.md)):
today's `Debug` prints only `BytesSlot { payload: [97, 98] }`, not the raw
`bytes`/`len` fields shown here, and `PartialEq` compares only the written
prefix rather than deriving over both fields. The two structs shown are
still genuinely equal under the new impl too, since both hold the same
2-byte payload — `whole slot equal true` still holds — only the literal
printed form has changed.*

```rust
// -ev_probe/src/bin/refused_write_geometry.rs
let mut slot = BytesSlot::< 8 >::empty();
publish_into( &mut slot, &b"ab"[ .. ] ).unwrap();
let before = slot.clone();

let refused = publish_into( &mut slot, &b"123456789"[ .. ] );
```

```
    refused            Err(BatchTooLarge { requested: 9, capacity: 8 })
    before             BytesSlot { bytes: [97, 98, 0, 0, 0, 0, 0, 0], len: 2 }
    after              BytesSlot { bytes: [97, 98, 0, 0, 0, 0, 0, 0], len: 2 }
    whole slot equal   true
    what the test sees Some([97, 98])
    bytes read() hides 6
```

---

### EV23 — Three Places State the Property and Neither Trait Contract Does

`Fill::fill` takes `&mut S`. An implementor is free to write half a payload, then
discover it does not fit, then return `Err` — the signature permits it and no
sentence forbids it.

The one impl that can refuse does not do that. `BytesSlot::write` tests
`payload.len() > N` and returns before touching either field, so a refused write
is a pure read of the slot. Three places in the family say so: `ring_slot`'s
doctest at `:344`, its test's doc comment at `slot_test.rs:249` — which goes
further, "the slot is not half-updated, so a caller that handles the error still
has valid data" — and this crate's own assertion message, "the refused write left
the old one".

Neither trait says it. `Fill::fill`'s summary line is "Write `self` into `slot`,
replacing whatever it held", which describes the success case. Its `# Errors`
section is unusually thorough about *which* error and *why* the distinction from
`Full` matters — "no amount of draining makes the payload fit" — and says nothing
about the slot's state afterwards.

**Finding.** This is the sentence a caller most needs from an `# Errors` section
and the one it does not have. A caller who sees `Err( BatchTooLarge )` from
`publish_into` must decide whether the slot is still usable, and the correct
answer — it is, unchanged, and the claim it held is still good — is currently
recoverable only by reading `ring_slot`'s implementation or trusting a test name.
Adding *"a refused fill leaves `slot` unchanged"* to `# Errors` costs one line and
turns three independent statements into one contract every future implementor
is held to. Without it the extension point admits an implementor that mutates and
then fails, and nothing would catch it.

---

### EV24 — The Assertion Is Total Only Because the Test Fills the Slot Completely

`a_byte_payload_longer_than_the_slot_is_refused_and_changes_nothing` checks the
survivor with `assert_eq!( drain_from( &slot ), Some( &b"abcd"[ .. ] ) )`, and
`drain_from` forwards to `read()`, which returns `bytes[ ..len ]`. So the
assertion sees `len` bytes and the `len` value, and nothing else.

The test uses a `BytesSlot< 4 >` holding four bytes. `len` is `N`, there is no
tail, and the assertion therefore happens to cover the entire slot. That is a
property of the numbers chosen, not of the assertion — it is the one geometry in
which `read()` hides nothing.

`ring_slot`'s own test picks the same one. `a_failed_write_leaves_the_previous_contents_intact`
is also a `BytesSlot< 4 >` holding `b"abcd"`, and its three assertions —
`read()`, `len()`, `is_empty()` — all read `bytes[ ..len ]` and `len`. Its doc
comment makes the strongest claim of the three, "the slot is not half-updated",
which is a statement about the whole struct that none of its assertions can
check. Neither crate compares a whole slot after a refusal; the workspace's only
`assert_eq!( slot, … )` is in `ring_batch` and compares an index.

The probe runs the same refusal at a geometry neither suite uses: a
`BytesSlot< 8 >` holding two bytes, refusing nine. The property still holds
completely — `before == after` over both fields — but the assertion form both
tests use would have seen only `[ 97, 98 ]` either way, with six bytes outside its
reach. Had `write` corrupted the tail before returning `Err`, both tests would
still pass.

**Finding.** The invariant is real and the probe confirms it at a case neither
suite reaches. What the suites demonstrate is narrower than what they claim:
"changes nothing" and "not half-updated" are verified over the readable prefix,
and the readable prefix is everything only because both fixtures are full. Either
test rewritten with `BytesSlot< 8 >` and a two-byte payload would assert strictly
less while looking identical — and `assert_eq!( slot, before )` after a
`let before = slot.clone()` covers the whole struct at any geometry, in one extra
line.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`invariant/001`](001_two_readings_of_one_emptiness.md) | The other property the suite pins by state comparison |
| [`api/002`](../api/002_a_result_one_impl_can_never_return.md) | Why only one impl can reach this path at all |
| [`data_structure/002`](../data_structure/002_what_clear_leaves_behind.md) | The same blind spot, at a different operation |
| [`decisions/001`](../decisions/001_the_write_half_on_the_payload_the_read_half_on_the_slot.md) | The implementor the missing contract would bind |

### Sources

| Fact | Where |
|------|-------|
| `fill` taking `&mut S` with no state guarantee on `Err` | `ring_event/src/lib.rs:55-63` |
| The early return, before either field is touched | `ring_slot/src/lib.rs:349-352` |
| "A failed write leaves the previous contents intact" | `ring_slot/src/lib.rs:344`, `ring_slot/tests/slot_test.rs:249` |
| The one assertion in the family, over `read()` only | `ring_event/tests/event_test.rs:157` |
| Both fixtures being full, so `read()` hides nothing | `ring_event/tests/event_test.rs:147-148`, `ring_slot/tests/slot_test.rs:254-255` |
| The property holding at a geometry with a tail | Probe above |

### Tests

| Test | Covers |
|------|--------|
| `a_byte_payload_longer_than_the_slot_is_refused_and_changes_nothing` | The property, at a full slot |
| `a_byte_payload_of_every_length_up_to_slot_size_survives_storage` | Every accepted length, none refused |
| `a_typed_publish_cannot_fail` | The shape that never reaches this path |
