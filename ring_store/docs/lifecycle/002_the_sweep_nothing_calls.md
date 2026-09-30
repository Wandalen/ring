# Lifecycle: The Sweep Nothing Calls

### Scope

**Purpose:** Record that `Buffer::clear` documents `ring_shutdown` as the reason
it exists, that `ring_shutdown` neither depends on this crate nor mentions a
buffer, that the only call in the family is this crate's own test, and that the
guarantee the doc states is delivered by one slot shape and not the other.

**Responsibility:** The one lifecycle event between `new` and drop — who asked
for it, who calls it, and what it actually promises.

**In Scope:** `ring_store/src/lib.rs:98-125`; `ring_shutdown/`.

**Out of Scope:** What `clear` leaves in a `BytesSlot` is `ring_slot`'s
[`pitfall/002`](../../../ring_slot/docs/pitfall/002_clear_forgets_it_does_not_erase.md)
(SL43, SL44). Drop is
[`lifecycle/001`](001_allocate_once_borrow_forever_drop_plainly.md).

---

### BF32 — The Doc Names a Requester That Cannot Call It

`clear`'s doc opens by attributing the operation to a specific consumer:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A5 -F '  /// Empty every slot, keeping the allocation.' ring_store/src/lib.rs
```

Live output:

```
    /// Empty every slot, keeping the allocation.
    ///
    /// A reset for a recycled ring: reallocating would defeat the allocation
    /// behaviour the ring was chosen for, so this sweeps every slot back to
    /// empty in place instead. No consumer in the family calls this today —
    /// `ring_shutdown` does not depend on this crate, and its reopen story is a
```

`ring_shutdown` does not depend on `ring_store`, and the word does not occur in
its source:

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/^\[dependencies\]/,/^$/p' ring_shutdown/Cargo.toml
printf 'lines mentioning "buffer" in ring_shutdown/src/lib.rs: '
grep -ci 'buffer' ring_shutdown/src/lib.rs || true
```

Live output:

```
[dependencies]
ring_cursor = { path = "../ring_cursor" }
ring_wait = { path = "../ring_wait" }
ring_core = { path = "../ring_core" }
ring_types = { path = "../ring_types" }

lines mentioning "buffer" in ring_shutdown/src/lib.rs: 0
```

And the operation has exactly one caller family-wide:

```sh
cd "$(git rev-parse --show-toplevel)"
# sorted: grep is shimmed to a parallel ugrep here, so hits arrive in completion order
grep -r 'buffer\.clear()\|Buffer::clear' ring_*/src/*.rs ring_*/tests/*.rs 2>/dev/null \
  | grep -vE ':[[:space:]]*(///|//!|//)' | sed 's|ring/||' | sort
```

Live output:

```
ring_store/tests/buffer_test.rs:    buffer.clear();
```

**Finding.** The stated requester cannot reach the operation. `ring_shutdown`
sits on cursors, waiting and `ring_core`, holds no storage, and has no path to a
`Buffer` — its reopen story is a state machine over `Stopped`, not a payload
sweep. Whatever reset it needs, it is not this one.

That leaves `clear` with a real capability, a plausible rationale, and no
demonstrated user: one call in the whole family, in the test that exists to prove
the method works. The method is not dead — a recycled ring is a coherent thing to
want and this is how you would get one — but the doc presents an established
requirement where there is a hypothesis, and it names a crate that will not be
the one to use it. Someone auditing `ring_shutdown` for the reset described here
will find nothing, and someone changing `clear` will look for the consumer named
in its doc and find that consumer has no dependency on this crate at all.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -A9 -F '/// Empty every slot, keeping the allocation.' ring_store/src/lib.rs
```

Live output:

```
    /// Empty every slot, keeping the allocation.
    ///
    /// A reset for a recycled ring: reallocating would defeat the allocation
    /// behaviour the ring was chosen for, so this sweeps every slot back to
    /// empty in place instead. No consumer in the family calls this today —
    /// `ring_shutdown` does not depend on this crate, and its reopen story is a
    /// different state machine, over `Stopped`, not a payload sweep. The
    /// emptiness delivered is only as strong as `Slot::clear` for the shape in
    /// use: a `TypedSlot`'s previous payload is dropped, but a `BytesSlot`'s
    /// bytes stay resident and only the length marking them unreachable moves.
