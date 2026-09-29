# Doc Definitions

Module Index for `ring_bench` — every doc definition this crate declares, every
instance under each, every architecture decision, and every finding, in one place.

## Master Doc Definitions Table

| Type | Purpose | Master File | Instances | Findings |
|------|---------|-------------|----------:|---------:|
| `algorithm/` | How one workload becomes six outcomes, and how a verdict is picked from them | [algorithm/readme.md](../algorithm/readme.md) | 2 | 4 |
| `api/` | The public surface: what runs a comparison, and what renders one | [api/readme.md](../api/readme.md) | 2 | 4 |
| `data_structure/` | The workload description, and the three counts a run produces | [data_structure/readme.md](../data_structure/readme.md) | 2 | 4 |
| `decisions/` | Choices with live alternatives — the candidate set, and what is never asserted | [decisions/readme.md](../decisions/readme.md) | 2 | 4 |
| `integration/` | Which crates this one reaches, and the three edges the Contract could not carry | [integration/readme.md](../integration/readme.md) | 2 | 4 |
| `invariant/` | The count ordering, and the placement of the counters relative to the clock | [invariant/readme.md](../invariant/readme.md) | 2 | 5 |
| `item/` | Seven nouns and forty verbs as a set, where twenty-five accessors and one false doc sentence become visible | [item/readme.md](../item/readme.md) | 2 | 4 |
| `lifecycle/` | Seven phases from a description to a verdict, and the six states one candidate occupies inside them | [lifecycle/readme.md](../lifecycle/readme.md) | 2 | 4 |
| `non_functional_requirement/` | Reproducibility, same conditions, and keeping the harness out of the number | [non_functional_requirement/readme.md](../non_functional_requirement/readme.md) | 2 | 4 |
| `pattern/` | The two shaping choices that make the output auditable | [pattern/readme.md](../pattern/readme.md) | 2 | 4 |
| `pitfall/` | Three traps this crate walked into and measured its way out of | [pitfall/readme.md](../pitfall/readme.md) | 3 | 4 |
| `type/` | The candidate enum and the refusal enum | [type/readme.md](../type/readme.md) | 2 | 4 |
| `workaround/` | The two external constraints this crate absorbs, with costs and deletion conditions | [workaround/readme.md](../workaround/readme.md) | 2 | 4 |

**Total: 27 instances and 53 findings across 13 definitions.**

`item/` and `workaround/` were the last two to be filled, and both were argued
against first — `item/` in [`../readme.md`](../readme.md) as a second place for
rustdoc to say the same thing, `workaround/` here as empty on purpose because
everything this crate ran into looked like a *finding about the family* rather
than a constraint to route around. Both arguments were wrong in the same way:
they were about what the directory would restate, and what each found was a
property of a *set* that no single declaration or pitfall can hold.

## Master Doc Instances Table

