# Pitfall: A Zero-Length Payload Reads as Nothing

### Scope

**Purpose:** Record the crate's one genuine behavioural trap — a byte payload of
length zero that reads back exactly like a slot nobody ever wrote to — and check
the two escapes the documentation offers against what a caller can actually
reach.

**Responsibility:** `Peek::peek`'s account of the limitation, `drain_from`'s
silence on it, the mechanism in `ring_slot::BytesSlot::write`, the test that pins
the behaviour, and the reachability of `TypedSlot< () >` and the
published-sequence handshake.

**In Scope:** `ring_event/src/lib.rs:109-125`, `:211`, `:192-206`;
`ring_slot/src/lib.rs:347-355`;
`ring_event/tests/event_test.rs:129-140`;
`ring_event/Cargo.toml`.

**Out of Scope:** The equality trap that the same residue creates is
[`pitfall/002`](002_a_recycled_slot_is_not_equal_to_an_empty_one.md). The
invariant this state is the exception to is
[`invariant/001`](../invariant/001_two_readings_of_one_emptiness.md).

---

## The Trap, Its Mechanism, and Its Two Escapes

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- what the read half documents about the ambiguity --'
command grep -m1 -A13 -F '  /// how a drain observes it.' ring_event/src/lib.rs | tail -n 13
echo '  -- and what the function a ring actually calls documents about it --'
command grep -m1 -B1 -A10 -F '/// Read `slot` — the one read path both shapes take.' ring_event/src/lib.rs
echo '  -- the mechanism, in the slot crate --'
command grep -m1 -A8 -F '  pub fn write( &mut self, payload : &[ u8 ] ) -> Result< (), RingError >' ring_slot/src/lib.rs
echo '  -- the first escape the doc names, everywhere in the workspace --'
command grep -r 'TypedSlot< () >\|TypedSlot::< () >' --include=*.rs */ | wc -l
echo '  -- the second escape, and the crates that carry it --'
for c in ring_*/; do
  n=$( command grep -rhc 'published' --include=*.rs "$c"src 2>/dev/null | awk '{ s += $1 } END { print s + 0 }' )
  [ "$n" -gt 10 ] && printf '  %-16s %s\n' "$( basename $c )" "$n"
done
echo '  -- against everything this crate is allowed to name --'
sed -n '/^\[dependencies\]/,/^\[/p' ring_event/Cargo.toml | command grep -o '^[a-z_]* =' | tr -d ' ='
```

Live output:

```
  -- what the read half documents about the ambiguity --
  ///
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
  -- and what the function a ring actually calls documents about it --

