# Doc Definitions

Module Index for `ring_handle` — every doc definition this crate declares and every
instance under each, in one place.

## Master Doc Definitions Table

| Type | Purpose | Master File | Instances |
|------|---------|-------------|----------:|
| `algorithm/` | The split that creates every guarantee, and the delegation path that must add nothing | [algorithm/readme.md](../algorithm/readme.md) | 2 |
| `api/` | Two caller surfaces, documented as much by their absences as by their operations | [api/readme.md](../api/readme.md) | 2 |
| `data_structure/` | One field per handle, why only one of three candidate shapes was reachable, and the single struct that is not a newtype | [data_structure/readme.md](../data_structure/readme.md) | 2 |
| `decisions/` | Why this crate exists given `ring_core` already splits, and why `is_closed()` is absent | [decisions/readme.md](../decisions/readme.md) | 2 |
| `integration/` | One dependency concealing three backends, and an export boundary this crate sits *on* | [integration/readme.md](../integration/readme.md) | 2 |
| `invariant/` | Two restrictions on one surface, enforced by absence along two axes | [invariant/readme.md](../invariant/readme.md) | 2 |
| `item/` | The five nouns and twelve verbs the surface consists of, catalogued against what each wraps | [item/readme.md](../item/readme.md) | 2 |
| `lifecycle/` | The pair's arc from split to drop, and ownership and liveness as two orthogonal axes that combine badly in one state | [lifecycle/readme.md](../lifecycle/readme.md) | 4 |
| `non_functional_requirement/` | This crate's Reached condition, split into the two independent properties it bundles | [non_functional_requirement/readme.md](../non_functional_requirement/readme.md) | 2 |
| `pattern/` | Enforce by withholding rather than by checking, and the three-way rule every method obeys | [pattern/readme.md](../pattern/readme.md) | 2 |
| `pitfall/` | Eight reasonable-looking edits, five of which nothing catches, and a pinned diagnostic nobody declared | [pitfall/readme.md](../pitfall/readme.md) | 2 |
| `type/` | Two rights expressed as values, whose absent trait impls are part of their definition | [type/readme.md](../type/readme.md) | 2 |
| `workaround/` | External constraints this crate absorbs, with costs and deletion conditions | [workaround/readme.md](../workaround/readme.md) | 2 |

## Master Doc Instances Table

