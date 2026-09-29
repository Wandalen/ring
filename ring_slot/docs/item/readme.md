# item

Ten inherent methods across the two shapes, catalogued one at a time — four on
`TypedSlot`, six on `BytesSlot`. The extra two are `capacity` and `len`, the size
questions a fixed array can answer and an `Option` cannot.

What the two instances find is that neither set is documented the way it is
actually used. `TypedSlot`'s accessors are consumed point-free far more often
than they are called, and the idiom lives entirely in the three crates that use
it; the `&self`/`&mut self` split between `get` and `take` decides the shape of a
drain loop three layers up and is explained twice in `ring_spsc` and never here.
`BytesSlot`'s `capacity` names a receiver it never reads and has no caller
outside the crate's own suite, and the read/write pair admits no partial access
at all — a payload can only be replaced whole.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [The Four of a `TypedSlot`](001_the_four_of_a_typed_slot.md) | SL25, SL26 — accessors consumed as function values, and a mutability split that determines an API elsewhere |
| 002 | [The Six of a `BytesSlot`](002_the_six_of_a_bytes_slot.md) | SL27, SL28 — a receiver that is never read, and the absence of any incremental writer |

### The Ten, by Shape

| Shape | Method | Receiver | Returns | Role |
|-------|--------|----------|---------|------|
| `TypedSlot` | `empty` | — | `Self` | Construct; `Default` delegates here |
| `TypedSlot` | `set` | `&mut self` | `Option< T >` | Produce; returns what it displaced |
| `TypedSlot` | `get` | `&self` | `Option< &T >` | Read without consuming |
| `TypedSlot` | `take` | `&mut self` | `Option< T >` | Consume; leaves the slot empty |
| `BytesSlot` | `empty` | — | `Self` | Construct; zeroes the array |
| `BytesSlot` | `capacity` | `&self` | `usize` | Report `N` — **without reading `self`** |
| `BytesSlot` | `len` | `&self` | `usize` | Report bytes written |
| `BytesSlot` | `is_empty` | `&self` | `bool` | `len == 0` |
| `BytesSlot` | `write` | `&mut self` | `Result< (), RingError >` | Replace the payload, or refuse |
| `BytesSlot` | `read` | `&self` | `&[ u8 ]` | Borrow the payload |

### Called Without a Receiver

Fifty path-form uses of `TypedSlot::get` and `TypedSlot::take` across
`ring_core`, `ring_mpsc`, and `ring_spsc`, in two shapes that are the family's
entire drain vocabulary:

```rust
batch.get_mut( offset ).and_then( TypedSlot::take )      // drain, owning
batch.iter().filter_map( TypedSlot::get ).copied()       // drain, borrowing
```

Both depend on the signatures being exactly `fn( &T ) -> U` and
`fn( &mut T ) -> U` — no extra argument, no error path. `set` cannot join them:
it takes a value, so it has arity two. `ring_slot`'s own doctests call
`slot.get()` and `slot.take()` with a receiver, the form nothing downstream uses,
so adding a parameter or a failure mode to either would compile here and break
fifty call sites elsewhere.

**Correction (2026-09-28):** this section read "Thirty-five" both times above
until the census in [`item/001`](001_the_four_of_a_typed_slot.md) was re-run —
`ring_mpsc`'s and `ring_spsc`'s test suites have both grown since, and
`ring_core` gained a path-form use in its own `tests/` the original count had
no room for. Full disposition in `item/001` § SL25.

### The Split That Reaches Three Layers Up

`get` borrows and `take` moves, so a consumer's choice is forced before it
reaches a slot — by whether it holds `&S` or `&mut S`. `Batch::get_mut` exists
for no other reason, and the rule is stated twice in `ring_spsc` (once in source,
once in a test doc comment recording that `get_mut` was *added* because owned
drains were otherwise impossible) and zero times in `ring_slot`.

### What the Six Do Not Offer

No `get_mut`, no `as_mut_slice`, no `push`, no `extend` — no mutable accessor and
no incremental writer anywhere in the crate. A caller assembling a payload in
pieces assembles it elsewhere and hands over a finished slice.

That is right for a ring, and unstated: a slot is written once per lap by whoever
claimed it, so an incremental writer would introduce a *partially written* state
the length field cannot distinguish from a completed short write — the same
conflation [`decisions/002`](../decisions/002_a_length_rather_than_a_flag.md)
records for zero-length payloads. The cost falls on a caller with a large `N`,
who copies through a staging buffer and finds no note saying so.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# the four, then the six
awk '/^  \/\/\/ assert!\( TypedSlot::< u8 >::empty\(\)\.get\(\)\.is_none\(\) \);$/{ n1 = NR } n1 && NR == n1 + 3 { print } /^  pub fn set\( &mut self, value : T \) -> Option< T >$/{ print } /^  pub const fn get\( &self \) -> Option< &T >$/{ print } /^  pub fn take\( &mut self \) -> Option< T >$/{ print }' ring_slot/src/lib.rs
awk '/^  \/\/\/ assert_eq!\( BytesSlot::< 8 >::empty\(\)\.capacity\(\), 8 \);$/{ n1 = NR } n1 && NR == n1 + 3 { print } /^  pub const fn capacity\( &self \) -> usize$/{ print } /^  pub const fn len\( &self \) -> usize$/{ print } /^  pub const fn is_empty\( &self \) -> bool$/{ print } /^  pub fn write\( &mut self, payload : &\[ u8 \] \) -> Result< \(\), RingError >$/{ print } /^  pub fn read\( &self \) -> &\[ u8 \]$/{ print }' ring_slot/src/lib.rs

# path-form uses, by crate and directory
grep -rn 'TypedSlot::\(get\|take\)' ring_*/src/*.rs ring_*/tests/*.rs 2>/dev/null \
  | sed 's|/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/\([a-z_]*\)/\([a-z]*\)/.*|\1/\2|' | sort | uniq -c

# the split, stated twice in the crate that pays for it
command grep -m1 -A3 -F '  /// **This is the only way to move a record out of a batch**, because taking a' ring_spsc/src/lib.rs
command grep -m1 -A2 -F '  /// `get` returns `&S`, and `TypedSlot::take` needs `&mut S`, so before' ring_spsc/tests/spsc_test.rs

# capacity's body is the const parameter; self is never read
command grep -m1 -A4 -F '  pub const fn capacity( &self ) -> usize' ring_slot/src/lib.rs

# no mutable accessor, no incremental writer
grep -n 'get_mut\|as_mut\|_mut(\|push\|extend\|append' ring_slot/src/lib.rs \
  || echo '  none anywhere in the crate'
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SL25 | `ring_slot` | n/a — doc gap | `get` and `take` are consumed as function values at fifty sites in three crates; the crate's own doctests demonstrate only the receiver form, and the point-free contract is undeclared |
| SL26 | `ring_slot` | n/a — doc gap | The `&self`/`&mut self` split is the whole drain contract and the reason `Batch::get_mut` exists; it is explained twice in `ring_spsc` and not at the definition site |
| SL27 | `ring_slot` | n/a — observation | `capacity` names a receiver it never reads, has no caller outside the crate's own suite, and cannot be reached from the type alone because no associated const exists |
| SL28 | `ring_slot` | n/a — doc gap | No mutable accessor and no incremental writer, so a payload is replaced whole — correct for a ring, and unstated, leaving a large-`N` caller to discover the staging copy |
