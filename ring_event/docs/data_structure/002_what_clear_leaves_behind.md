# Data Structure: What `clear` Leaves Behind

### Scope

**Purpose:** Record that the third of the crate's three operations means two
different things depending on the shape it is called on, that the contract it
forwards states only one of them, and that the suite is shaped so as not to
notice.

**Responsibility:** `Slot::clear`'s stated contract, its two impls, what each
actually leaves in memory, and what the crate's own recycle tests assert.

**In Scope:** `ring_event/src/lib.rs:214-218`, `:229-234`;
`ring_slot/src/lib.rs:46-66`, `:183-186`, `:392-395`;
`ring_event/tests/event_test.rs:177-190`, `:202-206`.

**Out of Scope:** Why this crate cannot state the difference itself is
[`data_structure/001`](001_a_crate_that_declares_no_data.md). The other two
operations' divergences are [`api/002`](../api/002_a_result_one_impl_can_never_return.md).

---

## One Contract, Two Meanings

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the contract recycle delivers, as ring_slot states it --'
command grep -m1 -A3 -F '  /// Return the slot to its empty state.' ring_slot/src/lib.rs
echo '  -- and the two impls behind it --'
awk '/^    self\.0\.is_none\(\)$/{ n1 = NR } n1 && NR >= n1 + 3 && NR <= n1 + 6 { print } /^    Self::is_empty\( self \)$/{ n2 = NR } n2 && NR >= n2 + 3 && NR <= n2 + 6 { print }' ring_slot/src/lib.rs
echo '  -- recycle, its rationale and its body --'
sed -n '/^\/\/\/ Empty `slot` through the shared path\.$/,/^\/\/\/ the identical-path claim does not cover\.$/p;/^pub fn recycle< S >( slot : &mut S )$/,/^}$/p' ring_event/src/lib.rs
echo '  -- everything the suite asserts after recycling both shapes --'
command grep -m1 -A4 -F '  recycle( &mut bytes );' ring_event/tests/event_test.rs
echo '  -- and the shape the one ring_shutdown-facing test uses --'
command grep -m1 -A2 -F '  // The property `ring_shutdown`'"'"'s reset depends on: a slot cleared during' ring_event/tests/event_test.rs
echo '  -- whether a ring buffer can hold the other shape --'
command grep 'Buffer< BytesSlot' ring_store/src/lib.rs ring_store/tests/buffer_test.rs
```

Live output:

```
  -- the contract recycle delivers, as ring_slot states it --
  /// Return the slot to its empty state.
  ///
  /// **Not a promise to overwrite.** For a shape that owns what it stores
  /// (`TypedSlot`), the old value's destructor runs, so nothing survives the
  -- and the two impls behind it --
  fn clear( &mut self )
  {
    self.0 = None;
  }
  fn clear( &mut self )
  {
    self.len = 0;
  }
  -- recycle, its rationale and its body --
/// Empty `slot` through the shared path.
///
/// The third of the three operations a ring performs on a slot, here for the
/// same reason as the other two: a shape-specific reset would be a fourth path
/// the identical-path claim does not cover.
pub fn recycle< S >( slot : &mut S )
where
  S : Slot,
{
  slot.clear();
}
  -- everything the suite asserts after recycling both shapes --
  recycle( &mut bytes );

  assert!( typed.is_empty(), "the third call is the one that empties it" );
  assert!( bytes.is_empty(), "for both shapes, through the same generic body" );
}
  -- and the shape the one ring_shutdown-facing test uses --
  // The property `ring_shutdown`'s reset depends on: a slot cleared during
  // recycling must not hand the next lap the previous world's payload.
  let mut buffer : Buffer< TypedSlot< u32 > > = Buffer::new( cap( 2 ) );
  -- whether a ring buffer can hold the other shape --
ring_store/src/lib.rs:  /// let buffer : Buffer< BytesSlot< 8 > > = Buffer::new( Capacity::new( 16 ).unwrap() );
ring_store/tests/buffer_test.rs:  let mut bytes : Buffer< BytesSlot< 8 > > = Buffer::new( cap( 4 ) );
```

---

## What Each Shape Actually Retains

*The recordings below predate `BytesSlot`'s `Debug`/`PartialEq` rewrite. The
bytes still survive exactly as shown; what changed is that no public surface
renders or compares them any more, so the `BytesSlot { bytes: … }` lines and
both `equal false` readings are historical — see EV12's Disposition.*

A probe against the real crates. It recycles a typed slot holding a value with a
`Drop` impl, then recycles a byte slot holding eight bytes and inspects what is
left — first directly, then after a shorter write, against a slot that never held
anything else.

```rust
// -ev_probe/src/bin/recycled_bytes.rs
let mut typed = TypedSlot::empty();
publish_into( &mut typed, Noisy( "the payload" ) ).unwrap();
recycle( &mut typed );

let mut used = BytesSlot::< 8 >::empty();
publish_into( &mut used, &b"secret!!"[ .. ] ).unwrap();
recycle( &mut used );

