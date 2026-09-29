# Non-Functional Requirement: Errors and Positions Do Not Allocate

### Scope

- **Purpose**: State the allocation-freedom requirement every type in this crate satisfies, give the measurement that would detect a violation, and record what the requirement has already cost the family.
- **Responsibility**: State the quality attribute, the requirement, its measurement method, and its acceptance threshold.
- **In Scope**: All six exported types; the `Copy` bound that implies the property; the one consumer the requirement drove away.
- **Out of Scope**: The enum-set closure requirement (→ [`non_functional_requirement/002`](002_the_enum_sets_are_closed_and_asserted.md)); the acyclicity rule (→ [`invariant/002`](../invariant/002_tier_zero_depends_on_nothing.md)).

### Quality Attribute

**Performance predictability, specifically on the tick path.**

The write path is measured under a per-tick latency budget. A type that may
allocate introduces a variable-cost operation into a
path whose whole purpose is to have a bounded one — and an allocation in an
*error* path is worse than one in the success path, because it fires exactly
when the system is already under pressure. A ring returns `Full` when producers
outrun consumers; allocating to report it makes the overloaded case the
expensive one.

The attribute is not throughput. It is the absence of a variable-cost operation
where the measurement assumes a fixed one.

### Statement

**Every type this crate exports is `Copy`, allocation-free, and has no
destructor.**

```text
R1.  Every exported type derives Copy
R2.  No exported type owns heap memory — every field is a usize, a u64,
     or a fieldless discriminant
R3.  No exported type has a Drop impl — implied by R1, which forbids it
R4.  Constructing, propagating, classifying and dropping any of them
     performs zero allocations
```

**R1 implies R3 and is implied by R2, which is why the requirement is
enforceable by a derive.** `Copy` and `Drop` are mutually exclusive in Rust, and
`Copy` cannot be derived for a type owning heap memory. So a single derive on
each type carries the whole requirement, and violating it fails to compile
rather than failing a benchmark.

| Type | Size | `Copy` | Owns heap | Has `Drop` |
|------|-----:|:------:|:---------:|:----------:|
| `Capacity` | 8 | ✅ | ❌ | ❌ |
| `Seq` | 8 | ✅ | ❌ | ❌ |
| `SlotIndex` | 8 | ✅ | ❌ | ❌ |
| `WaitKind` | 1 | ✅ | ❌ | ❌ |
| `OverflowPolicy` | 1 | ✅ | ❌ | ❌ |
| `RingError` | 24 | ✅ | ❌ | ❌ |

**`RingError` is the type the requirement is really about.** The other five are
obviously small; the error is the one a designer would naturally give a
`String`, a `Box< dyn Error >`, or a backtrace. It has none of those, and
`src/error.rs:16` states why in one line: *"`Copy` and allocation-free — an error
on the tick path must not allocate."*

**The one place the requirement is visibly *not* free:** formatting.
`Display::fmt` writes into a caller-supplied formatter, so whether reporting an
error allocates is the caller's choice, not this crate's. R4 covers construction,
propagation, classification and drop — not the caller's decision to format into a
`String`.

### Measurement Method

**Three checks, in increasing cost:**

**1. The compile-time check, which is the enforcement.** A generic bound that
only a `Copy` type satisfies:

```rust
fn accepts_copy< T : Copy >( _ : T ) {}
accepts_copy( RingError::Full );
```

This is what `error_is_copy` in the suite does. It is not a sampling
measurement — it either compiles or it does not, for the whole type.

**2. The layout check, which pins the sizes:**

```sh
cd "$(git rev-parse --show-toplevel)"
cargo build -p ring_types --quiet
```

Live output:

```
```

```rust
size_of::< RingError >()                      // 24, align 8
size_of::< Result< Capacity, RingError > >()  // 24
size_of::< WaitKind >()                       // 1
```

**The middle line is the one worth pinning.** `Result< Capacity, RingError >`
being the same 24 bytes as the error alone means the fallible constructor costs
nothing over an infallible one — so no future optimisation should trade the
specific `CapacityNotPowerOfTwo( n )` diagnostic for a smaller result.

**3. The heap check, which nothing currently runs.** A test allocator counting
allocations across a construct-propagate-classify-drop cycle would measure R4
directly rather than inferring it from R1. It does not exist, and R1's compile
check makes it close to redundant — `Copy` cannot own an allocation. It would
catch only an allocation performed *by* a method rather than *held in* a type,
which no method here does, since every body is a field read, an arithmetic
expression or a `matches!`.

### Acceptance Threshold

