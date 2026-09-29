# Doc Definitions

## Master Doc Definitions Table

| Type | Purpose | Master File | Instances |
|------|---------|-------------|----------:|
| `algorithm/` | The producer's claim-and-publish procedure and the consumer's batch drain, step by step | [algorithm/readme.md](../algorithm/readme.md) | 2 |
| `api/` | The two asymmetric caller surfaces — many lock-free producers, one batch-shaped consumer | [api/readme.md](../api/readme.md) | 2 |
| `data_structure/` | The ring's field-level shape — slots, per-slot stamps, and the two cache-line-separated cursors | [data_structure/readme.md](../data_structure/readme.md) | 2 |
| `decisions/` | Two open questions the item census raised: a name on the unsafe allowlist that carries no unsafe, and an observation surface with no caller | [decisions/readme.md](../decisions/readme.md) | 2 |
| `integration/` | The eight sibling crates beneath this one and the prospective consumers above it | [integration/readme.md](../integration/readme.md) | 2 |
| `invariant/` | The producer/consumer contract any winning ring mechanism must hold | [invariant/readme.md](../invariant/readme.md) | 2 |
| `item/` | Eighty items by kind, the four names that leave the crate, and the five constants nothing imports | [item/readme.md](../item/readme.md) | 2 |
| `lifecycle/` | The ring's four phases, a producer's shorter cycle nested inside them, and the slot and occupancy states both move through | [lifecycle/readme.md](../lifecycle/readme.md) | 4 |
| `non_functional_requirement/` | The measured adoption gate — quality thresholds a benchmark can pass or fail | [non_functional_requirement/readme.md](../non_functional_requirement/readme.md) | 2 |
| `pattern/` | How a declared channel binds to a ring instance and to a compile-time slot type | [pattern/readme.md](../pattern/readme.md) | 2 |
| `pitfall/` | Traps that arrive bundled with the ring pattern, and what each costs the wrong workload | [pitfall/readme.md](../pitfall/readme.md) | 2 |
| `type/` | The value types the correctness arguments are written in — sequence and capacity | [type/readme.md](../type/readme.md) | 2 |
| `workaround/` | The one language constraint this crate absorbs, seen from both sides — the opt-out and the `Sync` impl | [workaround/readme.md](../workaround/readme.md) | 2 |

## Master Doc Instances Table

