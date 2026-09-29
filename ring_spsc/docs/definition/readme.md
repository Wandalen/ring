# Doc Definitions

Module Index for `ring_spsc` — every doc definition this crate declares and every
instance under each, in one place.

## Master Doc Definitions Table

| Type | Purpose | Master File | Instances |
|------|---------|-------------|----------:|
| `algorithm/` | The claim-and-publish and drain procedures — steps, orderings, and the atomic operations the single-producer cardinality removes | [algorithm/readme.md](../algorithm/readme.md) | 2 |
| `api/` | The two caller surfaces, and which of their open shape questions are cheap to leave open | [api/readme.md](../api/readme.md) | 2 |
| `data_structure/` | Three fields where the multi-producer sibling needs four, and the per-slot state deliberately not carried | [data_structure/readme.md](../data_structure/readme.md) | 2 |
| `decisions/` | Architecture Decision Records for open trade-offs this crate could not close unilaterally | [decisions/readme.md](../decisions/readme.md) | 2 |
| `integration/` | Five dependencies, four pointed absences, and the export boundary the capability crosses while the crate does not | [integration/readme.md](../integration/readme.md) | 2 |
| `invariant/` | The cardinality precondition every saving is purchased with, and the synchronization budget it buys | [invariant/readme.md](../invariant/readme.md) | 2 |
| `item/` | Sixty items by kind, the three public names that leave the crate, and the two published ordering constants | [item/readme.md](../item/readme.md) | 2 |
| `lifecycle/` | The ring's phases and handle arcs, and the contiguous-prefix property that eliminates per-slot stamps | [lifecycle/readme.md](../lifecycle/readme.md) | 4 |
| `non_functional_requirement/` | The correctness-floor obligation carried on the family's behalf, and this crate's own binary Reached condition | [non_functional_requirement/readme.md](../non_functional_requirement/readme.md) | 2 |
| `pattern/` | Prove the degenerate configuration first — the practice this crate embodies, with an honest account of when it does not pay | [pattern/readme.md](../pattern/readme.md) | 2 |
| `pitfall/` | The trap this crate's own success creates: properties true here and false one cardinality up | [pitfall/readme.md](../pitfall/readme.md) | 2 |
| `type/` | The two values the correctness arguments are written in | [type/readme.md](../type/readme.md) | 2 |
| `workaround/` | External constraints this crate absorbs, with costs and deletion conditions — including the `unsafe` opt-out gate G6 requires justified here | [workaround/readme.md](../workaround/readme.md) | 2 |

## Master Doc Instances Table