| Type | ID | Name |
|------|----|------|
| `algorithm/` | 001 | [One Workload Through Six Runners](../algorithm/001_one_workload_through_six_runners.md) |
| `algorithm/` | 002 | [The Eligibility Filter Runs Before the Comparison](../algorithm/002_the_eligibility_filter_runs_before_the_comparison.md) |
| `api/` | 001 | [The Run Surface](../api/001_the_run_surface.md) |
| `api/` | 002 | [The Report Surface](../api/002_the_report_surface.md) |
| `data_structure/` | 001 | [The Workload Description](../data_structure/001_the_workload_description.md) |
| `data_structure/` | 002 | [Three Counts That Are Not Interchangeable](../data_structure/002_three_counts_that_are_not_interchangeable.md) |
| `decisions/` | 001 | [Five Candidates for Four Named Paths](../decisions/001_five_candidates_for_four_named_paths.md) |
| `decisions/` | 002 | [No Test Asserts an Ordering](../decisions/002_no_test_asserts_an_ordering.md) |
| `integration/` | 001 | [Declared Edges and the Three That Were Missing](../integration/001_declared_edges_and_the_three_that_were_missing.md) |
| `integration/` | 002 | [The Only Consumer of Two Contract Names](../integration/002_the_only_consumer_of_two_contract_names.md) |
| `invariant/` | 001 | [Received Never Exceeds Reported Never Exceeds Offered](../invariant/001_received_never_exceeds_reported_never_exceeds_offered.md) |
| `invariant/` | 002 | [The Counters Are Written Outside the Clock](../invariant/002_the_counters_are_written_outside_the_clock.md) |
| `item/` | 001 | [Seven Nouns, and the One That Is Copied but Never Compared](../item/001_seven_nouns_and_the_one_never_compared.md) |
| `item/` | 002 | [Forty Verbs, Twenty-Five of Them `const`](../item/002_forty_verbs_twenty_five_of_them_const.md) |
| `lifecycle/` | 001 | [From a Description to a Verdict](../lifecycle/001_from_a_description_to_a_verdict.md) |
| `lifecycle/` | 002 | [One Candidate Through One Run](../lifecycle/002_one_candidate_through_one_run.md) |
| `non_functional_requirement/` | 001 | [The Comparison Is Reproducible and Same-Conditions](../non_functional_requirement/001_the_comparison_is_reproducible_and_same_conditions.md) |
| `non_functional_requirement/` | 002 | [The Harness Is Not in the Measurement](../non_functional_requirement/002_the_harness_is_not_in_the_measurement.md) |
| `pattern/` | 001 | [The Measurement Is a Value](../pattern/001_the_measurement_is_a_value.md) |
| `pattern/` | 002 | [A Refusal Is a Row](../pattern/002_a_refusal_is_a_row.md) |
| `pitfall/` | 001 | [The Door Caps What the Structure Does Not](../pitfall/001_the_door_caps_what_the_structure_does_not.md) |
| `pitfall/` | 002 | [A Counter Inside the Timed Region Measures Itself](../pitfall/002_a_counter_inside_the_timed_region_measures_itself.md) |
| `pitfall/` | 003 | [`Ok` Is Not Kept, and the Verdict Inverts](../pitfall/003_ok_is_not_kept_and_the_verdict_inverts.md) |
| `type/` | 001 | [Candidate](../type/001_candidate.md) |
| `type/` | 002 | [Run Error](../type/002_run_error.md) |
| `workaround/` | 001 | [The Clock Is the Platform's, and the Tie-Break Is the List](../workaround/001_the_clock_is_the_platforms_and_the_tie_break_is_the_list.md) |
| `workaround/` | 002 | [A Feature That Cannot Be Negated at the Use Site](../workaround/002_a_feature_that_cannot_be_negated_at_the_use_site.md) |

## Architecture Decision Records

| ADR | Rules | Status |
|---|---|---|
| [decisions/001_five_candidates_for_four_named_paths.md](../decisions/001_five_candidates_for_four_named_paths.md) | The candidate set | accepted — and BN16 records that its stated grounds do not hold |
| [decisions/002_no_test_asserts_an_ordering.md](../decisions/002_no_test_asserts_an_ordering.md) | What the suite may assert about speed | accepted — and BN13 records that its one licensed verdict is contradicted by its own table |

Indexed here and in [`decisions/readme.md`](../decisions/readme.md) only, not in
`graph.yml` — `doc_des.rulebook.md` classifies `docs/decisions/` as a
non-doc-definition directory. Both are **accepted and both carry a finding**,
which is the shape to expect from a crate whose output is a verdict: the
decision was right and the reason written down for it was measurable, so the
reason is what failed.

## Reading order

[`pitfall/003`](../pitfall/003_ok_is_not_kept_and_the_verdict_inverts.md) first —
an `Ok` that is not a kept record is what inverted this crate's first verdict,
and nine instances read differently once that is known. Then
[`pattern/001`](../pattern/001_the_measurement_is_a_value.md) for why the
comparison is a value rather than a print,
[`invariant/002`](../invariant/002_the_counters_are_written_outside_the_clock.md)
for the boundary every timing claim rests on, and
[`pitfall/001`](../pitfall/001_the_door_caps_what_the_structure_does_not.md) for
the ceiling that decides how many candidates there are to compare at all.

## Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'definitions declared:        %s\n' "$( ls -d ring_bench/docs/*/ | command grep -cv '/definition/$' )"
printf 'instances on disk:           %s\n' "$( ls ring_bench/docs/*/[0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the instances table: %s\n' "$( command grep -cE '^\| `[a-z_]+/` \| [0-9]{3} \|' ring_bench/docs/definition/readme.md )"
printf 'finding headings on disk:    %s\n' "$( command grep -rhoE '^### BN[0-9]+ — ' ring_bench/docs/*/[0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the findings table:  %s\n' "$( command grep -cE '^\| BN[0-9]+ \|' ring_bench/docs/definition/readme.md )"
```

## Findings

**53 findings — four per definition, plus BN53 as an added fifth for
`invariant/`** — each recorded three times: as a `### BNn` section at the end
of the instance that measured it, as a row in that definition's own
`### Findings Recorded Here` table, and here. This table is the only copy
ordered by ID.

| ID | Finding | Subject | Tier | Where |
|----|---------|---------|------|-------|
| BN1 | `commit_batch` increments its own counter only on a successful push, so `reported` and `received` are the same number by construction and two of the suite's `conserved()` assertions cannot fail for the one candidate everything else is ranked against | `Comparison::conserved` | n/a — coverage | [algorithm/001](../algorithm/001_one_workload_through_six_runners.md) |
| BN2 | The crate's one guarantee about what a record contains is bypassed by two of the six runners, which build the range inline rather than calling it, and asserted by no test either way | `Workload::records_of` | n/a — coverage | [algorithm/001](../algorithm/001_one_workload_through_six_runners.md) |
| BN3 | The whole filter is `received == offered`, so a candidate is rankable only on a workload where nothing was refused, and the crate can therefore rank the uncontended case only | `Comparison::fastest`'s eligibility filter | n/a — observation | [algorithm/002](../algorithm/002_the_eligibility_filter_runs_before_the_comparison.md) |
| BN4 | Every fixture in the crate is all-eligible or all-ineligible — 256 into 4096, 1024 into 4096, 256 into 16 — and both `fastest()` doctests repeat the same split, so the filter has never dropped one candidate while keeping another | `Comparison::fastest`'s eligibility filter | n/a — coverage | [algorithm/002](../algorithm/002_the_eligibility_filter_runs_before_the_comparison.md) |
| BN5 | "Accounted for exactly once" is checked as `outcomes.len() + refusals.len() == ALL.len()`, which holds at any split and is therefore weaker than the sentence it carries | the Compatibility Guarantee's accounting claim | n/a — coverage | [api/001](../api/001_the_run_surface.md) |
| BN6 | `RunError::Flush` is documented as unreachable because the staging buffer's capacity and the flush trigger are the same number, and the test named for that tie compares two numbers neither side reads | the unreachable `Flush` variant, and the test named for it | n/a — coverage | [api/001](../api/001_the_run_surface.md) |
| BN7 | `fastest()` filters before it minimises and `report()` does not filter at all, so the eligibility rule the crate's ranking depends on reaches only the report's last line | `Comparison::report` | **misleading doc** | [api/002](../api/002_the_report_surface.md) |
| BN8 | The header reads two of its five fields through different paths, so it can print a batch and a capacity that `RingConfig` would refuse to construct together | `Comparison::report`'s header line | **latent hazard** | [api/002](../api/002_the_report_surface.md) |
| BN9 | The document says `batch` is tied the same way as capacity; the two setters disagree, and both tests that assert the tie were handed the one fixture where it happens to hold | `Workload::with_batch` | **latent hazard** | [data_structure/001](../data_structure/001_the_workload_description.md) |
| BN10 | The section claims both duplicate representations are closed by making the second unreachable; the second is reachable, and nothing in the family reads it | the two fields called "producers" | **wrong doc** | [data_structure/001](../data_structure/001_the_workload_description.md) |
| BN11 | Two of the three counters feeding `RingStats`'s one derived reading are handed the identical `received` expression, so the in-flight count is structurally zero and the assertion that pins it cannot fail | `run`'s four `stats.record_*` writes | **latent hazard** | [data_structure/002](../data_structure/002_three_counts_that_are_not_interchangeable.md) |
| BN12 | `record_drop` buckets every loss by overflow policy, and the mutex baseline — which has no policy and never consults the one it is handed — files its drops under it anyway | `RingStats::record_drop` | n/a — inconsistency | [data_structure/002](../data_structure/002_three_counts_that_are_not_interchangeable.md) |
| BN13 | "The lock-free paths beat the mutex baseline by 3–5x with no overlap in any round" is the single ordering claim this decision says is safe to state, and its own evidence table disagrees | `decisions/002`'s licensed ordering verdict | **wrong doc** | [decisions/002](../decisions/002_no_test_asserts_an_ordering.md) |
| BN14 | Every recorded round names `off_the_shelf`, which exists only under `--features crossbeam`, so the variance evidence was taken in a build the suite does not run | `decisions/002`'s B3 variance rounds | **misleading doc** | [decisions/002](../decisions/002_no_test_asserts_an_ordering.md) |
| BN15 | This decision's entire evidentiary base is fenced ```bash and the corpus recipe checker reads only ```sh, so the premise is re-run by nothing | `decisions/001`'s *What forced it* block | n/a — unenforced | [decisions/001](../decisions/001_five_candidates_for_four_named_paths.md) |
| BN16 | `DirectSpsc` exists on the stated grounds that it and `ContractRing` differ only in the layers between the caller and the same data structure, and this crate's own algorithm document records that they do not | `decisions/001`'s grounds for `DirectSpsc` | **wrong doc** | [decisions/001](../decisions/001_five_candidates_for_four_named_paths.md) |
| BN17 | The only cross-crate test citation in this crate has to be written in the two-column File/Relationship schema, which is the one schema `citations.py` cannot read, so it is checked by nothing | `integration/001`'s `### Tests` table | n/a — unenforced | [integration/001](../integration/001_declared_edges_and_the_three_that_were_missing.md) |
| BN18 | The Compatibility Requirements table makes `Display` and `Error` the precondition for wrapping a dependency's error; both impls landed, and neither implements `source`, so nothing chains | `impl core::error::Error for RunError` | n/a — doc gap | [integration/001](../integration/001_declared_edges_and_the_three_that_were_missing.md) |
| BN19 | Two of the five names on the export Contract have exactly one consumer and it is this crate, while `ring_types` on the same Contract is taken by thirty | the five exported names' consumer counts | n/a — observation | [integration/002](../integration/002_the_only_consumer_of_two_contract_names.md) |
| BN20 | `ring_handle` supplies the type every Contract candidate is built through, and appears nowhere in this crate's manifest or code | `ring_handle` | n/a — doc gap | [integration/002](../integration/002_the_only_consumer_of_two_contract_names.md) |
| BN21 | `reported ≤ offered` is claimed as structurally guaranteed on the grounds that every runner iterates `records_of( i )` exactly once, and two of the six do not iterate it at all | `invariant/001`'s "structurally guaranteed" | **misleading doc** | [invariant/001](../invariant/001_received_never_exceeds_reported_never_exceeds_offered.md) |
| BN22 | Violation Consequences rests on integer underflow panicking in release as well as debug; `overflow-checks` is set nowhere in the workspace, so the subtraction wraps instead | `Outcome::dropped` and `Outcome::silently_discarded` | **latent hazard** | [invariant/001](../invariant/001_received_never_exceeds_reported_never_exceeds_offered.md) |
| BN23 | The invariant warns specifically against candidate-dependent cost inside the clock, and `std::thread::scope` is inside the timed region of exactly the two runners the ranking turns on | `run_mutex_queue` and `run_direct_mpsc`'s timed regions | **measured cost** | [invariant/002](../invariant/002_the_counters_are_written_outside_the_clock.md) |
| BN24 | The boundary is presented as one readable region with a single line marked "← the boundary"; it is six lines in six runner functions, and the four `record_*` writes it is drawn against are in a seventh | `invariant/002`'s Invariant Statement | **misleading doc** | [invariant/002](../invariant/002_the_counters_are_written_outside_the_clock.md) |
| BN25 | Three of `RunError`'s four variants carry no candidate, so a refusal that is not a producer-ceiling refusal cannot be attributed — and the test whose name says the report names every refusal only ever constructs the fourth | `Comparison::refusals` | **latent hazard** | [item/001](../item/001_seven_nouns_and_the_one_never_compared.md) |
| BN26 | `RunError` is `Copy` only because three other crates' error types are, one of which reserves the right to add variants with `#[ non_exhaustive ]`, and neither side records the coupling | `RunError`'s derive list | **latent hazard** | [item/001](../item/001_seven_nouns_and_the_one_never_compared.md) |
| BN27 | Twenty-five are `const fn` accessors on a finished value; six functions run a candidate and two of those six spawn a thread, so the multi-producer comparison this crate performs is two candidates wide in the default build | the forty verbs | n/a — observation | [item/002](../item/002_forty_verbs_twenty_five_of_them_const.md) |
| BN28 | Its doc says every candidate honours the batch and warns that a batch only one arm observed would measure batching against nothing; two of the five default candidates never read it | `Workload::with_batch` | **misleading doc** | [item/002](../item/002_forty_verbs_twenty_five_of_them_const.md) |
| BN29 | Construction carries a single ✅ with two error variants beside it, which reads as a property every candidate passes through, and half the runners cannot refuse at all — their signatures say so | `lifecycle/001`'s Phase 3 | **misleading doc** | [lifecycle/001](../lifecycle/001_from_a_description_to_a_verdict.md) |
| BN30 | Phase 7 is described as comparative, and what feeds it is a plain sequential loop that measures every candidate exactly once, in declaration order, with no warm-up | `Comparison::run`'s candidate loop | n/a — observation | [lifecycle/001](../lifecycle/001_from_a_description_to_a_verdict.md) |
| BN31 | The table and the transition diagram describe a seven-state, five-edge machine, and six of the seven state names appear nowhere in the crate | `lifecycle/002`'s States table and diagram | n/a — drift | [lifecycle/002](../lifecycle/002_one_candidate_through_one_run.md) |
| BN32 | Four lines apart, this document gives the same edge two different refusal-variant counts | `lifecycle/002`'s `Building → Unbuildable` edge | n/a — inconsistency | [lifecycle/002](../lifecycle/002_one_candidate_through_one_run.md) |
| BN33 | R3's two rows report six candidates at one producer and two run / four refused at four; both counts include `OffTheShelf`, which the default build does not compile, and the table does not say so | `non_functional_requirement/001` R3's measured table | **misleading doc** | [non_functional_requirement/001](../non_functional_requirement/001_the_comparison_is_reproducible_and_same_conditions.md) |
| BN34 | R1's "exact — the same value, not equal values" threshold is met by construction, and the value it shares carries two different batch numbers | `non_functional_requirement/001` R1's shared `Workload` | n/a — observation | [non_functional_requirement/001](../non_functional_requirement/001_the_comparison_is_reproducible_and_same_conditions.md) |
| BN35 | Residue 1 discloses `run_mutex_queue` timing its own `std::thread::scope`, with the right consequence for a reader; a second runner does the identical thing and appears in the section zero times | `non_functional_requirement/002`'s Residue section | n/a — doc gap | [non_functional_requirement/002](../non_functional_requirement/002_the_harness_is_not_in_the_measurement.md) |
| BN36 | The operative threshold is "zero harness bookkeeping that scales with the record count", and Residue 2 discloses bookkeeping that scales with the record count, in the same document that records the status as MET | `non_functional_requirement/002`'s Status and Residue 2 | n/a — inconsistency | [non_functional_requirement/002](../non_functional_requirement/002_the_harness_is_not_in_the_measurement.md) |
| BN37 | The Consequences section states as a fact about the crate that the price of returning a value is that there is no binary; the binary exists, and no document under `docs/` names it outside a findings table | `pattern/001` Consequences, and `examples/comparison.rs` | **wrong doc** | [pattern/001](../pattern/001_the_measurement_is_a_value.md) |
| BN38 | The one place this crate states a verdict prints a fixed literal about lock-free paths winning with no overlap, and the code above it computes a spread over a single candidate | `examples/comparison.rs::stability` | **misleading doc** | [pattern/001](../pattern/001_the_measurement_is_a_value.md) |
| BN39 | The Problem section opens on one refusal count and the Consequences section closes on a different one, about the same run | `pattern/002` Problem vs Consequences | **wrong doc** | [pattern/002](../pattern/002_a_refusal_is_a_row.md) |
| BN40 | The document diagnoses that `outcomes + refusals == ALL` holds at any split and therefore never contradicted the wrong number, and the test it names as this pattern's own is still that assertion | `a_comparison_lists_refusals_rather_than_shortening_the_table` | n/a — coverage | [pattern/002](../pattern/002_a_refusal_is_a_row.md) |
| BN41 | Three sections make three non-agreeing statements about C5, and the test named as its guard pins the very value C5 would falsify | C5, Mitigation 3, and `every_candidate_declares_a_name_and_a_ceiling` | **latent hazard** | [pitfall/001](../pitfall/001_the_door_caps_what_the_structure_does_not.md) |
| BN42 | Mitigation 3 presents two omissions as deliberate because both are asserted; one of the two is `wait_nanos() == 0`, which nothing in the crate can make fail | `the_counters_are_the_runs_own_totals`, and Mitigation 3 | n/a — coverage | [pitfall/002](../pitfall/002_a_counter_inside_the_timed_region_measures_itself.md) |
| BN43 | The Mitigation section's second rejected attempt is per-producer counters summed afterwards, and that is exactly what `reported` is on both threaded runners | `run_mutex_queue` and `run_direct_mpsc`'s `reported` | **misleading doc** | [pitfall/002](../pitfall/002_a_counter_inside_the_timed_region_measures_itself.md) |
| BN44 | The pair of tests is justified on the grounds that either alone would be consistent with the wrong behaviour, and "for every candidate" has one load-bearing iteration in the build the default `cargo test` compiles | `a_failing_policy_closes_the_gap_for_every_candidate` | n/a — coverage | [pitfall/003](../pitfall/003_ok_is_not_kept_and_the_verdict_inverts.md) |
| BN45 | One section states plainly that the candidate count is five or six depending on a feature, the next opens on "four variants" and closes on "four-sixths wrong", and twenty bare cardinals across thirteen documents name no build at all | `type/001` §`producer_ceiling`, and the crate's prose cardinals generally | **misleading doc** | [type/001](../type/001_candidate.md) |
| BN46 | §`Validation` says there is nothing to validate beyond uniqueness, and one `name()` string is a load-bearing identifier in an untested file where a mismatch prints a `usize::MAX` spread and `0.0x` | `Candidate::name`, and §`Validation` | **latent hazard** | [type/001](../type/001_candidate.md) |
| BN47 | §`Rendering` and the test's own doc comment promise that errors chain; the test boxes the one variant that wraps nothing, and no `Error` impl in the family implements `source` | `every_error_renders`, and §`Rendering` | **misleading doc** | [type/002](../type/002_run_error.md) |
| BN48 | The second reason given for the omission is that `#[ non_exhaustive ]` would cost the ability to construct a variant in a test; on an enum it blocks exhaustive matching rather than construction, and `ring_types::RingError` is constructed at 126 test sites across eleven other crates | §`Derives`, `No #[ non_exhaustive ]` | **wrong doc** | [type/002](../type/002_run_error.md) |
| BN49 | Nineteen source mentions, three test mentions and zero assertions — the compensation is exact, and it leaves the crate's headline ranking with the coverage of a value the crate has declared untestable | `Outcome::write_nanos` | n/a — coverage | [workaround/001](../workaround/001_the_clock_is_the_platforms_and_the_tie_break_is_the_list.md) |
| BN50 | A tie in `min_by_key` is resolved by `Candidate::ALL`'s declaration order, whose first entry is the mutex baseline, so a clock too coarse to separate two paths reports the control as winner | `Comparison::fastest` | **latent hazard** | [workaround/001](../workaround/001_the_clock_is_the_platforms_and_the_tie_break_is_the_list.md) |
| BN51 | The doc's summary sentence counts "the four bounded ones" over a table that has three rows unless `--features crossbeam` is passed, and the suite carries no `cfg` that could check it | `Candidate::producer_ceiling` | **misleading doc** | [workaround/002](../workaround/002_a_feature_that_cannot_be_negated_at_the_use_site.md) |
| BN52 | The list is declared twice by hand under opposite `cfg`s with identical doc text, nothing asserts the two agree, and that list is also the undocumented tie-break of BN50 | `Candidate::ALL` | **latent hazard** | [workaround/002](../workaround/002_a_feature_that_cannot_be_negated_at_the_use_site.md) |
| BN53 | BN22 guarded the two `Outcome` accessors that expose `offered - received`; `run` computes the identical subtraction earlier, inline, to feed `record_drop`, and that third call site was not covered by BN22's guard | `run`'s inline `stats.record_drop` computation | **latent hazard** | [invariant/001](../invariant/001_received_never_exceeds_reported_never_exceeds_offered.md) |
