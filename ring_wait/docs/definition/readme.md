# Doc Definitions

Module Index for `ring_wait` — every doc definition this crate declares and every
instance under each, in one place.

## Master Doc Definitions Table

| Type | Purpose | Master File | Instances |
|------|---------|-------------|----------:|
| `algorithm/` | One loop with two exits, and the two wrappers that fix its predicate | [algorithm/readme.md](../algorithm/readme.md) | 2 |
| `api/` | Seven items, one production caller, and a parameter that is the design | [api/readme.md](../api/readme.md) | 2 |
| `data_structure/` | A crate that declares nothing and reads the scheduler, a clock, and other people's memory | [data_structure/readme.md](../data_structure/readme.md) | 2 |
| `decisions/` | Two choices with live alternatives, recorded with their arguments | [decisions/readme.md](../decisions/readme.md) | 2 |
| `definition/` | Module Index — every definition and every instance in this crate, in one place | this file | — |
| `integration/` | Two dependencies, two dependents, and a roster naming this crate as the hazard | [integration/readme.md](../integration/readme.md) | 2 |
| `invariant/` | Two properties that must hold for every input, one of them an absence | [invariant/readme.md](../invariant/readme.md) | 2 |
| `item/` | Per-item contracts and coverage, arm by arm | [item/readme.md](../item/readme.md) | 2 |
| `lifecycle/` | A wait from the first look to one of two endings, and a ladder nobody climbs | [lifecycle/readme.md](../lifecycle/readme.md) | 2 |
| `non_functional_requirement/` | What a look costs in atomic loads, and what an attempt costs in nanoseconds | [non_functional_requirement/readme.md](../non_functional_requirement/readme.md) | 2 |
| `pattern/` | Two shapes this crate is the reference instance of | [pattern/readme.md](../pattern/readme.md) | 2 |
| `pitfall/` | Two things that are not what their names suggest | [pitfall/readme.md](../pitfall/readme.md) | 2 |
| `type/` | Two integer domains that never mix, and the annotation that went to the wrong item | [type/readme.md](../type/readme.md) | 2 |
| `workaround/` | Two capabilities the crate cannot have, with costs and deletion conditions | [workaround/readme.md](../workaround/readme.md) | 2 |

