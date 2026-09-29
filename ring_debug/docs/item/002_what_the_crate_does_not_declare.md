# Item: What the Crate Does Not Declare

### Scope

- **Purpose**: Catalogue the declarations this crate *omits*, and record which omissions are decisions and which are exposures.
- **Responsibility**: The absent attributes, the absent trait methods, and what each absence costs a caller.
- **In Scope**: `#[ non_exhaustive ]`, `#[ inline ]`, `#[ cfg ]`, `unsafe`, `fn source`, `impl Drop`.
- **Out of Scope**: The declarations that are present (→ [`item/001`](001_three_nouns_five_verbs_and_the_enum_two_doors_cannot_reach.md)); the runtime cost of the absent `#[ inline ]` (→ [`non_functional_requirement/001`](../non_functional_requirement/001_absent_unless_called.md)).

### Abstract

A catalogue of what is present says what the crate does. A catalogue of what is
absent says what a caller may assume — and two of the six absences below are
load-bearing in a way no individual declaration reveals, because an absence has
no rustdoc page to be documented on.

Six omissions, measured. Four are correct and one of the four is the crate's best
single design decision. Two are exposures.

### Data Structures

| # | Absent | Count in `src/lib.rs` | Verdict |
|---|---|---:|---|
| 1 | `unsafe` | 0 | **Correct, and structural.** Every read is a `SeqCell::load`; there is nothing to be unsafe about |
| 2 | `#[ cfg( ... ) ]` | 0 | **Correct.** The crate compiles identically under every feature and every target; see `non_functional_requirement/001` |
| 3 | `#[ inline ]` | 0 | **Correct by deferral.** A diagnostic that is not on a hot path does not need the hint, and adding one would be a claim about a cost nothing measured |
| 4 | `impl Drop` | 0 | **Correct.** `Watch` owns three `Copy` scalars and no resource |
| 5 | `fn source` | 0 | **Exposure — DB3.** `impl core::error::Error for Violation {}` is empty-bodied |
| 6 | `#[ non_exhaustive ]` | 0 | **Exposure — DB4.** `Violation` is a public four-variant enum with no future-proofing |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
S=ring_debug/src/lib.rs
while read -r label pattern ; do
  printf '%-18s %s\n' "$label" "$( command grep -cE -- "$pattern" $S || true )"
