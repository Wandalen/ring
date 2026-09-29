# Invariant: Every Error Renders Distinctly

### Scope

- **Purpose**: State that no two `RingError` variants produce the same `Display` string, and record that the mechanism holding it is a coverage gate rather than the compiler — with one violation already on record.
- **Responsibility**: State the invariant, its enforcement mechanism, and the consequences of violation.
- **In Scope**: The nine `Display` arms; the hand-maintained roster that exercises them; why `#[ non_exhaustive ]` removes the compiler from the loop.
- **Out of Scope**: What each variant means (→ [`../type/002`](../type/002_ring_error.md)); the two classifier predicates over the same enum (→ [`../algorithm/002`](../algorithm/002_classifying_an_error_into_configuration_or_traffic.md)); whether the strings allocate (→ [`../non_functional_requirement/001`](../non_functional_requirement/001_errors_and_positions_do_not_allocate.md)).

### Invariant Statement

**For every pair of distinct `RingError` values, `to_string()` returns distinct
non-empty strings; and every variant carrying a payload interpolates it.**

Two clauses, and the second is the one that earns the first its value. Nine
distinct strings would be satisfiable by numbering them; what a caller needs is
that `CapacityNotPowerOfTwo( 6 )` says `6` and `BatchTooLarge { requested : 9,
capacity : 8 }` says both `9` and `8`. Without the second clause the invariant is
about the enum; with it, the invariant is about the message a human reads.

The audience is not this family. **No crate in the workspace formats a
`RingError` in production** — the invariant exists for a consumer behind the
export Contract (→ [`../api/001`](../api/001_the_vocabulary_surface.md)), which
is exactly the caller who cannot debug from a `Debug` derive because they never
see the source.

### Enforcement Mechanism

**Three mechanisms of decreasing strength, and the strong one only covers half
the invariant.**

**(1) The `match` in `Display::fmt` is wildcard-free** (`src/error.rs:166`–`:180`).
Adding a variant to `RingError` is a compile error *in this crate* until an arm
exists for it. `#[ non_exhaustive ]` does not weaken this — the attribute
constrains matches in *other* crates, not the defining one. So "every variant has
an arm" is compiler-enforced.

That is the whole of what the compiler gives. It says nothing about the arms
being *distinct*, and nothing about payloads being interpolated: nine arms all
writing `"error"` compile fine.

**(2) `every_error_displays_distinctly` asserts both clauses** — a hand-written
roster of all nine variants, an `!is_empty()` check, a `!rendered.contains()`
check, `assert_eq!( rendered.len(), 9 )`, and the two payload assertions:

```sh
cd "$(git rev-parse --show-toplevel)"
awk '/^\/\/\/ Every variant renders a distinct, non-empty message, so a log line$/{ n1 = NR } n1 && NR >= n1 && NR <= n1 + 23 { print }' ring_types/tests/types_test.rs
```

Live output:

