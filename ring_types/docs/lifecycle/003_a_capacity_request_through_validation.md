# Lifecycle: A Capacity Request Through Validation

### Scope

- **Purpose**: Model a slot count as a state machine with four states and one unreachable fifth, and show that the fifth's unreachability — not the validation itself — is what seventeen downstream crates actually rely on.
- **Responsibility**: State the states, transitions, and behavioral invariants.
- **In Scope**: The four reachable states; the six transitions; the derive that is absent and why its absence is the machine's central fact.
- **Out of Scope**: The two tests as an algorithm (→ [`algorithm/001`](../algorithm/001_validating_a_slot_count_to_a_power_of_two.md)); the same arc as a lifecycle across crates (→ [`lifecycle/001`](../lifecycle/001_a_slot_count_from_request_to_mask.md)).

### States

| # | State | Rust representation | Reachable |
|---|-------|---------------------|:---------:|
| S0 | **Unvalidated** | `usize` | ✅ Every caller starts here |
| S1 | **Refused — zero** | `Err( RingError::CapacityZero )` | ✅ |
| S2 | **Refused — wrong shape** | `Err( RingError::CapacityNotPowerOfTwo( n ) )` | ✅ |
| S3 | **Validated** | `Capacity` | ✅ |
| S4 | **Validated but invalid** | a `Capacity` whose field is 0 or not a power of two | ❌ **unreachable** |

**S4 is the state the whole design exists to delete.** Every guarantee
downstream — `mask()` not underflowing, `&` folding across all slots, fifteen
crates not re-testing — is a claim that S4 is empty. Validation alone does not
establish that; validation only ensures S3 is *entered* correctly. What ensures
S3 cannot be entered any other way is the shape of the type.

**S1 and S2 are distinct states rather than one `Refused`.** They carry
different payloads and answer differently under classification: `CapacityZero`
is a bare discriminant, `CapacityNotPowerOfTwo( n )` carries the rejected
number, and both answer `true` to `is_configuration`. Merging them would cost
the caller the number they asked for.

**S0 has no representation of its own.** An unvalidated slot count is a plain
`usize` — indistinguishable from a length, an offset, or a loop bound. That is
the problem the machine solves, not a defect in the modelling.

### Transitions

| # | From → To | Trigger | Guard |
|---|-----------|---------|-------|
| T1 | S0 → S1 | `Capacity::new( slots )` | `slots == 0` |
| T2 | S0 → S2 | `Capacity::new( slots )` | `slots != 0 && !slots.is_power_of_two()` |
| T3 | S0 → S3 | `Capacity::new( slots )` | `slots != 0 && slots.is_power_of_two()` |
| T4 | S3 → S3 | Copy into a struct, a call, another crate | — |
| T5 | S3 → S0 | `get()` | — (the `usize` comes back out, unvalidated again) |
| T6 | S3 → ∅ | The holder drops | — |

```rust
pub const fn new( slots : usize ) -> Result< Self, RingError >
{
  if slots == 0
  {
    return Err( RingError::CapacityZero );
  }
  if !slots.is_power_of_two()
  {
    return Err( RingError::CapacityNotPowerOfTwo( slots ) );
  }
  Ok( Self( slots ) )
}
```

**T1, T2 and T3 are the same call.** One function, three outcomes, and the
guards are evaluated in the order written — which matters for *which error* a
zero produces, not for whether it is refused. `0.is_power_of_two()` is already
`false`, so swapping the two `if`s would still refuse zero, with the less useful
message (→ [`algorithm/001`](../algorithm/001_validating_a_slot_count_to_a_power_of_two.md)).

**T5 is the transition worth noticing.** `get()` returns a bare `usize`, which
is S0 again — the machine is exited as easily as it is entered, and a value that
makes a round trip through `get()` must be re-validated to come back. Nothing
prevents `Capacity::new( cap.get() ).unwrap()`, and nothing needs to: the round
trip is total, since a validated number passes both guards by construction.