13 definitions, 26 instances.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_wait/docs
find . -mindepth 1 -maxdepth 1 -type d ! -name definition | wc -l   # 13
find . -name '0*.md' | wc -l                                        # 26
```

Live output:

```
13
26
```

## Master Doc Instances Table

| Type | ID | Title | Subject |
|------|----|-------|---------|
| `algorithm/` | 001 | [One Loop and the Two Ways Out](../algorithm/001_one_loop_and_the_two_ways_out.md) | Thirteen lines, `:183-195`, and the clamp written twice |
| `algorithm/` | 002 | [Two Wrappers Over a Predicate They Fix](../algorithm/002_two_wrappers_over_a_predicate_they_fix.md) | Two-line bodies, the `map_err` asymmetry, and the `count = 0` tautology |
| `api/` | 001 | [Seven Items and the One With a Caller](../api/001_seven_items_and_the_one_with_a_caller.md) | The whole surface, its three external call sites, and a name census that returns twenty false positives |
| `api/` | 002 | [The Predicate Is the Parameter](../api/002_the_predicate_is_the_parameter.md) | `F : FnMut() -> bool` as the entire extension point, and what it forecloses |
| `data_structure/` | 001 | [A Crate With No Type of Its Own](../data_structure/001_a_crate_with_no_type_of_its_own.md) | Three of 33 crates declare nothing; this is the one that is not pure arithmetic |
| `data_structure/` | 002 | [The Budget and the Attempt Index](../data_structure/002_the_budget_and_the_attempt_index.md) | A count that is not a duration, and `Ok( n )`'s meaning |
| `decisions/` | 001 | [`Park` Sleeps Rather Than Parking](../decisions/001_park_sleeps_rather_than_parking.md) | The registration relationship declined, and the manual check that guards the explanation |
| `decisions/` | 002 | [The Discriminants Live in `ring_types`](../decisions/002_the_discriminants_live_in_ring_types.md) | A prior ruling, the one byte it costs, and the sibling enum that already drifted |
| `integration/` | 001 | [Two Dependencies, Two Dependents, and a Roster](../integration/001_two_dependencies_two_dependents_and_a_roster.md) | The manifest graph, the parking ban, and the tick-safety predicate the family does have |
| `integration/` | 002 | [The Wrapper That Had to Be Rewritten](../integration/002_the_wrapper_that_had_to_be_rewritten.md) | `ring_shutdown::for_space_or_close`, and the parameter `for_space` fixed that it needed back |
| `invariant/` | 001 | [Every Repetition Is a Counted `for`](../invariant/001_every_repetition_is_a_counted_for.md) | `spins × 8` as a source-visible bound, against the family's six bare loops |
| `invariant/` | 002 | [`None` Looks Exactly Once](../invariant/002_none_looks_exactly_once.md) | The tick-path contract, measured at 1.0–2.4 µs for a full budget |
| `item/` | 001 | [The Four Arms of the Pause](../item/001_the_four_arms_of_the_pause.md) | Four arms, 32 lines, and the parameter three of them ignore |
| `item/` | 002 | [The Loop, the Wrapper, and the Two Questions](../item/002_the_loop_the_wrapper_and_the_two_questions.md) | `wait_until`, `wait`, `for_space`, `for_data` — contracts, callers, and coverage |
| `lifecycle/` | 001 | [A Wait From the First Look to One of Two Endings](../lifecycle/001_a_wait_from_the_first_look_to_one_of_two_endings.md) | The states a wait passes through, and the pause after the last look |
| `lifecycle/` | 002 | [The Escalation Ladder Nobody Climbs](../lifecycle/002_the_escalation_ladder_nobody_climbs.md) | Three arms, four variants, zero callers, and the `#[ must_use ]` that guards nothing |
| `non_functional_requirement/` | 001 | [Two Atomic Loads for Every Look](../non_functional_requirement/001_two_atomic_loads_for_every_look.md) | Both predicates cost 2 `Acquire` loads; a default-budget `for_data` costs up to 2048 |
| `non_functional_requirement/` | 002 | [What Each Strategy Costs Per Attempt](../non_functional_requirement/002_what_each_strategy_costs_per_attempt.md) | Measured: 55 ns to 119 µs on one enum argument, a 1700× span |
| `pattern/` | 001 | [The Predicate, the Pause, and the Budget](../pattern/001_the_predicate_the_pause_and_the_budget.md) | The three-part shape, its four instances, and the pause after the final look |
| `pattern/` | 002 | [Discriminants Here, Handlers There](../pattern/002_discriminants_here_handlers_there.md) | The policy-enum split, and a comment-stripped census of who really matches on what |
| `pitfall/` | 001 | [Reading `Empty` as "Nothing to Do"](../pitfall/001_reading_empty_as_nothing_to_do.md) | One `map_err` against three items that hand a producer the wrong word |
| `pitfall/` | 002 | [The Backoff That Resets Every Eight Attempts](../pitfall/002_the_backoff_that_resets_every_eight_attempts.md) | A period-8 sawtooth called a backoff in two places, asserted in none |
| `type/` | 001 | [A `usize` Budget and a `u64` Count](../type/001_a_usize_budget_and_a_u64_count.md) | Two integer domains, zero casts, and the two zeroes that typecheck |
| `type/` | 002 | [One Return Type and the One `#[ must_use ]`](../type/002_one_return_type_and_the_one_must_use.md) | Three shapes, two of nine variants, and an annotation density of 0.17 against 0.89–1.00 |
| `workaround/` | 001 | [A Sleep Where a Park Belongs](../workaround/001_a_sleep_where_a_park_belongs.md) | 50 µs requested, 112–119 µs delivered, and two crates computing on the request |
| `workaround/` | 002 | [A Crate Name as a Load-Bearing String](../workaround/002_a_crate_name_as_a_load_bearing_string.md) | A dependency ban Cargo cannot express, substituted with a substring match |

