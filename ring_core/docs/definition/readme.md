# Doc Definitions

Module Index for `ring_core` — every doc definition this crate declares and every
instance under each, in one place.

## Master Doc Definitions Table

| Type | Purpose | Master File | Instances |
|------|---------|-------------|----------:|
| `algorithm/` | Dispatch to a backend on publish and on drain — the only two procedures this crate owns | [algorithm/readme.md](../algorithm/readme.md) | 2 |
| `api/` | The publish and drain contracts, marked per operation as uniform or per-backend | [api/readme.md](../api/readme.md) | 2 |
| `data_structure/` | Four parallel three-way enums whose arity is a compile-time property | [data_structure/readme.md](../data_structure/readme.md) | 2 |
| `decisions/` | Two open questions the item census raised: an unused discrimination surface, and one signature over two contracts | [decisions/readme.md](../decisions/readme.md) | 2 |
| `integration/` | Six in-house edges, one external edge, and the unreconciled surface above | [integration/readme.md](../integration/readme.md) | 2 |
| `invariant/` | What a composition layer must cost nothing of, and what swapping the thing beneath it may not change | [invariant/readme.md](../invariant/readme.md) | 2 |
| `item/` | Thirty-six items by kind, and which of the twenty-seven public ones the family actually reaches | [item/readme.md](../item/readme.md) | 2 |
| `lifecycle/` | The two irreversible moments in a ring's life, and the occupancy states a caller programs against between them | [lifecycle/readme.md](../lifecycle/readme.md) | 3 |
| `non_functional_requirement/` | "Swappable" given a threshold and a reading | [non_functional_requirement/readme.md](../non_functional_requirement/readme.md) | 2 |
| `pattern/` | The reusable composition shape, its two rules, and its characteristic bug | [pattern/readme.md](../pattern/readme.md) | 2 |
| `pitfall/` | One trap for callers of the surface, one for readers of the coverage number | [pitfall/readme.md](../pitfall/readme.md) | 2 |
| `type/` | One public enum with build-dependent arity, and one concept deliberately not a type | [type/readme.md](../type/readme.md) | 2 |
| `workaround/` | External constraints this crate absorbs, with costs and deletion conditions | [workaround/readme.md](../workaround/readme.md) | 2 |

## Master Doc Instances Table

