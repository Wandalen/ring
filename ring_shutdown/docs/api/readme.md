# API Doc Definition

### Scope

- **Purpose**: Fix the operation surface this crate presents — the graded half, where each promise is marked as holding by construction or by convention, and the ungraded half the grading leaves out.
- **Responsibility**: Method sets, signatures, error shape, the guarantee column, and what a holder of each returned value can reach from it.
- **In Scope**: `Shutdown`, `Stopped`, `Guarded`, `Refusal`, `Wake`, and the free functions `reset`, `wait_for_close`, `for_space_or_close`.
- **Out of Scope**: The drain procedure itself (→ [`algorithm/`](../algorithm/readme.md)); the two token types' rationale (→ [`type/`](../type/readme.md)); attribute-by-attribute census (→ [`item/001`](../item/001_the_declared_surface_and_its_attributes.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Shutdown Surface](001_shutdown_surface.md) | The whole surface in one table, with the guarantee-by-construction column that is the point of the crate | 🔄 |
| 002 | [The Surface the Table Does Not Grade](002_the_surface_the_table_does_not_grade.md) | The ten public names the table omits, and the two exits from the guarantee it grades | 🔄 |

**The split is the table and its complement.** `001` is the graded surface — the
eleven entry points a caller performs *on* a shutdown, each marked by whether
its promise survives a mistake. `002` is what the grading rule quietly excluded:
the producer side and the accessors, the operations a caller performs on the
values a shutdown hands back.

They are separate documents because they go stale on different edits. `001`
changes when a promise changes; `002` changes when a declaration is added, which
in this crate has happened more often. Splitting them also stops the second from
reading as an erratum for the first — the omissions are a rule consistently
applied, and the rule is worth stating on its own terms before its consequences
are argued.

The four findings pair off across that split, and the two pairs are different in
kind. `001`'s are **arithmetic**: a table whose own summary miscounts it in two
places and has propagated the wrong denominator into two more (SD5), and a table
introduced as the whole surface that covers eleven of twenty-one names (SD6).
`002`'s are **semantic**: a `const` accessor that grants strictly more than the
method documented as giving up the guarantee (SD7), and one verb used for four
predicates that answer about four different instants (SD8).

