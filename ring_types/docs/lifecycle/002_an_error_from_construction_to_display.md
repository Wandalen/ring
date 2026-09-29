# Lifecycle: An Error From Construction to Display

### Scope

- **Purpose**: Trace a `RingError` from the crate that detects a failure to the caller that reports it, and identify the two variants whose arc has no first phase.
- **Responsibility**: State the lifecycle phases, phase transitions, dependencies, and cleanup requirements.
- **In Scope**: Five phases; who constructs, who classifies, who formats; the arc's cost in allocations.
- **Out of Scope**: The classifier membership sets (→ [`algorithm/002`](../algorithm/002_classifying_an_error_into_configuration_or_traffic.md)); the second error type in the family (→ [`integration/002`](../integration/002_the_registry_that_declined_the_shared_error.md)).

### Lifecycle Phases

| # | Phase | Owner | What exists |
|---|-------|-------|-------------|
| E1 | **Constructed** | The crate that detected the failure | A 24-byte `Copy` value, built by literal syntax at the failure site |
| E2 | **Propagated** | Every crate on the return path | The same value, copied through `?` and `Result` |
| E3 | **Classified** | The recipient — in practice, nobody | `is_configuration` / `is_transient` |
| E4 | **Formatted** | The recipient | `Display::fmt` writes into a formatter |
| E5 | **Dropped** | Whoever holds it last | Nothing happens |

**E1 is not owned by this crate for seven of nine variants.** This crate
constructs `CapacityZero` and `CapacityNotPowerOfTwo` inside
`Capacity::new`; five more are constructed elsewhere in the family; two are
constructed nowhere:

| Variant | E1 owner | Sites |
|---------|----------|------:|
| `CapacityZero` | This crate | 1 |
| `CapacityNotPowerOfTwo` | This crate | 1 |
| `Full` | `ring_spsc`, `ring_mpsc`, `ring_overflow`, others | 36 |
| `Empty` | Consumer-side crates | 15 |
| `BatchTooLarge` | `ring_batch`, `ring_gating`, others | 10 |
| `Closed` | `ring_shutdown`, others | 5 |
| `PolicyUnsupported` | `ring_core` | 5 |
| `NameTaken` | **nobody** | 0 |
| `NameUnknown` | **nobody** | 0 |

**The two zeros mean two variants have no lifecycle at all.** They are declared,
they classify (as neither), they format, and they are tested — but no phase E1
exists for them, so E2 through E5 are unreachable. A declaration that can be
tested without ever being constructed is the shape
[`pitfall/002`](../pitfall/002_two_name_errors_nothing_constructs.md) records.

**E3 is the phase nothing performs.** Across all 33 crates there is exactly one
production call to any of the crate's five predicates, and it is not on either
error predicate (→ [`api/002`](../api/002_the_five_classifier_predicates.md)).
E3 is written for the consumer behind the export Contract, who has not arrived.

### Phase Transitions

| # | From → To | Trigger | Notes |
|---|-----------|---------|-------|
| Y1 | ∅ → E1 | A failure is detected; the variant is written as a literal | No constructor function exists — this is deliberate |
| Y2 | E1 → E2 | `return Err( … )` or `?` | A copy, not a move: `RingError` is `Copy` |
| Y3 | E2 → E2 | Crossing a crate boundary | No conversion. One error type on the ring path means no `From` chain |
| Y4 | E2 → E3 | `is_configuration()` / `is_transient()` | Optional, and skipped in practice |
| Y5 | E2 → E4 | `{}`, `to_string()`, or a `dyn Error` report | Nine `match` arms at `src/error.rs:166–181` |
| Y6 | E3 → E2 | Classification returns a `bool`; the error is untouched | The predicates take `self` by value and return nothing derived from it |
| Y7 | any → E5 | The holder drops | No `Drop` impl anywhere in the crate |

**Y3 is the phase transition the "one error type" argument was written to
buy**, and it delivers on the ring path: an error constructed in `ring_gating`
reaches `ring_factory` unchanged, through four crate boundaries, with no `From`
impl anywhere. The exception is registration, where `ring_registry` produces a
different type entirely.

**Y1 has no constructor function, and that is why the family needs no knowledge
here.** Variants are written as literals at the failure site — `Err(
RingError::Full )` — so this crate never learns who fails or why. That is what
"no ring logic" means for the error module.

**Y6 is worth stating because it is unusual.** Classification does not consume,
transform, or wrap; it reads a discriminant and returns a `bool`. An error can
be classified any number of times, in any order, and the value is identical
afterwards.

### Dependencies

| Crate | Phase | Relationship |
|-------|-------|--------------|
| **This crate** | E1 (2 variants), E3, E4, E5 | Declares the type, both predicates, and the hand-written `Display` |
| `ring_spsc`, `ring_mpsc` | E1 | The bulk of `Full` and `Empty` |
| `ring_core` | E1 | The only producer of `PolicyUnsupported` |
| `ring_shutdown` | E1 | `Closed` |
| `ring_batch`, `ring_gating` | E1 | `BatchTooLarge` |
| `ring_factory` | E2 | Wraps it as `BuildError::Unsupported( RingError )` at the Contract boundary |
| **No crate** | E1 for `NameTaken`, `NameUnknown` | The gap |

