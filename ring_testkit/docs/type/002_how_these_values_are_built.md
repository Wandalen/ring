# Type: How These Values Are Built

### Scope

- **Purpose**: Account for how each of the crate's four types comes into existence — by constructor, by builder, or by struct literal — and what that admits that a constructor would not.
- **Responsibility**: The construction path per type, the sites that use each, what `#[ non_exhaustive ]` does and does not freeze, and where the suite's invalid values actually come from.
- **In Scope**: `Outcome`, `Anomaly`, `Step`, `Script` as constructed values; the literal sites inside the crate.
- **Out of Scope**: What the fields mean (→ [`001`](001_outcome_and_anomaly.md)); what `Script::new`, `then` and `run` promise (→ [`../api/001`](../api/001_the_script_surface.md)).

### Four types, three construction paths

| Type | How a value is obtained | Constructor | Fields or variants reachable from outside |
|---|---|---|---|
| `Script` | `Script::new( stage_limit )`, then `then( step )` per step | yes | none — both fields are private |
| `Step` | Naming a variant, three of them with a payload | n/a — a fieldless enum | all 10 |
| `Outcome` | Returned by `Script::run`, **or** written as a struct literal | no | all 10, every one `pub` |
| `Anomaly` | Returned by `audit` or `audit_received`, **or** written as a struct-variant literal | no | all 4, every field `pub` |

The top two rows are closed: a caller cannot reach inside a `Script`, and a
`Step` has no state a caller could get wrong. The bottom two are open by design —
`001`'s Validation section argues for it directly, because a type that refused to
represent a broken run could not report one.

### What being open costs, and what it freezes

`Anomaly` carries `#[ non_exhaustive ]` and `Outcome` does not; the count of that
attribute in `src/lib.rs` is **one**, across all four types. Combined with public
fields, that freezes one shape and leaves the other additive:

| Shape | Frozen at | A change would break |
|---|---|---|
| `Outcome`'s ten fields | every struct literal that names all ten | in-crate literal sites, then every consumer's |
| `Anomaly`'s four variants | every exhaustive `match` inside this crate | in-crate matches only — `#[ non_exhaustive ]` obliges a consumer's `match` to carry a wildcard, so a fifth variant reaches them additively |

This is the intended trade for a testkit — an opaque `Outcome` would defeat the
whole returned-verdict pattern
(→ [`../pattern/002`](../pattern/002_the_fixture_returns_a_verdict_instead_of_making_one.md)) —
and it is worth naming because `001` argues against a tenth field on staleness
grounds and never mentions that a tenth field is also a breaking change at every
literal site the crate already has.

**The tenth field has since arrived, and it was not the one `001` argued about.**
`published` records the order in which records were accepted; it is new data, not
a cached derivation, so the staleness objection never reached it. The literal-site
cost this section names did: all four in-crate `Outcome` literals had to be
rewritten to name the new field. The cost landed exactly where predicted, paid by
a change the argument against it did not cover.

### Where the suite's invalid values come from

An `Outcome` that fails its audit can arrive two ways: a script produced it, or
someone wrote it. Both are legal, and both are exercised.

| Failing case | How the value was obtained |
|---|---|
| `an_outcome_that_lost_a_record_fails_the_audit` | struct literal |
| `an_outcome_that_adds_up_is_still_checked_for_its_records` | struct literal |
| `an_outcome_that_delivered_more_than_it_accepted_is_caught` | struct literal |
| `a_script_run_twice_on_one_ring_produces_an_outcome_that_fails_its_audit` | `Script::run`, over a ring an earlier run had already filled |
| The four `audit_received` and `audit_received_unordered` failure tests | bare `&[ u32 ]` slices, no `Outcome` at all |

One scripted run in the suite produces an `Outcome` that fails its own audit, and
it needs no cooperation from the caller beyond a mistake `run`'s own contract
names. The other seven failing cases are written by hand. So the design rationale
in `001` — that the type stays constructible so a broken run can still be
described — is exercised mostly, but no longer only, by values no run produced
→ TK50.

### Evidence

