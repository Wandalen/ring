# API: The Vocabulary Surface

### Scope

- **Purpose**: State the six names this crate exports, the one path each is reachable by, and what a consumer may rely on about them — the surface 31 of the family's 33 crates import and an external consumer may name through the export Contract.
- **Responsibility**: State the abstract, operations, error handling, and compatibility guarantees.
- **In Scope**: The six exports; the four private modules behind them; the single fallible operation.
- **Out of Scope**: The five predicates, which are their own instance (→ [`api/002`](002_the_five_classifier_predicates.md)); per-type definitions (→ [`type/`](../type/)).

### Abstract

**Six names, four private modules, one fallible call.** Everything this crate
offers is reachable from the crate root and nowhere else: the module layout is
an implementation detail behind four `pub use` lines.

The surface is vocabulary rather than behaviour. Three of the six are data
types, two are enums, one is an error, and none of them does anything to a ring
— discriminants live here and the handlers that act
on them live in `ring_wait` and `ring_overflow`
(→ [`pattern/001`](../pattern/001_discriminants_here_handlers_elsewhere.md)).

`ring_types` is one of the five crates on the family's export Contract, so this
surface is external as well as internal — but it is the only one of the five
that a consumer imports for *names* rather than for *doing something*.

### Operations

**The six exports and their one path each:**

```sh
cd "$(git rev-parse --show-toplevel)"
command grep '^mod \|^pub use ' ring_types/src/lib.rs
```

Live output:

```
mod capacity;
mod error;
mod id;
mod policy;
pub use capacity::Capacity;
pub use error::RingError;
pub use id::{Seq, SlotIndex};
pub use policy::{OverflowPolicy, WaitKind};
```

| Export | Kind | Declared in | Consuming crates |
|--------|------|-------------|-----------------:|
| [`Capacity`](../type/001_capacity.md) | Struct, private field | `capacity.rs` | 17 |
| [`RingError`](../type/002_ring_error.md) | Enum, 9 variants, `#[ non_exhaustive ]` | `error.rs` | 19 |
| [`Seq`](../item/struct/002_seq.md) | Struct, `pub u64` | `id.rs` | 18 |
| [`SlotIndex`](../item/struct/003_slot_index.md) | Struct, `pub usize` | `id.rs` | 3 |
| [`OverflowPolicy`](../item/enum/003_overflow_policy.md) | Enum, 3 variants | `policy.rs` | 10 |
| [`WaitKind`](../item/enum/001_wait_kind.md) | Enum, 4 variants | `policy.rs` | 7 |

**Every module is private and every type reaches consumers through the root.**
`ring_types::error::RingError` is not a path any consumer can write, so the four
modules can be merged, split, or renamed without a downstream edit. That is
worth one `pub use` line per module.

**Fourteen callable operations, and exactly one can fail:**

| Operation | Signature | Fallible | `const` |
|-----------|-----------|:--------:|:-------:|
| `Capacity::new` | `( usize ) -> Result< Capacity, RingError >` | **Yes** | ✅ |
| `Capacity::get` | `( self ) -> usize` | No | ✅ |
| `Capacity::mask` | `( self ) -> usize` | No | ✅ |
| `Seq::next` | `( self ) -> Seq` | No | ✅ |
| `Seq::advanced_by` | `( self, u64 ) -> Seq` | No | ✅ |
| `Seq::distance_to` | `( self, Seq ) -> u64` | No | ✅ |
| `SlotIndex::get` | `( self ) -> usize` | No | ✅ |
| `RingError::is_configuration` | `( self ) -> bool` | No | ✅ |
| `RingError::is_transient` | `( self ) -> bool` | No | ✅ |
| `WaitKind::is_non_blocking` | `( self ) -> bool` | No | ✅ |
| `OverflowPolicy::reports_failure` | `( self ) -> bool` | No | ✅ |
| `OverflowPolicy::drops_silently` | `( self ) -> bool` | No | ✅ |
| `Display::fmt` for `RingError` | `( &self, … ) -> fmt::Result` | Trait | ❌ |
| `core::error::Error` for `RingError` | — | Trait | ❌ |

**Thirteen of the fourteen are `const`, and every inherent one takes `self` by
value.** Both properties follow from every type being `Copy` and every body
being either a field read, an arithmetic expression, or a `matches!`. The
practical consequence is that a consumer can validate a literal capacity at
compile time:

```rust
const CAP : Capacity = match Capacity::new( 1024 )
{
  Ok( c ) => c,
  Err( _ ) => panic!( "not a power of two" ),
};
```

