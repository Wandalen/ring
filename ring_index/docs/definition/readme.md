# Doc Definitions

Module Index for `ring_index` — every doc definition this crate declares and every
instance under each, in one place.

## Master Doc Definitions Table

| Type | Purpose | Master File | Instances |
|------|---------|-------------|----------:|
| `algorithm/` | One `and` where a modulo would be, and the same fold applied `count` times | [algorithm/readme.md](../algorithm/readme.md) | 2 |
| `api/` | Three functions of which one is reached, and the `const` all three could be | [api/readme.md](../api/readme.md) | 2 |
| `data_structure/` | A ring with no circular layout, and the one collection the crate can own | [data_structure/readme.md](../data_structure/readme.md) | 2 |
| `decisions/` | A precondition enforced one crate up, and a container chosen without a record | [decisions/readme.md](../decisions/readme.md) | 2 |
| `definition/` | Module Index — every definition and every instance in this crate, in one place | this file | — |
| `integration/` | Two dependents, a third that folded by hand, and a feature still marked planned | [integration/readme.md](../integration/readme.md) | 2 |
| `invariant/` | An identity asserted over a prefix, and totality claimed for one function of three | [invariant/readme.md](../invariant/readme.md) | 2 |
| `item/` | The one comment stated a lap too narrow, and two comments about having no callers | [item/readme.md](../item/readme.md) | 2 |
| `lifecycle/` | A cycle whose period this crate sets and whose phases it skips, and no lifecycle of its own | [lifecycle/readme.md](../lifecycle/readme.md) | 2 |
| `non_functional_requirement/` | The cycle count the module comment gets wrong, and one allocation nothing pays | [non_functional_requirement/readme.md](../non_functional_requirement/readme.md) | 2 |
| `pattern/` | One owner for one arithmetic fact, and the shape a crate with no state takes | [pattern/readme.md](../pattern/readme.md) | 2 |
| `pitfall/` | A run that panics in debug and wraps in release, and a second fold in `ring_mpsc` | [pitfall/readme.md](../pitfall/readme.md) | 2 |
| `type/` | Three types borrowed and none owned, and a prohibition a `pub` field cannot enforce | [type/readme.md](../type/readme.md) | 2 |
| `workaround/` | A bitmask published one crate up, and the iterator `ring_batch` wrote instead | [workaround/readme.md](../workaround/readme.md) | 2 |

