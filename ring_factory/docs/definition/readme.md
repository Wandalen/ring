# Doc Definitions

Module Index for `ring_factory` — every doc definition this crate declares and every
instance under each, in one place.

## Master Doc Definitions Table

| Type | Purpose | Master File | Instances |
|------|---------|-------------|----------:|
| `algorithm/` | One boolean choosing between two unrelated types, and five fields consumed in a fixed order | [algorithm/readme.md](../algorithm/readme.md) | 2 |
| `api/` | One total function and one fallible one, and the argument type a compliant consumer cannot name | [api/readme.md](../api/readme.md) | 2 |
| `data_structure/` | A 32-byte record in, the pair's owner out, and what each end got wrong | [data_structure/readme.md](../data_structure/readme.md) | 2 |
| `decisions/` | Architecture Decision Records for open trade-offs this crate could not close unilaterally | [decisions/readme.md](../decisions/readme.md) | 2 |
| `integration/` | Five declared edges, a closure that has moved three times, and the two re-exports that close the Contract | [integration/readme.md](../integration/readme.md) | 2 |
| `invariant/` | The property the sweep rests on, and the restriction that does not currently hold | [invariant/readme.md](../invariant/readme.md) | 2 |
| `item/` | The nine declarations, split into the verbs and the nouns | [item/readme.md](../item/readme.md) | 2 |
| `lifecycle/` | A configuration becoming a ring and a name becoming a registration — as phases, and as the states each passes through | [lifecycle/readme.md](../lifecycle/readme.md) | 4 |
| `non_functional_requirement/` | This crate's own stated criterion, and the performance-isolation one it omits | [non_functional_requirement/readme.md](../non_functional_requirement/readme.md) | 2 |
| `pattern/` | Configuration as a value, and construction as its only consumer | [pattern/readme.md](../pattern/readme.md) | 2 |
| `pitfall/` | A field corrected before arrival and a field arriving intact with nothing to act on it | [pitfall/readme.md](../pitfall/readme.md) | 2 |
| `type/` | A fieldless service and a two-variant error, one variant of which arrived from a dependency | [type/readme.md](../type/readme.md) | 2 |
| `workaround/` | External constraints this crate absorbs, with costs and deletion conditions | [workaround/readme.md](../workaround/readme.md) | 2 |

