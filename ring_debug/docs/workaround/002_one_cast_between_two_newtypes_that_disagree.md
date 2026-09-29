# Workaround: One Cast Between Two Newtypes That Disagree

### Scope

- **Purpose**: Record the `as u64` on line 303 as an absorbed constraint — two family newtypes with different widths and opposite field visibility, compared in one expression.
- **Responsibility**: What the cast routes around, what it costs, and what would delete it.
- **In Scope**: `capacity.get() as u64`; `Seq`'s public field against `Capacity`'s private one.
- **Out of Scope**: The ordering that makes the subtraction sound (→ [`pattern/001`](../pattern/001_the_guard_that_makes_the_next_line_legal.md)); what the comparison means (→ [`invariant/001`](../invariant/001_cursor_invariants_over_a_live_ring.md)).

### Constraint

The family's two relevant newtypes were designed independently and agree about
nothing:

| | `Seq` | `Capacity` |
|---|---|---|
| Declared in | `ring_types/src/id.rs` | `ring_types/src/capacity.rs` |
| Inner type | `u64` | `usize` |
| Field visibility | `pub` — read as `seq.0` | private — read as `capacity.get()` |
| Validated | No | Yes — rejects zero and non-powers-of-two |

D2 must compare a distance between two `Seq` against a `Capacity`. Every one of
those four rows has to be crossed to do it.

### The Workaround

One line, doing three conversions at once:

```rust
if producer.0 - consumer.0 > capacity.get() as u64
```

`.0` twice for the public fields, `.get()` for the private one, `as u64` for the
width. The comparison itself is the smallest part of the expression.

### Cost

| # | Cost | Detail |
|---|---|---|
| W5 | An unchecked numeric cast | `usize as u64` is widening on this target and on every 64-bit target, and identity-or-widening on all supported ones — but the compiler is not being asked to confirm that, and would not complain if it narrowed |
| W6 | The validation is discarded at the comparison | `Capacity` guarantees non-zero and power-of-two; the `u64` it becomes guarantees neither, and nothing downstream of the cast can recover it |
| W7 | Two access idioms in one expression | `.0` and `.get()` for the same *kind* of operation, which is a readability cost paid at every site in the family that compares the two |

**W5 is the one that would bite, and only on a 128-bit target**, where `usize as
u64` narrows silently. That is not a supported target for this family today, and
the cast is written as `as` rather than `try_into` because the alternative is an
error path for a condition that cannot occur on any platform the workspace builds
for. Recorded rather than hardened: the honest statement is "this is a cast we
have decided not to check", and a `TryFrom` here would trade a real impossibility
for a real unreachable branch.

### Deletion Condition

**`Seq` and `Capacity` agreeing on width, or a family-level comparison helper.**
Either removes the cast from this crate. Neither is this crate's to make — both
types belong to `ring_types`, which is on the export Contract, and changing
`Capacity`'s inner type from `usize` to `u64` would ripple through every
indexing site in the family.

The narrower fix, and the one this crate would actually ask for, is
`Capacity::get_u64()` — a single accessor that performs the widening once, in the
crate that owns the invariant, rather than at each of the sites that compare
against it.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '-- the two newtypes, as declared --'
command grep 'pub struct Seq' ring_types/src/id.rs
command grep 'pub struct Capacity' ring_types/src/capacity.rs
echo '-- the cast, and its whole expression --'
command grep 'as u64' ring_debug/src/lib.rs
echo '-- how often the family crosses the same boundary --'
printf 'sites casting a capacity to u64, family-wide: %s\n' \
  "$( command grep -rc 'capacity.get() as u64\|\.get() as u64' --include=*.rs ring_*/src | command grep -v ':0$' | wc -l )"
printf 'does Capacity offer a u64 accessor?           %s\n' \
  "$( command grep -c 'fn get_u64\|-> u64' ring_types/src/capacity.rs || true )"
echo '-- and DB18: is ring_debug named by any other crate at all? --'
# Cargo.toml (the workspace root) lists this crate as a member path, not
# another crate naming it — excluded for the same reason as pitfall/001 and
# integration/002.
printf 'other ring crates naming it in code or manifest: %s\n' \
  "$( command grep -rl 'ring_debug' --include=*.rs --include=Cargo.toml | command grep -v '^ring_debug/' | command grep -v '^Cargo\.toml$' | wc -l )"
```

Live output:

```
-- the two newtypes, as declared --
pub struct Seq( pub u64 );
pub struct Capacity( usize );
-- the cast, and its whole expression --
  if pending > capacity.get() as u64
-- how often the family crosses the same boundary --
sites casting a capacity to u64, family-wide: 3
does Capacity offer a u64 accessor?           0
-- and DB18: is ring_debug named by any other crate at all? --
other ring crates naming it in code or manifest: 2
```

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_the_guard_that_makes_the_next_line_legal.md](../pattern/001_the_guard_that_makes_the_next_line_legal.md) | The same line, read for its ordering rather than its conversions |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_four_variants_and_the_newtype_they_drop.md](../data_structure/002_four_variants_and_the_newtype_they_drop.md) | DB7 — the same newtype discarded again, on the way out |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | The cast |

### Tests

| Test | Relationship |
|------|--------------|
| `a_producer_exactly_a_lap_ahead_is_full_not_lapped` | The boundary the cast is on the wrong side of by one, asserted |
| `a_producer_more_than_a_lap_ahead_is_caught` | The other side of the same comparison |

### DB19 — one expression crosses four disagreements between two types from the same crate

`Seq` and `Capacity` are declared in the same crate, in two files, and share no
convention: different widths, opposite field visibility, one validated and one
not. Line 303 crosses all of it in a single comparison.

Nothing here is wrong and every individual choice is defensible — `Seq` is a
public-field newtype because it is arithmetic and wants to be transparent;
`Capacity` is opaque because it has an invariant to protect. **The observation is
that the two conventions meet at a comparison neither type anticipated**, and the
meeting point is a diagnostic crate rather than either owner.

`ring_types` offers no `get_u64`, so every site in the family comparing a distance
against a capacity writes the widening itself. The number of such sites is small
today, which is precisely when a helper is cheap to add and nobody does.

### DB20 — the crate that checks the family's arithmetic contains the family's least checked arithmetic

This crate exists because derived readings can lie
([`pitfall/001`](../pitfall/001_saturating_arithmetic_reports_health.md)), and when
this finding was raised its own comparison rested on three unchecked things: an
unsigned subtraction kept sound by statement order (DB9), an unchecked width cast
(W5), and a discarded validation (W6) — none of which the compiler was asked to
confirm. DB9's leg is closed: `check_seqs` now reads
`producer.0.checked_sub( consumer.0 )` with the `None` branch reporting
`ConsumerAheadOfProducer`, so that one is a compiler-confirmed case rather than an
ordering convention. Two remain.

Each was individually justified and together they were a pattern worth naming:
**the checker is written in exactly the idiom it was built to catch.** It was not
a defect — a checker cannot check itself without regress, and every one of these
choices bought something real. It is the reason this crate's own arithmetic
deserves a document rather than a comment, and the reason DB9's positional
soundness was recorded as a hazard rather than filed as a style note: recorded
that way, it was fixable, and it was fixed.

