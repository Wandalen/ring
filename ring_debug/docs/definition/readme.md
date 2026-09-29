# Doc Definitions

Module Index for `ring_debug` — every doc definition this crate declares, every
instance under each, every architecture decision, and every finding, in one place.

## Master Doc Definitions Table

| Type | Purpose | Master File | Instances | Findings |
|------|---------|-------------|----------:|---------:|
| `algorithm/` | How a pair is evaluated without perturbing it, and the ordering the family never asked for | [algorithm/readme.md](../algorithm/readme.md) | 2 | 4 |
| `api/` | The three entry points and their preconditions, and the rendered message as a second surface | [api/readme.md](../api/readme.md) | 2 | 4 |
| `data_structure/` | Three scalars with no identity, and four variants that drop the newtype they came from | [data_structure/readme.md](../data_structure/readme.md) | 2 | 4 |
| `decisions/` | What this crate may depend on, and whether the instrument remembers a fault | [decisions/readme.md](../decisions/readme.md) | 2 | 4 |
| `integration/` | What can be pointed at this crate, and the eight edges nobody drew | [integration/readme.md](../integration/readme.md) | 2 | 4 |
| `invariant/` | The cursor properties the family assumes, and the conservation law that holds only at rest | [invariant/readme.md](../invariant/readme.md) | 2 | 4 |
| `item/` | Three nouns and five verbs as a set, and what the crate declines to declare | [item/readme.md](../item/readme.md) | 2 | 4 |
| `lifecycle/` | A watch's states at run time, and the crate's own state in the task system | [lifecycle/readme.md](../lifecycle/readme.md) | 2 | 4 |
| `non_functional_requirement/` | The constraints keeping this crate off the hot path, and what actually enforces each | [non_functional_requirement/readme.md](../non_functional_requirement/readme.md) | 2 | 4 |
| `pattern/` | The guard that makes the next line legal, and committing nothing until every check has passed | [pattern/readme.md](../pattern/readme.md) | 2 | 4 |
| `pitfall/` | The arithmetic that reports a corrupt ring as healthy, and the number the borrow checker hides | [pitfall/readme.md](../pitfall/readme.md) | 2 | 4 |
| `type/` | The reported value, and the stateful instrument carrying a derive line written for reports | [type/readme.md](../type/readme.md) | 2 | 4 |
| `workaround/` | The door `ring_core` does not open, and one cast between two newtypes that disagree | [workaround/readme.md](../workaround/readme.md) | 2 | 4 |

**Total: 26 instances and 52 findings across 13 definitions.**

The crate is 420 lines with three entry points, and the corpus above is larger
than the crate. That is the shape to expect here rather than a sign of padding:
this crate's subject is what *other* crates assume and never check, so almost
every instance ends up measuring something outside `src/lib.rs` — the arithmetic
in `ring_seqno`, the export boundary in `ring_core`, the eight manifests that
never took the edge, the task file that still says the work was never scoped.

## Master Doc Instances Table

