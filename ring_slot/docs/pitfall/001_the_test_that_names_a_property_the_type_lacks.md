# Pitfall: The Test That Names a Property the Type Lacks

### Scope

**Purpose:** Record that `slots_compare_by_payload_not_by_tail` named a property
`BytesSlot` did not have, that it passed for years because the fixture it built
could not expose the difference, and that the gap was closed by giving the type
the property rather than by renaming the test.

**Responsibility:** The equality behaviour of `BytesSlot< N >`, and the one test
in the suite that claims to pin it.

**In Scope:** `slots_compare_by_payload_not_by_tail` in
`ring_slot/tests/slot_test.rs`; the `#[ derive ]` line above
`pub struct BytesSlot` and the hand-written `Debug`/`PartialEq`/`Eq` impls below
it, in `ring_slot/src/lib.rs`. Addressed by name rather than by line —
SL42's own fix moved every address this section used to carry.

**Out of Scope:** `TypedSlot`'s equality, which compares an `Option< T >` and has
no tail to disagree about. What `clear` leaves behind is
[`pitfall/002`](002_clear_forgets_it_does_not_erase.md); this instance is about
comparison alone.

---

## The Test, and What It Used to Say

```sh
cd "$(git rev-parse --show-toplevel)"
# anchored on the function's own name, not on a sentence in its doc comment:
# the comment is the thing findings here get applied to, so quoting it by text
# would freeze this block the moment it is corrected
awk '
  /^\/\/\//                                        { buf = buf $0 "\n"; next }
  /^#\[ test \]$/                                  { pre = buf; buf = ""; next }
  /^fn slots_compare_by_payload_not_by_tail\(\)$/  { printf "%s#[ test ]\n", pre; f = 1 }
  f                                                { print }
  f && /^\}$/                                      { exit }
                                                   { buf = "" }
' ring_slot/tests/slot_test.rs
```

Live output:

```
/// Two slots holding the same payload compare equal even when their backing
/// arrays differ — which is the whole point, and is what a derived `PartialEq`
/// would get wrong. The first fixture pair is written once each from empty, so
/// their tails coincide and it could not distinguish the two relations; the
/// second is the case that can, and the one this test's name promises. See
/// `pitfall/001` SL41 and SL42.
#[ test ]
fn slots_compare_by_payload_not_by_tail()
{
  let mut written_once = BytesSlot::< 8 >::empty();
  written_once.write( b"ab" ).unwrap();

  let mut also_written_once = BytesSlot::< 8 >::empty();
  also_written_once.write( b"ab" ).unwrap();

  assert_eq!( written_once, also_written_once );
  assert_eq!( written_once.read(), also_written_once.read() );

  // Same payload, different tail: `bytes` is [ 97, 98, 88, 88, 88, 88, 88, 88 ]
  // here against [ 97, 98, 0, 0, 0, 0, 0, 0 ] above. A derived comparison reads
  // all eight and reports these unequal; the hand-written one reads `read()`.
  let mut overwritten = BytesSlot::< 8 >::empty();
  overwritten.write( b"XXXXXXXX" ).unwrap();
  overwritten.write( b"ab" ).unwrap();

  assert_eq!( overwritten.read(), written_once.read(), "indistinguishable through every accessor" );
  assert_eq!( overwritten, written_once, "and therefore indistinguishable through `==`" );
}
```

The second fixture pair is the one this instance is about, and until SL41 and
SL42 were acted on it did not exist. The test held two slots, both named for a
distinction it never built: both were constructed by `empty()` and written
exactly once from the same payload, so their backing arrays were byte-for-byte
identical — `[ 97, 98, 0, 0, 0, 0, 0, 0 ]` in each — and the assertion passed for
a reason that had nothing to do with the property the name claimed. The variable
called `overwritten` was never overwritten. It is now, and the assertion below it
is the one that would have failed.

---

### SL41 — The Comparison Reads the Tail, Which Is What the Test Denies

`BytesSlot` derived `PartialEq` when this was found. A derive compares every
field, and `bytes` is `[ u8; N ]` — all `N` of them, including the `N - len`
bytes past the payload that no reader can reach through `read()`. So two slots
agreed only when their *unreachable* tails also agreed. Build the fixture the
test's name describes — one slot that actually was overwritten, `bytes` of
`[ 97, 98, 88, 88, 88, 88, 88, 88 ]` against `[ 97, 98, 0, 0, 0, 0, 0, 0 ]`, both
reading `[ 97, 98 ]` — and the assertion inverted. Two slots indistinguishable
through every public accessor the type has, `read`, `len`, `capacity` and
`is_empty` alike, compared unequal.

