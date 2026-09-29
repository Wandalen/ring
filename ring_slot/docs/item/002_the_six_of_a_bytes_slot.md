# Item: The Six of a `BytesSlot`

### Scope

**Purpose:** Catalogue `BytesSlot< N >`'s six inherent methods one at a time,
record that `capacity` ignores its receiver to report a type parameter, and
record that the read/write pair admits no partial access — so a payload can only
ever be replaced whole.

**Responsibility:** `BytesSlot::empty`, `capacity`, `len`, `is_empty`, `write`,
`read` — what each does and what the set as a whole does not offer.

**In Scope:** `ring_slot/src/lib.rs:278, 290, 304, 323, 347, 367`;
`ring_event/src/lib.rs:137-145`.

**Out of Scope:** `TypedSlot`'s four are
[`item/001`](001_the_four_of_a_typed_slot.md). `write`'s three steps are
[`algorithm/001`](../algorithm/001_write_is_a_bound_check_and_a_copy.md) — this
instance is about the method set, not the step sequence. What `N` costs in
layout is [`data_structure/001`](../data_structure/001_sixteen_bytes_to_carry_eight.md).

---

## The Six

```sh
cd "$(git rev-parse --show-toplevel)"
awk '/^  \/\/\/ assert_eq!\( BytesSlot::< 8 >::empty\(\)\.capacity\(\), 8 \);$/{ n1 = NR } n1 && NR == n1 + 3 { print } /^  pub const fn capacity\( &self \) -> usize$/{ print } /^  pub const fn len\( &self \) -> usize$/{ print } /^  pub const fn is_empty\( &self \) -> bool$/{ print } /^  pub fn write\( &mut self, payload : &\[ u8 \] \) -> Result< \(\), RingError >$/{ print } /^  pub fn read\( &self \) -> &\[ u8 \]$/{ print }' ring_slot/src/lib.rs
```

Live output:

```
  pub const fn empty() -> Self
  pub const fn capacity( &self ) -> usize
  pub const fn len( &self ) -> usize
  pub const fn is_empty( &self ) -> bool
  pub fn write( &mut self, payload : &[ u8 ] ) -> Result< (), RingError >
  pub fn read( &self ) -> &[ u8 ]
```

| Method | Receiver | Returns | Role |
|--------|----------|---------|------|
| `empty` | — | `Self` | Construct. `const`; zeroes the array |
| `capacity` | `&self` | `usize` | Report `N` |
| `len` | `&self` | `usize` | Report bytes written |
| `is_empty` | `&self` | `bool` | `len == 0` |
| `write` | `&mut self` | `Result< (), RingError >` | Replace the payload, or refuse |
| `read` | `&self` | `&[ u8 ]` | Borrow the payload |

Four `const` reads, one fallible mutator, one borrow. Six against `TypedSlot`'s
four — the extra two are `capacity` and `len`, the size questions a
fixed-array shape can answer and an `Option` cannot.

---

### SL27 — `capacity` Takes a Receiver It Does Not Read

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A3 -F '  pub const fn capacity( &self ) -> usize' ring_slot/src/lib.rs
```

Live output:

```
  pub const fn capacity( &self ) -> usize
  {
    N
  }
```

The body is the const parameter. `self` is named in the signature and never
mentioned again — the function is `fn( &BytesSlot< N > ) -> usize` returning a
value fixed at monomorphisation.

That has a cost and a reason. The cost: knowing a type's capacity requires
constructing one. `BytesSlot::< 8 >::capacity()` does not compile; the tests
write `BytesSlot::< 8 >::empty().capacity()` instead, constructing a
sixteen-byte value to read a compile-time `8`. An associated `const CAPACITY :
usize = N;` would be reachable from the type alone.

The reason is that no shipped code needs either form:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '--- capacity() on a BytesSlot, in any src/ ---'
grep -r 'BytesSlot' ring_*/src/*.rs 2>/dev/null | grep -vE ':[[:space:]]*//' \
  | grep 'capacity' || echo '  none'
echo '--- and in ring_slot own tests ---'
grep -c 'capacity()' ring_slot/tests/slot_test.rs
```

Live output:

```
--- capacity() on a BytesSlot, in any src/ ---
  none
--- and in ring_slot own tests ---
7
```

Seven callers, all of them assertions in the crate's own suite. No crate in the
family asks a `BytesSlot` how large it is.

**Finding.** `capacity` is a method-shaped answer to a type-level question, with
no caller outside its own tests. The method form would be justified if the
question had to be asked generically — but `Slot` does not declare `capacity`
([`api/002`](../api/002_a_trait_with_two_methods.md)), so generic code cannot
call it either, and the one form that would make it reachable is exactly the one
that does not exist.

