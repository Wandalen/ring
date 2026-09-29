# Type: RingError

### Scope

- **Purpose**: Define the family's shared error enum — nine variants, `Copy`, `#[ non_exhaustive ]`, 24 bytes — and state which of its rules are enforced, which are conventions, and which are already broken.
- **Responsibility**: State the definition and the validation.
- **In Scope**: The nine variants and their payloads; the `Copy` constraint and what it costs; the two variants nothing constructs.
- **Out of Scope**: The two classifier predicates as an algorithm (→ [`algorithm/002`](../algorithm/002_classifying_an_error_into_configuration_or_traffic.md)); the registry that declined this type (→ [`integration/002`](../integration/002_the_registry_that_declined_the_shared_error.md)).

### Definition

```rust
#[ derive( Debug, Clone, Copy, PartialEq, Eq, Hash ) ]
#[ non_exhaustive ]
pub enum RingError
{
  CapacityZero,
  CapacityNotPowerOfTwo( usize ),
  Full,
  Empty,
  Closed,
  NameTaken,
  NameUnknown,
  BatchTooLarge { requested : usize, capacity : usize },
  PolicyUnsupported,
}
```

| Property | Value | Consequence |
|----------|-------|-------------|
| Size | **24 bytes** | Set entirely by `BatchTooLarge`'s two `usize` fields plus a discriminant |
| Alignment | 8 | `usize` |
| `Copy` | Yes | A tick-path error must not allocate; asserted by `error_is_copy` |
| `#[ non_exhaustive ]` | Yes | Consumers must write a wildcard arm; variants may be added without a major bump |
| `Default` | No | There is no default failure |
| Trait impls | `Display` (hand-written), `core::error::Error` | 20 lines of `match`, and no `thiserror` — see below |
| Constructed in this crate | 2 variants | `CapacityZero`, `CapacityNotPowerOfTwo` |
| Constructed in the family | 5 more | `Full`, `Empty`, `Closed`, `BatchTooLarge`, `PolicyUnsupported` |
| Constructed nowhere | **2** | `NameTaken`, `NameUnknown` |

**`Result< Capacity, RingError >` is 24 bytes — the same as the error alone.**

```rust
size_of::< RingError >()                     // 24
size_of::< Result< Capacity, RingError > >() // 24
```

The `Ok` payload rides free in space the error already occupies. This is why
`Capacity::new` returning a `Result` costs nothing over returning an `Option`,
and it is worth knowing before anyone proposes trimming the error to make the
result cheaper.

**`BatchTooLarge` alone sets the size, and it is the only struct-shaped
variant.** Without it the enum would be 16 bytes (`CapacityNotPowerOfTwo`'s one
`usize` plus a discriminant). The extra 8 bytes buy a message that names both
the request and the limit — `"batch of 64 exceeds ring capacity 16"` — rather
than one that names neither.

**No `error_tools`, no `thiserror`, and that is a rule rather than an
oversight.** `ring_types` declares zero dependencies so tier 0 has no outgoing
edges and the family's forest is acyclic by construction
(→ [`invariant/002`](../invariant/002_tier_zero_depends_on_nothing.md)). The
`Display` impl is written by hand at `src/error.rs:162–182`. This is a
documented, deliberate exception to the workspace's `error_tools` convention.

**Variant payloads, and what each one's absence would cost:**

| Variant | Payload | Why that payload |
|---------|---------|------------------|
| `CapacityZero` | none | There is only one zero |
| `CapacityNotPowerOfTwo` | `usize` | Infinitely many illegal values; the caller needs to know which arrived |
| `Full` | none | The caller knows the ring |
| `Empty` | none | Same |
| `Closed` | none | Same |
| `NameTaken` | **none** | ⚠️ Cannot carry the name — a `String` would end `Copy`. This is why `ring_registry` declared its own error instead |
| `NameUnknown` | none | Same constraint; also describes a shape the registry did not build (`get_mut` returns `Option`) |
| `BatchTooLarge` | `{ requested, capacity }` | Neither number is derivable from the other, and the ratio is the diagnostic |
| `PolicyUnsupported` | none | ⚠️ Does not say *which* policy or *which* backend refused it |

**Two rows carry a warning and they are different problems.** `NameTaken`'s
payload is forbidden by the `Copy` constraint, which is why the type lost a
consumer. `PolicyUnsupported`'s payload is merely absent — nothing stops it
carrying an `OverflowPolicy`, and today the only producer is `ring_core`
refusing `DropOldest`, so the missing field is recoverable from context. That
stops being true the moment a second backend refuses a second policy.

### Validation

**This type validates nothing — it *is* the validation result.** The rules below
are properties of the declaration, and their enforcement is structural, by
convention, or absent.

