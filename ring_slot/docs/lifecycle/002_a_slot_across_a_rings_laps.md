# Lifecycle: A Slot Across a Ring's Laps

### Scope

**Purpose:** Record what a slot carries from one lap to the next, that the two
shapes differ completely on the question, and that nothing in the family ever
resets a ring's slots in bulk.

**Responsibility:** Slot state persistence across reuse — what a lap releases,
what it retains, and for how long.

**In Scope:** the two `Slot::clear` bodies in `ring_slot/src/lib.rs`;
`Buffer::clear` in `ring_store/src/lib.rs`; the retention measured across
two laps. Addressed by name rather than by line — SL31's and SL32's own fixes
moved every line number this section used to carry.

**Out of Scope:** One publish's own state sequence is
[`lifecycle/001`](001_a_slot_across_one_publish.md). That `clear` forgets rather
than erases is [`pitfall/002`](../pitfall/002_clear_forgets_it_does_not_erase.md)
SL43 — this instance is about how long the forgotten bytes then stay, and what
that costs.

---

## The Premise: Slots Are Allocated Once

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A3 -F '/// The shape for traffic arriving from outside the process, decoded later by' ring_slot/src/lib.rs
```

Live output:

```
/// The shape for traffic arriving from outside the process, decoded later by
/// whoever knows the wire format. Fixed capacity because a ring's slots are
/// allocated once, so a slot that could grow would defeat the allocation
/// behaviour the ring was chosen for.
```

Allocated once and reused forever. A ring of capacity `C` revisits a given slot
every `C` publishes, and never frees it while the ring lives. That is the whole
point of the design and it is also what makes everything below a consequence
rather than a bug.

---

### SL31 — One Shape Releases a Lap's Payload and the Other Retains It in Full

Both shapes implement the same `clear`, and the two bodies do different things:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '--- TypedSlot ---'
command grep -m1 -A6 -F '    self.0.is_none()' ring_slot/src/lib.rs | tail -n 4
echo '--- BytesSlot ---'
command grep -m1 -A6 -F '    Self::is_empty( self )' ring_slot/src/lib.rs | tail -n 4
```

Live output:

```
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

`self.0 = None` drops whatever the `Option` held; `self.len = 0` touches nothing
but a counter. Two tests hold the halves apart — one counts destructions, one
reads the layout:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '--- TypedSlot: what a lap releases ---'
command grep -m1 -A6 -F '  slot.clear();
  assert_eq!( DROPS.load( Ordering::Relaxed ), 1' ring_slot/tests/slot_test.rs
echo '--- BytesSlot: what a lap cannot release ---'
command grep -m1 -A9 -F 'fn a_byte_slots_payload_is_inline_so_clear_has_no_allocation_to_release()' ring_slot/tests/slot_test.rs \
  | command grep -E 'size_of|"'
```

Live output:

```
--- TypedSlot: what a lap releases ---
  slot.clear();
  assert_eq!( DROPS.load( Ordering::Relaxed ), 1, "clearing runs the destructor" );
  assert!( slot.is_empty() );

  slot.clear();
  assert_eq!( DROPS.load( Ordering::Relaxed ), 1, "and a second clear has nothing left to run" );
}
--- BytesSlot: what a lap cannot release ---
  use core::mem::size_of;
    size_of::< BytesSlot< 16 > >(),
    16 + size_of::< usize >(),
    "sixteen bytes of payload and a length, with no pointer to anything else",
```

The `TypedSlot` releases its payload's memory at `clear` — a destructor runs, and
runs once. The `BytesSlot` releases nothing, because there is nothing to release:
the size assertions show the payload sized by `N` and reached without a pointer,
so it is inline in a slot the ring owns and will keep.

| | `TypedSlot< T >` | `BytesSlot< N >` |
|---|---|---|
| `clear` does | Drops `T` | Sets `len = 0` |
| Payload memory after `clear` | Freed | Resident, all `N` bytes |
| Observable through the API? | No — `get()` is `None` | No — `read()` is `[]` |
| Observable through `Debug` or `==`? | No | No — both are hand-written over `read()` |
| Retention window | Until `clear` | Until overwritten, or the ring dies |

**Finding.** The two shapes' `clear` agree on everything the API can see and
disagree on everything underneath. That is the correct outcome — a shape whose
payload is inline cannot free it without zeroing, and zeroing `N` bytes per
recycle is a cost the design explicitly declined
([`decisions/002`](../decisions/002_a_length_rather_than_a_flag.md) SL7 prices
the *construction* zeroing; a per-lap zeroing would pay it again every lap).

What the asymmetry costs is a reasoning trap. A reader who has satisfied
themselves that a `TypedSlot` ring releases each lap's payload — which it does,
provably, at the drop counter above — has learned nothing about a `BytesSlot`
ring, where the opposite holds and the same word `clear` describes it. The crate
gives the same one-line summary for both trait impls and the difference appears
only in the bodies.

---

### SL32 — Nothing Resets a Ring in Bulk, So Residue Persists for the Ring's Lifetime

A shorter write on the next lap does not close the window. Since the fix for
[`pitfall/002`](../pitfall/002_clear_forgets_it_does_not_erase.md) SL44, no safe
API prints or compares the tail — so the retention is established from the
crate's own field census instead, which is the stronger evidence anyway: every
read and write of `BytesSlot`'s two fields, in the whole crate.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'self\.bytes\|self\.len' ring_slot/src/lib.rs
```

Live output:

```
    self.len
    self.len == 0
    self.bytes[ ..payload.len() ].copy_from_slice( payload );
    self.len = payload.len();
    &self.bytes[ ..self.len ]
    self.len = 0;