Left as-is it costs nothing: it is `const`, it inlines to a literal, and the
`&self` is erased. It is worth recording because it is the crate's only method
that reads none of its receiver, and because the shape it *should* have —
an associated const, or a trait method — is determined by a decision (which
questions belong on `Slot`) that was made elsewhere and never revisited here.

---

### SL28 — There Is No Partial Access, So a Payload Can Only Be Replaced Whole

The mutating half of the pair is `write( &mut self, &[ u8 ] )` and the reading
half is `read( &self ) -> &[ u8 ]`. There is no third option:

```sh
cd "$(git rev-parse --show-toplevel)"
grep 'get_mut\|as_mut\|_mut(\|push\|extend\|append' ring_slot/src/lib.rs \
  || echo '  no mutable accessor, no incremental writer, anywhere in the crate'
```

Live output:

```
  no mutable accessor, no incremental writer, anywhere in the crate
```

So a caller assembling a payload in pieces must assemble it somewhere else and
hand over a finished slice. There is no `&mut [ u8 ]` to build into and no way
to append.

The one shipped consumer reads both halves in a single line:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A5 -F '  type Out< '"'"'a > = &'"'"'a [ u8 ];' ring_event/src/lib.rs
```

Live output:

```
  type Out< 'a > = &'a [ u8 ];

  fn peek( &self ) -> Option< &[ u8 ] >
  {
    if self.is_empty() { None } else { Some( self.read() ) }
  }
```

`is_empty` to decide, `read` to borrow — and `Out< 'a >` is a borrow, so nothing
is copied. That is the whole of `BytesSlot`'s consumption in the family.

**Finding.** The all-or-nothing shape is right for a ring and the crate never
says so. A slot lives in a pre-allocated array and is written once per lap by
whoever claimed it; there is no phase in which two parties append to the same
slot, so an incremental writer would add a state — partially written — that the
length field alone cannot distinguish from a completed short write. That is the
same conflation [`decisions/002`](../decisions/002_a_length_rather_than_a_flag.md)
SL8 records for zero-length payloads, and refusing partial access is what keeps
it from mattering.

The cost lands on a caller with a payload built in pieces: it needs a scratch
buffer and a copy into the slot, so the bytes move twice. For the small `N` the
family uses ([`data_structure/001`](../data_structure/001_sixteen_bytes_to_carry_eight.md):
4 to 32) that is a handful of bytes and not worth an API. For a large `N` it
would be, and the crate offers no guidance on where that line is — a reader
sizing a `BytesSlot< 4096 >` finds `write` and no note that it will be copying
through a staging buffer to use it.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`item/001`](001_the_four_of_a_typed_slot.md) | The other shape's four, and how they are reached |
| [`algorithm/001`](../algorithm/001_write_is_a_bound_check_and_a_copy.md) | `write`'s three steps and the ordering that makes failure total |
| [`data_structure/001`](../data_structure/001_sixteen_bytes_to_carry_eight.md) | What `N` costs, and the `N` the family uses |
| [`type/002`](../type/002_the_const_parameter_as_capacity.md) | `N` as a type parameter, which `capacity` reports |
| [`invariant/001`](../invariant/001_a_read_returns_what_was_written.md) | The property `write` and `read` jointly uphold |

### Sources

| Fact | Where |
|------|-------|
| The six signatures | `ring_slot/src/lib.rs:278, 290, 304, 323, 347, 367` |
| `capacity`'s body | `ring_slot/src/lib.rs:290-293` |
| No `BytesSlot::capacity` caller in any `src/` | `ring_*/src/*.rs` — no occurrence |
| Seven callers in the crate's own suite | `ring_slot/tests/slot_test.rs` |
| No mutable accessor or incremental writer | `ring_slot/src/lib.rs` — no occurrence |
| The only shipped consumer | `ring_event/src/lib.rs:137-145` |

### Tests

| Test | Covers |
|------|--------|
| `capacity_is_the_const_parameter` | `capacity` reporting `N` at three widths |
| `a_bytes_slot_round_trips_its_payload` | `write` then `read`, and `len` agreeing |
| `a_fresh_bytes_slot_is_empty` | `empty`, `len`, and `is_empty` at construction |
| `a_zero_capacity_slot_accepts_only_nothing` | `capacity() == 0`, the degenerate instantiation |
| `ring_event` — the `Peek` doctest | `is_empty` and `read` composed, the one shipped use |
