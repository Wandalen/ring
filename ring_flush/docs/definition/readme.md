# Doc Definitions

Module Index for `ring_flush` — every doc definition this crate declares, every
instance under each, every architecture decision, and every finding, in one place.

## Master Doc Definitions Table

| Type | Purpose | Master File | Instances | Findings |
|------|---------|-------------|----------:|---------:|
| `algorithm/` | The per-append decision that must cost nothing, and the five-step sequence that reordering collapsed to three | [algorithm/readme.md](../algorithm/readme.md) | 2 | 4 |
| `api/` | A policy surface and a driver surface, kept apart because one is configuration and the other is a schedule | [api/readme.md](../api/readme.md) | 2 | 4 |
| `data_structure/` | Two pointer widths with no indirection, and the log that exists only because the criterion is negative | [data_structure/readme.md](../data_structure/readme.md) | 2 | 4 |
| `decisions/` | Two questions this crate has the evidence for and not the authority to rule | [decisions/readme.md](../decisions/readme.md) | 2 | 4 |
| `integration/` | Three declared dependencies, eighteen reachable crates, and an unshared position on the export surface | [integration/readme.md](../integration/readme.md) | 2 | 4 |
| `invariant/` | Trigger exclusivity, and the restriction the crate exists to impose | [invariant/readme.md](../invariant/readme.md) | 2 | 4 |
| `item/` | Seven nouns and seventeen verbs, catalogued against the four documents that describe them by role | [item/readme.md](../item/readme.md) | 2 | 4 |
| `lifecycle/` | The consolidation cycle and a policy's own arc, plus the two state machines a flush spans as orthogonal axes | [lifecycle/readme.md](../lifecycle/readme.md) | 4 | 4 |
| `non_functional_requirement/` | This crate's Reached condition, and the cost ceiling the acceptance table does not state | [non_functional_requirement/readme.md](../non_functional_requirement/readme.md) | 2 | 4 |
| `pattern/` | Policy as a value; driven rather than self-firing | [pattern/readme.md](../pattern/readme.md) | 2 | 4 |
| `pitfall/` | A policy that cannot see its own trigger, and two batch sizes that must not diverge | [pitfall/readme.md](../pitfall/readme.md) | 2 | 4 |
| `type/` | Three variants and four outcomes, two of which separate "did not fire" from "fired, found nothing" | [type/readme.md](../type/readme.md) | 2 | 4 |
| `workaround/` | External constraints this crate absorbs, with costs and deletion conditions | [workaround/readme.md](../workaround/readme.md) | 2 | 4 |

## Master Doc Instances Table

