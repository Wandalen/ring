# Pattern: A Trait on the Varying Side and One Function Over It

### Scope

**Purpose:** Name the shape this crate is built out of, locate every other
instance of it in the family, and compare the one that is wired against the one
that is not.

**Responsibility:** The family's four public traits and nine impls, its one
generic associated type, its fifteen trait-generic free functions, and the single
production call that crosses a crate boundary through one.

**In Scope:** `ring_atomic/src/lib.rs:108`, `:211`, `:474`;
`ring_cursor/src/lib.rs:199`; `ring_event/src/lib.rs:53`, `:103`,
`:106`, `:161-204`; `ring_slot/src/lib.rs:41`;
`ring_batch/src/lib.rs:221`; `ring_tls/src/lib.rs:280`.

**Out of Scope:** The extension point's coherence limits are
[`item/002`](../item/002_four_impls_and_what_the_blanket_one_does_not_claim.md).
Why the read half needs a GAT is
[`type/001`](../type/001_an_associated_type_with_a_lifetime.md).

---

## Every Instance of the Shape in Thirty-Three Crates

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- every public trait in the 33-crate family --'
command grep -r '^pub trait ' --include=*.rs ring_*/src
echo '  -- and every impl of one --'
command grep -r 'impl.* SeqCell for \|impl.* Slot for \|impl.* Fill< \|impl.* Peek for ' --include=*.rs ring_*/src
echo '  -- every generic associated type in the family --'
command grep -r 'type [A-Za-z]*< .a >' --include=*.rs ring_*/src
echo '  -- public free functions generic over a type parameter, per crate --'
for c in ring_*/; do
  n=$( command grep -rhc '^pub fn [a-z_]*<' --include=*.rs "$c"src 2>/dev/null | awk '{ s += $1 } END { print s + 0 }' )
  [ "$n" -gt 0 ] && printf '%-18s %s\n' "$( basename $c )" "$n"
done
echo '  -- and the one production call of a trait-generic free function outside its crate --'
command grep 'let claim = claim(' ring_tls/src/lib.rs
command grep -m1 -A1 -F 'pub fn claim< C : SeqCell >( cursor : &C, count : usize, order : Ordering ) -> BatchClaim' ring_batch/src/lib.rs
```

Live output:

```
  -- every public trait in the 33-crate family --
ring_atomic/src/lib.rs:pub trait SeqCell : Sync
ring_event/src/lib.rs:pub trait Fill< S >
ring_event/src/lib.rs:pub trait Peek
ring_slot/src/lib.rs:pub trait Slot
  -- and every impl of one --
ring_atomic/src/lib.rs:impl SeqCell for AtomicSeq
ring_atomic/src/lib.rs:impl SeqCell for CountingSeq
ring_cursor/src/lib.rs:impl SeqCell for PaddedCursor
ring_event/src/lib.rs:impl< T > Fill< TypedSlot< T > > for T
ring_event/src/lib.rs:impl< const N : usize > Fill< BytesSlot< N > > for &[ u8 ]
ring_event/src/lib.rs:impl< T > Peek for TypedSlot< T >
ring_event/src/lib.rs:impl< const N : usize > Peek for BytesSlot< N >
ring_slot/src/lib.rs:impl< T > Slot for TypedSlot< T >
ring_slot/src/lib.rs:impl< const N : usize > Slot for BytesSlot< N >
  -- every generic associated type in the family --
ring_event/src/lib.rs:  type Out< 'a > where Self : 'a;
ring_event/src/lib.rs:  type Out< 'a > = &'a T where T : 'a;
ring_event/src/lib.rs:  type Out< 'a > = &'a [ u8 ];
  -- public free functions generic over a type parameter, per crate --
ring_batch         2
ring_debug         1
ring_event         3
ring_poll          4
ring_shutdown      1
ring_testkit       2
ring_wait          2
  -- and the one production call of a trait-generic free function outside its crate --
    let claim = claim( cursor, self.items.len(), order );
