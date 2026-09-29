# Doc Definitions

Module Index for `ring_seqno` — every doc definition this crate declares and every
instance under each, in one place.

## Master Doc Definitions Table

| Type | Purpose | Master File | Instances |
|------|---------|-------------|----------:|
| `algorithm/` | The computations, and the equivalences between them | [algorithm/readme.md](../algorithm/readme.md) | 2 |
| `api/` | The public surface and what its shape commits to | [api/readme.md](../api/readme.md) | 2 |
| `data_structure/` | What the crate operates on, all of it owned elsewhere | [data_structure/readme.md](../data_structure/readme.md) | 2 |
| `decisions/` | Choices with live alternatives, recorded with their arguments | [decisions/readme.md](../decisions/readme.md) | 2 |
| `integration/` | The one dependency, and how the readings reached four tiers | [integration/readme.md](../integration/readme.md) | 2 |
| `invariant/` | Properties that must hold for every input | [invariant/readme.md](../invariant/readme.md) | 2 |
| `item/` | Per-function contracts and coverage | [item/readme.md](../item/readme.md) | 2 |
| `lifecycle/` | The readings across a lap, and how long each stays true | [lifecycle/readme.md](../lifecycle/readme.md) | 2 |
| `non_functional_requirement/` | Cost and portability requirements | [non_functional_requirement/readme.md](../non_functional_requirement/readme.md) | 2 |
| `pattern/` | Reusable shapes this crate instantiates | [pattern/readme.md](../pattern/readme.md) | 2 |
| `pitfall/` | Wrong uses that compile and look right | [pitfall/readme.md](../pitfall/readme.md) | 2 |
| `type/` | What the readings are measured in | [type/readme.md](../type/readme.md) | 2 |
| `workaround/` | Compensations the family applies around this crate's shape | [workaround/readme.md](../workaround/readme.md) | 2 |