| Definition | ID | Name | File |
|------------|----|------|------|
| `algorithm/` | 001 | Evaluating a Policy at an Append | [algorithm/001_evaluating_a_policy_at_an_append.md](../algorithm/001_evaluating_a_policy_at_an_append.md) |
| `algorithm/` | 002 | Sequencing Seal, Drain and Reset | [algorithm/002_sequencing_seal_drain_reset.md](../algorithm/002_sequencing_seal_drain_reset.md) |
| `api/` | 001 | The Policy Surface | [api/001_the_policy_surface.md](../api/001_the_policy_surface.md) |
| `api/` | 002 | The Driver Surface | [api/002_the_driver_surface.md](../api/002_the_driver_surface.md) |
| `data_structure/` | 001 | The Policy Enum | [data_structure/001_the_policy_enum.md](../data_structure/001_the_policy_enum.md) |
| `data_structure/` | 002 | The Flush Log | [data_structure/002_the_flush_log.md](../data_structure/002_the_flush_log.md) |
| `decisions/` | 001 | Whether `FlushPolicy` Is `#[non_exhaustive]` | [decisions/001_whether_the_policy_enum_is_non_exhaustive.md](../decisions/001_whether_the_policy_enum_is_non_exhaustive.md) |
| `decisions/` | 002 | `drain_final`'s Signature | [decisions/002_the_final_drains_signature.md](../decisions/002_the_final_drains_signature.md) |
| `integration/` | 001 | Two Dependencies and the Barrier It Cannot See | [integration/001_two_dependencies_and_the_barrier_it_cannot_see.md](../integration/001_two_dependencies_and_the_barrier_it_cannot_see.md) |
| `integration/` | 002 | A Decision on the Export Surface | [integration/002_a_decision_on_the_export_surface.md](../integration/002_a_decision_on_the_export_surface.md) |
| `invariant/` | 001 | A Policy Fires Only at Its Trigger | [invariant/001_a_policy_fires_only_at_its_trigger.md](../invariant/001_a_policy_fires_only_at_its_trigger.md) |
| `invariant/` | 002 | The Publication Point Is Designed, Not Inherited | [invariant/002_publication_point_is_designed_not_inherited.md](../invariant/002_publication_point_is_designed_not_inherited.md) |
| `item/` | 001 | Seven Nouns, Thirteen Variants, and One Measured Width | [item/001_seven_nouns_thirteen_variants.md](../item/001_seven_nouns_thirteen_variants.md) |
| `item/` | 002 | Seventeen Verbs That Never Touch the Producer | [item/002_seventeen_verbs_that_never_touch_the_producer.md](../item/002_seventeen_verbs_that_never_touch_the_producer.md) |
| `lifecycle/` | 001 | The Consolidation Cycle | [lifecycle/001_the_consolidation_cycle.md](../lifecycle/001_the_consolidation_cycle.md) |
| `lifecycle/` | 002 | From Configuration to the Final Drain | [lifecycle/002_from_configuration_to_the_final_drain.md](../lifecycle/002_from_configuration_to_the_final_drain.md) |
| `lifecycle/` | 003 | Buffer State Through a Flush | [lifecycle/003_buffer_state_through_a_flush.md](../lifecycle/003_buffer_state_through_a_flush.md) |
| `lifecycle/` | 004 | Policy Arming and Firing | [lifecycle/004_policy_arming_and_firing.md](../lifecycle/004_policy_arming_and_firing.md) |
| `non_functional_requirement/` | 001 | Three Triggers, Proven by a Recorded Flush Log | [non_functional_requirement/001_three_triggers_proven_by_a_flush_log.md](../non_functional_requirement/001_three_triggers_proven_by_a_flush_log.md) |
| `non_functional_requirement/` | 002 | The Decision Costs Nothing on the Append Path | [non_functional_requirement/002_the_decision_costs_nothing_on_the_append_path.md](../non_functional_requirement/002_the_decision_costs_nothing_on_the_append_path.md) |
| `pattern/` | 001 | Policy as a Value, Not a Call Site | [pattern/001_policy_as_a_value.md](../pattern/001_policy_as_a_value.md) |
| `pattern/` | 002 | Driven, Not Self-Firing | [pattern/002_driven_not_self_firing.md](../pattern/002_driven_not_self_firing.md) |
| `pitfall/` | 001 | `OnBarrier` Cannot See the Barrier | [pitfall/001_on_barrier_cannot_see_the_barrier.md](../pitfall/001_on_barrier_cannot_see_the_barrier.md) |
| `pitfall/` | 002 | Two Batch Sizes That Must Not Diverge | [pitfall/002_two_batch_sizes_that_must_not_diverge.md](../pitfall/002_two_batch_sizes_that_must_not_diverge.md) |
| `type/` | 001 | Flush Policy | [type/001_flush_policy.md](../type/001_flush_policy.md) |
| `type/` | 002 | Flush Outcome | [type/002_flush_outcome.md](../type/002_flush_outcome.md) |
| `workaround/` | 001 | The Obligation That Binds Nobody | [workaround/001_the_obligation_that_binds_nobody.md](../workaround/001_the_obligation_that_binds_nobody.md) |
| `workaround/` | 002 | The Compilation Boundary That Was Never Built | [workaround/002_the_compilation_boundary_that_was_never_built.md](../workaround/002_the_compilation_boundary_that_was_never_built.md) |