| Type | ID | Name |
|------|----|------|
| `algorithm/` | 001 | [Checking a Pair Without Touching It](../algorithm/001_checking_a_pair_without_touching_it.md) |
| `algorithm/` | 002 | [The Third Ordering](../algorithm/002_the_third_ordering.md) |
| `api/` | 001 | [The Check Surface](../api/001_the_check_surface.md) |
| `api/` | 002 | [The Message Is a Second API](../api/002_the_message_is_a_second_api.md) |
| `data_structure/` | 001 | [A Watch Is Three Scalars and No Identity](../data_structure/001_a_watch_is_three_scalars_and_no_identity.md) |
| `data_structure/` | 002 | [Four Variants and the Newtype They Drop](../data_structure/002_four_variants_and_the_newtype_they_drop.md) |
| `decisions/` | 001 | [Four Edges, Not Two](../decisions/001_four_edges_not_two.md) |
| `decisions/` | 002 | [A Watch Does Not Latch](../decisions/002_a_watch_does_not_latch.md) |
| `integration/` | 001 | [Reaching the Cursors of a Live Ring](../integration/001_reaching_the_cursors_of_a_live_ring.md) |
| `integration/` | 002 | [The Edges That Were Never Drawn](../integration/002_the_edges_that_were_never_drawn.md) |
| `invariant/` | 001 | [Cursor Invariants Over a Live Ring](../invariant/001_cursor_invariants_over_a_live_ring.md) |
| `invariant/` | 002 | [The Conservation Law and Why It Holds](../invariant/002_the_conservation_law_and_why_it_holds.md) |
| `item/` | 001 | [Three Nouns, Five Verbs, and the Enum Two Doors Cannot Reach](../item/001_three_nouns_five_verbs_and_the_enum_two_doors_cannot_reach.md) |
| `item/` | 002 | [What the Crate Does Not Declare](../item/002_what_the_crate_does_not_declare.md) |
| `lifecycle/` | 001 | [From One Observation to a Sequence](../lifecycle/001_from_one_observation_to_a_sequence.md) |
| `lifecycle/` | 002 | [A Finished Crate in the First Stage](../lifecycle/002_a_finished_crate_in_the_first_stage.md) |
| `non_functional_requirement/` | 001 | [Absent Unless Called](../non_functional_requirement/001_absent_unless_called.md) |
| `non_functional_requirement/` | 002 | [The Constraints With No Number](../non_functional_requirement/002_the_constraints_with_no_number.md) |
| `pattern/` | 001 | [The Guard That Makes the Next Line Legal](../pattern/001_the_guard_that_makes_the_next_line_legal.md) |
| `pattern/` | 002 | [Commit Nothing Until Every Check Has Passed](../pattern/002_commit_nothing_until_every_check_has_passed.md) |
| `pitfall/` | 001 | [Saturating Arithmetic Reports Health](../pitfall/001_saturating_arithmetic_reports_health.md) |
| `pitfall/` | 002 | [The Number the Borrow Checker Hides](../pitfall/002_the_number_the_borrow_checker_hides.md) |
| `type/` | 001 | [Violation](../type/001_violation.md) |
| `type/` | 002 | [Watch, and the Traits Nothing Asks For](../type/002_watch_and_the_traits_nothing_asks_for.md) |
| `workaround/` | 001 | [The Door Ring Core Does Not Open](../workaround/001_the_door_ring_core_does_not_open.md) |
| `workaround/` | 002 | [One Cast Between Two Newtypes That Disagree](../workaround/002_one_cast_between_two_newtypes_that_disagree.md) |

## Architecture Decision Records

| ADR | Rules | Status |
|---|---|---|
| [decisions/001_four_edges_not_two.md](../decisions/001_four_edges_not_two.md) | What this crate is allowed to depend on | accepted — and DB13 records that the correction lives only in the manifest |
| [decisions/002_a_watch_does_not_latch.md](../decisions/002_a_watch_does_not_latch.md) | Whether the instrument remembers a fault | pending — and DB15 records that the behaviour it defers on is untested |

Indexed here and in [`decisions/readme.md`](../decisions/readme.md) only, not in
`graph.yml` — `doc_des.rulebook.md` classifies `docs/decisions/` as a
non-doc-definition directory. The register behind them holds four closed
decisions and three pending ones, each with the condition that would settle it;
the two promoted to instances above are the two that grew a measurement.

## Reading order

[`pitfall/001`](../pitfall/001_saturating_arithmetic_reports_health.md) first —
it is the measurement everything else answers a question raised by: a ring whose
consumer has run ahead of its producer reports the readings of a healthy empty
one. Then [`invariant/001`](../invariant/001_cursor_invariants_over_a_live_ring.md)
for the properties that measurement turns into standing restrictions,
[`api/001`](../api/001_the_check_surface.md) for the three entry points and their
preconditions, and
[`integration/001`](../integration/001_reaching_the_cursors_of_a_live_ring.md) for
the boundary a reader is otherwise likely to find the hard way — the only check
reachable from a `ring_core::Ring` is the one that cannot see the corruption the
first document is about.

## Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'definitions declared:        %s\n' "$( ls -d ring_debug/docs/*/ | command grep -cv '/definition/$' )"
printf 'instances on disk:           %s\n' "$( ls ring_debug/docs/*/[0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the instances table: %s\n' "$( command grep -cE '^\| `[a-z_]+/` \| [0-9]{3} \|' ring_debug/docs/definition/readme.md )"
printf 'finding headings on disk:    %s\n' "$( command grep -rhoE '^### DB[0-9]+ — ' ring_debug/docs/*/[0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the findings table:  %s\n' "$( command grep -cE '^\| DB[0-9]+ \|' ring_debug/docs/definition/readme.md )"
```

Live output:

```
definitions declared:        13
instances on disk:           26
rows in the instances table: 26
finding headings on disk:    52
rows in the findings table:  52
```

## Findings

**52 findings, four per definition**, each recorded three times: as a `### DBn`
section at the end of the instance that measured it, as a row in that
definition's own `### Findings Recorded Here` table, and here. This table is the
only copy ordered by ID.

| ID | Finding | Subject | Tier | Where |
|----|---------|---------|------|-------|
| DB1 | A public enum is reachable from one of three entry points and the type system does not carry the condition. | `Cursor` | n/a — observation | [item/001](../item/001_three_nouns_five_verbs_and_the_enum_two_doors_cannot_reach.md) |
| DB2 | The crate's four attributes are exactly the complement of what `core` already provides, with no overlap and no gap. | attribute set | n/a — observation | [item/001](../item/001_three_nouns_five_verbs_and_the_enum_two_doors_cannot_reach.md) |
| DB3 | The `Error` impl is empty for a worked-out reason and six sibling crates are empty for unexamined ones, so the unanimity carries no information. | `impl Error for Violation` | n/a — inconsistency | [item/002](../item/002_what_the_crate_does_not_declare.md) |
| DB4 | A four-variant defect enum is closed to extension by default rather than by decision, in the one crate whose subject is unanticipated defects. | `Violation` | **latent hazard** | [item/002](../item/002_what_the_crate_does_not_declare.md) |
| DB5 | A stateful watch keeps no identity for the pair it watches, so observing a different ring produces a confident verdict about neither — declined, and pinned by a test. | `Watch::observe` | **latent hazard** | [data_structure/001](../data_structure/001_a_watch_is_three_scalars_and_no_identity.md) |
| DB6 | `Copy` was derived on a type whose value is being the single record of what was last seen; `Clone` alone now makes the forked baseline visible at the call site. | `Watch` | **latent hazard** | [data_structure/001](../data_structure/001_a_watch_is_three_scalars_and_no_identity.md) |
| DB7 | A validated capacity newtype is unwrapped to `usize` at the error boundary, correctly in one variant and incidentally in the other, with the two cases never distinguished. | `Violation` payloads | n/a — observation | [data_structure/002](../data_structure/002_four_variants_and_the_newtype_they_drop.md) |
| DB8 | Every payload is a detection-time copy rather than a reference, which is what stops a held violation from reporting the ring as healthy when it is finally read. | `Violation` | n/a — observation | [data_structure/002](../data_structure/002_four_variants_and_the_newtype_they_drop.md) |
| DB9 | `checked_sub` makes the D1 case the subtraction's own `None` branch, so the ordering that used to keep it sound is no longer expressible; eight unguarded subtractions remain elsewhere in the family. | `check_seqs` | **latent hazard** | [pattern/001](../pattern/001_the_guard_that_makes_the_next_line_legal.md) |
| DB10 | The family leaves `overflow-checks` unset in every profile, so whether a wrapped subtraction panics or lies silently is decided by build profile rather than by decision. | workspace profiles | n/a — unenforced | [pattern/001](../pattern/001_the_guard_that_makes_the_next_line_legal.md) |
| DB11 | One test of twenty-two separates a correct watch from one that reports a permanent fault once and then calls it normal, because the property is about the receiver rather than the return value. | `Watch::observe` | n/a — coverage | [pattern/002](../pattern/002_commit_nothing_until_every_check_has_passed.md) |
| DB12 | The construction-time guarantee `new` provides is voided by observing a different pair, so the crate's strongest guarantee is undone by a call that type-checks. | `Watch::new` | **latent hazard** | [pattern/002](../pattern/002_commit_nothing_until_every_check_has_passed.md) |
| DB13 | The design document assigns two dependency edges and the manifest declares four, with nothing reconciling them and no test that would fail if one were removed. | `Cargo.toml` against the design document | n/a — drift | [decisions/001](../decisions/001_four_edges_not_two.md) |
| DB14 | The deferred widening is framed as adding a new capability to a ring end, and `ring_spsc` already offers exactly that accessor on both of its own ends. | Pending 1 | **misleading doc** | [decisions/001](../decisions/001_four_edges_not_two.md) |
| DB15 | The transition this decision is entirely about is named in the lifecycle document and walked by none of the 22 tests. | `Watch::observe` recovery | n/a — coverage | [decisions/002](../decisions/002_a_watch_does_not_latch.md) |
| DB16 | The deferral's cost is stated as cheap by the one measure that can be counted, and the cost that cannot be counted is the one a caller pays. | Pending 2 Consequences | n/a — observation | [decisions/002](../decisions/002_a_watch_does_not_latch.md) |
| DB17 | The crate's central limitation is pinned by a passing test rather than a prose caveat, so it fails at the moment the limitation is fixed. | `check_ends_cannot_see_the_corruption_check_can` | n/a — observation | [workaround/001](../workaround/001_the_door_ring_core_does_not_open.md) |
| DB18 | The door built for a Contract-following consumer has no consumer, so the cost it was written to avoid is a prediction rather than a measurement. | `check_ends` | n/a — observation | [workaround/001](../workaround/001_the_door_ring_core_does_not_open.md) |
| DB19 | Two newtypes from one crate disagree about width, field visibility and validation, and one comparison in a third crate crosses all of it. | `Seq` against `Capacity` | n/a — inconsistency | [workaround/002](../workaround/002_one_cast_between_two_newtypes_that_disagree.md) |
| DB20 | The crate that checks the family's arithmetic rested on an unguarded subtraction, an unchecked cast and a discarded validation in a single expression; DB9's `checked_sub` closed the first, and two remain. | `check_seqs` | n/a — observation | [workaround/002](../workaround/002_one_cast_between_two_newtypes_that_disagree.md) |
| DB21 | The failure table quantifies a regression as costing exactly one test when four independent tests across three entry points would fail, understating the suite's coverage in the one place a reader would rely on the figure. | failure mode N2 | **misleading doc** | [algorithm/001](../algorithm/001_checking_a_pair_without_touching_it.md) |
| DB22 | The crate's only claim about machine behaviour prices an acquire load as free on x86, which is not the architecture this workspace builds on and not a target the manifest configures. | the `Acquire` cost row | **misleading doc** | [algorithm/001](../algorithm/001_checking_a_pair_without_touching_it.md) |
| DB23 | Reading the producer before the consumer selects D1 — the violation with no corroborating symptom — as the false positive read skew can manufacture; the decision was spelled at all three read sites and recorded at none, and is now made once in `observe_pair` and stated in its rustdoc. | the load order | **latent hazard** | [algorithm/002](../algorithm/002_the_third_ordering.md) |
| DB24 | Every design decision in the crate is a concurrency decision and the suite contains no threads, so the acquire ordering, the quiescence precondition, and the skew window are all argued rather than exercised. | the test suite | n/a — coverage | [algorithm/002](../algorithm/002_the_third_ordering.md) |
| DB25 | The crate defines an enum whose only job is naming which end a cursor belongs to, then returns both ends as an unlabelled pair whose order lives in a doc comment. | `Watch::last` | n/a — inconsistency | [api/001](../api/001_the_check_surface.md) |
| DB26 | The precedence of D3 over D1/D2 is pinned by one fixture whose numbers were chosen for a different purpose, so tidying the fixture would silently delete the guarantee's only coverage. | guarantee A4 | n/a — coverage | [api/001](../api/001_the_check_surface.md) |
| DB27 | The crate's most load-bearing sentence is a claim about a non-dependency's arithmetic held in a string literal, and nothing compared the two until a test built the D1 pair, asked `ring_core` what it reports, and asserted the words against the answer. | `ConsumerAheadOfProducer` message | **latent hazard** | [api/002](../api/002_the_message_is_a_second_api.md) |
| DB28 | The one number the crate derives outside a check was computed with saturating arithmetic in a publicly constructible variant, and its only fixture picked a consumer of zero, which made the subtraction indistinguishable from the field beside it; `checked_sub` and a nonzero fixture make both observable. | `ProducerLappedConsumer` rendering | **latent hazard** | [api/002](../api/002_the_message_is_a_second_api.md) |
| DB29 | The seam documented as having no failure shape routes through a plain subtraction that underflows on a lapped ring, so the crate's only Contract-reachable entry point panics inside a crate it does not name on the input most worth checking. | the Error Handling table | **latent hazard** | [integration/001](../integration/001_reaching_the_cursors_of_a_live_ring.md) |
| DB30 | An option is rejected as breaking for a third of the backends, on a backend that is opt-in, not requested by this crate, and never compiled in any build it participates in. | rejected option I1 | **misleading doc** | [integration/001](../integration/001_reaching_the_cursors_of_a_live_ring.md) |
| DB31 | Every crate named as a natural caller takes the prerequisite dependency the argument identifies and none takes the edge itself, leaving a complete and correct case for the crate unacted-on in eight manifests. | the eight named callers | n/a — unadopted | [integration/002](../integration/002_the_edges_that_were_never_drawn.md) |
| DB32 | The family built a testkit and a cursor checker for the same reader and neither names the other, so the one place the check would be idiomatic is the one place it has to be remembered. | `ring_testkit` and `ring_debug` | n/a — observation | [integration/002](../integration/002_the_edges_that_were_never_drawn.md) |
| DB33 | The inventory of what enforces the cursor invariants names both entry points a Contract caller cannot reach and omits the only one they can. | the enforcement table | n/a — doc gap | [invariant/001](../invariant/001_cursor_invariants_over_a_live_ring.md) |
| DB34 | Monotonicity protection is credited to a type that offers none — `Seq` has no decrementing method and a public field, and this crate's own suite writes a lower one into a live cursor nine times. | mechanism E1 | **misleading doc** | [invariant/001](../invariant/001_cursor_invariants_over_a_live_ring.md) |
| DB35 | The conservation law is an algebraic identity in the arithmetic that evaluates it, so no cursor corruption of any kind can fail it; the single corrupt input that does not simply pass underflows into a panic in a debug build and wraps back to a pass in a release one. | `check_ends` and D4 | **latent hazard** | [invariant/002](../invariant/002_the_conservation_law_and_why_it_holds.md) |
| DB36 | The family computes free capacity twice, saturating in one crate and by plain subtraction in another, and they diverge exactly on the lapped ring — while the test documenting this crate's central limitation models the function under test with the arithmetic it does not use. | two `free_slots` implementations | n/a — inconsistency | [invariant/002](../invariant/002_the_conservation_law_and_why_it_holds.md) |
| DB37 | The guard against reporting a consequence instead of a cause is credited to a test whose pair breaks no ordering invariant, so it would pass under the failure it is cited against — while the test that does discriminate is listed two clauses earlier under a different heading. | failure mode M3 | **misleading doc** | [lifecycle/001](../lifecycle/001_from_one_observation_to_a_sequence.md) |
| DB38 | The recovery transition, defended at greater length than any other and carrying the whole of the no-latch decision, is the one transition no test exercises; a change making the failed state absorbing would pass the entire suite. | transition T6 | n/a — coverage | [lifecycle/001](../lifecycle/001_from_one_observation_to_a_sequence.md) |
| DB39 | The implementation task sits in the state meaning "not yet scoped" and contains, twenty lines lower, a 6/6 gate verdict and a satisfied acceptance criterion — both halves accurate, because the work went through the staged run instead and all 33 ring tasks are in the same position. | task 126's state | n/a — inconsistency | [lifecycle/002](../lifecycle/002_a_finished_crate_in_the_first_stage.md) |
| DB40 | The prose inventory added so the task file would not understate the crate now understates it, naming a definition directory that has since been renamed away and omitting seven that exist — a hand-written count in a file on no regeneration path. | the Implementation Record | n/a — drift | [lifecycle/002](../lifecycle/002_a_finished_crate_in_the_first_stage.md) |
| DB41 | The consequence offered as evidence for the design — that every test asserts on a whole `Violation` — holds for five of the suite's twenty-six, and the precise claim it should have made is stronger than the loose one. | the value-carrying argument | **misleading doc** | [type/001](../type/001_violation.md) |
| DB42 | The guard against a rendering that throws the numbers away cannot detect a rendering that keeps every number and exchanges two of them; all four cases survive an operand swap, including the one whose entire diagnostic value is direction. | the `Display` fixture | n/a — coverage | [type/001](../type/001_violation.md) |
| DB43 | The only type holding a baseline derived `Copy`, so a by-value use duplicated it silently instead of moving it and the caller stopped checking the one invariant a watch exists to add; the derive is gone and a by-value use is a move again. | `Copy` on `Watch` | **latent hazard** | [type/002](../type/002_watch_and_the_traits_nothing_asks_for.md) |
| DB44 | One trait surface is declared across an inert label, an inert report and a stateful instrument; measured per type it is used on the reports, unused on the instrument, and on the instrument the one derive with a consequence is the one with no call site. | the shared derive line | n/a — observation | [type/002](../type/002_watch_and_the_traits_nothing_asks_for.md) |
| DB45 | The stated per-call cost counts the two acquire loads issued by the three entry points a family caller cannot reach, and none of the loads issued by the one it can — which goes through a backend-dispatching enum and reads each cursor twice, at two different orderings, in a crate this one does not name. | the cost model | **misleading doc** | [non_functional_requirement/001](../non_functional_requirement/001_absent_unless_called.md) |
| DB46 | The block holding the guards for the two constraints the requirement calls mechanically checkable is fenced `bash`, and the corpus recipe checker executes `sh` fences only — so the crate's own enforcement commands are the one recipe in its docs that nothing has ever run. | the bash fence | n/a — unenforced | [non_functional_requirement/001](../non_functional_requirement/001_absent_unless_called.md) |
| DB47 | The requirement records one open question as having nothing that greps for it, while stage M3 of the manual plan in the same crate runs exactly that grep and its Run Record carries a dated passing result. | Q4's "no mechanical guard" | **misleading doc** | [non_functional_requirement/002](../non_functional_requirement/002_the_constraints_with_no_number.md) |
| DB48 | The latency figure was deferred to a crate this one has no dependency edge to, and the allocation claim has nothing in the crate able to falsify it, so neither has a route to a number. | the two claims with no number | n/a — doc gap | [non_functional_requirement/002](../non_functional_requirement/002_the_constraints_with_no_number.md) |
| DB49 | The branch a defensive `distance_to` would add is rejected as a cost on "the gating read that every claim performs", and neither of the family's two in-house backends takes a `ring_seqno` edge at all — each computes its own headroom in a crate that never reaches the line being defended. | the cost P4 defends against | **misleading doc** | [pitfall/001](../pitfall/001_saturating_arithmetic_reports_health.md) |
| DB50 | The masking is attributed to one `saturating_sub` in another crate; a second one sits in `ring_seqno::free_slots` itself and is the clamp that fires on the other corruption, making two of that corruption's three readings identical to a legitimately full ring. | the asymmetry's second clamp | **misleading doc** | [pitfall/001](../pitfall/001_saturating_arithmetic_reports_health.md) |
| DB51 | The only check reachable from a live ring compares against a capacity the caller cannot derive from either value it passes — the object holding it is mutably borrowed for as long as those values exist — so it has to be captured before the split, which the rustdoc now shows and three of the suite's four call sites now do. | `check_ends`'s capacity argument | **latent hazard** | [pitfall/002](../pitfall/002_the_number_the_borrow_checker_hides.md) |
| DB52 | One construction site reports both a genuinely inconsistent ring and a mistyped third argument in the same sentence about the ring, and the only cause the suite ever produces is the second one. | `ReadingsDisagree`'s two causes | n/a — coverage | [pitfall/002](../pitfall/002_the_number_the_borrow_checker_hides.md) |
