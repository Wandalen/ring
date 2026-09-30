# Type: Three Types Borrowed, None Owned

### Scope

**Purpose:** Record that `ring_index` declares no type of its own, name the three
it borrows, and show that the opacity split across them — two transparent, one
opaque — is what `of`'s totality actually rests on.

**Responsibility:** The crate's type vocabulary: where it comes from, what each
member protects, and which single one of them is load-bearing.

**In Scope:** `ring_index/src/lib.rs:24-26`;
`ring_types/src/id.rs:24-25, 98-99`;
`ring_types/src/capacity.rs:22-23, 40-78`.

**Out of Scope:** the `pub` field on `SlotIndex` and what it lets a caller do is
[`type/002`](002_the_sentence_the_public_field_contradicts.md). Where the
power-of-two rule is enforced is
[`decisions/001`](../decisions/001_a_power_of_two_or_nothing.md).

---

## The Whole Vocabulary, in One Line

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- types this crate declares --'
command grep -E '^\s*(pub )?(struct|enum|type|trait|union) ' ring_index/src/lib.rs || echo '  (none)'
echo '  -- types it uses --'
command grep '^use ' ring_index/src/lib.rs
```

Live output:

```
  -- types this crate declares --
  (none)
  -- types it uses --
use ring_types::{Capacity, Seq, SlotIndex};
```

---

### IX37 — Three Borrowed Types, Two Transparent and One Opaque

The three arrive with different amounts of protection:

```sh
cd "$(git rev-parse --show-toplevel)"
awk '/^\/\/\/ assert_eq!\( Seq\( 3 \)\.next\(\), Seq\( 4 \) \);$/{ n1 = NR } n1 && NR >= n1 + 2 && NR <= n1 + 3 { print } /^\/\/\/ assert_eq!\( SlotIndex\( 3 \)\.get\(\), 3 \);$/{ n2 = NR } n2 && NR >= n2 + 2 && NR <= n2 + 3 { print }' ring_types/src/id.rs
command grep -m1 -A1 -F '#[ derive( Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash ) ]' ring_types/src/capacity.rs
```

Live output:

```
#[ derive( Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default ) ]
pub struct Seq( pub u64 );
#[ derive( Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default ) ]
pub struct SlotIndex( pub usize );
#[ derive( Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash ) ]
pub struct Capacity( usize );
```

**Finding.** `Seq` and `SlotIndex` have public tuple fields; `Capacity` does not.
Every caller in the family can write `Seq( 12 )` or `SlotIndex( 3 )` out of thin
air, and no caller anywhere can write `Capacity( 12 )` — the only path in is
`Capacity::new`, which returns a `Result`.

The split is not arbitrary and it is not decoration. `Seq` and `SlotIndex` are
labels: they carry no invariant beyond "this number means a sequence" and "this
number means a slot," and the type system's whole contribution is refusing to let
one be passed where the other is expected. `Capacity` carries a real invariant —
power of two, non-zero — and is opaque for exactly that reason.

Two of the three derive lists are identical. `Capacity`'s differs in one
position, and the difference is downstream of the same fact: there is no sensible
default number of slots, so it does not derive `Default`, while `Seq::default()`
is sequence zero and `SlotIndex::default()` is slot zero and both are legal for
every ring that exists.

---

### IX38 — The Opacity Is What Makes `of` Total

`of`'s only precondition is that `capacity.mask()` is a sensible bitmask. That
holds because `Capacity::new` rejected zero and rejected non-powers-of-two
before the value ever reached this crate — and it holds *unconditionally*
because there is no second way to make a `Capacity`:

```sh
cd "$(git rev-parse --show-toplevel)"
echo "  Capacity constructions outside ring_types/src/capacity.rs: $( command grep -rn 'ring_types::Capacity( ' --include=*.rs */ | wc -l )"
command grep -E 'pub const fn (get|mask)' ring_types/src/capacity.rs
command grep -m1 -A3 -F '  pub const fn mask( self ) -> usize' ring_types/src/capacity.rs
```

Live output:

```
  Capacity constructions outside ring_types/src/capacity.rs: 0
  pub const fn get( self ) -> usize
  pub const fn mask( self ) -> usize
  pub const fn mask( self ) -> usize
  {
    self.0 - 1
  }
```

**Finding.** `mask` is `self.0 - 1` with no check. On a `usize` that is a
subtraction that underflows at zero — it would panic in a debug build and wrap in
a release one. `Capacity`'s privacy is the only thing standing between that line
and a caller, and it is sufficient: zero sites in the family construct a
`Capacity` by any route other than `new`.

What the wrap would produce, if the field were public:

```
  capacity 0 -> mask               : 18446744073709551615
  is that usize::MAX               : true
  of( 12345, that mask ) -> slot   : 12345
  a legal slot for 0 slots would be: none — the range 0..0 is empty
  and `slots - 1` checked          : None
```

`of` would become the identity function on the sequence number, returning an
unbounded `SlotIndex` that indexes straight into a buffer. That is the failure
the type prevents, and this crate does not participate in preventing it at all —
it inherits the guarantee whole from a type it does not own.

This is the concrete content of the module comment's claim that "the validation
lives upstream." It is not a division of labour. It is a *transfer*: the
invariant is enforced once, at a constructor in another crate, and every function
downstream is total as a consequence. `ring_index` is the first beneficiary and
adds nothing.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`type/002`](002_the_sentence_the_public_field_contradicts.md) | What the two transparent fields let callers do, and what `SlotIndex`'s doc says about it |
| [`algorithm/001`](../algorithm/001_one_and_of_a_mask.md) | The body that consumes `mask()` without checking it |
| [`decisions/001`](../decisions/001_a_power_of_two_or_nothing.md) | The one enforcement site, and what it costs a caller |
| [`invariant/002`](../invariant/002_the_fold_is_total_and_the_run_is_not.md) | The totality this opacity buys, and the function it does not cover |

### Sources

| Fact | Where |
|------|-------|
| The crate declares no types | Census above |
| The three declarations | `ring_types/src/id.rs:24-25, 98-99`; `ring_types/src/capacity.rs:22-23` |
| `mask` is an unchecked subtraction | `ring_types/src/capacity.rs:75-78` |
| What capacity zero would compute | Standalone `rustc -O` probe, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `non_power_of_two_capacity_is_rejected_upstream` | That the enforcement is upstream, asserted from this crate |
| `capacity_one_maps_everything_to_slot_zero` | The smallest legal capacity, where `mask` is `0` |
| `mask_equals_modulo_over_four_laps_of_every_capacity` | Every capacity that can exist, which is the set opacity defines |
