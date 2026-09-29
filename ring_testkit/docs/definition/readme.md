# Doc Definitions

Module Index for `ring_testkit` — every doc definition this crate declares, every
instance under each, every architecture decision, and every finding, in one place.

## Master Doc Definitions Table

| Type | Purpose | Master File | Instances | Findings |
|------|---------|-------------|----------:|---------:|
| `algorithm/` | How ten step kinds become ten fields, and the four passes that check them | [algorithm/readme.md](../algorithm/readme.md) | 2 | 4 |
| `api/` | The eight public declarations and what each promises, and the surface no crate has taken | [api/readme.md](../api/readme.md) | 2 | 4 |
| `data_structure/` | A script as a flat list with no nesting, and ten fields one of which means two things | [data_structure/readme.md](../data_structure/readme.md) | 2 | 4 |
| `decisions/` | Why records are `u32`, and why the loom model lives in `tests/` rather than `src/` | [decisions/readme.md](../decisions/readme.md) | 2 | 4 |
| `integration/` | The three manifest edges and the fourth nobody drew, and the edge only a cfg creates | [integration/readme.md](../integration/readme.md) | 2 | 4 |
| `invariant/` | Every minted record accounted somewhere, and the shape a delivered list must have | [invariant/readme.md](../invariant/readme.md) | 2 | 4 |
| `item/` | Two components meeting in no line of `src/`, and the attributes the crate declines to declare | [item/readme.md](../item/readme.md) | 2 | 4 |
| `lifecycle/` | A script's states from built to run, and the object whose lifecycle has no end | [lifecycle/readme.md](../lifecycle/readme.md) | 2 | 4 |
| `non_functional_requirement/` | Determinism as a testable property, and what a fixture owes consumers it does not have | [non_functional_requirement/readme.md](../non_functional_requirement/readme.md) | 2 | 4 |
| `pattern/` | The script as data with the run as its interpreter, and returning a verdict instead of making one | [pattern/readme.md](../pattern/readme.md) | 2 | 4 |
| `pitfall/` | Three traps found by building — a reading, a step, and a join two crates cannot make | [pitfall/readme.md](../pitfall/readme.md) | 3 | 6 |
| `type/` | What the four values mean, and the three paths by which one comes into existence | [type/readme.md](../type/readme.md) | 2 | 4 |
| `workaround/` | The two allocations loom forces, and the suite its cfg splits in half | [workaround/readme.md](../workaround/readme.md) | 2 | 4 |

**Total: 27 instances and 54 findings across 13 definitions.**

The crate is 741 lines of `src/` with eight public declarations, and the corpus
above is larger than the crate. That is the shape to expect for a fixture: its
subject is what *other* crates do when driven, so most instances end up measuring
something outside `src/lib.rs` — the two constructors `ring_core` offers on the
same type, the decision log `ring_shutdown` never opened, the amortised path in
`ring_tls` that no ring can reach, and the cfg no automated command in this
repository sets.

## Master Doc Instances Table