/// Read `slot` — the one read path both shapes take.
///
/// **This does not empty the slot, and the name is the trap.** It takes `&S`
/// and hands back a borrow, so after it returns the slot still reports
/// non-empty and still holds the payload. Freeing the slot for the next lap is
/// a separate, mandatory third call to [`recycle`] — omitting it is neither a
/// compile error nor a runtime error, and produces a drained ring every slot
/// of which reads occupied. The borrow is the point rather than an oversight:
/// `BytesSlot` cannot hand back an owned payload without copying it, so the
/// one signature that fits both shapes is the borrowing one, and consuming the
/// slot is factored out to the caller that knows when the read is finished.
  -- the mechanism, in the slot crate --
  pub fn write( &mut self, payload : &[ u8 ] ) -> Result< (), RingError >
  {
    if payload.len() > N
    {
      return Err( RingError::BatchTooLarge { requested : payload.len(), capacity : N } );
    }
    self.bytes[ ..payload.len() ].copy_from_slice( payload );
    self.len = payload.len();
    Ok( () )
  -- the first escape the doc names, everywhere in the workspace --
0
  -- the second escape, and the crates that carry it --
  ring_consume     27
  ring_mpsc        38
  ring_publish     19
  ring_spsc        23
  ring_stats       25
  ring_testkit     12
  -- against everything this crate is allowed to name --
ring_types
ring_slot
```

## Every State a Four-Byte Slot Can Be In

*The four rows below predate `BytesSlot`'s `Debug`/`PartialEq` rewrite (see
EV12's Disposition in
[`data_structure/002`](../data_structure/002_what_clear_leaves_behind.md)):
today's `Debug` prints only the written prefix, not the raw `bytes`/`len`
fields shown here. The `is_empty`/`peek` columns — including the
first-and-last-row identity this section's own prose relies on — do not
depend on `Debug` and are unaffected; only the literal `Debug` rendering in
this table is frozen.*

```rust
// -ev_probe/src/bin/slot_states.rs
// `show` prints the label, the slot's own `Debug`, `Slot::is_empty` and `drain_from`.
let mut slot = BytesSlot::< 4 >::empty();
show( "never written", &slot );

publish_into( &mut slot, &b"ab"[ .. ] ).unwrap();
show( "written, two bytes", &slot );

recycle( &mut slot );
show( "recycled", &slot );

let mut fresh = BytesSlot::< 4 >::empty();
publish_into( &mut fresh, &b""[ .. ] ).unwrap();
show( "written, zero bytes", &fresh );
```

```
    never written          BytesSlot { bytes: [0, 0, 0, 0], len: 0 }  is_empty true   peek None
    written, two bytes     BytesSlot { bytes: [97, 98, 0, 0], len: 2 }  is_empty false  peek Some([97, 98])
    recycled               BytesSlot { bytes: [97, 98, 0, 0], len: 0 }  is_empty true   peek None
    written, zero bytes    BytesSlot { bytes: [0, 0, 0, 0], len: 0 }  is_empty true   peek None
```

The first and last rows are byte-identical in every column. The zero-length
publish is not merely reported the same as an untouched slot — it produces the
same slot.

## What the Recommended Alternative Does

*The probe below is frozen at authoring time — the scratch crate that ran it
has since been swept — and no drift was found: `TypedSlot::< () >`,
`drain_from` and `Slot::is_empty`'s signatures are unchanged, and every
printed value is a plain `None`/`Some(())`/`bool`, uninvolved with
`BytesSlot`'s since-rewritten `Debug`/`PartialEq`. The recorded output
should still reproduce; it just cannot be re-run to confirm.*

```rust
// -ev_probe/src/bin/payload_free_signal.rs
// The escape the doc recommends for "somebody published nothing".
let mut unit = TypedSlot::< () >::empty();
println!( "    unit slot, fresh      {:?}  is_empty {}", drain_from( &unit ), Slot::is_empty( &unit ) );
publish_into( &mut unit, () ).unwrap();
println!( "    unit slot, published  {:?}  is_empty {}", drain_from( &unit ), Slot::is_empty( &unit ) );

// The shape it replaces, at the same signal.
let mut bytes = BytesSlot::< 8 >::empty();
publish_into( &mut bytes, &[][ .. ] ).unwrap();
println!( "    byte slot, published  {:?}  is_empty {}", drain_from( &bytes ), Slot::is_empty( &bytes ) );
```

```
    unit slot, fresh      None  is_empty true
    unit slot, published  Some(())  is_empty false
    byte slot, published  None  is_empty true
```

---

### EV41 — The Limitation Is Documented on the Trait Method and Absent From the Function a Ring Calls

`Peek::peek` handles this well. It gives the limitation its own `#` heading,
states it precisely, says it is deliberate, prices the alternative it rejected
("not worth a flag byte per slot"), and names two ways out. Thirteen of the
crate's one hundred and twenty-nine doc lines go to this one point. It is the
most careful passage in the file.

`drain_from` is the function that reads the slot. The module documentation
frames the free functions as the ring's entry points — "a ring's publish calls
this" — and `publish_into` spends twenty-three doc lines on why it exists. The
read half gets one sentence: "Read `slot` — the one read path both shapes take."
Nothing about the ambiguity.

And then the doctest under that sentence is:

```rust
let slot = BytesSlot::< 4 >::empty();
assert_eq!( drain_from( &slot ), None );
```

That is the byte shape, and the answer is `None`. It is the exact shape where
the ambiguity lives, showing the exact value the ambiguity produces, in the one
worked example a reader of this function sees — presented as the unambiguous
case, because in that example it is.

**Finding.** A caller reaches this crate through the three free functions; that
is the crate's own stated framing, and it is how the tests and the one external
consumer use it. `Peek::peek` is one implementation detail down from there, and
a reader who never opens the trait — which is most readers of a two-hundred-line
crate whose module doc says the interesting thing is the functions — will meet
the limitation for the first time when a zero-length publish silently vanishes
in their code. One cross-reference on `drain_from` fixes it: a sentence pointing
at [`Peek::peek`]'s section, or a second `assert` in the existing doctest
publishing an empty slice and getting `None` back, which would put the trap in
the same example that currently teaches the reading it invalidates.

---

### EV42 — Both Escapes the Documentation Names Are Outside What the Crate Can Reach

The doc offers two ways to send "somebody published nothing". Neither is
demonstrable from here.

The first is `TypedSlot< () >`, called "the cheaper way to send a payload-free
signal". The probe confirms it works exactly as advertised: fresh reads `None`,
published reads `Some(())`, and `is_empty` tracks the difference — the typed
shape carries occupancy in an `Option` rather than inferring it from a length,
so there is nothing to conflate. But the census finds `TypedSlot< () >`
constructed **zero times** anywhere in the workspace: not in this crate's
seventeen tests, not in its five doctests, not in any of the other thirty-two
ring crates. The recommended alternative to the crate's one documented trap
has never been compiled outside this probe.

The second is the published-sequence handshake — "the ring already carries the
distinction… a caller that needs 'somebody published nothing' must read it
there rather than from the slot." That is sound advice about a real mechanism,
and the mechanism is in `ring_publish`, `ring_consume`, `ring_mpsc` and
`ring_spsc`. The same census threshold now also catches `ring_stats` and
`ring_testkit`, which track the published count rather than implement the
handshake itself. This crate's dependency closure is `ring_types` and `ring_slot`.
The advice points outside the closure, and it points there without a name: no
type, no function, no crate is mentioned, so a caller following it has the whole
family to search.

**Finding.** The limitation is documented better than anything else in the crate
and both of its remedies are unreachable from where they are written — one never
instantiated anywhere in the workspace, the other in four crates this one has no
edge to and does not name. The fix is proportionate to the gap: a single test
constructing `TypedSlot< () >` and asserting the two states makes the first
escape real and costs three lines, and it would also be the crate's only
exercise of a zero-sized payload through the blanket impl. Naming the crate for
the second — even just "see `ring_publish`" — turns a direction into an address.
Neither changes the design; both change whether the advice can be acted on.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`invariant/001`](../invariant/001_two_readings_of_one_emptiness.md) | The agreement this state is the interesting case of |
| [`pitfall/002`](002_a_recycled_slot_is_not_equal_to_an_empty_one.md) | The other trap the same `len` field creates |
| [`pattern/002`](../pattern/002_proof_by_monomorphisation.md) | The shared body that returns `false` here and `true` everywhere else |
| [`lifecycle/001`](../lifecycle/001_four_positions_three_representations_two_observable_states.md) | The full state table this row belongs to |

### Sources

| Fact | Where |
|------|-------|
| The limitation, its rationale and its two escapes | `ring_event/src/lib.rs:113-125` |
| `drain_from`'s one-line doc and its doctest | `ring_event/src/lib.rs:180`, `:192-206` |
| The mechanism — a zero-length copy and `self.len = 0` | `ring_slot/src/lib.rs:347-355` |
| The test that pins the behaviour | `ring_event/tests/event_test.rs:129-140` |
| `TypedSlot< () >` constructed nowhere | Census above |
| The handshake's crates against this crate's closure | Census above |
| Every state a `BytesSlot< 4 >` can hold | Probe above |
| The unit slot behaving as the doc claims | Probe above |

### Tests

| Test | Covers |
|------|--------|
| `a_zero_length_publish_is_indistinguishable_from_unpublished_and_says_so` | The trap itself, named in the test |
| `an_unpublished_bytes_slot_reads_as_nothing_rather_than_as_an_empty_payload` | The state it is confused with |
| `peek_agrees_with_the_slots_own_emptiness` | The agreement that makes the confusion consistent |
