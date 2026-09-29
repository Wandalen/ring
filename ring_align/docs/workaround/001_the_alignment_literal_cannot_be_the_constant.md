# Workaround: The Alignment Literal Cannot Be the Constant

### Scope

- **Purpose**: Record the language constraint that forces this crate to write its own number twice — `#[ repr( align( … ) ) ]` accepts only a literal — with the cost it imposes and the condition under which the workaround can be deleted.
- **Responsibility**: State the constraint, prove it, give the workaround as written, price it, and name the deletion condition.
- **In Scope**: The `repr` attribute's argument.
- **Out of Scope**: The constant's value, which is [`decisions/001`](../decisions/001_the_constant_is_not_conditional.md); the family-wide ownership claim this constraint dents, which is [`invariant/002`](../invariant/002_one_constant_for_the_whole_family.md).

### The Constraint

`#[ repr( align( N ) ) ]` requires `N` to be a literal integer. A `const` item
of the right type and value is rejected. Verified on
`rustc 1.97.1 (8bab26f4f 2026-07-14)`:

```sh
cat > ./-probe.rs <<'EOF'
pub const CACHE_LINE : usize = 64;
#[ repr( align( CACHE_LINE ) ) ]
pub struct Probe( u64 );
fn main() {}
EOF
rustc --crate-name probe --edition 2021 -o /dev/null ./-probe.rs
# error[E0693]: `align` expects a literal integer as argument
rm -f ./-probe.rs
```

Live output:

```
error[E0693]: incorrect `repr(align)` attribute format: `align` expects a literal integer as argument
 --> ./-probe.rs:2:17
  |
2 | #[ repr( align( CACHE_LINE ) ) ]
  |                 ^^^^^^^^^^

error: aborting due to 1 previous error

For more information about this error, try `rustc --explain E0693`.
```

This is not a `const fn` restriction or a const-evaluation limit — the value is
known and correct. Attribute arguments are parsed before name resolution, so no
identifier is admissible there regardless of what it resolves to.

### The Workaround

```rust
// ring_align/src/lib.rs:36
pub const CACHE_LINE : usize = 64;

// ring_align/src/lib.rs:68
#[ repr( align( 64 ) ) ]
pub struct CacheAligned< T >( T );
```

**The number is written twice, in two separate places, in the crate whose
reason for existing is that the number is written once.** There is no way to
express the dependency between them in the language.

### The Cost

| # | Cost | Severity |
|---|------|----------|
| W1 | A port must edit two sites, not one, and only one of them is named `CACHE_LINE` | Real. It is step 2 of [`lifecycle/002`](../lifecycle/002_the_constant_across_a_platform_port.md) and the reason that lifecycle has a "must move together" note |
| W2 | The crate's own single-ownership claim is weakened at its own declaration | Rhetorical, and worth stating plainly rather than eliding — [`invariant/002`](../invariant/002_one_constant_for_the_whole_family.md) reserves one declaration and the code contains two numbers |
| W3 | A generic `Aligned< T, const N : usize >` is not expressible | Structural. It is why this crate has one wrapper at one alignment rather than a parameterised family (→ [`pattern/001`](../pattern/001_the_newtype_as_layout_carrier.md) K3) |

**W1 is mitigated and W2 is not.** A divergence between the two sites is caught
immediately — `align_test.rs` asserts `align_of::< CacheAligned< u8 > >()`
against `CACHE_LINE`, so raising one without the other fails the suite. Nothing
mitigates W2, because there is nothing to detect: the code is correct as
written and the claim is simply narrower than it sounds.

### Alternatives Considered

| # | Alternative | Why not |
|---|-------------|---------|
| V1 | A macro generating the struct, taking the alignment as a token | Works, and buys one textual source for the number. Costs a macro in a crate whose entire surface is three items, and the macro's own definition still contains a literal somewhere unless the caller supplies it — which relocates the duplication rather than removing it |
| V2 | `build.rs` emitting the struct with the value substituted | Works and is worse: a generated type that an IDE cannot see, for one line of source |
| V3 | Accept the two sites and assert they agree | **Chosen.** The assertion is one line and already exists |

**V1 deserves the honest note that it is not obviously wrong** — a
`declare_cache_aligned!( 64 )` macro would make the literal appear once. It
loses because the assertion in V3 gives the same protection against divergence
at a fraction of the complexity, and divergence is the only failure mode either
one prevents.

### Deletion Condition

Delete this workaround when `#[ repr( align( … ) ) ]` accepts a constant
expression. Re-check by running the probe above: if it compiles, replace the
literal at `src/lib.rs:68` with `CACHE_LINE`, delete this file, and remove W1's
"must move together" note from
[`lifecycle/002`](../lifecycle/002_the_constant_across_a_platform_port.md).

**Do not assume this has happened because a release note mentions const
generics or `const` in attributes.** The probe is three lines and it is the
only reliable check.

### AL49 — The Deletion Condition Is Decidable in Three Lines and Nothing Runs It

Both of this crate's workarounds end with a deletion condition phrased as a
compiler question — does `#[ repr( align( CACHE_LINE ) ) ]` still produce
`E0693`, does destructuring still produce `E0493` — and each is answered by a
three-line probe that lives in a document.

**Finding.** Nothing executes either one. The day the language lifts the
restriction, the crate keeps the workaround, keeps the duplicated literal, and
nobody is told; the condition is written down and unwatched. That is the same
shape as [`decisions/002`](../decisions/002_the_predicate_takes_integers.md)'s
AL16 — a stated reversal trigger with no mechanism behind it — and it is worth
naming as a class rather than twice as a coincidence.

---

### AL51 — This Workaround's Cost Is Smaller Than the Prose Above Claims

W1 calls the duplicated literal "a second edit site on every platform port".
True, and incomplete: `tests/align_test.rs:55` compares
`size_of::< CacheAligned< … > >()` against `CACHE_LINE`, so the two sites cannot
ship out of step. Raise the constant without raising the attribute and the suite
fails on the next run.

**Finding.** The residual cost is therefore *remembering a second edit*, not
*risking a silent divergence* — a materially smaller claim than the table
conveys, and one the table should carry, because a reader deciding whether V1's
macro is worth building is weighing exactly this difference.

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_the_constant_is_not_conditional.md](../decisions/001_the_constant_is_not_conditional.md) | A4 — this constraint is also what rules out a runtime-determined line size |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_one_constant_for_the_whole_family.md](../invariant/002_one_constant_for_the_whole_family.md) | Q5 and W2 — the ownership claim this constraint qualifies |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_cache_aligned_and_its_associated_functions.md](../item/001_cache_aligned_and_its_associated_functions.md) | The attribute as declared, in its attribute table |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_the_constant_across_a_platform_port.md](../lifecycle/002_the_constant_across_a_platform_port.md) | Sites 1 and 2 — W1's cost, priced in edit steps |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_the_newtype_as_layout_carrier.md](../pattern/001_the_newtype_as_layout_carrier.md) | K3 — W3 stated as the pattern's ceiling |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_cache_line.md](../type/001_cache_line.md) | The declaration this constraint prevents the attribute from referring to |

### Workarounds

| File | Relationship |
|------|--------------|
| [002_into_inner_cannot_be_const.md](002_into_inner_cannot_be_const.md) | The crate's other absorbed language constraint |

### Sources

| File | Relationship |
|------|--------------|
| `ring_align/src/lib.rs:36,68` | The two sites |

### Tests

| File | Relationship |
|------|--------------|
| `tests/align_test.rs` | `a_wrapped_value_occupies_exactly_one_line` — the V3 assertion that keeps the two sites in agreement |
