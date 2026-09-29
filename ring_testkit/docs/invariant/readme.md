# Invariant Doc Definition

### Scope

- **Purpose**: State the laws a run must satisfy — the accounting one over five counters, and the shape one over the delivered list — together with what enforces each and what each deliberately does not catch.
- **Responsibility**: The invariant statements, their enforcement mechanisms, the anomalies each violation produces, and the settings each law is actually checked in.
- **In Scope**: `Outcome::audit`, `audit_received`, and the `Anomaly` variants they return.
- **Out of Scope**: The procedure the checks are written as (→ [`algorithm/002`](../algorithm/002_the_four_passes_of_an_audit.md)); the fields the accounting sums (→ [`data_structure/002`](../data_structure/002_nine_counters_and_a_number_that_is_two_things.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Every Minted Record Is Somewhere](001_every_minted_record_is_somewhere.md) | The accounting law over five buckets, and the destruction it is silent about on purpose | 🔄 |
| 002 | [The Shape A Delivered List Must Have](002_the_shape_a_delivered_list_must_have.md) | The two properties of a bare slice, and which of them the concurrent test can reach | 🔄 |

**The split is by what the check needs to be given.** `001`'s law is a statement
about five counters and cannot be evaluated without an `Outcome`. `002`'s two
properties need a slice and one number, which is precisely the difference that
makes `audit_received` a free function and makes it the only one of the two
reachable from inside a `loom::model` closure.

They are apart because the boundary between them is the boundary between the
crate's two halves. A reader who only ever writes scripts needs `001` and gets
`002` for free through `Outcome::audit`. A reader writing a concurrent test gets
`002` and cannot have `001` at all.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit/docs/invariant
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### TK[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| TK[0-9]+ ' readme.md )"
printf 'properties stated:        %s\n' "$( command grep -hcE '^\| R[0-9] \|' [0-9][0-9][0-9]_*.md | paste -sd+ | bc )"
```

Live output:

```
instances:                2
finding headings inside:  4
rows in the table below:  4
properties stated:        5
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TK21 | E2's coverage of the R1 sum | n/a — unenforced | E2 states the enforcement as exhaustive matching of every `try_push` result, which holds at both sites with six arms and no wildcard; `refused_staging`, the fifth of R1's five buckets, is filled by `if staging.push( record ).is_err()` instead, so a second staging failure mode would compile silently where a third `Refusal` variant breaks the build twice. |
| TK22 | V3's duplicate signature | **misleading doc** | V3 tells a reader a repeat is `OutOfOrder` with `previous == then`; that equality is a property of adjacency, so `[ 0, 1, 0 ]` reports `previous : 1, then : 0` and is indistinguishable from a reordering — and W7's two assertions, `&[ 1, 1 ]` and `&[ 2, 1 ]`, are both adjacent pairs. |
| TK23 | where the blind spots are recorded | n/a — doc gap | `001`'s "What this law does not catch" names one thing — a record the ring accepted and destroyed — while the larger gap, that `tests/exhaustive_test.rs` calls `.audit()` zero times so R1 holds in no interleaving at all, appears only as a parenthetical in the Tests table two sections later. |
| TK24 | the ascent pass under loom | n/a — coverage | The model asserts `received.len() <= 1` and calls `audit_received( &received, 1 )`, and `windows( 2 )` over a slice that short yields no pairs — so the ordering check, the property a concurrent test can plausibly break and the reason E4 makes the function free, cannot execute in the only test that explores interleavings. |
