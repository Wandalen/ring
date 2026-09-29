# Decision: A Length Rather Than a Flag

### Scope

**Purpose:** Record that `BytesSlot` tracks occupancy with a length and nothing
else, what that buys, the one distinction it gives up, and where the argument
for it is written.

**Responsibility:** The internal representation ruling inside `BytesSlot` — a
`usize` length against the alternatives of a flag byte or `MaybeUninit`.

**In Scope:** `ring_slot/src/lib.rs:22-24, 237-241, 347-356`;
`ring_event/src/lib.rs:105-116`.

**Out of Scope:** What the length *fails* to distinguish at the invariant level
is [`invariant/002`](../invariant/002_the_two_emptiness_paths_agree.md). What
the tail behind the length holds is
[`pitfall/002`](../pitfall/002_clear_forgets_it_does_not_erase.md).

---

## The Ruling as Stated

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A2 -F '//! No `unsafe`. A [`BytesSlot`] is a fixed-length buffer plus a length, so a' ring_slot/src/lib.rs
```

Live output:

```
//! No `unsafe`. A [`BytesSlot`] is a fixed-length buffer plus a length, so a
//! partially-filled slot reads back exactly what was written and nothing else,
//! without `MaybeUninit`.
```

The alternative the module comment names is `MaybeUninit` — the usual way to
avoid initialising a buffer you are about to overwrite. Rejecting it is what
lets the crate carry no `unsafe`, since reading a partially-initialised
`MaybeUninit` buffer requires an `unsafe` assertion that the read region was
written.

The struct that results is two fields:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A7 -F '/// assert!( !slot.is_empty() );' ring_slot/src/lib.rs | tail -n 6
```

Live output:

```
/// slot.clear();
/// assert_eq!( slot, BytesSlot::< 16 >::empty(), "clear returns it to a fresh slot's value" );
/// assert_eq!( format!( "{slot:?}" ), "BytesSlot { payload: [] }" );
/// ```
#[ derive( Clone ) ]
pub struct BytesSlot< const N : usize >
```

---

### SL7 — The Length Buys `unsafe`-Freedom at a Fixed Zeroing Cost

`empty()` initialises the whole array:

```sh
cd "$(git rev-parse --show-toplevel)"
# BytesSlot's, not TypedSlot's — both are named `empty`
command grep -m1 -A6 -F '  /// assert_eq!( BytesSlot::< 8 >::empty().capacity(), 8 );' ring_slot/src/lib.rs | tail -n 4
```

Live output:

```
  pub const fn empty() -> Self
  {
    Self { bytes : [ 0; N ], len : 0 }
  }
```

Every `BytesSlot< N >` costs `N` bytes of zeroing at construction, whether or not
a single byte is ever written. That is the price of not using `MaybeUninit`, and
for a ring it is paid exactly once per slot at allocation rather than per publish
— the ring allocates its slots up front and reuses them, so the cost amortises to
nothing across laps.

`empty()` being `const` is what makes this cheap in practice: a `[ BytesSlot< N >;
K ]` in static position is zeroed by the linker rather than at runtime.

**Finding.** The ruling is correct and its cost is structurally bounded by the
ring's own allocation pattern — the crate pays `N` zero-bytes once per slot for
life, in exchange for the workspace-wide `unsafe-code = "deny"` lint applying to
this crate without exception. The observation worth recording is that the
argument depends on the ring's allocate-once behaviour, which is stated in the
crate ("a ring's slots are allocated once") but stated as a justification for
*fixed capacity*, not for this ruling. The two arguments share a premise and only
one of them names it.

---

### SL8 — A Length Cannot Distinguish Empty From Zero-Length, and the Crate Does Not Say So

`write` sets the length to exactly the payload's length, including zero:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A9 -F '  pub fn write( &mut self, payload : &[ u8 ] ) -> Result< (), RingError >' ring_slot/src/lib.rs
```

Live output:

```
  pub fn write( &mut self, payload : &[ u8 ] ) -> Result< (), RingError >
  {
    if payload.len() > N
    {
      return Err( RingError::BatchTooLarge { requested : payload.len(), capacity : N } );
    }
    self.bytes[ ..payload.len() ].copy_from_slice( payload );
    self.len = payload.len();
    Ok( () )
  }
```

