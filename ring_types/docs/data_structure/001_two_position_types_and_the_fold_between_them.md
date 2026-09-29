# Data Structure: Two Position Types and the Fold Between Them

### Scope

- **Purpose**: Describe `Seq` and `SlotIndex` as one design — two 8-byte newtypes over different integers, deliberately not convertible into each other — and record that the derived one reaches three crates while the source reaches eighteen.
- **Responsibility**: State the abstract, the structure, and the operations.
- **In Scope**: Both types' layout, their derives, the asymmetry in their operation sets, and where the fold that connects them lives.
- **Out of Scope**: The bitmask the fold uses (→ [`type/001`](../type/001_capacity.md)); `Seq::next`'s incorrect overflow documentation (→ [`pitfall/001`](../pitfall/001_seq_next_wraps_where_its_doc_says_it_saturates.md)).

### Abstract

**One position, two representations, and no conversion between them in this
crate.** A `Seq` counts publications for a ring's whole history and never
wraps; a `SlotIndex` names a cell in storage and wraps every lap. They are the
same underlying quantity at different resolutions, and they are separate types
so the compiler refuses to compare one against the other.

The reason is plain: a gate must be able to compare two
positions that are a full lap apart, and that comparison is impossible once both
have been folded into `0..capacity`. Two sequences 16 apart in a 16-slot ring
produce the identical `SlotIndex`, so `SlotIndex` equality answers a different
question from `Seq` equality — and a type error is the only mechanism that stops
a reader from asking the wrong one.

**The fold itself is not here.** `ring_index` owns it. This crate owns the two
endpoints and the guarantee that they cannot be confused.

### Structure

```rust
pub struct Seq( pub u64 );        // 8 bytes, align 8
pub struct SlotIndex( pub usize );  // 8 bytes, align 8
```

| | `Seq` | `SlotIndex` |
|---|-------|-------------|
| Inner type | `u64` | `usize` |
| Size / align | 8 / 8 | 8 / 8 |
| Field visibility | **`pub`** | **`pub`** |
| Wraps | No — 584 years at 10⁹/s | Yes, every lap |
| Derives | `Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default` | identical |
| Associated const | `ZERO` | — |
| Methods | `next`, `advanced_by`, `distance_to` | `get` |
| Crates naming it | **18** | **3** |

**`u64` versus `usize` is deliberate and is the one structural difference.** A
`Seq` is a count, and its non-wrapping guarantee is a width argument — 2⁶⁴
publications is 584 years at a billion per second, so the wrap point is
unreachable. Pinning it to `u64` keeps that argument true on a 32-bit target,
where `usize` would be 32 bits and the same argument would give **4.3 seconds**.
A `SlotIndex` is an offset into memory, so `usize` is the correct type by
definition. On the family's 64-bit targets the two are the same width, which is
exactly why the distinction has to be made on purpose.

**Both fields are `pub`, and both types still work.** The privacy that makes
[`Capacity`](../type/001_capacity.md) a validating newtype is absent here,
because neither position has a validity rule to protect — every `u64` is a legal
`Seq` and every `usize` is a legal `SlotIndex`. What these two newtypes buy is
not validation but **non-interchangeability**, and that is delivered by being
distinct nominal types regardless of field visibility.

```rust
let s = Seq( 5 );
let i = SlotIndex( 5 );
// s == i          // does not compile — no PartialEq< SlotIndex > for Seq
assert_eq!( s.0, i.0 as u64 );   // the escape hatch, and it is explicit
```

The `pub` fields mean the escape hatch exists, and requiring `.0` and an `as`
cast makes taking it visible in review. A private field would forbid it outright
at the cost of a constructor and an accessor on each type — more surface for a
guarantee the `as` cast already makes conspicuous.

**Both derive `Default`, and for `Seq` that is meaningful:** `Seq::default()`
is `Seq( 0 )`, the position of a ring that has published nothing, which is also
what the `ZERO` associated const names. Two spellings of the same value, one
inherited from a derive and one written for readability at a call site
(`Seq::ZERO` reads better than `Seq::default()` in a cursor initialiser).