```sh
cd "$(git rev-parse --show-toplevel)"
for v in Full Empty Closed BatchTooLarge PolicyUnsupported NameTaken NameUnknown; do
  printf '%-20s %s\n' "$v" \
    "$( command grep -rn "RingError::$v" ring_*/src | command grep -vc '^ring_types/' )"
done
```

Live output:

```
Full                 36
Empty                16
Closed               5
BatchTooLarge        10
PolicyUnsupported    5
NameTaken            0
NameUnknown          0
```

**`ring_factory` is where the arc leaves the family**, and it does not leave as
a `RingError`: `Factory::build` returns `BuildError`, whose `Unsupported`
variant wraps one. So the export Contract's outermost door adds exactly one
layer, and only for construction failures — ring operations return `RingError`
directly through `ring_handle`.

**Nineteen crates name `RingError`**; five construct it; one wraps it. The ratio
is what a shared error type looks like when it works — most consumers only
propagate.

### Cleanup Requirements

**None, at every phase, and the reason is structural rather than reviewed.**

| Phase | Allocation | Destructor |
|-------|-----------:|-----------:|
| E1 | 0 | — |
| E2 | 0 | — |
| E3 | 0 | — |
| E4 | **caller's** | — |
| E5 | 0 | none |

**`RingError` is `Copy`, which forbids `Drop` outright** — the two traits are
mutually exclusive in Rust — so E5 cannot do anything, by construction. No
review is required to keep it that way; a `Drop` impl would not compile.

**E4 is the only phase that can allocate, and the allocation is not this
crate's.** `Display::fmt` writes into a caller-supplied `fmt::Formatter`; whether
that formatter's backing buffer allocates is the caller's decision. Formatting
into a stack buffer allocates nothing.

**The whole arc is allocation-free up to the caller's own reporting choice**,
which is what makes `RingError` usable on the tick path
(→ [`non_functional_requirement/001`](../non_functional_requirement/001_errors_and_positions_do_not_allocate.md)).
That property is what forbids `NameTaken` from carrying a name, which is what
cost this type its registry consumer — **the cleanup requirement being "none" is
the same fact as the family having two error types.**

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_validating_a_slot_count_to_a_power_of_two.md](../algorithm/001_validating_a_slot_count_to_a_power_of_two.md) | E1 for the two variants this crate constructs |
| [../algorithm/002_classifying_an_error_into_configuration_or_traffic.md](../algorithm/002_classifying_an_error_into_configuration_or_traffic.md) | E3 |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_five_classifier_predicates.md](../api/002_the_five_classifier_predicates.md) | Why E3 has no production performer |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_error_enum_as_a_closed_copy_set.md](../data_structure/002_the_error_enum_as_a_closed_copy_set.md) | The 24 bytes E2 copies, and the constraints that keep the arc allocation-free |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_the_registry_that_declined_the_shared_error.md](../integration/002_the_registry_that_declined_the_shared_error.md) | The one failure whose arc uses a different type entirely |

### Items

| File | Relationship |
|------|--------------|
| [../item/enum/002_ring_error.md](../item/enum/002_ring_error.md) | The declaration, and every crate that names it |
| [../item/implementation/003_impl_display_for_ring_error.md](../item/implementation/003_impl_display_for_ring_error.md) | Y5 |
| [../item/implementation/004_impl_error_for_ring_error.md](../item/implementation/004_impl_error_for_ring_error.md) | What makes E4 reachable through `dyn Error` |

### Lifecycles

| File | Relationship |
|------|--------------|
| [001_a_slot_count_from_request_to_mask.md](001_a_slot_count_from_request_to_mask.md) | The arc whose two refusals begin this one |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_errors_and_positions_do_not_allocate.md](../non_functional_requirement/001_errors_and_positions_do_not_allocate.md) | The cleanup table as a measurable requirement |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_two_name_errors_nothing_constructs.md](../pitfall/002_two_name_errors_nothing_constructs.md) | The two variants with no E1 |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_ring_error.md](../type/002_ring_error.md) | The nine variants and the rules that hold for them |

### Sources

| File | Relationship |
|------|--------------|
| [`src/error.rs`](../../src/error.rs) | E1 for two variants; E3's predicates; E4's nine arms at 166–181; E5's absence |
| [`ring_factory/src/lib.rs`](../../../ring_factory/src/lib.rs) | Where the arc leaves the family, wrapped as `BuildError::Unsupported` |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/types_test.rs`](../../tests/types_test.rs) | ✅ E3 and E4 are asserted for all nine variants — `errors_split_configuration_from_traffic`, `every_error_displays_distinctly`. ❌ **E1 is untestable from here for seven of the nine**, since this crate constructs only two; and the two with no E1 at all pass every test in this suite, which is precisely how their absence stayed invisible |

### TY40 — Seven of Nine Variants Are Not Constructed Where They Are Declared

Seven of the nine variants are constructed nowhere in `ring_types`, which is what
makes the *whole* lifecycle uncheckable from here: `ring_types` can test that an
error classifies and renders, and cannot test that it is ever raised.

Those seven do not divide evenly. Five begin their life in another crate — the
dependents that raise them. The remaining two are constructed by nothing at all,
so they have no first phase anywhere rather than one owned elsewhere
(→ [`../decisions/002`](../decisions/002_the_two_name_errors_and_the_two_crates_that_redeclared_them.md), TY6).
That division is why this instance's phases are owned by different crates.