13 definitions, 26 instances.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_seqno/docs
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
| `algorithm/` | 001 | [Four Readings of One Subtraction](../algorithm/001_four_readings_of_one_subtraction.md) | The equivalence lattice, and which edge is tested |
| `algorithm/` | 002 | [The `slowest` Fold](../algorithm/002_the_slowest_fold.md) | `min()` over a slice, and its four-tier chain |
| `api/` | 001 | [Five Functions and No Types](../api/001_five_functions_and_no_types.md) | The whole surface, and what a type-free crate can promise |
| `api/` | 002 | [The Argument Order Split](../api/002_the_argument_order_split.md) | Three take `( ahead, behind )`; one takes `( behind, ahead )` |
| `data_structure/` | 001 | [The Crate That Declares No Type](../data_structure/001_the_crate_that_declares_no_type.md) | Zero declarations, two borrowed vocabulary types |
| `data_structure/` | 002 | [The Slice `slowest` Reads](../data_structure/002_the_slice_that_slowest_reads.md) | `&[ Seq ]`, and the allocation its shape forces one tier up |
| `decisions/` | 001 | [`None` Rather Than Zero for an Empty Set](../decisions/001_none_rather_than_zero_for_an_empty_set.md) | Why the fold declines to pick an identity |
| `decisions/` | 002 | [Saturating Rather Than Signed](../decisions/002_saturating_rather_than_signed.md) | Total arithmetic, and the state it erases |
| `integration/` | 001 | [One Dependency and Five Declared Dependents](../integration/001_one_dependency_and_five_declared_dependents.md) | The manifest graph, and the two genuinely unused edges family-wide |
| `integration/` | 002 | [How the Fold Crossed Four Tiers](../integration/002_how_the_fold_crossed_four_tiers.md) | Why `slowest` propagated where `on_distinct_lines` forked |
| `invariant/` | 001 | [The Sequence Is Never Folded Here](../invariant/001_the_sequence_is_never_folded_here.md) | No `%`, no capacity mask, no `ring_index` dependency |
| `invariant/` | 002 | [Every Reading Is Total](../invariant/002_every_reading_is_total.md) | No panic, no wrap, for any input — and what pays for it |
| `item/` | 001 | [The Three Capacity Readings](../item/001_the_three_capacity_readings.md) | `laps_between`, `may_claim`, `free_slots` — contracts and coverage |
| `item/` | 002 | [The Two Readings Without a Capacity](../item/002_the_two_readings_without_a_capacity.md) | `pending` and `slowest` — what a capacity-free reading can decide |
| `lifecycle/` | 001 | [One Pair Across One Lap](../lifecycle/001_one_pair_across_one_lap.md) | Every reading at every position of a capacity-4 cycle |
| `lifecycle/` | 002 | [The Validity Window of an Answer](../lifecycle/002_the_validity_window_of_an_answer.md) | Decay direction per reading; the one answer that decays unsafely |
| `non_functional_requirement/` | 001 | [Every Reading Is Allocation-Free](../non_functional_requirement/001_every_reading_is_allocation_free.md) | `no_std`-shaped, and the one allocation the family pays anyway |
| `non_functional_requirement/` | 002 | [The Arithmetic Must Survive a Narrow `usize`](../non_functional_requirement/002_the_arithmetic_must_survive_a_narrow_usize.md) | The single narrowing cast, and when it is reachable |
| `pattern/` | 001 | [The Predicate Beside Its Quantity](../pattern/001_the_predicate_beside_its_quantity.md) | `may_x()` shipped alongside `x_count()`, at three tiers |
| `pattern/` | 002 | [The Shared Fold That Declines an Identity](../pattern/002_the_shared_fold_that_declines_an_identity.md) | `Option` instead of an identity, and the four resolutions that justify it |
| `pitfall/` | 001 | [Implementing `may_claim` With `laps_between`](../pitfall/001_implementing_may_claim_with_laps_between.md) | The correct-and-costly reformulation |
| `pitfall/` | 002 | [Reading `free_slots` on a Narrow Target](../pitfall/002_reading_free_slots_on_a_narrow_target.md) | Where the two boundary readings disagree |
| `type/` | 001 | [What a Span Is Measured In](../type/001_what_a_span_is_measured_in.md) | `usize` vs `u64`, and the family-wide fault line |
| `type/` | 002 | [The `Option` That `slowest` Returns](../type/002_the_option_that_slowest_returns.md) | The one sum type in the crate's surface |
| `workaround/` | 001 | [The Diagnostic That Reimplements the Readings](../workaround/001_the_diagnostic_that_reimplements_the_readings.md) | `ring_debug` bypassing saturation, and the coupling that created |
| `workaround/` | 002 | [`laps_between` Has No Caller](../workaround/002_laps_between_has_no_caller.md) | An export nobody wanted, and the disposition options |

## Findings

54 findings, each evidenced in the instance that records it and repeated
verbatim in its own definition readme, so the three locations cannot drift apart.

