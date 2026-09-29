# Invariant: Every Capacity Has a Valid Mask

### Scope

- **Purpose**: State the property seventeen crates rely on without checking — that for every `Capacity` that exists, `mask()` equals `get() - 1` and folding by it is equivalent to a modulo — and identify what enforces it.
- **Responsibility**: State the invariant, its enforcement mechanism, and the consequences of violation.
- **In Scope**: The identity, the private field that makes it permanent, and the failure modes that produce no error.
- **Out of Scope**: The construction algorithm's step ordering (→ [`algorithm/001`](../algorithm/001_validating_a_slot_count_to_a_power_of_two.md)); the fold's implementation, owned by `ring_index`.

### Invariant Statement

**For every value of type `Capacity` that exists anywhere in the family:**

```text
I1.  c.get()  is a power of two, and c.get() >= 1
I2.  c.mask() == c.get() - 1              — and the subtraction never underflows
I3.  ( n & c.mask() ) == ( n % c.get() )  for every n : usize
```

**I3 is the one the family consumes and I1 is what makes it true.** The fold
from a sequence to a slot index is `seq & mask` rather than `seq % capacity`
because the bitwise form compiles to a single instruction and the modulo does
not. The two are equivalent only for power-of-two divisors, so I3 is a
precondition on every fold in the family — discharged once, at construction.

**The invariant is scoped to values that exist, not to inputs.** A slot count
of 100 does not violate I1; it simply never becomes a `Capacity`. This is the
distinction that makes the property enforceable at all: the type has no
inhabitants that break it.

**Seventeen crates depend on this; the lone non-zero hit below is not a re-check:**

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -rlE '\bCapacity\b' ring_*/src | command grep -v '^ring_types/' \
  | cut -d/ -f1 | sort -u | wc -l
command grep -rn 'is_power_of_two' ring_*/src | command grep -vc '^ring_types/'
```

Live output:

```
17
1
```

**The one non-zero hit is not a re-check.** It is `ring_align`'s
`CACHE_LINE.is_power_of_two()` compile-time assertion on the cache-line
constant — the same false positive TY34 (below) identifies and rules out.
Across the other sixteen consumers the count is genuinely zero: had any of
them felt the need to re-validate `Capacity` itself, the type would have
failed at its job.

### Enforcement Mechanism

**Three mechanisms, in decreasing order of strength:**

| # | Mechanism | Enforces | Strength |
|---|-----------|----------|----------|
| M1 | The field is private, and no method takes `&mut self` | I1 permanently, after construction | **Structural** — no program can violate it |
| M2 | `Capacity::new` rejects zero and non-powers-of-two | I1 at construction | Runtime, and the only path in |
| M3 | `Capacity` derives no `Default` | Removes the second, infallible constructor | Structural |

**M1 is the mechanism, and M2 is only its gatekeeper.** A validating
constructor on a struct with a public field guarantees nothing — the value can
be edited afterwards. What makes I1 hold for the whole lifetime of a value is
that `Capacity( usize )`'s field is private to this crate and nothing in this
crate mutates it:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'pub struct Capacity' ring_types/src/capacity.rs
command grep -c '&mut self' ring_types/src/capacity.rs
```

Live output:

```
pub struct Capacity( usize );
0
```

**No `pub` before the field, and no `&mut self` anywhere in the file.** Those
two facts together are the enforcement; everything else is commentary.

**M3 is easy to overlook and is load-bearing.** Every other type in this crate
has a `Default` — `Seq` and `SlotIndex` derive it, `WaitKind` and
`OverflowPolicy` declare one. If `Capacity` derived `Default` it would be
`Capacity( 0 )`, which violates I1 and I2 simultaneously and would be reachable
from safe code with no `Result` in sight. **The absent derive is a deliberate
omission that reads as an oversight**, which is why it is listed here as a
mechanism rather than left implicit.

**What does *not* enforce this invariant:**

| Non-mechanism | Why it does not help |
|---------------|----------------------|
| A debug assertion in `mask()` | Would fire in debug and vanish in release, exactly where the wrong offset does damage |
| A check at each of 17 consumers | Is what the type exists to avoid, and any one of them could be forgotten |
| A test | Tests sample; the invariant is universally quantified over a type's inhabitants |
| The `#[ must_use ]` on `mask()` | Prevents discarding the result, which is unrelated to the result being correct |

### Violation Consequences

**Both failure modes are silent, and that is the argument for enforcing this
structurally rather than by convention.**

| # | If violated by | Immediate effect | How it presents |
|---|----------------|------------------|-----------------|
| C1 | `get() == 0` reaching `mask()` | `0 - 1` underflows to `usize::MAX` | In release, no panic. Every sequence folds to an index far past any allocation — an out-of-bounds access or, worse, a wrapped one that lands in valid memory |
| C2 | `get()` not a power of two | `n & mask()` stops equalling `n % get()` | No error at all. Some slots become unreachable and others collide, so a ring silently loses or duplicates entries |
| C3 | The field becoming mutable | I1 holds at construction and not afterwards | The worst of the three — the constructor's evidence is still there, and it is no longer evidence of anything |

**C2 is the dangerous one because it degrades rather than crashes.** Take a
capacity of 6, mask 5 (`0b101`): sequences 0…7 fold to 0,1,0,1,4,5,4,5 — slots
2, 3, 6 and 7 are never addressed and four sequences collide in pairs. A ring
built this way runs, benchmarks, and reports plausible throughput while
overwriting live entries. **No assertion in the family would fire.**