**One of the family's `Capacity::new` call sites used to be exactly this round
trip, until [TY20](../algorithm/001_validating_a_slot_count_to_a_power_of_two.md#ty20--an-infallible-accessor-re-runs-the-validation-and-panics-on-failure)'s
fix removed it:**

```rust
// ring_core/src/lib.rs:219, before the TY20 fix
Capacity::new( queue.capacity() ).expect( "constructed from a validated Capacity" )
```

A `Capacity` was handed to crossbeam's `ArrayQueue` at construction; asking the
queue for its capacity got a `usize` back — S3 → T5 → S0 — and re-entering S3
required T3 again. The `expect` was justified by the totality of the round trip,
and the justification was written into the message rather than left implicit.
**There was no unchecked constructor to reach for instead**, which is what made
this the well-behaved outcome rather than a nuisance — though "well-behaved"
still meant a fallible re-validation of a value the crate already knew was
valid, which is what made it TY20's hazard rather than a design worth keeping.
`ring_core::Ring::capacity` now carries the `Capacity` it was constructed from
instead of performing this round trip, so no production call site in the
family exercises T5 today; `Capacity::new( cap.get() ).unwrap()` remains valid
Rust for a caller who chooses to write it, but nothing in the family does.

**There is no transition into S4.** That is the machine's only interesting
absence, and it is enforced by three separate omissions rather than by any
transition guard:

| Mechanism | Would create S4 if present |
|-----------|----------------------------|
| Private tuple field | `Capacity( 6 )` from any crate |
| No `Default` derive | `Capacity::default()` → `Capacity( 0 )` |
| No second constructor | `Capacity::new_unchecked` |
| No `&mut self` method | `cap.set( 6 )` after construction |

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'Default' ring_types/src/*.rs | sort
```

Live output:

```
ring_types/src/id.rs:#[ derive( Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default ) ]
ring_types/src/id.rs:#[ derive( Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default ) ]
ring_types/src/policy.rs:#[ derive( Debug, Clone, Copy, PartialEq, Eq, Hash, Default ) ]
ring_types/src/policy.rs:#[ derive( Debug, Clone, Copy, PartialEq, Eq, Hash, Default ) ]
```

*(`| sort` is not cosmetic — the installed `grep` groups multi-file output in
its own order, which varies between runs. Two consecutive invocations here
returned `id.rs` first and `policy.rs` first respectively.)*

**Four of the crate's five derive-carrying types have `Default`. `Capacity` is
the one that does not**, and it is the one with an invariant. `Seq::default()`
is `Seq( 0 )`, a perfectly good starting position; `Capacity::default()` would
be `Capacity( 0 )`, which is S4. The absence is not an oversight in a
copy-pasted derive list — it is the difference between the two kinds of newtype
this crate declares (→ [`pattern/002`](../pattern/002_a_newtype_that_makes_a_check_unnecessary.md)).

### Behavioral Invariants

| # | Invariant | Holds because |
|---|-----------|---------------|
| B1 | S3 is entered only via T3 | The field is private and `new` is the only constructor |
| B2 | Once in S3, a value never leaves except by T5 or T6 | No `&mut self` method exists anywhere in `capacity.rs` |
| B3 | S4 is unreachable from any state | The four omissions above, jointly |
| B4 | T1–T3 are total over `usize` | Every `usize` satisfies exactly one guard |
| B5 | The machine is memoryless | S3 stores the accepted value and nothing about the request |
| B6 | Every transition is `const`-evaluable | `new`, `get` and `mask` are all `const fn` |

**B2 is verifiable in one line:**

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -c '&mut self' ring_types/src/capacity.rs
```

Live output:

```
0
```

Seventeen crates hold a `Capacity`; none of them can mutate one. The value is
`Copy`, so what travels is a duplicate, and there is no shared cell for a
mutation to be observed through.

**B3 is the invariant everything downstream is actually built on**, and it is
the one nothing asserts. B1 and B2 have tests; B3 cannot have one — a test
attempting `Capacity( 6 )` would fail to compile, so it cannot be written as a
test in the first place. The mechanism is stronger than a test and leaves
nothing in the suite to point at
(→ [`invariant/001`](../invariant/001_every_capacity_has_a_valid_mask.md)).

**B4 makes the refusals exhaustive rather than best-effort.** There is no
`usize` for which `Capacity::new` returns something undefined, panics, or
silently corrects. Compare `ring_config`'s setters, which clamp — a caller who
asks for an out-of-range value there gets a different value back, not an error.
Here, asking for 100 yields `CapacityNotPowerOfTwo( 100 )` rather than 128.

**B5 is a deliberate loss.** Nothing recorded in S3 says what was requested, so
a diagnostic that wants to say "you asked for 100, the ring has 128" cannot be
written from a `Capacity` — but nothing rounds, so the situation does not arise
inside this machine. It arises one layer up, wherever a caller chooses to round
before calling, and that caller owns the message.

**B6 is what lets the whole machine run at compile time:**

```rust
const CAP : Capacity = match Capacity::new( 1024 )
{
  Ok( c ) => c,
  Err( _ ) => panic!( "not a power of two" ),
};
```

A literal capacity is validated during compilation and a bad literal is a build
failure rather than a startup one. Nothing in the family currently does this:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -rn -B2 'Capacity::new' */ --include=*.rs \
  | command grep -E '^\S+[-:][0-9]+[-:].*\bconst\b' | command grep -v 'const fn' || true
```

Live output:

```
```

**Not one `const` binding anywhere in the workspace**, so B6 is available and
entirely unexercised. Nor is there much to convert: the family has exactly two
production `Capacity::new` call sites, and neither takes a literal — one
validates a configured `slots` field, the other re-validates a number returned
by crossbeam. B6's value is therefore to an external consumer with a fixed
capacity in their own code, which is the same absent Contract consumer the
classifier predicates were written for
(→ [`api/002`](../api/002_the_five_classifier_predicates.md)).

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_validating_a_slot_count_to_a_power_of_two.md](../algorithm/001_validating_a_slot_count_to_a_power_of_two.md) | T1–T3's guards, and why their order is about the message |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_vocabulary_surface.md](../api/001_the_vocabulary_surface.md) | G4, and the `const` evaluation B6 enables |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_every_capacity_has_a_valid_mask.md](../invariant/001_every_capacity_has_a_valid_mask.md) | B3 stated as the crate's invariant, with its three mechanisms |

### Items

| File | Relationship |
|------|--------------|
| [../item/struct/001_capacity.md](../item/struct/001_capacity.md) | The private field, and the derive list `Default` is absent from |
| [../item/associated_function/001_capacity_new.md](../item/associated_function/001_capacity_new.md) | T1, T2, T3 |
| [../item/associated_function/002_capacity_get.md](../item/associated_function/002_capacity_get.md) | T5 |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_a_slot_count_from_request_to_mask.md](../lifecycle/001_a_slot_count_from_request_to_mask.md) | The same arc across four crates, with the fold S3 makes safe |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_a_newtype_that_makes_a_check_unnecessary.md](../pattern/002_a_newtype_that_makes_a_check_unnecessary.md) | Why `Capacity` has no `Default` and `Seq` does |

### State Machines

| File | Relationship |
|------|--------------|
| [004_an_overflow_policy_from_declaration_to_refusal.md](004_an_overflow_policy_from_declaration_to_refusal.md) | The crate's other machine — a discriminant whose states are held by other crates |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_capacity.md](../type/001_capacity.md) | V1–V5 as rules over these states |
| [../type/002_ring_error.md](../type/002_ring_error.md) | S1 and S2's payloads |

### Sources

| File | Relationship |
|------|--------------|
| [`src/capacity.rs`](../../src/capacity.rs) | Lines 22–23 the derive list and private field; 40–51 the three transitions; 60 and 75 the accessors |
| [`src/id.rs`](../../src/id.rs) | Lines 24 and 98 — the two types that do derive `Default`, and safely |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/types_test.rs`](../../tests/types_test.rs) | ✅ B1 and B4 covered by `capacity_accepts_powers_of_two` and `capacity_rejects_zero_and_non_powers_of_two`, both refusals asserted by exact error value; `capacity_compares_by_slot_count` covers the ordering S3 inherits from its derive. ❌ **B3, B5 and B6 are unasserted.** B3 is untestable by construction; B6 is testable in one `const` binding and would pin a property no current call site uses; B5 is a design choice with nothing to assert |

### TY41 — The Unreachable State Is Guaranteed by the Private Field Alone

A test can show that `Capacity::new` rejects zero and rejects six. It cannot show
that no *other* path produces an invalid `Capacity`, because demonstrating the
absence of a path is not something a test can do.

What guarantees it is that the field is private and `new` is the only
constructor — the property
[`../item/002`](../item/002_three_newtypes_and_two_open_fields.md) records as
holding for exactly one of the crate's three newtypes. **The unreachable state is
unreachable because of an access modifier, and the corpus should say so rather
than crediting the validation.**