| Definition | ID | Name | File |
|------------|----|------|------|
| `algorithm/` | 001 | Backend Dispatch and the Refusal Seam | [001_backend_dispatch_and_the_refusal_seam.md](../algorithm/001_backend_dispatch_and_the_refusal_seam.md) |
| `algorithm/` | 002 | Uniform Drain Over Three Drain Shapes | [002_uniform_drain_over_three_shapes.md](../algorithm/002_uniform_drain_over_three_shapes.md) |
| `api/` | 001 | Producer Surface | [001_producer_surface.md](../api/001_producer_surface.md) |
| `api/` | 002 | Consumer Surface | [002_consumer_surface.md](../api/002_consumer_surface.md) |
| `data_structure/` | 001 | The Three-Way Storage Enum | [001_three_way_storage_enum.md](../data_structure/001_three_way_storage_enum.md) |
| `data_structure/` | 002 | Four Parallel Enums Over One Backend Choice | [002_four_parallel_enums_over_one_backend_choice.md](../data_structure/002_four_parallel_enums_over_one_backend_choice.md) |
| `decisions/` | 001 | The Backend Discrimination Surface Has No Caller | [001_the_backend_discrimination_surface_has_no_caller.md](../decisions/001_the_backend_discrimination_surface_has_no_caller.md) |
| `decisions/` | 002 | `free_capacity` Keeps One Signature Over Two Contracts | [002_free_capacity_keeps_one_signature_over_two_contracts.md](../decisions/002_free_capacity_keeps_one_signature_over_two_contracts.md) |
| `integration/` | 001 | The Family Dependency Seam | [001_family_dependency_seam.md](../integration/001_family_dependency_seam.md) |
| `integration/` | 002 | Divergence From `ring_handle`'s Specified Surface | [002_handle_surface_divergence.md](../integration/002_handle_surface_divergence.md) |
| `invariant/` | 001 | This Crate Adds No Atomic of Its Own | [001_no_atomic_of_its_own.md](../invariant/001_no_atomic_of_its_own.md) |
| `invariant/` | 002 | Uniform Delivery Across Every Backend | [002_uniform_delivery_across_backends.md](../invariant/002_uniform_delivery_across_backends.md) |
| `item/` | 001 | Thirty-Six Items, and the Four Names the Family Imports | [001_thirty_six_items_and_the_four_names_the_family_imports.md](../item/001_thirty_six_items_and_the_four_names_the_family_imports.md) |
| `item/` | 002 | The Backend Discrimination Surface | [002_the_backend_discrimination_surface.md](../item/002_the_backend_discrimination_surface.md) |
| `lifecycle/` | 001 | Construction and Backend Selection | [001_construction_and_backend_selection.md](../lifecycle/001_construction_and_backend_selection.md) |
| `lifecycle/` | 002 | The Ends Split and Handle Lifetimes | [002_ends_split_and_handle_lifetimes.md](../lifecycle/002_ends_split_and_handle_lifetimes.md) |
| `lifecycle/` | 003 | Occupancy Across Backends | [003_occupancy_across_backends.md](../lifecycle/003_occupancy_across_backends.md) |
| `non_functional_requirement/` | 001 | A Backend Swap Is a Build Flag, Not a Rewrite | [001_backend_swap_is_a_build_flag.md](../non_functional_requirement/001_backend_swap_is_a_build_flag.md) |
| `non_functional_requirement/` | 002 | The Composition Adds No Atomic and One Allocation | [002_the_composition_adds_no_atomic_and_one_allocation.md](../non_functional_requirement/002_the_composition_adds_no_atomic_and_one_allocation.md) |
| `pattern/` | 001 | A Uniform Surface Over Unequal Backends | [001_uniform_surface_over_unequal_backends.md](../pattern/001_uniform_surface_over_unequal_backends.md) |
| `pattern/` | 002 | Claim Before Move, So a Refusal Can Hand the Record Back | [002_claim_before_move_so_a_refusal_can_hand_the_record_back.md](../pattern/002_claim_before_move_so_a_refusal_can_hand_the_record_back.md) |
| `pitfall/` | 001 | `free_capacity` Carries Two Contracts Under One Signature | [001_free_capacity_carries_two_contracts.md](../pitfall/001_free_capacity_carries_two_contracts.md) |
| `pitfall/` | 002 | Feature-Gated Code Reads as Uncovered | [002_feature_gated_code_reads_as_uncovered.md](../pitfall/002_feature_gated_code_reads_as_uncovered.md) |
| `type/` | 001 | `Backend` | [001_backend.md](../type/001_backend.md) |
| `type/` | 002 | Producer Cardinality as a Return Value | [002_producer_cardinality.md](../type/002_producer_cardinality.md) |
| `workaround/` | 001 | crossbeam-queue as the Interim Backend | [001_crossbeam_queue_as_interim_backend.md](../workaround/001_crossbeam_queue_as_interim_backend.md) |
| `workaround/` | 002 | A `debug_assert` Where the Type System Cannot Reach | [002_a_debug_assert_where_the_type_system_cannot_reach.md](../workaround/002_a_debug_assert_where_the_type_system_cannot_reach.md) |

## Findings

52, each argued in the instance named under **Where** and verified
there by a command whose output is quoted beneath it.