**The three-versus-eighteen split is the structural finding.**

```sh
cd "$(git rev-parse --show-toplevel)"
for t in Seq SlotIndex; do
  printf '%-12s %s\n' "$t" \
    "$( command grep -rlE "\b$t\b" ring_*/src | command grep -v '^ring_types/' \
         | cut -d/ -f2 | sort -u | tr '\n' ' ' )"
done
```

Live output:

```
Seq          ring_atomic ring_barrier ring_batch ring_store ring_claim ring_consume ring_core ring_cursor ring_debug ring_gating ring_index ring_mpsc ring_publish ring_seqno ring_spsc ring_tls ring_trace ring_wait 
SlotIndex    ring_batch ring_store ring_index 
```

**`SlotIndex` reaches only the fold and the two crates that address storage.**
`ring_index` performs the fold, `ring_store` indexes with the result, and
`ring_batch` carries a run of them. Every other crate in the family works in
`Seq` — gates, cursors, claims, publication, tracing, waiting — because every
other question is about ordering, and ordering is what folding destroys.

**This is the type split working, not failing.** A `SlotIndex` that travelled as
widely as `Seq` would mean the family was passing folded positions around, which
is the exact hazard the split exists to prevent. The number is low because the
design confines it.

### Operations

**`Seq` — three operations, all `const`, all `#[ must_use ]`:**

| Operation | Signature | Semantics | Callers outside this crate |
|-----------|-----------|-----------|---------------------------:|
| [`next`](../item/associated_function/007_seq_next.md) | `const fn next( self ) -> Self` | `self.0 + 1` | 4 src, 11 test |
| [`advanced_by`](../item/associated_function/008_seq_advanced_by.md) | `const fn advanced_by( self, n : u64 ) -> Self` | `self.0 + n` | 14 src, 10 test |
| [`distance_to`](../item/associated_function/009_seq_distance_to.md) | `const fn distance_to( self, later : Self ) -> u64` | `later.0.saturating_sub( self.0 )` | 11 src |

**`advanced_by` outnumbers `next` in production by three to one**, which says
what the family actually does: it advances cursors by batch lengths, not by
ones. `ring_index/src/lib.rs:121` folds a whole run in one expression, and
`ring_mpsc` builds batch iterators from it.

**`distance_to` saturates, and it is the only one of the three that says so
accurately.** Its doc comment explains the choice: a caller that needs the
direction has already compared the two positions, so returning `0` for a
backwards pair is more useful than a signed result nobody would use. The other
two used to claim a saturating release behaviour they do not have, and both
have since been corrected to state the actual wrapping behaviour
(→ [`pitfall/001`](../pitfall/001_seq_next_wraps_where_its_doc_says_it_saturates.md)).

**`SlotIndex` — one operation:**

| Operation | Signature | Semantics |
|-----------|-----------|-----------|
| [`get`](../item/associated_function/010_slot_index_get.md) | `const fn get( self ) -> usize` | `self.0` |

**One accessor over a `pub` field is redundant on its face and is not.** `.0`
and `.get()` return the same value; `get()` exists so a call site reads as an
intention (*take the offset*) rather than as tuple-field access, and so the
representation could change without touching call sites. The same argument
applies to `Seq`, which has no `get` — an asymmetry with no stated reason, and
the only inconsistency in the pair.

**No operation converts between the two types.** There is no
`Seq::to_slot_index`, no `From< Seq > for SlotIndex`, and no
`SlotIndex::from_seq`. The conversion requires a `Capacity`, which this crate
declines to thread through a position type — so the fold lives in `ring_index`,
takes both inputs explicitly, and is the single place the mask is applied
(→ [`invariant/001`](../invariant/001_every_capacity_has_a_valid_mask.md)).

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_validating_a_slot_count_to_a_power_of_two.md](../algorithm/001_validating_a_slot_count_to_a_power_of_two.md) | Produces the mask the absent conversion would need |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_vocabulary_surface.md](../api/001_the_vocabulary_surface.md) | Both types among the six exported names |