13 definitions, 26 instances.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_index/docs
find . -mindepth 1 -maxdepth 1 -type d ! -name definition | wc -l   # definitions
find . -name '0*.md' | wc -l                                        # instances
command grep -rho '^### IX[0-9]*' . | wc -l                         # findings
```

Live output:

```
13
26
56
```

## Master Doc Instances Table

| Type | ID | Title | Subject |
|------|----|-------|---------|
| `algorithm/` | 001 | [One `and` of a Mask](../algorithm/001_one_and_of_a_mask.md) | The body, the instruction it compiles to, and the validation it inherits |
| `algorithm/` | 002 | [A Run Is the Fold Applied `count` Times](../algorithm/002_a_run_is_the_fold_applied_count_times.md) | The loop, the constraint its comment describes that is not one, and where it overflows |
| `api/` | 001 | [Three Functions, Three `must_use`, One Reached](../api/001_three_functions_three_must_use_one_reached.md) | A surface of three, of which two crates import exactly one |
| `api/` | 002 | [The Three Signatures and the `const` They Are Not](../api/002_the_three_signatures_and_the_const_they_are_not.md) | `of` const-evaluating unchanged, and the accessor call that would have made `aliases` const |
| `data_structure/` | 001 | [The Ring Is a Computation, Not a Layout](../data_structure/001_the_ring_is_a_computation_not_a_layout.md) | Two flat fields, no head or tail, and zero `SlotIndex` fields family-wide |
| `data_structure/` | 002 | [The One Collection the Crate Builds](../data_structure/002_the_one_collection_the_crate_builds.md) | `run`'s `Vec`: exactly sized by body shape, mostly header at batch sizes |
| `decisions/` | 001 | [A Power of Two, or Nothing](../decisions/001_a_power_of_two_or_nothing.md) | One enforcement site, zero rounding affordances, and a justification that omits totality |
| `decisions/` | 002 | [A `Vec` Where an Iterator Would Do](../decisions/002_a_vec_where_an_iterator_would_do.md) | The crate's only real decision, made silently, and the change that breaks nothing |
| `integration/` | 001 | [Two Dependents, and a Third That Did It Again](../integration/001_two_dependents_and_a_third_that_did_it_again.md) | Transitive reach through `Buffer::at`, a claim-path error, and `ring_mpsc`'s duplicate |
| `integration/` | 002 | [The Feature It Implements Half Of](../integration/002_the_feature_it_implements_half_of.md) | A feature marked planned while three crates implement it, and a benchmark a level too high |
| `invariant/` | 001 | [The Mask Equals the Modulo](../invariant/001_the_mask_equals_the_modulo.md) | Exhaustive to 4095, sampled to `u32::MAX`, and a truncating cast proved nowhere |
| `invariant/` | 002 | [The Fold Is Total and the Run Is Not](../invariant/002_the_fold_is_total_and_the_run_is_not.md) | One totality claim read as crate-wide, and a boundary that is not where a reader guesses |
| `item/` | 001 | [The Fold Itself](../item/001_the_fold_itself.md) | "Exactly one lap apart" — one lap narrower than the property, on the function everyone calls |
| `item/` | 002 | [The Two That Nothing Calls](../item/002_the_two_that_nothing_calls.md) | One comment that justifies zero callers correctly, one that cites the feature that replaced it |
| `lifecycle/` | 001 | [The Lap Is the Only Cycle](../lifecycle/001_the_lap_is_the_only_cycle.md) | Six phases, a period this crate sets, and a gate that makes exactly one lap reachable |
| `lifecycle/` | 002 | [No Initialization and No Teardown](../lifecycle/002_no_initialization_and_no_teardown.md) | Zero `impl` blocks, and ten tests that never build a ring |
| `non_functional_requirement/` | 001 | [What the Fold Costs](../non_functional_requirement/001_what_the_fold_costs.md) | 7×, not 20–40×, on the wrong architecture — and a constant divisor that costs nothing at all |
| `non_functional_requirement/` | 002 | [The One Allocation and the Zero Callers](../non_functional_requirement/002_the_one_allocation_and_the_zero_callers.md) | One allocation per call, exactly sized, disqualifying, and never paid |
| `pattern/` | 001 | [One Owner for One Arithmetic Fact](../pattern/001_one_owner_for_one_arithmetic_fact.md) | A rule with no lint behind it, and the narrower rule that predicts its one violation |
| `pattern/` | 002 | [Stateless Arithmetic Over Borrowed Types](../pattern/002_stateless_arithmetic_over_borrowed_types.md) | Eight functions across two crates with no struct, trait, static, or `mut` |
| `pitfall/` | 001 | [The Run That Panics in Debug and Wraps in Release](../pitfall/001_the_run_that_panics_in_debug_and_wraps_in_release.md) | An inherited overflow, a doc that names saturation where Rust wraps, and `UNSTAMPED` |
| `pitfall/` | 002 | [The Second Fold Nobody Noticed](../pitfall/002_the_second_fold_nobody_noticed.md) | A mask from one capacity indexing an array sized by another, three copies of one number |
| `type/` | 001 | [Three Types Borrowed, None Owned](../type/001_three_types_borrowed_none_owned.md) | Two transparent newtypes, one opaque, and the privacy that makes `of` total |
| `type/` | 002 | [The Sentence the Public Field Contradicts](../type/002_the_sentence_the_public_field_contradicts.md) | "Never constructed by counting," a `pub usize`, and 36 constructions that count |
| `workaround/` | 001 | [The Mask That Lives One Crate Up](../workaround/001_the_mask_that_lives_one_crate_up.md) | Why the crate exists, what a public `mask()` costs, and the method move that deletes it |
| `workaround/` | 002 | [The Iterator `ring_batch` Built Instead](../workaround/002_the_iterator_ring_batch_built_instead.md) | `run` with the allocation removed, carrying the only mechanical test of the one-owner rule |

## Findings

Fifty-six, each verified by a command whose output is quoted in its instance.

| ID | Finding | Subject | Reachable | Where |
|----|---------|---------|-----------|-------|
| IX1 | Totality is bought by a constructor one crate away; neither `mask` nor `of` checks anything | `ring_index` | n/a — observation | [algorithm/001](../algorithm/001_one_and_of_a_mask.md) |
| IX2 | The mask identity holds at `u64::MAX`; no test goes above `u32::MAX` | `ring_index` | n/a — coverage | [algorithm/001](../algorithm/001_one_and_of_a_mask.md) |
| IX3 | `run`'s doc states "wraps at most once" as a property of `run`; it is a property of batch claims, and `run` accepts any `count` | `ring_index` | **misleading doc** | [algorithm/002](../algorithm/002_a_run_is_the_fold_applied_count_times.md) |
| IX4 | `ring_batch::drain_order` is a lazy `run` built from `of`; the one crate with the use case did not call `run` | `ring_batch` | n/a — duplication | [algorithm/002](../algorithm/002_a_run_is_the_fold_applied_count_times.md) |
| IX5 | `#[ must_use ]` is 3 of 3, because a stateless crate has no function whose value is incidental | `ring_index` | n/a — observation | [api/001](../api/001_three_functions_three_must_use_one_reached.md) |
| IX6 | `aliases` and `run` have no caller in any of the 33 crates; the crate is cited by name five times more widely than it is called | `ring_index` | n/a — coverage | [api/001](../api/001_three_functions_three_must_use_one_reached.md) |
| IX7 | `of` compiles as a `const fn` unchanged and const-evaluates; it is not declared one, in a tier where `ring_types` is 12 of 12 | `ring_index` | n/a — observation | [api/002](../api/002_the_three_signatures_and_the_const_they_are_not.md) |
| IX8 | `aliases` is kept from `const` only by comparing `SlotIndex` rather than calling `SlotIndex::get`, which `ring_types` publishes as `pub const fn` | `ring_index` | n/a — observation | [api/002](../api/002_the_three_signatures_and_the_const_they_are_not.md) |
| IX9 | Nothing takes a `SlotIndex` — the fold has no inverse and cannot have one, and the crate never says so | `ring_index` | n/a — doc gap | [api/002](../api/002_the_three_signatures_and_the_const_they_are_not.md) |
| IX10 | The fold reaches further than its two manifest edges, transitively through `Buffer::at`; `ring_store` holds the family's only written statement of the one-owner rule | `ring_store` | n/a — observation | [integration/001](../integration/001_two_dependents_and_a_third_that_did_it_again.md) |
| IX11 | The module comment says the fold sits on the claim path; none of the five claim/gate/consume crates depends on this one | `ring_index` | **wrong doc** | [integration/001](../integration/001_two_dependents_and_a_third_that_did_it_again.md) |
| IX12 | A second, character-identical fold at `ring_mpsc:543`, in a crate with eight `ring_*` dependencies and no edge to `ring_index` | `ring_mpsc` | **latent hazard** | [integration/001](../integration/001_two_dependents_and_a_third_that_did_it_again.md) |
| IX13 | Marked `present` by the 167-188 block flip rather than by any check of the three crates that implement it; the citation still runs one way only | `docs/feature` | **misleading doc** | [integration/002](../integration/002_the_feature_it_implements_half_of.md) |
| IX14 | The feature promises the masking claim will be measured; the family's 1098-line measurement crate compares whole write paths and never mentions mask, modulo, or `ring_index` | `ring_bench` | n/a — coverage | [integration/002](../integration/002_the_feature_it_implements_half_of.md) |
| IX15 | The "every claim and every read" error in the module comment is a faithful restatement of the feature's own line 12 | `docs/feature` | **wrong doc** | [integration/002](../integration/002_the_feature_it_implements_half_of.md) |
| IX16 | `run` inherits overflow behaviour from a bare `+` in `ring_types` and documents no `# Panics` | `ring_index` | n/a — doc gap | [pitfall/001](../pitfall/001_the_run_that_panics_in_debug_and_wraps_in_release.md) |
| IX17 | `Seq::next`'s comment says release builds saturate; they wrap, producing the exact monotonicity violation the same comment says the design avoids | `ring_types` | **wrong doc** | [pitfall/001](../pitfall/001_the_run_that_panics_in_debug_and_wraps_in_release.md) |
| IX18 | `UNSTAMPED` publishes `Seq( u64::MAX )`, making the overflow boundary reachable in one expression from a public constant | `ring_mpsc` | **latent hazard** | [pitfall/001](../pitfall/001_the_run_that_panics_in_debug_and_wraps_in_release.md) |
| IX19 | `stamp()` masks with `consumers`' capacity and indexes `stamps`, sized from an unstored constructor argument; the comment above it asserts the wrong guarantee | `ring_mpsc` | **latent hazard** | [pitfall/002](../pitfall/002_the_second_fold_nobody_noticed.md) |
| IX20 | All three `stamps().len()` assertions compare against hard-coded integers, never against `capacity()` — the field the fold actually uses | `ring_mpsc` | n/a — coverage | [pitfall/002](../pitfall/002_the_second_fold_nobody_noticed.md) |
| IX21 | The fold is ~7× cheaper than a runtime-divisor modulo, not the 22–44× implied by "20–40 cycles"; the cited architecture is not the one this builds on | `ring_index` | **misleading doc** | [non_functional_requirement/001](../non_functional_requirement/001_what_the_fold_costs.md) |
| IX22 | A constant-divisor `%` compiles to the same mask and costs 0.000 ns — the naive benchmark of this claim measures nothing, and the comment does not warn of it | `ring_index` | **measured cost** | [non_functional_requirement/001](../non_functional_requirement/001_what_the_fold_costs.md) |
| IX23 | `run` allocates once per call at exactly `8 × count` bytes, which is optimal for its return type and still rules it out of every path the crate exists to serve | `ring_index` | **measured cost** | [non_functional_requirement/002](../non_functional_requirement/002_the_one_allocation_and_the_zero_callers.md) |
| IX24 | The allocation is never paid: `run` has no caller, so it has never appeared in a profile | `ring_index` | n/a — observation | [non_functional_requirement/002](../non_functional_requirement/002_the_one_allocation_and_the_zero_callers.md) |
| IX25 | The constraint is enforced by rejection at one site and assisted nowhere; `next_power_of_two` appears in no crate, so rounding is every caller's problem | `ring_types` | n/a — observation | [decisions/001](../decisions/001_a_power_of_two_or_nothing.md) |
| IX26 | The stated justification is the cycle count; totality, the absence of error paths, and the aliasing relation all follow from the same decision and go unmentioned | `ring_index` | n/a — doc gap | [decisions/001](../decisions/001_a_power_of_two_or_nothing.md) |
| IX27 | `run`'s only real decision is its return type, and its doc comment discusses its inputs instead | `ring_index` | n/a — doc gap | [decisions/002](../decisions/002_a_vec_where_an_iterator_would_do.md) |
| IX28 | Random access and known length are both available without the `Vec`; the lazy signature is strictly more general, and changing it would break nothing outside this crate | `ring_index` | n/a — observation | [decisions/002](../decisions/002_a_vec_where_an_iterator_would_do.md) |
| IX29 | The identity is asserted exhaustively to `seq = 4095` and sampled to `u32::MAX`; the top 32 bits of the domain are measured by probe and asserted by nothing | `ring_index` | n/a — coverage | [invariant/001](../invariant/001_the_mask_equals_the_modulo.md) |
| IX30 | `seq.0 as usize` truncates on any target below 64 bits and provably cannot change the result, because the mask discards a superset of what the cast does — written down nowhere | `ring_index` | n/a — doc gap | [invariant/001](../invariant/001_the_mask_equals_the_modulo.md) |
| IX31 | The one totality claim is correctly scoped to `of` and sits where it reads as covering the crate; `run` is partial and carries no `# Panics` | `ring_index` | n/a — doc gap | [invariant/002](../invariant/002_the_fold_is_total_and_the_run_is_not.md) |
| IX32 | `run`'s real boundary is `start + ( count - 1 )` against `u64::MAX`, not `count` against `capacity` (nor the coarser `start + count`); the doc comment addresses the constraint that is not one | `ring_index` | n/a — doc gap | [invariant/002](../invariant/002_the_fold_is_total_and_the_run_is_not.md) |
| IX33 | `of`'s comment says "exactly one lap apart," one lap narrower than the property; the general form is stated correctly on `aliases`, which nothing calls | `ring_index` | **misleading doc** | [item/001](../item/001_the_fold_itself.md) |
| IX34 | Seventeen lines of code carry 100 lines of doc comment and 188 lines of test, so a wrong sentence is as costly here as a wrong expression | `ring_index` | n/a — observation | [item/001](../item/001_the_fold_itself.md) |
| IX35 | `aliases`' comment justifies its own zero callers coherently and its two-hop cross-crate reference resolves — the one in this crate that does — while no gate test uses it | `ring_index` | n/a — observation | [item/002](../item/002_the_two_that_nothing_calls.md) |
| IX36 | `run` cites this family's own batch-claim contract as its reason to exist; `ring_batch` implements that contract by importing `of` and writing `drain_order` instead | `ring_index` | n/a — drift | [item/002](../item/002_the_two_that_nothing_calls.md) |
| IX37 | Three borrowed types split two-transparent/one-opaque along the invariant boundary; `Capacity` alone withholds its field and alone declines `Default` | `ring_types` | n/a — observation | [type/001](../type/001_three_types_borrowed_none_owned.md) |
| IX38 | `Capacity::mask` is an unchecked `self.0 - 1`; privacy is the only guard, and it is sufficient — zero constructions outside `new` family-wide | `ring_types` | n/a — observation | [type/001](../type/001_three_types_borrowed_none_owned.md) |
| IX39 | `SlotIndex`'s "never constructed by counting" is contradicted by its own doctest and its `pub` field, yet 35 of 36 constructions are tests and the 36th is `of` | `ring_types` | n/a — unenforced | [type/002](../type/002_the_sentence_the_public_field_contradicts.md) |
| IX40 | One line converts between sequence space and slot space by unwrapping one public field and wrapping another; the newtypes' protection comes from that line being unique, not from their fields | `ring_index` | n/a — observation | [type/002](../type/002_the_sentence_the_public_field_contradicts.md) |
| IX41 | The one-owner pattern has no lint, test, or visibility restriction behind it; the census that detects a violation exists only in this corpus | `ring_index` | n/a — unenforced | [pattern/001](../pattern/001_one_owner_for_one_arithmetic_fact.md) |
| IX42 | "The fold travels with the container" predicts the one violation exactly: `ring_mpsc` wraps `slots` in a `Buffer` and holds `stamps` bare seven lines below | `ring_mpsc` | n/a — observation | [pattern/001](../pattern/001_one_owner_for_one_arithmetic_fact.md) |
| IX43 | `ring_index` and `ring_seqno` are the only crates with zero structs, traits, statics and zero occurrences of `mut`; eight total functions over `ring_types` values | `ring_index` | n/a — observation | [pattern/002](../pattern/002_stateless_arithmetic_over_borrowed_types.md) |
| IX44 | One of the eight allocates and it is the one with no callers; `ring_batch` declined the container rather than the arithmetic | `ring_index` | n/a — observation | [pattern/002](../pattern/002_stateless_arithmetic_over_borrowed_types.md) |
| IX45 | The whole crate exists because `Capacity` publishes `mask()` rather than the fold; every item `of`'s body touches is re-exported from `ring_types` two lines apart | `ring_types` | n/a — observation | [workaround/001](../workaround/001_the_mask_that_lives_one_crate_up.md) |
| IX46 | Keeping `mask()` public across a crate boundary is the door `ring_mpsc:543` walked through; making the fold an inherent method would make that line a compile error | `ring_types` | **latent hazard** | [workaround/001](../workaround/001_the_mask_that_lives_one_crate_up.md) |
| IX47 | `drain_order` is `run` with `.collect()` removed and the `Seq` paired in; the arithmetic is identical and nothing in `ring_index` records that its version was declined | `ring_batch` | n/a — duplication | [workaround/002](../workaround/002_the_iterator_ring_batch_built_instead.md) |
| IX48 | The workaround carries the only mechanical assertion of the one-owner rule in the family, and neither it nor `run` has a caller outside its own crate | `ring_batch` | n/a — coverage | [workaround/002](../workaround/002_the_iterator_ring_batch_built_instead.md) |
| IX49 | `Buffer` holds a flat `Box< [ S ] >` and a `Capacity` — no head, tail, or wrap marker; the ring topology is recomputed at every access and stored nowhere | `ring_store` | n/a — observation | [data_structure/001](../data_structure/001_the_ring_is_a_computation_not_a_layout.md) |
| IX50 | `SlotIndex` is never a struct field in any of the 33 crates; every one that exists is a temporary between `of` and a subscript | `ring_types` | n/a — observation | [data_structure/001](../data_structure/001_the_ring_is_a_computation_not_a_layout.md) |
| IX51 | `run`'s `Vec` is exactly sized because the body collects an `ExactSizeIterator`; the property is undocumented and no test would notice a rewrite that lost it | `ring_index` | n/a — doc gap | [data_structure/002](../data_structure/002_the_one_collection_the_crate_builds.md) |
| IX52 | A `Vec< SlotIndex >` is 24 bytes of header per 8 bytes of element; at the batch sizes the family actually uses, over a quarter of the structure is bookkeeping | `ring_index` | **measured cost** | [data_structure/002](../data_structure/002_the_one_collection_the_crate_builds.md) |
| IX53 | The crate sets the reuse cycle's period at `capacity` and participates in none of its six phases; the reuse phase is `of` called again, not a second behaviour | `ring_index` | n/a — observation | [lifecycle/001](../lifecycle/001_the_lap_is_the_only_cycle.md) |
| IX54 | `may_claim`'s strict `<` makes one lap the entire reachable aliasing state, so `of`'s narrow comment is exactly as wide as the gate — while `laps_between`'s doctest asserts a two-lap distance the gate prevents | `ring_seqno` | n/a — observation | [lifecycle/001](../lifecycle/001_the_lap_is_the_only_cycle.md) |
| IX55 | Zero `impl` blocks, constructors, or `Drop` impls; `of` is callable before any ring exists and after every ring is dropped, and nothing here can leak | `ring_index` | n/a — observation | [lifecycle/002](../lifecycle/002_no_initialization_and_no_teardown.md) |
| IX56 | Ten tests, two imports, no dev-dependencies, zero rings — the suite proves the arithmetic and proves nothing about whether the arithmetic is reached | `ring_index` | n/a — coverage | [lifecycle/002](../lifecycle/002_no_initialization_and_no_teardown.md) |

