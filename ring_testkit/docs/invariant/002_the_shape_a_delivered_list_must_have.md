# Invariant: The Shape A Delivered List Must Have

### Scope

- **Purpose**: State the two properties a list of delivered records must have on its own, without any accounting beside it, and be precise about where each one is actually exercised.
- **Responsibility**: R2 and R3 as properties of a bare `&[ u32 ]`; why they are checkable with no `Outcome`; which of them the concurrent test can reach.
- **In Scope**: `audit_received`, its two passes, and its call sites in both settings.
- **Out of Scope**: The accounting law the list sits beside (→ [`001`](001_every_minted_record_is_somewhere.md)); why strict ascent is a single-producer property (→ [`algorithm/002`](../algorithm/002_the_four_passes_of_an_audit.md) TK3).

### Invariant Statement

Given a list of records and the number of records a run minted, and **nothing
else**:

| # | Property | Holds when |
|---|---|---|
| R2 | Provenance — every delivered record was minted by this run | every `value` in the list is `< minted` |
| R3 | Shape — the list strictly ascends | no adjacent pair has `then <= previous` |

Neither property needs a count of refusals, a staging buffer, or a ring. That is
the whole reason `audit_received` is a free function rather than a second method
on `Outcome`: a consumer thread under `loom::model` has a `Vec` and a number, and
no accounting to attach them to.

### Why the list is checked apart from the counts

[`001`](001_every_minted_record_is_somewhere.md)'s R1 is a statement about five
counters. R2 and R3 are statements about one slice. The split follows the two
settings the crate runs in:

| Setting | What exists | Which properties are checkable |
|---|---|---|
| A script, single-threaded | a full `Outcome` — five counters, a list, a flag | R1, R2, R3 |
| A `loom::model` closure | a `Vec` a consumer thread pushed into, and a literal bound | R2, R3 |

`Outcome::audit` is the composition: R1 first, then delegate the list to
`audit_received`. Checking R1 first is deliberate — a count failure is reported
as a count failure rather than as whatever record anomaly it also happens to
produce.

### Enforcement Mechanism

| # | Mechanism | What it prevents |
|---|---|---|
| E1 | R2 is a bound against `minted`, not a set membership test | The check costs one comparison per record and needs no allocation inside a model closure |
| E2 | R3 is stated as strict ascent, so a repeat is the equality case | One comparison finds duplicates and reorderings both, with one anomaly variant |
| E3 | Both passes return on the first failure | A list with two faults names one of them, and the caller fixes one thing at a time |
| E4 | The function takes `&[ u32 ]`, not `&Outcome` | A concurrent test with no accounting can still assert the list's shape |

### Violation Consequences

| # | Violation | Reported as |
|---|---|---|
| V1 | A record appears that this run could not have minted | `Anomaly::Unminted { value, minted }` |
| V2 | An adjacent repeat | `Anomaly::OutOfOrder { previous, then }`, `previous == then` |
| V3 | A reordering, or a non-adjacent repeat | `Anomaly::OutOfOrder { previous, then }`, `previous != then` |

V2 and V3 share a variant on purpose, and the signature that separates them is
narrower than [`001`](001_every_minted_record_is_somewhere.md) states —
→ TK22 there.

### Where each property is exercised

The two passes do not get equal exercise, and the asymmetry follows from the
loom model's declared bound rather than from anything in this file:

| Pass | Scripted suite | Loom model |
|---|---|---|
| R2 — provenance | direct assertions, plus every scripted run | yes — `audit_received( &received, 1 )` |
| R3 — ascent | direct assertions, plus every scripted run | **no** — see TK24 |

### Evidence