```
/// Every variant renders a distinct, non-empty message, so a log line
/// identifies which one occurred without the `Debug` form.
///
/// **The roster below is maintained by hand, and nothing here can tell you it
/// is short.** `RingError` is `#[ non_exhaustive ]`, so a `match` written in
/// this file — a separate crate — needs a wildcard arm and cannot be made to
/// fail the build when a variant is added. What actually catches an omission is
/// gate G1's 100% line-coverage threshold: an unrostered variant leaves its
/// `Display` arm unexecuted, and the gate names the file and the fraction.
///
/// That is not hypothetical. `PolicyUnsupported` was added to
/// `ring_types` while implementing `ring_core`, this list was not updated, and
/// G1 reported `ring_types/src/error.rs 16/17` on the next run. Detection took
/// one gate run rather than a compiler error — slower, but not silent, which is
/// the property that matters.
#[ test ]
fn every_error_displays_distinctly()
{
  let all =
  [
    RingError::CapacityZero,
    RingError::CapacityNotPowerOfTwo( 6 ),
    RingError::Full,
    RingError::Empty,
```

**The roster is maintained by hand and nothing in the test can tell you it is
short.** The test lives in `tests/`, a separate crate, so its own `match`-free
array construction has no exhaustiveness obligation at all — a variant simply
absent from the array is not an error, it is a shorter loop.

**(3) Gate G1's 100% line-coverage threshold is what actually catches an
omission.** An unrostered variant leaves its `write!` line unexecuted, and the
gate reports the file and the fraction:

```bash
cd "$(git rev-parse --show-toplevel)"
bench_harness/gate/g1_coverage.sh
```

**This is not hypothetical.** `PolicyUnsupported` was added to `ring_types`
during `ring_core`'s implementation, the roster was not updated, and G1 reported
`ring_types/src/error.rs 16/17` on the next run. Detection took one gate run
rather than a compiler error — slower, and not silent, which is the property that
matters.

**Note what mechanism (3) does and does not check.** Coverage proves the arm
*ran*. It does not prove the arm's string is distinct from its neighbours', and
it does not prove a payload was interpolated. Those remain on mechanism (2)'s
assertions, which are only as complete as the roster coverage forces you to
extend. The three mechanisms interlock rather than overlap: G1 forces the roster
to be complete, and the roster's assertions then check distinctness. **Remove G1
and clause one degrades to "every variant an author remembered."**

### Violation Consequences

**A duplicate string is a support cost, not a crash**, and that is what makes it
worth an invariant rather than a test. Nothing fails. A consumer reports "ring is
full" and the maintainer cannot tell whether the ring was full or a batch
exceeded capacity, because both rendered the same. The failure surfaces weeks
later, in someone else's issue tracker, as a question that cannot be answered
from the message.

**A missing payload is worse and more likely.** `ring capacity is not a power of
two` without the offending number tells the caller only what they already knew
from the error's name. The two payload-carrying arms are the two most useful
messages in the enum and the two easiest to write uselessly, which is why they
get their own assertions rather than relying on the distinctness loop — two
different non-interpolating messages are still distinct from each other.

**The realistic violation is not editing an arm — it is adding a variant.** The
compiler stops the arm from being missing; nothing stops a new arm from being
copy-pasted from its neighbour, and nothing stops the roster from lagging by one.
The `16/17` incident is exactly this sequence caught one step early, and the
reason it was caught is a gate the crate does not own.

**The residual gap, stated precisely.** If a variant is added, its arm
copy-pasted verbatim from a sibling, and the roster updated in the same change,
then G1 passes at 17/17, the distinctness loop fails on the duplicate, and the
invariant holds. If the arm is copy-pasted and given a trivially different
string — `"ring is closed."` beside `"ring is closed"` — every mechanism passes
and the invariant is violated in substance while satisfied in letter. Nothing
here detects that, and nothing cheap would.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_classifying_an_error_into_configuration_or_traffic.md](../algorithm/002_classifying_an_error_into_configuration_or_traffic.md) | The other pair of operations over the same nine variants — and the one whose incompleteness *is* silent |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_vocabulary_surface.md](../api/001_the_vocabulary_surface.md) | The export Contract that makes this invariant's audience external |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_error_enum_as_a_closed_copy_set.md](../data_structure/002_the_error_enum_as_a_closed_copy_set.md) | The nine variants and their payloads |

### Invariants

| File | Relationship |
|------|--------------|
| [001_every_capacity_has_a_valid_mask.md](001_every_capacity_has_a_valid_mask.md) | The contrast: an invariant the type system holds outright, with no gate in the loop |
| [002_tier_zero_depends_on_nothing.md](002_tier_zero_depends_on_nothing.md) | The other extreme: an invariant with no mechanism at all |

### Items

| File | Relationship |
|------|--------------|
| [../item/associated_function/006_display_fmt_for_ring_error.md](../item/associated_function/006_display_fmt_for_ring_error.md) | The nine arms, line by line |
| [../item/implementation/003_impl_display_for_ring_error.md](../item/implementation/003_impl_display_for_ring_error.md) | The `impl` block |
| [../item/enum/002_ring_error.md](../item/enum/002_ring_error.md) | The `#[ non_exhaustive ]` attribute that removes the compiler from the test's side |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_an_error_from_construction_to_display.md](../lifecycle/002_an_error_from_construction_to_display.md) | Where rendering sits in an error's life |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_the_enum_sets_are_closed_and_asserted.md](../non_functional_requirement/002_the_enum_sets_are_closed_and_asserted.md) | The same hand-maintained-roster weakness, on the three `ALL` arrays instead |

### Sources

| File | Relationship |
|------|--------------|
| [`src/error.rs`](../../src/error.rs) | Lines 166–180, the wildcard-free match; 168–179, the nine arms |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/types_test.rs`](../../tests/types_test.rs) | ✅ `every_error_displays_distinctly` asserts non-emptiness, pairwise distinctness, roster length 9, and both payload interpolations. ⚠️ The roster is hand-maintained and the test crate cannot enforce its completeness — that job belongs to gate G1, which is outside this crate |

### TY38 — Distinctness Is Asserted and Wording Is Not

The test asserts that the nine messages differ from one another. It does not
assert what any of them says. Swap the `Full` and `Empty` arms — so a full ring
reports "ring is empty" — and every assertion in the suite still passes: the
strings are still nine, still non-empty, still pairwise distinct, and the two
data-carrying variants still embed their data.

**The invariant this instance documents is real and the test is weaker than the
invariant.** A wording assertion for the seven payload-free variants is seven
lines and would close it; the two data-carrying variants are already covered,
because their assertions test content rather than identity.
