# Pitfall: `size_of` Proves Nothing About Addresses

### Scope

- **Purpose**: Record why the crate's structural assertions are not the property the crate exists to deliver, and why a negative control is what separates the two.
- **Responsibility**: State the gap between the claims, the test that would pass while the property failed, the mitigation, and the doc-example variant of the same trap.
- **In Scope**: `size_of`/`align_of` assertions versus real-address assertions; the negative control; the stack-locals doc-example trap.
- **Out of Scope**: Whether 64 is the right number, which is [`pitfall/001`](001_a_constant_too_small_buys_nothing.md); the layout rule itself, which is [`algorithm/002`](../algorithm/002_rounding_a_payload_up_to_whole_lines.md).

### The Trap

These are two different claims, and only the second is what
this crate exists to deliver:

| # | Claim | Asserted by |
|---|-------|-------------|
| A | `CacheAligned< T >` is 64 bytes and 64-aligned | `size_of` / `align_of` |
| B | Two `CacheAligned` fields in one struct sit on different cache lines | Reading their real addresses |

**A is evidence for B, not proof of it.** A describes one type in isolation; B
is a statement about a layout the compiler produced for a struct containing
two of them. Field ordering, niche optimisation, and future `repr` behaviour
all sit between them.

A suite that asserts only A is the trap: it is easy to write, it looks
thorough, it is all `const`-evaluable, and it would keep passing if the
compiler laid the two fields out on one line.

### What the Suite Does Instead

`two_wrapped_fields_land_on_different_lines` builds the real struct and reads
the real addresses:

```rust
struct TwoCursors
{
  producer : CacheAligned< u64 >,
  consumer : CacheAligned< u64 >,
}

let a = core::ptr::from_ref( &pair.producer ) as usize;
let b = core::ptr::from_ref( &pair.consumer ) as usize;
assert!( a.abs_diff( b ) >= CACHE_LINE );
assert!( on_distinct_lines( a, b ) );
```

**Both assertions, deliberately** — the distance form because it demands a full
line of separation, the membership form because it is the exported question
(→ [`algorithm/001`](../algorithm/001_deciding_line_membership_by_division.md) § Why Both Forms Appear).

### The Negative Control Is the Load-Bearing Part

`two_unwrapped_fields_share_a_line` builds the same struct without the wrapper
and asserts the fields **do** share a line:

```rust
struct Naive { producer : u64, consumer : u64 }
assert!( a.abs_diff( b ) < CACHE_LINE );
assert!( !on_distinct_lines( a, b ), "the unpadded pair should share a line" );
```

Without it, the positive test is uninterpretable. Two `u64` fields *might* have
landed on different lines for reasons having nothing to do with this crate —
allocation alignment, stack layout, a struct that straddles a boundary by
chance. The control establishes that the unpadded arrangement really does
contend on this machine, which is what makes the padded assertion a measurement
of the padding rather than of the environment.

**This is the general rule the instance exists to state:** a test asserting
that a mechanism produced an effect is worth what its control is worth. The
control here is one test, eleven lines, and it carries more of the suite's
meaning than any other single test in the crate.

### The Same Trap in a Doc Example

The identical mistake is available in `on_distinct_lines`'s own documentation,
and it was made once:

```rust
// wrong — two adjacent stack locals share a line, so this asserts
// something false about the machine while looking reasonable
let a = 0u64;
let b = 0u64;
assert!( on_distinct_lines( &a as *const _ as usize, &b as *const _ as usize ) );
```

The example now uses plain integers — `(0, 63)`, `(63, 64)`, `(128, 130)` —
which say exactly what the predicate does with no dependence on where the
compiler put anything. The manual plan's M4 exists to keep it that way, and its
run record names this as "a real failure earlier in the stage and… the reason
the check is written the way it is".

**Doc examples are tests**, so a doc example resting on incidental layout is a
test that passes for the wrong reason — the same defect as A-without-B, in the
place a reader meets the API first.

### Mitigation

| # | Mitigation | Strength |
|---|-----------|----------|
| N1 | Assert on real addresses, not `size_of` alone | Real — it tests the actual claim |
| N2 | A negative control asserting the unpadded pair contends | Real, and it is what makes N1 interpretable |
| N3 | Doc examples over integer literals, never stack locals | Real, enforced by `cargo test --doc` only if the example is written correctly in the first place |
| N4 | `tests/manual/readme.md` M3 runs N1 and N2 together and names N2 as the one that matters | Documentation of the discipline |
| N5 | A check that no doc example in the crate takes an address of a local | **Does not exist** — M4 is a human reading the example |

**N2 is the only mechanism that turns N1 from an assertion into evidence.** The
others are discipline.

### AL43 — Five Assertions Decide by Size and Four by Address

```
    size_of 5, from_ref 4
```

Only the second kind can observe the property the crate exists for. A `size_of`
assertion says the type is 64 bytes; it says nothing about where two of them
land.

**Finding.** The split is close to even, which is the healthy shape for a crate
whose claim is structural, and the test file says so itself at
`tests/align_test.rs:48` — "real addresses, not on `size_of` alone". Recording
it as a finding is not a criticism: it is the measurement that would show the
balance shifting if someone later added size assertions instead of address ones,
which is the cheaper thing to write.

---

### AL44 — The Negative Control Is What Makes the Positive One Mean Anything

```
75:fn two_unwrapped_fields_share_a_line()
```

Its partner asserts that two *wrapped* fields do not share a line. This one
asserts that two *unwrapped* fields do.

**Finding.** Without the negative, a broken address computation that returned
"different lines" for everything would pass the positive test and look like
proof. With it, such a break has to fail in two opposite directions
simultaneously to stay hidden. That is the same reasoning the corpus applies to
its own empty-result recipes — a zero needs a non-zero beside it — and it is
here in the test suite, arrived at independently.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_deciding_line_membership_by_division.md](../algorithm/001_deciding_line_membership_by_division.md) | Why the test asserts both the distance and membership forms |
| [../algorithm/002_rounding_a_payload_up_to_whole_lines.md](../algorithm/002_rounding_a_payload_up_to_whole_lines.md) | Claim A — the structural fact this instance says is insufficient alone |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_reading_surface.md](../api/001_the_reading_surface.md) | The predicate whose doc example carried this trap |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_two_wrapped_fields_never_share_a_line.md](../invariant/001_two_wrapped_fields_never_share_a_line.md) | Claim B — the property, stated as a standing restriction |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_the_structural_claim_is_testable.md](../non_functional_requirement/001_the_structural_claim_is_testable.md) | The third claim — throughput — which has no test at all, control or otherwise |

### Pitfalls

| File | Relationship |
|------|--------------|
| [001_a_constant_too_small_buys_nothing.md](001_a_constant_too_small_buys_nothing.md) | The trap that survives even a correct negative control, because it miscalibrates both arms |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_cache_aligned.md](../type/002_cache_aligned.md) | The wrapper whose effect is being measured |

### Sources

| File | Relationship |
|------|--------------|
| [`../invariant/001_two_wrapped_fields_never_share_a_line.md`](../invariant/001_two_wrapped_fields_never_share_a_line.md) | Requires a cursor that "occupies a cache line by itself" — a claim about placement, which is B |

### Tests

| File | Relationship |
|------|--------------|
| `tests/align_test.rs` | `two_wrapped_fields_land_on_different_lines` is N1; `two_unwrapped_fields_share_a_line` is N2 |
| `tests/manual/readme.md` | M3 runs both and names the control as the one that matters; M4 covers the doc-example variant |