That is no longer what the type does:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '--- what BytesSlot derives ---'
command grep -m1 -A5 -F '#[ derive( Clone ) ]' ring_slot/src/lib.rs
echo '--- and what it does not: the comparison, written out ---'
command grep -m1 -A6 -F 'impl< const N : usize > PartialEq for BytesSlot< N >' ring_slot/src/lib.rs
```

Live output:

```
--- what BytesSlot derives ---
#[ derive( Clone ) ]
pub struct BytesSlot< const N : usize >
{
  bytes : [ u8; N ],
  len : usize,
}
--- and what it does not: the comparison, written out ---
impl< const N : usize > PartialEq for BytesSlot< N >
{
  fn eq( &self, other : &Self ) -> bool
  {
    self.read() == other.read()
  }
}
```

**Finding.** `slots_compare_by_payload_not_by_tail` was named for, documented as,
and understood to assert a property `BytesSlot` did not have. It passed because
its two fixtures were constructed identically, so the tail it claimed to be
insensitive to was the same in both. The suite contained no case where the two
paths could disagree, which is why the gap survived.

**Disposition:** applied — both halves, the test and the type. `PartialEq` is no
longer derived: it is written out as `self.read() == other.read()`, so the
relation the test's name promises is the relation the type has, and `Eq` is
written out beside it rather than derived so it cannot silently re-acquire the
field-wise bound. The test no longer stops at the fixture that cannot tell the
two apart — it keeps that pair, then builds the second one this finding
described, an eight-byte write overwritten by a two-byte write, and asserts both
that the two read identically and that they compare equal. Its doc comment says
which pair does which work and why the first alone was not enough. Falsified by
putting the field-wise relation back — `self.bytes == other.bytes && self.len ==
other.len`, which is what the derive emits — and re-running the suite: 2 of 26
fail, `slots_compare_by_payload_not_by_tail` at
`and therefore indistinguishable through ==` and
`a_cleared_slot_is_indistinguishable_from_a_fresh_one` at
`a cleared slot is a fresh slot's value`. Both panics print their two operands
identically, which is the finding restated by the failure itself: `Debug` reads
`read()`, the mutated `PartialEq` reads the array, and the values differ only
where nothing can look.
Now prints: `self.read() == other.read()`

---

### SL42 — Equality Is Not the Relation the Rest of the Crate Uses

Every observation the crate offers about a `BytesSlot` goes through `read()`,
which is length-bounded — and since this finding was acted on, that now includes
the two that did not:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '--- read, the length bound every accessor is built on ---'
command grep -m1 -A3 -F 'pub fn read( &self ) -> &[ u8 ]' ring_slot/src/lib.rs
echo '--- Debug, now built on the same one ---'
command grep -m1 -A6 -F 'impl< const N : usize > core::fmt::Debug for BytesSlot< N >' ring_slot/src/lib.rs
echo '--- and the test that pins what a cleared slot prints ---'
command grep -m1 -A12 -F 'fn a_cleared_slot_is_indistinguishable_from_a_fresh_one()' ring_slot/tests/slot_test.rs \
  | command grep -E 'assert'
```

Live output:

```
--- read, the length bound every accessor is built on ---
  pub fn read( &self ) -> &[ u8 ]
  {
    &self.bytes[ ..self.len ]
  }
