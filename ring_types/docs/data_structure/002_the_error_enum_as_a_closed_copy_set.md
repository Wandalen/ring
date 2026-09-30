# Data Structure: The Error Enum as a Closed Copy Set

### Scope

- **Purpose**: Describe `RingError` as a layout — nine variants in 24 bytes, one of which sets the size, all of which are `Copy` — and account for what that budget buys and what it forbids.
- **Responsibility**: State the abstract, the structure, and the operations.
- **In Scope**: Byte layout, the `Copy` and `#[ non_exhaustive ]` constraints, the hand-written `Display`, and the free `Result`.
- **Out of Scope**: The classification predicates as an algorithm (→ [`algorithm/002`](../algorithm/002_classifying_an_error_into_configuration_or_traffic.md)); the crate that declined this type (→ [`integration/002`](../integration/002_the_registry_that_declined_the_shared_error.md)).

### Abstract

**Nine variants, 24 bytes, no allocation, and a wildcard arm required of every
consumer.** `RingError` is the family's one error type by declaration, sized by
its single struct-shaped variant, and constrained to `Copy` because a failure on
the tick path must not allocate.

The three shaping decisions are independent and each costs something:
`Copy` forbids owned payloads, `#[ non_exhaustive ]` forbids exhaustive matching
by consumers, and the zero-dependency rule forbids a derive macro for `Display`.
The first is the expensive one — it is why a second error type exists in the
family.

### Structure

**Nine variants, three payload shapes:**

| Variant | Payload | Bytes used | Class |
|---------|---------|-----------:|-------|
| `CapacityZero` | — | 0 | Configuration |
| `Full` | — | 0 | Traffic |
| `Empty` | — | 0 | Traffic |
| `Closed` | — | 0 | *Neither* |
| `NameTaken` | — | 0 | *Neither* |
| `NameUnknown` | — | 0 | *Neither* |
| `PolicyUnsupported` | — | 0 | Configuration |
| `CapacityNotPowerOfTwo` | `( usize )` | 8 | Configuration |
| `BatchTooLarge` | `{ requested : usize, capacity : usize }` | **16** | Configuration |

**Measured layout:**

```rust
size_of::< RingError >()                      // 24, align 8
size_of::< Result< Capacity, RingError > >()  // 24
size_of::< Result< (), RingError > >()        // 24
```

| Fact | Value | Why |
|------|-------|-----|
| Enum size | 24 | 16 bytes of `BatchTooLarge` + 8 for the discriminant, at align 8 |
| Without `BatchTooLarge` | would be 16 | `CapacityNotPowerOfTwo`'s one `usize` + discriminant |
| `Result< Capacity, _ >` | 24 | The 8-byte `Ok` rides inside space the error already reserves |

**The `Result` being free is the fact worth carrying.** Returning
`Result< Capacity, RingError >` from
[`Capacity::new`](../item/associated_function/001_capacity_new.md) costs nothing
over returning `Option< Capacity >`, so the specific diagnostic is free. Anyone
proposing to shrink the error in order to make results cheaper is optimising a
number that is already zero.

**Three structural constraints, and what each forbids:**

| Constraint | Declared at | Forbids | Cost paid |
|-----------|-------------|---------|-----------|
| `Copy` | `src/error.rs:42` derive | Any owned payload — `String`, `Box`, `Vec` | `NameTaken` cannot carry the name it is about |
| `#[ non_exhaustive ]` | `src/error.rs:43` | Exhaustive matching by external consumers | A dead variant produces no warning at a consumer's match site |
| Zero dependencies | `Cargo.toml` | `thiserror`, `error_tools` | 20 hand-written `match` arms for `Display` |

**The `Copy` constraint is the expensive one and it is correct.** A ring
operation returning an error on the tick path must not allocate; `Copy` and
allocation-freedom together guarantee it structurally rather than by review. The
cost lands on exactly one variant — `NameTaken`, whose natural payload is an
owned `String` — and the family paid it by growing a second error type in
`ring_registry` instead
(→ [`integration/002`](../integration/002_the_registry_that_declined_the_shared_error.md)).

**`#[ non_exhaustive ]`'s cost is subtler and compounds with a different
defect.** It forces consumers to write a wildcard arm, which is the point — the
family is still adding backends. But a wildcard arm also absorbs the difference
between a variant that is live and one that nothing constructs, so
`#[ non_exhaustive ]` is precisely what makes
[`pitfall/002`](../pitfall/002_two_name_errors_nothing_constructs.md)'s dead arms
silent. The attribute is right and its interaction with two unconstructed
variants is not visible to any compiler.

**The zero-dependency constraint is the cheapest of the three.** Twenty lines of
`match`, written once, at `src/error.rs:162–182` — in exchange for tier 0 having
no outgoing edges and the family's forest being acyclic by construction
(→ [`invariant/002`](../invariant/002_tier_zero_depends_on_nothing.md)). This is
a documented, deliberate exception to the workspace's `error_tools` convention.

**Derives, and the one that is absent:**

```rust
#[ derive( Debug, Clone, Copy, PartialEq, Eq, Hash ) ]
```

`PartialOrd`/`Ord` are absent, and correctly — there is no total order on these
failures. Severity is not a ranking here; the two useful distinctions are carved
by `is_configuration` and `is_transient`, which are predicates rather than
comparisons.

### Operations

**Two inherent predicates, two trait impls, no constructors.**

| Operation | Signature | Notes |
|-----------|-----------|-------|
| [`is_configuration`](../item/associated_function/004_ring_error_is_configuration.md) | `const fn is_configuration( self ) -> bool` | 4 of 9 variants |
| [`is_transient`](../item/associated_function/005_ring_error_is_transient.md) | `const fn is_transient( self ) -> bool` | 2 of 9 variants |
| [`Display::fmt`](../item/associated_function/006_display_fmt_for_ring_error.md) | `fn fmt( &self, … ) -> fmt::Result` | Hand-written, nine arms, `src/error.rs:164–181` |
| `core::error::Error` | — | Empty impl, `src/error.rs:184`; makes the type usable as a `dyn Error` source |