So `write( b"" )` produces `len == 0`, which is what `empty()` produces. A slot
that deliberately published nothing and a slot nobody touched are the same value:

Measured, release:

```
(3) zero-length published payload is indistinguishable from untouched
    untouched.is_empty()          = true
    published_nothing.is_empty()  = true
    equal                         = true
    TypedSlot<()> after set(())   is_empty = false
```

The last line is the contrast: `TypedSlot< () >` holds `Some( () )` after a set,
so a payload-free signal *is* distinguishable there — the `Option`'s discriminant
carries the distinction that `BytesSlot`'s length cannot.

This limitation is documented, thoroughly, and in the wrong crate:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A11 -F '  /// # A `BytesSlot` cannot distinguish empty from zero-length' ring_event/src/lib.rs
# and the same search against the crate that owns the type
grep -i 'zero.length\|zero_length\|distinguish' ring_slot/src/lib.rs \
  || echo "  ring_slot: never mentioned"
```

Live output:

```
  /// # A `BytesSlot` cannot distinguish empty from zero-length
  ///
  /// A [`BytesSlot`] records a length and nothing more, so a deliberately
  /// published zero-byte payload reads back as `None` — identical to a slot
  /// nobody has touched. This is a real limitation, not an oversight, and it is
  /// not worth a flag byte per slot to remove: the ring already carries the
  /// distinction, in the published-sequence handshake, and a caller that needs
  /// "somebody published nothing" must read it there rather than from the slot.
  /// A [`TypedSlot<()>`](TypedSlot) does not share the limitation, and is the
  /// cheaper way to send a payload-free signal.
  fn peek( &self ) -> Option< Self::Out< '_ > >;
}
// cleared slot's former payload and distinguish two slots no caller can tell
```

**Finding.** `ring_event::Peek` carries a complete, well-argued statement of a
limitation that belongs to `ring_slot::BytesSlot` — it names the cause (a length
and nothing more), prices the alternative (a flag byte per slot), points at where
the distinction actually lives (the published-sequence handshake), and names the
cheaper substitute (`TypedSlot< () >`). It is the best paragraph written about
this crate and it is two crates away from the type it describes.

`ring_slot` itself never uses the words. A caller reading `BytesSlot`'s
documentation to decide whether it can carry a payload-free signal finds
`is_empty`, `len`, and `write`, all behaving exactly as the limitation predicts,
and no sentence telling them the conflation exists. They reach `ring_event::Peek`
only if they happen to use `ring_event`.

This is a leaf-proximate placement failure: the documentation sits with a
consumer instead of with the type that owns the behaviour. Its own test suite
confirms the placement is known to be wrong — `ring_event`'s manual plan records
that a first pass found the limitation asserted only in `tests/` and moved it
into `src/`, which fixed the *depth* of the placement without moving it to the
right crate.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`decisions/001`](001_two_shapes_rather_than_one.md) | The ruling above this one — two shapes, and the trait that keeps them interchangeable |
| [`invariant/002`](../invariant/002_the_two_emptiness_paths_agree.md) | The emptiness property that does hold, across both access paths |
| [`pitfall/002`](../pitfall/002_clear_forgets_it_does_not_erase.md) | What lies behind the length after a clear |
| [`data_structure/001`](../data_structure/001_sixteen_bytes_to_carry_eight.md) | What the `usize` length costs in layout terms |

### Sources

| Fact | Where |
|------|-------|
| The `MaybeUninit` rejection | `ring_slot/src/lib.rs:22-24` |
| The two-field struct | `ring_slot/src/lib.rs:237-241` |
| `empty()` zeroes the array | `ring_slot/src/lib.rs:278-281` |
| `write` sets length to the payload's | `ring_slot/src/lib.rs:347-356` |
| The limitation, documented downstream | `ring_event/src/lib.rs:105-116` |
| The limitation, absent upstream | `ring_slot/src/lib.rs` — no occurrence |
| The zero-length measurement | Release probe, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `a_zero_length_write_empties_the_slot` | The behaviour SL8 describes, asserted without naming its consequence |
| `a_zero_capacity_slot_accepts_only_nothing` | The degenerate `N == 0` case, where every slot is permanently empty |
| `ring_event` — the `Peek` doctest | The limitation, exercised where it is documented |
| *(to create)* | A `ring_slot` doctest on `BytesSlot` stating the conflation where the type is defined |