publish_into( &mut used, &b"hi"[ .. ] ).unwrap();
let mut fresh = BytesSlot::< 8 >::empty();
publish_into( &mut fresh, &b"hi"[ .. ] ).unwrap();
```

```
  -- typed shape: does recycle run the destructor? --
    recycling now
    dropped: the payload
    recycled
  -- byte shape: what is left in the slot after recycle? --
    after recycle, is_empty  true
    after recycle, debug     BytesSlot { bytes: [115, 101, 99, 114, 101, 116, 33, 33], len: 0 }
  -- and after the next, shorter write --
    recycled slot            BytesSlot { bytes: [104, 105, 99, 114, 101, 116, 33, 33], len: 2 }
    fresh slot               BytesSlot { bytes: [104, 105, 0, 0, 0, 0, 0, 0], len: 2 }
    read() equal             true
    slots equal              false
```

And the same question asked of the suite's own fixture — the byte slot
`recycling_empties_either_shape_through_the_same_call` recycles, compared against
a slot that never held anything:

```rust
// -ev_probe/src/bin/one_line_assert.rs
let mut bytes = BytesSlot::< 8 >::empty();
publish_into( &mut bytes, &b"x"[ .. ] ).unwrap();
recycle( &mut bytes );
```

```
  recycled  BytesSlot { bytes: [120, 0, 0, 0, 0, 0, 0, 0], len: 0 }
  empty     BytesSlot { bytes: [0, 0, 0, 0, 0, 0, 0, 0], len: 0 }
  equal     false
```

---

### EV11 — `Slot::clear`'s Contract Says "Dropping Whatever It Held" and One of Its Two Impls Drops Nothing

*Written before both rewrites this section rests on. The contract half is
recorded in this section's own Disposition below; the observability half —
`Debug` and `PartialEq` no longer deriving over the fields — in EV12's.*

The trait declares one sentence of contract at `ring_slot:46`: "Return the slot
to its empty state, dropping whatever it held."

For `TypedSlot< T >` that is exactly right. `self.0 = None` overwrites the
`Option`, the old value's destructor runs, and the probe's first section is the
demonstration — `dropped: the payload` prints between "recycling now" and
"recycled", inside `clear`.

For `BytesSlot< N >` it is false. `self.len = 0` writes one `usize`. The array is
untouched: `115, 101, 99, 114, 101, 116, 33, 33` — `secret!!`, byte for byte,
still resident in a slot that now reports `is_empty()`. Nothing was dropped
because a `[ u8; N ]` has nothing to drop, and nothing was overwritten because
`clear` does not overwrite.

**Finding.** A contract stated on the trait is true for one implementor and false
for the other, and the false half is the one where it would matter. "Dropping
whatever it held" is precisely the phrasing a reader checks when asking whether a
recycled slot still contains the previous payload; for half the family's slot
shapes the answer is yes and the sentence says no.

The residue is observable, not merely present. `BytesSlot` derives `PartialEq`
over both fields, so a recycled slot rewritten with `b"hi"` compares *unequal* to
a fresh slot written with the same `b"hi"` — the probe's last two lines are the
same two slots reading equal through `read()` and comparing unequal as values.
`Debug` prints the whole array too, so a log line or an assertion failure will
show bytes the caller believed were gone.

Two accurate replacements exist and neither is expensive: state that `clear`
returns the slot to its empty state without guaranteeing the previous contents
are overwritten, or have `BytesSlot::clear` zero `bytes[ ..self.len ]` before
zeroing `len` and keep the sentence. The second costs one write of at most `N`
bytes on an operation that already touches the struct.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -A6 -F 'Not a promise to overwrite' ring_slot/src/lib.rs
```

Live output:

```
    /// **Not a promise to overwrite.** For a shape that owns what it stores
    /// (`TypedSlot`), the old value's destructor runs, so nothing survives the
    /// call. For a shape that stores by copying into fixed storage
    /// (`BytesSlot`), the bytes are not zeroed — only the length that marks
    /// them unreachable through this trait's own API moves. Both are "empty"
    /// by [`Slot::is_empty`]; only one is empty in memory.
    ///
```

**Disposition:** applied — `Slot::clear`'s trait doc comment in
`ring_slot/src/lib.rs` no longer states "dropping whatever it held" as
one contract true of both impls; it now states the cheaper of this instance's
two accurate replacements — that `clear` does not promise the previous
contents are overwritten — and names which shape actually drops (`TypedSlot`)
and which only moves a length (`BytesSlot`). Now prints: `Not a promise to
overwrite`

---

### EV12 — The Suite Recycles Both Shapes and Asserts Only the Thing That Cannot Distinguish Them

`recycle`'s rationale at `:216-218` is the strongest version of the crate's
central claim: a shape-specific reset "would be a fourth path the identical-path
claim does not cover." It is also the operation where the claim is least
defensible, because `recycle` returns nothing — with no value to compare, the two
shapes can diverge arbitrarily and the signature will not show it.

