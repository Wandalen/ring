# Algorithm: Validating a Slot Count to a Power of Two

### Scope

- **Purpose**: Specify the two-test validation [`Capacity::new`](../item/associated_function/001_capacity_new.md) performs, and why passing it makes [`mask`](../item/associated_function/003_capacity_mask.md) total rather than merely usually-correct.
- **Responsibility**: State the abstract and the algorithm.
- **In Scope**: The two rejections, their order, the `const` context, and the arithmetic identity the mask relies on.
- **Out of Scope**: The fold from a `Seq` to a `SlotIndex` (owned by `ring_index`); the invariant this algorithm establishes (→ [`invariant/001`](../invariant/001_every_capacity_has_a_valid_mask.md)).

### Abstract

**Two tests, in a fixed order, and the second is the one the family is built
on.** A slot count is rejected if it is zero and rejected if it has more than
one bit set; anything surviving both is wrapped in a `Capacity` whose inner
field is private and never written again.

The point of doing this in a type rather than at each use site is that
`ring_index`'s fold — sequence to slot — is `seq & mask` instead of
`seq % capacity`. The bitmask is only equivalent to the modulo when the
capacity is a power of two, so the equivalence is a precondition, and this
algorithm is where it is discharged. **Fifteen crates take the mask's validity
on trust; one function checks it.**

That requirement is what this algorithm discharges. The design choice this
instance records is where the check lives: once, at construction, rather than
at each of the fifteen places a capacity is read.

### Algorithm

**Input:** `slots : usize`, unvalidated — from a config, a manifest row, or a
literal.
**Output:** `Result< Capacity, RingError >`.

```sh
cd "$(git rev-parse --show-toplevel)"
awk '/pub const fn new\( slots : usize \) -> Result< Self, RingError >/,/^  }$/' ring_types/src/capacity.rs
```

Live output:

```
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

| Step | Test | On failure | Why here |
|------|------|-----------|----------|
| 1 | `slots == 0` | `RingError::CapacityZero` | A zero ring can never accept a publish; rejecting at construction beats failing at first use |
| 2 | `!slots.is_power_of_two()` | `RingError::CapacityNotPowerOfTwo( slots )` | Establishes `mask == slots - 1` as a correct fold |
| 3 | — | `Ok( Self( slots ) )` | The private field is written once and never again |

**Step 1 must precede step 2, and not for the reason it looks like.**
`usize::is_power_of_two()` already returns `false` for zero, so step 2 alone
would reject every illegal input — step 1 is not required for *correctness*. It
is required for the *error*: without it, a zero capacity reports
`CapacityNotPowerOfTwo( 0 )`, which is true and useless. The ordering buys a
specific diagnostic for the most likely mistake, which is an unset field
defaulting to zero.

```rust
assert!( !0_usize.is_power_of_two() );   // step 2 would catch it
assert_eq!( Capacity::new( 0 ).unwrap_err(), RingError::CapacityZero );
```

**Step 2 carries the offending value; step 1 does not.** `CapacityZero` is a
unit variant because there is only one zero, and a caller that asked for zero
knows it. `CapacityNotPowerOfTwo( usize )` carries `slots` because "not a power
of two" describes infinitely many values and a caller reading a log needs to
know which one arrived — typically 100, 1000, or some other decimal-round number
a human typed.

**Step 3's guarantee is what the rest of the family consumes.** After it, the
identity holds unconditionally:

```text
for every Capacity c :  c.mask() == c.get() - 1
                        c.get() is a power of two, so
                        ( n & c.mask() ) == ( n % c.get() )   for all n
```

The subtraction in [`mask`](../item/associated_function/003_capacity_mask.md)
cannot underflow because step 1 excluded zero, and the bitwise identity holds
because step 2 excluded everything else. **Neither guarantee is asserted at the
call site anywhere in the family, because neither can fail.**

**The whole function is `const`**, which is what lets a capacity be validated at
compile time when the slot count is a literal:

```rust
const CAP : Capacity = match Capacity::new( 1024 )
{
  Ok( c ) => c,
  Err( _ ) => panic!( "not a power of two" ),
};
```

A `const` evaluation of an illegal literal is a compile error rather than a
runtime `Err` — the strongest form the check can take, available for free
because the two tests are both `const`-compatible. `usize::is_power_of_two` has
been `const` since Rust 1.32, so nothing here required a workaround.

**What the algorithm deliberately does not do:**

| Not done | Why |
|----------|-----|
| Round up to the next power of two | Silent correction. The caller asked for 100 and would get 128 with no signal — the same class of defect `ring_config`'s clamping setters are criticised for |
| Impose an upper bound | A capacity that does not fit in memory fails at allocation, in the crate that allocates. Tier 0 has no basis for a limit |
| Return the mask directly | The mask alone cannot answer `get()`, and half the consumers want the count rather than the fold |
| Take `NonZeroUsize` | Would move step 1 to the caller and change its error from `CapacityZero` to a construction the caller must already have handled |

**The last row is the closest call.** `NonZeroUsize` would encode step 1 in the
type system, which is strictly stronger — but it moves the rejection to the
caller's `NonZeroUsize::new`, whose error is an `Option`, not a `RingError`. The
family would gain a type-level guarantee and lose the specific diagnostic that
step 1's ordering exists to produce. Recorded as a question in
[`decisions/`](../decisions/readme.md) rather than settled here.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_vocabulary_surface.md](../api/001_the_vocabulary_surface.md) | `Capacity::new` as the crate's one fallible operation |

### Algorithms

| File | Relationship |
|------|--------------|
| [002_classifying_an_error_into_configuration_or_traffic.md](002_classifying_an_error_into_configuration_or_traffic.md) | Where this algorithm's two errors land in the classifier — both configuration |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_two_position_types_and_the_fold_between_them.md](../data_structure/001_two_position_types_and_the_fold_between_them.md) | The fold the mask serves, and the two types it moves between |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_every_capacity_has_a_valid_mask.md](../invariant/001_every_capacity_has_a_valid_mask.md) | The property step 3 establishes, stated as an invariant with its enforcement |

### Items

| File | Relationship |
|------|--------------|
| [../item/associated_function/001_capacity_new.md](../item/associated_function/001_capacity_new.md) | The declaration |
| [../item/associated_function/003_capacity_mask.md](../item/associated_function/003_capacity_mask.md) | The consumer of the guarantee |
| [../item/struct/001_capacity.md](../item/struct/001_capacity.md) | The newtype whose private field makes step 3 permanent |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_a_slot_count_from_request_to_mask.md](../lifecycle/001_a_slot_count_from_request_to_mask.md) | This algorithm as one phase of a longer arc |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_a_newtype_that_makes_a_check_unnecessary.md](../pattern/002_a_newtype_that_makes_a_check_unnecessary.md) | The general shape — validate once, so fifteen consumers need not |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/003_a_capacity_request_through_validation.md](../lifecycle/003_a_capacity_request_through_validation.md) | The states this algorithm moves between |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_capacity.md](../type/001_capacity.md) | The type and its validation rules |

### Sources

| File | Relationship |
|------|--------------|
| [`src/capacity.rs`](../../src/capacity.rs) | Lines 40–51, the algorithm; 65–78, the guarantee it discharges |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/types_test.rs`](../../tests/types_test.rs) | ✅ `capacity_accepts_powers_of_two` and `capacity_rejects_zero_and_non_powers_of_two` cover both steps, including step 1's ordering — the zero case asserts `CapacityZero` specifically, so a reordering that produced `CapacityNotPowerOfTwo( 0 )` fails rather than passing on a technicality |