## Findings

Fifty-three, each verified by a command whose output is quoted in its instance,
or by a probe program in a scratch crate.

| ID | Finding | Subject | Reachable | Where |
|----|---------|---------|-----------|-------|
| WT1 | Of seven public items, only `wait_until` is called from another crate's `src/`; the other six are reached solely from `tests/wait_test.rs` and their own doctests | `ring_wait` | n/a — coverage | [api/001](../api/001_seven_items_and_the_one_with_a_caller.md) |
| WT2 | No crate anywhere writes `use ring_wait`; all three call sites spell the path in full, which is what a single-call dependency looks like | family | n/a — observation | [api/001](../api/001_seven_items_and_the_one_with_a_caller.md) |
| WT3 | Fixing the predicate is what makes both named wrappers unreachable: a caller needing one extra exit condition has nowhere to put it, so `ring_shutdown::for_space_or_close` re-typed `for_space`'s closure over `wait_until` rather than wrapping it | `ring_shutdown` | n/a — duplication | [algorithm/002](../algorithm/002_two_wrappers_over_a_predicate_they_fix.md) |
| WT4 | Two of the three production call sites discard the attempt count, both because they had something better to return, so the crate's return type was designed for the consumer it does not have | family | n/a — observation | [item/002](../item/002_the_loop_the_wrapper_and_the_two_questions.md) |
| WT5 | `attempt % 8` discards everything above the low three bits, so attempt 1000 and attempt 0 emit the same single hint; over `DEFAULT_SPINS` the arm emits 4608 hints in the same 8-step pattern 128 times | `ring_wait` | n/a — inconsistency | [pitfall/002](../pitfall/002_the_backoff_that_resets_every_eight_attempts.md) |
| WT6 | Measured: the same call with the same budget spans roughly 1700× on one enum argument — 55 ns to 119 µs per attempt — and the default is `Spin`, the arm that holds a core | `ring_wait` | **measured cost** | [non_functional_requirement/002](../non_functional_requirement/002_what_each_strategy_costs_per_attempt.md) |
| WT7 | `Park` delivers 112–119 µs against a requested 50 µs, and two tests in two other crates compute bounds from the requested figure without being able to see the delivered one | family | **measured cost** | [non_functional_requirement/002](../non_functional_requirement/002_what_each_strategy_costs_per_attempt.md) |
| WT8 | The crate's single `#[ must_use ]` is on `escalation_hint`, which has no caller, while `pause`'s `bool` carries the tick-path guarantee and is silently discardable; six neighbours annotate at 0.89–1.00 per `pub fn` against this crate's 0.17 | `ring_wait` | n/a — unenforced | [type/002](../type/002_one_return_type_and_the_one_must_use.md) |
| WT9 | `for_data( pair, 0, … )` returns `Ok( 0 )` on a ring that was never published to, because `pending() >= 0` is a tautology; no test and no doctest covers it | `ring_wait` | n/a — coverage | [algorithm/002](../algorithm/002_two_wrappers_over_a_predicate_they_fix.md) |
| WT10 | `ring_poll::PARKING_CRATES` is declared as "crates from which a parking operation is reachable" and measured as "manifests containing the text"; `ring_consume` reaches one through `ring_barrier` and `ring_testkit` through `ring_shutdown`, and neither appears on either list | `ring_poll` | n/a — unenforced | [workaround/002](../workaround/002_a_crate_name_as_a_load_bearing_string.md) |
| WT11 | Of the three entries the manifest scan does produce, two are dependency edges and the third is `ring_wait`'s own `name =` field; the rule is "the text appears", which reaches the right answer for a reason unrelated to the question | `ring_poll` | n/a — unenforced | [workaround/002](../workaround/002_a_crate_name_as_a_load_bearing_string.md) |
| WT12 | Four bounded-retry-with-a-pause-hint loops exist in the family and only one is this crate's; `ring_poll` open-codes it three times because the crate is the only granularity a manifest can enforce at build time | family | n/a — duplication | [integration/001](../integration/001_two_dependencies_two_dependents_and_a_roster.md) |
| WT13 | `ring_handle` forbids seven parking-shaped names in its own source; three of the seven are in `ring_wait`'s code and a fourth is in the comment W4 requires to exist, so the two guards are one rule enforced from opposite ends | family | n/a — observation | [integration/001](../integration/001_two_dependencies_two_dependents_and_a_roster.md) |
| WT14 | Three of the family's 33 crates declare no `struct`, `enum`, or `trait`: `ring_index`, `ring_seqno`, and `ring_wait`; the first two are pure arithmetic and this one reads the scheduler, a clock, and memory it was never handed | family | n/a — observation | [data_structure/001](../data_structure/001_a_crate_with_no_type_of_its_own.md) |
| WT15 | The two `RingError` variants this crate can produce are exactly the set `RingError::is_transient` matches on — two definitions written independently that landed on the same pair | family | n/a — observation | [type/002](../type/002_one_return_type_and_the_one_must_use.md) |
| WT16 | Both named predicates issue exactly two `Acquire` loads per look, because neither question can be answered from one cursor, so a `for_data` at the default budget costs up to 2048 gated loads on two lines another thread is writing | `ring_cursor` | **measured cost** | [non_functional_requirement/001](../non_functional_requirement/001_two_atomic_loads_for_every_look.md) |
| WT17 | `spins.max( 1 )` and `ring_poll::Budget::new`'s `if attempts == 0 { Self( 1 ) }` are the same rule written twice, at the loop header and at the constructor; `ring_poll` may not depend on this crate, so they cannot be shared | family | n/a — duplication | [algorithm/001](../algorithm/001_one_loop_and_the_two_ways_out.md) |
| WT18 | Every `wait(` call site outside this crate — twenty-four of them, twenty when this was written — is `RingConfig::with_wait`, `RingConfig::wait` or `RingStats::record_wait`, and none is `ring_wait::wait`, so a bare-name census returns nothing but false positives and misses the one real caller | family | n/a — inconsistency | [api/001](../api/001_seven_items_and_the_one_with_a_caller.md) |
| WT19 | `WaitKind` is one byte, `Copy`, and fieldless, which is what lets `RingConfig::with_wait` be a `const fn` and makes the discriminants/handlers split free at every call site | `ring_types` | n/a — observation | [decisions/002](../decisions/002_the_discriminants_live_in_ring_types.md) |
| WT20 | W4's second command requires `thread::park` to *appear* in the file, in a comment; it is the family's only manual check that asserts an explanation still exists rather than that code does | `ring_wait` | n/a — observation | [decisions/001](../decisions/001_park_sleeps_rather_than_parking.md) |
| WT21 | Three of `pause`'s four arms take an `attempt` parameter they never read, and the fourth reads it through `% 8`, so the parameter's consumed information content is three bits delivered to one arm | `ring_wait` | n/a — observation | [item/001](../item/001_the_four_arms_of_the_pause.md) |
| WT22 | `WaitKind` has exactly one handler and `OverflowPolicy`, ruled identically, already has two, so adding a variant to the latter is an edit in two crates that neither compiler nor test connects | family | n/a — drift | [pattern/002](../pattern/002_discriminants_here_handlers_there.md) |
| WT23 | The family *does* have a per-variant tick-safety predicate (`WaitKind::is_non_blocking`, surfaced as `RingConfig::is_tick_safe`), documented as "whether this strategy can be used on the tick path" and true for `None` alone — while `ring_poll`, the crate the tick path belongs to, spins on it three times | family | n/a — inconsistency | [integration/001](../integration/001_two_dependencies_two_dependents_and_a_roster.md) |
| WT24 | `wait_until` pauses after the final look, for a look that never happens: a `Park` wait at a budget of 1 sleeps 133 µs and that sleep is 100% of the call's cost; `ring_poll`'s copies guard against it with one comparison and nothing tests it in either crate | `ring_wait` | **measured cost** | [pattern/001](../pattern/001_the_predicate_the_pause_and_the_budget.md) |
| WT25 | `wait_until` returns `Result< usize, RingError >` and never documents what the `usize` counts: no `# Returns` section, and a doctest that asserts `is_ok()` plus the closure's own counter rather than the value | `ring_wait` | n/a — doc gap | [algorithm/001](../algorithm/001_one_loop_and_the_two_ways_out.md) |
| WT26 | Every public item carries exactly one executable example, so documentation effort is allocated by position on the surface rather than by reach — six of the seven demonstrate items no other crate calls | `ring_wait` | n/a — observation | [api/001](../api/001_seven_items_and_the_one_with_a_caller.md) |
| WT27 | The `ring_cursor` dependency exists for `for_space` and `for_data` alone, neither of which is called outside this crate; the manual plan's W6 asks whether every dependency is *used*, which is a question a manifest can answer and reach is not | `ring_wait` | n/a — observation | [data_structure/001](../data_structure/001_a_crate_with_no_type_of_its_own.md) |
| WT28 | Eight crates name `WaitKind` in their source and two take a dependency on `ring_wait`, so the vocabulary travels four times further than the code — which is what makes the parking ban enforceable at all | family | n/a — observation | [data_structure/001](../data_structure/001_a_crate_with_no_type_of_its_own.md) |
| WT29 | `for_data`'s `count` is bounded by nothing, so a request above capacity can never be satisfied and burns the whole budget; the only test of the failing case uses a budget of 1, where an unsatisfiable request and an unsatisfied one cost the same | `ring_wait` | n/a — coverage | [data_structure/002](../data_structure/002_the_budget_and_the_attempt_index.md) |
| WT30 | `ring_types::WaitKind::Park` used to document the variant as "Block until a publisher signals. Costs nothing while idle, pays a wakeup" — the only implementation sleeps 50 µs on a timer, no signalling primitive exists anywhere in the family, and this correction sits in a crate two-thirds of `WaitKind`'s users never depend on, so the wording was fixed at its own source and now reads "Idle between reads; no publisher wakes it. Cheapest, highest latency." | `ring_types` | **misleading doc** | [decisions/001](../decisions/001_park_sleeps_rather_than_parking.md) |
| WT31 | The family's two bounded-retry crates ship defaults a factor of 1024 apart — `DEFAULT_SPINS` at 1024, `ring_poll::Budget::once` at 1 — encoding the tick-path distinction in two numbers that carry no reference to each other in either direction | family | n/a — inconsistency | [decisions/002](../decisions/002_the_discriminants_live_in_ring_types.md) |
| WT32 | `ring_claim:34` tells the caller to compose `ring_wait::for_space` then `Claimer::claim`; `ring_claim` correctly does not depend on `ring_wait`, and `for_space` has no call site in any crate, so the prescribed composition exists in one sentence and nowhere in the tree | family | n/a — unadopted | [integration/001](../integration/001_two_dependencies_two_dependents_and_a_roster.md) |
| WT33 | `wait_until` documents "# Panics — Never", justified by a bounded loop, on the one function whose body is a caller-supplied closure; a panicking predicate unwinds straight through and nothing tests it in this crate or the two that call it | `ring_wait` | n/a — doc gap | [invariant/001](../invariant/001_every_repetition_is_a_counted_for.md) |
| WT34 | The crate's only budget sweep runs `1..8`, stopping one iteration short of the `attempt % 8` period, so the sawtooth WT5 records sits just outside the reach of the only test whose loop would have run into it | `ring_wait` | n/a — coverage | [invariant/001](../invariant/001_every_repetition_is_a_counted_for.md) |
| WT35 | The test file is `#![ cfg( not( loom ) ) ]` at file scope with no `#[ cfg( loom ) ]` counterpart, so the crate whose entire subject is what a thread does while waiting contributes no interleaving model to a family where 27 crates' tests are loom-aware | `ring_wait` | n/a — coverage | [invariant/002](../invariant/002_none_looks_exactly_once.md) |
| WT36 | The non-blocking guarantee is a `match` arm returning `false`, while `WaitKind::is_non_blocking` says the same thing in `ring_types`; the only occurrence of the predicate in this crate's source is a rustdoc link, and one test in the wrong crate is all that connects the two definitions | family | n/a — duplication | [invariant/002](../invariant/002_none_looks_exactly_once.md) |
| WT37 | `escalation_hint` is the crate's only `const fn`, and the five items callers actually reach are excluded from const-ness by the two `std` calls in `pause` — so the keyword marks the one function with nothing observable to do and no caller to do it for | `ring_wait` | n/a — observation | [item/001](../item/001_the_four_arms_of_the_pause.md) |
| WT38 | `pause`'s doctest exercises `Spin` and `None`, the two arms that stay inside `core`; the two that reach `std` — the ones that make the crate un-`no_std`, trigger `ring_handle`'s guard, and carry the crate's largest decision — have no executable example | `ring_wait` | n/a — coverage | [item/002](../item/002_the_loop_the_wrapper_and_the_two_questions.md) |
| WT39 | The file's one cross-thread test waits with `Yield` at a budget of 100 000, roughly 100× `DEFAULT_SPINS`; `Spin` and `Park` — the default variant and the expensive one — are never observed against a predicate that changes underneath them | `ring_wait` | n/a — coverage | [lifecycle/001](../lifecycle/001_a_wait_from_the_first_look_to_one_of_two_endings.md) |
| WT40 | A wait leaves the loop three ways — ready, `pause` refusing to continue, and range exhaustion — and the last two share one `Err( RingError::Empty )`, so "nothing right now" and "nothing after 1024 attempts" reach the caller as one variant | `ring_wait` | n/a — observation | [lifecycle/001](../lifecycle/001_a_wait_from_the_first_look_to_one_of_two_endings.md) |
| WT41 | The ladder ends at `Park`, so "cheapest CPU" and "most expensive attempt" are the same rung; and `escalation_hint` folds `Park` and `None` into one arm, making the four-variant enum a three-rung ladder whose fourth variant is reachable only by choosing it outright | `ring_wait` | n/a — observation | [lifecycle/002](../lifecycle/002_the_escalation_ladder_nobody_climbs.md) |
| WT42 | Three of the file's 24 tests are dedicated to `escalation_hint`, which no crate calls, while `wait_until` — the one item another crate's `src/` reaches — has no test named for it; verification effort tracks how easy an item is to test, not what depends on it | `ring_wait` | n/a — coverage | [lifecycle/002](../lifecycle/002_the_escalation_ladder_nobody_climbs.md) |
| WT43 | No crate in the family has a `benches/` directory or a benchmark framework in its manifest, so every ns and µs figure in this corpus is a transcription from a scratch crate — the structural claims are re-run by the gate and the timing claims, including WT7's basis for saying two crates budget from the wrong number, are not | family | n/a — unenforced | [non_functional_requirement/002](../non_functional_requirement/002_what_each_strategy_costs_per_attempt.md) |
| WT44 | The shared three-part shape disagrees about whether its budget is a domain type: `ring_poll` wraps it in a `Budget` newtype that clamps in its constructor, this crate passes a bare `usize` and clamps at every loop header (WT17) — and the crates may not depend on each other, so the disagreement is unresolvable | family | n/a — inconsistency | [pattern/001](../pattern/001_the_predicate_the_pause_and_the_budget.md) |
| WT45 | `escalation_hint` folds `Park` with `WaitKind::None` into one arm, the only place in the handler crate the enum is not matched variant by variant; a fifth variant would break `pause`'s compile and be silently absorbed as terminal here, in the one function whose correctness rests on which variants are terminal | `ring_wait` | n/a — drift | [pattern/002](../pattern/002_discriminants_here_handlers_there.md) |
| WT46 | Both production callers of `wait_until` hit the same budget-exhausted condition and name it differently — `ring_shutdown` rewrites it to `Full`, `ring_barrier` propagates `Empty` — so a caller branching on the variant gets a different answer depending on which consumer it went through | family | n/a — inconsistency | [pitfall/001](../pitfall/001_reading_empty_as_nothing_to_do.md) |
| WT47 | The crate's only `map_err` is the producer-side rename inside `for_space`, which nothing calls; every producer in the family that waits does so through `wait_until` and receives `Empty` for a ring with no room, so the fix was written and placed where the callers are not | `ring_wait` | n/a — unadopted | [pitfall/001](../pitfall/001_reading_empty_as_nothing_to_do.md) |
| WT48 | `pause`'s rustdoc says `attempt` exists "so a strategy can behave differently early and late"; the only arm that reads it folds through `% 8`, so there is no late — over 99.2% of a default-budget wait the parameter distinguishes nothing | `ring_wait` | n/a — observation | [pitfall/002](../pitfall/002_the_backoff_that_resets_every_eight_attempts.md) |
| WT49 | `spins` and the returned attempt count are different quantities sharing one type, so feeding a result back as the next budget typechecks and means nothing; `ring_poll` avoids the same collision by construction with `Budget` and `Progress` newtypes | `ring_wait` | n/a — observation | [type/001](../type/001_a_usize_budget_and_a_u64_count.md) |
| WT50 | The crate's one type parameter is `F : FnMut() -> bool`, so its thirteen-line loop is monomorphised per predicate — the right trade against a vtable call inside a spin loop measured at two `Acquire` loads per look, and the reason the crate has one return *type* and no single return instance | `ring_wait` | n/a — observation | [type/002](../type/002_one_return_type_and_the_one_must_use.md) |
| WT51 | The crate exports one public constant and has two tunables: `DEFAULT_SPINS` is documented, doctested, and overridable by argument at every call site; the 50 µs sleep is an unnamed literal reachable through no parameter, and is the one two other crates transcribe rather than reference (WT7) | `ring_wait` | n/a — observation | [workaround/001](../workaround/001_a_sleep_where_a_park_belongs.md) |
| WT52 | `PARKING_CRATES` is a fixed-size array in the enforcing crate, so a crate that legitimately needs to wait cannot declare that for itself — it adds the dependency, `ring_poll`'s suite fails, and the fix is an edit to `ring_poll`; the permission lives with the enforcer, which is the correct trade for a rule Cargo cannot express | `ring_poll` | n/a — observation | [workaround/002](../workaround/002_a_crate_name_as_a_load_bearing_string.md) |
| WT53 | The guard asserting no unbounded loop matched nothing in 33 crates because its pattern required a brace on the keyword's own line, against a codebase that puts it on the next; the corrected pattern finds six, one of them the `ring_publish::publish` loop the same instance's next paragraph cites by line number | `ring_wait` | n/a — unenforced | [invariant/001](../invariant/001_every_repetition_is_a_counted_for.md) |