### Fifty-Six Findings About Seventeen Lines

The crate is three functions — one of which compiles to a single `and` — over
three types it does not define. It owns no state, no `impl` block, no trait, and
no allocation on any path anyone calls. The distribution:

| Subject of the finding | Count |
|------------------------|------:|
| `ring_index` itself | 34 |
| `ring_types`, the tier below | 8 |
| `ring_mpsc`, four tiers above, with no edge to this crate | 5 |
| the two dependents — `ring_batch` 3, `ring_store` 2 | 5 |
| this family's own feature record, the design document it was built from | 2 |
| `ring_seqno` 1, `ring_bench` 1 | 2 |

**Twenty-two of fifty-six are not about this crate**, and unlike a storage tier's
findings they are not all about neighbours. `ring_types` is one tier down and
`ring_batch`/`ring_store` are the two manifest dependents, but `ring_mpsc` is
four tiers up with no dependency edge here at all. It appears five times because
it wrote the fold a second time (IX12), because that copy masks with one capacity
while indexing an array sized by another (IX19), and because it is the crate that
reveals which rule the family actually follows (IX42). A crate whose job is a
claim about everyone else can only be checked by reading everyone else.

### The Shape Every Definition Found

Stated once because most of the thirteen definitions arrive at it independently:
**the seventeen lines of code are correct and the one hundred lines of comment
above them are not.** Nothing in `of`, `aliases`, or `run` computes a wrong
value — the identity is asserted exhaustively to `seq = 4095` and sampled to
`u32::MAX`. Every defect recorded here is a sentence.