| Definition | ID | Name | File |
|------------|----|------|------|
| `algorithm/` | 001 | Splitting a Ring Into Two Ends | [algorithm/001_splitting_a_ring_into_two_ends.md](../algorithm/001_splitting_a_ring_into_two_ends.md) |
| `algorithm/` | 002 | Delegating an Operation to the Backend | [algorithm/002_delegating_to_the_backend.md](../algorithm/002_delegating_to_the_backend.md) |
| `api/` | 001 | Producer Surface | [api/001_producer_surface.md](../api/001_producer_surface.md) |
| `api/` | 002 | Consumer Surface | [api/002_consumer_surface.md](../api/002_consumer_surface.md) |
| `data_structure/` | 001 | Two Handles Over One Backend | [data_structure/001_two_handles_over_one_backend.md](../data_structure/001_two_handles_over_one_backend.md) |
| `data_structure/` | 002 | The One Struct That Is Not a Newtype | [data_structure/002_the_one_struct_that_is_not_a_newtype.md](../data_structure/002_the_one_struct_that_is_not_a_newtype.md) |
| `decisions/` | 001 | What This Crate Is For, Given `ring_core` Already Splits | [decisions/001_what_this_crate_is_for.md](../decisions/001_what_this_crate_is_for.md) |
| `decisions/` | 002 | Why `is_closed` Is Absent | [decisions/002_why_is_closed_is_absent.md](../decisions/002_why_is_closed_is_absent.md) |
| `integration/` | 001 | One Dependency and the Backends Beneath It | [integration/001_one_dependency_and_the_backends_beneath.md](../integration/001_one_dependency_and_the_backends_beneath.md) |
| `integration/` | 002 | On the Export Surface | [integration/002_on_the_export_surface.md](../integration/002_on_the_export_surface.md) |
| `invariant/` | 001 | Capability Follows the Handle | [invariant/001_capability_follows_the_handle.md](../invariant/001_capability_follows_the_handle.md) |
| `invariant/` | 002 | Nothing Reachable From a Handle Can Park | [invariant/002_no_parking_operation_is_reachable.md](../invariant/002_no_parking_operation_is_reachable.md) |
| `item/` | 001 | Five Nouns, Four of Them the Same Width as What They Wrap | [item/001_five_nouns_four_of_them_the_same_width.md](../item/001_five_nouns_four_of_them_the_same_width.md) |
| `item/` | 002 | Twelve Verbs, Eight of Them a Bare Forward | [item/002_twelve_verbs_eight_bare_forwards.md](../item/002_twelve_verbs_eight_bare_forwards.md) |
| `lifecycle/` | 001 | Split, Move and Drop | [lifecycle/001_split_move_and_drop.md](../lifecycle/001_split_move_and_drop.md) |
| `lifecycle/` | 002 | The Barrier Holds the Consumer | [lifecycle/002_the_barrier_holds_the_consumer.md](../lifecycle/002_the_barrier_holds_the_consumer.md) |
| `lifecycle/` | 003 | Handle Ownership | [lifecycle/003_handle_ownership.md](../lifecycle/003_handle_ownership.md) |
| `lifecycle/` | 004 | Ring Liveness Through a Handle | [lifecycle/004_ring_liveness_through_a_handle.md](../lifecycle/004_ring_liveness_through_a_handle.md) |
| `non_functional_requirement/` | 001 | Proven by Code That Must Not Compile | [non_functional_requirement/001_proven_by_code_that_must_not_compile.md](../non_functional_requirement/001_proven_by_code_that_must_not_compile.md) |
| `non_functional_requirement/` | 002 | Send Without Sync | [non_functional_requirement/002_send_without_sync.md](../non_functional_requirement/002_send_without_sync.md) |
| `pattern/` | 001 | Enforce by Withholding, Not by Checking | [pattern/001_enforce_by_withholding.md](../pattern/001_enforce_by_withholding.md) |
| `pattern/` | 002 | Forward, Narrow, or Add | [pattern/002_forward_narrow_or_add.md](../pattern/002_forward_narrow_or_add.md) |
| `pitfall/` | 001 | A Convenience Method Undoes the Crate | [pitfall/001_a_convenience_method_undoes_the_crate.md](../pitfall/001_a_convenience_method_undoes_the_crate.md) |
| `pitfall/` | 002 | A Pinned Diagnostic Is a Dependency Nobody Declared | [pitfall/002_a_pinned_diagnostic_is_a_dependency_nobody_declared.md](../pitfall/002_a_pinned_diagnostic_is_a_dependency_nobody_declared.md) |
| `type/` | 001 | Producer | [type/001_producer.md](../type/001_producer.md) |
| `type/` | 002 | Consumer | [type/002_consumer.md](../type/002_consumer.md) |
| `workaround/` | 001 | Asserting an Absence Needs a Second Compiler Run | [workaround/001_asserting_an_absence_needs_a_second_compiler_run.md](../workaround/001_asserting_an_absence_needs_a_second_compiler_run.md) |
| `workaround/` | 002 | The Expected Output Names Crates This One Never Sees | [workaround/002_the_expected_output_names_crates_this_one_never_sees.md](../workaround/002_the_expected_output_names_crates_this_one_never_sees.md) |

**28 instances across 13 definitions**, every one of them carrying at least two.
The corpus standard's floor is 26 across 13; the surplus is `lifecycle/`, which
needed four because ownership and liveness are orthogonal axes rather than two
points on one arc.

Twelve of the thirteen sit at three grains. `invariant/` and
`non_functional_requirement/` hold the **contract grain** — the two restrictions
on this surface and the binary Reached condition the acceptance table grades
them by. `algorithm/`, `data_structure/`, `pattern/`, `pitfall/` and
`workaround/` hold the **mechanism grain**: a partition created once, preserved
by adding nothing, and asserted by a compiler run that has to fail on purpose.
`api/`, `item/`, `lifecycle/`, `type/` and `integration/` hold the
**surface grain**. `decisions/` is the thirteenth and sits across all three —
it records what was chosen rather than what holds.

**Being on the export list is what makes the surface grain expensive**, and it
is the reason these instances commit to signatures where
[`ring_spsc`](../../../ring_spsc/docs/definition/readme.md)'s leave three
candidates open. This crate's own design
puts the internal crates' capabilities behind this surface; a change here
reaches every consumer of the family rather than stopping two crates away.

