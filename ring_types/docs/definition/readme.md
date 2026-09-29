# Doc Definitions

Module Index for `ring_types` — every doc definition this crate declares and every
instance under each, in one place.

## Master Doc Definitions Table

| Type | Purpose | Master File | Instances |
|------|---------|-------------|----------:|
| `algorithm/` | The one validation the crate performs, and the classification it performs without ever being asked to | [algorithm/readme.md](../algorithm/readme.md) | 2 |
| `api/` | Five types exported to thirty crates, and five predicates exported to none | [api/readme.md](../api/readme.md) | 2 |
| `data_structure/` | Two position types and the fold between them; an error set closed by `Copy` rather than by ceremony | [data_structure/readme.md](../data_structure/readme.md) | 2 |
| `decisions/` | Open questions and the fixes deliberately not applied, kept separate from the instances that found them | [decisions/readme.md](../decisions/readme.md) | 2 |
| `integration/` | Thirty dependents and one that declined the shared error — the crate's position read from both ends | [integration/readme.md](../integration/readme.md) | 2 |
| `invariant/` | Three properties on one axis: how much of each the compiler actually holds | [invariant/readme.md](../invariant/readme.md) | 3 |
| `item/` | Two readings of the callable surface, over a nested catalogue of all 40 source items with their measured call sites | [item/readme.md](../item/readme.md) | 2 |
| `lifecycle/` | Two values traced across crate boundaries, and two traced through the states the design permits — including the fifth it forbids | [lifecycle/readme.md](../lifecycle/readme.md) | 4 |
| `non_functional_requirement/` | Three thresholds with measurement recipes — two met, one met by a mechanism outside `src/` | [non_functional_requirement/readme.md](../non_functional_requirement/readme.md) | 3 |
| `pattern/` | Behaviour pushed out, a check pulled in — opposite directions, one protected property | [pattern/readme.md](../pattern/readme.md) | 2 |
| `pitfall/` | A doc comment that contradicts its own function, and two errors nothing constructs | [pitfall/readme.md](../pitfall/readme.md) | 2 |
| `type/` | The two types with enough internal structure to be worth a domain reading | [type/readme.md](../type/readme.md) | 2 |
| `workaround/` | The one external constraint a dependency-free crate can still have: the language itself | [workaround/readme.md](../workaround/readme.md) | 2 |

## Master Doc Instances Table