pub fn claim< C : SeqCell >( cursor : &C, count : usize, order : Ordering ) -> BatchClaim
{
```

---

### EV37 — Two of the Family's Four Traits and All of Its Generic Associated Types Are Here

Thirty-three crates declare four public traits between them: `SeqCell` in
`ring_atomic`, `Slot` in `ring_slot`, and `Fill` and `Peek` in this one. Half the
family's abstraction surface is declared in a crate of 234 lines with two
executable statements.

The generic associated type census is starker. `Peek::Out` and its two definitions
are the only `type X< 'a >` declarations anywhere in the 33 crates — the most
advanced type-level construct in the codebase, in the crate with the least code.

The shape those pieces make is consistent across all three of the crate's
functions: put the varying behaviour behind a trait implemented on whichever side
actually varies, then write the operation exactly once as a free function generic
over that trait. `publish_into` varies by payload, so `Fill` sits on the payload.
`drain_from` varies by slot, so `Peek` sits on the slot. `recycle` varies by slot
too, and reuses `ring_slot`'s existing `Slot` rather than declaring a third trait.

**Finding.** This is a coherent, deliberate design pattern and the crate is its
purest expression in the family — the only one whose entire public surface is
traits plus generic functions over them, with no data of its own at all. The
module documentation explains each decision individually, in the language of the
specific problem ("adding a payload kind never touches the slot types", "the two
shapes genuinely return different things"), and never steps back to name the
shape or say that all three functions are the same shape applied three times.
For a crate whose stated value is that there is only one of each operation, the
generalisation is the thing worth writing down.

---

### EV38 — The Family Already Runs This Pattern Across a Crate Boundary, Just Not This One

`SeqCell` is the same shape: a trait declared in one crate, implemented by types
that vary, consumed by a free function generic over it. And it works. Three types
implement it, and they are spread across two crates — `AtomicSeq` and
`CountingSeq` in `ring_atomic`, `PaddedCursor` in `ring_cursor`, a different crate
entirely. `ring_batch::claim< C : SeqCell >` is the generic function over it, and
`ring_tls/src/lib.rs:280` calls it from production code: `claim( cursor, … )`.

So the family has a working proof that the pattern carries across a boundary: a
downstream crate supplied an implementor, and a third crate called the generic
function without knowing which implementor it had.

`Fill` and `Peek` have four impls between them and all four are in the crate that
declares the traits. The generic functions over them have no production caller at
all — one manifest declares this crate and does so under `[dev-dependencies]`
([`integration/001`](../integration/001_one_declarer_and_it_is_a_dev_dependency.md)).

**Finding.** The difference between the two is not the pattern. `SeqCell` and
`Peek` are the same construct, applied at the same level, in the same family, and
one of them is load-bearing while the other is exercised only by its own tests.
That is worth stating precisely because the usual doubt about a design like this
— whether the abstraction will actually be taken up — has already been answered
in the affirmative next door. What is missing here is not evidence that the shape
works; it is a caller. The comparison also gives a concrete model for what
adoption looks like: `ring_cursor` did not need a new crate or a redesign to
implement `SeqCell`, only the trait's own crate as a dependency and an impl block.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`pattern/002`](002_proof_by_monomorphisation.md) | How the suite exploits the same shape |
| [`decisions/001`](../decisions/001_the_write_half_on_the_payload_the_read_half_on_the_slot.md) | Why each trait sits on the side it does |
| [`integration/001`](../integration/001_one_declarer_and_it_is_a_dev_dependency.md) | The caller this instance of the pattern lacks |
| [`api/001`](../api/001_two_traits_three_functions_one_associated_type.md) | What the shape costs at a call site |

### Sources

| Fact | Where |
|------|-------|
| Four public traits in 33 crates | `ring_atomic/src/lib.rs:108`, `ring_slot/src/lib.rs:41`, `ring_event/src/lib.rs:53`, `:103` |
| Nine impls, and where each lives | Census above |
| `Peek::Out` being the family's only GAT | `ring_event/src/lib.rs:106`, `:129`, `:139` |
| Fifteen trait-generic free functions across seven crates | Census above |
| `SeqCell` implemented in a crate that does not declare it | `ring_cursor/src/lib.rs:199` |
| A production call through the generic function | `ring_tls/src/lib.rs:280`, `ring_batch/src/lib.rs:221` |

### Tests

| Test | Covers |
|------|--------|
| `fill_is_implemented_on_the_payload_so_a_new_payload_needs_no_slot_change` | The pattern's write half, named in the test itself |
| `peek_is_implemented_on_the_slot_so_the_reader_needs_no_payload_type` | Its read half, likewise |
| `one_generic_body_round_trips_a_typed_slot` | The single function the pattern buys |