**Three instances carry findings that outrank the rest.**
[`pitfall/001`](../pitfall/001_a_convenience_method_undoes_the_crate.md)
establishes that the acceptance criterion covers the one edit nobody makes and
misses the four that actually arrive. Its own conclusion — that closing the gap
requires amending a shared, family-grain table — turned out to be half right:
the `!Clone` half was closed locally, because a *stricter* local test needs no
amendment to a shared criterion, while `Deref` and an accessor stay open for a
reason no amendment would fix.
[`lifecycle/002`](../lifecycle/002_the_barrier_holds_the_consumer.md)
establishes that the determinism this crate is credited with enabling is *not*
enforced by it, names the one crate that could enforce it, and states what that
would cost.
[`decisions/001`](../decisions/001_what_this_crate_is_for.md) states plainly
that `ring_core` already partitions the capabilities this crate's own row asks for, so
this crate's justification is four narrowings rather than the split itself —
an open question left unresolved rather than a defence.

`decisions/` records two questions. The backend reference's shape is **not**
among them: it was expected to be a decision and turned out to be forced, so it
is documented where it takes effect
(→ [`data_structure/001`](../data_structure/001_two_handles_over_one_backend.md)).

No `format/` directory exists: this crate defines no byte layout. Its five
structs are newtypes over values whose layout belongs to `ring_core` and the
backend crates.

**The `item/` paragraph that stood here asserted two things and both were
false.** It read: "No `item/` directory exists either … no crate in the 33-crate
family carries an `item/`, which makes it a family-grain deferral rather than a
local omission." There is an `item/` here with two instances, and the family
count is not zero:

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'crates in the family:        %s\n' "$( ls -d ring_*/ | wc -l )"
printf 'of them carrying docs/item:  %s\n' "$( ls -d ring_*/docs/item 2>/dev/null | wc -l )"
printf 'instances in this crate:     %s\n' "$( ls ring_handle/docs/*/[0-9][0-9][0-9]_*.md | wc -l )"
printf 'definitions declared:        %s\n' "$( ls -d ring_handle/docs/*/ | command grep -cv '/definition/$' )"
printf 'findings in this crate:      %s\n' "$( command grep -rhoE '^### HD[0-9]+ — ' ring_handle/docs/*/[0-9][0-9][0-9]_*.md | wc -l )"
```

This is the same failure shape `ring_factory`'s index records: a claim about
*other* crates written into *this* one, which nothing that changes it ever
touches. The local half was refuted by work in this crate; the family-wide half
was never this index's to hold, and survives only as the measurement above.

## Findings

**52 findings, four per definition**, each recorded three times: as a `### HDn`
section at the end of the instance that measured it, as a row in that
definition's own `### Findings Recorded Here` table, and here. This table is the
only copy ordered by ID.

**Sixteen carry a record tier** — six `**wrong doc**`, five `**latent hazard**`,
four `**misleading doc**`, one `**measured cost**` — and the remaining
thirty-six are observations, drift, gaps and coverage notes. Two of the closed
vocabulary's thirteen tiers go unused here: nothing in this crate was found
unadopted, and nothing was found to be a diagnostics problem.

