# item

A container is defined as much by what it declines to know about its contents as
by what it holds. `Buffer` holds `S` and three words of its own, and the two
instances here take the census: which values cross the boundary, what the buffer
asks of each, and how long each lives. The answers are unusually short. Two trait
method calls, no payload access at all, one stored `Capacity`, and two parameter
types that are consumed within a line of arriving.

That shortness is the whole reason one `Buffer` definition serves both slot
shapes without a branch, and it is worth recording as a mechanism rather than as
a coincidence. It also has a cost the instances name: an abstraction exercised at
three call sites family-wide is an abstraction whose contract nothing checks, and
a container addressed by sequence that stores no sequence is a container whose
returned reference carries no evidence of which lap it belongs to.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [The Slot as the Buffer Sees It](001_the_slot_as_the_buffer_sees_it.md) | BF26, BF27 — two method calls and no payload access, and a trait whose generic reach is three lines |
| 002 | [The SlotIndex the Buffer Trusts](002_the_slotindex_the_buffer_trusts.md) | BF28, BF29 — an accessor used where a public field was offered, and a sequence that lives for one line |

### The Complete Census

| Item | Crosses at | Stored? | What the buffer does with it |
|------|-----------|---------|------------------------------|
| `S : Default` (`+ Slot` to sweep) | Every accessor's return | Yes — `Box< [ S ] >` | `S::default` to allocate; `clear` and `is_empty` only in the sweep; never a payload |
| `Capacity` | `new`, `capacity` | Yes — one field | Sizes the slice once; hands back unchanged |
| `SlotIndex` | `get`, `get_mut` | No | Unwrapped via `SlotIndex::get`, used as a slice index |
| `Seq` | `at`, `at_mut` | No | Folded by `ring_index::of`, then discarded |

Two stored, two borrowed. Nothing in the type can answer "which sequence
occupies this slot" — that mapping is `ring_mpsc`'s `stamps` array, a second
allocation the same length as the slots, held one tier up.

### Why the Trait Is Exactly Two Methods

`Slot` carries `clear` and `is_empty` and nothing else, and the fit to this
consumer is exact: a container that allocates `N` items up front needs to
initialise them (`Default`), to recycle them (`clear`), and to ask whether a
position is occupied (`is_empty`). Payload access is missing because payload
shapes differ — `TypedSlot::set` takes a `T`, `BytesSlot::write` takes a byte
slice — and a trait spanning both would have to name a payload type, which is the
coupling that would stop one `Buffer` from serving both.

The consequence recorded in BF27 is that the trait's *generic* surface is barely
used: three call sites across all 33 crates, and the only generic reach to
`is_empty` is inside `all_empty`, which nothing calls. Both shapes' `is_empty` is
called constantly through their concrete types, where the trait is not involved.
So the abstraction is load-bearing for the type system and nearly untouched by
executing generic code.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# every use the buffer makes of the Slot trait
grep -n 'Slot::\|slot\.clear()' ring_store/src/lib.rs | grep -vE ':[[:space:]]*(///|//!|//)'

# and every payload operation it does not make
grep -n '\.set(\|\.write(\|\.take(' ring_store/src/lib.rs | grep -vE ':[[:space:]]*(///|//!|//)' || echo '  none outside doctests'

# sorted: grep is shimmed to a parallel ugrep here, so hits arrive in completion order
grep -rn 'Slot::clear\|slot\.clear()\|Slot::is_empty' ring_*/src/*.rs \
  | grep -v '^ring_slot/' | sort

# every signature and field naming a ring_types type
grep -n 'SlotIndex\|Seq\|Capacity' ring_store/src/lib.rs | grep -E 'pub (const )?fn |^[0-9]+:  (slots|capacity) :' | grep -vE ':[[:space:]]*(///|//!|//)'

# how a SlotIndex is unwrapped here
grep -n 'index\.get()\|index\.0' ring_store/src/lib.rs

# the mapping the buffer refuses to hold
command grep -m1 -A6 -F '  /// One stamp per slot, holding the sequence whose payload currently occupies' ring_mpsc/src/lib.rs
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BF26 | `ring_store` | n/a — observation | The buffer makes exactly two `Slot` calls and never touches a payload, which is the structural reason one definition serves both slot shapes without a branch |
| BF27 | `ring_slot` | n/a — coverage | The `Slot` trait's generic surface is three call sites family-wide; `Slot::is_empty` is reached generically only inside `all_empty`, which has no caller, so nothing exercises the generic contract |
| BF28 | `ring_store` | n/a — observation | Both `SlotIndex` unwraps go through the accessor rather than the public tuple field, while the crate's own tests build twenty-five indices by tuple syntax — the discipline holds in the source and is dropped in the examples |
| BF29 | `ring_store` | n/a — doc gap | `at( seq )` reads as sequence-addressed but stores no sequence, so a returned `&S` carries no evidence of which lap it belongs to; the correspondence lives in `ring_mpsc`'s stamps array and is documented only there |