**28 instances across all thirteen declared definitions**, none of them a
readme-only directory. That is the shape the corpus standard asks for and it is
not the shape this crate started with: `decisions/`, `item/` and `workaround/`
each held a readme and nothing else, and `item/` had a written justification for
staying that way.

## Architecture Decision Records

| ADR | Rules | Status |
|---|---|---|
| [decisions/001_whether_the_policy_enum_is_non_exhaustive.md](../decisions/001_whether_the_policy_enum_is_non_exhaustive.md) | Pending 3 | open — evidence settled, ruling is family-grain |
| [decisions/002_the_final_drains_signature.md](../decisions/002_the_final_drains_signature.md) | Pending 4 | open — and now more expensive than when raised |

Indexed here and in [`decisions/readme.md`](../decisions/readme.md) only, not in
`graph.yml` — `doc_des.rulebook.md` classifies `docs/decisions/` as a
non-doc-definition directory. Both are filed **open**: each records what a
ruling would cost rather than making one, because both questions are wider than
this crate and FL13/FL14 measure the sibling paying the larger share.

## Reading order

[`pattern/001`](../pattern/001_policy_as_a_value.md) first — a policy is a value
here, not a call site, and four instances read differently once that is known.
Then [`pattern/002`](../pattern/002_driven_not_self_firing.md) for who fires it,
[`api/002`](../api/002_the_driver_surface.md) for the three verbs that do the
firing, and [`invariant/001`](../invariant/001_a_policy_fires_only_at_its_trigger.md)
for the one property all of it exists to hold.

## Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'definitions declared:        %s\n' "$( ls -d ring_flush/docs/*/ | command grep -cv '/definition/$' )"
printf 'instances on disk:           %s\n' "$( ls ring_flush/docs/*/[0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the instances table: %s\n' "$( command grep -cE '^\| `[a-z_]+/` \| [0-9]{3} \|' ring_flush/docs/definition/readme.md )"
printf 'finding headings on disk:    %s\n' "$( command grep -rhoE '^### FL[0-9]+ — ' ring_flush/docs/*/[0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the findings table:  %s\n' "$( command grep -cE '^\| FL[0-9]+ \|' ring_flush/docs/definition/readme.md )"
```

## Findings

**52 findings, four per definition**, each recorded three times: as a `### FLn`
section at the end of the instance that measured it, as a row in that
definition's own `### Findings Recorded Here` table, and here. This table is the
only copy ordered by ID.

