# algorithm

There is one algorithm in this crate and it is three steps long. `write` checks a
bound, copies a slice, and stores a length; everything else is a field read or a
field store. The crate has a cyclomatic complexity of one everywhere except that
single comparison.

Both instances are about what follows from that flatness. The first records that
the check comes *first*, which is the entire reason a refused write leaves the
slot untouched — reorder the three steps and the property needs a rollback. The
second takes the three ways emptiness is computed and finds that two of them read
different fields, cannot be merged, and are folded across a whole buffer by a
method in `ring_store` that had to be renamed to avoid colliding with a
same-named method meaning something else entirely.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Write Is a Bound Check and a Copy](001_write_is_a_bound_check_and_a_copy.md) | SL13, SL14 — the ordering that makes failure total, and the absence of every control-flow construct |
| 002 | [Emptiness Three Ways](002_emptiness_three_ways.md) | SL15, SL16 — two irreducible computations, and a fold whose name collides in the crate above |

### The Whole Algorithm, Once

```rust
pub fn write( &mut self, payload : &[ u8 ] ) -> Result< (), RingError >
{
  if payload.len() > N
  {
    return Err( RingError::BatchTooLarge { requested : payload.len(), capacity : N } );
  }
  self.bytes[ ..payload.len() ].copy_from_slice( payload );
  self.len = payload.len();
  Ok( () )
}
```

One comparison, one copy, one store. The other nine functions have no branch at
all.

### Why the Order Is the Property

The three steps could be arranged three ways and only one of them gives a total
failure:

| Order | On refusal |
|-------|------------|
| check → copy → store | Nothing was touched — **this is the shipped order** |
| copy → check → store | Bytes already overwritten; needs a rollback |
| check → store → copy | A slot whose `len` exceeds its written bytes |

`a_failed_write_leaves_the_previous_contents_intact` and
`a_failed_write_onto_an_empty_slot_leaves_it_empty` assert both directions, as
separate tests rather than two assertions in one — which is what keeps the second
from being an afterthought.

### Three Emptiness Computations, Two Distinct

`TypedSlot::is_empty` reads a discriminant, `BytesSlot::is_empty` compares a
length to zero, and `BytesSlot`'s trait method delegates to its inherent twin. So
there are three call paths and two computations, and the two cannot be unified
because the shapes share no more primitive field to derive emptiness from. That
is exactly why `Slot` declares `is_empty` as a method rather than providing a
default body.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# the whole algorithm
command grep -m1 -A9 -F '  pub fn write( &mut self, payload : &[ u8 ] ) -> Result< (), RingError >' ring_slot/src/lib.rs

# no loop, no retry, no atomic anywhere in the crate
grep -vE '^[[:space:]]*//' ring_slot/src/lib.rs \
  | grep -nE '\bloop\b|\bwhile\b|\bfor \b|compare_exchange|Atomic' || echo '  none'

# the three emptiness bodies
awk '/^impl< T > Slot for TypedSlot< T >$/{ n1 = NR } n1 && NR >= n1 + 2 && NR <= n1 + 5 { print } /^  \/\/\/ assert!\( !s\.is_empty\(\) \);$/{ n2 = NR } n2 && NR >= n2 + 2 && NR <= n2 + 6 { print } /^impl< const N : usize > Slot for BytesSlot< N >$/{ n3 = NR } n3 && NR >= n3 + 2 && NR <= n3 + 5 { print }' ring_slot/src/lib.rs

# the fold, and the same-named method that is not it
sed -n '/^  \/\/\/ Whether every slot is empty\.$/,/^  }$/p;/^  \/\/\/ Always false — a `Capacity` cannot be zero, so a buffer always has slots\.$/,/^  }$/p' ring_store/src/lib.rs

# both refusal-direction tests
grep -n 'fn a_failed_write' ring_slot/tests/slot_test.rs
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SL13 | `ring_slot` | n/a — observation | The bound check precedes every mutation, so a refused write is total with no rollback; both directions are covered by separate tests |
| SL14 | `ring_slot` | n/a — observation | No loop, no retry, no atomic, no branch outside `write` — complexity one everywhere but a single comparison |
| SL15 | `ring_slot` | n/a — observation | The two real emptiness computations read different fields and cannot be merged, which is why `Slot::is_empty` has no default body |
| SL16 | `ring_store` | **misleading doc** | `Buffer::all_empty` is the fold; `Buffer::is_empty` is a differently-named method that is always `false`, existing only to satisfy a clippy lint |