**Three associated constants complete the surface:**

| Constant | Type | Purpose |
|----------|------|---------|
| [`Seq::ZERO`](../item/associated_constant/001_seq_zero.md) | `Seq` | The position of a ring that has published nothing |
| [`WaitKind::ALL`](../item/associated_constant/002_wait_kind_all.md) | `[ WaitKind; 4 ]` | Lets a test assert the set is *exactly* four, not merely contains four |
| [`OverflowPolicy::ALL`](../item/associated_constant/003_overflow_policy_all.md) | `[ OverflowPolicy; 3 ]` | Same, for three |

**The two `ALL` arrays are the crate's one concession to its consumers'
tests**, and they are used: `ring_overflow` and `ring_stats` iterate
`OverflowPolicy::ALL`, `ring_wait` and `ring_config` iterate `WaitKind::ALL`.
Their length is the assertion — adding a variant fails a length check somewhere
rather than silently leaving a sweep axis under-covered.

### Error Handling

**One error channel, opened once.**

| Call | Error | Condition |
|------|-------|-----------|
| `Capacity::new( 0 )` | `RingError::CapacityZero` | The only zero |
| `Capacity::new( n )`, `n` not a power of two | `RingError::CapacityNotPowerOfTwo( n )` | Carries `n` |

Every other operation is total. There is no fallible accessor, no panicking
path documented as reachable, and no `unwrap` in the crate's own source.

**`RingError` is exported for the family's use, not for this crate's failures.**
Two of its nine variants are constructed here; five more are constructed by
other crates; two are constructed nowhere at all
(→ [`pitfall/002`](../pitfall/002_two_name_errors_nothing_constructs.md)). A
consumer reading this surface for "what can `ring_types` do to me" should read
the two-row table above; a consumer reading it for "what can the family do to
me" should read [`type/002`](../type/002_ring_error.md).

**The error is `#[ non_exhaustive ]`, so every consumer match needs a wildcard
arm.** This is deliberate — the family is still adding backends — and it has a
cost noted elsewhere: a wildcard arm also hides the difference between a live
variant and a dead one.

**Nothing here panics on a documented path.** `Seq::next` and
`Seq::advanced_by` can overflow arithmetically, which panics in a debug build;
the wrap point is 584 years out at 10⁹ publications per second, so no reachable
workload gets there. The doc comment describing that behaviour used to be wrong
about the release half and has since been corrected (→ [`pitfall/001`](../pitfall/001_seq_next_wraps_where_its_doc_says_it_saturates.md)).

### Compatibility Guarantees

| # | Guarantee | Basis | State |
|---|-----------|-------|-------|
| G1 | The six names stay reachable from the crate root | `src/lib.rs:41–44` | ✅ Held |
| G2 | Module layout is not part of the surface | All four modules are private | ✅ Held |
| G3 | `RingError`'s variant set may grow, never shrink | `#[ non_exhaustive ]` | ✅ Held |
| G4 | `Capacity`'s only constructor stays validating | Private field, no `Default` | ✅ Held |
| G5 | Every type stays `Copy` | Derived on all six | ✅ Held |
| G6 | `WaitKind::ALL` and `OverflowPolicy::ALL` stay exhaustive | A wildcard-free `match` in the test suite — **not** the array length | ⚠️ Held, but by a mechanism outside `src/` |
| G7 | No behaviour that dispatches on a policy is added here | The discriminant/handler split (→ [`pattern/001`](../pattern/001_discriminants_here_handlers_elsewhere.md)) | ✅ Held |

**G3 is the guarantee doing the most work.** Nine variants today, two backends
shipped and a third behind a feature flag; a tenth variant is likely and must
not be a breaking change. `#[ non_exhaustive ]` buys that at the cost of a
wildcard arm at every consumer match.

**G6 is held by something that is not in this file, and the obvious reading of
how is wrong.** `WaitKind::ALL` is a hand-written array literal:

```rust
pub const ALL : [ Self; 4 ] = [ Self::Spin, Self::Yield, Self::Park, Self::None ];
```

The explicit `4` looks like it makes a fifth variant a compile error. It does
not — a five-variant enum with a four-element `ALL` compiles cleanly, because
the length constrains the array against its own initialiser and never against
the enum (probed in
[`non_functional_requirement/002`](../non_functional_requirement/002_the_enum_sets_are_closed_and_asserted.md)).
What actually catches a new variant is a wildcard-free `match` inside
`wait_kind_has_exactly_four_variants`, whose loop body is empty and whose only
purpose is to be a place a fifth variant cannot pass through.

