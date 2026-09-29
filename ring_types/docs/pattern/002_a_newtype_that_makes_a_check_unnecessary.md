# Pattern: A Newtype That Makes a Check Unnecessary

### Scope

- **Purpose**: Record the pattern `Capacity` implements — a validating constructor behind a private field, so that seventeen consumer crates may use an unchecked `mask()` without re-testing anything — and measure what it actually bought against the two newtypes in the same crate that deliberately do not use it.
- **Responsibility**: State the problem, solution, applicability, and consequences.
- **In Scope**: `Capacity`'s private field and sole constructor; the zero re-validations downstream; the contrast with `Seq` and `SlotIndex`, whose fields are public on purpose.
- **Out of Scope**: The validation as an algorithm (→ [`algorithm/001`](../algorithm/001_validating_a_slot_count_to_a_power_of_two.md)); the invariant it establishes (→ [`invariant/001`](../invariant/001_every_capacity_has_a_valid_mask.md)).

### Problem

**A ring folds a sequence to a slot with `seq & (slots - 1)`, and that is
correct only if `slots` is a power of two.** Nothing in a `usize` says so.

Passed around raw, the number arrives at each use site with its validity
unknown, and every site faces the same three bad options:

| Option | Cost |
|--------|------|
| Re-check at each use | The check is duplicated across every crate that folds, and each copy can drift |
| Check once at the boundary and trust thereafter | The trust is a convention; the next call site added does not know about it |
| Use `%` instead of `&` | Correct for any `slots`, and a division on the hot path |

The third is the one worth naming, because it is the option this pattern trades
away. `seq % slots` needs no invariant at all. Choosing `&` is choosing to make
the number's shape a precondition — and a precondition held by convention across
seventeen crates is a precondition that will eventually not hold.

**The failure mode is silence.** A non-power-of-two `slots` makes `slots - 1`
a mask with interior zero bits, so the fold lands on a subset of the slots,
repeatedly, forever. No panic, no error, no out-of-bounds — just a ring that
uses some of its storage and silently aliases the rest
(→ [`type/001`](../type/001_capacity.md)).

### Solution

**Put the number behind a newtype whose only constructor validates, and make
the field private.**

```rust
pub struct Capacity( usize );

impl Capacity
{
  pub const fn new( slots : usize ) -> Result< Self, RingError > { /* two tests */ }
  pub const fn get( self ) -> usize { self.0 }
  pub const fn mask( self ) -> usize { self.0 - 1 }
}
```

**The private field is the load-bearing part, not the validation.** A public
field would leave `Capacity( 6 )` writable from any of the seventeen consumer
crates, and the constructor would become advice. With the field private,
`Capacity::new` is the only way to obtain the type, so **holding a `Capacity` is
proof the check ran** — not evidence that it probably did.

`mask()` is then unchecked by design. `self.0 - 1` cannot underflow because zero
was rejected, and the result has no interior zero bits because non-powers-of-two
were rejected. Neither fact is re-established; both are inherited from
construction.