| Definition | ID | Name | File |
|------------|----|------|------|
| `algorithm/` | 001 | Uncontended Claim and Publish | [001_uncontended_claim_and_publish.md](../algorithm/001_uncontended_claim_and_publish.md) |
| `algorithm/` | 002 | Single-Consumer Drain to the Published Bound | [002_single_consumer_drain.md](../algorithm/002_single_consumer_drain.md) |
| `api/` | 001 | Producer Surface | [001_producer_surface.md](../api/001_producer_surface.md) |
| `api/` | 002 | Consumer Surface | [002_consumer_surface.md](../api/002_consumer_surface.md) |
| `data_structure/` | 001 | Two-Cursor Ring Without Per-Slot State | [001_two_cursor_ring.md](../data_structure/001_two_cursor_ring.md) |
| `data_structure/` | 002 | The Absent Stamp Array | [002_the_absent_stamp_array.md](../data_structure/002_the_absent_stamp_array.md) |
| `decisions/` | 001 | The Switching-Cost Argument Undercounts Its Own Blast Radius | [001_the_switching_cost_argument_undercounts_its_own_blast_radius.md](../decisions/001_the_switching_cost_argument_undercounts_its_own_blast_radius.md) |
| `decisions/` | 002 | The Loom Seam Runs Through a Crate This Manifest Never Names | [002_the_loom_seam_runs_through_a_crate_this_manifest_never_names.md](../decisions/002_the_loom_seam_runs_through_a_crate_this_manifest_never_names.md) |
| `integration/` | 001 | Family Dependency Seam | [001_family_dependency_seam.md](../integration/001_family_dependency_seam.md) |
| `integration/` | 002 | Reached Through the Export Surface | [002_reached_through_the_export_surface.md](../integration/002_reached_through_the_export_surface.md) |
| `invariant/` | 001 | Exactly One Producer, Exactly One Consumer | [001_exactly_one_producer_one_consumer.md](../invariant/001_exactly_one_producer_one_consumer.md) |
| `invariant/` | 002 | No Lock in the Path | [002_no_lock_in_the_path.md](../invariant/002_no_lock_in_the_path.md) |
| `item/` | 001 | Sixty Items, and What Actually Reaches Them | [001_sixty_items_and_what_actually_reaches_them.md](../item/001_sixty_items_and_what_actually_reaches_them.md) |
| `item/` | 002 | Two Ordering Constants Where the Sibling Has Five | [002_two_ordering_constants_where_the_sibling_has_five.md](../item/002_two_ordering_constants_where_the_sibling_has_five.md) |
| `lifecycle/` | 001 | Ring Construction and Teardown | [001_ring_construction_and_teardown.md](../lifecycle/001_ring_construction_and_teardown.md) |
| `lifecycle/` | 002 | Producer and Consumer Pairing | [002_producer_consumer_pairing.md](../lifecycle/002_producer_consumer_pairing.md) |
| `lifecycle/` | 003 | Slot State Without Holes | [003_slot_state_without_holes.md](../lifecycle/003_slot_state_without_holes.md) |
| `lifecycle/` | 004 | Ring Occupancy Between the Cursors | [004_ring_occupancy.md](../lifecycle/004_ring_occupancy.md) |
| `non_functional_requirement/` | 001 | Correctness Floor for the Family | [001_correctness_floor_for_the_family.md](../non_functional_requirement/001_correctness_floor_for_the_family.md) |
| `non_functional_requirement/` | 002 | Byte-Parity Over 100 000 Items | [002_byte_parity_over_one_hundred_thousand.md](../non_functional_requirement/002_byte_parity_over_one_hundred_thousand.md) |
| `pattern/` | 001 | Validate the Simple Configuration Before the General One | [001_validate_simple_before_general.md](../pattern/001_validate_simple_before_general.md) |
| `pattern/` | 002 | Absence as Specification | [002_absence_as_specification.md](../pattern/002_absence_as_specification.md) |
| `pitfall/` | 001 | SPSC Correctness Does Not Transfer to MPSC | [001_spsc_correctness_does_not_transfer.md](../pitfall/001_spsc_correctness_does_not_transfer.md) |
| `pitfall/` | 002 | A Departed Counterpart Is Indistinguishable From a Slow One | [002_a_departed_counterpart_is_indistinguishable_from_a_slow_one.md](../pitfall/002_a_departed_counterpart_is_indistinguishable_from_a_slow_one.md) |
| `type/` | 001 | Producer Cursor | [001_producer_cursor.md](../type/001_producer_cursor.md) |
| `type/` | 002 | Free Capacity | [002_free_capacity.md](../type/002_free_capacity.md) |
| `workaround/` | 001 | The `unsafe_code` Opt-Out and What Bounds It | [001_the_unsafe_code_opt_out_and_what_bounds_it.md](../workaround/001_the_unsafe_code_opt_out_and_what_bounds_it.md) |
| `workaround/` | 002 | An `unsafe impl Sync` on a Type Whose Ends Are Not `Sync` | [002_an_unsafe_impl_sync_on_a_type_whose_ends_are_not_sync.md](../workaround/002_an_unsafe_impl_sync_on_a_type_whose_ends_are_not_sync.md) |

## Findings

52, each argued in the instance named under **Where** and verified
there by a command whose output is quoted beneath it.