**So the guarantee is real and its mechanism lives in `tests/`, not `src/`** —
which is worth stating on the surface document, because a consumer reading
`policy.rs` alone would conclude the length is the enforcement and could
"simplify" the array to an inferred length without touching anything that
fails.

**G5 is load-bearing beyond ergonomics.** It is what lets every predicate take
`self` by value, and it is what forbids any variant of `RingError` from carrying
an owned payload — the constraint that cost this crate a consumer
(→ [`integration/002`](../integration/002_the_registry_that_declined_the_shared_error.md)).

### APIs

| File | Relationship |
|------|--------------|
| [002_the_five_classifier_predicates.md](002_the_five_classifier_predicates.md) | The five `bool` operations in this table, and how little production code calls them |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_validating_a_slot_count_to_a_power_of_two.md](../algorithm/001_validating_a_slot_count_to_a_power_of_two.md) | The one fallible operation |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_two_position_types_and_the_fold_between_them.md](../data_structure/001_two_position_types_and_the_fold_between_them.md) | `Seq` and `SlotIndex`, and why one reaches 18 crates and the other 3 |
| [../data_structure/002_the_error_enum_as_a_closed_copy_set.md](../data_structure/002_the_error_enum_as_a_closed_copy_set.md) | G3 and G5 as a layout |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_the_crate_thirty_one_of_thirty_three_depend_on.md](../integration/001_the_crate_thirty_one_of_thirty_three_depend_on.md) | The per-export consumer counts, and the Contract this surface sits on |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_tier_zero_depends_on_nothing.md](../invariant/002_tier_zero_depends_on_nothing.md) | Why no foreign type appears anywhere in this surface |

### Items

| File | Relationship |
|------|--------------|
| [../item/use_declaration/001_pub_use_capacity.md](../item/use_declaration/001_pub_use_capacity.md) | G1 and G2's mechanism, one line at a time |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_discriminants_here_handlers_elsewhere.md](../pattern/001_discriminants_here_handlers_elsewhere.md) | G7 |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_capacity.md](../type/001_capacity.md) | G4 |
| [../type/002_ring_error.md](../type/002_ring_error.md) | G3 |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | Lines 36–44 — the whole surface declaration, and the module table above it |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/types_test.rs`](../../tests/types_test.rs) | ✅ Every operation in the table is exercised; `wait_kind_has_exactly_four_variants` and `overflow_policy_has_no_overwrite_variant` pin G6's two arrays by length and membership. ❌ **G1, G2 and G7 are unasserted** — a test compiled against this crate cannot distinguish a root-level path from a module-level one, and cannot see a behaviour being added |
| [`tests/manual/readme.md`](../../tests/manual/readme.md) | M1's shape — the surface claims a test cannot make about itself |

### TY23 — The Six Exported Names Are Adopted at Very Different Rates

One crate, one export, six adoption curves:

```sh
cd "$(git rev-parse --show-toplevel)"
for n in RingError Seq Capacity WaitKind OverflowPolicy SlotIndex; do
  printf '%-16s %2d crate(s)\n' "$n" \
    "$( grep -rn "\b$n\b" ring_*/src/*.rs | grep -v '^ring_types/' \
        | grep -vE ':[0-9]+: *(//|///|//!)' | cut -d/ -f1 | sort -u | wc -l )"
done
```

Live output:

```
RingError        18 crate(s)
Seq              16 crate(s)
Capacity         11 crate(s)
WaitKind          4 crate(s)
OverflowPolicy    4 crate(s)
SlotIndex         3 crate(s)
```

**The spread is the finding.** `docs/api/001` describes the surface as a
vocabulary the family shares; measured, it is two names nearly everything uses,
one most things use, and three that three or four crates use. A consumer reading
the export as a coherent unit will over-estimate how much of it is load-bearing
by a factor of six at the thin end.

**These figures do not match the "seventeen consumers" this corpus quotes for
`Capacity` elsewhere, and both are correct.** Seventeen is every crate that names
`Capacity` anywhere, doc comments included; eleven is every crate that names it
in code. The catalogue under [`../item/`](../item/readme.md) enumerates the
fifteen because a `use` declaration resolved in a doc example is still a
compilation edge. This instance counts production reach because the question here
is what the export surface carries, and the gap between the two measurements is
the systematic effect [`../item/001`](../item/001_the_forty_items_and_the_six_the_family_calls.md)
records as TY10. Where the two numbers appear together, this is the one that
answers "who would break".