done << 'EOF'
unsafe           unsafe
#[cfg]           ^\s*#\[ cfg
#[inline]        ^\s*#\[ inline
impl_Drop        ^impl Drop
fn_source        fn source\(
#[non_exhaustive] ^\s*#\[ non_exhaustive
EOF
echo '-- the whole Error impl --'
command grep 'impl core::error::Error' $S
echo '-- and the family-wide picture: empty Error impls, and non_exhaustive types --'
printf 'ring crates with an Error impl:   %s\n' "$( command grep -rlE '^impl (core::)?error::Error' --include=*.rs ring_*/src | wc -l )"
printf 'of those, any with a fn source:   %s\n' "$( command grep -rl 'fn source(' --include=*.rs ring_*/src | wc -l )"
echo '-- non_exhaustive: the attribute, and the unrelated Debug call it is a substring of --'
command grep -r 'non_exhaustive' --include=*.rs ring_*/src \
  | command grep -vE ': *//' | sed 's|ring/||; s|\.rs:|.rs:  |'
```

Live output:

```
unsafe             0
#[cfg]             0
#[inline]          0
impl_Drop          0
fn_source          0
#[non_exhaustive]  0
-- the whole Error impl --
impl core::error::Error for Violation {}
-- and the family-wide picture: empty Error impls, and non_exhaustive types --
ring crates with an Error impl:   7
of those, any with a fn source:   0
-- non_exhaustive: the attribute, and the unrelated Debug call it is a substring of --
ring_spsc/src/lib.rs:        .finish_non_exhaustive()
ring_testkit/src/lib.rs:  #[ non_exhaustive ]
ring_types/src/error.rs:  #[ non_exhaustive ]
```

### Types

| File | Relationship |
|------|--------------|
| [../type/001_violation.md](../type/001_violation.md) | The enum DB4 is about |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_absent_unless_called.md](../non_functional_requirement/001_absent_unless_called.md) | Why absence 2 and absence 3 are the same decision seen from two sides |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | The six absences |

### Tests

| Test | Relationship |
|------|--------------|
| `a_violation_propagates_as_an_error` | DB3 — the `Error` impl is exercised, and the test cannot observe that `source` is absent |
| `a_violation_reports_the_numbers_it_was_derived_from` | DB3 — the numbers a `source()` would otherwise have been asked for are already in the variant |

### DB3 — the `Error` impl is empty and the crate is one of eight


`impl core::error::Error for Violation {}` — the entire impl, on line 245. No
`source`, so a `Violation` propagated through `?` into a caller's own error type
terminates the chain: `err.source()` returns `None`, and a tool printing the full
causal chain prints one line.

Here that is defensible, and the reason is specific rather than general. A
`Violation` **has** no source. It is not a wrapper around a failed operation; it
is the bottom of a diagnosis — the numbers themselves, read off two atomic loads,
with nothing underneath them. `ConsumerAheadOfProducer { producer, consumer }`
already carries everything a caller could ask `source()` for.

**The finding is not that this crate is wrong. It is that the family cannot tell
the difference.** Seven ring crates carry an `Error` impl, this one included, and
all seven are empty-bodied — zero `fn source` across the family. This one is empty
for a reason that was worked out; the other six have not been asked. A convention
that is unanimous is a convention that carries no information, and a reader who
finds `{}` here learns nothing about whether it was considered.

### DB4 — `Violation` is not `#[ non_exhaustive ]` and a fifth defect is a breaking change


`Violation` has four variants and is matched exhaustively in this crate's own test
suite and, by construction, in every downstream tool that wants to react
differently to D1 than to D3. Without `#[ non_exhaustive ]`, adding a fifth
variant is a major-version break for all of them.

That matters more here than it would for an ordinary error type, because **this
crate's whole subject is defects that were not anticipated.** The four variants
are the four failures somebody thought of; the crate exists on the premise that a
ring can be broken in ways its own code does not check for. A defect enum that is
closed to extension is the one shape most at odds with what the crate is for.

The counter-argument is real and is why this is recorded rather than fixed:
`#[ non_exhaustive ]` forces a `_ =>` arm on every downstream match, which turns
"I handled all four" into "I handled some and shrugged at the rest" — and a
diagnostic tool that shrugs at a violation is worse than one that fails to
compile when a new violation appears. The family has exactly one
`#[ non_exhaustive ]` type today (`ring_types::RingError`), so adopting it here
would also make this the second, against a family-wide convention that has never
been stated.

**Both readings are defensible and neither had been chosen deliberately.** That
was the finding: the enum was closed by default, not by decision. What it asked
for was a ruling, and the ruling is below.

**The closed enum wins, on the strength of what this crate is for.** A diagnostic
that fails to compile when a new violation appears is a diagnostic whose callers
are forced to look at it; one that compiles and falls into a `_ =>` arm is a
diagnostic whose callers are told, by the type system, that ignoring the new case
is fine. The premise the counter-argument rests on — that a fifth variant is
likely — is the same premise that makes the shrug expensive, because the fifth
variant is by construction the one nobody anticipated and therefore the one least
safe to route into a default arm.

The major-version break is real and is accepted as the price. It is also
bounded in a way the alternative is not: a break happens once, at a version
boundary, with a compiler pointing at every site that needs a decision. A `_ =>`
arm is silent every time, forever.

**Disposition:** declined — `Violation` stays closed, deliberately. A
`#[ non_exhaustive ]` here would force a `_ =>` arm on every consumer that today
matches all four variants exhaustively, and in a crate whose whole subject is
unanticipated defects that arm is the one place a new defect would be routed to
silence; the family's single precedent, `ring_types::RingError`, is a public
error type crossing a crate boundary for callers who cannot act on its variants
individually, which is the opposite situation. The break a fifth variant would
cause is accepted, and this paragraph is the record that it was chosen rather
than inherited.
