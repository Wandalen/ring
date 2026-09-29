# Decisions: Panic Rather Than Option

### Scope

**Purpose:** Record the decision to panic on an out-of-range slot index rather
than return an `Option`, the argument the source gives for it, and that the
argument's premise — that such an index cannot reach the crate — is not held by
the type system and is contradicted twenty-five times inside this crate.

**Responsibility:** The out-of-range decision: what was chosen, why, and what the
reasoning actually rests on.

**In Scope:** `ring_store/src/lib.rs:195-202`;
`ring_types/src/id.rs:88-99`; the index-construction census.

**Out of Scope:** What the `# Panics` claim rests on at the type level
is [`type/002`](../type/002_the_types_that_cross_the_boundary.md). The
`SlotIndex` type itself is [`item/002`](../item/002_the_slotindex_the_buffer_trusts.md).

---

### BF6 — The Decision Is Argued From a Property of the Producer, Not of the Type

Most containers returning a borrow by index either panic or hand back an
`Option`. This one panics, and unusually, says why:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A13 -F '  /// Borrow the slot at `index`.' ring_store/src/lib.rs
```

Live output:

```
  /// Borrow the slot at `index`.
  ///
  /// # Panics
  ///
  /// If `index` is at or beyond the capacity. Not an error return: a
  /// `SlotIndex` is not validated against this buffer's capacity — one built
  /// any other way than through `ring_index::of` for this same capacity is
  /// the caller's responsibility, and one that reached here out of range is a
  /// caller that mixed two rings' capacities, which is a defect rather than a
  /// condition to handle.
  #[ must_use ]
  pub fn get( &self, index : SlotIndex ) -> &S
  {
    &self.slots[ index.get() ]
```

The reasoning is sound on its own terms and better than most. It does not say
"indexing panics"; it says an out-of-range index means two rings' capacities were
mixed, and offers a reason that condition should surface rather than be folded
into an `Option` the caller would unwrap anyway.

**Finding.** The argument turns entirely on the sentence *a `SlotIndex` reaching
this crate came from `ring_index::of`*. That is a claim about where values come
from, not about what the type permits — and the decision inherits whatever
strength that claim has. If every `SlotIndex` really is produced by the fold, the
panic is unreachable in correct code and the choice is free. If not, the panic is
an ordinary bounds check wearing an argument, and the message it produces names
neither the ring nor the capacity that were mixed.

The rest of this instance establishes which of those it is.

---

### BF7 — The Premise Is Not Held by the Type, and the Publicness That Breaks It Is Load-Bearing

`SlotIndex` is a newtype with a public field:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A11 -F '/// A position within a ring'"'"'s storage, always in `0..capacity`.' ring_types/src/id.rs
```

Live output:

```
/// A position within a ring's storage, always in `0..capacity`.
///
/// Derived from a [`Seq`] by `ring_index`, never constructed by counting.
/// Two sequences a full lap apart produce the identical `SlotIndex`, which is
/// the whole reason [`Seq`] exists as a separate type.
///
/// ```
/// use ring_types::SlotIndex;
/// assert_eq!( SlotIndex( 3 ).get(), 3 );
/// ```
#[ derive( Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default ) ]
pub struct SlotIndex( pub usize );
```

Its own prose says *never constructed by counting*, and its own doctest, three
lines above the struct, constructs one by counting. That is not carelessness on
either part: both are true statements about different things. The prose describes
the discipline; the `pub` describes the permission.

The permission is not incidental. Search the family's executable code for the
tuple constructor and it appears twice — once as the definition, once in the one
function entitled to use it:

```sh
cd "$(git rev-parse --show-toplevel)"
# sorted: grep is shimmed to a parallel ugrep here, so hits arrive in completion order
grep -r 'SlotIndex( ' ring_*/src/*.rs | grep -vE ':[[:space:]]*(///|//!|//)' | sed 's|^ring/||' | sort
```

Live output:

```
ring_index/src/lib.rs:  SlotIndex( ( seq.0 as usize ) & capacity.mask() )
ring_types/src/id.rs:pub struct SlotIndex( pub usize );
```

`ring_index::of` is a *different crate* from `ring_types`. Rust offers no
visibility that admits a sibling crate and excludes everyone else, so making the
fold possible at all requires making the field public to the world. The
publicness is doing real work, and the discipline the prose describes is the only
thing standing between it and arbitrary construction.

This crate does not observe that discipline:

```sh
cd "$(git rev-parse --show-toplevel)"
printf '  SlotIndex( n ) written by hand in this crate: %s\n' "$( grep -rho 'SlotIndex( [0-9]' ring_store/src/lib.rs ring_store/tests/buffer_test.rs | wc -l )"
printf '  SlotIndex( i ) written by hand in this crate: %s\n' "$( grep -rho 'SlotIndex( [a-z]' ring_store/src/lib.rs ring_store/tests/buffer_test.rs | wc -l )"
printf '  produced by ring_index::of instead:           %s\n' "$( grep -rho 'of( seq\|of( Seq' ring_store/src/lib.rs ring_store/tests/buffer_test.rs | wc -l )"
```

Live output:

```
  SlotIndex( n ) written by hand in this crate: 20
  SlotIndex( i ) written by hand in this crate: 5
  produced by ring_index::of instead:           3
```

Twenty-five hand-built indices against three folded ones, in the doctests and
tests of the very crate whose panic documentation asserts that indices arrive
folded. And it is measurably free to build one out of range:

```
--- (3) the index the type says cannot exist ---
  SlotIndex( 999 ) constructed with no unsafe and no ring_index: SlotIndex(999)
  its .get() = 999
  Buffer::get( it ) panicked = true
```

**Finding.** The decision to panic is right and its stated justification is not
load-bearing in the way it reads. *An index that is out of range is a caller that
mixed two rings' capacities* describes one way to get there; typing `SlotIndex(
999 )` is another, needs no `unsafe`, no `ring_index`, and no second ring, and is
how this crate's own suite produces every index it uses.

Nothing here argues for a different decision. Panicking is still correct: an
`Option` would push a `.unwrap()` into every call site on the ring's hot path to
handle a case that only a defect produces. What is worth recording is that the
*reason* offered is weaker than the reader is invited to think, and the gap is
one sentence wide. Saying "a `SlotIndex` is not validated against this buffer's
capacity; one produced by `ring_index::of` for the same capacity is always in
range, and one built any other way is the caller's responsibility" would cost
nothing and would survive contact with the test file.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -A4 -F 'is not validated against' ring_store/src/lib.rs
```

Live output:

```
  /// `SlotIndex` is not validated against this buffer's capacity — one built
  /// any other way than through `ring_index::of` for this same capacity is
  /// the caller's responsibility, and one that reached here out of range is a
  /// caller that mixed two rings' capacities, which is a defect rather than a
  /// condition to handle.
```

**Disposition:** applied — `Buffer::get`'s `# Panics` doc in
`ring_store/src/lib.rs` no longer argues from where a `SlotIndex`
came from; it now states this instance's own proposed sentence — that a
`SlotIndex` is not validated against the buffer's capacity at all, that one
folded through `ring_index::of` for the same capacity is always in range,
and that one built any other way is the caller's responsibility. Now
prints: `is not validated against`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`type/002`](../type/002_the_types_that_cross_the_boundary.md) | The two unvalidated types the `# Panics` claim rests on |
| [`item/002`](../item/002_the_slotindex_the_buffer_trusts.md) | `SlotIndex` as a value crossing the boundary |
| [`invariant/002`](../invariant/002_two_distinct_indices_never_alias.md) | The non-aliasing property, which holds for folded indices and not for hand-built ones |
| [`decisions/002`](002_a_length_kept_to_be_checked_against_itself.md) | The crate's other argued decision, and its own cost |
| [`algorithm/001`](../algorithm/001_one_mask_no_modulo.md) | `ring_index::of`, the producer the argument names |

### Sources

| Fact | Where |
|------|-------|
| The panic decision as argued | `ring_store/src/lib.rs:195-202` |
| `SlotIndex`'s prose and its public field | `ring_types/src/id.rs:88-99` |
| The only sanctioned construction site | `ring_index/src/lib.rs:51` |
| Hand-built against folded, in this crate | Census, quoted above |
| An out-of-range index built and dereferenced | Release probe, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `an_index_past_the_capacity_panics_rather_than_wrapping` | The panic itself — using a hand-built `SlotIndex( 4 )` |
| `a_sequence_addresses_the_slot_ring_index_says_it_does` | The folded path, the one the argument describes |
| `two_distinct_slot_indices_never_alias` | Twenty-five hand-built indices, none of them out of range |
| *(to create)* | A `# Panics` note stating the index is unvalidated, asserted by a doctest |
