# workaround

External constraints `ring_tls` absorbs.

### Scope

- **Purpose**: Record every external constraint this crate compensates for, so each one carries a cost and a deletion condition.
- **Responsibility**: Document this crate's workarounds, or record explicitly that it has none.
- **In Scope**: Constraints originating outside this repository — for a crate whose only dependencies are workspace siblings, that means the language, the toolchain, and the targets.
- **Out of Scope**: This crate's own design decisions, which are not workarounds however unusual they look (→ [`../decisions/readme.md`](../decisions/readme.md)).

### Overview

**Two, and both are the language's.**

The earlier text here read "**None.**", on the argument that a crate depending
only on workspace siblings has nothing external to compensate for. The Scope
line above already contradicts that: the language and the toolchain are named
as in scope, and this crate absorbs a constraint from each. "No published
dependency" answers a question about *crates*; a workaround answers a question
about *constraints*, and the two sets are not the same.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_tls
printf 'sibling path dependencies:        %s\n' "$( grep -c 'path = "\.\./ring_' Cargo.toml )"
printf 'published dependencies:           %s\n' "$( grep -cE '^[a-z_]+ = "' Cargo.toml )"
printf 'std types in a public signature:  %s\n' "$( grep -cE '^  pub (const )?fn .*std::' src/lib.rs )"
printf 'unsafe blocks or fns:             %s\n' "$( grep -vE '^\s*(//|///|//!)' src/lib.rs | grep -c 'unsafe' )"
```

Every count above is a `grep -c` inside a command substitution rather than a
bare pipeline, because two of the four are legitimately zero and `grep` exits 1
when it matches nothing — a block whose last command is a zero-count `grep`
fails the recipe gate for being correct.

Six sibling dependencies, no published crate, no `unsafe` — and one `std` type
in a public signature, which is the whole of the family's `std` leakage.

The line about `Cargo.toml` being "currently empty" was written against a
skeleton and never revisited; the manifest carries three dependencies and three
dev-dependencies, which is what the first count above reports.

### Instances

| ID | Name | Records |
|----|------|---------|
| [001](001_the_only_std_type_in_a_public_signature.md) | The Only `std` Type in a Public Signature | `Drain` in `drain`'s return type, and what it pins |
| [002](002_a_unit_error_variant_cannot_carry_the_refused_item.md) | A Unit Error Variant Cannot Carry the Refused Item | The constraint behind `push`'s contradiction, and its cost |

### Sources

| File | Relationship |
|------|-----------------|
| `Cargo.toml` | The dependency surface examined for this finding — three dependencies and three dev-dependencies |
| `src/lib.rs` | Both constraints are visible in one signature each |




### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_tls/docs/workaround
printf 'instances:                '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### TL[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| TL[0-9]+ ' readme.md
# instances:                2
# finding headings inside:  3
# rows in the table below:  3
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TL50 | `drain` | n/a — observation | `pub fn drain( &mut self ) -> std::vec::Drain< '_, T >` is the family's only `std` type in a public signature. |
| TL51 | `Vec` | **latent hazard** | `Drain` puts `Vec` in the public contract, so changing the storage is breaking even though `drain`'s meaning would not change. |
| TL52 | `RingError` | n/a — observation | Two of nine `RingError` variants carry data, both a number the caller passed in, so the constraint is specific to `Full` rather than a property of the enum. |
