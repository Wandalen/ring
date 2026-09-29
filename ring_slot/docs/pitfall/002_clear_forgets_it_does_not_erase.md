# Pitfall: Clear Forgets, It Does Not Erase

### Scope

**Purpose:** Record that `Slot::clear`'s contract once said the slot drops what
it held, that `TypedSlot` does drop it and `BytesSlot` does not, and that the
difference used to be observable through two derived traits — the two this
instance's own findings caused to be written by hand.

**Responsibility:** What each shape's `clear` actually does to the payload, and
what remains reachable afterwards.

**In Scope:** the `Slot::clear` contract, `TypedSlot`'s impl of it and
`BytesSlot`'s, in `ring_slot/src/lib.rs`. Addressed by name rather than by
line — SL43's and SL44's own fixes rewrote the contract and moved every address
this section used to carry.

**Out of Scope:** The equality relation itself, argued in
[`pitfall/001`](001_the_test_that_names_a_property_the_type_lacks.md). This
instance is about what survives a `clear`; that one is about what a comparison
reads.

---

## One Contract, Two Implementations

```sh
cd "$(git rev-parse --show-toplevel)"
echo '--- the contract ---'
command grep -m1 -A9 -F '  /// Return the slot to its empty state.' ring_slot/src/lib.rs
echo '--- TypedSlot ---'
command grep -m1 -A6 -F '    self.0.is_none()' ring_slot/src/lib.rs | tail -n 4
echo '--- BytesSlot ---'
command grep -m1 -A6 -F '    Self::is_empty( self )' ring_slot/src/lib.rs | tail -n 4
```

Live output:

```
--- the contract ---
  /// Return the slot to its empty state.
  ///
  /// **Not a promise to overwrite.** For a shape that owns what it stores
  /// (`TypedSlot`), the old value's destructor runs, so nothing survives the
  /// call. For a shape that stores by copying into fixed storage
  /// (`BytesSlot`), the bytes are not zeroed — only the length that marks
  /// them unreachable through this trait's own API moves. Both are "empty"
  /// by [`Slot::is_empty`]; only one is empty in memory.
  ///
  /// **How long the residue lasts.** For `BytesSlot`, until a write of at
--- TypedSlot ---
  fn clear( &mut self )
  {
    self.0 = None;
  }
--- BytesSlot ---
  fn clear( &mut self )
  {
    self.len = 0;
  }
```

`TypedSlot` assigns `None` over the `Option`, which drops the `T` — for a
`String`, the allocation is freed at that line. `BytesSlot` assigns `0` to a
`usize`. Nothing is dropped, because nothing owns anything: the payload is
already inline in a `[ u8; N ]` that outlives every write. The array keeps
whatever was last copied into it. The contract quoted above says that in its own
words — but it did not when this instance was written. It read, in full, "return
the slot to its empty state, dropping whatever it held", which is what SL43 is
about.

---

### SL43 — A Cleared Slot Was Not Equal to a Fresh One

The case the finding names, run as a test:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A12 -F 'fn a_cleared_slot_is_indistinguishable_from_a_fresh_one()' ring_slot/tests/slot_test.rs \
  | command grep -E 'write|clear|assert'
```

Live output:

```
fn a_cleared_slot_is_indistinguishable_from_a_fresh_one()
  let mut cleared = BytesSlot::< 8 >::empty();
  cleared.write( b"secret" ).unwrap();
  cleared.clear();
  assert!( cleared.is_empty() );
  assert_eq!( cleared.read(), fresh.read() );
  assert_eq!( cleared, fresh, "a cleared slot is a fresh slot's value" );
  assert_eq!( format!( "{cleared:?}" ), "BytesSlot { payload: [] }" );
  assert!( !format!( "{cleared:?}" ).contains( "115" ), "no byte of `secret` survives into the printed form" );