```

**Disposition:** applied — `Buffer::clear`'s doc comment in
`ring_store/src/lib.rs` no longer names `ring_shutdown` as the
requester this reset serves; it now states plainly that no consumer in the
family calls this today, that `ring_shutdown` does not depend on this crate,
and that its reopen story is a different state machine over `Stopped`, not a
payload sweep. Now prints: `No consumer in the family calls this today`

---

### BF33 — The Guarantee Is Stated Here and Delivered by One Shape of Two

The doc's second clause is a guarantee about payload visibility: *a recycled ring
must not hand a consumer the previous world's payloads*. That is the strongest
statement of the property anywhere in the family — `ring_slot`'s `clear` promises
only that the slot becomes empty. Measured against both shapes:

```
--- (4) what BytesSlot::clear leaves behind ---
  all_empty after clear = true
  Debug of slot 0       = BytesSlot { bytes: [108, 97, 112, 32, 48, 58, 32, 115, 101, 99, 114, 101, 116, 52, 53, 54], len: 0 }
```

`all_empty` reports `true` for a buffer whose every byte of the previous world's
payload is still resident and still printable. `TypedSlot` behaves as the
sentence claims — its `clear` assigns `None`, the destructor runs, the payload is
gone, and the same probe counts eight destructors at the `clear()` call.
`BytesSlot::clear` moves a length to zero.

**Finding.** The residue itself is `ring_slot`'s to own and is recorded there
(SL31, SL32, SL43, SL44). What belongs here is that the *promise* is made in this
crate, about a container, in language a reader will take as a security property —
"must not hand a consumer the previous world's payloads" — while the container
has no mechanism of its own to deliver it. `Buffer::clear` is a `for` loop over
`Slot::clear`; it can promise exactly what the trait promises and no more, and
the trait promises emptiness, not erasure.

So the family's clearest statement of the guarantee sits one tier above the code
that would have to implement it, on a method whose only caller is a test, phrased
so that both shapes appear covered. The fix is a sentence, not a design change:
`clear` empties every slot as each shape defines emptiness, and for `BytesSlot`
that leaves the bytes in place — which is either fine for the payloads in
question or a reason to reach for `TypedSlot`, and either way is the caller's
decision to make knowingly.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -A9 -F '/// Empty every slot, keeping the allocation.' ring_store/src/lib.rs
```

Live output:

```
    /// Empty every slot, keeping the allocation.
    ///
    /// A reset for a recycled ring: reallocating would defeat the allocation
    /// behaviour the ring was chosen for, so this sweeps every slot back to
    /// empty in place instead. No consumer in the family calls this today —
    /// `ring_shutdown` does not depend on this crate, and its reopen story is a
    /// different state machine, over `Stopped`, not a payload sweep. The
    /// emptiness delivered is only as strong as `Slot::clear` for the shape in
    /// use: a `TypedSlot`'s previous payload is dropped, but a `BytesSlot`'s
    /// bytes stay resident and only the length marking them unreachable moves.
```

**Disposition:** applied — `Buffer::clear`'s doc comment in
`ring_store/src/lib.rs` no longer states the payload-visibility
guarantee as a plain promise; it now scopes emptiness to what `Slot::clear`
actually delivers per shape — a `TypedSlot`'s previous payload dropped, but a
`BytesSlot`'s bytes staying resident with only the length moving. Now prints:
`bytes stay resident and only the length`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`lifecycle/001`](001_allocate_once_borrow_forever_drop_plainly.md) | The other three lifecycle events, and what drop destroys |
| [`item/001`](../item/001_the_slot_as_the_buffer_sees_it.md) | `clear` as one of the buffer's two trait calls |
| [`integration/001`](../integration/001_four_dependents_two_that_build_on_it.md) | The other function with no library caller |
| `ring_slot` — [`pitfall/002`](../../../ring_slot/docs/pitfall/002_clear_forgets_it_does_not_erase.md) | The residue, where it is owned |
| `ring_slot` — [`lifecycle/002`](../../../ring_slot/docs/lifecycle/002_a_slot_across_a_rings_laps.md) | The two `clear` bodies compared |

### Sources

| Fact | Where |
|------|-------|
| `clear`'s doc and body | `ring_store/src/lib.rs:98-125` |
| `ring_shutdown`'s dependencies | `ring_shutdown/Cargo.toml` |
| Zero mentions of a buffer there | `ring_shutdown/src/lib.rs` |
| The single caller | `ring_store/tests/buffer_test.rs:200` |
| Both shapes measured through `clear` | Release probe, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `clear_empties_every_slot_and_keeps_the_allocation` | `clear` at `TypedSlot`, address preserved |
| `ring_slot` — `clearing_a_bytes_slot_empties_the_reading` | The residue at the slot level: the reading empties, the bytes do not |
| *(to create)* | The same sweep over `BytesSlot`, asserting what `all_empty` does and does not mean |
| *(to create)* | A `ring_shutdown` reset path, or a doc correction where the claim is |