### Fifty-Three Findings and One Reachable Defect

One entry is reachable and it is not in this crate: `ring_types::WaitKind::Park`
promised a caller that the variant blocks until a publisher signals, costs
nothing while idle, and pays a wakeup, and the only implementation of it sleeps
on a timer (WT30). Its fix was an edit to another crate's source, since applied
there. Four more are `**measured cost**`. The remaining forty-eight
are `n/a`, and as with `ring_barrier` that is the headline rather than an
accident of scoring. The crate is 269 lines with one
loop, no `unsafe`, no `Ordering`, no atomic operation of its own, no type
declaration, and no cast. Its entire behaviour is one counted `for` and a
four-arm `match` on a one-byte enum.

What the findings are about instead is **reach, cost, and instruments**.

The reach: seven public items with one production caller between them (WT1),
`escalation_hint` with none at all (WT8, WT20), a wrapper whose one real consumer
had to rewrite it (WT3), and a name so ordinary that a census for it returns
nothing but false positives (WT18).

The cost: two `Acquire` loads per look scaling to 2048 at the default budget
(WT16), a 1700× span across one enum argument (WT6), a `Park` arm 2.3× more
expensive than the figure two other crates reason from (WT7), and a pause after
the final look that is 100% of a budget-1 `Park` wait (WT24).