The two pairs meet at one point. SD6's most consequential omission — the
`Guarded` accessor that is not on the table — is SD7's subject. The counting
error and the reachability hazard are the same gap seen from two sides: nobody
has read this surface as a list.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_shutdown/docs/api
printf 'instances:                    %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:      %s\n' "$( command grep -hoE '^### SD[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:      %s\n' "$( command grep -coE '^\| SD[0-9]+ ' readme.md )"
printf 'each instance has a recipe:   %s\n' "$( command grep -lc '^### Regenerate' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows the surface table grades: %s\n' "$( awk -F'\\|' '/^\| /&&( $4 ~ /construction|convention/ || $4 == " — " ){ n++ } END{ print n+0 }' 001_shutdown_surface.md )"
printf 'names it puts in those rows:  %s\n' "$( awk -F'\\|' '/^\| /{ print $3 }' 001_shutdown_surface.md | command grep -ohE '[a-z_]+\(' | tr -d '(' | sort -u | wc -l )"
printf 'distinct public fn names:     %s\n' "$( command grep -ohE '^ *pub (const )?fn [a-z_]+' ../../src/lib.rs | awk '{ print $NF }' | sort -u | wc -l )"
printf 'public fn declarations:       %s\n' "$( command grep -cE '^ *pub (const )?fn ' ../../src/lib.rs )"
printf 'methods returning a reference: %s\n' "$( command grep -cE '^ *pub (const )?fn .*-> &' ../../src/lib.rs )"
printf 'and what close asks for:      %s\n' "$( awk '/^  pub fn close/{ sub( /.*close/, "close", $0 ); print }' ../../src/lib.rs )"
```

Live output:

```
instances:                    2
finding headings inside:      4
rows in the table below:      4
each instance has a recipe:   2
rows the surface table grades: 11
names it puts in those rows:  11
distinct public fn names:     21
public fn declarations:       23
methods returning a reference: 2
and what close asks for:      close( &self ) -> Stopped< '_ >
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SD5 | the document's own summary of its own table is wrong, twice, and has propagated | **wrong doc** | [`001`](001_shutdown_surface.md)'s Abstract says *"Four rows are construction, two are convention"* and a later section repeats *"Four rows read **construction**"* / *"Two rows read **convention**"*, while the table four lines away carries eleven rows of which six read **construction**, one reads **convention** and four read `—`; both figures are wrong in both places and wrong in opposite directions, and the wrong denominator has since been copied into two further statements — `001`'s own Types cross-reference row and the crate reading-order table in [`../readme.md`](../readme.md) both introduce `type/001` as *"the mechanism behind three of the four **construction** rows"* — so one uncounted table has produced four wrong statements across two files, in a column the Scope line calls *"the distinction the rest of this crate's documentation turns on"*, and no corpus checker compares two statements inside one document. |
| SD6 | the "whole surface" table omits the push it grades and the method that revokes the grade | n/a — coverage | [`001`](001_shutdown_surface.md) promises to *"fix the whole surface in one table"* and names eleven of the crate's twenty-one distinct public function names; the ten omitted are `drain_all_bounded free_capacity into_inner into_record is_blocked is_ready reason shutdown try_push try_push_batch`, which include `try_push` — the operation the row *"Wrap a producer … **construction** — `Guarded`'s only push checks"* is a claim *about*, graded without being listed — and `into_inner`, the method that ends the guarantee three of the six **construction** rows rest on and the subject of a whole decision instance; the selection rule (operations *on* a shutdown, not on what it returns) is defensible and unstated, so it is the word "whole" that is wrong, and it is what stops a reader looking further. |
| SD7 | a `const` accessor granted more than the method documented as giving up the guarantee, and now says so in both rustdocs | **latent hazard** | `Guarded::into_inner( self )` is the documented exit — it consumes the guard, says *"Give up the guarantee and take the raw producer back"*, and has [`decisions/001`](../decisions/001_should_into_inner_exist.md) to itself — while `Guarded::shutdown( &self ) -> &'a Shutdown` is a `#[ must_use ] const fn` documented as *"The shutdown this producer consults"*; but `Shutdown::close` also takes `&self`, so that reference is not read access — it grants `close()`, hence a `Stopped`, hence `drain_all`, `discard_all` and `reopen`, letting a `Guarded` holder close and drain the ring it is guarding while still holding the guard, at no cost — and it was graded by no table, argued in no decision and named by no test; both `shutdown` methods now carry a `# What this hands out` section stating plainly that the non-consuming accessor yields strictly more than the consuming exit, and `closing_through_the_guards_own_accessor_stops_the_guard` closes a ring through it, while the `api/001` grading stays deliberately absent pending `decisions/002`. |
| SD8 | four predicates share one verb and answer about four different instants | n/a — inconsistency | The crate declares twenty-three public functions under twenty-one names; the two repeated names behave oppositely — `shutdown` is a true synonym on `Stopped` and `Guarded`, while `is_closed` is a homonym: `Shutdown::is_closed` is *"whether the ring has been closed to further publication"*, a live `Acquire` load that can differ between consecutive calls, and `Refusal::is_closed` is *"whether the refusal was a close rather than back-pressure"*, a `matches!` over an owned enum frozen at one past instant; widening by one letter gives four — `Shutdown::is_closed` (now), `Guarded::is_blocked` (now, and *"does not predict a refusal"*), `Refusal::is_closed` (then), `Wake::is_ready` (during a completed wait, and possibly already false) — of which [`001`](001_shutdown_surface.md) grades exactly one, and the crate's own doctest reads the frozen one three lines after closing the live one with no cue that the receiver changed. |