| ID | Finding | Subject | Reachable | Where |
|----|---------|---------|:---------:|-------|
| SQ1 | Three predicates state the same boundary and only `may_claim` against `free_slots > 0` is swept; `may_claim` against `laps_between == 0` has no assertion anywhere in the crate | The three boundary equivalences | n/a — coverage | [algorithm/001](../algorithm/001_four_readings_of_one_subtraction.md) |
| SQ2 | The single unasserted equivalence is also the only one whose statement requires reversing argument order, so the one edge no test pins is the edge a caller is most likely to write backwards | The untested equivalence | **latent hazard** | [algorithm/001](../algorithm/001_four_readings_of_one_subtraction.md) |
| SQ3 | All 33 crates contain exactly three `/` operators outside comments — `ring_align`'s cache-line membership test, `ring_bench::destination_of`'s producer decode, and `laps_between`'s lap count. None is on a hot write or read path, and the third is still the only one in a function with no caller at all | The family's three divisions | n/a — observation | [algorithm/001](../algorithm/001_four_readings_of_one_subtraction.md) |
| SQ4 | Two `.min()` folds exist across the 33 crates and both functions are named `slowest`: this crate's `iter().copied().min()` and `ring_cursor`'s `iter().map( load ).min()`. The reading that this was the one decision *not* to fork is contradicted by SQ35 in this crate's own corpus — `b7e075ca` forked it deliberately, to delete the allocation the `&[ Seq ]` parameter forced | `.min()` | n/a — observation | [algorithm/002](../algorithm/002_the_slowest_fold.md) |
| SQ5 | Four of the five functions are const-legal and none is declared `const fn`, while all twelve `pub fn` in `ring_types` — the crate directly below — are | `const fn` | n/a — unadopted | [api/001](../api/001_five_functions_and_no_types.md) |
| SQ6 | All five functions carry `#[ must_use ]`, one of exactly three crates in the family at full coverage, and the crate has no `const fn` at all — the most disciplined about return values and the least about compile-time evaluation | `#[ must_use ]` | n/a — observation | [api/001](../api/001_five_functions_and_no_types.md) |
| SQ7 | `#![ deny( missing_docs ) ]` is the crate's only lint attribute, so nothing forbids `unsafe` and the crate's zero-`unsafe` property is a fact about the text rather than a guarantee about the future | Lint attributes | n/a — unenforced | [api/001](../api/001_five_functions_and_no_types.md) |
| SQ8 | `laps_between( earlier, later, … )` reverses the order of `may_claim`, `free_slots` and `pending`, all of which take the later position first, and `seq_test.rs:143-145` uses both conventions on three adjacent lines | Argument order | **latent hazard** | [api/002](../api/002_the_argument_order_split.md) |
| SQ9 | Declaring no type is what keeps the crate free of the fork risk that hit `ring_align`'s constant — there is nothing to copy | Declaring no type | n/a — observation | [data_structure/001](../data_structure/001_the_crate_that_declares_no_type.md) |
| SQ10 | Five functions reduce to four calls of `Seq::distance_to` and one `.min()`, so the crate borrows all of its arithmetic and contributes only the framing | The crate's whole body | n/a — observation | [data_structure/001](../data_structure/001_the_crate_that_declares_no_type.md) |
| SQ11 | `&[ Seq ]` cannot be produced from `&[ PaddedCursor ]` without materialising the loads — the whole origin of `ring_cursor`'s per-call `Vec`, which the caller resolved by ceasing to call rather than by materialising | `&[ Seq ]` | n/a — observation | [data_structure/002](../data_structure/002_the_slice_that_slowest_reads.md) |
| SQ12 | `slowest` is tested with at most three elements and never with a set larger than a single cache line's worth, while the wrapper that faces real gating sets is one crate up | The slice parameter | n/a — coverage | [data_structure/002](../data_structure/002_the_slice_that_slowest_reads.md) |
| SQ13 | `GatingSet::headroom` and `Barrier::available` resolve the same `None` to opposite values, which is the empirical proof that an identity would have been wrong | `None` for an empty set | n/a — observation | [decisions/001](../decisions/001_none_rather_than_zero_for_an_empty_set.md) |
| SQ14 | Saturation makes a swapped `may_claim` or `free_slots` return a permissive answer while a swapped `pending` or `laps_between` returns an inert one — a distinction the source draws nowhere | Saturation's two directions | n/a — doc gap | [decisions/002](../decisions/002_saturating_rather_than_signed.md) |
| SQ15 | The test's rationale says a swapped caller "gets an obviously-wrong answer instead of a plausible one", but zero laps is exactly the value that reads as room to publish — it documents the permissive failure as though it were the safe one | `laps_backward_read_zero` | **misleading doc** | [decisions/002](../decisions/002_saturating_rather_than_signed.md) |
| SQ16 | `may_claim` compared in `u64` while `free_slots` subtracted in `usize`, so the two readings that must agree did their arithmetic in different types — the mechanism that made a narrow-target divergence possible at all. `free_slots` now subtracts in `u64` too and narrows only its already-bounded result | The two integer widths | **latent hazard** | [decisions/002](../decisions/002_saturating_rather_than_signed.md) |
| SQ17 | `ring_atomic` declares `ring_seqno` in `Cargo.toml` and asserts the edge in three more documents; nothing in the crate uses it, and `unused_crate_dependencies` is not enabled anywhere in the workspace | `ring_atomic`'s edge | n/a — unenforced | [integration/001](../integration/001_one_dependency_and_five_declared_dependents.md) |
| SQ18 | Only `ring_gating` and `ring_batch` call these functions directly; every other consumer reaches them through `ring_cursor`'s wrappers, and `ring_batch` — the one that bypasses the wrapper layer — is also the one that wrote a redundant cast around the result | Direct callers | n/a — observation | [integration/001](../integration/001_one_dependency_and_five_declared_dependents.md) |
| SQ19 | A free function crosses a manifest boundary when each tier has a reason to wrap it — a type change supplies that reason, but does not keep the call: tier 2 stopped calling tier 1 in `b7e075ca` because the adaptation cost an allocation per call | Crossing a manifest boundary | n/a — observation | [integration/002](../integration/002_how_the_fold_crossed_four_tiers.md) |
| SQ20 | `Barrier::available` has no callers outside its own crate, while `ring_consume` — which holds a `Barrier` — re-expands its body inline | `Barrier::available` | n/a — duplication | [integration/002](../integration/002_how_the_fold_crossed_four_tiers.md) |
| SQ21 | The family calls `Seq::distance_to` fourteen times and four of those are here, making a 136-line crate the densest consumer of the one span primitive everything else is built on | `distance_to` density | n/a — observation | [integration/002](../integration/002_how_the_fold_crossed_four_tiers.md) |
| SQ22 | M1's grep is satisfied by the module doc alone; it would still pass if the folding invariant were violated in a function body | M1's grep | n/a — diagnostics | [invariant/001](../invariant/001_the_sequence_is_never_folded_here.md) |
| SQ23 | `positions_many_laps_apart_stay_comparable` performs the folding the crate exists to avoid in order to show what would be lost — the only place in the family a test computes the collision deliberately | The one test that folds | n/a — observation | [invariant/001](../invariant/001_the_sequence_is_never_folded_here.md) |
| SQ24 | The division in `laps_between` is total only because `Capacity::new` rejects zero — a guarantee from another crate, with nothing local restating it | Totality's source | n/a — observation | [invariant/002](../invariant/002_every_reading_is_total.md) |
| SQ25 | No function returns `Result` and the crate's single `Option` encodes an absent input rather than a failure, so every reading is total in the strict sense and the type signatures say so without a word of prose | Return types | n/a — observation | [invariant/002](../invariant/002_every_reading_is_total.md) |
| SQ26 | `free_slots` is the only reading that narrows at all, and it is the one two other crates decide writes with — which is why its narrowing was moved onto the already-bounded result rather than left on the raw distance | `free_slots` | n/a — observation | [item/001](../item/001_the_three_capacity_readings.md) |
| SQ27 | `laps_between` has zero callers outside this crate — none in any of the other 32 crates, in production or in test code | `laps_between`'s callers | n/a — observation | [item/001](../item/001_the_three_capacity_readings.md) |
| SQ28 | The crate crossed a width boundary three times — `capacity.get() as u64` twice, which cannot lose, and `distance_to( … ) as usize` once, which could — and named the risk at exactly one of the three. The losing cast is gone; line 98 now widens, saturates, then narrows a result bounded by `capacity` | The three casts | **latent hazard** | [item/001](../item/001_the_three_capacity_readings.md) |
| SQ29 | `pending` is `Seq::distance_to` with the arguments swapped and nothing added | `pending` | n/a — duplication | [item/002](../item/002_the_two_readings_without_a_capacity.md) |
| SQ30 | `pending` and `slowest` are the two functions that take no `Capacity`, and they are also the two whose answers are not bounded by one — the parameter list states the mathematical fact | The two capacity-free readings | n/a — observation | [item/002](../item/002_the_two_readings_without_a_capacity.md) |
| SQ31 | Every reading is a function of the difference alone; `slowest` alone is equivariant rather than invariant | Difference alone | n/a — observation | [lifecycle/001](../lifecycle/001_one_pair_across_one_lap.md) |
| SQ32 | Three of the four pair functions demonstrate themselves at capacity 4 and `laps_between` at 8, so the one function whose doc claims it "is the reading that decides" is also the one whose example cannot be read line-for-line against its neighbours | The doctest capacities | n/a — inconsistency | [lifecycle/001](../lifecycle/001_one_pair_across_one_lap.md) |
| SQ33 | `may_claim`'s `true` and `false` decay in opposite directions, and the crate documents neither — the warning exists only as a comment in `ring_claim` | Answer decay | n/a — doc gap | [lifecycle/002](../lifecycle/002_the_validity_window_of_an_answer.md) |
| SQ34 | `pending`'s doc is the only one of five that states the half-open interval convention ("up to but not including"), and the other four depend on it silently | The half-open convention | n/a — doc gap | [lifecycle/002](../lifecycle/002_the_validity_window_of_an_answer.md) |
| SQ35 | `slowest`'s `&[ Seq ]` parameter is what forces `ring_cursor` to heap-allocate in the condition of a lock-free retry loop | `slowest`'s parameter | **measured cost** | [non_functional_requirement/001](../non_functional_requirement/001_every_reading_is_allocation_free.md) |
| SQ36 | `laps_between` divides by a runtime value the compiler cannot prove is a power of two, so the family's only lap count is also its only integer division — and nothing calls it | The division's cost | n/a — observation | [non_functional_requirement/001](../non_functional_requirement/001_every_reading_is_allocation_free.md) |
| SQ37 | `free_slots` truncated on any target where `usize` is under 64 bits while `may_claim` and `laps_between` did not, and `free_slots` is the one two other crates gate writes with. It now widens, saturates, then narrows a result already bounded by `capacity`, so the loss is gone on every target width | Narrow `usize` | **latent hazard** | [non_functional_requirement/002](../non_functional_requirement/002_the_arithmetic_must_survive_a_narrow_usize.md) |
| SQ38 | The truncation was reachable in roughly seventy minutes at a modest publication rate, unlike the `Seq` overflow it superficially resembles — which is what made SQ37 worth fixing rather than documenting. The same fix closed it: the seventy-minute clock still runs, and has no truncation left to reach at the end of it | Time to reach it | **latent hazard** | [non_functional_requirement/002](../non_functional_requirement/002_the_arithmetic_must_survive_a_narrow_usize.md) |
| SQ39 | The predicate/quantity pair recurs at three tiers of the family, and at the third tier one half of it is unused | Predicate and quantity | n/a — observation | [pattern/001](../pattern/001_the_predicate_beside_its_quantity.md) |
| SQ40 | The sweep that pins the predicate to its quantity iterates the producer forward only, so the saturating branch both functions depend on is exercised by three point assertions and never by the sweep | The sweep's blind side | n/a — coverage | [pattern/001](../pattern/001_the_predicate_beside_its_quantity.md) |
| SQ41 | `ring_gating` and `ring_barrier` resolve the same `None` to `capacity` and `0` — the empirical proof the fold was right to decline an identity | Opposite resolutions | n/a — observation | [pattern/002](../pattern/002_the_shared_fold_that_declines_an_identity.md) |
| SQ42 | Twenty-three lines of module doc cite four external documents, one of which exists to correct this crate's own earlier description | The module doc's citations | n/a — observation | [pattern/002](../pattern/002_the_shared_fold_that_declines_an_identity.md) |
| SQ43 | `ring_batch::claim_gated` takes an `order : Ordering` and hardcodes `Acquire` for two of its three atomic operations — the shape this instance warns about, already present one crate over | `ring_batch::claim_gated` | n/a — observation | [pitfall/001](../pitfall/001_implementing_may_claim_with_laps_between.md) |
| SQ44 | `Seq::next`'s doc claims release-mode saturation; the body is `Self( self.0 + 1 )` and no `overflow-checks` override exists, so release wraps | `Seq::next` | n/a — doc gap | [pitfall/001](../pitfall/001_implementing_may_claim_with_laps_between.md) |
| SQ45 | `ring_batch:323` casts `free_slots( … ) as usize` when it already returns `usize` — a no-op that would have silently absorbed a *widening* repair. The repair taken narrowed the result instead, so the cast absorbed nothing and is still there, still loaded for the next person; SQ54 is why nothing reports it | `ring_batch`'s cast | n/a — observation | [pitfall/002](../pitfall/002_reading_free_slots_on_a_narrow_target.md) |
| SQ46 | The sweep that would have caught the narrowing ran entirely inside values where `usize` and `u64` coincide, so it passed identically on a 32-bit target and the divergence at 2^32 had no test that could reach it. The suite is unchanged after the fix, because the repair was structural | The sweep's width | n/a — coverage | [pitfall/002](../pitfall/002_reading_free_slots_on_a_narrow_target.md) |
| SQ47 | `free_slots` returns `usize` and `pending` returns `u64` for quantities in the same units, which is what makes the narrowing cast look deliberate | Two widths for one unit | n/a — inconsistency | [type/001](../type/001_what_a_span_is_measured_in.md) |
| SQ48 | A free-slot count and a slot index are both `usize` in `0..=capacity`, so the compiler cannot separate them | Count and index | n/a — observation | [type/001](../type/001_what_a_span_is_measured_in.md) |
| SQ49 | `Option< Seq >` is 16 bytes — `Seq` has no niche — and is never stored anywhere in the family | `Option< Seq >` | n/a — observation | [type/002](../type/002_the_option_that_slowest_returns.md) |
| SQ50 | The test suite's one `expect` is in the `cap` helper, so every assertion in ten tests rests on a capacity constructor from another crate and the suite has no way to fail other than by asserting | The single failure path | n/a — observation | [type/002](../type/002_the_option_that_slowest_returns.md) |
| SQ51 | `check_seqs`'s two guards must stay in their current order or the second one panics — an undocumented coupling created by bypassing `distance_to` | `check_seqs`'s guards | **latent hazard** | [workaround/001](../workaround/001_the_diagnostic_that_reimplements_the_readings.md) |
| SQ52 | `may_claim`'s `<` and `check_seqs`'s `>` are exact complements around one boundary, in different crates, with nothing linking them | Complementary comparisons | n/a — duplication | [workaround/001](../workaround/001_the_diagnostic_that_reimplements_the_readings.md) |
| SQ53 | `laps_between` has zero callers outside this crate while its own doc claims it "is the reading that decides" publication safety — the role `may_claim` actually fills | `laps_between`'s doc | n/a — doc gap | [workaround/002](../workaround/002_laps_between_has_no_caller.md) |
| SQ54 | `clippy::unnecessary_cast` is warn-by-default in `complexity`, needs no `pedantic` opt-in, and is active in `ring_batch` — it flags four other redundant-cast shapes in that very file — yet says nothing about `ring_batch:323`: it skips casts on values returned by free functions from *other* crates. SQ45 is therefore reachable by no automated check, only by reading | `clippy::unnecessary_cast` | n/a — observation | [pitfall/002](../pitfall/002_reading_free_slots_on_a_narrow_target.md) |

