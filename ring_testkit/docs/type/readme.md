# Type Doc Definition

### Scope

- **Purpose**: Define the four values this crate owns — what each field and variant means, and how each value is brought into existence — so a consumer reading a returned `Outcome` knows both what it says and what it permits.
- **Responsibility**: Fields, variants, derives, and the reasoning behind each; construction paths, the sites that use them, and what a public field set freezes where `#[ non_exhaustive ]` does not reach.
- **In Scope**: `Outcome`, `Anomaly`, `Step`, `Script` as declared types.
- **Out of Scope**: What the signatures promise (→ [`api/`](../api/readme.md)); the laws the values must satisfy (→ [`invariant/`](../invariant/readme.md)); the design choice that made them returned rather than asserted (→ [`pattern/`](../pattern/readme.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Outcome And Anomaly](001_outcome_and_anomaly.md) | Ten fields, four variants, four derive sets, and why validation is a separate call | 🔄 |
| 002 | [How These Values Are Built](002_how_these_values_are_built.md) | Three construction paths across four types, and where the suite's invalid values actually come from | 🔄 |

**The split is meaning and provenance.** `001` answers *what does this field
say* — why `refused_full` and `refused_closed` are two counters, why `vanished`
is a method rather than an eleventh field, why no field is a duration. `002` answers
*where did this value come from* — a builder, a run, or a struct literal — and
what each path admits.

They separate cleanly because the answers point in opposite directions. `001`'s
reasoning is about reading a value correctly, and its findings are about claims
made and not checked. `002`'s is about writing one, and its findings are about
what the crate's own test suite has already frozen and what its audit path has
never actually been handed.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit/docs/type
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### TK[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| TK[0-9]+ ' readme.md )"
printf 'each instance has a recipe: %s\n' "$( command grep -lc '^### Regenerate' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'types the crate declares: %s\n' "$( command grep -cE '^pub (struct|enum) ' ../../src/lib.rs )"
printf 'evidence rows, both files: %s\n' "$( command grep -hcE '^\| (Y[0-9]|Z[0-9]) \| ' [0-9][0-9][0-9]_*.md | paste -sd+ | bc )"
printf 'distinct tests they cite: %s\n' "$( command grep -hoE '\| .[a-z_]{6,}. \|$' [0-9][0-9][0-9]_*.md | tr -d '| \140' | sort -u | wc -l )"
```

Live output:

```
instances:                2
finding headings inside:  4
rows in the table below:  4
each instance has a recipe: 2
types the crate declares: 4
evidence rows, both files: 9
distinct tests they cite: 7
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TK47 | the reason given for `Script`'s `Clone` | **misleading doc** | The derives table justifies `Clone` as *"so one script can be run against several rings without rebuilding it"*, but `run` takes `&self` and needs no clone for that — the suite runs scripts against rings at twenty-three `script.run(` call sites with zero `.clone()` anywhere in `src/` or `tests/`, and `pattern/001` P3 names the same `&self` as the property the central requirement rests on. |
| TK48 | the fixed reporting order | n/a — unenforced | The Validation section states that `audit` reports the first failure and that the ordering *"is fixed rather than incidental"*; `audit` is three checks — `Unaccounted`, the `Overdelivered` ceiling, then `audit_delivery_order` — and the four failing-audit inputs in the suite violate exactly one property each, so no input can produce two answers and all six orderings of the three checks leave every test passing. `OutOfOrder` is never reached through `audit` at all. |
| TK49 | what the suite has already frozen, and what it has already paid | n/a — observation | `Outcome` is constructor-free with ten public fields and no `#[ non_exhaustive ]`, and the crate's own tests hold four struct literals naming all ten — so the field `001` rejects on staleness grounds breaks four in-crate sites before any consumer. That cost has since been paid: `published` landed as the tenth field and all four literals were rewritten. `Anomaly` does not share it — its eighteen test mentions are constructions and prose, never matches, and it carries the crate's only `#[ non_exhaustive ]`. |
| TK50 | where the audited broken values come from | n/a — coverage | Eight tests assert a failing audit: three struct literals, four bare `&[ u32 ]` slices, and one `Outcome` straight out of `Script::run`. This entry originally recorded that last count as zero, and the single-line census grep meant to verify it could not have matched the five-line assertion that contradicts it — so `audit` is verified to compute the right answer for a given ten-tuple, and misusing `run` can assemble one that trips it; neither establishes that a ring under test ever will. |
