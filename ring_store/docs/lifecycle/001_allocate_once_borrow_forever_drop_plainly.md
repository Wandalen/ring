# Lifecycle: Allocate Once, Borrow Forever, Drop Plainly

### Scope

**Purpose:** Record that a buffer has three lifecycle events and no `Drop` impl,
that payload destructors nevertheless run — measured — and that because nothing
in the family ever empties a consumed slot, a dropped buffer destroys payloads
the ring has long since delivered.

**Responsibility:** The buffer's own arc from `new` to drop, and what happens to
the payloads it owns at each point.

**In Scope:** `ring_store/src/lib.rs:87-93`; the family's `Drop` impls;
`ring_slot/src/lib.rs:154-157`.

**Out of Scope:** `clear` is [`lifecycle/002`](002_the_sweep_nothing_calls.md).
The single allocation is
[`algorithm/002`](../algorithm/002_one_allocation_n_defaults.md).

---

### BF30 — No `Drop` Impl, and Destructors Run Anyway

The whole arc is three events: `new` allocates, accessors borrow, and the value
goes out of scope. There is no `Drop for Buffer`, and the family's only `Drop`
impls sit one tier up:

```sh
cd "$(git rev-parse --show-toplevel)"
# sorted: grep is shimmed to a parallel ugrep here, so hits arrive in completion order
grep -r '^impl.*Drop for' ring_*/src/*.rs | sed 's|ring/||' | sort
```

Live output:

```
ring_mpsc/src/lib.rs:impl< S > Drop for Batch< '_, S >
ring_mpsc/src/lib.rs:impl< S > Drop for Reserved< '_, S >
ring_spsc/src/lib.rs:impl< S > Drop for Batch< '_, S >
ring_spsc/src/lib.rs:impl< S > Drop for Reservation< '_, S >
```

Four, all RAII publish guards. Nothing in the family — not `Ring`, not `Buffer` —
runs cleanup when storage dies. A drop-counting payload measures what happens
instead:

```
--- (2) dropping a full buffer runs them too ---
  before scope end: drops = 0
  after scope end:  drops = 8
```

**Finding.** The buffer needs no destructor because `Box< [ S ] >` already has
one: dropping the box drops every `S`, and dropping a `TypedSlot< T >` drops its
`Option< T >`. Eight payloads in, eight destructors out, with no code in this
crate arranging it.

That is the right shape, and it is worth stating explicitly because storage is
the only thing in the family that owns a payload. Every ring holds its `Buffer`
behind an `UnsafeCell` and reaches slots through raw pointers; the cursors,
stamps and gating structures hold sequence numbers. So the entire family's
payload ownership resolves to one `Box< [ S ] >` per ring, and the entire
family's payload destruction resolves to that box going out of scope. A `Drop for
Ring` would be adding a second owner to a design that deliberately has one.

---

### BF31 — Nothing Ever Empties a Consumed Slot, So Drop Destroys Delivered Payloads

`TypedSlot::take` is the operation that would empty a slot on read. Its
family-wide call census, in library code:

```sh
cd "$(git rev-parse --show-toplevel)"
# sorted: grep is shimmed to a parallel ugrep here, so hits arrive in completion order
grep -r '\.take(' ring_*/src/*.rs | grep -vE ':[[:space:]]*(///|//!|//)' | sed 's|ring/||' | sort
```

Live output:

```
ring_slot/src/lib.rs:        self.0.take()
```

One line, and it is `take`'s own body. No consumer in the family calls it — a
consumer reads through a borrow and leaves the payload where it is. `ring_spsc`
says so directly, at the one site that has to reason about it:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A6 -F '    let Ok( mut reservation ) = self.claim() else { return Err( record ) };' ring_spsc/src/lib.rs
```

Live output:

```
    let Ok( mut reservation ) = self.claim() else { return Err( record ) };

    // The slot's previous occupant, if any, was committed by the consumer a lap
    // ago; dropping it here is what makes the ring's storage bounded.
    drop( reservation.set( record ) );

    Ok( () )
```

**Finding.** A payload's residency in the buffer outlives its delivery. It is
written, read by a consumer, and then sits in the slot for up to a full lap until
a producer overwrites it — at which point the displaced value is dropped by
whoever handled the `set` return. If the ring dies before that lap completes,
those already-delivered payloads are destroyed by the buffer's own drop instead.

Nothing is wrong here — bounded storage is exactly what the comment says it buys,
and destroying a delivered payload late is harmless for the record types this
family carries. What is missing is the statement. Residency is not liveness in
this design, and a reader who assumes the two coincide will draw two wrong
conclusions: that a buffer's occupied-slot count says something about undelivered
work, and that a payload holding an external resource — a file handle, a lock
guard, a channel sender — releases it when the consumer is finished rather than a
lap later or at ring teardown. That second reading is the one that costs
something, and neither `Buffer` nor `Slot` mentions it. `ring_spsc`'s comment is
the family's only written trace of the rule, and it is a line comment inside a
private method.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`lifecycle/002`](002_the_sweep_nothing_calls.md) | The one operation that empties slots, and who calls it |
| [`algorithm/002`](../algorithm/002_one_allocation_n_defaults.md) | The allocation that opens the arc |
| [`item/001`](../item/001_the_slot_as_the_buffer_sees_it.md) | The two trait calls, neither of which is `take` |
| [`data_structure/001`](../data_structure/001_three_words_whatever_the_slot_costs.md) | The three words that die with the box |
| `ring_slot` — [`pattern/002`](../../../ring_slot/docs/pattern/002_the_displaced_value_returned.md) | What happens to the payload a lap overwrite displaces |

### Sources

| Fact | Where |
|------|-------|
| The family's four `Drop` impls | `ring_mpsc/src/lib.rs:985, 1257`; `ring_spsc/src/lib.rs:788, 1130` |
| `take`'s single occurrence | `ring_slot/src/lib.rs:154` |
| The consumer-does-not-clear rule | `ring_spsc/src/lib.rs:717-719` |
| Eight destructors at scope end | Release probe, quoted above |
| The buffer's own construction | `ring_store/src/lib.rs:87-93` |

### Tests

| Test | Covers |
|------|--------|
| `clear_empties_every_slot_and_keeps_the_allocation` | The only lifecycle event between `new` and drop |
| `ring_spsc` — the bounded-storage assertions | That a lap overwrite is what frees a payload |
| *(to create)* | A drop-counting payload asserting `capacity` destructors run at buffer drop |
| *(to create)* | The same counter asserting a delivered payload is not dropped at read |