```

```text
cargo nextest run -p ring_slot a_cleared_slot_is_indistinguishable_from_a_fresh_one
```

The slot is written with `b"secret"` and then cleared. Every accessor agrees it
is empty, and so — now — do `Debug` and `==`. When this was found neither did:
`Debug` printed
`BytesSlot { bytes: [115, 101, 99, 114, 101, 116, 0, 0], len: 0 }`, the six bytes
of `secret` verbatim, and `PartialEq` distinguished the slot from one that never
held anything. This test is the disposition of SL42 in
[`pitfall/001`](001_the_test_that_names_a_property_the_type_lacks.md); it did not
exist when SL43 was written, and it fails if either hand-written impl is put back
on the derive list.

**Finding.** `Slot::clear`'s documented contract — "return the slot to its empty
state, dropping whatever it held" — was met by `TypedSlot` and only half met by
`BytesSlot`. The slot returned to its empty *state*; what it held was not
dropped, zeroed, or made unreachable. It stayed in the array, printable by
`Debug` and significant to `PartialEq`, until the next write of at least that
length overwrote it byte by byte.

That is the correct implementation for the crate's purpose. Zeroing `N` bytes on
every clear would put a memset on the drain path of a ring whose whole point is
that slots are allocated once and reused — and the crate says exactly that about
capacity ("a ring's slots are allocated once, so a slot that could grow would
defeat the allocation behaviour the ring was chosen for"). The finding was not
that the code is wrong. It was that the trait's contract promised a stronger
thing than one of its two implementors delivered, and neither the trait's doc nor
`BytesSlot`'s said so. Both now do: the contract quoted at the top of this
instance opens by denying the promise — "**Not a promise to overwrite.**" — names
which shape drops and which does not, and closes on the one sentence that
separates them, "Both are 'empty' by `Slot::is_empty`; only one is empty in
memory."

---

### SL44 — The Residue Outlives the Publish That Produced It

A ring reuses slot `i` on every lap. Combining that with the above: after a
consumer drains lap 0's payload from slot `i` and the drain calls `clear`, the
payload is still in the slot's array when the producer claims slot `i` for lap 1.
It is overwritten only to the extent the new payload is long enough to cover it.

```sh
cd "$(git rev-parse --show-toplevel)"
# every place the family clears a ring_slot slot
# sorted: grep is shimmed to a parallel ugrep here, so hits arrive in completion order
command grep -r 'Slot::clear\|slot\.clear()' ring_*/src/*.rs \
  | command grep -v '^ring_slot/' | LC_ALL=C sort
```

Live output:

```
ring_store/src/lib.rs:      slot.clear();
ring_store/src/lib.rs:  /// emptiness delivered is only as strong as `Slot::clear` for the shape in
ring_event/src/lib.rs:  slot.clear();
```

Two call sites, both generic over `Slot` and neither able to distinguish the two
implementations — `ring_store::Buffer::clear` sweeps every slot in the buffer,
`ring_event::recycle` clears one. Whichever shape is instantiated decides whether
the payload is dropped or merely forgotten, and both call sites are written
against the contract that says it is dropped. The lap boundary is walked in full
in [`lifecycle/002`](../lifecycle/002_a_slot_across_a_rings_laps.md).

The crate's own test suite already recognises the shape of this risk. Its module
comment says a partially-filled slot leaking its previous contents "would be a
data leak across ring laps", and `a_shorter_write_does_not_leak_the_longer_one`
asserts that it does not — *through `read()`*. That assertion is sound and this
finding does not weaken it: no caller using `read()` can see the residue.

**Finding.** The suite established that the residue is unreachable through
`read()`, and stopped there. It was reachable through `Debug` and significant to
`PartialEq`, both derived, both public, and neither considered by the test that
frames the leak question. A `BytesSlot< N >` in a long-lived ring holds, at any
moment, the longest payload ever written to that slot position, not the current
one — and a diagnostic dump, a snapshot comparison, or an assertion failure
message would print it.

The cheap statement that would close this is a sentence on `BytesSlot`: *clear
resets the length; the bytes remain until overwritten, and `Debug` and
`PartialEq` see them.* The expensive fix is hand-written `Debug` and `PartialEq`
over `read()`, which costs nothing at runtime and makes the two traits agree with
the four accessors.

**Disposition:** applied — the expensive one, because it was not expensive.
`Debug` and `PartialEq` are written by hand over `read()` and `Eq` is written out
beside them, so `BytesSlot` no longer has any public way to observe a byte past
its own length: the six observations the type offers — `read`, `len`,
`is_empty`, `capacity`, `==`, and the printed form — now report one value.
`a_cleared_slot_is_indistinguishable_from_a_fresh_one` asserts both halves in one
body, that a slot written with `b"secret"` and cleared equals a fresh one and
that its printed form contains no `115`, and it is what fails if either impl goes
back on the derive list — measured: reverting `Debug` to the field-wise rendering
fails it with
`BytesSlot { bytes: [115, 101, 99, 114, 101, 116, 0, 0], len: 0 }` against the
`BytesSlot { payload: [] }` it expects. The cheap statement was applied too
rather than instead, because unreachable is not the same as gone: `Slot::clear`'s
contract now carries a "**How long the residue lasts.**" paragraph
([`lifecycle/002`](../lifecycle/002_a_slot_across_a_rings_laps.md) SL32) saying
the bytes stay in the process's memory until a write of at least that length
lands on the same slot. What is declined is zeroing on `clear`, which would put a
memset of `N` bytes on the drain path this shape exists to keep clear of one. The
residue that remains is `Clone`'s: it is still derived, still copies all `N`
bytes, and no test pins that.
Now prints: `assert_eq!( format!( "{cleared:?}" ), "BytesSlot { payload: [] }" );`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`pitfall/001`](001_the_test_that_names_a_property_the_type_lacks.md) | The same two traits, seen from the comparison side; the test that used to deny this behaviour, and now asserts it |
| [`lifecycle/002`](../lifecycle/002_a_slot_across_a_rings_laps.md) | The lap boundary where the residue outlives its publish, and the contract paragraph that times it |
| [`invariant/002`](../invariant/002_the_two_emptiness_paths_agree.md) | Emptiness after `clear` is consistent across both paths — the part that always held |
| [`non_functional_requirement/002`](../non_functional_requirement/002_four_traits_and_what_they_compare.md) | What each of the four traits on each shape actually reads, and which are still derived |

### Sources

Addressed by content rather than by line number throughout — SL43's fix rewrote
the trait's contract and SL44's added three impls below the struct, which moved
every address this table used to carry.

| Fact | Where |
|------|-------|
| The trait's contract | the doc line `/// Return the slot to its empty state.` and the two bold paragraphs under it, quoted above |
| `TypedSlot::clear` drops | the `clear` body reading `self.0 = None;`, quoted above |
| `BytesSlot::clear` forgets | the `clear` body reading `self.len = 0;`, quoted above |
| The capacity argument for a fixed array | the doc line opening `/// whoever knows the wire format. Fixed capacity because a ring's slots are` |
| The suite's own leak framing | the module comment of `ring_slot/tests/slot_test.rs` |
| The `secret` residue, and that nothing safe now reads it | `a_cleared_slot_is_indistinguishable_from_a_fresh_one`, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `clearing_a_bytes_slot_empties_the_reading` | That `clear` empties the *reading* — the half that always held |
| `clearing_a_typed_slot_is_idempotent` | `TypedSlot`'s clear, which does drop |
| `clearing_a_typed_slot_runs_the_payloads_destructor` | That it drops, counted rather than assumed — the half of the contract `BytesSlot` cannot deliver |
| `a_shorter_write_does_not_leak_the_longer_one` | That the residue is unreachable through `read()` |
| `a_cleared_slot_is_indistinguishable_from_a_fresh_one` | A cleared slot compared against a fresh one, and its printed form — the case SL43 names and SL44's disposition closed |
| *(to create)* | A clone of a slot with a tail, asserting the tail travels with it — `Clone` is still derived over all `N` bytes |
