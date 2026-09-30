# Three Newtypes, and Two Open Fields

### Scope

- **Purpose**: Record that this crate declares three newtypes and encapsulates one of them, and measure what the other two cost in the thirty-one crates that construct them directly.
- **Responsibility**: State the asymmetry, its consequence for each type's stated invariant, and the construction paths the family actually takes.
- **In Scope**: `Capacity( usize )`, `Seq( pub u64 )`, `SlotIndex( pub usize )`, and every direct-construction site in `ring_*/src`.
- **Out of Scope**: `Capacity::new`'s validation itself — see [`algorithm/001`](../algorithm/001_validating_a_slot_count_to_a_power_of_two.md); the reachability argument the private field supports — see [`lifecycle/003`](../lifecycle/003_a_capacity_request_through_validation.md).

### The Asymmetry

```sh
cd "$(git rev-parse --show-toplevel)"
grep -E '^pub struct (Capacity|Seq|SlotIndex)' ring_types/src/capacity.rs ring_types/src/id.rs
```

Live output:

```
ring_types/src/capacity.rs:pub struct Capacity(usize);
ring_types/src/id.rs:pub struct Seq(pub u64);
ring_types/src/id.rs:pub struct SlotIndex(pub usize);
```

**One private field and two public ones**, in a crate whose entire job is to make
four values impossible to get wrong. `Capacity` cannot be built except through
`Capacity::new`; `Seq` and `SlotIndex` can be built by anyone, from any integer,
in any state.

### What the Open Fields Cost

| Type | Stated property | Enforced by | Bypassable by |
|------|-----------------|-------------|---------------|
| `Capacity` | Non-zero, power of two | The private field plus `new`'s two tests | Nothing — there is no other path |
| `Seq` | Monotonic; advanced through `next`/`advanced_by` | Nothing | `Seq( n )` — 7 production sites in 5 crates |
| `SlotIndex` | An index within a ring's capacity | Nothing | `SlotIndex( n )` — 1 production site |

`Seq`'s doc describes advancement as though `next` were the mechanism. It is a
convenience. The mechanism is the `u64` inside it, which every consumer can read
and write:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -rE 'Seq\( ?[a-z0-9_]|SlotIndex\( ?[a-z0-9_(]' ring_*/src/*.rs \
  | command grep -v '^ring_types/' | command grep -vE ':[ \t]*(//|///|//!)'
```

Live output:

```
ring_atomic/src/lib.rs:        Seq(self.0.load(order))
ring_atomic/src/lib.rs:        Seq(self.0.fetch_add(n, order))
ring_batch/src/lib.rs:        Seq(self.start.0 + self.count as u64)
ring_index/src/lib.rs:    SlotIndex((seq.0 as usize) & capacity.mask())
ring_mpsc/src/lib.rs:pub const UNSTAMPED: Seq = Seq(u64::MAX);
ring_mpsc/src/lib.rs:        if end == from { None } else { Some(Seq(end.0 - 1)) }
ring_tls/src/lib.rs:        let seq = Seq(self.next);
ring_trace/src/lib.rs:        Seq(self.seq.0.saturating_add(self.count as u64))
```

**Seven sites in five crates**, and the comment filter is doing real work here:
the same search without it reports seventeen crates, because the family's doc
examples construct sequences literally far more often than its code does
(→ [TY15](#ty15--every-seq-0-in-the-family-is-in-a-doc-comment)).

### This Is Not Obviously Wrong

The open field is what lets `ring_index` write `( seq.0 as usize ) & capacity.mask()`
without a conversion method, and what lets `ring_atomic` reconstruct a `Seq` from
an `AtomicU64` load without an accessor pair. One of the seven sites cannot be
replaced by a method at all under the current design — `ring_mpsc:222` declares
`pub const UNSTAMPED : Seq = Seq( u64::MAX )`, and a `const` initialiser needs a
`const fn` constructor, which is the same wall
[`../workaround/002`](../workaround/002_the_conversion_that_cannot_be_a_trait.md)
records for `Capacity`.

Sealing `Seq` is therefore a `const fn from_raw` plus seven call-site edits in
five crates — not the seventeen-crate change the unfiltered count suggests.

**The finding is not that the fields should be private. It is that the crate
argues one invariant into the type system and leaves two as documentation, and
says nowhere that it has done so.** A reader who trusts `Capacity`'s
encapsulation and generalises it to `Seq` will believe a `Seq` in hand has been
advanced rather than assembled.

### The Documentation Teaches the Spelling the Code Does Not Use

`Seq::ZERO` exists so that a zero sequence has a name. Both spellings appear in
the family; they do not appear in the same places.

```sh
cd "$(git rev-parse --show-toplevel)"
for p in 'Seq::ZERO' 'Seq(0)'; do
  all=$( grep -rF "$p" ring_*/src/*.rs | grep -vc '^ring_types/' )
  code=$( grep -rF "$p" ring_*/src/*.rs | grep -v '^ring_types/' \
          | grep -vcE ': *(//|///|//!)' )
  printf '%-12s all %3d   code %3d\n' "$p" "$all" "$code"
done
```

Live output:

```
Seq::ZERO    all  59   code   6
Seq(0)       all  19   code   0
```

**Every `Seq( 0 )` in every dependent crate is inside a doc comment.** Production
code uses the named constant exclusively. The literal spelling survives only in
the examples a consumer reads first — so the one construction form that would
stop compiling if the field were sealed is the form the crate's own
documentation demonstrates.

That inverts the sealing cost estimate. The code change is seven sites; the
documentation change is larger and lands in crates that would otherwise be
untouched.

### TY13 — One Newtype Is Encapsulated and Two Are Not

`Capacity`'s field is private; `Seq`'s and `SlotIndex`'s are `pub`. The crate
enforces one of its three newtype invariants in the type system and states the
other two in prose, with nothing recording that the three are not alike.

### TY14 — Five Dependents Build `Seq` From a Raw Integer at Seven Sites

`ring_atomic`, `ring_batch`, `ring_mpsc`, `ring_tls` and `ring_trace` construct
`Seq( .. )` directly in production code. `Seq::next` and `Seq::advanced_by` are
therefore conveniences over an open field rather than the advancement mechanism
their documentation describes, and no monotonicity property of a received `Seq`
can be relied on by the crate that receives it.

### TY15 — Every `Seq( 0 )` in the Family Is in a Doc Comment

The literal spelling appears in dependent crates only inside `///` and `//!`
examples; production code uses `Seq::ZERO` exclusively. The construction form the
crate's own documentation demonstrates is the one form that would stop compiling
if the field were sealed, so the documentation cost of encapsulating `Seq`
exceeds its code cost and falls on crates the code change would not touch.

### TY16 — `SlotIndex` Reaches Three Dependents at Nine Lines

`SlotIndex` is named in production code in `ring_batch` (2 lines), `ring_store`
(3) and `ring_index` (4), and nowhere else. It is by a wide margin the
least-adopted of the six exported names — against `RingError`'s 18 crates,
`Seq`'s 16 and `Capacity`'s 11 — and the three that use it are the three that own
the sequence-to-slot fold.

### Items

| File | Relationship |
|------|--------------|
| [`struct/001_capacity.md`](struct/001_capacity.md) | The encapsulated one |
| [`struct/002_seq.md`](struct/002_seq.md) | The open one, named in 16 crates |
| [`struct/003_slot_index.md`](struct/003_slot_index.md) | The open one, named in 3 |
| [`001_the_forty_items_and_the_six_the_family_calls.md`](001_the_forty_items_and_the_six_the_family_calls.md) | The callable surface these construction paths bypass |
