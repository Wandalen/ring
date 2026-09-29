# Data Structure Doc Definition

### Scope

- **Purpose**: The physical shape of what this crate holds — one atomic field, and five public types with the derive sets that decide what a caller can do with them.
- **Responsibility**: The layout, the orderings, the write set's owners, and the derive lists with the bounds they imply.
- **In Scope**: `Shutdown::closed` and its three access sites; the derive attributes on `Shutdown`, `Stopped`, `Refusal`, `Guarded`, `Wake`.
- **Out of Scope**: What the types argue for, which is semantic rather than structural (→ [`../type/readme.md`](../type/readme.md)); the ring's own storage, which is `ring_core`'s (→ [`ring_core/docs/data_structure/readme.md`](../../../ring_core/docs/data_structure/readme.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [The Close Flag and Its Orderings](001_the_close_flag_and_its_orderings.md) | One `AtomicBool`, one `Release`/`Acquire` pair, and a write set split across two types | 🔄 |
| 002 | [Five Public Types and Their Derive Sets](002_five_public_types_and_their_derive_sets.md) | What each type derives, what that permits, and where the sets stop agreeing | 🔄 |

**One document is about a single field; the other is about everything wrapped
around it.** They are separate because they answer different questions and go
stale on different edits — `001` on any change to the flag's access sites or the
family's atomic arrangement, `002` on any derive attribute anywhere in the crate
— and because a reader chasing a memory-ordering question has no reason to read
about `Hash`.

The four findings divide the same way, and both pairs land on the same shape:
**a structural decision that is correct and unrecorded.** `001`'s are about the
one atomic the family's twenty-nine loom models cannot reach, because it comes
from `core` rather than from `ring_atomic` (SD9), and about a cross-type private
field access that is load-bearing and unexplained at the site (SD10). `002`'s
are about two sibling enums with different derive sets and no rule anywhere
distinguishing them (SD11), and about a type whose whole argument is
payload-preservation being tested only at `u32`, where preservation is moot
(SD12).

Read together they say the crate's structure is deliberate throughout and
documented nowhere in the code — every one of the four is a choice a maintainer
would have to re-derive from scratch.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_shutdown/docs/data_structure
printf 'instances:                    %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:      %s\n' "$( command grep -hoE '^### SD[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:      %s\n' "$( command grep -coE '^\| SD[0-9]+ ' readme.md )"
printf 'each instance has a recipe:   %s\n' "$( command grep -lc '^### Regenerate' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'fields of shared state:       %s\n' "$( awk '/^pub struct Shutdown$/{f=1} f&&/^\}$/{exit} f&&/ : /' ../../src/lib.rs | wc -l )"
printf 'public types with a derive:   %s\n' "$( command grep -c '^#\[ derive' ../../src/lib.rs )"
printf 'public types in total:        %s\n' "$( command grep -cE '^pub (struct|enum) ' ../../src/lib.rs )"
printf 'distinct derive sets among:   %s\n' "$( command grep -hE '^#\[ derive' ../../src/lib.rs | sort -u | wc -l )"
```

Live output:

```
instances:                    2
finding headings inside:      4
rows in the table below:      4
each instance has a recipe:   2
fields of shared state:       1
public types with a derive:   5
public types in total:        5
distinct derive sets among:   3
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SD9 | the family's one liveness flag is the one atomic its loom models cannot reach, and the trade is now recorded where the orderings are | **latent hazard** | [`invariant/001`](../invariant/001_exactly_one_liveness_flag.md) establishes that the family holds exactly one "is this ring closed" flag and that it is `Shutdown::closed`, and that field is a `core::sync::atomic::AtomicBool` — this crate's manifest never names `ring_atomic`, which is what the family's loom instrumentation swaps; seven crates depend on it and twenty-nine files run a `loom::model`, none of which names `Shutdown`, so the single piece of shared mutable state whose `Release`/`Acquire` pair is this crate's only ordering decision sits structurally outside every model the family runs, and closing the gap means taking a `ring_atomic` dependency edge; the original claim that [`integration/001`](../integration/001_family_dependency_seam.md) argues against that edge was wrong — that document names the crate zero times and its stated rule would admit it — and the Orderings section now records the trade and the condition that would reverse it, leaving the gap itself open and measured. |
| SD10 | the field is private to the module rather than to its struct, and nothing says so at the site | n/a — doc gap | `Shutdown::close` writes the flag true and `Stopped::reopen` writes it false by reaching through `self.shutdown.closed` into another struct's private field, which compiles only because Rust's privacy is module-scoped and both types share one module; the arrangement is load-bearing — a `Shutdown::reopen` would let the flag be cleared without consuming the token, dissolving [`type/001`](../type/001_stopped_proof_token.md)'s second property — but `reopen`'s doc explains only why it takes `self` by value, so a maintainer splitting the module into `shutdown.rs` and `stopped.rs` meets a privacy error with no explanation in view and the obvious repair is the method that breaks the guarantee. |
| SD11 | two public enums, one hashable, and no rule anywhere saying which | n/a — inconsistency | `Wake` derives `Hash` and `Refusal` does not, though they are the crate's only two public enums, both small, both returned from operations a caller dispatches on, and both plausible map keys; `#[ derive( Hash ) ]` on a generic enum adds a `where T : Hash` bound and costs nothing unused — exactly as `Refusal`'s existing `Copy` and `PartialEq` derives already do — so the asymmetry was available to avoid, and neither [`api/001`](../api/001_shutdown_surface.md)'s whole-surface table nor either type's doc comment mentions derives at all, leaving a difference that reads as deliberate with no evidence that it was. |
| SD12 | the suite can only exercise `Refusal` in the shape where its purpose is moot | n/a — coverage | [`type/002`](../type/002_refusal_carries_the_record.md) argues the two-armed type exists so the record comes back intact — an argument about not losing a payload that is expensive or impossible to rebuild — yet every ring in the suite and in every doctest is a `Ring< u32 >`, where `Copy` makes the hand-back free and nothing is at stake; and the gap does not close by widening the element type, because one of the seven `Refusal` assertions is a whole-value `assert_eq!( refused, Refusal::Full( 3 ) )` that compiles only at a `PartialEq` `T`, so reaching the shape the type was built for means re-authoring the tests rather than adding one. |
