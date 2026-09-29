# pattern

Two shapes account for nearly everything this crate does. The first is
delegation: the fold from a sequence to a slot index belongs to `ring_index`, the
storage belongs here, and `at`/`at_mut` compose them in three lines each. The
second is borrow discipline: after `new`, every operation takes `&self` or
`&mut self` and returns a borrow, so the type never names a payload and never
lets one out by value.

Both shapes work. What the instances record is where each has an edge the docs
do not mark — the fold's safety rests on one word in `at`'s body and is given up
by the one function that folds without a buffer, and the borrow discipline's
private field is nonetheless pinned in place by four public signatures naming
`core::slice::Iter`.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Delegate the Fold, Own the Storage](001_delegate_the_fold_own_the_storage.md) | BF34, BF35 — one fold reaching both rings without either depending on it, and the route that opens the capacity back up |
| 002 | [Every Write Is a Borrow](002_every_write_is_a_borrow.md) | BF36, BF37 — a lattice complete on borrows and empty by value, and a private field fixed by public types |

### The Two Routes to a Slot Address

| Route | Capacity comes from | Mismatch possible? | Callers |
|-------|--------------------|--------------------|---------|
| `Buffer::at` / `at_mut` | `self.capacity`, set in `new` | No — the caller supplies none | Both rings, transitively |
| `ring_batch::drain_order` | A parameter | Yes — one wrong argument | `ring_batch`'s own consumers |

Neither `ring_mpsc` nor `ring_spsc` depends on `ring_index`. They reach the fold
only through `Buffer::at`, which is why the family has one mask implementation
and no way for a ring to get it wrong.

### What the Borrow Discipline Buys

Because no operation takes or returns an `S` by value, `Buffer` never names a
payload type. That is the mechanism behind one definition serving both slot
shapes: a `Buffer::set( index, value )` would have to know what a payload is, and
the two shapes disagree — `TypedSlot::set` takes a `T`, `BytesSlot::write` takes
a byte slice. Expressing the write as `&mut S` moves that disagreement entirely
into `ring_slot`, where it belongs.

The cost is that a buffer cannot be taken apart. There is no `into_inner`, no
`IntoIterator for Buffer< S >`, no `Vec< S >` exit — a test asserting on final
contents walks `iter` and copies. For the consumers that exist, which hold their
buffer inside an `UnsafeCell` for the ring's whole life, that is not a cost at
all.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# who depends on the fold — sorted: a glob is not expanded in a stable order
grep -ln 'ring_index' ring_*/Cargo.toml | sed 's|/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/||;s|/Cargo.toml||' | sort

# every call site of it
grep -rn '[^_a-zA-Z]of( ' ring_*/src/*.rs | grep -vE ':[[:space:]]*(///|//!|//)' \
  | grep -vE '::of\(|fn of\(' | sed 's|/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/||' | sort

# the composition point, and the route that takes capacity as a parameter
command grep -m1 -A13 -F '  /// assert_eq!( buffer.get( SlotIndex( 2 ) ).get(), Some( &1 ) );' ring_store/src/lib.rs | tail -n 12
command grep -m1 -A4 -F 'pub fn drain_order( claim : &BatchClaim, capacity : Capacity )' ring_batch/src/lib.rs

# the whole public surface, and the absence of any by-value form
grep -nE 'pub (const )?fn |^  type ' ring_store/src/lib.rs
grep -n 'fn into_inner\|IntoIterator for Buffer<\|IntoIterator for Buffer <\|fn into_vec\|fn into_slots' ring_store/src/lib.rs \
  || echo '  none — a Buffer cannot be taken apart'

# the private field's type, published four times
grep -n 'core::slice::' ring_store/src/lib.rs
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BF34 | `ring_store` | n/a — observation | The family has one fold implementation and both rings reach it transitively through `Buffer::at` without depending on `ring_index`; the safety of that route rests on `at` closing over `self.capacity` |
| BF35 | `ring_batch` | n/a — doc gap | `drain_order` manufactures `SlotIndex` values against a capacity supplied as a parameter, making it the one place `get`'s mixed-capacity panic scenario becomes reachable; neither crate's docs connect the two |
| BF36 | `ring_store` | n/a — observation | Every operation after `new` is a borrow and no by-value exit exists, which is what keeps `Buffer` from ever naming a payload type — and is stated nowhere, so absence reads as oversight |
| BF37 | `ring_store` | n/a — doc gap | `slots` is private but its representation is published by four signatures naming `core::slice::Iter`, so the storage layout cannot change without a breaking release and nothing near the field says so |