| Definition | ID | Name | File |
|------------|----|------|------|
| `algorithm/` | 001 | Claim-Then-Publish Slot Acquisition | [001_claim_then_publish.md](../algorithm/001_claim_then_publish.md) |
| `algorithm/` | 002 | Batch Drain by Single Cursor Swap | [002_batch_drain_by_cursor_swap.md](../algorithm/002_batch_drain_by_cursor_swap.md) |
| `api/` | 001 | Producer Publish Surface | [001_producer_publish_surface.md](../api/001_producer_publish_surface.md) |
| `api/` | 002 | Consumer Drain Surface | [002_consumer_drain_surface.md](../api/002_consumer_drain_surface.md) |
| `data_structure/` | 001 | Sequence-Stamped Ring | [001_sequence_stamped_ring.md](../data_structure/001_sequence_stamped_ring.md) |
| `data_structure/` | 002 | A Second Array of Sequence Stamps | [002_a_second_array_of_sequence_stamps.md](../data_structure/002_a_second_array_of_sequence_stamps.md) |
| `decisions/` | 001 | `ring_core` Sits on the Unsafe Allowlist Without Unsafe | [001_ring_core_sits_on_the_unsafe_allowlist_without_unsafe.md](../decisions/001_ring_core_sits_on_the_unsafe_allowlist_without_unsafe.md) |
| `decisions/` | 002 | The Observation Surface Kept Without a Caller | [002_the_observation_surface_kept_without_a_caller.md](../decisions/002_the_observation_surface_kept_without_a_caller.md) |
| `integration/` | 001 | Family Dependency Seam | [001_family_dependency_seam.md](../integration/001_family_dependency_seam.md) |
| `integration/` | 002 | Prospective Consumer Adoption | [002_prospective_consumer_adoption.md](../integration/002_prospective_consumer_adoption.md) |
| `invariant/` | 001 | Single-Consumer Total Order | [001_single_consumer_total_order.md](../invariant/001_single_consumer_total_order.md) |
| `invariant/` | 002 | Publication Ordering — Claim Acquire, Publish Release | [002_publication_ordering.md](../invariant/002_publication_ordering.md) |
| `item/` | 001 | Eighty Items, and the Four Names That Leave the Crate | [001_eighty_items_and_the_four_names_that_leave_the_crate.md](../item/001_eighty_items_and_the_four_names_that_leave_the_crate.md) |
| `item/` | 002 | Five Public Ordering Constants | [002_five_public_ordering_constants.md](../item/002_five_public_ordering_constants.md) |
| `lifecycle/` | 001 | Ring Construction and Teardown | [001_ring_construction_and_teardown.md](../lifecycle/001_ring_construction_and_teardown.md) |
| `lifecycle/` | 002 | Producer Attachment and Detachment | [002_producer_attachment_and_detachment.md](../lifecycle/002_producer_attachment_and_detachment.md) |
| `lifecycle/` | 003 | Slot State Across One Lap | [003_slot_state_across_one_lap.md](../lifecycle/003_slot_state_across_one_lap.md) |
| `lifecycle/` | 004 | Ring Occupancy Between the Cursors | [004_ring_occupancy_between_cursors.md](../lifecycle/004_ring_occupancy_between_cursors.md) |
| `non_functional_requirement/` | 001 | Measured Before Adopted | [001_measured_before_adopted.md](../non_functional_requirement/001_measured_before_adopted.md) |
| `non_functional_requirement/` | 002 | Bounded Capacity and Backpressure Policy | [002_bounded_capacity_backpressure.md](../non_functional_requirement/002_bounded_capacity_backpressure.md) |
| `pattern/` | 001 | Channel-to-Ring Binding | [001_channel_to_ring_binding.md](../pattern/001_channel_to_ring_binding.md) |
| `pattern/` | 002 | A Sequence Stamp as a Lap-Safe Publication Marker | [002_a_sequence_stamp_as_a_lap_safe_publication_marker.md](../pattern/002_a_sequence_stamp_as_a_lap_safe_publication_marker.md) |
| `pitfall/` | 001 | The Spinning Consumer Owns a Core | [001_spinning_consumer_owns_a_core.md](../pitfall/001_spinning_consumer_owns_a_core.md) |
| `pitfall/` | 002 | A Stale Stamp Reads as Unpublished, Not as Wrong | [002_a_stale_stamp_reads_as_unpublished_not_as_wrong.md](../pitfall/002_a_stale_stamp_reads_as_unpublished_not_as_wrong.md) |
| `type/` | 001 | Sequence Number | [001_sequence_number.md](../type/001_sequence_number.md) |
| `type/` | 002 | Capacity | [002_capacity.md](../type/002_capacity.md) |
| `workaround/` | 001 | The `unsafe_code` Opt-Out and Its Obligations | [001_the_unsafe_code_opt_out_and_its_obligations.md](../workaround/001_the_unsafe_code_opt_out_and_its_obligations.md) |
| `workaround/` | 002 | An `unsafe impl Sync` the Compiler Cannot Derive | [002_an_unsafe_impl_sync_the_compiler_cannot_derive.md](../workaround/002_an_unsafe_impl_sync_the_compiler_cannot_derive.md) |

## Findings

52, each argued in the instance named under **Where** and verified
there by a command whose output is quoted beneath it.