### TY19 — The Validation Ran at Two Places in a Live Program, Now Runs at One

One call site now, not two. `ring_config:71` validates a slot count a caller
supplied — the real entry point, and the only place the two tests in
`Capacity::new` can fail on user input. `ring_core:219` used to validate a
number this crate already produced a second time;
[TY20](#ty20--an-infallible-accessor-re-runs-the-validation-and-panics-on-failure)'s
fix removed that call, so the accessor now carries the validated value forward
instead of re-deriving it.

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r 'Capacity::new' ring_*/src/*.rs | grep -v '^ring_types/' \
  | grep -vE ': *(//|///|//!)'
```

Live output:

```
ring_config/src/lib.rs:        capacity : Capacity::new( slots )?,
```

**The validation this algorithm describes therefore guards one door.** Everything
downstream of `ring_config` holds a `Capacity` whose invariant is carried by the
type rather than re-established, which is the property
[`../pattern/002`](../pattern/002_a_newtype_that_makes_a_check_unnecessary.md)
argues for — and it is worth stating as a count, because a reader of this
algorithm would reasonably assume it runs wherever a ring is built.

### TY20 — An Infallible Accessor Re-Runs the Validation and Panics on Failure

The crossbeam arm of `Ring::capacity` cannot return the `Capacity` it was
built from — it asks the queue and rebuilds one:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the accessor, found by name rather than by line number --'
awk '/pub fn capacity\( &self \) -> Capacity/,/^  }$/' ring_core/src/lib.rs
echo '  -- and what it re-validates on the way --'
printf '    validations inside the accessor: %s\n' "$( awk '/pub fn capacity\( &self \) -> Capacity/,/^  }$/' ring_core/src/lib.rs | command grep -cE 'Capacity::new|\.expect\(' )"
printf '    Capacity::new().expect in crate: %s\n' "$( command grep -cE 'Capacity::new\(.*\)\.expect' ring_core/src/lib.rs )"
printf '    tests holding all three equal:   %s\n' "$( command grep -c 'fn every_backend_reports_the_capacity_it_was_configured_with' ring_core/tests/core_test.rs )"
```

Live output:

```
  -- the accessor, found by name rather than by line number --
  pub fn capacity( &self ) -> Capacity
  {
    match &self.storage
    {
      Storage::Spsc( ring ) => ring.capacity(),
      Storage::Mpsc( ring ) => ring.capacity(),
      // The validated `Capacity` is carried in the variant rather than rebuilt
      // from `ArrayQueue::capacity`. Rebuilding it re-ran this crate's only
      // fallible validation inside an infallible accessor, so a backend that
      // ever rounded its capacity would turn `ring.capacity()` into a panic.
      #[ cfg( feature = "crossbeam" ) ]
      Storage::Crossbeam( _, capacity ) => *capacity,
    }
  }
  -- and what it re-validates on the way --
    validations inside the accessor: 0
    Capacity::new().expect in crate: 0
    tests holding all three equal:   1
```

**What was found.** `crossbeam::ArrayQueue::capacity` returned the requested
capacity, so the `expect` never fired. The hazard was that nothing held it to
that: the safety argument was a string literal inside the `expect`, no test
asserted that a crossbeam ring reports the capacity it was built from, and the
signature (`pub fn capacity( &self ) -> Capacity`) told a caller that reading a
capacity cannot fail.

**It was the one place in the family where this crate's validation could abort a
program**, and it was reached by an accessor rather than by a constructor —
which is what made it worth acting on rather than recording.

**Disposition:** applied — `Storage::Crossbeam` now carries the validated
`Capacity` it was constructed from, so the accessor returns it instead of
rebuilding one, and `every_backend_reports_the_capacity_it_was_configured_with`
holds all three backends to the capacity they were configured with across four
sizes. Now prints: `validations inside the accessor: 0`
