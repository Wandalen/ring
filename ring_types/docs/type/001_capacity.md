# Type: Capacity

### Scope

- **Purpose**: Define the validated power-of-two slot count and state the rules that hold for every value of it, so fifteen consuming crates can use its mask without checking anything.
- **Responsibility**: State the definition and the validation.
- **In Scope**: The newtype, its three operations, its size, and the guarantees a constructed value carries.
- **Out of Scope**: The construction algorithm's step ordering (→ [`algorithm/001`](../algorithm/001_validating_a_slot_count_to_a_power_of_two.md)); the fold that consumes the mask (owned by `ring_index`).

### Definition

```rust
#[ derive( Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash ) ]
pub struct Capacity( usize );
```

A single-field tuple struct with a **private** field. The field's privacy is the
type's entire mechanism: `Capacity` cannot be constructed by a consumer except
through [`new`](../item/associated_function/001_capacity_new.md), and `new` is
the only code that ever writes it.

| Property | Value | Established by |
|----------|-------|----------------|
| Size | 8 bytes | One `usize`, no discriminant |
| Alignment | 8 | `usize` |
| `Copy` | Yes | Derived; a capacity is passed by value throughout the family |
| `Ord` | Yes | Derived — compares by slot count, which is the only sensible order |
| `Hash` | Yes | Derived; usable as a map key in a sweep harness |
| `Default` | **No** | Deliberate — there is no defensible default slot count |
| Constructors | 1, fallible, `const` | `src/capacity.rs:40` |

**Three operations, and two of them are guaranteed total:**

| Operation | Signature | Totality |
|-----------|-----------|----------|
| [`new`](../item/associated_function/001_capacity_new.md) | `const fn new( usize ) -> Result< Self, RingError >` | Fallible — the only fallible thing in this crate |
| [`get`](../item/associated_function/002_capacity_get.md) | `const fn get( self ) -> usize` | Total |
| [`mask`](../item/associated_function/003_capacity_mask.md) | `const fn mask( self ) -> usize` | **Total, and that is the point** |

**`mask`'s totality is the type's reason to exist.** It returns `self.0 - 1`,
a subtraction that would underflow on zero and a bitmask that would be wrong for
any non-power-of-two. Both hazards are eliminated by the constructor, not by a
check inside `mask`:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -B1 -A3 -F 'pub const fn mask' ring_types/src/capacity.rs
```

Live output:

```
  #[ must_use ]
  pub const fn mask( self ) -> usize
  {
    self.0 - 1
  }