**Both predicates take `self` by value, which only works because of `Copy`.**
The consequence is that an error can be classified *after* being moved into a
log line or a `Result`, with no borrow and no clone — the ergonomic payoff of
the constraint that costs `NameTaken` its payload.

**Both are `const`**, so classification can occur in a `const` context. Nothing
in the family currently does, but the property is free given the bodies are
`matches!`.

**The union of the two predicates covers six variants; three are in neither.**
`Closed`, `NameTaken` and `NameUnknown` answer `false` to both, and nothing on
the type says so — a caller branching on `is_configuration` alone routes all
three into the retry path
(→ [`algorithm/002`](../algorithm/002_classifying_an_error_into_configuration_or_traffic.md)).

**`Display` is where the payload decisions become visible:**

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -F 'Self::CapacityNotPowerOfTwo( n ) => write!' ring_types/src/error.rs
command grep -A3 -F 'Self::BatchTooLarge { requested, capacity } =>' ring_types/src/error.rs
```

Live output:

```
      Self::CapacityNotPowerOfTwo( n ) => write!( f, "ring capacity {n} is not a power of two" ),
      Self::BatchTooLarge { requested, capacity } =>
      {
        write!( f, "batch of {requested} exceeds ring capacity {capacity}" )
      }
```

The two variants that carry data produce specific messages; the seven that do
not produce fixed strings. **`PolicyUnsupported`'s string — "this backend cannot
honour the configured overflow policy" — names neither the backend nor the
policy**, and unlike `NameTaken` there is no `Copy` constraint stopping it from
carrying an `OverflowPolicy`, which is one byte.

**No constructors, no `From` impls, no conversion surface at all.** Variants are
built by struct-literal syntax at the point of failure, in whichever crate
detects it. That keeps this crate free of any knowledge about who fails and how,
which is what "no ring logic" means for the error module.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_classifying_an_error_into_configuration_or_traffic.md](../algorithm/002_classifying_an_error_into_configuration_or_traffic.md) | The two predicates' membership sets and the three-variant gap |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_five_classifier_predicates.md](../api/002_the_five_classifier_predicates.md) | These two among the crate's five predicates |

### Data Structures

| File | Relationship |
|------|--------------|
| [001_two_position_types_and_the_fold_between_them.md](001_two_position_types_and_the_fold_between_them.md) | The crate's other structure — two newtypes rather than one enum |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_the_registry_that_declined_the_shared_error.md](../integration/002_the_registry_that_declined_the_shared_error.md) | What the `Copy` constraint cost the family |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_tier_zero_depends_on_nothing.md](../invariant/002_tier_zero_depends_on_nothing.md) | The rule that makes `Display` hand-written |

### Items

| File | Relationship |
|------|--------------|
| [../item/enum/002_ring_error.md](../item/enum/002_ring_error.md) | The declaration, variant by variant |
| [../item/implementation/003_impl_display_for_ring_error.md](../item/implementation/003_impl_display_for_ring_error.md) | The nine `match` arms |
| [../item/implementation/004_impl_error_for_ring_error.md](../item/implementation/004_impl_error_for_ring_error.md) | The empty impl that makes it a `dyn Error` |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_errors_and_positions_do_not_allocate.md](../non_functional_requirement/001_errors_and_positions_do_not_allocate.md) | The `Copy` constraint as a measurable requirement |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_two_name_errors_nothing_constructs.md](../pitfall/002_two_name_errors_nothing_constructs.md) | How `#[ non_exhaustive ]` makes the dead variants silent |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_ring_error.md](../type/002_ring_error.md) | The same nine variants as a contract rather than a layout |

### Sources

| File | Relationship |
|------|--------------|
| [`src/error.rs`](../../src/error.rs) | Lines 42–81, the declaration; 162–182, the hand-written `Display`; 184, the `Error` impl |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/types_test.rs`](../../tests/types_test.rs) | ✅ `error_is_copy` pins the `Copy` constraint, `every_error_displays_distinctly` pins nine distinct messages, `error_implements_the_error_trait` pins the trait impl via a generic bound. ❌ **No test asserts the 24-byte layout or the free `Result`** — both are stable facts a `size_of` assertion would pin, and a future variant carrying three `usize` fields would grow the enum silently |

### TY28 — Nine Variants and Nine `Display` Arms Stay in Step by Exhaustive Match

```sh
cd "$(git rev-parse --show-toplevel)"/ring_types
printf 'variants:     '; awk '/^pub enum RingError/,/^}/' src/error.rs | grep -cE '^    [A-Z]'
printf 'Display arms: '; awk '/impl fmt::Display/,/^}/' src/error.rs | grep -cE 'Self::[A-Za-z]+.*=>'
printf 'wildcards:    '; awk '/impl fmt::Display/,/^}/' src/error.rs | grep -c '_ =>'
# The command above can legitimately exit nonzero -- a count of zero, a diff
# that differs -- and the sweep that keeps this output current skips any block
# that exits nonzero, so without this the block freezes instead of refreshing.
:
```

Live output:

```
variants:     9
Display arms: 9
wildcards:    0
```

Worth recording because it is the exception. This crate's other closed sets —
the two `ALL` arrays — are hand-written and have no compiler backstop at all
(→ [`../workaround/001`](../workaround/001_hand_written_all_arrays_stand_in_for_variant_enumeration.md)).
The `Display` match is the only place where forgetting to update a list fails
the build rather than a test.