**5 of the 54 are reachable** — SQ2, SQ8, SQ15, SQ35, SQ51. Four more were, and are the only ones ever fixed: SQ16, SQ28, SQ37 and SQ38 all
named the same narrowing cast, and one change to `free_slots` closed all four at once. The rest are records: an observation, a gap in
documentation, a gap in coverage, a duplication, or a convention nothing enforces.

### Severity

| Tier | Findings | Count |
|------|----------|------:|
| **latent hazard** | SQ2, SQ8, SQ16, SQ28, SQ37, SQ38, SQ51 | 7 |
| **measured cost** | SQ35 | 1 |
| **misleading doc** | SQ15 | 1 |
| n/a — coverage | SQ1, SQ12, SQ40, SQ46 | 4 |
| n/a — diagnostics | SQ22 | 1 |
| n/a — doc gap | SQ14, SQ33, SQ34, SQ44, SQ53 | 5 |
| n/a — duplication | SQ20, SQ29, SQ52 | 3 |
| n/a — inconsistency | SQ32, SQ47 | 2 |
| n/a — observation | SQ3, SQ4, SQ6, SQ9, SQ10, SQ11, SQ13, SQ18, SQ19, SQ21, SQ23, SQ24, SQ25, SQ26, SQ27, SQ30, SQ31, SQ36, SQ39, SQ41, SQ42, SQ43, SQ45, SQ48, SQ49, SQ50, SQ54 | 27 |
| n/a — unadopted | SQ5 | 1 |
| n/a — unenforced | SQ7, SQ17 | 2 |