| ID | Finding | Subject | Reachable | Where |
|----|---------|---------|-----------|-------|
| SP1 | No path in this crate performs a read-modify-write, so the producer cursor advances without a compare-exchange loop. | the claim step | n/a — observation | [algorithm/001](../algorithm/001_uncontended_claim_and_publish.md) |
| SP2 | The same guard-drop-publishes contract as the multi-producer sibling, and the same sharp edge: an early return emits a default record. | `Reservation` | **latent hazard** | [algorithm/001](../algorithm/001_uncontended_claim_and_publish.md) |
| SP3 | With one producer the cursor is the published watermark, so the drain reads a value rather than scanning for one. | `drain` | n/a — observation | [algorithm/002](../algorithm/002_single_consumer_drain.md) |
| SP4 | `drain_up_to_zero_is_a_legitimate_no_op` and `drain_up_to_more_than_available_yields_what_there_is` make every `usize` a legal argument. | `drain_up_to` | n/a — coverage | [algorithm/002](../algorithm/002_single_consumer_drain.md) |
| SP5 | Duplicating or sharing the producer would break the single-producer invariant, and the negatives are pinned by compile-fail doc tests. | `Producer` | n/a — observation | [api/001](../api/001_producer_surface.md) |
| SP6 | `claim`, `push_with` and `try_push` are three entry points onto the same claim-write-publish sequence, differing only in who owns the record. | the producer surface | n/a — observation | [api/001](../api/001_producer_surface.md) |
| SP7 | `a_batch_commits_exactly_its_own_length` means a caller that reads three of ten records still releases all ten. | `Batch` | n/a — observation | [api/002](../api/002_consumer_surface.md) |
| SP8 | `get_and_iter_agree_at_every_offset` guards two access paths onto the same slots against drifting apart. | `Batch` | n/a — coverage | [api/002](../api/002_consumer_surface.md) |
| SP9 | A single imported type owns both cursors and their cache-line separation, so this crate declares no padding of its own. | `CursorPair` | n/a — observation | [data_structure/001](../data_structure/001_two_cursor_ring.md) |
| SP10 | Both crates wrap `Buffer< UnsafeCell< S > >` per slot, for the same reason and with the same Miri history behind it. | `slots` | n/a — duplication | [data_structure/001](../data_structure/001_two_cursor_ring.md) |
| SP11 | Two of the three conditions that make a stamp necessary hold here; only out-of-order publication fails, and it fails because of producer count. | the absent stamp | n/a — observation | [data_structure/002](../data_structure/002_the_absent_stamp_array.md) |
| SP12 | `ring_mpsc`'s stale-stamp pitfall has no analogue here, so this crate's `pitfall/002` documents an entirely different failure. | the absent stamp | n/a — observation | [data_structure/002](../data_structure/002_the_absent_stamp_array.md) |
| SP13 | A pinned `.stderr` in `ring_handle` copies this crate's `Producer` declaration verbatim, so reformatting it reddens a test two hops away with no dependency edge to explain why. | the `ring_handle` fixture | **latent hazard** | [decisions/001](../decisions/001_the_switching_cost_argument_undercounts_its_own_blast_radius.md) |
| SP14 | The scan that reported two consumers looped over a four-crate list written from memory, leaving twenty-eight crates unexamined for the right answer. | the first reach scan | n/a — observation | [decisions/001](../decisions/001_the_switching_cost_argument_undercounts_its_own_blast_radius.md) |
| SP15 | Three consumers break three ways, and the two the argument omitted are the two whose failure does not name the file that caused it. | the consumer set | n/a — observation | [decisions/001](../decisions/001_the_switching_cost_argument_undercounts_its_own_blast_radius.md) |
| SP16 | The flag changes a type two crates away and appears in no build file as a declared configuration. | `--cfg loom` | n/a — unenforced | [decisions/002](../decisions/002_the_loom_seam_runs_through_a_crate_this_manifest_never_names.md) |
| SP17 | Both manifests carry the same `ring_atomic` argument; only `ring_mpsc` declares the dependency it argues from. | the loom comment | n/a — duplication | [decisions/002](../decisions/002_the_loom_seam_runs_through_a_crate_this_manifest_never_names.md) |
| SP18 | The three absent ones — `ring_atomic`, `ring_claim`, `ring_gating` — are exactly the crates single-producer makes unnecessary. | `Cargo.toml` | n/a — observation | [integration/001](../integration/001_family_dependency_seam.md) |
| SP19 | `docs/workstream/008_ring_write_path.md` did not exist there; the workstream lived at `008_ring_write_path/readme.md`. | the module documentation | n/a — drift (historical; source corpus unreachable since extraction) | [integration/001](../integration/001_family_dependency_seam.md) |
| SP20 | The instance argues this crate is internal; the count that supports it is five code references from `ring_core` and two from `ring_bench`. | `adoption` | n/a — observation | [integration/002](../integration/002_reached_through_the_export_surface.md) |
| SP21 | All five references are storage or handle variants in `ring_core`'s backend enums, never a call. | `ring_core` | n/a — observation | [integration/002](../integration/002_reached_through_the_export_surface.md) |
| SP22 | `split` takes `&mut self` so a second pair cannot coexist with the first, and the negatives are pinned where no runtime assertion could reach. | `cardinality` | n/a — observation | [invariant/001](../invariant/001_exactly_one_producer_one_consumer.md) |
| SP23 | The invariant is one producer and one consumer, not two threads, and the same-thread case is legal and unexercised. | `cardinality` | n/a — observation | [invariant/001](../invariant/001_exactly_one_producer_one_consumer.md) |
| SP24 | `the_two_orderings_are_the_ones_the_design_names` runs always; the `exhaustive` module that checks behaviour needs `--cfg loom`. | the orderings | n/a — unenforced | [invariant/002](../invariant/002_no_lock_in_the_path.md) |
| SP25 | Every cross-end load carries `ring_cursor::GATING`, which this crate imports rather than declares and does not pin. | `GATING` | n/a — observation | [invariant/002](../invariant/002_no_lock_in_the_path.md) |
| SP26 | `Ring`, `Producer` and `Consumer` leave the crate; `Reservation`, `Batch` and both constants never do. | the public surface | n/a — observation | [item/001](../item/001_sixty_items_and_what_actually_reaches_them.md) |
| SP27 | Both `ring_spsc` and `ring_mpsc` are flat files of structs and impls with no variant type anywhere. | the census | n/a — observation | [item/001](../item/001_sixty_items_and_what_actually_reaches_them.md) |
| SP28 | The distribution is uneven on purpose — `Ring` carries five, `Batch` carries six, `Reservation` carries one. | the census | n/a — observation | [item/001](../item/001_sixty_items_and_what_actually_reaches_them.md) |
| SP29 | The sibling asserts each of its five constants in a rustdoc example; this crate asserts both of its two in one integration test. | `OWN` and `HANDOFF` | n/a — observation | [item/002](../item/002_two_ordering_constants_where_the_sibling_has_five.md) |
| SP30 | `GATING` is used on every cross-end load and is not among this crate's published constants, so the vocabulary is incomplete where it is read. | the ordering vocabulary | n/a — inconsistency | [item/002](../item/002_two_ordering_constants_where_the_sibling_has_five.md) |
| SP31 | `the_ends_going_out_of_scope_in_either_order_releases_the_storage_once` covers the case a borrow-based design makes easy to get wrong. | `teardown` | n/a — coverage | [lifecycle/001](../lifecycle/001_ring_construction_and_teardown.md) |
| SP32 | `with_config_takes_the_capacity_and_ignores_the_rest` — the overflow policy and producer count are `ring_core`'s to interpret. | `with_config` | n/a — observation | [lifecycle/001](../lifecycle/001_ring_construction_and_teardown.md) |
| SP33 | `a_second_pair_may_be_split_once_the_first_is_gone` — the ring is not consumed by pairing, only borrowed. | `split` | n/a — observation | [lifecycle/002](../lifecycle/002_producer_consumer_pairing.md) |
| SP34 | `sequences_are_issued_consecutively_across_a_wrap` is what "without holes" means, asserted rather than argued. | slot state | n/a — coverage | [lifecycle/003](../lifecycle/003_slot_state_without_holes.md) |
| SP35 | The move-out semantics are asserted at the wrap boundary specifically, which is where a stale payload would survive into the next lap. | slot state | n/a — coverage | [lifecycle/003](../lifecycle/003_slot_state_without_holes.md) |
| SP36 | `available_and_is_empty_agree_at_every_point_of_a_lap` guards two derived views of the same cursor gap. | `occupancy` | n/a — coverage | [lifecycle/004](../lifecycle/004_ring_occupancy.md) |
| SP37 | This crate is where the family's correctness properties are cheapest to establish, and `ring_mpsc` is where they have to survive contention. | the floor | n/a — observation | [non_functional_requirement/001](../non_functional_requirement/001_correctness_floor_for_the_family.md) |
| SP38 | `a_ring_smaller_than_the_traffic_still_loses_nothing` is the wrap-under-pressure case the parity test alone would not force. | the floor | n/a — coverage | [non_functional_requirement/001](../non_functional_requirement/001_correctness_floor_for_the_family.md) |
| SP39 | The scale matches the sibling's four-producer test, so the two are comparable — and only the sibling's can fail from an ordering defect. | the parity test | n/a — observation | [non_functional_requirement/002](../non_functional_requirement/002_byte_parity_over_one_hundred_thousand.md) |
| SP40 | `a_failed_push_returns_the_record_rather_than_swallowing_it` is the property that makes refusal usable rather than lossy. | `backpressure` | n/a — observation | [non_functional_requirement/002](../non_functional_requirement/002_byte_parity_over_one_hundred_thousand.md) |
| SP41 | No other pair in the family stands in the simple/general relationship this pattern describes. | `validate-simple-before-general` | n/a — unadopted | [pattern/001](../pattern/001_validate_simple_before_general.md) |
| SP42 | Each of `ring_gating`, `ring_claim`, `ring_publish` and `ring_consume` is a real crate and none is a dependency of this one. | the absence list | n/a — observation | [pattern/002](../pattern/002_absence_as_specification.md) |
| SP43 | `ring_mpsc` names its eight dependencies and does not state what it gains from each, so the technique is used in one direction only. | the absence list | n/a — doc gap | [pattern/002](../pattern/002_absence_as_specification.md) |
| SP44 | Code written against this crate cannot be pointed at `ring_mpsc` without editing, because `split` returns a pair here and `Ends` there. | `non-transfer` | n/a — observation | [pitfall/001](../pitfall/001_spsc_correctness_does_not_transfer.md) |
| SP45 | Moving code from `ring_mpsc` to `ring_spsc` is the dangerous direction, and nothing prevents it beyond the same signature change. | `non-transfer` | **latent hazard** | [pitfall/001](../pitfall/001_spsc_correctness_does_not_transfer.md) |
| SP46 | A departed producer leaves its published records drainable; a departed consumer stalls the producer permanently. | `departure` | n/a — observation | [pitfall/002](../pitfall/002_a_departed_counterpart_is_indistinguishable_from_a_slow_one.md) |
| SP47 | `Full` is the honest error and it names a transient condition, so a caller reading only the error type will retry a permanent failure. | `RingError::Full` | **misleading doc** | [pitfall/002](../pitfall/002_a_departed_counterpart_is_indistinguishable_from_a_slow_one.md) |
| SP48 | The cursor type is imported from `ring_cursor` and its atomicity is that crate's guarantee, swapped under `--cfg loom`. | `SeqCell` | n/a — observation | [type/001](../type/001_producer_cursor.md) |
| SP49 | One producer means the returned number cannot go stale, but `ring_core` shares one signature across both backends and its callers take the weaker contract. | `free_capacity` | **misleading doc** | [type/002](../type/002_free_capacity.md) |
| SP50 | Both composed cores carry exactly ten, and no other crate in the family carries any. | the unsafe census | n/a — observation | [workaround/001](../workaround/001_the_unsafe_code_opt_out_and_what_bounds_it.md) |
| SP51 | No `Clone`, no `Sync`, no second live split, no batch outliving its commit — none is assertable by a test that runs. | `compile_fail` | n/a — observation | [workaround/001](../workaround/001_the_unsafe_code_opt_out_and_what_bounds_it.md) |
| SP52 | `ring_spsc::Producer` must not be `Sync`; `ring_mpsc::Producer` must be — and `ring_core` holds both behind one enum. | `Producer` | **latent hazard** | [workaround/002](../workaround/002_an_unsafe_impl_sync_on_a_type_whose_ends_are_not_sync.md) |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_spsc/docs
printf 'doc definitions:          '; ls -d */ | grep -vc '^definition/'
printf 'instances:                '; ls */[0-9][0-9][0-9]_*.md | wc -l
printf 'findings in the corpus:   '; grep -rhoE '^### SP[0-9]+ ' */[0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| SP[0-9]+ ' definition/readme.md
# doc definitions:          13
# instances:                28
# findings in the corpus:   52
# rows in the table below:  52
```