| Where the gap is | Findings |
|------------------|----------|
| a doc sentence narrower, wider, or simply wrong about the code below it | IX3, IX11, IX15, IX17, IX21, IX31, IX32, IX33, IX39 |
| a true property with nothing stating it where a reader would need it | IX9, IX16, IX26, IX27, IX30, IX51 |
| a function nothing in the 33 crates calls | IX6, IX24, IX36, IX44, IX48 |
| a claim measured in prose and by no test | IX2, IX14, IX20, IX29, IX56 |
| a rule obeyed everywhere and enforced nowhere | IX41 |

The ratio is what makes this expensive: 100 lines of prose over 17 lines of code
(IX34). A reader who takes the module comment at face value will be wrong about
where the fold is called, wrong about how wide its aliasing relation is, and
wrong by a factor of three to six about what the modulo it replaces costs —
three separate sentences, all in the same 122-line file.

### Severity

| Tier | Meaning | Findings | Count |
|------|---------|----------|------:|
| **latent hazard** | well-typed code silently does the wrong thing | IX12, IX18, IX19, IX46 | 4 |
| **wrong doc** | a statement the code it describes contradicts | IX11, IX15, IX17 | 3 |
| **misleading doc** | a true statement a reader will generalize wrongly | IX3, IX21, IX33 | 3 |
| **measured cost** | a real, measured runtime cost nothing records | IX22, IX23, IX52 | 3 |
| n/a — doc gap | a true fact is unstated where it is needed | IX9, IX16, IX26, IX27, IX30, IX31, IX32, IX51 | 8 |
| n/a — coverage | the tests do not reach what they appear to | IX2, IX6, IX14, IX20, IX29, IX48, IX56 | 7 |
| n/a — drift | a document and its implementation have diverged | IX13, IX36 | 2 |
| n/a — unenforced | a real property with no lint, test, or check behind it | IX39, IX41 | 2 |
| n/a — duplication | one computation written twice, both correct | IX4, IX47 | 2 |
| n/a — observation | true, useful, and carrying no defect | the remaining 22 | 22 |

