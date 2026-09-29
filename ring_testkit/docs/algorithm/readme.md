# Algorithm Doc Definition

### Scope

- **Purpose**: Describe the two procedures this crate is made of — the one that produces a reading of a run, and the one that judges it.
- **Responsibility**: Step-by-step bodies, their costs, their termination arguments, and the assumptions each carries that its signature does not.
- **In Scope**: `Script::run`; `Outcome::audit`, `audit_delivery_order` and `audit_received`.
- **Out of Scope**: What the values mean (→ [`type/`](../type/readme.md)); what the properties are (→ [`invariant/`](../invariant/readme.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [From A Step To An Outcome](001_from_a_step_to_an_outcome.md) | What `Script::run` does per step, and where each counter moves | 🔄 |
| 002 | [The Four Passes Of An Audit](002_the_four_passes_of_an_audit.md) | The fixed order the checks run in, and the model each scan assumes | 🔄 |

**The split is producing a reading against judging one, and they are separable
because the second half is reachable without the first.** `001` describes a
procedure that needs a `Script` and a `Ring`; `002` describes one whose third
and fourth passes also exist as a free function taking a slice, called from a
`loom::model` where no `Outcome` exists. That is not a stylistic division —
`audit_received`'s own doc comment gives the same reason for being public.

Keeping them in one file would also merge two different termination arguments:
`run` terminates because every step is bounded by a constant or a buffer size,
`audit` because two `for` loops over a finite slice cannot not terminate. The
first is worth writing down and the second is not, and that asymmetry is only
visible when they are apart.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit/docs/algorithm
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### TK[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| TK[0-9]+ ' readme.md )"
```

Live output:

```
instances:                2
finding headings inside:  4
rows in the table below:  4
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TK1 | the `too_many_lines` reason | **misleading doc** | The suppression on `Script::run` justifies its length as "one match arm per Step", and the body collapses three pairs of variants into three shared arms — ten variants, seven arms — so the stated shape argument is contradicted by the first three arms of the match it annotates. |
| TK2 | `Stopped` in three arms | n/a — inconsistency | Three adjacent arms of one `match` produce the family's proof-of-closure token and drop it, produce and immediately consume it, and produce and use it; the discard is legal and its justification lives in another crate's doc example rather than anywhere in this one. |
| TK3 | `audit_received`'s ascent check | **latent hazard** | The ordering pass rejects any pair that does not strictly ascend — a single-producer property — inside the one function whose doc comment says it exists for the concurrent case, where two producers legitimately interleave and would be reported as an anomaly. |
| TK4 | "name the first that does not" | **misleading doc** | `audit` reports the earliest violation in *pass* order, not in list order: the provenance scan traverses the whole slice before the ordering scan begins, so a list holding both kinds reports the later-positioned one and the earlier is never reached. |