The instruments: a roster that measures a substring where its own heading claims
transitive reachability (WT10), whose third entry is a self-reference (WT11), a
tick-safety predicate whose doc comment overreaches its body while the tick-path
crate spins on it three times (WT23), and a sawtooth called a backoff by the
source comment and the test file, and asserted by neither (WT5). To those, a
guard that could not have failed: the check asserting this crate contains no
unbounded loop used a pattern no file in the codebase can match, and passed for
33 crates including the one whose bare `loop` it was written to distinguish
against (WT53).

The gaps: no `benches/` anywhere in the family, so every timing figure quoted
here is a transcription nothing re-runs (WT43); a test file compiled out of the
family's loom runs with no model to replace it, in the crate whose entire subject
is waiting (WT35); and a budget sweep that stops one iteration short of the
period it would have revealed (WT34).

### Severity

| Tier | Findings | Why |
|------|----------|-----|
| **measured cost** | WT6, WT7, WT16, WT24 | Four figures a reader would want and the source states nowhere — a 1700× span across one argument, a sleep 2.3× its stated cost, 2048 gated loads at the default budget, and a final pause that is the whole of a budget-1 wait |
| **misleading doc** | WT30 | A published variant doc whose three claims were each false of the only implementation; fixed at source in `ring_types`, the artifact at fault being one line of another crate's public doc rather than anything in this one |
| n/a — observation | WT2, WT4, WT13, WT14, WT15, WT19, WT20, WT21, WT26, WT27, WT28, WT37, WT40, WT41, WT48, WT49, WT50, WT51, WT52 | Correct as written; each records a property of the crate or the family the source states nowhere |
| n/a — coverage | WT1, WT9, WT29, WT34, WT35, WT38, WT39, WT42 | An input class, a variant, or an execution mode the suite does not reach — including the sweep that stops one iteration short of the period it would have exposed |
| n/a — inconsistency | WT5, WT18, WT23, WT31, WT44, WT46 | One fact spelled two ways across crates that cannot reference each other — a budget, an error name, a backoff, a method name |
| n/a — unenforced | WT8, WT10, WT11, WT43, WT53 | A check, guard, or roster that passes without being able to fail, or a claim no instrument re-derives |
| n/a — duplication | WT3, WT12, WT17, WT36 | One rule written twice, each time because the crate boundary is the only enforceable granularity |
| n/a — doc gap | WT25, WT33 | A documented guarantee or return value the rustdoc does not actually describe |
| n/a — drift | WT22, WT45 | A shape that would absorb a change silently, in the one place the enum is not handled variant by variant |
| n/a — unadopted | WT32, WT47 | A composition or a fix that was written and placed where no caller reaches it |
