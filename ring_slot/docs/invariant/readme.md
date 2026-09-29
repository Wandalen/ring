# invariant

Two properties hold across this crate, and both are about `BytesSlot` — the shape
with an unused tail to get wrong. The first is the crate's reason to exist: a
read returns the bytes last written and never the tail behind them. The second is
smaller and stranger: the same question, "am I empty?", is answered by two
functions that can never disagree, because one of them is a call to the other.

The interesting part of both is where the property stops. The first is
established by an exhaustive test — every length from empty to full, on a fresh
slot — and is stated in the module comment as a property of *the slot*, while
being true only of `read`; two derived traits used to see the tail, until their
findings replaced both with hand-written impls over `read()`. The second is upheld
structurally rather than by discipline, and is asserted anyway by the only test
that can, because every ordinary call site resolves to whichever `is_empty`
happens to be in scope.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [A Read Returns What Was Written](001_a_read_returns_what_was_written.md) | SL9, SL10 — a domain exhausted rather than sampled, and the one fixture with a non-zero tail |
| 002 | [The Two Emptiness Paths Agree](002_the_two_emptiness_paths_agree.md) | SL11, SL12 — a delegation invariant, and the shape that has no second path at all |

### The First Property, in Two Halves

| Half | Upheld by | Where |
|------|-----------|-------|
| *reads back exactly what was written* | `write` copies to the front of the array | `src/lib.rs:353` |
| *and nothing else* | `read` slices to `self.len`, not to `N` | `src/lib.rs:369` |

Neither half alone suffices, and the two functions holding them sit eleven lines
apart. The test that establishes them runs `0..=CAP` inclusive on a fresh slot each time —
nine cases at `N == 8`, including the boundary where a `>` and a `>=` in `write`
would differ. The domain is exhausted, not sampled, and the test's doc comment
says why: an off-by-one leaks exactly one stale byte, which no sampled length
would reliably catch.

### Where the Statement Outruns the Property

The module comment calls this a property of a *partially-filled slot*. It is a
property of `read`. Four accessors make that the same thing; two derives once did
not — `==` and `Debug` both reached past `self.len` into the tail, so a slot
written `"AAAAAAAA"` then `"bb"` used to be distinguishable from a slot only ever
written `"bb"`. Both are now hand-written over `read()` instead, so all six
observations agree
([`pitfall/001`](../pitfall/001_the_test_that_names_a_property_the_type_lacks.md)).

Exactly one test in the crate builds a slot whose tail is non-zero, and every
assertion it makes goes through `read()` — so the invariant is verified precisely
where it holds and nowhere near where it does not.

### The Second Property Is a Call, Not a Convention

```rust
impl< const N : usize > Slot for BytesSlot< N >
{
  fn is_empty( &self ) -> bool
  {
    Self::is_empty( self )      // not a re-implementation — a call
  }
  // ...
}
```

One comparison in the crate decides `BytesSlot` emptiness and both paths reach
it, so disagreement is unrepresentable without editing the delegation. The suite
asserts it regardless, with fully-qualified syntax at five payloads plus the
post-`clear` state — which is the right treatment: the test does not guard
today's code, it guards the edit that replaces the delegation with a duplicated
comparison.

`TypedSlot` has no inherent `is_empty` at all — four inherent methods, none of
them that one — so the ergonomic argument `BytesSlot`'s doc makes at length
applies to `TypedSlot` and is not honoured there, unremarked.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# the invariant, as the crate states it in both places
grep -n 'reads back exactly\|and only those\|never the unused tail' ring_slot/src/lib.rs

# its two halves, fourteen lines apart
sed -n '/^    self\.bytes\[ \.\.payload\.len() ]\.copy_from_slice( payload );$/,/^    self\.len = payload\.len();$/p;/^  pub fn read( &self ) -> &\[ u8 ]$/,/^  }$/p' ring_slot/src/lib.rs

# the exhaustive test, and the one non-zero-tail fixture
sed -n '/^\/\/\/ A read returns the written bytes and only those — never the unused tail\.$/,/^}$/p;/^\/\/\/ A shorter write over a longer one truncates the reading — the stale tail must$/,/^}$/p' ring_slot/tests/slot_test.rs

# the delegation that makes disagreement unrepresentable
command grep -m1 -A11 -F 'impl< const N : usize > Slot for BytesSlot< N >' ring_slot/src/lib.rs

# TypedSlot's four inherent methods — none of them is_empty
command grep -m1 -A72 -F 'impl< T > TypedSlot< T >' ring_slot/src/lib.rs | grep -E '^\s*pub (const )?fn '
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SL9 | `ring_slot` | n/a — observation | The invariant is exhausted over every length `0..=CAP` on a fresh slot rather than sampled, with the reason stated in one line of doc comment |
| SL10 | `ring_slot` | n/a — coverage | The only fixture with a non-zero tail asserts solely through `read()`; the module comment states the property of the slot, where it holds only of `read` |
| SL11 | `ring_slot` | n/a — observation | The trait `is_empty` calls the inherent one, so disagreement is unrepresentable; the test that asserts it anyway guards the future edit, not the present code |
| SL12 | `ring_slot` | n/a — doc gap | `TypedSlot` has no inherent `is_empty`, so the concrete-type ergonomics `BytesSlot`'s doc argues for are absent on the other shape and nothing says so |