| # | Claim | Test |
|---|---|---|
| Z1 | A script-produced `Outcome` is comparable against a full struct literal | `an_empty_script_produces_an_empty_outcome` |
| Z2 | A hand-built `Outcome` reaches `audit` and fails it | `an_outcome_that_lost_a_record_fails_the_audit` |
| Z3 | An `Anomaly` literal is comparable against a returned one | `an_outcome_that_adds_up_is_still_checked_for_its_records` |
| Z4 | A `Script` is built only through `new` and `then` | `a_script_reports_what_it_was_built_from` |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit
printf 'non_exhaustive in src:          %s\n' "$( command grep -c 'non_exhaustive' src/lib.rs || true )"
printf 'pub fields on Outcome:          %s\n' "$( awk '/^pub struct Outcome/{f=1} f && /^}$/{exit} f' src/lib.rs | command grep -cE '^  pub ' || true )"
printf 'pub fields on Script:           %s\n' "$( awk '/^pub struct Script/{f=1} f && /^}$/{exit} f' src/lib.rs | command grep -cE '^  pub ' || true )"
printf 'constructors on Outcome:        %s\n' "$( awk '/^impl Outcome/{f=1} f && /^}$/{exit} f' src/lib.rs | command grep -cE 'pub fn new' || true )"
printf 'constructors on Script:         %s\n' "$( awk '/^impl Script/{f=1} f && /^}$/{exit} f' src/lib.rs | command grep -cE 'pub fn new' || true )"
printf 'Outcome literals in tests:      %s\n' "$( cat tests/*.rs | command grep -c 'Outcome$' || true )"
printf 'Anomaly literals in tests:      %s\n' "$( cat tests/*.rs | command grep -c 'Anomaly::[A-Z]' || true )"
printf 'tests failing audit, hand-built: %s\n' "$( awk '/^fn [a-z_]+\(/{ n=$2 ; b="" } /^fn /,/^}$/ { b = b $0 } /^}$/ { if ( b ~ /let outcome = Outcome/ && b ~ /audit\(\),/ && b ~ /Err\( Anomaly/ ) print n }' tests/testkit_test.rs | wc -l )"
printf 'tests failing audit, bare slice: %s\n' "$( awk '/^fn [a-z_]+\(/{ n=$2 ; b="" } /^fn /,/^}$/ { b = b $0 } /^}$/ { if ( b ~ /audit_received/ && b ~ /Err\( Anomaly/ ) print n }' tests/testkit_test.rs | wc -l )"
printf 'tests failing audit, from a run: %s\n' "$( awk '/^fn [a-z_]+\(/{ n=$2 ; b="" } /^fn /,/^}$/ { b = b $0 } /^}$/ { if ( b ~ /\.run\( &mut/ && b ~ /audit\(\),/ && b ~ /Err\( Anomaly/ ) print n }' tests/testkit_test.rs | wc -l )"
printf 'script-produced audits asserted Ok: %s\n' "$( cat tests/*.rs | command grep -c 'audit(), Ok' || true )"
printf 'Step variants a caller can name: %s\n' "$( awk '/^pub enum Step/{f=1} f && /^  [A-Z]/{n++} f && /^}$/{exit} END{print n+0}' src/lib.rs )"
```

Live output:

```
non_exhaustive in src:          1
pub fields on Outcome:          10
pub fields on Script:           0
constructors on Outcome:        0
constructors on Script:         1
Outcome literals in tests:      4
Anomaly literals in tests:      18
tests failing audit, hand-built: 3
tests failing audit, bare slice: 4
tests failing audit, from a run: 1
script-produced audits asserted Ok: 12
Step variants a caller can name: 10
```

### Types

| File | Relationship |
|------|--------------|
| [001_outcome_and_anomaly.md](001_outcome_and_anomaly.md) | What the fields and variants mean, and why validation is opt-in |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_script_surface.md](../api/001_the_script_surface.md) | `new`, `then` and `run` as the constructing surface |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_the_fixture_returns_a_verdict_instead_of_making_one.md](../pattern/002_the_fixture_returns_a_verdict_instead_of_making_one.md) | Why the returned value has to stay inspectable |

### Items

| File | Relationship |
|------|--------------|
| [../item/002_what_the_crate_does_not_declare.md](../item/002_what_the_crate_does_not_declare.md) | `#[ non_exhaustive ]` on `Anomaly` as an item-level absence — A4, closed by TK27 once the attribute and the fourth variant landed |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | Every count above |

