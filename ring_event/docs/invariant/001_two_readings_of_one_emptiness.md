# Invariant: Two Readings of One Emptiness

### Scope

**Purpose:** Record the invariant the drain path rests on — that `peek()` and
`Slot::is_empty()` agree — establish that nothing enforces it, and measure how
much of it the suite actually pins.

**Responsibility:** Both `Peek` impls and what each consults, both `Slot::is_empty`
impls, the absence of any relation between the two traits, and the states
`peek_agrees_with_the_slots_own_emptiness` visits.

**In Scope:** `ring_event/src/lib.rs:103`, `:131-134`, `:141-144`;
`ring_slot/src/lib.rs:41-44`, `:178-181`, `:133-136`, `:323-326`,
`:387-390`; `ring_event/tests/event_test.rs:282-298`; probe below.

**Out of Scope:** The one state where the two agree on a surprising answer is
[`pitfall/001`](../pitfall/001_a_zero_length_payload_reads_as_nothing.md). The
three-bound cost of keeping the traits separate is
[`api/001`](../api/001_two_traits_three_functions_one_associated_type.md).

---

## What Each Side Actually Reads

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the invariant, and why the suite says it matters --'
command grep -m1 -A7 -F '  assert_eq!( Peek::peek( &bytes ), Some( &b"cd"[ .. ] ) );' ring_event/tests/event_test.rs | tail -n 5
echo '  -- every state the test visits --'
command grep -m1 -A11 -F '  let mut typed = TypedSlot::< u8 >::empty();' ring_event/tests/event_test.rs
echo '  -- the two readings of a typed slots emptiness --'
awk '/^  pub const fn get\( &self \) -> Option< &T >$/{ n1 = NR } n1 && NR >= n1 && NR <= n1 + 4 { print } /^impl< T > Slot for TypedSlot< T >$/{ n2 = NR } n2 && NR >= n2 + 2 && NR <= n2 + 5 { print }' ring_slot/src/lib.rs
echo '  -- and of a byte slots --'
awk '/^  pub const fn is_empty\( &self \) -> bool$/{ n1 = NR } n1 && NR >= n1 && NR <= n1 + 4 { print } /^impl< const N : usize > Slot for BytesSlot< N >$/{ n2 = NR } n2 && NR >= n2 + 2 && NR <= n2 + 5 { print }' ring_slot/src/lib.rs
echo '  -- what each Peek impl consults --'
sed -n '/^  fn peek( &self ) -> Option< &T >$/,/^  }$/p;/^  fn peek( &self ) -> Option< &\[ u8 ] >$/,/^  }$/p' ring_event/src/lib.rs
echo '  -- and whether either trait names the other --'
command grep -m1 -A3 -F 'pub trait Slot' ring_slot/src/lib.rs
command grep -m1 -A3 -F 'pub trait Peek' ring_event/src/lib.rs
```

Live output:

```
  -- the invariant, and why the suite says it matters --
#[ test ]
fn peek_agrees_with_the_slots_own_emptiness()
{
  // Two independent readings of one state; a divergence between them would let
  // a drain read a slot the handshake considers empty.
  -- every state the test visits --
  let mut typed = TypedSlot::< u8 >::empty();
  let mut bytes = BytesSlot::< 4 >::empty();

  assert_eq!( typed.peek().is_none(), typed.is_empty() );
  assert_eq!( bytes.peek().is_none(), bytes.is_empty() );

  typed.set( 1 );
  bytes.write( b"z" ).unwrap();

  assert_eq!( typed.peek().is_none(), typed.is_empty() );
  assert_eq!( bytes.peek().is_none(), bytes.is_empty() );
}
  -- the two readings of a typed slots emptiness --
  pub const fn get( &self ) -> Option< &T >
  {
    self.0.as_ref()
  }

  fn is_empty( &self ) -> bool
  {
    self.0.is_none()
  }
  -- and of a byte slots --
  pub const fn is_empty( &self ) -> bool
  {
    self.len == 0
  }

  fn is_empty( &self ) -> bool
  {
    Self::is_empty( self )
  }
  -- what each Peek impl consults --
  fn peek( &self ) -> Option< &T >
  {
    self.get()
  }
  fn peek( &self ) -> Option< &[ u8 ] >
  {
    if self.is_empty() { None } else { Some( self.read() ) }
  }
  -- and whether either trait names the other --