```

No branch, no `checked_sub`, no debug assertion. **The absence of a check is
the deliverable** — that is what fifteen crates buy by taking a `Capacity`
rather than a `usize`.

**The absent `Default` is a deliberate omission and worth stating.** Every other
type in this crate derives `Default` (`Seq`, `SlotIndex`) or declares one
(`WaitKind::Spin`, `OverflowPolicy::DropNewest`). `Capacity` does not, because a
default slot count is a performance decision with no defensible answer at tier
0 — and because a `Default` would be an infallible constructor, reintroducing
the second path the private field exists to prevent.

### Validation

**Two rules are checked at construction; three more hold for every constructed
value and are never checked again.**

| # | Rule | Kind | Enforced by |
|---|------|------|-------------|
| V1 | `slots != 0` | Checked | `new`, step 1 — `RingError::CapacityZero` |
| V2 | `slots.is_power_of_two()` | Checked | `new`, step 2 — `RingError::CapacityNotPowerOfTwo( slots )` |
| V3 | `mask() == get() - 1`, never underflowing | Structural | V1 makes the subtraction safe; no runtime check exists |
| V4 | `( n & mask() ) == ( n % get() )` for all `n : usize` | Structural | V2 makes the identity hold; no runtime check exists |
| V5 | The inner value never changes after construction | Structural | Private field, no setter, no `&mut self` method |

**V3 and V4 are the rules consumers actually depend on, and neither is
expressible as a check.** They are consequences of V1 and V2 that hold for the
whole lifetime of a value. This is what makes the newtype worth its ceremony:
the checkable rules are checked once, and the useful rules follow from them
forever.

**V5 has no enforcement code at all, which is the strongest kind.** There is no
`set_capacity`, no `&mut self` method, and no public field — so there is no
program that can violate V5, and no test that can meaningfully assert it. A test
would have to attempt a mutation that does not compile.

**What is deliberately not validated:**

| Not validated | Why |
|---------------|-----|
| An upper bound on slots | A capacity too large for memory fails in the crate that allocates. Tier 0 has no basis for a number |
| That the capacity suits the workload | 2 slots is legal and usually wrong. Suitability is a benchmark question, not a type question |
| That the capacity matches an existing ring's | Nothing here knows about rings — `Capacity` is a number with a proof attached |

**Failure modes if validation were removed:**

| Removed | Immediate symptom | Where it surfaces |
|---------|-------------------|-------------------|
| V1 | `mask()` underflows to `usize::MAX` | Silently, in release — every sequence folds to a slot index beyond any allocation |
| V2 | `n & mask()` stops equalling `n % get()` | Silently — slots collide or are never reached, so a ring loses or duplicates entries with no error |
| Both | — | **Neither failure produces an error.** They produce wrong addresses, which is why the check is at construction and not at use |

**The last row is the argument for the whole type.** A validation whose failure
mode is a loud error can reasonably be deferred to the point of use; a
validation whose failure mode is a silently wrong memory offset cannot.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_validating_a_slot_count_to_a_power_of_two.md](../algorithm/001_validating_a_slot_count_to_a_power_of_two.md) | V1 and V2 as a procedure, including why their order matters |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_vocabulary_surface.md](../api/001_the_vocabulary_surface.md) | `Capacity` among the six exported names, and its fifteen consumers |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_two_position_types_and_the_fold_between_them.md](../data_structure/001_two_position_types_and_the_fold_between_them.md) | The fold `mask` serves, and the two position types either side of it |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_every_capacity_has_a_valid_mask.md](../invariant/001_every_capacity_has_a_valid_mask.md) | V3 and V4, stated as the family-wide invariant they are |

### Items

| File | Relationship |
|------|--------------|
| [../item/associated_function/001_capacity_new.md](../item/associated_function/001_capacity_new.md) | The one constructor |
| [../item/associated_function/002_capacity_get.md](../item/associated_function/002_capacity_get.md) | The slot count accessor |
| [../item/associated_function/003_capacity_mask.md](../item/associated_function/003_capacity_mask.md) | The unchecked subtraction V1 makes safe |
| [../item/struct/001_capacity.md](../item/struct/001_capacity.md) | The declaration and its usage across the family |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_a_slot_count_from_request_to_mask.md](../lifecycle/001_a_slot_count_from_request_to_mask.md) | A value's arc from an unvalidated number to a fold |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_a_newtype_that_makes_a_check_unnecessary.md](../pattern/002_a_newtype_that_makes_a_check_unnecessary.md) | The general form this type instantiates |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/003_a_capacity_request_through_validation.md](../lifecycle/003_a_capacity_request_through_validation.md) | The states V1 and V2 separate |

### Types

| File | Relationship |
|------|--------------|
| [002_ring_error.md](002_ring_error.md) | The error `new` returns, and the two variants only this type produces |

### Sources

| File | Relationship |
|------|--------------|
| [`src/capacity.rs`](../../src/capacity.rs) | The whole type — 79 lines, of which 12 are the constructor |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/types_test.rs`](../../tests/types_test.rs) | ✅ V1 and V2 asserted by `capacity_rejects_zero_and_non_powers_of_two`; V3 and V4 by `capacity_accepts_powers_of_two`, which checks the mask identity across the legal range rather than at one point; ordering by `capacity_compares_by_slot_count`. ❌ **V5 is not asserted and cannot be** — a test that mutates the field does not compile, which is the enforcement |
| [`tests/manual/readme.md`](../../tests/manual/readme.md) | Where V5's kind of claim belongs — a compile-failure check a human runs, not an assertion |

### TY51 — `Capacity` Is the Only Public Type Here Without `Default`

```sh
cd "$(git rev-parse --show-toplevel)"/ring_types
awk '/^#\[ derive/ { d = $0 }
     /^pub (struct|enum) / { n = $3; sub( /[({;].*/, "", n );
       printf "%-16s Default: %s\n", n, ( d ~ /Default/ ? "yes" : "no" ) }' src/*.rs
```

Live output:

```
Capacity         Default: no
RingError        Default: no
Seq              Default: yes
SlotIndex        Default: yes
WaitKind         Default: yes
OverflowPolicy   Default: yes
```

`Capacity`'s absence is load-bearing: a default capacity would be a number
someone picked, and every ring in the family would inherit it silently. The type
has no sensible zero — which is exactly the argument that made `CapacityZero` an
error rather than a default.

**This corrects the Definition section above**, which says every other type in
this crate derives or declares a `Default`. Four of the five do; `RingError` does
not, and its absence needs no argument — a default error is not a thing. What the
sentence should say is that `Capacity` is the only type here whose missing
`Default` is a design decision rather than a category error.

The finding is that the four types *with* `Default` include `Seq`, whose default is
`Seq( 0 )` — a value the family otherwise only ever writes as `Seq::ZERO`
(→ [`../item/002`](../item/002_three_newtypes_and_two_open_fields.md), TY15), and
a third spelling of the same sequence.