--- Debug, now built on the same one ---
impl< const N : usize > core::fmt::Debug for BytesSlot< N >
{
  fn fmt( &self, f : &mut core::fmt::Formatter< '_ > ) -> core::fmt::Result
  {
    f.debug_struct( "BytesSlot" ).field( "payload", &self.read() ).finish()
  }
}
--- and the test that pins what a cleared slot prints ---
  assert!( cleared.is_empty() );
  assert_eq!( cleared.read(), fresh.read() );
  assert_eq!( cleared, fresh, "a cleared slot is a fresh slot's value" );
  assert_eq!( format!( "{cleared:?}" ), "BytesSlot { payload: [] }" );
  assert!( !format!( "{cleared:?}" ).contains( "115" ), "no byte of `secret` survives into the printed form" );
```

`read`, `len`, `is_empty` and `capacity` all agree that the slot *is* its first
`len` bytes. When this was found, `PartialEq` disagreed and `Debug` with it: the
derived rendering printed the whole array, so a slot written with `secret` and
then cleared printed
`BytesSlot { bytes: [115, 101, 99, 114, 101, 116, 0, 0], len: 0 }` — the byte
string `secret`, out of a slot whose `is_empty()` was `true` and whose `read()`
was `[]`.

**Finding.** The crate had two inconsistent notions of what a `BytesSlot` *is*:
the length-bounded one that every accessor implemented, and the whole-array one
that the two derives implemented. Nothing in the source named the split, and the
one test positioned to catch it asserted the opposite. A caller comparing slots —
in an assertion, a dedup, a change-detection check — got the array relation while
reading the source for the accessor relation.

The fix was a choice, not a bug report: either hand-write `PartialEq` and `Debug`
over `read()`, or say in the type's own documentation that equality includes the
tail and that `clear` does not erase. The first is what the test already claimed
was true; the second was what the code did.

**Disposition:** applied — the first of the two, because the second would have
required every caller to learn a rule no other accessor follows. `Debug` and
`PartialEq` are both hand-written over `read()` and `Eq` is written out beside
them, so all six observations the type offers now report the same value, and the
split this finding names no longer exists. The tail is still there — that is
[`lifecycle/002`](../lifecycle/002_a_slot_across_a_rings_laps.md) SL32's subject
and it is unchanged — but nothing safe can read it, which is the stronger
property: a `BytesSlot` in a log line or an `assert_eq!` cannot disclose a
previous lap's payload. The cost is that `Clone` still copies all `N` bytes,
tail included, and no test pins that. Falsified by putting the field-wise
rendering back — `.field( "bytes", &&self.bytes[ .. ] ).field( "len",
&self.len )`, which is what the derive emits — and re-running the suite:
`a_cleared_slot_is_indistinguishable_from_a_fresh_one` fails printing
`BytesSlot { bytes: [115, 101, 99, 114, 101, 116, 0, 0], len: 0 }` against the
`BytesSlot { payload: [] }` it asserts.
Now prints: `f.debug_struct( "BytesSlot" ).field( "payload", &self.read() ).finish()`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`pitfall/002`](002_clear_forgets_it_does_not_erase.md) | The other half of the same relation: `clear` leaves the payload in `bytes`, which used to make a cleared slot compare unequal to a fresh one |
| [`non_functional_requirement/002`](../non_functional_requirement/002_four_traits_and_what_they_compare.md) | The full four-trait census across both shapes, and what each one actually reads |
| [`data_structure/001`](../data_structure/001_sixteen_bytes_to_carry_eight.md) | Why there is a tail at all — the fixed array is the allocation decision the ring was chosen for |
| [`invariant/001`](../invariant/001_a_read_returns_what_was_written.md) | The property that *is* upheld, asserted at every length from empty to full |

### Sources

| Fact | Where |
|------|-------|
| The test as written | `slots_compare_by_payload_not_by_tail` in `ring_slot/tests/slot_test.rs` |
| What `BytesSlot` derives, and what it no longer does | the `#[ derive ]` above `pub struct BytesSlot`, quoted above |
| The comparison, written out | `impl< const N : usize > PartialEq for BytesSlot< N >`, quoted above |
| `read`'s length bound | `BytesSlot::read` in `ring_slot/src/lib.rs`, quoted above |
| The counterexample and the historical `Debug` output | the two mutation runs recorded in the dispositions above |

### Tests

| Test | Covers |
|------|--------|
| `slots_compare_by_payload_not_by_tail` | Both fixtures: the pair that cannot distinguish the two relations, then the overwritten-versus-fresh pair that can — the case SL41 named and SL41's disposition added |
| `a_cleared_slot_is_indistinguishable_from_a_fresh_one` | The same relation over a cleared slot, through `==` and through the printed form together — the case SL42's disposition added |
| `a_read_never_returns_the_unused_tail` | The length-bounded relation, at every length from 0 to capacity — this one was always sound |
| `a_shorter_write_does_not_leak_the_longer_one` | The overwrite case, checked through `read()` |
| *(to create)* | A clone of a slot with a tail, asserting the tail travels with it — `Clone` is still derived over all `N` bytes and nothing pins it |