**C1 is loud only by luck.** `usize::MAX` as an index usually faults, but a
sufficiently large allocation and a wrapping add can land it inside a valid
mapping. Debug builds catch the underflow; release builds do not, and release is
where this crate's benchmarks are run.

**C3 is the one a well-meaning refactor causes.** Making the field `pub` to
avoid writing an accessor, or adding a `set_capacity` for a resize feature,
retains `new`'s validation and destroys the invariant — the code still *looks*
validated. This is why M1 is listed above M2: the check is the visible part and
the immutability is the load-bearing part.

**Nothing detects any of the three.** There is no gate, no lint, and no test
that would catch a future edit adding a public field or a `Default` — the
enforcement is entirely in the shape of the declaration, and the only defence is
that the declaration is 79 lines long and this instance says what to look for.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_validating_a_slot_count_to_a_power_of_two.md](../algorithm/001_validating_a_slot_count_to_a_power_of_two.md) | M2 as a procedure — the two tests and why their order matters |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_vocabulary_surface.md](../api/001_the_vocabulary_surface.md) | `Capacity` among the exported names, and its seventeen consumers |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_two_position_types_and_the_fold_between_them.md](../data_structure/001_two_position_types_and_the_fold_between_them.md) | The fold I3 makes correct, and the two types either side of it |

### Invariants

| File | Relationship |
|------|--------------|
| [002_tier_zero_depends_on_nothing.md](002_tier_zero_depends_on_nothing.md) | The crate's other invariant — also enforced by a shape rather than a check, and also undetected if broken |

### Items

| File | Relationship |
|------|--------------|
| [../item/associated_function/001_capacity_new.md](../item/associated_function/001_capacity_new.md) | M2 |
| [../item/associated_function/003_capacity_mask.md](../item/associated_function/003_capacity_mask.md) | The unchecked subtraction I2 protects |
| [../item/struct/001_capacity.md](../item/struct/001_capacity.md) | M1 — the private field |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_a_slot_count_from_request_to_mask.md](../lifecycle/001_a_slot_count_from_request_to_mask.md) | Where in a value's arc each mechanism acts |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_a_newtype_that_makes_a_check_unnecessary.md](../pattern/002_a_newtype_that_makes_a_check_unnecessary.md) | The general form — and M1's precedence over M2 stated as the pattern's core |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/003_a_capacity_request_through_validation.md](../lifecycle/003_a_capacity_request_through_validation.md) | The states, and why there is no transition out of the valid one |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_capacity.md](../type/001_capacity.md) | I1–I3 as the type's V1–V4 validation rules |

### Sources

| File | Relationship |
|------|--------------|
| [`src/capacity.rs`](../../src/capacity.rs) | Line 23, M1's private field; 40–51, M2; and no `&mut self` in the file |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/types_test.rs`](../../tests/types_test.rs) | ✅ `capacity_accepts_powers_of_two` checks I2 and I3 across the legal range rather than at a single point, and `capacity_rejects_zero_and_non_powers_of_two` checks M2's two rejections. ❌ **M1 and M3 are unassertable** — a test that mutates the field or calls `Capacity::default()` does not compile, which is the enforcement. C3 is therefore the violation with no automated defence at all |
| [`tests/manual/readme.md`](../../tests/manual/readme.md) | The right home for M1 and M3 — compile-failure checks a human runs |

### TY34 — No Dependent Hand-Rolls a Power-of-Two Check

```sh
cd "$(git rev-parse --show-toplevel)"
grep -rnE 'is_power_of_two|count_ones\(\) *== *1' ring_*/src/*.rs | grep -v '^ring_types/' | wc -l
```

Live output:

```
1
```

One, not zero — but not a violation. The match is
`ring_align::CACHE_LINE.is_power_of_two()` (`src/lib.rs`), a compile-time
assertion on the cache-line-size constant, not a re-check of `Capacity`.
`ring_align`'s own comment already distinguishes the two: `Capacity::new`
asserts "the analogous precondition for its own number" — analogous, not
duplicated; `CACHE_LINE` needs the property for an unrelated reason
(`on_distinct_lines` divides by it to compute a line index). The regex matches
any power-of-two check, not specifically a re-check of `Capacity`, so it
cannot tell the two apart on its own.

Excluding that one false match, the count is genuinely zero across the other
thirty-one dependents. This remains the strongest positive result in this
crate's corpus and it is only visible family-wide: no single dependent can
demonstrate that it *did not* need to re-check, and the invariant's value is
exactly the sum of those non-events.

The measurement's weakness cuts both ways: it will keep reading zero if the
invariant is deleted, until something breaks, and it will flag an unrelated
invariant as if it were this one — either way, a human has to read the match,
not just the count.

### TY35 — The Invariant Is Established Once, Relied On at Thirty-One Sites

The ratio is the invariant's leverage: one validation carries thirty-one uses.
It used to be riskier — two calls established it, and one of them
(→ [`../algorithm/001`](../algorithm/001_validating_a_slot_count_to_a_power_of_two.md), TY20)
was an `expect` inside an infallible accessor, so a rounding backend could turn
a capacity read into a panic. TY20's fix removed that call: `ring_core::Ring::capacity`
now carries the `Capacity` its crossbeam variant was constructed from instead of
rebuilding one. `Capacity::new` has a single production call site left —
`ring_config/src/lib.rs:71` — so the invariant now has one point of establishment,
not two.