### Tests

| File | Relationship |
|------|--------------|
| `tests/testkit_test.rs` | Z1–Z4, and every literal site counted |

### TK49 — one of the four types is frozen by the crate's own literals, and that freeze has already been paid once

`Outcome` has ten public fields and no constructor. `Anomaly` has four variants
with public fields and no constructor, and it carries `#[ non_exhaustive ]` — the
count of that attribute across `src/lib.rs` is one, and `Outcome` is not where it
sits.

Openness is the right call and `001` argues it well. What is not written down is
the second consequence, and it falls on exactly one of the two open types. A
struct literal that names all ten fields is a compile-time dependency on there
being exactly ten, and the crate's own suite contains four of them. So adding a
field to `Outcome` — the change `001` discusses explicitly, and rejects on the
grounds that a stored `vanished` could go stale — is also a change that breaks
four sites inside this crate before a single consumer is considered. `Anomaly`
does not carry the matching cost: its eighteen mentions across the test files are
constructions and prose, not matches, and the suite contains no `match` over an
`Anomaly` at all. The one exhaustive `match` on it is the `Display` impl in
`src/lib.rs`, and `#[ non_exhaustive ]` keeps a fifth variant additive for every
consumer regardless.

That did not make the rejection wrong; the staleness argument stands on its own.
It made the cost of the decision larger than the document stated — and the cost
has since been paid. `published` was added as `Outcome`'s tenth field, and all
four in-crate literals were rewritten to name it. The prediction held. What it
got wrong was which field would trigger it: `published` records new data rather
than caching the derivation `001` argued against, so the objection on record was
never the one that had to be overcome.

What remains is the bound, and it is narrower than this entry originally claimed.
A testkit is a type whose consumers write struct literals against it, and the
moment there is more than zero of them
(→ [`../api/002`](../api/002_the_surface_no_crate_has_taken.md) TK7) `Outcome`'s
ten fields stop being cheap to extend. `Anomaly`'s four variants never acquire
that constraint, because the attribute whose absence would have imposed it is
present.

### TK50 — one of the eight broken values the suite audits came out of a run, and the census line that said otherwise could not have seen it

The Validation section's rationale for leaving `Outcome` freely constructible is
that a fixture *"that refused to build an `Outcome` whose counters did not add up
would be unable to report the one case worth reporting"*. The case worth
reporting is a run whose accounting is broken.

Eight tests assert a failing audit. Three build an `Outcome` as a struct literal
in the test body. Four pass a bare `&[ u32 ]` slice to `audit_received` or
`audit_received_unordered`, with no `Outcome` behind them at all. One —
`a_script_run_twice_on_one_ring_produces_an_outcome_that_fails_its_audit` —
audits a value that came straight out of `Script::run`.

**This entry originally recorded that last count as zero, and the census line
meant to verify it could not have returned anything else.** The recipe matched
`run( … ).audit(), Err` on a single line; the test that contradicts it binds the
second run to a variable and spreads its assertion across five lines, so the
grep reported a true `0` for a question it was not asking — and that `0` is what
made the headline read as measured rather than assumed. The sibling line counting
hand-built literals by `outcome.audit(), Err` undercounted for the same reason,
missing the third literal. Both are now function-body scans, which see a
multi-line `assert_eq!`.

The scripted failure is not a ring losing a record. `Script::run`'s contract
warns that a caller *"comparing two runs must supply two rings, not run twice on
one"*; run twice on one and the second `Outcome` describes six records leaving a
ring that accepted three — every field individually plausible, the set of them
impossible, and `vanished()` reporting a healthy `0` throughout. Only `audit`
catches it. That makes `001`'s constructible-type rationale load-bearing for a
reachable caller mistake, not only for hand-written fixtures.

The narrower claim still holds. No scripted run in this suite fails its audit
because the *ring* lost a record — the event the crate exists to catch — and none
can, because the in-house rings do not produce it. So what the suite establishes
is that `audit` computes the right answer for a given ten-tuple, and that
misusing `run` can assemble a ten-tuple that trips it; it does not establish that
a ring under test ever will. That last one needs a consumer's own implementation,
and there are none
(→ [`../non_functional_requirement/002`](../non_functional_requirement/002_what_a_fixture_owes_its_consumers.md)).