`recycling_empties_either_shape_through_the_same_call` is the test that would
catch it. It recycles a typed slot and a byte slot through the same call, and then
asserts `drain_from( … ) == None` for each. That assertion passes for the byte
shape for a reason unrelated to the bytes: `drain_from` forwards to `Peek`, which
for `BytesSlot` returns `None` when `is_empty()`, and `is_empty()` reads `len`.
The one field `clear` writes is the one field the assertion consults.

The same shape appears in the crate's `ring_shutdown`-facing test.
`a_recycled_storage_slot_stops_returning_the_previous_lap` states the dependent
property in a comment — "a slot cleared during recycling must not hand the next
lap the previous world's payload" — and then instantiates
`Buffer< TypedSlot< u32 > >`, the shape for which the property holds.
`Buffer< BytesSlot< 8 > >` is a supported configuration, constructed in
`ring_store`'s own doctest at `:84` and its suite at `:236`; in that
configuration the previous world's payload is still in the buffer after the
reset, and no test anywhere looks.

**Finding.** This is the crate's one-path claim failing in the place its own
documentation nominates as the reason the function exists, undetected by a test
written specifically to cover both shapes. The test is not wrong — it asserts
what the public API can observe, and by that measure the shapes do behave
identically. That is the trap: the observable surface is where the two agree, and
the disagreement lives one field below it, reachable through `Debug`, through
`PartialEq`, and through anything that reads the buffer's memory rather than its
API. Adding `assert_eq!( bytes, BytesSlot::< 8 >::empty() )` to the existing test
is one line, and the second probe above is that line's two operands —
`[ 120, 0, … ]` against `[ 0, 0, … ]`. It fails today.

That premise no longer holds. `ring_slot/src/lib.rs`'s `BytesSlot` `Debug` and
`PartialEq` are no longer derived over the fields; they are hand-written over
`read()`, so both agree with every other public observation instead of
distinguishing values recycling has made observationally identical:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -A3 'impl< const N : usize > PartialEq for BytesSlot' ring_slot/src/lib.rs
```

Live output:

```
impl< const N : usize > PartialEq for BytesSlot< N >
{
  fn eq( &self, other : &Self ) -> bool
  {
```

The exact one-line assertion this finding proposes is now permanently in the
suite — added to `recycling_empties_either_shape_through_the_same_call` —
and it passes rather than fails, which is the regression pin the finding
asked for, aimed at the opposite outcome the finding predicted:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -A1 -F 'assert_eq!( bytes, BytesSlot::< 8 >::empty()' ring_event/tests/event_test.rs
```

Live output:

```
  assert_eq!( bytes, BytesSlot::< 8 >::empty(), "a recycled byte slot equals a fresh one" );
}
```

**Disposition:** applied — `recycling_empties_either_shape_through_the_same_call`
in `ring_event/tests/event_test.rs` now carries the exact assertion
this finding recommends, pinning that a recycled byte slot equals a fresh one
through `BytesSlot`'s own `PartialEq` — the crate's own claim that `recycle`
is one identical path for both shapes, checked at the value level rather than
only through `is_empty`/`drain_from`. It passes because `ring_slot`'s
`Debug`/`PartialEq` were rewritten (outside this crate, verified above) to
compare via `read()` instead of the derived field-wise comparison this
finding was written against; the crate's test suite re-verified passing
(`cargo test -p ring_event --all-features`, 2026-09-04). Now prints: `a
recycled byte slot equals a fresh one`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`data_structure/001`](001_a_crate_that_declares_no_data.md) | Why this crate cannot state the difference itself |
| [`api/002`](../api/002_a_result_one_impl_can_never_return.md) | The other two operations' divergences, in the return type |
| [`algorithm/001`](../algorithm/001_two_executable_statements_and_one_branch.md) | `slot.clear()` as one of the crate's two statements |
| [`integration/002`](../integration/002_the_ring_writes_slots_without_this_crate.md) | Whether anything in production reaches this yet |

### Sources

| Fact | Where |
|------|-------|
| "dropping whatever it held" — the contract as it then read | `ring_slot/src/lib.rs:46`, since rewritten |
| `TypedSlot::clear` assigning `None` | `ring_slot/src/lib.rs:183-186` |
| `BytesSlot::clear` writing only `len` | `ring_slot/src/lib.rs:392-395` |
| The destructor running, and the bytes surviving | Probe above |
| A recycled byte slot comparing unequal to a fresh one, under the derived `PartialEq` | Probe above, since rewritten |
| `recycle`'s "a fourth path the identical-path claim does not cover" | `ring_event/src/lib.rs:217-218` |
| Both recycle assertions consulting only `len` | `ring_event/tests/event_test.rs:221-222` |
| `Buffer< BytesSlot< 8 > >` as a supported configuration | `ring_store/src/lib.rs:84`, `ring_store/tests/buffer_test.rs:236` |

### Tests

| Test | Covers |
|------|--------|
| `recycling_empties_either_shape_through_the_same_call` | Both shapes recycled; `drain_from`, plus value equality since EV12 |
| `a_recycled_storage_slot_stops_returning_the_previous_lap` | The `ring_shutdown` property, on the shape that holds it |
| `recycling_a_slot_that_was_never_published_is_harmless` | `clear` twice, typed shape only |