| Definition | ID | Name | File |
|------------|----|------|------|
| `algorithm/` | 001 | Validating a Slot Count to a Power of Two | [001_validating_a_slot_count_to_a_power_of_two.md](../algorithm/001_validating_a_slot_count_to_a_power_of_two.md) |
| `algorithm/` | 002 | Classifying an Error Into Configuration or Traffic | [002_classifying_an_error_into_configuration_or_traffic.md](../algorithm/002_classifying_an_error_into_configuration_or_traffic.md) |
| `api/` | 001 | The Vocabulary Surface | [001_the_vocabulary_surface.md](../api/001_the_vocabulary_surface.md) |
| `api/` | 002 | The Five Classifier Predicates | [002_the_five_classifier_predicates.md](../api/002_the_five_classifier_predicates.md) |
| `data_structure/` | 001 | Two Position Types and the Fold Between Them | [001_two_position_types_and_the_fold_between_them.md](../data_structure/001_two_position_types_and_the_fold_between_them.md) |
| `data_structure/` | 002 | The Error Enum as a Closed Copy Set | [002_the_error_enum_as_a_closed_copy_set.md](../data_structure/002_the_error_enum_as_a_closed_copy_set.md) |
| `decisions/` | 001 | "One Error Type" Is a Rule the Family Does Not Keep | [001_one_error_type_is_a_rule_the_family_does_not_keep.md](../decisions/001_one_error_type_is_a_rule_the_family_does_not_keep.md) |
| `decisions/` | 002 | The Two Name Errors, and the Two Crates That Redeclared Them | [002_the_two_name_errors_and_the_two_crates_that_redeclared_them.md](../decisions/002_the_two_name_errors_and_the_two_crates_that_redeclared_them.md) |
| `integration/` | 001 | The Crate Thirty-One of Thirty-Three Depend On | [001_the_crate_thirty_one_of_thirty_three_depend_on.md](../integration/001_the_crate_thirty_one_of_thirty_three_depend_on.md) |
| `integration/` | 002 | The Registry That Declined the Shared Error | [002_the_registry_that_declined_the_shared_error.md](../integration/002_the_registry_that_declined_the_shared_error.md) |
| `invariant/` | 001 | Every Capacity Has a Valid Mask | [001_every_capacity_has_a_valid_mask.md](../invariant/001_every_capacity_has_a_valid_mask.md) |
| `invariant/` | 002 | Tier Zero Depends on Nothing | [002_tier_zero_depends_on_nothing.md](../invariant/002_tier_zero_depends_on_nothing.md) |
| `invariant/` | 003 | Every Error Renders Distinctly | [003_every_error_renders_distinctly.md](../invariant/003_every_error_renders_distinctly.md) |
| `item/` | 001 | The Forty Items, and the Six the Family Calls | [001_the_forty_items_and_the_six_the_family_calls.md](../item/001_the_forty_items_and_the_six_the_family_calls.md) |
| `item/` | 002 | Three Newtypes, and Two Open Fields | [002_three_newtypes_and_two_open_fields.md](../item/002_three_newtypes_and_two_open_fields.md) |
| `lifecycle/` | 001 | A Slot Count From Request to Mask | [001_a_slot_count_from_request_to_mask.md](../lifecycle/001_a_slot_count_from_request_to_mask.md) |
| `lifecycle/` | 002 | An Error From Construction to Display | [002_an_error_from_construction_to_display.md](../lifecycle/002_an_error_from_construction_to_display.md) |
| `lifecycle/` | 003 | A Capacity Request Through Validation | [003_a_capacity_request_through_validation.md](../lifecycle/003_a_capacity_request_through_validation.md) |
| `lifecycle/` | 004 | An Overflow Policy From Declaration to Refusal | [004_an_overflow_policy_from_declaration_to_refusal.md](../lifecycle/004_an_overflow_policy_from_declaration_to_refusal.md) |
| `non_functional_requirement/` | 001 | Errors and Positions Do Not Allocate | [001_errors_and_positions_do_not_allocate.md](../non_functional_requirement/001_errors_and_positions_do_not_allocate.md) |
| `non_functional_requirement/` | 002 | The Enum Sets Are Closed and Asserted | [002_the_enum_sets_are_closed_and_asserted.md](../non_functional_requirement/002_the_enum_sets_are_closed_and_asserted.md) |
| `non_functional_requirement/` | 003 | The Crate Compiles Without std | [003_the_crate_compiles_without_std.md](../non_functional_requirement/003_the_crate_compiles_without_std.md) |
| `pattern/` | 001 | Discriminants Here, Handlers Elsewhere | [001_discriminants_here_handlers_elsewhere.md](../pattern/001_discriminants_here_handlers_elsewhere.md) |
| `pattern/` | 002 | A Newtype That Makes a Check Unnecessary | [002_a_newtype_that_makes_a_check_unnecessary.md](../pattern/002_a_newtype_that_makes_a_check_unnecessary.md) |
| `pitfall/` | 001 | Seq::next Wraps Where Its Doc Says It Saturates | [001_seq_next_wraps_where_its_doc_says_it_saturates.md](../pitfall/001_seq_next_wraps_where_its_doc_says_it_saturates.md) |
| `pitfall/` | 002 | Two Name Errors Nothing Constructs | [002_two_name_errors_nothing_constructs.md](../pitfall/002_two_name_errors_nothing_constructs.md) |
| `type/` | 001 | Capacity | [001_capacity.md](../type/001_capacity.md) |
| `type/` | 002 | RingError | [002_ring_error.md](../type/002_ring_error.md) |
| `workaround/` | 001 | Hand-Written ALL Arrays Stand In for Variant Enumeration | [001_hand_written_all_arrays_stand_in_for_variant_enumeration.md](../workaround/001_hand_written_all_arrays_stand_in_for_variant_enumeration.md) |
| `workaround/` | 002 | The Conversion That Cannot Be a Trait | [002_the_conversion_that_cannot_be_a_trait.md](../workaround/002_the_conversion_that_cannot_be_a_trait.md) |