**What it bought, measured.** No crate outside this one re-validates:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -rn 'is_power_of_two' ring_*/src | command grep -vc '^ring_types/'
```

Live output:

```
1
```

Seventeen crates hold a `Capacity`, and **not one of them tests the number
again.** The single hit above is not a re-check either — it is `ring_align`'s
`CACHE_LINE.is_power_of_two()` compile-time assertion, unrelated to `Capacity`
(→ [`../invariant/001`](../invariant/001_every_capacity_has_a_valid_mask.md),
TY34). The check exists once, in one function, in the crate with no
dependencies.

The construction side is smaller than a raw grep suggests, and the difference is
worth separating:

```sh
cd "$(git rev-parse --show-toplevel)"
ext=$( command grep -rn 'Capacity::new' ring_*/src | command grep -v '^ring_types/' )
echo "total external references: $( echo "$ext" | wc -l )"
echo "doc/comment mentions:      $( echo "$ext" | command grep -cE ':[0-9]*: *//' )"
echo "production call sites:     $( echo "$ext" | command grep -vcE ':[0-9]*: *//' )"
```

Live output:

```
total external references: 105
doc/comment mentions:      104
production call sites:     1
```

**One production call site, not 105.** Ninety-nine percent of the references
are documentation examples or comments mentioning the name — which is its own
signal about a vocabulary crate, where being *shown* is most of what a type
does. The one real site is `ring_config/src/lib.rs:71`, the family's single
entry point for a configured capacity
([`../item/associated_function/001`](../item/associated_function/001_capacity_new.md)
covers the second site this used to have, in `ring_core`, and why it stopped).

### Applicability

**Apply when a raw value carries a precondition that more than one consumer
depends on, and the precondition is cheap to establish and expensive to
re-establish everywhere.**

| Signal | `Capacity` | `Seq` | `SlotIndex` |
|--------|:----------:|:-----:|:-----------:|
| Some values of the underlying type are invalid | ✅ zero, non-powers-of-two | ❌ every `u64` is a valid position | ❌ every `usize` is a valid index |
| More than one consumer relies on the precondition | ✅ 17 crates | — | — |
| A violation fails silently rather than loudly | ✅ | — | — |
| **Field visibility** | **private** | **`pub`** | **`pub`** |

**The bottom row is the pattern's own boundary, drawn inside this crate.**
`Seq` and `SlotIndex` are newtypes of exactly the same syntactic shape whose
fields are deliberately public:

```rust
pub struct Seq( pub u64 );
pub struct SlotIndex( pub usize );
```

They buy something different — **non-interchangeability, not validity.** A
function taking `( Seq, Capacity )` cannot be called with the arguments
transposed, and a `SlotIndex` cannot be added to a `Seq` by accident. Neither
guarantee needs a constructor, because there is no invalid `u64` position to
exclude. Hiding those fields would cost 187 direct `Seq( … )` constructions
across the family a constructor call each, and buy nothing:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -rnE '\bSeq\( ' ring_*/src | command grep -vc '^ring_types/'
```

Live output:

```
187
```

**So the pattern is applied once out of three opportunities, and the two
declines are the evidence it was applied deliberately.** A crate that wrapped
everything in a private field would be applying a habit; this one distinguishes
the value with an invariant from the two values with only an identity.

### Consequences

**Benefit — the check is unforgeable rather than merely conventional.** There is
no `Default` impl, no public field, and no second constructor, so every
`Capacity` in the workspace passed the same two tests. That is what lets
`ring_index` fold with `&` instead of `%`.

**Benefit — the error names the number.** Because validation happens where the
number is still a `usize`, `CapacityNotPowerOfTwo( n )` can carry `n`. A check
performed later, at the fold, would have only a mask to report.

