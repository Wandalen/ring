# Capacity::mask

## Representation

Returns `capacity - 1`, the bitmask that folds a sequence into a slot index.
**This is the function the entire crate exists to make total.**

`self.0 - 1` underflows for `self.0 == 0`, and in a release build that is not a
panic but a wrap to `usize::MAX` — a mask that passes every index through
unchanged and addresses memory outside the ring. The value cannot be zero because
[`new`](001_capacity_new.md) rejects zero, and there is no other constructor. So
the subtraction is unchecked and correct, and it is unchecked *because* it is
correct rather than for speed.

The power-of-two half matters just as much and differently: for `n` a power of
two, `n - 1` is the low-bit mask, so `seq & mask` is `seq % n`. For `n = 6`,
`n - 1 = 5 = 0b101`, and `seq & 5` skips slots 2 and 3 entirely — no crash, no
error, a ring that silently uses four of its six slots. **The failure mode this
type prevents is not a panic, it is a wrong answer**
(→ [`../../invariant/001`](../../invariant/001_every_capacity_has_a_valid_mask.md)).

## Kind

Associated Function/Method (§ Item Kind Taxonomy : Associated Item Kinds #1)

## Definition

`ring_types/src/capacity.rs:75`

```rust
#[ must_use ]
pub const fn mask( self ) -> usize
```

Body is `self.0 - 1`.

## File Usage

| File | Line(s) | Context |
|------|---------|---------|
| `ring_types/src/capacity.rs` | 65-67, 69-72, 74-75, 77 | Doc explaining that the constructor rejected every value for which the mask would be invalid (65-67); doc example asserting `mask()` for 1 and 1024 (69-72); `#[ must_use ]` and **the definition (74-75)**; body `self.0 - 1` (77) |

Test-only references: `ring_types` — 4 in `tests/types_test.rs`, all inside
`capacity_accepts_powers_of_two`'s loop over the powers of two up to 2¹⁶,
asserting `mask() == slots - 1` and that `mask()` has exactly the low bits set.

## Crate Usage

| Crate | Via File | Purpose |
|-------|----------|---------|
| `ring_types` | `src/capacity.rs` | Defining crate |
| `ring_index` | `src/lib.rs` | `of( seq, capacity )` — the family's designated fold (`:49`) |
| `ring_mpsc` | `src/lib.rs` | `stamp( &self, seq )` — the same fold, inlined (`:541`) |

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -rn '\.mask()' ring_*/src | command grep -v ring_types/ | command grep -v ':[0-9]*: *//' \
  | command sed -E 's/^([^:]+):[0-9]+:/\1:/'
```

Live output:

```
ring_index/src/lib.rs:    SlotIndex((seq.0 as usize) & capacity.mask())
ring_mpsc/src/lib.rs:        let index = (seq.0 as usize) & self.capacity().mask();
```

returns exactly those two lines.

**Two call sites, and they are the same expression written twice:**

```rust
SlotIndex( ( seq.0 as usize ) & capacity.mask() )        // ring_index:51
let index = ( seq.0 as usize ) & self.capacity().mask(); // ring_mpsc:543
```

`ring_mpsc` does not depend on `ring_index` — its `[dependencies]` are
`ring_atomic`, `ring_store`, `ring_claim`, `ring_config`, `ring_cursor`,
`ring_gating`, `ring_slot`, `ring_types` — so this is duplication, deliberate and
safe. It is safe **because of the pattern**: the `Capacity` invariant travels
with the value, so a second implementation of the fold is correct for exactly the
same reason the first one is. The cost is maintenance, not correctness, and that
is a different and stronger property than preventing the duplication
(→ [`../../pattern/002`](../../pattern/002_a_newtype_that_makes_a_check_unnecessary.md)).

## Caller Tree

- *No caller within `ring_types`* — the crate never folds a sequence; it only makes folding safe
- *External: `ring_index::of`* (`ring_index/src/lib.rs:49`)
- *External: `ring_mpsc::Ring::stamp`* (`ring_mpsc/src/lib.rs:543`)

## Callee Tree

- *(none)* — the body is one subtraction on a field

**The narrowest function in the crate by callee count and by caller count, and
the one the design is about.** Fifteen crates hold a `Capacity`; two ever ask it
for a mask. The other thirteen benefit from this function without calling it,
because what they get is the absence of a check they would otherwise have to
write (→ [`002_capacity_get.md`](002_capacity_get.md), whose 29 sites are what
they do instead).