| ID | Finding | Subject | Reachable | Where |
|----|---------|---------|-----------|-------|
| MP1 | This crate owns the stamp and the slot; the cursor arithmetic and headroom check belong to `ring_claim` and `ring_cursor`. | the claim step | n/a — observation | [algorithm/001](../algorithm/001_claim_then_publish.md) |
| MP2 | Dropping a `Reserved` guard without calling `set` publishes whatever the slot already held, and one test pins that behaviour as intended. | `Reserved` | **latent hazard** | [algorithm/001](../algorithm/001_claim_then_publish.md) |
| MP3 | A published sequence behind an unpublished one is not drained, which makes one slow producer stall every later record. | `drain` | n/a — observation | [algorithm/002](../algorithm/002_batch_drain_by_cursor_swap.md) |
| MP4 | A `max` exceeding capacity is clamped, so a caller cannot walk into the next lap by asking for too much. | `drain_up_to` | n/a — observation | [algorithm/002](../algorithm/002_batch_drain_by_cursor_swap.md) |
| MP5 | `Producer` is `Send + Sync + Copy`, so duplication needs no method and no reference count. | `Producer` | n/a — observation | [api/001](../api/001_producer_publish_surface.md) |
| MP6 | Copying a producer yields another view of the same cursor, which is the property that makes `Copy` sound. | `Producer` | n/a — coverage | [api/001](../api/001_producer_publish_surface.md) |
| MP7 | The consumer cursor advances when the batch drops, not when it is created, so slots stay reserved while records are read. | `Batch` | n/a — observation | [api/002](../api/002_consumer_drain_surface.md) |
| MP8 | `Consumer::is_empty` means "nothing drainable now"; `Batch::is_empty` means "this batch took nothing" — and the names are identical. | `is_empty` | **latent hazard** | [api/002](../api/002_consumer_drain_surface.md) |
| MP9 | Wrapping the whole `Buffer` materialised `&mut Buffer< S >` per write, aliasing the entire allocation; Miri caught it as a retag conflict. | `slots` | n/a — observation | [data_structure/001](../data_structure/001_sequence_stamped_ring.md) |
| MP10 | `Buffer< UnsafeCell< S > >` was unconstructible until `Buffer::new` was bounded on `Default` rather than `Slot`. | `ring_store` | n/a — observation | [data_structure/001](../data_structure/001_sequence_stamped_ring.md) |
| MP11 | The two cursors get `PaddedCursor`; the per-slot stamps are deliberately unpadded, on an argument about which contention is structural. | `padding` | **measured cost** | [data_structure/002](../data_structure/002_a_second_array_of_sequence_stamps.md) |
| MP12 | The sentinel is a legal `Seq` value, made safe by a wrap bound rather than by the type. | `UNSTAMPED` | n/a — observation | [data_structure/002](../data_structure/002_a_second_array_of_sequence_stamps.md) |
| MP13 | `ring_core` is declared as permitted to opt out of `unsafe-code = "deny"` and contains no `unsafe` and no opt-out attribute. | the allowlist | n/a — inconsistency | [decisions/001](../decisions/001_ring_core_sits_on_the_unsafe_allowlist_without_unsafe.md) |
| MP14 | The gate's scan pattern omitted the house spacing, so its justification half never ran for any crate. | `G6` | **latent hazard** | [decisions/001](../decisions/001_ring_core_sits_on_the_unsafe_allowlist_without_unsafe.md) |
| MP15 | The allowlist requires each named crate to justify its opt-out in its own `docs/workaround/readme.md`; this crate's said "None" and named a dependency list that was wrong. | `workaround/readme.md` | n/a — doc gap | [decisions/001](../decisions/001_ring_core_sits_on_the_unsafe_allowlist_without_unsafe.md) |
| MP16 | Every public method that reports state rather than changing it has zero production callers; every method that changes state has some. | the observation surface | n/a — unadopted | [decisions/002](../decisions/002_the_observation_surface_kept_without_a_caller.md) |
| MP17 | Making the seven private would make three of this crate's own assertions unwritable where the family requires tests to live. | the observation surface | n/a — coverage | [decisions/002](../decisions/002_the_observation_surface_kept_without_a_caller.md) |
| MP18 | Every dependency is a workspace sibling, so nothing outside this repository can impose a version constraint here. | `Cargo.toml` | n/a — observation | [integration/001](../integration/001_family_dependency_seam.md) |
| MP19 | The import count exceeds the dependency count because several siblings contribute more than one name. | the seam | n/a — observation | [integration/001](../integration/001_family_dependency_seam.md) |
| MP20 | `ring_bench` takes `Ring` alone; every handle type is reached through `ring_core`. | `adoption` | n/a — unadopted | [integration/002](../integration/002_prospective_consumer_adoption.md) |
| MP21 | The observation surface with no caller is exactly what a monitoring or backpressure adopter would need first. | `adoption` | n/a — observation | [integration/002](../integration/002_prospective_consumer_adoption.md) |
| MP22 | `ends` takes `&mut self` and `split` takes `&mut Ends`, so a second consumer cannot be obtained while a first is alive. | consumer uniqueness | n/a — observation | [invariant/001](../invariant/001_single_consumer_total_order.md) |
| MP23 | The test asserting that every handle points at the same ring is the one that would catch a split producing handles onto different allocations. | handle identity | n/a — coverage | [invariant/001](../invariant/001_single_consumer_total_order.md) |
| MP24 | The loom model needs `--cfg loom` and the 60-run mutation is manual, so an ordinary run checks this invariant only at the level of the constants' declared values. | the ordering checks | n/a — unenforced | [invariant/002](../invariant/002_publication_ordering.md) |
| MP25 | Earlier design material advances the read cursor `Relaxed`; this crate uses `Release` and says why at the constant. | `COMMIT` | n/a — observation | [invariant/002](../invariant/002_publication_ordering.md) |
| MP26 | The two dependent crates name `Ring`, `Ends`, `Producer` and `Consumer`, and nothing else. | the public surface | n/a — observation | [item/001](../item/001_eighty_items_and_the_four_names_that_leave_the_crate.md) |
| MP27 | Slot state is cursor arithmetic rather than a discriminant, so the crate has no state type to point at. | the census | n/a — observation | [item/001](../item/001_eighty_items_and_the_four_names_that_leave_the_crate.md) |
| MP28 | The impl count exceeds the struct count because several types carry separate `impl` blocks per bound. | the census | n/a — observation | [item/001](../item/001_eighty_items_and_the_four_names_that_leave_the_crate.md) |
| MP29 | Every one of the five carries a rustdoc example asserting its value, so the vocabulary is checked by `cargo test --doc`. | the five constants | n/a — coverage | [item/002](../item/002_five_public_ordering_constants.md) |
| MP30 | `PUBLISH`/`OBSERVE`/`COMMIT`/`OWN` name what the access is for; two of the four have the same `Ordering` value. | the five constants | n/a — observation | [item/002](../item/002_five_public_ordering_constants.md) |
| MP31 | The slot buffer and the stamp array are the only two allocations in the crate, both at construction. | `Ring::new` | n/a — observation | [lifecycle/001](../lifecycle/001_ring_construction_and_teardown.md) |
| MP32 | The config constructor takes the capacity and deliberately does not read the overflow policy or the producer count. | `with_config` | n/a — observation | [lifecycle/001](../lifecycle/001_ring_construction_and_teardown.md) |
| MP33 | A producer that goes away leaves no trace, because it never had state of its own to release. | `Producer` | n/a — observation | [lifecycle/002](../lifecycle/002_producer_attachment_and_detachment.md) |
| MP34 | The producer can read back what it wrote before the stamp store makes it visible to the consumer. | `Reserved` | n/a — observation | [lifecycle/003](../lifecycle/003_slot_state_across_one_lap.md) |
| MP35 | `a_taken_record_leaves_its_slot_empty` is what separates "drained" from "still present but past the cursor". | slot state | n/a — coverage | [lifecycle/003](../lifecycle/003_slot_state_across_one_lap.md) |
| MP36 | `a_commit_restores_exactly_the_capacity_it_released` is the conservation law occupancy depends on. | `occupancy` | n/a — coverage | [lifecycle/004](../lifecycle/004_ring_occupancy_between_cursors.md) |
| MP37 | `ring_bench` imports `Ring` from this crate but reaches the publish path through `ring_core`, so what is measured is the composition. | `measurement` | n/a — observation | [non_functional_requirement/001](../non_functional_requirement/001_measured_before_adopted.md) |
| MP38 | The observation surface is neither called nor measured, so its cost is unknown as well as unused. | `measurement` | n/a — coverage | [non_functional_requirement/001](../non_functional_requirement/001_measured_before_adopted.md) |
| MP39 | A full ring returns `RingError::Full`; it never blocks, never grows, and never drops. | `backpressure` | n/a — observation | [non_functional_requirement/002](../non_functional_requirement/002_bounded_capacity_backpressure.md) |
| MP40 | The two arrays are sized once, so a burst is absorbed by refusal rather than by growth — which is what makes the bound meaningful. | `Capacity` | n/a — observation | [non_functional_requirement/002](../non_functional_requirement/002_bounded_capacity_backpressure.md) |
| MP41 | The pattern describes how a channel-shaped API binds onto this ring; exactly one crate does it. | channel-to-ring binding | n/a — unadopted | [pattern/001](../pattern/001_channel_to_ring_binding.md) |
| MP42 | The sibling crate wraps the same way and reuses slots the same way, and uses cursor comparison instead — which is the pattern's applicability condition made concrete. | the stamp pattern | n/a — observation | [pattern/002](../pattern/002_a_sequence_stamp_as_a_lap_safe_publication_marker.md) |
| MP43 | Nothing in the test reads a cursor, which is what lets the drain scan stop at the first gap without coordination. | the stamp pattern | n/a — observation | [pattern/002](../pattern/002_a_sequence_stamp_as_a_lap_safe_publication_marker.md) |
| MP44 | `try_recv`-shaped drain with no blocking variant means a consumer that wants to wait must write the loop itself. | `spinning` | n/a — observation | [pitfall/001](../pitfall/001_spinning_consumer_owns_a_core.md) |
| MP45 | The pitfall names a cost — one core — and no benchmark in the family produces the number. | `spinning` | n/a — coverage | [pitfall/001](../pitfall/001_spinning_consumer_owns_a_core.md) |
| MP46 | `stamp != UNSTAMPED` and `stamp == seq` agree on every ring that never wraps, so a suite that fills a ring once cannot distinguish them. | the stale-stamp trap | **latent hazard** | [pitfall/002](../pitfall/002_a_stale_stamp_reads_as_unpublished_not_as_wrong.md) |
| MP47 | The accessor exposes `&[ AtomicSeq ]`, so an external reader must reimplement the publication test — and the wrong version is the intuitive one. | `stamps` | **latent hazard** | [pitfall/002](../pitfall/002_a_stale_stamp_reads_as_unpublished_not_as_wrong.md) |
| MP48 | This crate stamps, compares and folds sequences without owning the type or any of its guarantees. | `Seq` | n/a — observation | [type/001](../type/001_sequence_number.md) |
| MP49 | Slot lookup is `( seq.0 as usize ) & capacity.mask()` with no bounds check, which is sound only because `Capacity::new` rejected zero and non-powers-of-two. | `Capacity` | n/a — observation | [type/002](../type/002_capacity.md) |
| MP50 | The four call sites of `slot`/`slot_mut` are all inside `Reserved` or `Batch`, neither of which can be constructed for an ungated sequence. | the unsafe sites | n/a — observation | [workaround/001](../workaround/001_the_unsafe_code_opt_out_and_its_obligations.md) |
| MP51 | `ring_mpsc` and `ring_spsc` each carry exactly ten `unsafe` lines in code, and no other crate in the family carries any. | the unsafe census | n/a — observation | [workaround/001](../workaround/001_the_unsafe_code_opt_out_and_its_obligations.md) |
| MP52 | The `Sync` impl's first safety clause cites `ends`' and `split`'s receivers, so a convenience change to either invalidates it silently. | consumer uniqueness | **latent hazard** | [workaround/002](../workaround/002_an_unsafe_impl_sync_the_compiler_cannot_derive.md) |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_mpsc/docs
printf 'doc definitions:          '; ls -d */ | grep -vc '^definition/'
printf 'instances:                '; ls */[0-9][0-9][0-9]_*.md | wc -l
printf 'findings in the corpus:   '; grep -rhoE '^### MP[0-9]+ ' */[0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| MP[0-9]+ ' definition/readme.md
# doc definitions:          13
# instances:                28
# findings in the corpus:   52
# rows in the table below:  52
```