**Cost — the guarantee travels, but the code that uses it does not
centralise.** The fold's ruled owner is `ring_index::of`, three items in the
family's narrowest crate. It has two call sites for `mask()` family-wide, and
one of them is not it:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -r '\.mask(' ring_*/src | command grep -v '^ring_types/'
```

Live output:

```
ring_index/src/lib.rs:  SlotIndex( ( seq.0 as usize ) & capacity.mask() )
ring_mpsc/src/lib.rs:    let index = ( seq.0 as usize ) & self.capacity().mask();
```

`ring_mpsc` — the family's largest crate at 50 items — writes `ring_index::of`'s
body inline rather than calling it, and its manifest does not list `ring_index`
among its eight dependencies. Only three crates depend on `ring_index` at all.

**That duplication is safe, and the pattern is why.** The inlined fold is
correct for exactly the same reason the ruled one is: it holds a `Capacity`, so
the mask is valid regardless of which crate does the `&`. The cost is a second
site to edit if the fold ever changes — a maintenance cost, not a correctness
risk. **This is the pattern's real return: it makes duplicating the operation
harmless, which is a different and stronger property than preventing the
duplication.**

**Cost — one lost opportunity, and it is nameable.** `Capacity` wraps a `usize`
where a `NonZeroUsize` would let the compiler carry half the invariant. The
zero-rejection would become the type's own, `CapacityZero` would lose its
reason to exist, and `mask()`'s underflow-freedom would be structural rather
than argued. The change is not free — `NonZeroUsize::new` returns an `Option`
the constructor must thread — and it is deferred rather than declined
(→ [`decisions/readme.md`](../decisions/readme.md)).

**Cost — the pattern cannot protect a value it does not wrap.** `Seq`'s public
field is the right call, and it means nothing prevents `Seq( u64::MAX )` from
being constructed and then advanced. That is the shape
[`pitfall/001`](../pitfall/001_seq_next_wraps_where_its_doc_says_it_saturates.md)
records: no invariant, no constructor, and a doc comment doing the work a type
would have done. The choice is still correct — a position genuinely has no
invalid values within any reachable workload — but the asymmetry is worth
stating plainly rather than leaving as an accident of which type got which
treatment.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_validating_a_slot_count_to_a_power_of_two.md](../algorithm/001_validating_a_slot_count_to_a_power_of_two.md) | The two tests the constructor runs, and why their order is about the error rather than correctness |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_vocabulary_surface.md](../api/001_the_vocabulary_surface.md) | G4 — the guarantee the private field carries |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_two_position_types_and_the_fold_between_them.md](../data_structure/001_two_position_types_and_the_fold_between_them.md) | Why `Seq` and `SlotIndex` are public-field newtypes, at length |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_every_capacity_has_a_valid_mask.md](../invariant/001_every_capacity_has_a_valid_mask.md) | The property this pattern establishes, and the three mechanisms that hold it |

### Items

| File | Relationship |
|------|--------------|
| [../item/struct/001_capacity.md](../item/struct/001_capacity.md) | The private field |
| [../item/struct/002_seq.md](../item/struct/002_seq.md) | The public one, and 187 constructions that rely on it |
| [../item/associated_function/001_capacity_new.md](../item/associated_function/001_capacity_new.md) | The sole constructor |
| [../item/associated_function/003_capacity_mask.md](../item/associated_function/003_capacity_mask.md) | The unchecked consumer of the invariant |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_a_slot_count_from_request_to_mask.md](../lifecycle/001_a_slot_count_from_request_to_mask.md) | L2 as the one phase that can refuse, and the seventeen crates downstream of it |

### Patterns

| File | Relationship |
|------|--------------|
| [001_discriminants_here_handlers_elsewhere.md](001_discriminants_here_handlers_elsewhere.md) | The crate's other structural pattern — moving a behaviour outward rather than a check inward |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_seq_next_wraps_where_its_doc_says_it_saturates.md](../pitfall/001_seq_next_wraps_where_its_doc_says_it_saturates.md) | What the un-wrapped newtype leaves to a doc comment |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/003_a_capacity_request_through_validation.md](../lifecycle/003_a_capacity_request_through_validation.md) | The states a slot count occupies, and the one the private field makes unreachable |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_capacity.md](../type/001_capacity.md) | V1–V5 — the rules the pattern encodes |

### Sources

| File | Relationship |
|------|--------------|
| [`src/capacity.rs`](../../src/capacity.rs) | Line 23's private field; lines 40–51 the constructor; line 75 the unchecked `mask` |
| [`src/id.rs`](../../src/id.rs) | Lines 25 and 99 — the two `pub` fields, the deliberate declines |
| [`ring_index/src/lib.rs`](../../../ring_index/src/lib.rs) | Line 51 — the fold the invariant exists for |
| [`ring_mpsc/src/lib.rs`](../../../ring_mpsc/src/lib.rs) | Line 543 — the same fold, inlined, correct for the same reason |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/types_test.rs`](../../tests/types_test.rs) | ✅ `capacity_rejects_zero_and_non_powers_of_two` and `capacity_accepts_powers_of_two` cover the constructor; `slot_index_is_its_own_type` covers the non-interchangeability the public-field newtypes buy. ❌ **The private field itself is unasserted and untestable** — a test that tried `Capacity( 6 )` would not compile, so the pattern's central mechanism can only be verified by reading the declaration. That is the correct outcome, not a gap: a compile error is a stronger guarantee than a passing test, but it leaves nothing in the suite to point at |

### TY48 — The Pattern Succeeded, and Its Evidence Is an Absence

The pattern claims that wrapping a validated integer removes the need for
downstream checks. The claim is confirmed by
[`../invariant/001`](../invariant/001_every_capacity_has_a_valid_mask.md)'s TY34:
zero hand-rolled checks family-wide.

**Recording the shape of that evidence matters as much as the result.** An
absence cannot distinguish "nobody needed to re-check" from "nobody remembered
to", and it will keep reading zero after the invariant is deleted. The
distinguishing evidence is that seventeen crates hold a `Capacity` they could
not have built wrongly, which is a property of the private field
(→ [`../item/002`](../item/002_three_newtypes_and_two_open_fields.md), TY13) —
not of the count.