**28 doc instances across 13 definitions**, carrying 52 findings — four per
definition, listed under [`## Findings`](#findings) below. **No `format/`** —
this crate defines no byte layout, wire encoding, or serialised representation;
it composes values whose layouts belong to `ring_types` and the backend crates.

**The `item/` line above used to read "none, matching every other crate in the
family", and both halves have expired:**

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'crates in the family:      %s\n' "$( ls -d ring_*/ | wc -l )"
printf 'of them with docs/item:    %s\n' "$( ls -d ring_*/docs/item 2>/dev/null | wc -l )"
printf 'instances in this crate:   %s\n' "$( ls ring_factory/docs/*/[0-9][0-9][0-9]_*.md | wc -l )"
printf 'findings in this crate:    %s\n' "$( command grep -rhcoE '^### FC[0-9]+ — ' ring_factory/docs/*/[0-9][0-9][0-9]_*.md | awk '{ n += $1 } END { print n + 0 }' )"
```

The count went from zero to twenty-six while this paragraph asserted zero, which
is the ordinary failure of a claim about *other* crates written into *this* one:
nothing that changes it touches this file. The family-wide half was never this
index's to hold, and is kept only as a measurement with the command beside it.

`decisions/` and `workaround/` carry a readme plus their own contents, by their
own conventions: `decisions/` indexes ADRs rather than doc instances — **ten
questions were recorded there as pending, the largest count in the family, and
eight are now closed** — and `workaround/` records two external constraints as
instances alongside its readme.

## Master Doc Instances Table

| Definition | ID | Name | File |
|------------|----|------|------|
| `algorithm/` | 001 | Selecting a Backend From One Boolean | [algorithm/001_selecting_a_backend_from_one_boolean.md](../algorithm/001_selecting_a_backend_from_one_boolean.md) |
| `algorithm/` | 002 | Assembling a Ring From a Validated Record | [algorithm/002_assembling_a_ring_from_a_validated_record.md](../algorithm/002_assembling_a_ring_from_a_validated_record.md) |
| `api/` | 001 | The Build Surface | [api/001_the_build_surface.md](../api/001_the_build_surface.md) |
| `api/` | 002 | The Named Build Surface | [api/002_the_named_build_surface.md](../api/002_the_named_build_surface.md) |
| `data_structure/` | 001 | The Configuration Record as Input | [data_structure/001_the_configuration_record_as_input.md](../data_structure/001_the_configuration_record_as_input.md) |
| `data_structure/` | 002 | The Handle Pair as Output — *and why the pair is not the output* | [data_structure/002_the_handle_pair_as_output.md](../data_structure/002_the_handle_pair_as_output.md) |
| `integration/` | 001 | The Declared Edges and the Reached Closure | [integration/001_declared_edges_and_the_reached_closure.md](../integration/001_declared_edges_and_the_reached_closure.md) |
| `integration/` | 002 | The Crate the Export Surface Routes Through | [integration/002_the_crate_the_export_surface_routes_through.md](../integration/002_the_crate_the_export_surface_routes_through.md) |
| `invariant/` | 001 | The Configuration Fully Determines the Ring | [invariant/001_configuration_fully_determines_the_ring.md](../invariant/001_configuration_fully_determines_the_ring.md) |
| `invariant/` | 002 | Construction Is the Only Path | [invariant/002_construction_is_the_only_path.md](../invariant/002_construction_is_the_only_path.md) |
| `item/` | 001 | Three Verbs, One of Them Conditional | [item/001_three_verbs_one_of_them_conditional.md](../item/001_three_verbs_one_of_them_conditional.md) |
| `item/` | 002 | Four Nouns, Two of Them Somebody Else's | [item/002_four_nouns_two_of_them_somebody_elses.md](../item/002_four_nouns_two_of_them_somebody_elses.md) |
| `lifecycle/` | 001 | From a Record to a Handle Pair | [lifecycle/001_from_a_record_to_a_handle_pair.md](../lifecycle/001_from_a_record_to_a_handle_pair.md) |
| `lifecycle/` | 002 | The Factory Outlives Nothing | [lifecycle/002_the_factory_outlives_nothing.md](../lifecycle/002_the_factory_outlives_nothing.md) |
| `non_functional_requirement/` | 001 | Five Fields, Asserted One at a Time | [non_functional_requirement/001_five_fields_asserted_one_at_a_time.md](../non_functional_requirement/001_five_fields_asserted_one_at_a_time.md) |
| `non_functional_requirement/` | 002 | Construction Cost Is Paid Once and Never on the Path | [non_functional_requirement/002_construction_cost_is_paid_once.md](../non_functional_requirement/002_construction_cost_is_paid_once.md) |
| `pattern/` | 001 | Configuration as Data | [pattern/001_configuration_as_data.md](../pattern/001_configuration_as_data.md) |
| `pattern/` | 002 | One Way In | [pattern/002_one_way_in.md](../pattern/002_one_way_in.md) |
| `pitfall/` | 001 | The Criterion Grades the Clamped Value | [pitfall/001_the_criterion_grades_the_clamped_value.md](../pitfall/001_the_criterion_grades_the_clamped_value.md) |
| `pitfall/` | 002 | A Wait Strategy It Can Read and Cannot Honour | [pitfall/002_a_wait_strategy_it_can_read_and_cannot_honour.md](../pitfall/002_a_wait_strategy_it_can_read_and_cannot_honour.md) |
| `lifecycle/` | 003 | Config State Through a Build | [lifecycle/003_config_state_through_a_build.md](../lifecycle/003_config_state_through_a_build.md) |
| `lifecycle/` | 004 | Name State Through a Registration | [lifecycle/004_name_state_through_a_registration.md](../lifecycle/004_name_state_through_a_registration.md) |
| `type/` | 001 | Factory | [type/001_factory.md](../type/001_factory.md) |
| `type/` | 002 | Build Error | [type/002_build_error.md](../type/002_build_error.md) |
| `workaround/` | 001 | Returning Ownership Through the Error | [workaround/001_returning_ownership_through_the_error.md](../workaround/001_returning_ownership_through_the_error.md) |
| `workaround/` | 002 | A Feature Cannot Be a Value | [workaround/002_a_feature_cannot_be_a_value.md](../workaround/002_a_feature_cannot_be_a_value.md) |
| `decisions/` | 001 | The Owner Is The Return Value | [decisions/001_the_owner_is_the_return_value.md](../decisions/001_the_owner_is_the_return_value.md) |
| `decisions/` | 002 | Two Doors, Not One That Routes | [decisions/002_two_doors_not_one_that_routes.md](../decisions/002_two_doors_not_one_that_routes.md) |

## Architecture Decision Records

| ADR | Rules | Status |
|---|---|---|
| [decisions/001_the_owner_is_the_return_value.md](../decisions/001_the_owner_is_the_return_value.md) | Pendings 1, 2, 3, 4 | accepted 2026-08-28 |
| [decisions/002_two_doors_not_one_that_routes.md](../decisions/002_two_doors_not_one_that_routes.md) | Pendings 9, 10 | accepted 2026-08-28 |

Indexed here and in [`decisions/readme.md`](../decisions/readme.md) only, not in
`graph.yml` — `doc_des.rulebook.md` classifies `docs/decisions/` as a
non-doc-definition directory.

## Reading order

[`decisions/001`](../decisions/001_the_owner_is_the_return_value.md) first — it
is why `build` returns what it returns, and four instances read differently
once it is known. Then [`pitfall/001`](../pitfall/001_the_criterion_grades_the_clamped_value.md)
for what the acceptance criterion actually grades,
[`type/002`](../type/002_build_error.md) for the refusal that arrived from a
dependency, and [`api/001`](../api/001_the_build_surface.md) for the surface
itself.

## Findings

**52 findings, four per definition**, each recorded as a `### FCn` section
at the end of its instance and as a row in its own definition readme's
`### Findings Recorded Here` table. This table is the third copy and the only
one ordered by ID.

| ID | Finding | Subject | Tier | Where |
|----|---------|---------|------|-------|
| FC1 | Three backends, two selection mechanisms, and neither can see the other: `is_multi_producer()` chooses between the in-house rings, a Cargo feature chooses crossbeam, and the crossbeam path reads only `capacity()` and `overflow()` — `is_multi_producer` is named zero times in this crate | `ring_factory` | n/a — observation | [algorithm/001](../algorithm/001_selecting_a_backend_from_one_boolean.md) |
| FC2 | The crate's only algorithm is a branch made one crate down, and no test at this crate's grain asserts which backend a build selected, because nothing on the return path reports it | `ring_factory` | n/a — coverage | [algorithm/001](../algorithm/001_selecting_a_backend_from_one_boolean.md) |
| FC3 | The assembly has no control flow at all — zero `if`, zero `return`, zero loops outside doc comments, and both `match` arms belong to error translation rather than assembly | `ring_factory` | n/a — observation | [algorithm/002](../algorithm/002_assembling_a_ring_from_a_validated_record.md) |
| FC4 | Two of the five fields the assembly consumes — `wait` and `batch` — reach nothing, and the only crate that reads either builds no rings | `ring_factory` | n/a — unenforced | [algorithm/002](../algorithm/002_assembling_a_ring_from_a_validated_record.md) |
| FC5 | The five-name export surface has zero consumers outside `ring_*`, so every guarantee this instance states is currently a promise to the family about itself | `ring_factory` | n/a — observation | [api/001](../api/001_the_build_surface.md) |
| FC6 | The re-export rule is derivable and correct — re-export exactly the non-Contract crates a caller is forced to name — and is stated nowhere; the surface test's own doc comment enumerates three of its four `ring_*` imports, omitting `ring_handle` | `ring_factory` | n/a — doc gap | [api/001](../api/001_the_build_surface.md) |
| FC7 | The Contract is five crates wide by declaration and six wide in use: `Registry` carries eight public methods through the re-export, and `ring_registry` is not on the surface | `ring_factory` | n/a — inconsistency | [api/002](../api/002_the_named_build_surface.md) |
| FC8 | `build_named` is the one operation with a refusal of its own and it cannot report which half failed — `BuildError` flattens "the ring could not be built" and "the name was taken" into two variants with no shared discriminant a caller can branch on before acting | `ring_factory` | **latent hazard** | [api/002](../api/002_the_named_build_surface.md) |
| FC9 | Three of `RingConfig`'s fields carry their constraint in the type and two carry it in a setter body, so the record's legality is half checkable by a reader and half only by running the setter | `ring_factory` | n/a — observation | [data_structure/001](../data_structure/001_the_configuration_record_as_input.md) |
| FC10 | Config equality compares five fields and the ring it builds depends on two, so two configs that are unequal can produce indistinguishable rings — the invariant holds in the direction stated and not in its converse | `ring_factory` | **latent hazard** | [data_structure/001](../data_structure/001_the_configuration_record_as_input.md) |
| FC11 | The owner is exactly the ring and the error rides in a niche: `Ring<u32>`, `Split<u32>` and `Result<Split<u32>, BuildError>` all measure 320 bytes, so the handle pair and its error channel are free | `ring_factory` | **measured cost** | [data_structure/002](../data_structure/002_the_handle_pair_as_output.md) |
| FC12 | A name collision constructs and drops 320 bytes to return 24 — `build_named` builds the ring before consulting the registry, and the refusal path pays the full construction | `ring_factory` | **measured cost** | [data_structure/002](../data_structure/002_the_handle_pair_as_output.md) |
| FC13 | The ruling is longer than the crate it rules: ADR 001 is 197 lines against `src/lib.rs`'s 76 non-doc lines, and produces two `pub use` lines and two `Ok( Split::new( ring ) )` statements | `ring_factory` | n/a — observation | [decisions/001](../decisions/001_the_owner_is_the_return_value.md) |
| FC14 | The decision was forced before it was taken — `ring_handle::Split::split` borrowing `&'a mut self` fixed the owner as the return value while the question was still recorded as open | `ring_factory` | n/a — observation | [decisions/001](../decisions/001_the_owner_is_the_return_value.md) |
| FC15 | The two doors have the same signature, so the ruling is enforced by a name: `build` and `build_crossbeam` differ only in identifier, and a caller who picks the wrong one gets a different backend with no type error | `ring_factory` | **latent hazard** | [decisions/002](../decisions/002_two_doors_not_one_that_routes.md) |
| FC16 | Eight of ten pendings are closed and the two left are not this ruling's residue; the residue it did create — `build_crossbeam`'s expiry condition — appears in no pending, no task, and no gate | `ring_factory` | n/a — doc gap | [decisions/002](../decisions/002_two_doors_not_one_that_routes.md) |
| FC17 | Every declared edge is used and three quarters of the manifest is about edges that are not: 15 comment lines against 5 dependency lines | `ring_factory` | n/a — observation | [integration/001](../integration/001_declared_edges_and_the_reached_closure.md) |
| FC18 | Nineteen crates in the reached closure, five declared, and the difference is invisible here — the manifest names direct edges and nothing in this crate states the transitive set | `ring_factory` | n/a — observation | [integration/001](../integration/001_declared_edges_and_the_reached_closure.md) |
| FC19 | G5's confinement half cannot fail, and the half that can is not the one this instance is about — a second instance of the vacuous-gate class first found in G6 | `ring_factory` | n/a — unenforced | [integration/002](../integration/002_the_crate_the_export_surface_routes_through.md) |
| FC20 | Eight empty `Error` impls across the family, five wrapping variants, and zero `source()` implementations, so every error chain in the family walks to length one | `ring_factory` | n/a — unadopted | [integration/002](../integration/002_the_crate_the_export_surface_routes_through.md) |
| FC21 | The enforcement mechanism is structural — `Factory` has no fields, so a second input cannot exist — and the mechanism this instance names is a grep that would pass on a stateful factory | `ring_factory` | **misleading doc** | [invariant/001](../invariant/001_configuration_fully_determines_the_ring.md) |
| FC22 | The determinism test passes for a reason that would survive the invariant being false: it compares observable behaviour, and a built ring retains only two of the five fields, so three could differ undetected | `ring_factory` | **latent hazard** | [invariant/001](../invariant/001_configuration_fully_determines_the_ring.md) |
| FC23 | Nine public ways to obtain a ring exist family-wide and three of them are this crate's; the pattern documenting the restriction still says five | `ring_factory` | n/a — inconsistency | [invariant/002](../invariant/002_construction_is_the_only_path.md) |
| FC24 | The export surface confines crates, not types, so every leak it permits is a type living inside a confined crate — `Registry`'s eight methods being the worked example | `ring_factory` | n/a — observation | [invariant/002](../invariant/002_construction_is_the_only_path.md) |
| FC25 | `build_crossbeam` and both of its tests are behind `#[ cfg( feature = "crossbeam" ) ]`, no manifest in the workspace enables it, and a default `cargo test -p ring_factory` reports green having compiled neither | `ring_factory` | n/a — coverage | [item/001](../item/001_three_verbs_one_of_them_conditional.md) |
| FC26 | `build_named` is the crate's only union refusal set, and its `NameTaken` arm costs a full 320-byte `Split` construction and drop before returning a 24-byte error — stated in a doc comment's sequencing clause and nowhere a caller would look | `ring_factory` | n/a — doc gap | [item/001](../item/001_three_verbs_one_of_them_conditional.md) |
| FC27 | `Factory` derives `Clone` without `Copy`, so a zero-byte stateless value moves out of a binding; the crate argues at length for the absent `Default` and never mentions either granted derive | `ring_factory` | n/a — observation | [item/002](../item/002_four_nouns_two_of_them_somebody_elses.md) |
| FC28 | `BuildError` is 24 bytes, identical to the `RingError` it wraps, so the relay is free — the strongest argument for the design, and one the source never makes | `ring_factory` | n/a — observation | [item/002](../item/002_four_nouns_two_of_them_somebody_elses.md) |
| FC29 | The construction arc crosses four crates and this crate holds two statements of it — a `Ring::new` call and a `Split::new` wrap; every phase a reader would want named happens in a dependency | `ring_factory` | n/a — observation | [lifecycle/001](../lifecycle/001_from_a_record_to_a_handle_pair.md) |
| FC30 | Nothing can be retained because there is no retaining machinery in the crate: zero `Drop`, `Default`, `new`, `static`, `Arc`, `Rc`, `Box` and `Vec` outside doc comments | `ring_factory` | n/a — observation | [lifecycle/002](../lifecycle/002_the_factory_outlives_nothing.md) |
| FC31 | The requested value has no state — it is overwritten in place at the setter, so the "requested" and "clamped" states this instance distinguishes never coexist in any value | `ring_factory` | n/a — observation | [lifecycle/003](../lifecycle/003_config_state_through_a_build.md) |
| FC32 | Registration is atomic and the name's later states are not this crate's to pin: of the registry's eight public methods this crate calls one, and the tests reach six more that no build path touches | `ring_factory` | n/a — observation | [lifecycle/004](../lifecycle/004_name_state_through_a_registration.md) |
| FC33 | The requirement is capped at two fields by the return type, not by the tests — a built `Ring` retains storage and `overflow`, so "every field asserted one at a time" can only reach two of five | `ring_factory` | n/a — unenforced | [non_functional_requirement/001](../non_functional_requirement/001_five_fields_asserted_one_at_a_time.md) |
| FC34 | Two doctests are the only tests of the documented call shape, they sit above `build` and `build_named`, and `build_crossbeam` — the method whose call shape is least guessable — has none | `ring_factory` | n/a — coverage | [non_functional_requirement/001](../non_functional_requirement/001_five_fields_asserted_one_at_a_time.md) |
| FC35 | The requirement's own measurement appears nowhere in the suite; the single `alloc` match in the test file is a doc comment asserting that an allocation hook exists | `ring_factory` | n/a — unenforced | [non_functional_requirement/002](../non_functional_requirement/002_construction_cost_is_paid_once.md) |
| FC36 | Construction allocates once and the crate cannot see it — the allocation is `ring_store`'s, reached through `ring_core`, and this crate names neither | `ring_factory` | n/a — observation | [non_functional_requirement/002](../non_functional_requirement/002_construction_cost_is_paid_once.md) |
| FC37 | The pattern makes an illegal configuration representable and one actually is: `OverflowPolicy::DropOldest` passes every `ring_config` validation and is refused by `ring_core`, which is why `build` returns a `Result` at all | `ring_factory` | **latent hazard** | [pattern/001](../pattern/001_configuration_as_data.md) |
| FC38 | A built ring keeps one of the record's five fields as a stored value, so nothing downstream can report what a ring was built from | `ring_factory` | n/a — observation | [pattern/001](../pattern/001_configuration_as_data.md) |
| FC39 | The count was five and is now nine — nine public constructors across `ring_spsc`, `ring_mpsc`, `ring_core` and this crate, against the five the pattern's status still records | `ring_factory` | n/a — drift | [pattern/002](../pattern/002_one_way_in.md) |
| FC40 | Seven crates import `ring_core` directly and one imports this crate, so "one way in" is a promise to consumers outside the family rather than a practice inside it | `ring_factory` | n/a — observation | [pattern/002](../pattern/002_one_way_in.md) |
| FC41 | The two clamped fields are exactly the two nothing reads, so the pitfall is dormant rather than absent — it arms itself the moment `wait` or `batch` acquires a consumer | `ring_factory` | n/a — observation | [pitfall/001](../pitfall/001_the_criterion_grades_the_clamped_value.md) |
| FC42 | There is a legality check on `RingConfig` — `is_tick_safe` — that no build path calls; every one of its callers is a test of itself | `ring_factory` | n/a — unadopted | [pitfall/001](../pitfall/001_the_criterion_grades_the_clamped_value.md) |
| FC43 | The implementing crate is not in the closure, measured rather than asserted: `ring_wait` is absent from all nineteen crates this crate reaches | `ring_factory` | n/a — observation | [pitfall/002](../pitfall/002_a_wait_strategy_it_can_read_and_cannot_honour.md) |
| FC44 | The unhonourable field is on the export surface via `ring_types` and the crate that would honour it is not — five public `ring_wait` functions take a `WaitKind` no consumer of the Contract can reach them with | `ring_factory` | **latent hazard** | [pitfall/002](../pitfall/002_a_wait_strategy_it_can_read_and_cannot_honour.md) |
| FC45 | The only way to obtain a `Factory` is the unit literal and that is stated nowhere; the absent `Default` is argued at length and the absent `new` — the one a Rust reader looks for first — is not mentioned | `ring_factory` | n/a — doc gap | [type/001](../type/001_factory.md) |
| FC46 | Three methods, all `&self`, zero fields: the type is a namespace with a receiver, and replacing it with three free functions would change every call site's spelling and nothing else | `ring_factory` | n/a — observation | [type/001](../type/001_factory.md) |
| FC47 | `Copy` is the family convention rather than this type's choice — seven of eight family error types derive it, `RegistryError` is the lone exception because it holds a `String` — and nothing records that | `ring_factory` | n/a — doc gap | [type/002](../type/002_build_error.md) |
| FC48 | The enum and its payload make opposite exhaustiveness choices: `BuildError` is deliberately exhaustive and wraps `RingError`, which is `#[ non_exhaustive ]` — the opposite choice, one `match` apart | `ring_factory` | n/a — inconsistency | [type/002](../type/002_build_error.md) |
| FC49 | `RegistryError::NameTaken` is the family's one tuple-payload error, carrying the refused value back to its caller, and this crate — its only caller — drops the payload with `_refused` | `ring_factory` | n/a — observation | [workaround/001](../workaround/001_returning_ownership_through_the_error.md) |
| FC50 | The name is preserved by the callee and discarded by the door: `ring_registry` returns the rejected `String` and `BuildError::NameTaken` carries nothing, so a caller who wants it back must have kept it | `ring_factory` | n/a — observation | [workaround/001](../workaround/001_returning_ownership_through_the_error.md) |
| FC51 | The invariant is preserved by making it not apply — the crossbeam feature is declared in four manifests, names a dependency in exactly one, and is enabled nowhere in the workspace | `ring_factory` | n/a — observation | [workaround/002](../workaround/002_a_feature_cannot_be_a_value.md) |
| FC52 | The deletion condition is reachable and nothing is watching for it: `build_crossbeam` folds away the moment backend selection becomes a config value, and no pending, task or gate records that | `ring_factory` | n/a — unenforced | [workaround/002](../workaround/002_a_feature_cannot_be_a_value.md) |