### Data Structures

| File | Relationship |
|------|--------------|
| [002_the_error_enum_as_a_closed_copy_set.md](002_the_error_enum_as_a_closed_copy_set.md) | The crate's other data structure — nine variants rather than two newtypes |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_the_crate_thirty_one_of_thirty_three_depend_on.md](../integration/001_the_crate_thirty_one_of_thirty_three_depend_on.md) | The 18-versus-3 counts in the family-wide table |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_every_capacity_has_a_valid_mask.md](../invariant/001_every_capacity_has_a_valid_mask.md) | What makes the fold total, wherever it is performed |

### Items

| File | Relationship |
|------|--------------|
| [../item/associated_constant/001_seq_zero.md](../item/associated_constant/001_seq_zero.md) | The named zero, and its overlap with the derived `Default` |
| [../item/struct/002_seq.md](../item/struct/002_seq.md) | The declaration and its eighteen consumers |
| [../item/struct/003_slot_index.md](../item/struct/003_slot_index.md) | The declaration and its three |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_a_slot_count_from_request_to_mask.md](../lifecycle/001_a_slot_count_from_request_to_mask.md) | The other half of the fold — where the mask comes from |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_a_newtype_that_makes_a_check_unnecessary.md](../pattern/002_a_newtype_that_makes_a_check_unnecessary.md) | The contrast — these two newtypes buy non-interchangeability, not validity |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_seq_next_wraps_where_its_doc_says_it_saturates.md](../pitfall/001_seq_next_wraps_where_its_doc_says_it_saturates.md) | The overflow claim on two of `Seq`'s three operations |

### Sources

| File | Relationship |
|------|--------------|
| [`src/id.rs`](../../src/id.rs) | Both types — 114 lines, of which `SlotIndex` occupies the last 27 |
| [`ring_index/src/lib.rs`](../../../ring_index/src/lib.rs) | The fold this crate declines to own; line 88 folds a whole run |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/types_test.rs`](../../tests/types_test.rs) | ✅ `seq_orders_by_count`, `seq_advances`, `seq_distance_saturates_backward` and `slot_index_is_its_own_type` cover both types' operations. `seq_does_not_wrap_within_any_reachable_workload` asserts the reachability argument, which is the true claim — `src/id.rs:34–38` used to state a different, false one and has since been corrected to match |
| [`tests/manual/readme.md`](../../tests/manual/readme.md) | M1 — that the two types do not mix, checked by confirming the comparison does not compile. **The central guarantee of this instance is the one the suite cannot assert** |

### TY26 — The Sequence-to-Slot Fold Is Written Twice, Identically

The two production callers of `Capacity::mask` in the whole family are the same
expression:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r '\.mask()' ring_*/src/*.rs | grep -v '^ring_types/' \
  | grep -vE ': *(//|///|//!)'
```

Live output:

```
ring_index/src/lib.rs:  SlotIndex( ( seq.0 as usize ) & capacity.mask() )
ring_mpsc/src/lib.rs:    let index = ( seq.0 as usize ) & self.capacity().mask();
```

`ring_index` is a crate whose entire subject is this fold. `ring_mpsc` depends on
`ring_types` and computes it inline instead — one line, correct, and a second
definition of the family's most-relied-on arithmetic.

**The cost is not the duplicated line; it is that a change to the fold has two
homes.** If the mask discipline were ever replaced — by a modulo for non-power-of-two
capacities, say — `ring_index` would be edited and `ring_mpsc` would keep
working, silently, on the old rule.

### TY27 — Both Fold Sites Reach Through the Public Field

`( seq.0 as usize )` is not an accessor call — there is no accessor. It is a
field read, available because `Seq`'s field is `pub`
(→ [`../item/002`](../item/002_three_newtypes_and_two_open_fields.md), TY13).

This matters for the sealing estimate: the fold is the *reason* the field is
open, and any proposal to seal `Seq` has to supply a conversion these two lines
can use in the same expression, in a `const` context for
`ring_mpsc::UNSTAMPED`.