The first four tiers are the reachable ones — 13 of 56. Everything below them is
a record, not a problem.

**All four hazards are one hazard seen from four sides.** `ring_mpsc::stamp`
computes `( seq.0 as usize ) & self.capacity().mask()` and indexes `stamps` with
it (IX12) — but `capacity()` reads the `consumers` buffer's capacity, while
`stamps` was sized from a constructor argument the struct never stored, and the
comment directly beneath the line asserts the two agree (IX19). Nothing checks
that: all three tests touching `stamps().len()` compare against hard-coded
integers rather than against `capacity()`, so the one assertion that would catch
a divergence is the one nobody wrote (IX20). The line is reachable at all only
because `ring_types` publishes `mask()` across a crate boundary (IX46) — moving
the fold onto `Capacity` as an inherent method and making `mask` private turns
line 543 into a compile error and deletes this crate in the same edit. The
fourth is the overflow end of the same arithmetic: `UNSTAMPED` is a public
`Seq( u64::MAX )`, which puts `run`'s inherited, undocumented overflow one
expression away from any caller (IX18).

The three **wrong doc** findings share one origin. This family's own cited feature record says the
fold sits on "every claim and every read"; the module comment restates that line
almost verbatim (IX15); and it is false, because none of the five claim, gate, or
consume crates depends on `ring_index` at all (IX11). The third is a tier below
and independent: `Seq::next` says release builds saturate where Rust's `+` wraps,
and names as avoided the exact monotonicity violation that wrapping produces
(IX17).

The three **measured costs** are separated from the hazards deliberately, and two
of the three are costs nobody pays. `run` allocates `8 × count` bytes per call
and has no caller in any of the 33 crates (IX23), in a container whose 24-byte
header outweighs a quarter of its payload at the batch sizes the family actually
uses (IX52) — which is precisely why `ring_batch` wrote a lazy iterator instead.
The third is the one with consequences: the module comment justifies the whole
crate with "20–40 cycles on current x86," and on this host the fold is ~7×
cheaper than a runtime-divisor modulo rather than 22–44× (IX21), while against a
*constant* divisor the modulo compiles to the same mask and costs nothing at all
(IX22). The obvious benchmark of the crate's own reason to exist reports no
difference, and nothing warns the person about to write it.