## Findings

Fifty-two, each argued in the instance named under **Where** and verified there
by a command whose output is quoted beneath it.

| ID | Finding | Subject | Reachable | Where |
|----|---------|---------|-----------|-------|
| TY1 | `src/error.rs:1` states this is the one error type the whole ring family returns; the workspace declares six, five of them in other crates | `ring_types` | **misleading doc** | [decisions/001](../decisions/001_one_error_type_is_a_rule_the_family_does_not_keep.md) |
| TY2 | This crate's own P2 entry names `ring_registry` as the single counterexample to the one-error-type claim; four more exist and none is named anywhere in the corpus | `ring_types` | n/a — doc gap | [decisions/001](../decisions/001_one_error_type_is_a_rule_the_family_does_not_keep.md) |
| TY3 | Not one of the six error enums carries an `impl From` for any other; `ring_bench::RunError` unions three of them by hand-written variant instead | ring family | n/a — observation | [decisions/001](../decisions/001_one_error_type_is_a_rule_the_family_does_not_keep.md) |
| TY4 | A Contract-bound consumer receives `RingError` from the ring-building crate only inside `BuildError::Unsupported`, and cannot name `RegistryError` at all | `ring_factory` | n/a — observation | [decisions/001](../decisions/001_one_error_type_is_a_rule_the_family_does_not_keep.md) |
| TY5 | Two `#[ non_exhaustive ]` attributes across 23 public enums — `RingError` marked, `WaitKind` and `OverflowPolicy` beside it not — and no crate matches on a `RingError` value at all, so it currently costs nothing and buys nothing | `ring_types` | n/a — inconsistency | [decisions/001](../decisions/001_one_error_type_is_a_rule_the_family_does_not_keep.md) |
| TY6 | `RingError::NameTaken` is constructed nowhere while `ring_factory::BuildError::NameTaken` and `ring_registry::RegistryError::NameTaken` are each constructed and each public | ring family | n/a — duplication | [decisions/002](../decisions/002_the_two_name_errors_and_the_two_crates_that_redeclared_them.md) |
| TY7 | No lookup path in `ring_registry` — the crate whose lookup it names — mentions `RingError::NameUnknown` | `ring_types` | n/a — coverage | [decisions/002](../decisions/002_the_two_name_errors_and_the_two_crates_that_redeclared_them.md) |
| TY8 | The only one of the 31 declared dependents that names no item from `ring_types` in `src/`; the manifest edge outlived the decision that created it | `ring_registry` | n/a — unadopted | [decisions/002](../decisions/002_the_two_name_errors_and_the_two_crates_that_redeclared_them.md) |
| TY9 | `is_configuration`, `is_transient`, `reports_failure` and `drops_silently` are called from no library outside this crate; `is_non_blocking` is called once | ring family | n/a — coverage | [item/001](../item/001_the_forty_items_and_the_six_the_family_calls.md) |
| TY10 | One of 113 references across `ring_*/src` is a call — TY20's fix removed the other; a reach figure that does not strip comment lines overstates this crate by roughly a hundredfold | ring family | n/a — coverage | [item/001](../item/001_the_forty_items_and_the_six_the_family_calls.md) |
| TY11 | The method that encodes the power-of-two invariant is the least-used thing the type exports, and both its callers use it for the same expression | ring family | n/a — observation | [item/001](../item/001_the_forty_items_and_the_six_the_family_calls.md) |
| TY12 | No free function, trait, static, type alias or macro — 40 items across 7 of the taxonomy 18 kinds, so a consumer cannot partially adopt the crate | `ring_types` | n/a — observation | [item/001](../item/001_the_forty_items_and_the_six_the_family_calls.md) |
| TY13 | `Capacity`'s field is private and `Seq`'s and `SlotIndex`'s are `pub`; one of three newtype invariants is in the type system and two are in prose | `ring_types` | n/a — inconsistency | [item/002](../item/002_three_newtypes_and_two_open_fields.md) |
| TY14 | `Seq::next` and `advanced_by` are conveniences over an open field, so no monotonicity property of a received `Seq` can be relied on by its receiver | ring family | n/a — unenforced | [item/002](../item/002_three_newtypes_and_two_open_fields.md) |
| TY15 | Production code uses `Seq::ZERO` exclusively; the spelling the documentation demonstrates is the one that would stop compiling if the field were sealed | ring family | n/a — inconsistency | [item/002](../item/002_three_newtypes_and_two_open_fields.md) |
| TY16 | Against `RingError`'s 18 crates, `Seq`'s 16 and `Capacity`'s 11 — the least-adopted exported name, used only by the three crates owning the sequence-to-slot fold | ring family | n/a — observation | [item/002](../item/002_three_newtypes_and_two_open_fields.md) |
| TY17 | `Capacity::new` cannot be `TryFrom< usize >` and stay `const fn`, so zero `try_into()` uses and zero `TryFrom` bounds exist across 31 dependents | ring family | **measured cost** | [workaround/002](../workaround/002_the_conversion_that_cannot_be_a_trait.md) |
| TY18 | No `From` and no `TryFrom` on any of its four types — the same absence that leaves the six error enums unable to compose | `ring_types` | n/a — observation | [workaround/002](../workaround/002_the_conversion_that_cannot_be_a_trait.md) |
| TY19 | The one production call to `Capacity::new` is `ring_config/src/lib.rs:71` — `ring_core/src/lib.rs:227` was the other until TY20's fix removed it; the other 30 crates receive an already-validated value and never re-check | ring family | n/a — coverage | [algorithm/001](../algorithm/001_validating_a_slot_count_to_a_power_of_two.md) |
| TY20 | `ring_core::Ring::capacity` no longer calls `Capacity::new` at all — the crossbeam variant now carries its already-validated `Capacity` instead of rebuilding one, so the accessor that once could abort on a rounding backend no longer re-runs the check | `ring_core` | **latent hazard** | [algorithm/001](../algorithm/001_validating_a_slot_count_to_a_power_of_two.md) |
| TY21 | `is_configuration` and `is_transient` are named in `ring_gating` and `ring_shutdown` respectively, and both mentions are `///` examples rather than calls | ring family | n/a — coverage | [algorithm/002](../algorithm/002_classifying_an_error_into_configuration_or_traffic.md) |
| TY22 | `Closed`, `NameTaken` and `NameUnknown` are neither configuration nor transient, and no predicate names that third class | `ring_types` | n/a — observation | [algorithm/002](../algorithm/002_classifying_an_error_into_configuration_or_traffic.md) |
| TY23 | `RingError` is named in 18 dependent crates, `Seq` 16, `Capacity` 11, `WaitKind` 4, `OverflowPolicy` 4 and `SlotIndex` 3 — a six-fold spread across one export surface | ring family | n/a — observation | [api/001](../api/001_the_vocabulary_surface.md) |
| TY24 | The crate spent both attributes on every classifier, and four of the five have no production caller anywhere in the family to benefit from either | `ring_types` | n/a — observation | [api/002](../api/002_the_five_classifier_predicates.md) |
| TY25 | `ring_config` is the single production caller of any classifier this crate exports | `ring_config` | n/a — coverage | [api/002](../api/002_the_five_classifier_predicates.md) |
| TY26 | `ring_index:42` and `ring_mpsc:543` both fold a `Seq` with `( seq.0 as usize ) & ... .mask()`, and `ring_index` is the crate that exists to own that fold | `ring_mpsc` | n/a — duplication | [data_structure/001](../data_structure/001_two_position_types_and_the_fold_between_them.md) |
| TY27 | Neither call site can be written without `Seq`'s `pub` field, so the fold and the encapsulation asymmetry are the same fact seen twice | ring family | n/a — observation | [data_structure/001](../data_structure/001_two_position_types_and_the_fold_between_them.md) |
| TY28 | The `Display` impl matches without a wildcard, so adding a variant is a compile error there — the one drift in this crate the compiler catches by itself | `ring_types` | n/a — observation | [data_structure/002](../data_structure/002_the_error_enum_as_a_closed_copy_set.md) |
| TY29 | `ring_registry` is the one declared dependent naming no item from `ring_types` in `src/`, so the manifest graph overstates the code graph by exactly one edge | ring family | n/a — unadopted | [integration/001](../integration/001_the_crate_thirty_one_of_thirty_three_depend_on.md) |
| TY30 | Construction reach per variant ranges from 8 dependent crates down to 0, and the four with none include both name variants and both capacity variants | ring family | n/a — observation | [integration/001](../integration/001_the_crate_thirty_one_of_thirty_three_depend_on.md) |
| TY31 | 30 crates declare the dependency and the most-used exported name reaches 18, so most dependents adopt a small slice of a small crate | ring family | n/a — observation | [integration/001](../integration/001_the_crate_thirty_one_of_thirty_three_depend_on.md) |
| TY32 | `RingError::NameTaken`, `BuildError::NameTaken` and `RegistryError::NameTaken` all mean a ring is already registered under this name, and two of the three render the identical message | ring family | n/a — duplication | [integration/002](../integration/002_the_registry_that_declined_the_shared_error.md) |
| TY33 | It converts `RegistryError::NameTaken` into its own `BuildError::NameTaken` at `ring_factory:214`, so the Contract consumer never learns which crate refused | `ring_factory` | n/a — doc gap | [integration/002](../integration/002_the_registry_that_declined_the_shared_error.md) |
| TY34 | `is_power_of_two` and `count_ones() == 1` have zero occurrences across all 32 other crates, which is the invariant working as an absence | ring family | n/a — observation | [invariant/001](../invariant/001_every_capacity_has_a_valid_mask.md) |
| TY35 | One `Capacity::new` call establishes it now (`ring_config/src/lib.rs:71`) — TY20 removed the other; 29 `.get()` reads and 2 `.mask()` reads consume it without re-checking | ring family | n/a — coverage | [invariant/001](../invariant/001_every_capacity_has_a_valid_mask.md) |
| TY36 | The acyclicity property the family rests on is stated in prose and checked by no gate; the six scripts under `bench_harness/gate/` check the corpus, not the manifest | `ring_types` | n/a — unenforced | [invariant/002](../invariant/002_tier_zero_depends_on_nothing.md) |
| TY37 | `#![ no_std ]` is now declared (`src/lib.rs:27`), so the property holds because a `std::` path fails to compile, not because none happens to be written | `ring_types` | n/a — unenforced | [invariant/002](../invariant/002_tier_zero_depends_on_nothing.md) |
| TY38 | `every_error_displays_distinctly` checks non-emptiness, pairwise distinctness and two embedded values; permuting all nine messages leaves it green | `ring_types` | n/a — unenforced | [invariant/003](../invariant/003_every_error_renders_distinctly.md) |
| TY39 | Only the request and validation phases run here; the mask, index, store and read phases run in crates holding a `Capacity` they cannot have constructed wrongly | ring family | n/a — coverage | [lifecycle/001](../lifecycle/001_a_slot_count_from_request_to_mask.md) |
| TY40 | Only the two capacity variants are constructed inside `ring_types`; five are constructed by dependents and two by nothing at all | ring family | n/a — observation | [lifecycle/002](../lifecycle/002_an_error_from_construction_to_display.md) |
| TY41 | No test asserts the fifth state is unreachable and none can; its emptiness follows structurally from `Capacity` having no public constructor other than `new` | `ring_types` | n/a — observation | [lifecycle/003](../lifecycle/003_a_capacity_request_through_validation.md) |
| TY42 | `ring_core:154` is the only place a policy is refused; `ring_factory` re-wraps that error and constructs none of its own | `ring_core` | n/a — coverage | [lifecycle/004](../lifecycle/004_an_overflow_policy_from_declaration_to_refusal.md) |
| TY43 | `ring_wait` matches all four `WaitKind` variants and `ring_overflow` all three `OverflowPolicy` variants with no wildcard, so a new variant is a compile error where it matters | ring family | n/a — observation | [lifecycle/004](../lifecycle/004_an_overflow_policy_from_declaration_to_refusal.md) |
| TY44 | `Copy` on `RingError` forbids a `String` payload, which is why `ring_registry` declared `RegistryError` and why three crates now spell the same name collision | ring family | **measured cost** | [non_functional_requirement/001](../non_functional_requirement/001_errors_and_positions_do_not_allocate.md) |
| TY45 | `overflow_policy_has_no_overwrite_variant` matched all three variants in one arm, so a fourth failed the build, but a repeated entry in `ALL` passed every test in the workspace until the `contains` loop closed it (→ `002`, T6) | `ring_types` | **latent hazard** | [non_functional_requirement/002](../non_functional_requirement/002_the_enum_sets_are_closed_and_asserted.md) |
| TY46 | `#![ no_std ]` is now declared (`src/lib.rs:27`), so the crate satisfies this requirement by being unable to contain a `std::` path, not merely by containing none | `ring_types` | n/a — unenforced | [non_functional_requirement/003](../non_functional_requirement/003_the_crate_compiles_without_std.md) |
| TY47 | `ring_wait` names no `OverflowPolicy` and `ring_overflow` names no `WaitKind`, so neither handler leaked into the other despite both discriminant sets being declared in one file | ring family | n/a — observation | [pattern/001](../pattern/001_discriminants_here_handlers_elsewhere.md) |
| TY48 | The newtype made re-validation unnecessary in 31 crates, and the only way to see that is that none of them contains a power-of-two check | ring family | n/a — observation | [pattern/002](../pattern/002_a_newtype_that_makes_a_check_unnecessary.md) |
| TY49 | `seq_does_not_wrap_within_any_reachable_workload` asserts arithmetic on `u64::MAX` and would pass unchanged if `Seq::next` were deleted | `ring_types` | n/a — unenforced | [pitfall/001](../pitfall/001_seq_next_wraps_where_its_doc_says_it_saturates.md) |
| TY50 | `NameTaken` and `NameUnknown` are classified, rendered and asserted by this crate suite, so coverage reports them as exercised while nothing in the workspace raises them | `ring_types` | n/a — observation | [pitfall/002](../pitfall/002_two_name_errors_nothing_constructs.md) |
| TY51 | `Seq`, `SlotIndex`, `WaitKind` and `OverflowPolicy` all derive or implement `Default`; `Capacity` and `RingError` do not, and only one of the two absences is deliberate | `ring_types` | n/a — observation | [type/001](../type/001_capacity.md) |
| TY52 | `CapacityNotPowerOfTwo( usize )` and `BatchTooLarge { requested, capacity }` carry data that only this crate's own tests ever destructure | ring family | n/a — coverage | [type/002](../type/002_ring_error.md) |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_types/docs
printf 'doc definitions:          '; ls -d */ | grep -vc '^definition/'
printf 'instances:                '; ls */[0-9][0-9][0-9]_*.md | wc -l
printf 'findings in the corpus:   '; grep -rhoE '^### TY[0-9]+ ' */[0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| TY[0-9]+ ' definition/readme.md
# doc definitions:          13
# instances:                30
# findings in the corpus:   52
# rows in the table below:  52
```