```

Six accesses. Exactly one writes the array — `copy_from_slice` over
`..payload.len()`, bounded by the payload rather than by `N`. `clear` writes the
counter and stops. So a lap-2 write of five bytes over a lap-1 write of sixteen
leaves eleven bytes of lap 1 in the array, and they stay until some lap writes at
least sixteen. `a_byte_slots_payload_is_inline_so_clear_has_no_allocation_to_release`
runs exactly that sequence and asserts what `read()` and `len()` report after it.

`ring_store` provides the one bulk reset that would clear them, and nothing
calls it:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '--- Buffer::clear callers in any shipped code ---'
command grep -r '\.clear()' ring_*/src/*.rs | command grep -vE ':[[:space:]]*///?' \
  | command grep -vE 'entries|items|sink|log|slot\.clear' || echo '  none — no crate resets a whole buffer'
echo '--- and where it is called at all ---'
# sorted: grep is shimmed to a parallel ugrep here, so hits arrive in completion order
command grep -rl 'buffer\.clear()' ring_*/src/*.rs ring_*/tests/*.rs 2>/dev/null | LC_ALL=C sort
```

Live output:

```
--- Buffer::clear callers in any shipped code ---
  none — no crate resets a whole buffer
--- and where it is called at all ---
ring_store/src/lib.rs
ring_store/tests/buffer_test.rs
```

Its definition and its test. No ring shuts down by clearing its slots, and none
clears them on drop — the boxed slice is freed and the bytes go back to the
allocator as they stand.

**Finding.** The retention window for a `BytesSlot` payload is not one lap; it is
"until a longer write lands on that exact slot, or the ring is dropped." For a
capacity-1024 ring with mostly-short payloads, a single long message can sit in
one slot's tail for the process's whole life, unreachable through every public
API this type has — `read()`, `Debug` and `==` all agree on the first `len`
bytes — and still visible to anything that reads the process's memory or the
freed allocation afterwards.

This is a real property with an honest justification and no statement of it
anywhere. The justification: zeroing per lap costs `N` bytes of writes per
publish on the hot path, which is exactly the cost the fixed-array shape exists
to avoid, and rings carrying secrets are not this crate's problem to solve
unilaterally.

The gap: nothing tells a caller the window is unbounded. `BytesSlot`'s
documentation says a partially-filled slot "reads back exactly what was written
and nothing else"
([`invariant/001`](../invariant/001_a_read_returns_what_was_written.md) SL10
records that this is true of `read` and narrower than it sounds), and a reader
who takes it as a statement about the *slot* will conclude the tail is gone. It
is not gone, it is not going, and the one API that would remove it —
`Buffer::clear` — is not wired to any lifecycle event at all.

A `zeroing_clear` on `BytesSlot`, or a note on `Slot::clear` naming the two
shapes' different guarantees, would close it without costing the hot path
anything.

**Disposition:** applied — the second of the two, which is the one that costs the
hot path nothing at all. `Slot::clear`'s documentation now carries a paragraph
headed "**How long the residue lasts.**" that answers the question this finding
says nothing answers: for `BytesSlot`, "until a write of at least that length
lands on the same slot, or the ring holding it is dropped — not until the next
lap", with `Buffer::clear` named as the only bulk reset in the family and
recorded as wired to no lifecycle event. It closes by stating the consequence in
the caller's terms — the bytes are in the process's memory until overwritten,
"which is the property a caller handling secrets has to plan around" — and by
naming what changed since: no public API on either shape can read past the
length, because `Debug` and `PartialEq` are now hand-written for exactly that
reason ([`pitfall/002`](../pitfall/002_clear_forgets_it_does_not_erase.md) SL44).
`zeroing_clear` is declined on the finding's own pricing: `N` bytes of writes per
publish is the cost the fixed-array shape exists to avoid, and it would buy
nothing the note does not already state.
Now prints: `self.bytes[ ..payload.len() ].copy_from_slice( payload );`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`lifecycle/001`](001_a_slot_across_one_publish.md) | The single-publish cycle these laps repeat |
| [`pitfall/002`](../pitfall/002_clear_forgets_it_does_not_erase.md) | That `clear` forgets rather than erases — this instance times it |
| [`invariant/001`](../invariant/001_a_read_returns_what_was_written.md) | The property that holds through `read`, and now through `Debug` too |
| [`pitfall/001`](../pitfall/001_the_test_that_names_a_property_the_type_lacks.md) | The two traits, now hand-written, that used to see the retained tail |
| [`decisions/002`](../decisions/002_a_length_rather_than_a_flag.md) | Why the bytes are inline and the zeroing was declined |

### Sources

| Fact | Where |
|------|-------|
| Slots allocated once | `ring_slot/src/lib.rs:191-194` |
| `TypedSlot::clear` drops | `ring_slot/src/lib.rs:183-186` |
| `BytesSlot::clear` forgets | `ring_slot/src/lib.rs:392-395` |
| The one bulk reset | `ring_store/src/lib.rs:119-125` |
| No shipped caller for it | `ring_*/src/*.rs` — no occurrence |
| Drop counter and two-lap retention | Release probe, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `a_shorter_write_does_not_leak_the_longer_one` | The lap-2 case, through `read()` — where the property holds |
| `slots_compare_by_payload_not_by_tail` | The tail existing, asserted from the comparison side |
| `clearing_a_bytes_slot_empties_the_reading` | `clear` as the API sees it, which is all it sees |
| `ring_store` — the `clear` doctest | The bulk reset, exercised only there |
| *(to create)* | A `BytesSlot` whose tail is asserted still resident after a shorter lap-2 write |