| ID | Finding | Subject | Tier | Where |
|----|---------|---------|------|-------|
| HD1 | The instance documents one procedure in six steps and the surface implements it as three separate calls, so no step boundary corresponds to a call boundary a caller can observe | the split procedure | n/a — inconsistency | [algorithm/001](../algorithm/001_splitting_a_ring_into_two_ends.md) |
| HD2 | The allocation the procedure's second step describes happens in `ring_core`, which this crate depends on but whose allocating call it cannot name — the step describes work no line of this crate performs | step 2 | n/a — observation | [algorithm/001](../algorithm/001_splitting_a_ring_into_two_ends.md) |
| HD3 | The delegation procedure's final step is that nothing is added on the way back, which is an absence: no assertion, no test and no gate checks it, and the only thing standing there is review | step 6 | n/a — unenforced | [algorithm/002](../algorithm/002_delegating_to_the_backend.md) |
| HD4 | Neither documented procedure has a failure path, and `ring_config` accepts a configuration that would give one — the procedures are total because the constructor already rejected everything that would make them partial | the two procedures | n/a — observation | [algorithm/002](../algorithm/002_delegating_to_the_backend.md) |
| HD5 | The producer surface's Error Handling table names three error types and none of them exists anywhere in the family's source — the surface's only refusal is the returned record itself | the Error Handling table | **wrong doc** | [api/001](../api/001_producer_surface.md) |
| HD6 | The crate's only constructor takes a `ring_core::Ring`, which a consumer outside `ring_*` cannot name, so the audience the export surface exists for cannot call it and must reach the crate through `ring_factory` | `Split::new` | n/a — observation | [api/001](../api/001_producer_surface.md) |
| HD7 | Every refusal on the draining surface is a bare `Option::None` carrying no reason, so empty, closed and drained are one observable state and a caller cannot distinguish them | `try_recv` | n/a — observation | [api/002](../api/002_consumer_surface.md) |
| HD8 | The only `Vec` in the crate's public surface is the caller-supplied output buffer on the draining side; the publishing side takes an iterator and allocates nothing | `try_recv_batch` | n/a — observation | [api/002](../api/002_consumer_surface.md) |
| HD9 | `Drain`'s bound is captured when `drain()` is called and never re-read, so a producer publishing during the drain is not seen — nothing in the type's name or signature says the bound is a snapshot | `Drain` | **latent hazard** | [data_structure/002](../data_structure/002_the_one_struct_that_is_not_a_newtype.md) |
| HD10 | `Drain` is the only structure in the crate whose correctness argument is this crate's own; every other type's behaviour is `ring_core`'s, forwarded | `Drain` | n/a — observation | [data_structure/002](../data_structure/002_the_one_struct_that_is_not_a_newtype.md) |
| HD11 | "No per-handle state of any kind" is true of this crate's two structs and false of what they wrap: `ring_core::Producer` carries an `OverflowPolicy` field, which is why the two handles measure different widths | the D2 shape | **misleading doc** | [data_structure/001](../data_structure/001_two_handles_over_one_backend.md) |
| HD12 | The ergonomic tax D2 was priced against — callers threading a lifetime through their own types — is paid by nobody: no crate in the family stores a handle in a struct of its own | the D2 tradeoff | n/a — observation | [data_structure/001](../data_structure/001_two_handles_over_one_backend.md) |
| HD13 | This instance's status line reads open and defers further resolution; `ring_factory/docs/decisions/001` is accepted and answers it, so the question is closed in one file and open in the other | the status line | n/a — drift | [decisions/001](../decisions/001_what_this_crate_is_for.md) |
| HD14 | N4's row says three `ring_core::Ring` methods are withheld and the count is four, and one of the four — `ends()` — is not a read but the construction route the narrowing is about | narrowing N4 | **wrong doc** | [decisions/001](../decisions/001_what_this_crate_is_for.md) |
| HD15 | The accepted option composes `is_closed` around a `Shutdown` value, and this crate neither produces nor accepts one — the composition it describes has no site in this crate's surface | the chosen option | n/a — inconsistency | [decisions/002](../decisions/002_why_is_closed_is_absent.md) |
| HD16 | `ring_poll` is named seven times across this crate's source, tests and manual plan and executed zero times: neither test file reads a manifest, so the coupling is entirely prose in this direction | the `ring_poll` coupling | n/a — observation | [decisions/002](../decisions/002_why_is_closed_is_absent.md) |
| HD17 | Every one of the twelve public methods obeys the three-way rule and no line of the source states it — the rule exists only in this instance, so an edit that breaks it reads as ordinary | the forward-narrow-or-add rule | n/a — doc gap | [pattern/002](../pattern/002_forward_narrow_or_add.md) |
| HD18 | The rule's middle category has exactly one member — `drain`, narrowed by a call-time bound — and the crate spends seven compile-fail cases enforcing the boundaries around it | the narrow category | n/a — observation | [pattern/002](../pattern/002_forward_narrow_or_add.md) |
| HD19 | Two of the rule's three kinds compile to nothing measurable — the eight bare forwards and the two rewrappings are the width of what they wrap — and the third, `Drain`, is the only one with a structure and a cost of its own | the three kinds | **measured cost** | [pattern/002](../pattern/002_forward_narrow_or_add.md) |
| HD20 | This instance names three axes as uncovered by the compile-fail suite and the suite covers two of them: the Clone axis has three cases and the Sync axis one, leaving only the accessor axis genuinely open | the withheld-property list | n/a — drift | [pattern/001](../pattern/001_enforce_by_withholding.md) |
| HD21 | "The identical test suite runs against every backend" is two tests, and the crossbeam one is behind a feature this crate's own source never uses, so the default build proves parity for one backend | the backend parity claim | **misleading doc** | [integration/001](../integration/001_one_dependency_and_the_backends_beneath.md) |
| HD22 | A4 is the seam table's only entry marked bidirectional and it has no edge in either direction: `ring_poll` names no type of this crate and this crate's manifest names no `ring_poll` | seam A4 | n/a — observation | [integration/001](../integration/001_one_dependency_and_the_backends_beneath.md) |
| HD23 | The instance's argument that gate G5's confinement half is vacuous is correct, and both pieces of evidence it cites have since moved — the single `export_surface.txt` became one file per family, and the gate now resolves the list per family | the confinement argument | n/a — drift | [integration/002](../integration/002_on_the_export_surface.md) |
| HD24 | B4 justifies `ring_types`' place on the Contract by the error type a consumer must name, and this crate exercises none of it: `ring_types` is a dev-dependency here and no public signature of this crate mentions it | the `ring_types` row | n/a — coverage | [integration/002](../integration/002_on_the_export_surface.md) |
| HD25 | The zero-cost claim is asserted for `Producer` and `Consumer`, the two structs that cannot acquire state without the assertion failing, and not for `Drain`, the one that carries a field of its own | the five nouns | n/a — coverage | [item/001](../item/001_five_nouns_four_of_them_the_same_width.md) |
| HD26 | `Drain` is the only one of the five nouns with a field and the only one with no counterpart in `ring_core` — the crate's single piece of original structure is also its single unmirrored one | `Drain` | n/a — observation | [item/001](../item/001_five_nouns_four_of_them_the_same_width.md) |
| HD27 | Of twelve public methods, eight forward a single `ring_core` call unchanged, two rewrap a returned value, one is new, and the twelfth is the withheld `try_clone` that is the crate's actual contribution | the twelve verbs | n/a — observation | [item/002](../item/002_twelve_verbs_eight_bare_forwards.md) |
| HD28 | `drain` is the only verb whose body is not a forward or a rewrap, which makes it the only one that can be wrong in a way `ring_core`'s own tests would not catch | `drain` | n/a — observation | [item/002](../item/002_twelve_verbs_eight_bare_forwards.md) |
| HD29 | L6 says the ring drops when the handles do, and the crate's own drop test drops the `Split` rather than the handles — what is measured is the owner's drop, not the phase the row describes | phase L6 | **misleading doc** | [lifecycle/001](../lifecycle/001_split_move_and_drop.md) |
| HD30 | E2's debt row asks for a test proving the barrier is the only drain point, and `ring_poll`'s suite now contains one covering exactly that — the row records the debt as outstanding | the E2 debt row | n/a — drift | [lifecycle/002](../lifecycle/002_the_barrier_holds_the_consumer.md) |
| HD31 | M9's row calls cloning the one forbidden transition with no compile-time detector, and the Tests table eleven rows later cites the case that detects it | transition M9 | n/a — inconsistency | [lifecycle/003](../lifecycle/003_handle_ownership.md) |
| HD32 | The preamble relocates one observation to a sibling instance and the entire transition set went with it, leaving a state machine whose states are described here and whose transitions are argued elsewhere | the preamble | n/a — observation | [lifecycle/004](../lifecycle/004_ring_liveness_through_a_handle.md) |
| HD33 | The threshold this instance grades against names four compile-fail cases and `tests/ui_test.rs` drives seven — three cases the crate added beyond the criterion are ungraded by it | the acceptance threshold | n/a — drift | [non_functional_requirement/001](../non_functional_requirement/001_proven_by_code_that_must_not_compile.md) |
| HD34 | P7's redder-not-greener evidence comes from a manual run recorded when the suite had five cases; two of the seven that exist now have never been shown to fail for the reason they were written | measurement P7 | n/a — coverage | [non_functional_requirement/001](../non_functional_requirement/001_proven_by_code_that_must_not_compile.md) |
| HD35 | The handle pair borrows from the `Split` and so cannot move to a `'static` thread at all; Q3 passes because the only two-thread test uses `std::thread::scope`, which the criterion does not mention | criterion Q3 | n/a — observation | [non_functional_requirement/002](../non_functional_requirement/002_send_without_sync.md) |
| HD36 | The crate's one two-thread test has no bounded loop, so a lost publication hangs it rather than failing it, and measurement 5 — the sanitizer run that would distinguish the two — is not run anywhere in the crate or its gates | measurement 5 | n/a — coverage | [non_functional_requirement/002](../non_functional_requirement/002_send_without_sync.md) |
| HD37 | The one fixture coupled to another crate's private type documents that it is coupled and not what the coupling costs — no row states what breaks, where, or who finds out first | the pinned `.stderr` | n/a — doc gap | [pitfall/002](../pitfall/002_a_pinned_diagnostic_is_a_dependency_nobody_declared.md) |
| HD38 | A positive test and a compile-fail case assert the same property, and only the compile-fail case breaks when an unrelated crate renames a private type | the two detectors | n/a — duplication | [pitfall/002](../pitfall/002_a_pinned_diagnostic_is_a_dependency_nobody_declared.md) |
| HD39 | Three of the eight Caught-by cells name a mechanism in another crate and none of the three exists: `ring_poll` has no compile-fail suite, its bounded-time test never names this crate, and `ring_spsc` has no counting shim | the Caught by column | **wrong doc** | [pitfall/001](../pitfall/001_a_convenience_method_undoes_the_crate.md) |
| HD40 | The one mitigation priced as future work is already built and wider than proposed — seven forbidden names, as a test rather than a CI grep — and the name-list mechanism cannot in principle catch F6, which parks without naming anything | the mitigation table | n/a — drift | [pitfall/001](../pitfall/001_a_convenience_method_undoes_the_crate.md) |
| HD41 | The Definition table gives the handle's size as one pointer and the Validation table gives 24 bytes for the producer and 16 for the consumer; the suite asserts an equality with the wrapped type and pins no absolute figure at all | the Definition table | n/a — inconsistency | [type/001](../type/001_producer.md) |
| HD42 | The row places `Producer` on the family Contract, which is a list of five crate names rather than type names — the coincidence that this crate exports exactly five types is what makes the error read as plausible | the Exported row | **wrong doc** | [type/001](../type/001_producer.md) |
| HD43 | Four rows in the crate declare a rule undetectable and two are contradicted by a Tests row in the same file; the two that are still accurate are the two nobody wrote a test for | the Enforced by column | n/a — drift | [type/002](../type/002_consumer.md) |
| HD44 | Both stated `Debug` constraints are decided four crates down by a hand-written impl in `ring_spsc` that gives a different reason, and the local test would pass a `Debug` that printed every pending record | the `Debug` derive | **latent hazard** | [type/002](../type/002_consumer.md) |
| HD45 | The workaround readme says this crate depends on no published crate and the manifest declares `trybuild = "1.0"` — the workaround itself is what introduced the dependency the claim denies | the dependency claim | **wrong doc** | [workaround/001](../workaround/001_asserting_an_absence_needs_a_second_compiler_run.md) |
| HD46 | The command that regenerates a stale expected-output file and the command that silently accepts a real regression are the same command, and nothing distinguishes the two uses after the fact | `TRYBUILD=overwrite` | **latent hazard** | [workaround/001](../workaround/001_asserting_an_absence_needs_a_second_compiler_run.md) |
| HD47 | One pinned `.stderr` names two crates this one has no dependency edge to, so a rename in either produces a failure in a crate that never mentions them | the expected-output files | n/a — observation | [workaround/002](../workaround/002_the_expected_output_names_crates_this_one_never_sees.md) |
| HD48 | A private `ring_core` type is pinned verbatim in this crate's expected compiler output, making a name no consumer can reach load-bearing in another crate's fixtures | `ring_core::ProducerInner` | **latent hazard** | [workaround/002](../workaround/002_the_expected_output_names_crates_this_one_never_sees.md) |
| HD49 | The Enforcement table records compile-time detection for the property V4 says nothing detects — the Detected when column means "when a violation is caught" in two rows and "when the property is currently true" in a third | the Enforcement table | **misleading doc** | [invariant/001](../invariant/001_capability_follows_the_handle.md) |
| HD50 | The family answers the cardinality question twice — at runtime by `ring_core::Producer::try_clone`, which grants a second producer on an MPSC ring, and by absence here, which refuses for every backend — and only the absence is written down | `try_clone` | n/a — doc gap | [invariant/001](../invariant/001_capability_follows_the_handle.md) |
| HD51 | The graph guard reads every sibling manifest and classifies on the single substring `ring_wait`, so an edge to `ring_shutdown` or `ring_barrier` — both on its own roster — leaves the measured set unchanged and the suite green | `PARKING_CRATES` | **latent hazard** | [invariant/002](../invariant/002_no_parking_operation_is_reachable.md) |
| HD52 | "Every method reachable from a handle returns a `Result` or an `Option`" is true of two of the twelve methods, and the file's own Tests row names the counterexamples and explains why the shape was the wrong thing to ask for | the invariant statement | **wrong doc** | [invariant/002](../invariant/002_no_parking_operation_is_reachable.md) |