| ID | Finding | Subject | Tier | Where |
|----|---------|---------|------|-------|
| FL1 | The path this procedure takes when the ring refuses is documented as returning the record to the caller, and `ring_tls::push` early-returns before pushing, so the record is dropped | the refusal path | **latent hazard** | [algorithm/001](../algorithm/001_evaluating_a_policy_at_an_append.md) |
| FL2 | The publication-ordering criterion this sequence is written against lives in a feature the instance does not cite, and the crate that owns it records its own half as unenforced | the ordering criterion | n/a — unenforced | [algorithm/002](../algorithm/002_sequencing_seal_drain_reset.md) |
| FL3 | O6 is not merely untested in this crate: the tick-path guard it defers to names only three crates, so thirty of the family are unexamined by it | O6 | n/a — coverage | [algorithm/002](../algorithm/002_sequencing_seal_drain_reset.md) |
| FL4 | `run`'s comment says the shortfall requires violating this crate's contract, and that contract is stated nowhere but inside `run` itself | `run` | n/a — doc gap | [algorithm/002](../algorithm/002_sequencing_seal_drain_reset.md) |
| FL5 | Guarantee 3 promises what the export surface confines, and `ring_bench` breached it and recorded the breach in its own manifest rather than against the guarantee | Compatibility Guarantee 3 | **wrong doc** | [api/001](../api/001_the_policy_surface.md) |
| FL6 | Guarantee 1 rules the `#[non_exhaustive]` question closed, `decisions/readme.md` records it open as P3, and `integration/002`'s X1 states it as an unresolved disjunction | Guarantee 1 | n/a — inconsistency | [api/001](../api/001_the_policy_surface.md) |
| FL7 | The claim that no operation on this surface returns `Result` is written seventy lines below the constructor row that returns one | the no-Result claim | **misleading doc** | [api/002](../api/002_the_driver_surface.md) |
| FL8 | The row deferring to an open decision points at P5, which closed by measurement and now has two tests pinning its answer | the deferral row | n/a — drift | [api/002](../api/002_the_driver_surface.md) |
| FL9 | Eight of this crate's instances carry a `### State Machines` heading and no crate in the family has a `state_machine/` definition; thirty-two have `lifecycle/` | the cross-reference headings | n/a — inconsistency | [data_structure/001](../data_structure/001_the_policy_enum.md) |
| FL10 | The snippet offered under "verify rather than trust" asserts a literal sixteen bytes and the assertion that shipped asserts two pointer widths | the width snippet | **wrong doc** | [data_structure/001](../data_structure/001_the_policy_enum.md) |
| FL11 | The section opens by retracting its own framing and keeps the bolded requirement that framing produced, so fifty-five lines argue for a boundary three lines say was never built | the compilation-boundary section | n/a — drift | [data_structure/002](../data_structure/002_the_flush_log.md) |
| FL12 | Twenty-two of this crate's instances use a `| File |` Tests table the citation checker cannot parse and four use the `| Test |` form it reads, so most citations are checked by nothing | the Tests table | n/a — coverage | [data_structure/002](../data_structure/002_the_flush_log.md) |
| FL13 | The family has answered the enum-evolution question twice in opposite directions for good reasons, and neither answer is in a decision record | the family precedent | n/a — doc gap | [decisions/001](../decisions/001_whether_the_policy_enum_is_non_exhaustive.md) |
| FL14 | Marking this crate's four enums costs nothing; marking `OverflowPolicy` forces a wildcard into four functions across two crates that do not declare it | the cost of ruling | **measured cost** | [decisions/001](../decisions/001_whether_the_policy_enum_is_non_exhaustive.md) |
| FL15 | Three driver methods share one `&mut self -> FlushOutcome` shape and only one of them carries a cardinality, which the type system cannot express | the driver signatures | n/a — observation | [decisions/002](../decisions/002_the_final_drains_signature.md) |
| FL16 | Pending 5 said the pair should be ruled together, was closed alone by measurement, and the two tests written to pin it are what a consuming signature would delete | the split pair | n/a — drift | [decisions/002](../decisions/002_the_final_drains_signature.md) |
| FL17 | The block under "confirm rather than take it on trust" is a typed manifest excerpt showing two dependencies, not a capture, and the manifest declares three | the dependency block | **misleading doc** | [integration/001](../integration/001_two_dependencies_and_the_barrier_it_cannot_see.md) |
| FL18 | Two documents recorded the same orphaned-crate finding independently, and only the one with routing authority could act; neither cites the other | the orphan finding | n/a — observation | [integration/001](../integration/001_two_dependencies_and_the_barrier_it_cannot_see.md) |
| FL19 | The G5 mechanism quoted here was replaced by the exact-match roster the script now uses, which is the thing its own header argues a path test gets wrong | the quoted gate mechanism | n/a — drift | [integration/002](../integration/002_a_decision_on_the_export_surface.md) |
| FL20 | `Flusher::new` takes a `ring_core::Producer`, so the family's own benchmark cannot be built without naming a crate the declared five-crate surface excludes | the export surface | n/a — inconsistency | [integration/002](../integration/002_a_decision_on_the_export_surface.md) |
| FL21 | The reasoning deleting V2 rests on a fault injection run against a suite of twenty-three tests, and the suite now has thirty-five | the V2 deletion | n/a — drift | [invariant/001](../invariant/001_a_policy_fires_only_at_its_trigger.md) |
| FL22 | Seven of this crate's instances record the ownership half-close and the two rows about that hole are not among them | the ownership half-close | n/a — coverage | [invariant/001](../invariant/001_a_policy_fires_only_at_its_trigger.md) |
| FL23 | The gate directory grew from six gates to nineteen and the one gate this invariant specified is still absent; no gate names `ring_tls` | the P5 gate | n/a — unenforced | [invariant/002](../invariant/002_publication_point_is_designed_not_inherited.md) |
| FL24 | C5 is written as a hazard and the method it describes is public, documented, and exercised fourteen times in its own crate's tests | C5 | **misleading doc** | [invariant/002](../invariant/002_publication_point_is_designed_not_inherited.md) |
| FL25 | `tests/manual/readme.md` measured four types and the suite asserts one of them, so three recorded widths would survive a layout change with nothing going red | the four measured widths | n/a — coverage | [item/001](../item/001_seven_nouns_thirteen_variants.md) |
| FL26 | Five of the seven nouns carry an identical five-trait derive list; the two that differ are the two with state, and the only hand-written impls in the crate belong to a noun that has none | `ConfigError` | n/a — observation | [item/001](../item/001_seven_nouns_thirteen_variants.md) |
| FL27 | Seven of the seventeen forward a single call into a container this crate does not own, and the containers are two different crates' — `ring_tls`'s buffer and the standard library's `Vec` | the seventeen verbs | n/a — observation | [item/002](../item/002_seventeen_verbs_that_never_touch_the_producer.md) |
| FL28 | The one verb whose stated contract is "called once, at teardown" has a `&mut self` signature that permits any number of calls, and the crate's own test suite exercises the second call and asserts it succeeds | `Flusher::drain_final` | **latent hazard** | [item/002](../item/002_seventeen_verbs_that_never_touch_the_producer.md) |
| FL29 | U3 claims no allocation and the test it cites observes only whether a log exists; four sibling crates ship a counting allocator and this crate has not adopted one | U3 | **misleading doc** | [lifecycle/001](../lifecycle/001_the_consolidation_cycle.md) |
| FL30 | A question closed by measurement in the decisions table is still recorded open in all three places it was asked | the reuse question | n/a — drift | [lifecycle/002](../lifecycle/002_from_configuration_to_the_final_drain.md) |
| FL31 | The tally counts two invariants, names three, and omits the one whose justification rests on two transitions the same instance calls unreachable | the invariant tally | **wrong doc** | [lifecycle/003](../lifecycle/003_buffer_state_through_a_flush.md) |
| FL32 | Four corpus checkers were added since this instance was written and all eighteen verdicts they can emit are structural; none opens a second document to compare claims | the corpus checkers | n/a — coverage | [lifecycle/004](../lifecycle/004_policy_arming_and_firing.md) |
| FL33 | The gate said to record a crate-to-feature edge searches every family crate's tests with no file-type filter, so prose satisfies it and the crate is not part of the tuple | the citation rule | **misleading doc** | [non_functional_requirement/001](../non_functional_requirement/001_three_triggers_proven_by_a_flush_log.md) |
| FL34 | M4's threshold cannot be missed: the crate's only `FlushEntry` literal derives the entry from the outcome, so the agreement it measures is guaranteed by construction | M4 | n/a — unenforced | [non_functional_requirement/001](../non_functional_requirement/001_three_triggers_proven_by_a_flush_log.md) |
| FL35 | The block introduced by "confirm it rather than taking it on trust" is fenced so the gate never runs it, and describes a scratch workspace nothing creates | the cycle-check block | n/a — coverage | [non_functional_requirement/002](../non_functional_requirement/002_the_decision_costs_nothing_on_the_append_path.md) |
| FL36 | This instance and the proxy test it produced both cite a four-name unsafe allowlist that was later replaced with three, precisely because the four were inert | the unsafe allowlist | n/a — drift | [non_functional_requirement/002](../non_functional_requirement/002_the_decision_costs_nothing_on_the_append_path.md) |
| FL37 | "Exactly one, by construction" holds on the single-producer backend and is a convention on the other two, where `try_clone` hands out a second producer | the comparison table | **misleading doc** | [pattern/001](../pattern/001_policy_as_a_value.md) |
| FL38 | The table's sharpest boundary — do not make a correctness requirement configurable — is crossed by the enum it was written for, since `OnBarrier` is an obligation and the other two are trades | the applicability boundary | n/a — doc gap | [pattern/001](../pattern/001_policy_as_a_value.md) |
| FL39 | The concession that `ring_barrier` is in the transitive closure is false: eighteen crates are reachable and it is not among them, nor named anywhere in this crate | the closure concession | **wrong doc** | [pattern/002](../pattern/002_driven_not_self_firing.md) |
| FL40 | R5 calls the forgotten-driver failure indistinguishable from an idle buffer, and `staged()` is a public accessor returning exactly the number that distinguishes them | R5 | n/a — diagnostics | [pattern/002](../pattern/002_driven_not_self_firing.md) |
| FL41 | The instance that names closure counts perishable ships two perished ones, behind the fence type that guarantees the gate never reruns its regeneration recipe | the closure recipe | n/a — drift | [pitfall/001](../pitfall/001_on_barrier_cannot_see_the_barrier.md) |
| FL42 | Mitigation 4 claims this instance is cited from the crate root; the crate root cites four of this crate's instances and neither pitfall is among them | Mitigation 4 | n/a — doc gap | [pitfall/001](../pitfall/001_on_barrier_cannot_see_the_barrier.md) |
| FL43 | `ring_batch` owns no claim width — `claim` takes it as a per-call argument and the crate declares no constant, so the coupling's other half is in call-site form | the claim width | **wrong doc** | [pitfall/002](../pitfall/002_two_batch_sizes_that_must_not_diverge.md) |
| FL44 | The only benchmark spends one scalar as buffer capacity, ring config batch and `OnBatch`'s `n`, which makes `OnBatch` and `OnFull` fire at identical instants | the benchmark binding | **measured cost** | [pitfall/002](../pitfall/002_two_batch_sizes_that_must_not_diverge.md) |
| FL45 | Six types carry the same five-derive line and the table reads it as five rulings about this one; the only departure derives the trait the table calls indefensible | the trait table | n/a — observation | [type/001](../type/001_flush_policy.md) |
| FL46 | The trait table marks `serde` open and points at a decisions record whose matching section is titled "one question deliberately not recorded here" | the serde row | n/a — inconsistency | [type/001](../type/001_flush_policy.md) |
| FL47 | M5 recommends deriving the log entry from the outcome as future work and `record` already does it, in the words the source's own comment uses | M5 | n/a — drift | [type/002](../type/002_flush_outcome.md) |
| FL48 | M1's enforcement column reads "Construction" and `count` comes from `try_push_batch`, which returns zero when the first push is refused — the path the source comment names | M1 | **misleading doc** | [type/002](../type/002_flush_outcome.md) |
| FL49 | The outcome type carries the obligation for three driver methods and the builder returns `Self` unguarded, so the W1 failure mode is reachable in one statement | the `#[must_use]` sites | n/a — unenforced | [workaround/001](../workaround/001_the_obligation_that_binds_nobody.md) |
| FL50 | The loss W1 calls unpreventable is pinned by a passing test, which turns a documented hazard into a contract a well-meaning `Drop` would have to break | the drop test | n/a — observation | [workaround/001](../workaround/001_the_obligation_that_binds_nobody.md) |
| FL51 | W2's compensation names a cargo feature the crate never grew; four family manifests carry a `[features]` section and none declares it | the compensation column | n/a — drift | [workaround/002](../workaround/002_the_compilation_boundary_that_was_never_built.md) |
| FL52 | The recipe that settles the dependency count sits one line below the wrong count, fenced so the gate never runs it and never reports it missing | the readme recipe | n/a — coverage | [workaround/002](../workaround/002_the_compilation_boundary_that_was_never_built.md) |