pub trait Slot
{
  /// Whether this slot currently holds nothing.
  fn is_empty( &self ) -> bool;
pub trait Peek
{
  /// What a reader is handed when the slot holds something.
  type Out< 'a > where Self : 'a;
```

---

## An Implementor Whose Two Readings Disagree

*The probe below is frozen at authoring time — the scratch crate that ran it
has since been swept — and no drift was found: `Slot::is_empty` and
`Peek::peek`'s signatures (`ring_slot/src/lib.rs:41`,
`ring_event/src/lib.rs:103`) are unchanged, and the output is a plain
`Option< u32 >`, uninvolved with `BytesSlot`'s since-rewritten
`Debug`/`PartialEq` (see
[`invariant/002`](002_a_refused_fill_changes_nothing.md)). The recorded
output should still reproduce; it just cannot be re-run to confirm.*

A probe writing the impls a downstream crate would write, with one deliberate
divergence: `Slot::is_empty` always answers `true`.

```rust
// -ev_probe/src/bin/broken_invariant.rs
#[ derive( Default ) ]
struct LyingSlot( Option< u32 > );

impl Slot for LyingSlot
{
  // Always reports empty, whatever it holds.
  fn is_empty( &self ) -> bool { true }
  fn clear( &mut self ) { self.0 = None; }
}

impl Peek for LyingSlot
{
  type Out< 'a > = &'a u32;
  fn peek( &self ) -> Option< &u32 > { self.0.as_ref() }
}
```

```
    Slot::is_empty says  true
    drain_from says      Some(42)
```

---

### EV21 — The Invariant the Drain Rests On Is Held by Coincidence in One Shape and by Construction in the Other, and by Nothing in the Trait

`peek_agrees_with_the_slots_own_emptiness` states the stake in its own comment:
"Two independent readings of one state; a divergence between them would let a
drain read a slot the handshake considers empty." That is the whole risk — a
consumer told a slot is empty while `drain_from` hands it a payload.

The two shipped shapes hold the invariant for different reasons.
`Peek for TypedSlot` calls the inherent `get()`, which is `self.0.as_ref()`;
`Slot::is_empty` separately reads `self.0.is_none()`. Two expressions over one
field, neither aware of the other — genuinely the "two independent readings" the
comment describes, agreeing because `Option` makes them agree.
`Peek for BytesSlot` is not independent at all: it calls the inherent
`is_empty()`, `self.len == 0`, and `Slot::is_empty` forwards to that same
inherent method. One reading wearing two names.

Above them, nothing connects the sides. `Slot` declares `is_empty` and `clear`
and names no supertrait; `Peek` declares `Out` and `peek` and names no supertrait
either. They live in different crates. A third implementor writes both halves
freely, and the probe is what that looks like: a slot whose `Slot::is_empty`
answers `true` while `drain_from` returns `Some( 42 )`. It compiles without a
warning and runs.

**Finding.** The property the family's drain correctness depends on is stated in a
test comment, in this crate, about types declared in another. Nothing in either
trait's contract requires it, no doc comment on `Peek::peek` mentions
`Slot::is_empty` or vice versa, and the extension point
[`decisions/001`](../decisions/001_the_write_half_on_the_payload_the_read_half_on_the_slot.md)
documents is exactly the door through which a violating implementor arrives. One
sentence on `Peek::peek` — *`peek` must return `None` exactly when
`Slot::is_empty` is true* — would make it a contract rather than an observation,
and costs nothing today because both shipped impls already satisfy it.

---

### EV22 — The Test Visits Two States per Shape and the Third Is Where the Answer Gets Interesting

The test constructs a fresh slot of each shape, asserts, writes one payload to
each, and asserts again. Four assertions, two states per shape: empty, and holding
one thing.

Three states it does not visit:

The slot after `clear` — and therefore after `recycle`, which is the whole third
operation of the crate. The byte shape reaches that state with residue in its
array (see
[`data_structure/002`](../data_structure/002_what_clear_leaves_behind.md)), and
whether the two readings still agree there is asserted nowhere, though both would
in fact answer from `len`.

The byte slot after a zero-length write, which is the one state where the two
readings agree on an answer a caller may not expect — a published slot reporting
empty. `Peek`'s own doc comment documents that state at length; the invariant test
does not visit it.

And the free-function path. Every assertion here calls `typed.peek()` and
`bytes.peek()` as methods. `drain_from` — the function the module documentation
says a ring's drain calls — appears nowhere in this test, so the invariant is
pinned for the trait method and left to inference for the surface actually
documented as the entry point.

**Finding.** The test is correctly named and its comment is the clearest statement
of the property in the codebase. What it covers is narrower than what it claims:
two states out of at least four, one of two call paths, and both of the shapes
that already hold the invariant structurally. Extending it costs two assertions
after a `recycle` and one substitution of `drain_from` for `.peek()`.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`invariant/002`](002_a_refused_fill_changes_nothing.md) | The other property the suite pins by state comparison |
| [`pitfall/001`](../pitfall/001_a_zero_length_payload_reads_as_nothing.md) | The state this test does not visit |
| [`data_structure/002`](../data_structure/002_what_clear_leaves_behind.md) | The other state, after `recycle` |
| [`decisions/001`](../decisions/001_the_write_half_on_the_payload_the_read_half_on_the_slot.md) | The door a violating implementor comes through |

### Sources

| Fact | Where |
|------|-------|
| The invariant, and the stake | `ring_event/tests/event_test.rs:283-286` |
| Four assertions over two states per shape | `ring_event/tests/event_test.rs:287-298` |
| `TypedSlot`'s two independent readings | `ring_slot/src/lib.rs:178-181`, `:133-136` |
| `BytesSlot`'s single reading under two names | `ring_slot/src/lib.rs:323-326`, `:387-390` |
| Neither trait naming the other | `ring_slot/src/lib.rs:41`, `ring_event/src/lib.rs:103` |
| A violating implementor compiling and running | Probe above |

### Tests

| Test | Covers |
|------|--------|
| `peek_agrees_with_the_slots_own_emptiness` | The invariant, at two states per shape |
| `an_unpublished_typed_slot_reads_as_nothing` | One side of it, typed shape |
| `an_unpublished_bytes_slot_reads_as_nothing_rather_than_as_an_empty_payload` | One side of it, byte shape |
