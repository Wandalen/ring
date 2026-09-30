# Item: The Slot as the Buffer Sees It

### Scope

**Purpose:** Record that the buffer calls exactly two slot methods, once each,
never touches a payload, and that `Slot`'s two-method shape is sized precisely to
what this consumer needs.

**Responsibility:** The slot as an item this container holds — what the container
asks of it and what it never asks.

**In Scope:** `ring_store/src/lib.rs:98-141`; the `Slot` trait's surface.

**Out of Scope:** That `Default` is also required is
[`data_structure/002`](../data_structure/002_the_default_bound_is_a_bound_on_the_type.md).
What `clear` does to the two shapes is
[`lifecycle/002`](../lifecycle/002_the_sweep_nothing_calls.md).

---

### BF26 — Two Method Calls, and No Payload Access at All

Every use the buffer makes of the `Slot` trait, in executable code:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -n 'Slot::\|slot\.clear()' ring_store/src/lib.rs | grep -vE ':[[:space:]]*(///|//!|//)' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
```

Live output:

```
            slot.clear();
        self.slots.iter().all(Slot::is_empty)
```

Two lines. `clear` inside the loop that sweeps the buffer, `is_empty` folded over
the slice. Nothing else — the buffer never calls `set`, `get`, `write`, `read` or
`take`, because those are inherent to the two shapes and not on the trait
([`pattern/001`](../pattern/001_delegate_the_fold_own_the_storage.md)).

```sh
cd "$(git rev-parse --show-toplevel)"
grep -n '\.set(\|\.write(\|\.take(' ring_store/src/lib.rs | grep -vE ':[[:space:]]*(///|//!|//)' || echo '  none outside doctests' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
```

Live output:

```
  none outside doctests
```

**Finding.** The container holds items it cannot inspect. It can construct one
(`S::default`), empty one, ask whether one is empty, and hand out a reference —
and that is the complete list. A payload enters and leaves through the borrow, on
the caller's own terms, with the buffer never learning what it was.

That is what makes one `Buffer` definition serve both slot shapes without a
branch, and it is the mechanism behind hard problem 125's answer: storage holds
nothing that would differ between rings because it holds nothing about payloads
at all. Recorded here as the structural reason rather than as a coincidence —
the generality is not that `Buffer` was written flexibly, it is that two method
calls are all it makes.

---

### BF27 — The Trait Is Sized to This Consumer, and Its Other Consumer Uses Half of It

`Slot` has exactly two methods, and this crate calls exactly those two. The fit
is not accidental — a container that allocates `N` items up front needs to
initialise them and to recycle them, which is `Default` plus `clear`, and needs a
way to ask whether a position is occupied, which is `is_empty`.

The family's only other generic user of the trait calls one of the two:

```sh
cd "$(git rev-parse --show-toplevel)"
# sorted: grep is shimmed to a parallel ugrep here, so hits arrive in completion order
grep -r 'Slot::clear\|slot\.clear()\|Slot::is_empty' ring_*/src/*.rs \
  | grep -v '^ring_slot/' | sed 's|ring/||' | sort
```

Live output:

```
ring_store/src/lib.rs:  /// emptiness delivered is only as strong as `Slot::clear` for the shape in
ring_store/src/lib.rs:    self.slots.iter().all( Slot::is_empty )
ring_store/src/lib.rs:      slot.clear();
ring_event/src/lib.rs:  slot.clear();
```

`ring_event::recycle` clears one slot. Nothing in the family calls
`Slot::is_empty` generically except this crate's `all_empty`, which — as
[`integration/001`](../integration/001_four_dependents_two_that_build_on_it.md)
BF3 records — has no caller of its own.

**Finding.** The trait's whole generic surface, across the whole family, is three
call sites: two here and one in `ring_event`. `Slot::is_empty` is reached
generically at exactly one place in shipped code, inside a function nothing
calls.

That is a smaller reach than the abstraction's prominence suggests, and it is not
an argument for removing anything — `is_empty` is also called *inherently* by
both shapes' own users and by tests throughout the family, where the concrete
type is known and the trait is not involved. What it does mean is that the
trait's generic contract is exercised by almost nothing: if `Slot::is_empty`'s
meaning drifted between the two shapes, the only generic code that would notice
is `all_empty`, and no consumer calls it. The two shapes already disagree
underneath about what `clear` leaves behind
([`lifecycle/002`](../lifecycle/002_the_sweep_nothing_calls.md)), and the same
thinness is why nothing caught it.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`item/002`](002_the_slotindex_the_buffer_trusts.md) | The other item crossing this boundary |
| [`data_structure/002`](../data_structure/002_the_default_bound_is_a_bound_on_the_type.md) | The third thing the buffer asks of a slot, and where it is written |
| [`lifecycle/002`](../lifecycle/002_the_sweep_nothing_calls.md) | What `clear` means to each shape |
| [`pattern/001`](../pattern/001_delegate_the_fold_own_the_storage.md) | Why payload access is not on the trait |
| [`invariant/001`](../invariant/001_capacity_equals_length_always.md) | The reached-test's "get/set", answered by a borrow |

### Sources

| Fact | Where |
|------|-------|
| The two trait calls | `ring_store/src/lib.rs:123, 140` |
| No payload access | `ring_store/src/lib.rs` — no occurrence outside doctests |
| The family's third generic call site | `ring_event/src/lib.rs:233` |
| Hard problem 125's question | `ring_store/src/lib.rs:12-17` |
| `all_empty` having no caller | Census in `integration/001` |

### Tests

| Test | Covers |
|------|--------|
| `every_slot_starts_empty` | `all_empty`, the one generic `is_empty` fold |
| `clear_empties_every_slot_and_keeps_the_allocation` | `clear` across every slot |
| `the_same_buffer_type_serves_both_slot_shapes` | One definition over two shapes, no branch |
| `ring_slot` — `both_shapes_drive_through_the_trait_alone` | The trait's generic contract, where it is exercised |
| *(to create)* | A generic assertion that `clear` leaves both shapes `is_empty`, at the buffer level |