| Type | ID | Name |
|------|----|------|
| `algorithm/` | 001 | [From A Step To An Outcome](../algorithm/001_from_a_step_to_an_outcome.md) |
| `algorithm/` | 002 | [The Four Passes Of An Audit](../algorithm/002_the_four_passes_of_an_audit.md) |
| `api/` | 001 | [The Script Surface](../api/001_the_script_surface.md) |
| `api/` | 002 | [The Surface No Crate Has Taken](../api/002_the_surface_no_crate_has_taken.md) |
| `data_structure/` | 001 | [The Script As A Flat Step List](../data_structure/001_the_script_as_a_flat_step_list.md) |
| `data_structure/` | 002 | [Nine Counters And A Number That Is Two Things](../data_structure/002_nine_counters_and_a_number_that_is_two_things.md) |
| `decisions/` | 001 | [The Record Type Is A `u32`](../decisions/001_the_record_type_is_a_u32.md) |
| `decisions/` | 002 | [The Model Lives In Tests](../decisions/002_the_model_lives_in_tests.md) |
| `integration/` | 001 | [The Three Edges And The One That Is Missing](../integration/001_the_three_edges_and_the_one_that_is_missing.md) |
| `integration/` | 002 | [The Edge That Only Exists Under A Cfg](../integration/002_the_edge_that_only_exists_under_a_cfg.md) |
| `invariant/` | 001 | [Every Minted Record Is Somewhere](../invariant/001_every_minted_record_is_somewhere.md) |
| `invariant/` | 002 | [The Shape A Delivered List Must Have](../invariant/002_the_shape_a_delivered_list_must_have.md) |
| `item/` | 001 | [Two Components That Meet In No Line Of `src/`](../item/001_two_components_that_meet_in_no_line_of_src.md) |
| `item/` | 002 | [What The Crate Does Not Declare](../item/002_what_the_crate_does_not_declare.md) |
| `lifecycle/` | 001 | [The States A Script Moves Through](../lifecycle/001_the_states_a_script_moves_through.md) |
| `lifecycle/` | 002 | [The Object Whose Lifecycle Has No End](../lifecycle/002_the_object_whose_lifecycle_has_no_end.md) |
| `non_functional_requirement/` | 001 | [Two Runs Compare Equal](../non_functional_requirement/001_two_runs_compare_equal.md) |
| `non_functional_requirement/` | 002 | [What A Fixture Owes Its Consumers](../non_functional_requirement/002_what_a_fixture_owes_its_consumers.md) |
| `pattern/` | 001 | [The Script Is Data And The Run Is An Interpreter](../pattern/001_the_script_is_data_and_the_run_is_an_interpreter.md) |
| `pattern/` | 002 | [The Fixture Returns A Verdict Instead Of Making One](../pattern/002_the_fixture_returns_a_verdict_instead_of_making_one.md) |
| `pitfall/` | 001 | [Neither The Count Nor The List Alone](../pitfall/001_neither_the_count_nor_the_list_alone.md) |
| `pitfall/` | 002 | [Reopening Closes First](../pitfall/002_reopening_closes_first.md) |
| `pitfall/` | 003 | [The Amortised Flush Has No Ring](../pitfall/003_the_amortised_flush_has_no_ring.md) |
| `type/` | 001 | [Outcome And Anomaly](../type/001_outcome_and_anomaly.md) |
| `type/` | 002 | [How These Values Are Built](../type/002_how_these_values_are_built.md) |
| `workaround/` | 001 | [Two Allocations Loom Cannot Avoid](../workaround/001_two_allocations_loom_cannot_avoid.md) |
| `workaround/` | 002 | [A Suite Split In Two By A Global Cfg](../workaround/002_a_suite_split_in_two_by_a_global_cfg.md) |

## Architecture Decision Records

| ADR | Rules | Status |
|---|---|---|
| [decisions/001_the_record_type_is_a_u32.md](../decisions/001_the_record_type_is_a_u32.md) | What a scripted record is, and what it may not be | accepted — and Pending 1 records the generic version as the open question |
| [decisions/002_the_model_lives_in_tests.md](../decisions/002_the_model_lives_in_tests.md) | Where the loom model is allowed to live | accepted — and TK54 records that its stated rationale names only the benefit |

Indexed here and in [`decisions/readme.md`](../decisions/readme.md) only, not in
`graph.yml` — `doc_des.rulebook.md` classifies `docs/decisions/` as a
non-doc-definition directory. The register behind them holds four pending
questions alongside the closed decisions, and three of those four name another
`ring_*` crate as where their answer lives
(→ [`workaround/001`](../workaround/001_two_allocations_loom_cannot_avoid.md)
TK51). The two promoted to instances above are the two that grew a
measurement.

## Reading order

[`pattern/002`](../pattern/002_the_fixture_returns_a_verdict_instead_of_making_one.md)
first — it is the choice everything else follows from: the fixture returns an
`Outcome` and asserts nothing, so every other document is about what that value
says and what it leaves to the caller. Then
[`type/001`](../type/001_outcome_and_anomaly.md) for what the ten fields and
four variants mean,
[`invariant/001`](../invariant/001_every_minted_record_is_somewhere.md) for the
law the audit checks, and
[`pitfall/001`](../pitfall/001_neither_the_count_nor_the_list_alone.md) for the
reading a consumer is otherwise likely to get wrong — under a dropping policy
neither the accepted count nor the delivered list detects a loss on its own, and
the accessor that resolves it has no call site in `src/`.

## Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'definitions declared:        %s\n' "$( ls -d ring_testkit/docs/*/ | command grep -cv '/definition/$' )"
printf 'instances on disk:           %s\n' "$( ls ring_testkit/docs/*/[0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the instances table: %s\n' "$( command grep -cE '^\| `[a-z_]+/` \| [0-9]{3} \|' ring_testkit/docs/definition/readme.md )"
printf 'finding headings on disk:    %s\n' "$( command grep -rhoE '^### TK[0-9]+ — ' ring_testkit/docs/*/[0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the findings table:  %s\n' "$( command grep -cE '^\| TK[0-9]+ \|' ring_testkit/docs/definition/readme.md )"
```

Live output:

```
definitions declared:        13
instances on disk:           27
rows in the instances table: 27
finding headings on disk:    54
rows in the findings table:  54
```

## Findings

**54 findings, four per definition and six for `pitfall/`**, each recorded three
times: as a `### TKn` section at the end of the instance that measured it, as a row
in that definition's own `### Findings Recorded Here` table, and here. This table
is the only copy ordered by ID.

| ID | Finding | Subject | Tier | Where |
|----|---------|---------|------|-------|
| TK1 | The suppression on `Script::run` justifies its length as "one match arm per Step", and the body collapses three pairs of variants into three shared arms — ten variants, seven arms — so the stated shape argument is contradicted by the first three arms of the match it annotates. | the `too_many_lines` reason | **misleading doc** | [algorithm/001](../algorithm/001_from_a_step_to_an_outcome.md) |
| TK2 | Three adjacent arms of one `match` produce the family's proof-of-closure token and drop it, produce and immediately consume it, and produce and use it; the discard is legal and its justification lives in another crate's doc example rather than anywhere in this one. | `Stopped` in three arms | n/a — inconsistency | [algorithm/001](../algorithm/001_from_a_step_to_an_outcome.md) |
| TK3 | The ordering pass rejects any pair that does not strictly ascend — a single-producer property — inside the one function whose doc comment says it exists for the concurrent case, where two producers legitimately interleave and would be reported as an anomaly. | `audit_received`'s ascent check | **latent hazard** | [algorithm/002](../algorithm/002_the_four_passes_of_an_audit.md) |
| TK4 | `audit` reports the earliest violation in *pass* order, not in list order: the provenance scan traverses the whole slice before the ordering scan begins, so a list holding both kinds reports the later-positioned one and the earlier is never reached. | "name the first that does not" | **misleading doc** | [algorithm/002](../algorithm/002_the_four_passes_of_an_audit.md) |
| TK5 | `steps()` and `stage_limit()` have four call sites across every crate in `ring/`, `any crate root` and `the spike root`, all four inside the two tests that assert them; `Script::run` reads both fields directly, so the encapsulation they provide has no consumer inside the crate or outside it. | `Script`'s two accessors | n/a — unadopted | [api/001](../api/001_the_script_surface.md) |
| TK6 | `Outcome` exposes ten bare `pub` fields with no accessors and `Script`, 424 lines later in the same file, exposes none and reaches its two fields through methods; both are defensible and neither is written down, so a type added later has two conventions to choose between. | two encapsulation policies | n/a — inconsistency | [api/001](../api/001_the_script_surface.md) |
| TK7 | Zero manifests in `ring/` declare `ring_testkit` and there are zero `use ring_testkit` statements outside it, while eighteen other crates name it in prose — so the fixtures, the audit and the loom bridge are exercised only by this crate's own 33 tests. | a testkit with no consumer | n/a — unadopted | [api/002](../api/002_the_surface_no_crate_has_taken.md) |
| TK8 | `ring_flush` defers its C2 allocation constraint to this crate as "the one place a counting allocator could be justified", and there is no `#[ global_allocator ]` here — four sibling crates each built their own instead — and no mention of allocation in the feature the routing cites. | `ring_flush`'s routed allocation measurement | n/a — doc gap | [api/002](../api/002_the_surface_no_crate_has_taken.md) |
| TK9 | The front-page example passes `Script::new( 4 )` two lines above `RingConfig::new( 8 )`, and the script contains no `Stage`, `StageMany` or `Flush` step — so the first number a reader of this crate sees sizes a staging buffer nothing writes to, next to the one that actually bounds the run. | the crate doc's first constant | **misleading doc** | [data_structure/001](../data_structure/001_the_script_as_a_flat_step_list.md) |
| TK10 | `PushMany`, `RecvMany` and `StageMany` carry an unchecked `usize` and `run` applies no `min`, assertion or comparison to it, while the algorithm document's termination argument describes each step as "bounded by a constant in the step itself" — a constant the caller supplies with no ceiling. | the `Many` step counts | **latent hazard** | [data_structure/001](../data_structure/001_the_script_as_a_flat_step_list.md) |
| TK11 | The one reading documented as saying *records were destroyed* answers `0` — the healthy value — whenever delivered-plus-held exceeds accepted, which the crate reaches two established ways: a second run on one ring, and the hand-built `Outcome` three tests already construct. | `vanished`'s saturating subtraction | **latent hazard** | [data_structure/002](../data_structure/002_nine_counters_and_a_number_that_is_two_things.md) |
| TK12 | The field is documented only as a count and is also the exclusive upper bound `audit_received` tests every record value against; the two readings coincide solely because minting is consecutive from zero, which is stated on the enum rather than the field and checked nowhere. | `minted`'s two meanings | n/a — observation | [data_structure/002](../data_structure/002_nine_counters_and_a_number_that_is_two_things.md) |
| TK13 | The entry asks whether a `Script` should be generic and argues entirely about minting, while `leak` and `leak_ends` eighty-one lines above `Script::run` are already `< T : Send >` — the crate answered the same question yes for the loom bridge and the entry does not say so. | Pending 1's scope | n/a — observation | [decisions/001](../decisions/001_the_record_type_is_a_u32.md) |
| TK14 | The settlement condition names `T : PartialEq` as the bound a generic script would need, and `audit_received`'s `value >= minted` pass needs a notion of *mintedness* that no standard trait supplies — a design decision the entry does not name among its costs. | Pending 1's stated cost | n/a — doc gap | [decisions/001](../decisions/001_the_record_type_is_a_u32.md) |
| TK15 | The `83/83` the decision is credited with is `src/lib.rs`'s, measured by a tarpaulin invocation filtered to `src`; the 163-line file the decision exists to place is behind `#![ cfg( loom ) ]` and is measured by nothing. | Closed 2's coverage payoff | n/a — coverage | [decisions/002](../decisions/002_the_model_lives_in_tests.md) |
| TK16 | Twenty-four test files in the family carry `#![ cfg( not( loom ) ) ]` because loom's atomics panic outside a model, and three of this crate's four doc examples construct a `Ring` from a comment that no attribute can gate — checked by nothing, since the one stage that sets the cfg runs clippy. | doc examples under the cfg | **latent hazard** | [decisions/002](../decisions/002_the_model_lives_in_tests.md) |
| TK17 | The `ring_shutdown` row explains `wait_for_close`, `for_space_or_close` and `Wake` being unused with "those need a second thread, and a script has one" — true of `Script::run` and not of the crate, which calls `thread::spawn` ten times across three files, twice inside `leak_ends`' own doc example. | the unused waiting surface | **misleading doc** | [integration/001](../integration/001_the_three_edges_and_the_one_that_is_missing.md) |
| TK18 | Evidence G2 cites a clean `cargo +nightly udeps` run, and M4's command carries no `RUSTFLAGS`, so the one dependency declared under `[target.'cfg(loom)']` is absent from the graph the tool walks — it can neither report `loom` unused nor confirm it used. | the citation behind "no fourth edge" | **misleading doc** | [integration/001](../integration/001_the_three_edges_and_the_one_that_is_missing.md) |
| TK19 | `tests/exhaustive_test.rs` is the only file in all thirty-three crates opening `#![ cfg( loom ) ]`, while five manifests declare `loom = "0.7"`; three of the other four never write `loom::` at all, and `001` presents them as four precedents for the shape this crate is in fact the first to use. | the loom dependency's five declarers | n/a — unadopted | [integration/002](../integration/002_the_edge_that_only_exists_under_a_cfg.md) |
| TK20 | The root `Cargo.toml` comment justifying a workspace-wide `check-cfg` says "Only ring_atomic, ring_cursor and ring_publish read the cfg"; the measured populations are 2, 5, 20 and 1, no one of which is that trio, and the crate holding the family's only loom model is named in none of them. | the workspace's record of the cfg's readers | n/a — drift | [integration/002](../integration/002_the_edge_that_only_exists_under_a_cfg.md) |
| TK21 | E2 states the enforcement as exhaustive matching of every `try_push` result, which holds at both sites with six arms and no wildcard; `refused_staging`, the fifth of R1's five buckets, is filled by `if staging.push( record ).is_err()` instead, so a second staging failure mode would compile silently where a third `Refusal` variant breaks the build twice. | E2's coverage of the R1 sum | n/a — unenforced | [invariant/001](../invariant/001_every_minted_record_is_somewhere.md) |
| TK22 | V3 tells a reader a repeat is `OutOfOrder` with `previous == then`; that equality is a property of adjacency, so `[ 0, 1, 0 ]` reports `previous : 1, then : 0` and is indistinguishable from a reordering — and W7's two assertions, `&[ 1, 1 ]` and `&[ 2, 1 ]`, are both adjacent pairs. | V3's duplicate signature | **misleading doc** | [invariant/001](../invariant/001_every_minted_record_is_somewhere.md) |
| TK23 | `001`'s "What this law does not catch" names one thing — a record the ring accepted and destroyed — while the larger gap, that `tests/exhaustive_test.rs` calls `.audit()` zero times so R1 holds in no interleaving at all, appears only as a parenthetical in the Tests table two sections later. | where the blind spots are recorded | n/a — doc gap | [invariant/002](../invariant/002_the_shape_a_delivered_list_must_have.md) |
| TK24 | The model asserts `received.len() <= 1` and calls `audit_received( &received, 1 )`, and `windows( 2 )` over a slice that short yields no pairs — so the ordering check, the property a concurrent test can plausibly break and the reason E4 makes the function free, cannot execute in the only test that explores interleavings. | the ascent pass under loom | n/a — coverage | [invariant/002](../invariant/002_the_shape_a_delivered_list_must_have.md) |
| TK25 | The call graph over the eight public declarations has two disconnected components inside `src/lib.rs` — the scripted fixture and the loom bridge — joined by no expression in the library and used together only in the two test files. | the reachability relation over the public surface | n/a — observation | [item/001](../item/001_two_components_that_meet_in_no_line_of_src.md) |
| TK26 | **Was exposed, now closed** — five attributes sat on four accessors and a builder pair whose results a caller can recompute for free, while `Script::run`, `leak` and `leak_ends` carried none and returned plain values, so `script.run( &mut ring );` and `leak( ring );` both compiled as statements; all three now carry the attribute with a message naming what is lost, which is what brings the count to eight. | where `#[ must_use ]` was placed | **latent hazard** | [item/001](../item/001_two_components_that_meet_in_no_line_of_src.md) |
| TK27 | **Was exposed, now closed** — the only type this crate returns for a caller to match had three variants and no room attribute, while `invariant/001` weighed adding a fourth as a live design option; `Anomaly` now carries `#[ non_exhaustive ]`, and the fourth variant arrived additively behind it. | `Anomaly`'s room attribute | **latent hazard** | [item/002](../item/002_what_the_crate_does_not_declare.md) |
| TK28 | `Script::run` takes a `ring_core::Ring< u32 >` and the crate declares zero `pub use`, so a consumer needs `ring_core` and `ring_config` in its own manifest before it can call the one function the fixture exists for — the same three-crate cost this crate's own dev-dependencies pay. | the import cost of the entry point | n/a — observation | [item/002](../item/002_what_the_crate_does_not_declare.md) |
| TK29 | The States table answers *"A push is"* once per state while the crate has three write paths, and `Step::Stage`'s arm holds zero references to `shutdown` or `guard` — so a `Stage` on a Closed ring mints, stages and is counted in `staged_at_end` where the row a reader consults says it is refused. | the state table's two write columns | **misleading doc** | [lifecycle/001](../lifecycle/001_the_states_a_script_moves_through.md) |
| TK30 | Behavioral invariant 2 credits termination to consumption being permitted in both states, but `Script::run` spawns zero threads, so nothing can be added because there is no other thread — the argument is `ring_shutdown`'s, holds identically on an open ring here, and is unfalsifiable in the only setting the scripted suite can construct. | the `DrainAll` termination argument | n/a — observation | [lifecycle/001](../lifecycle/001_the_states_a_script_moves_through.md) |
| TK31 | `leak`'s doc sizes the workaround at *"one ring per model execution"* and no loom test calls `leak`; all three models reach `leak_ends`, whose own doc corrects the count to two while explaining a borrow error, and a `Ring` owning a `Storage< T >` makes the real figure three heap blocks per execution. | where the leak's cost is stated | **misleading doc** | [lifecycle/002](../lifecycle/002_the_object_whose_lifecycle_has_no_end.md) |
| TK32 | `tests/testkit_test.rs` opens `#![ cfg( not( loom ) ) ]` and calls the bridge at three sites, so every default `cargo test` strands four allocations under a justification — that loom has no scoped threads — which is compiled out of that very run, and nothing in the scripted suite records it. | the bridge's use outside its cfg | n/a — observation | [lifecycle/002](../lifecycle/002_the_object_whose_lifecycle_has_no_end.md) |
| TK33 | C1 is measured on a five-step script over five `Step` variants where two refusal counters carry non-zero values; C2's ten repeats use a different, three-variant script with zero `Step::Push` occurrences, an eight-slot ring and a matching staging limit, so every refusal counter is structurally zero in the run that is actually repeated. | C2's ten-run script | n/a — coverage | [non_functional_requirement/001](../non_functional_requirement/001_two_runs_compare_equal.md) |
| TK34 | The section names E4 as *"the one that was not assumed"*, and E1–E3 are `#[ test ]` functions while E4 is `tests/manual/readme.md` M2 — a human procedure with one recorded pass and zero automated form in either test file; the manual plan independently calls M2 the stage it would be weakest without, and neither document arranges for it to run again. | E4's evidence form | n/a — unenforced | [non_functional_requirement/001](../non_functional_requirement/001_two_runs_compare_equal.md) |
| TK35 | `src/lib.rs` has zero `unwrap`, `expect` or `panic!` outside doc comments, which is the property that makes a fixture's verdict readable at all — and no `# Panics` section, module sentence, requirement row or test states it, so a consumer learns it by reading 741 lines and has no guarantee it survives the next change. | the crate's panic behavior | n/a — doc gap | [non_functional_requirement/002](../non_functional_requirement/002_what_a_fixture_owes_its_consumers.md) |
| TK36 | `ring_atomic` owns the family's only `--cfg loom` switch and states that *"no other crate needs to know the seam exists"*; `ring_testkit` reaches it through four crates, declares zero `cfg` attributes, and its own loom test records that the cursors *"panic if touched with no model running"* — so a consumer building under the cfg and calling `Script::run` outside a model panics from four crates down, and zero files in `src/` mention it. | the loom seam's reach | **latent hazard** | [non_functional_requirement/002](../non_functional_requirement/002_what_a_fixture_owes_its_consumers.md) |
| TK37 | Three of `Step`'s ten variants are the `n = 1` case of another three, costing three public variants, three verbatim copies of `let count = if let Step::XMany( n ) = *step { n } else { 1 };`, and one test whose sole job is to assert the pairs are equivalent — with no doc comment or decision anywhere weighing the ergonomic argument that would justify them. | the singular/`Many` variant pairs | n/a — duplication | [pattern/001](../pattern/001_the_script_is_data_and_the_run_is_an_interpreter.md) |
| TK38 | `run` returns `Outcome` and never `Result`, and none of its ten fields is a step index, so a `Flush` over an empty buffer, a `RecvMany( 100 )` on an empty ring and a `Reopen` on an open ring each produce an outcome byte-identical to that step's absence — the pattern makes the input fully inspectable and the execution entirely opaque, and does not say so. | what the interpreter reports | n/a — observation | [pattern/001](../pattern/001_the_script_is_data_and_the_run_is_an_interpreter.md) |
| TK39 | `impl core::error::Error for Anomaly {}` exists so a caller can `?` an anomaly out of a test, as the crate's own test comment states; zero of the 36 test functions across both files return a `Result`, and no crate outside depends on `ring_testkit`, so the affordance is compiled and printed but never walked end to end. | the `Error` impl's reachability | n/a — unadopted | [pattern/002](../pattern/002_the_fixture_returns_a_verdict_instead_of_making_one.md) |
| TK40 | Zero of `Anomaly`'s four `Display` arms and zero of `Outcome`'s ten fields name a step, so a failed audit reports a record number against a script the caller must replay by hand — cheap for the crate's own three-to-five-step scripts, and unchanged for a script of any length the pattern permits. | what a failing verdict names | n/a — diagnostics | [pattern/002](../pattern/002_the_fixture_returns_a_verdict_instead_of_making_one.md) |
| TK41 | `Outcome::audit` sums `accepted + refused_full + refused_closed + refused_staging + staged_at_end` and names neither `received` nor `in_ring_at_end`, so a record the ring destroyed is still `placed` and the audit returns `Ok( () )` — correct behavior, whose only counter-signal is `vanished()`, a method with zero call sites in `src/` on a struct whose ten fields are all public, so a consumer can read every field, take the packaged verdict and never meet it. | what the audit sums | **latent hazard** | [pitfall/001](../pitfall/001_neither_the_count_nor_the_list_alone.md) |
| TK42 | The Out of Scope line excludes `OverflowPolicy::DropOldest` because `ring_core::Ring::new` refuses it, but `Ring::new_crossbeam` accepts it on the same type and `Script::run` contains zero `backend` or `crossbeam` references, so the fixture cannot tell the two apart — the exclusion holds only because `crossbeam` appears zero times in this crate's manifest, a fact about its dependencies rather than about the fixture. | the constructor the exclusion rests on | **latent hazard** | [pitfall/001](../pitfall/001_neither_the_count_nor_the_list_alone.md) |
| TK43 | The window's third row has no evidence because it is unreachable from a single-threaded fixture, while the same `tests/` directory holds three `loom::model` closures spawning six threads — the only such file in all 33 `ring_*` crates — plus two real threads in the ordinary suite, and the count of `Shutdown`, `close` and `Refusal` across the loom file is zero in every case. | where the crate's concurrency tooling points | n/a — coverage | [pitfall/002](../pitfall/002_reopening_closes_first.md) |
| TK44 | The `ring_shutdown` API question this pitfall correctly declines to answer is parked in this crate's own decision log as Pending 2; `ring_shutdown` has since filed two decisions of its own and neither is this one (*Should `Guarded::into_inner` Exist*, *Should a `Stopped` Token Be Unique* — the second lands next door, asking whether the token can be duplicated rather than whether the ring can open without one), zero of its docs contain the phrase *unconditional open* and zero name `Step::Reopen`; what makes this worth keeping is that the cheap explanation has since been ruled out — when this was written exactly one file there named `ring_testkit` and it was a test comment, whereas ten do now, nine of them documents citing this crate's `Stopped` binding and its guard as the only ones outside `ring_shutdown` itself, so the channel exists and carries detail in both directions and the question still did not travel: **a question filed in a consumer's decision log is addressed to nobody**, and citation traffic moves facts about code rather than open questions about design. | where the deferred question is filed | n/a — doc gap | [pitfall/002](../pitfall/002_reopening_closes_first.md) |
| TK45 | `.flush_into(` has zero call sites outside `ring_tls` and fifteen inside it, and `ring_tls`'s dependents are exactly `ring_bench`, `ring_flush` and `ring_testkit` — all three of which also depend on `ring_core`, so every crate in the family holding a `TlsBuffer` also holds a `Ring` and all three independently reached for `drain()`, against one shared cause written down in only one of them. | how far the unadopted path reaches | n/a — unadopted | [pitfall/003](../pitfall/003_the_amortised_flush_has_no_ring.md) |
| TK46 | The mismatch argument correctly measures that zero of `ring_core`'s sixteen public functions name a cursor and reads that as the material being absent; `SeqCell` is declared in `ring_atomic` with three implementors in the family, `ring_atomic` sits three hops inside `ring_core`'s own dependency tree, and one implementor is the cursor type the SPSC backend already runs on — so the join is unavailable by encapsulation, not unavailable for want of a cursor. | what "exposes no cursor" establishes | **misleading doc** | [pitfall/003](../pitfall/003_the_amortised_flush_has_no_ring.md) |
| TK47 | The derives table justifies `Clone` as *"so one script can be run against several rings without rebuilding it"*, but `run` takes `&self` and needs no clone for that — the suite runs scripts against rings at twenty-three `script.run(` call sites with zero `.clone()` anywhere in `src/` or `tests/`, and `pattern/001` P3 names the same `&self` as the property the central requirement rests on. | the reason given for `Script`'s `Clone` | **misleading doc** | [type/001](../type/001_outcome_and_anomaly.md) |
| TK48 | The Validation section states that `audit` reports the first failure and that the ordering *"is fixed rather than incidental"*; `audit` is three checks — `Unaccounted`, the `Overdelivered` ceiling, then `audit_delivery_order` — and the four failing-audit inputs in the suite violate exactly one property each, so no input can produce two answers and all six orderings of the three checks leave every test passing. `OutOfOrder` is never reached through `audit` at all. | the fixed reporting order | n/a — unenforced | [type/001](../type/001_outcome_and_anomaly.md) |
| TK49 | `Outcome` is constructor-free with ten public fields and no `#[ non_exhaustive ]`, and the crate's own tests hold four struct literals naming all ten — so the field `001` rejects on staleness grounds breaks four in-crate sites before any consumer. That cost has since been paid: `published` landed as the tenth field and all four literals were rewritten. `Anomaly` does not share it — its eighteen test mentions are constructions and prose, never matches, and it carries the crate's only `#[ non_exhaustive ]`. | what the suite has already frozen, and what it has already paid | n/a — observation | [type/002](../type/002_how_these_values_are_built.md) |
| TK50 | Eight tests assert a failing audit: three struct literals, four bare `&[ u32 ]` slices, and one `Outcome` straight out of `Script::run`. This entry originally recorded that last count as zero, and the single-line census grep meant to verify it could not have matched the five-line assertion that contradicts it — so `audit` is verified to compute the right answer for a given ten-tuple, and misusing `run` can assemble one that trips it; neither establishes that a ring under test ever will. | where the audited broken values come from | n/a — coverage | [type/002](../type/002_how_these_values_are_built.md) |
| TK51 | Of the four Pending items in this crate's decision log, three name another `ring_*` crate as where the answer lives — Pending 2 in `ring_shutdown`, Pending 3 in `ring_core`, Pending 4 in `ring_spsc` — and `pitfall/002` TK44 measures the one case in detail: `ring_shutdown`'s own decision log has never heard the question and one file in that whole crate names `ring_testkit`, so nothing in the corpus carries an entry from the crate that observed a cost to the crate that could remove it. | three of four open questions resolved elsewhere | n/a — doc gap | [workaround/001](../workaround/001_two_allocations_loom_cannot_avoid.md) |
| TK52 | `leak`'s doc comment and this definition both retire the leak's cost as *"loom's own per-execution bookkeeping dwarfs it"* with no number on either side — the ring is checkably tiny at `CAPACITY = 2`, but nothing states loom's per-execution allocation, the executions the three models run, or the total for one `--cfg loom` invocation, and the leak is unbounded in exactly the dimension left uncounted. | the cost dismissed by unmeasured comparison | n/a — doc gap | [workaround/001](../workaround/001_two_allocations_loom_cannot_avoid.md) |
| TK53 | `verb/test` sets `RUSTFLAGS="-D warnings"` and the loom cfg nowhere, no file under `verb/` sets it, and `RUSTFLAGS="--cfg loom"` appears at six sites across four files — manifest comment, both test files' module docs, and `tests/manual/readme.md` twice — so the family's only `#![ cfg( loom ) ]` file, holding three models over six threads, runs when a human remembers. | the cfg no automated command sets | n/a — coverage | [workaround/002](../workaround/002_a_suite_split_in_two_by_a_global_cfg.md) |
| TK54 | The manifest justifies models living in `tests/` because *"the scripted fixture stays measurable by a coverage run that does not set the cfg"*, which is true and does not mention that the same run does not compile the loom half at all — so a rename or signature change that breaks it yields a fully green `verb/test`, and the asymmetry is real: the scripted file's exclusion under the cfg is a soundness requirement, the exhaustive file's exclusion without it is a build consequence with no automated compile check anywhere. | a placement rationale that states only its benefit | **misleading doc** | [workaround/002](../workaround/002_a_suite_split_in_two_by_a_global_cfg.md) |