| # | Rule | Enforcement | State |
|---|------|-------------|-------|
| E1 | `RingError` is `Copy` | Derived; asserted by `error_is_copy` | ✅ Held |
| E2 | No variant allocates | Structural — every payload is `usize` | ✅ Held |
| E3 | `#[ non_exhaustive ]` is present | Attribute at `src/error.rs:43` | ✅ Held |
| E4 | Every variant has a distinct `Display` string | Convention; asserted by `every_error_displays_distinctly` | ✅ Held |
| E5 | Every variant is constructed somewhere | **Nothing** | ❌ **Broken — two variants, zero constructors** |
| E6 | Every variant is classified by at least one predicate | **Nothing** | ❌ **Broken — three variants answer `false` to both** |
| E7 | It is the family's only error type | **Nothing** | ❌ **Broken — `ring_registry::RegistryError`** |

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -r 'RingError::NameTaken\|RingError::NameUnknown' . --include=*.rs
```

Live output:

```
ring_types/tests/types_test.rs:  let naming = [ RingError::NameTaken, RingError::NameUnknown ];
ring_types/tests/types_test.rs:    RingError::NameTaken,
ring_types/tests/types_test.rs:    RingError::NameUnknown,
```

**Three references in the whole workspace, all in this crate's own test
suite, none of them a production construction.** The two variants are named
only by the tests that assert they classify and display correctly — which is
the sharpest possible statement of E5: **the suite exercises them and no
program produces them.**

**E5, E6 and E7 are the three rules a reader would assume hold, and none does.**
They are not enforced by anything, and each is broken in a way that produces no
warning:

- **E5** — an unconstructed variant is indistinguishable from a live one at a
  match site. `#[ non_exhaustive ]` forces a wildcard arm, which absorbs the
  dead arm's absence of reachability.
- **E6** — `false` is a valid answer to both predicates, so `Closed`,
  `NameTaken` and `NameUnknown` fall through a caller's
  `if is_configuration() … else retry` into the retry branch. For `Closed`, that
  is an unbounded loop against a ring that will never reopen.
- **E7** — the claim is made in this module's own opening paragraph and refuted
  by a crate that is one manifest line away from complying and chose not to be.

**E1 is the rule that causes E7.** A tick-path error must not allocate, so no
variant may carry a `String`; the one crate whose error genuinely wants an owned
name therefore cannot use this type. **The constraint is right and its
consequence is a second error type**, and nothing in the family records that
trade as having been made deliberately.

**What is deliberately not validated:**

| Not validated | Why |
|---------------|-----|
| That variants are ordered by severity | There is no total order on these failures; `is_configuration`/`is_transient` carve the useful distinctions |
| That `Display` strings are stable | They are diagnostics, not a protocol. `every_error_displays_distinctly` pins distinctness, not wording |
| That the enum stays under N bytes | 24 is a consequence of `BatchTooLarge`, and `Result<_, RingError>` is already free |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_validating_a_slot_count_to_a_power_of_two.md](../algorithm/001_validating_a_slot_count_to_a_power_of_two.md) | Produces the only two variants constructed inside this crate |
| [../algorithm/002_classifying_an_error_into_configuration_or_traffic.md](../algorithm/002_classifying_an_error_into_configuration_or_traffic.md) | E6 — the two predicates and the three variants they miss |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_five_classifier_predicates.md](../api/002_the_five_classifier_predicates.md) | The two predicates on this type, among the crate's five |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_error_enum_as_a_closed_copy_set.md](../data_structure/002_the_error_enum_as_a_closed_copy_set.md) | The same nine variants as a layout — where the 24 bytes go |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_the_registry_that_declined_the_shared_error.md](../integration/002_the_registry_that_declined_the_shared_error.md) | E7, in full — the crate, the missing edge, and what a consumer sees |

### Items

| File | Relationship |
|------|--------------|
| [../item/enum/002_ring_error.md](../item/enum/002_ring_error.md) | The declaration and every crate that names it |
| [../item/implementation/003_impl_display_for_ring_error.md](../item/implementation/003_impl_display_for_ring_error.md) | The hand-written `Display` E2's dependency ban forces |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_an_error_from_construction_to_display.md](../lifecycle/002_an_error_from_construction_to_display.md) | A value's arc, and the two phases the dead variants never reach |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_errors_and_positions_do_not_allocate.md](../non_functional_requirement/001_errors_and_positions_do_not_allocate.md) | E1 and E2 as a measurable requirement rather than a derive |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_two_name_errors_nothing_constructs.md](../pitfall/002_two_name_errors_nothing_constructs.md) | E5, and the dead match arm nothing warns about |

### Types

| File | Relationship |
|------|--------------|
| [001_capacity.md](001_capacity.md) | The type whose constructor produces this one's first two variants |

### Sources

| File | Relationship |
|------|--------------|
| [`src/error.rs`](../../src/error.rs) | The whole type — 184 lines, of which 20 are the hand-written `Display` |
| [`ring_registry/src/lib.rs`](../../../ring_registry/src/lib.rs) | Line 56 — E7's counterexample |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/types_test.rs`](../../tests/types_test.rs) | ✅ E1 by `error_is_copy`, E4 by `every_error_displays_distinctly`, and `error_implements_the_error_trait` pins the `core::error::Error` impl. ❌ **E5, E6 and E7 are unasserted, and two of the three are unassertable from here** — E5 needs a workspace grep, E7 needs another crate's manifest. E6 is the one this suite *could* assert and does not: a test that exactly three variants answer `false` to both predicates would fail the moment a fourth is added |

### TY52 — The Type Carries Two Payload Shapes and the Family Reads Neither

Two of the nine variants carry a payload, in two different shapes — a tuple and
a struct variant — and both exist so an error message can name the offending
number. Outside `ring_types`' own suite, nothing in the family destructures
either: consumers propagate the error or render it, and the `Display` impl is the
only reader of the payload.

That is not wasted — the rendered message is the payload's purpose, and
`BatchTooLarge`'s two fields both appear in it. It does mean the struct-variant
shape buys nothing over a tuple today, and it is the shape that forces `..` into
any future pattern match.