| # | Claim | Test |
|---|---|---|
| W1 | V1 is detected | `a_record_that_was_never_minted_is_caught` |
| W2 | V2 and V3 are one anomaly | `a_duplicate_and_a_reversal_are_one_anomaly` |
| W3 | Gaps are legal — a dropped record leaves one | `an_ascending_list_of_minted_records_passes_the_audit` |
| W4 | The list is checked even when the counts add up | `an_outcome_that_adds_up_is_still_checked_for_its_records` |
| W5 | The function is reachable from a loom thread | `tests/exhaustive_test.rs`, inside the model closure |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit
printf 'passes in audit_received:      %s\n' "$( awk '/pub fn audit_received/{f=1} f && /^}$/{exit} f && /^  for /{n++} END{print n+0}' src/lib.rs )"
printf 'the provenance comparison:     %s\n' "$( awk '/pub fn audit_received/{f=1} f && /value >= minted/{ sub( /^ */, "" ); print; exit }' src/lib.rs )"
printf 'the ascent comparison:         %s\n' "$( awk '/pub fn audit_received/{f=1} f && /then <= previous/{ sub( /^ */, "" ); print; exit }' src/lib.rs )"
printf 'the window the ascent walks:   %s\n' "$( awk '/pub fn audit_received/{f=1} f && /windows/{ sub( /^ */, "" ); print; exit }' src/lib.rs )"
printf 'call sites, scripted suite:    %s\n' "$( command grep -c 'audit_received(' tests/testkit_test.rs || true )"
printf 'call sites, loom model:        %s\n' "$( command grep -c 'audit_received(' tests/exhaustive_test.rs || true )"
printf 'the loom call:                 %s\n' "$( awk '/audit_received\(/{ sub( /^ */, "" ); print; exit }' tests/exhaustive_test.rs )"
printf 'the loom bound on the list:    %s\n' "$( awk '/received.len\(\) <= /{ sub( /^ */, "" ); print; exit }' tests/exhaustive_test.rs )"
printf 'pushes the model declares:     %s\n' "$( command grep -m1 -oE 'and \*\*one\*\* push' tests/exhaustive_test.rs )"
printf 'Outcome::audit calls in loom:  %s\n' "$( command grep -c '\.audit()' tests/exhaustive_test.rs || true )"
```

Live output:

```
passes in audit_received:      2
the provenance comparison:     if value >= minted
the ascent comparison:         if then <= previous
the window the ascent walks:   for pair in received.windows( 2 )
call sites, scripted suite:    10
call sites, loom model:        1
the loom call:                 audit_received( &received, 1 ), Ok( () ),
the loom bound on the list:    assert!( received.len() <= 1, "drained {} past the only push ever made", received.len() );
pushes the model declares:     and **one** push
Outcome::audit calls in loom:  0
```

### Invariants

| File | Relationship |
|------|--------------|
| [001_every_minted_record_is_somewhere.md](001_every_minted_record_is_somewhere.md) | The accounting law these two properties sit beside |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_the_four_passes_of_an_audit.md](../algorithm/002_the_four_passes_of_an_audit.md) | Passes 3 and 4 as a procedure, and why ascent is single-producer |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_outcome_and_anomaly.md](../type/001_outcome_and_anomaly.md) | `Anomaly`'s four variants |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_the_edge_that_only_exists_under_a_cfg.md](../integration/002_the_edge_that_only_exists_under_a_cfg.md) | The cfg the loom column of the table above depends on |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | `audit_received` |

### Tests

| File | Relationship |
|------|--------------|
| `tests/testkit_test.rs` | W1–W4 |
| `tests/exhaustive_test.rs` | W5, and the bound that makes R3 unreachable there |

### TK23 — the accounting law has no enforcement in the only concurrent test

`tests/exhaustive_test.rs` calls `.audit()` **zero** times. It has no `Outcome`
to call it on: the model spawns two threads over a leaked ring's ends, and the
records a consumer thread saw arrive as a bare `Vec` with no counters beside
them. R1 — *every minted record is accepted, refused, or still staged*, the law
[`001`](001_every_minted_record_is_somewhere.md) is named for — is checked in
every scripted run and in none of the interleavings.

That is not avoidable and this finding does not propose avoiding it. Restoring
R1 under loom would mean threading five counters through two threads, which adds
atomics to a model whose execution count grows with exactly that.

What is worth recording is where the crate says so.
[`001`](001_every_minted_record_is_somewhere.md) has a section called *"What
this law does not catch"*, and it names one thing: a record the ring accepted
and destroyed. The second thing the law does not catch — *any* violation, in the
only setting where records move between threads — appears one table row further
down, in the Tests table, as a parenthetical: *"R2 and R3 under loom, where
there is no `Outcome` to check R1 with"*.

A reader who wants the law's blind spots reads the section named for them. The
larger of the two is not in it.

### TK24 — the ascent pass cannot execute under the model's declared bound

`audit_received`'s second pass is `for pair in received.windows( 2 )`. The loom
model asserts `received.len() <= 1` immediately before calling it, and the
declared bound at the top of the file is *"One producer, one consumer, a ring of
**2 slots**, and **one** push"*. `windows( 2 )` over a slice of length 0 or 1
yields no pairs, so the loop body never runs.

The call the model makes is `audit_received( &received, 1 )`. What executes
there is the provenance pass — one comparison, `value >= 1`, against the single
record that could exist. R3 is present in the call and unreachable through it.

This matters because R3 is the property the bridge is *for*. E4 justifies
`audit_received` being a free function so *"a concurrent test collecting records
from a thread"* can check the list's shape; ordering is the shape a concurrent
test can plausibly break, and provenance is the one it cannot. The bound is
right for what the model explores — a claim, a publish, a read — and it is
exactly one push short of the bound at which the ordering check starts existing.

Raising the bound to two pushes would make R3 live and multiply loom's execution
count, which is the trade the header already argues about for a different
reason. Nothing in either document currently connects the two.