| # | Threshold | Measured by | State |
|---|-----------|-------------|-------|
| T1 | All six types are `Copy` | Compile-time bound; `error_is_copy` | ✅ **Met** |
| T2 | `RingError` ≤ 24 bytes | `size_of` | ✅ Met — and 24 is set by `BatchTooLarge` alone |
| T3 | `Result< T, RingError >` costs no more than `RingError` for any `T` ≤ 16 bytes | `size_of` | ✅ Met for every type in this crate |
| T4 | Zero allocations across E1–E5 of an error's lifecycle | Implied by T1; **not directly measured** | ✅ Met by construction |
| T5 | Zero `Drop` impls in the crate | `grep`, and T1 forbids them | ✅ Met |

**T2's threshold is a ceiling, not a target.** Twenty-four bytes buys
`BatchTooLarge { requested, capacity }` — a message naming both the request and
the limit. Shrinking to 16 would delete one of those numbers, and since T3
already holds, there is nothing to gain.

**T4 is met by construction and unmeasured, which is the right trade here.**
Building a counting-allocator harness to verify a property the type system
already forbids violating would be effort spent confirming the compiler.

**The threshold this requirement fails is not in the table, because it is not
this crate's.** R1 forbids `RingError::NameTaken` from carrying the name it is
about, so `ring_registry` declared its own error type — non-`Copy`, allocating
on construction, and correct for its off-tick-path use. **The requirement is met
and its cost is a second error type in a family whose tier 0 opens by claiming
to have one.** That cost is recorded in
[`integration/002`](../integration/002_the_registry_that_declined_the_shared_error.md)
and is worth restating here: an NFR met at 100% can still be the reason a
different property is at 0%.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_vocabulary_surface.md](../api/001_the_vocabulary_surface.md) | The six types the requirement quantifies over |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_error_enum_as_a_closed_copy_set.md](../data_structure/002_the_error_enum_as_a_closed_copy_set.md) | Where the 24 bytes go, and the two other constraints beside `Copy` |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_the_registry_that_declined_the_shared_error.md](../integration/002_the_registry_that_declined_the_shared_error.md) | What R1 cost — the consumer that could not use a `Copy` error |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_tier_zero_depends_on_nothing.md](../invariant/002_tier_zero_depends_on_nothing.md) | The companion constraint — no dependencies, hence a hand-written `Display` |

### Items

| File | Relationship |
|------|--------------|
| [../item/enum/002_ring_error.md](../item/enum/002_ring_error.md) | The type the requirement is really about |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_an_error_from_construction_to_display.md](../lifecycle/002_an_error_from_construction_to_display.md) | T4's five phases, and the one that can allocate on the caller's behalf |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [002_the_enum_sets_are_closed_and_asserted.md](002_the_enum_sets_are_closed_and_asserted.md) | The crate's other requirement — enforced by a hand-written array rather than a derive |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_capacity.md](../type/001_capacity.md) | T3's `Ok` payload |
| [../type/002_ring_error.md](../type/002_ring_error.md) | T1, T2, T5 as the type's own rules |

### Sources

| File | Relationship |
|------|--------------|
| [`src/error.rs`](../../src/error.rs) | Line 16, the requirement stated in place; line 42, the derive that enforces it |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/types_test.rs`](../../tests/types_test.rs) | ✅ T1 asserted by `error_is_copy`, using a generic `Copy` bound rather than a value check — so it covers the type, not a sample. ❌ **T2, T3 and T5 are unasserted.** All three are one `assert_eq!( size_of::< … >(), … )` away, and their absence means a variant carrying three `usize` fields would grow the enum, break T3's free `Result`, and pass the whole suite |

### TY44 — The No-Allocation Guarantee Costs the Family Its Single Error Type

The requirement is met and the price is legible: one extra public error type,
one hand-written conversion at `ring_factory:214`, two unconstructed variants
here, and one message in the family that can name the colliding ring — thrown
away before it reaches a consumer
(→ [`../integration/002`](../integration/002_the_registry_that_declined_the_shared_error.md), TY33).

**Recorded as a measured cost rather than a defect.** An allocation on the error
path of a ring is the thing this requirement exists to prevent, and every item in
the list above is cheaper than that.

**Disposition:** declined — this instance's own conclusion records the trade
as accepted by design, not a defect; the one item in its own list with a
further correction available (the colliding-ring name thrown away before it
reaches a consumer) is tracked as TY33 in
`../integration/002_the_registry_that_declined_the_shared_error.md`, not this
instance's own action.