| ID | Finding | Subject | Reachable | Where |
|----|---------|---------|-----------|-------|
| CO1 | The one wildcard arm was on the policy enum, so a new overflow policy was the single addition that compiled silently; both remaining `Resolution` variants are named now. | `try_push` | **latent hazard** | [algorithm/001](../algorithm/001_backend_dispatch_and_the_refusal_seam.md) |
| CO2 | Each backend refuses in its own shape — the record, a `RingError`, or a `Result< (), T >` — and `try_push` normalizes all three. | the three backends | n/a — observation | [algorithm/001](../algorithm/001_backend_dispatch_and_the_refusal_seam.md) |
| CO3 | The two batch methods returned a load-bearing count the compiler would not make anyone read; `#[ must_use ]` now does, and it caught twenty-four discards across five crates the moment it was added. | batch methods | **latent hazard** | [algorithm/002](../algorithm/002_uniform_drain_over_three_shapes.md) |
| CO4 | The in-house arms of the batch drain are a loop over the single-record path, so a batch saves a call per record and nothing else. | `try_recv_batch` | **measured cost** | [algorithm/002](../algorithm/002_uniform_drain_over_three_shapes.md) |
| CO5 | `try_push`, `try_push_batch`, `free_capacity` and `is_full` are called by libraries; `try_clone` is not. | `Producer` | n/a — coverage | [api/001](../api/001_producer_surface.md) |
| CO6 | Thread-safety is a per-backend property of the borrowed handle, and the surface documentation states it nowhere. | `Producer` | n/a — doc gap | [api/001](../api/001_producer_surface.md) |
| CO7 | The consumer-side occupancy readings inherited the producer-side asymmetry undocumented; `len` and `is_empty` state it now and name `try_recv` as the authority. | `Consumer` | **latent hazard** | [api/002](../api/002_consumer_surface.md) |
| CO8 | One consumer per ring is a property of the split, not of the type, and holds because `Ends::split` is the only constructor. | `Consumer` | n/a — observation | [api/002](../api/002_consumer_surface.md) |
| CO9 | The optional crossbeam backend costs sixteen `cfg` gates in one 653-line file. | `Storage` | **measured cost** | [data_structure/001](../data_structure/001_three_way_storage_enum.md) |
| CO10 | `Storage` is the only backend enum that owns its payload, which is why it is the only one without a lifetime parameter. | the five enums | n/a — observation | [data_structure/001](../data_structure/001_three_way_storage_enum.md) |
| CO11 | Exhaustive matching converts a new backend into five compile errors rather than one silent fallthrough. | the five enums | **measured cost** | [data_structure/002](../data_structure/002_four_parallel_enums_over_one_backend_choice.md) |
| CO12 | The four private enums are kept variant-for-variant identical by convention, and no test or type asserts it. | the five enums | n/a — unenforced | [data_structure/002](../data_structure/002_four_parallel_enums_over_one_backend_choice.md) |
| CO13 | Across thirty-two sibling crates, the surface is exercised by `ring_factory` and `ring_handle` tests and named by three doc comments explaining non-use. | the discrimination surface | n/a — unadopted | [decisions/001](../decisions/001_the_backend_discrimination_surface_has_no_caller.md) |
| CO14 | `try_clone` has no production caller and is the only way to obtain a second producer, so "unused" and "removable" are different questions. | `try_clone` | n/a — observation | [decisions/001](../decisions/001_the_backend_discrimination_surface_has_no_caller.md) |
| CO15 | `ring_handle` and `ring_bench` each spend prose explaining that they do not forward `try_clone`, which no dependent was reaching for. | `ring_handle` | n/a — inconsistency | [decisions/001](../decisions/001_the_backend_discrimination_surface_has_no_caller.md) |
| CO16 | No dependent branches on backend before reading occupancy, so the decision's cost has not yet been paid by anyone. | `free_capacity` | n/a — observation | [decisions/002](../decisions/002_free_capacity_keeps_one_signature_over_two_contracts.md) |
| CO17 | `is_full` is `free_capacity() == 0`, so it races at MPSC while `ring_shutdown` branches on it; the asymmetry is stated on `is_full` itself now. | `is_full` | **latent hazard** | [decisions/002](../decisions/002_free_capacity_keeps_one_signature_over_two_contracts.md) |
| CO18 | Both instances under `decisions/` state a status and neither states what evidence would change it. | this decision | n/a — doc gap | [decisions/002](../decisions/002_free_capacity_keeps_one_signature_over_two_contracts.md) |
| CO19 | `ring_mpsc` and `ring_spsc` are reached only through fully-qualified paths, so the `use` block understates the seam. | `Cargo.toml` | n/a — observation | [integration/001](../integration/001_family_dependency_seam.md) |
| CO20 | Every in-family dependency is a path dependency; the one registry dependency is the one gated behind a feature. | `crossbeam-queue` | n/a — observation | [integration/001](../integration/001_family_dependency_seam.md) |
| CO21 | The import profile across the eight dependent crates is `Ring` 8, `Producer` 7, `Consumer` 5, `Ends` 2, `Backend` 0. | the eight dependents | n/a — unadopted | [integration/002](../integration/002_handle_surface_divergence.md) |
| CO22 | The divergence this instance documents is a narrowing — `ring_handle` withholds `try_clone` and adds no method of its own. | `ring_handle` | n/a — observation | [integration/002](../integration/002_handle_surface_divergence.md) |
| CO23 | Zero atomics in code, one in prose, zero `unsafe`, and no `ring_stats` dependency — the invariant holds by four independent measurements. | `src/lib.rs` | n/a — observation | [invariant/001](../invariant/001_no_atomic_of_its_own.md) |
| CO24 | Nothing in the build or the test suite fails if an atomic is added to this crate. | the no-atomic rule | n/a — unenforced | [invariant/001](../invariant/001_no_atomic_of_its_own.md) |
| CO25 | Uniform delivery is asserted by a single test that a helper re-runs for each compiled backend. | `the_same_program_behaves_identically_on_every_backend` | n/a — coverage | [invariant/002](../invariant/002_uniform_delivery_across_backends.md) |
| CO26 | Records delivered are identical across backends; the occupancy readings observably are not, and the invariant's title does not distinguish them. | the uniformity claim | **misleading doc** | [invariant/002](../invariant/002_uniform_delivery_across_backends.md) |
| CO27 | No trait, free function, static, macro, type alias or constant — everything here is reached by naming one of five types. | the census | n/a — observation | [item/001](../item/001_thirty_six_items_and_the_four_names_the_family_imports.md) |
| CO28 | `Ring::capacity`, `Consumer::len` and `Consumer::is_empty` collide with `Vec`, `str` and `RingConfig` methods, so a name-based reach count is meaningless for them. | `capacity`, `len`, `is_empty` | n/a — diagnostics | [item/001](../item/001_thirty_six_items_and_the_four_names_the_family_imports.md) |
| CO29 | `Ring::new` reads as 18 call sites unfiltered and 2 with doc examples removed. | the measurement | n/a — diagnostics | [item/001](../item/001_thirty_six_items_and_the_four_names_the_family_imports.md) |
| CO30 | The module documentation names `try_clone` as the machine-checkable way to tell the backends apart, and no library calls it. | the discrimination surface | n/a — unadopted | [item/002](../item/002_the_backend_discrimination_surface.md) |
| CO31 | The only `Backend` import in the family is a test import, asserting that the factory selects correctly. | `ring_factory` | n/a — coverage | [item/002](../item/002_the_backend_discrimination_surface.md) |
| CO32 | `ring_handle/tests/handle_test.rs:382` reads the backend into a binding and takes no decision from it. | `ring_handle` | n/a — observation | [item/002](../item/002_the_backend_discrimination_surface.md) |
| CO33 | `DropOldest` is refused by the in-house backends and honoured by crossbeam, so the same config succeeds or fails depending on a build flag. | `Ring::new` | n/a — observation | [lifecycle/001](../lifecycle/001_construction_and_backend_selection.md) |
| CO34 | The producer-count branch is written as a two-arm `match` rather than `if`/`else`, and the source says why. | `Ring::new` | n/a — observation | [lifecycle/001](../lifecycle/001_construction_and_backend_selection.md) |
| CO35 | `ends()` then `split()` is two calls because `ring_mpsc` needs an intermediate; the other two backends could have split in one. | `Ends` | n/a — observation | [lifecycle/002](../lifecycle/002_ends_split_and_handle_lifetimes.md) |
| CO36 | `split` borrows `&'a mut self`, so a caller who lets `ends` drop gets a borrow error the documentation does not anticipate. | `Ends` | n/a — doc gap | [lifecycle/002](../lifecycle/002_ends_split_and_handle_lifetimes.md) |
| CO37 | `free_capacity` is on the producer, `len` on the consumer, and at MPSC a caller can observe a pair that sums to neither zero nor the capacity. | `occupancy` | n/a — observation | [lifecycle/003](../lifecycle/003_occupancy_across_backends.md) |
| CO38 | The consumer-side agreement is asserted across a whole lap, single-threaded, which is exactly where CO37 says it holds. | `len_and_is_empty_agree_at_every_point_of_a_lap` | n/a — coverage | [lifecycle/003](../lifecycle/003_occupancy_across_backends.md) |
| CO39 | Swapping backends is a build flag with no code change except for `DropOldest`, which changes `Ring::new` from `Ok` to `Err`. | backend swap | **misleading doc** | [non_functional_requirement/001](../non_functional_requirement/001_backend_swap_is_a_build_flag.md) |
| CO40 | Every method here is a `match` forwarding to another crate, and none is marked `#[ inline ]`. | `src/lib.rs` | n/a — observation | [non_functional_requirement/001](../non_functional_requirement/001_backend_swap_is_a_build_flag.md) |
| CO41 | The crossbeam arm collects into an intermediate `Vec` per drain; the two in-house arms extend the caller's buffer in place. | `try_recv_batch` | **measured cost** | [non_functional_requirement/002](../non_functional_requirement/002_the_composition_adds_no_atomic_and_one_allocation.md) |
| CO42 | The zero-atomic requirement is asserted in a different crate's test suite and not at all in this one. | the no-atomic threshold | n/a — unenforced | [non_functional_requirement/002](../non_functional_requirement/002_the_composition_adds_no_atomic_and_one_allocation.md) |
| CO43 | Uniformity is achieved separately for construction, splitting, pushing and draining, and only the push case has a named pattern. | the uniform surface | n/a — duplication | [pattern/001](../pattern/001_uniform_surface_over_unequal_backends.md) |
| CO44 | Every method exists on every backend; the guarantees behind two of them differ, and the pattern has no way to express that. | the uniform surface | n/a — observation | [pattern/001](../pattern/001_uniform_surface_over_unequal_backends.md) |
| CO45 | The MPSC arm performs a reservation and a write where a direct push would do one call, so uniformity costs one extra step on the success path. | `claim-before-move` | **measured cost** | [pattern/002](../pattern/002_claim_before_move_so_a_refusal_can_hand_the_record_back.md) |
| CO46 | Two of the three backends need nothing from this pattern, which is what makes its applicability condition worth stating explicitly. | `claim-before-move` | n/a — observation | [pattern/002](../pattern/002_claim_before_move_so_a_refusal_can_hand_the_record_back.md) |
| CO47 | The rustdoc offers two remedies — call `try_clone` to learn the reading, or treat every reading as advisory — and all four callers took the second. | `free_capacity` | n/a — unadopted | [pitfall/001](../pitfall/001_free_capacity_carries_two_contracts.md) |
| CO48 | The same two-contract asymmetry applies to `is_full`, `len` and `is_empty`, and only `free_capacity` carries the warning. | occupancy readings | **misleading doc** | [pitfall/001](../pitfall/001_free_capacity_carries_two_contracts.md) |
| CO49 | The default build compiles two backends and the `--all-features` build three, and no single run covers both configurations. | feature-gated code | n/a — coverage | [pitfall/002](../pitfall/002_feature_gated_code_reads_as_uncovered.md) |
| CO50 | The type carries no data, exists to be compared, and is compared only in one test file. | `Backend` | n/a — observation | [type/001](../type/001_backend.md) |
| CO51 | `Option< Producer >` encodes "this backend permits one producer" as a runtime absence rather than as a compile-time distinction. | `try_clone` | n/a — observation | [type/002](../type/002_producer_cardinality.md) |
| CO52 | The `expect` is gone; the crate's one surviving runtime obligation is a `debug_assert` a release build compiles out. | `src/lib.rs` | **latent hazard** | [workaround/002](../workaround/002_a_debug_assert_where_the_type_system_cannot_reach.md) |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_core/docs
printf 'doc definitions:          '; ls -d */ | grep -vc '^definition/'
printf 'instances:                '; ls */[0-9][0-9][0-9]_*.md | wc -l
printf 'findings in the corpus:   '; grep -rhoE '^### CO[0-9]+ ' */[0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| CO[0-9]+ ' definition/readme.md
# doc definitions:          13
# instances:                27
# findings in the corpus:   52
# rows in the table below:  52
```
