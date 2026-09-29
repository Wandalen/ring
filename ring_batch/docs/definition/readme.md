# Doc Definitions

Module Index for `ring_batch` — every doc definition this crate declares and every
instance under each, in one place.

## Master Doc Definitions Table

| Type | Purpose | Master File | Instances |
|------|---------|-------------|----------:|
| `algorithm/` | One `fetch_add` at any batch size, and a gate that is three steps and not part of it | [algorithm/readme.md](../algorithm/readme.md) | 2 |
| `api/` | Twelve items, seven attributes, and the one bare statement that burns sequences | [api/readme.md](../api/readme.md) | 2 |
| `data_structure/` | Sixteen bytes with no niche, and the same struct written again three tiers up | [data_structure/readme.md](../data_structure/readme.md) | 2 |
| `decisions/` | An ordering parameter with two hard-coded exceptions, and an error split with a test behind it | [decisions/readme.md](../decisions/readme.md) | 2 |
| `definition/` | Module Index — every definition and every instance in this crate, in one place | this file | — |
| `integration/` | Four dependencies with two written down, one dependant, and a citation path all 33 crates get wrong | [integration/readme.md](../integration/readme.md) | 2 |
| `invariant/` | Disjointness that cannot break, and ordering that is ascending but not contiguous | [invariant/readme.md](../invariant/readme.md) | 2 |
| `item/` | The method whose stated reason its own test declined, and the crate's only addition | [item/readme.md](../item/readme.md) | 2 |
| `lifecycle/` | No destructor, no rollback, and the empty claim as a legal state at every entry point | [lifecycle/readme.md](../lifecycle/readme.md) | 2 |
| `non_functional_requirement/` | What a batch buys at two thread counts, and the heap the crate never touches | [non_functional_requirement/readme.md](../non_functional_requirement/readme.md) | 2 |
| `pattern/` | The range object written four times, and a split into two functions with no caller for one | [pattern/readme.md](../pattern/readme.md) | 2 |
| `pitfall/` | A cursor that outruns the ring, and an addition with no `# Panics` section | [pitfall/readme.md](../pitfall/readme.md) | 2 |
| `type/` | Two cursor parameters that can be the same cell, and an iterator nobody can name | [type/readme.md](../type/readme.md) | 2 |
| `workaround/` | A capture bound edition 2024 made necessary, and three casts of which one crosses nothing | [workaround/readme.md](../workaround/readme.md) | 2 |

13 definitions, 26 instances.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_batch/docs
find . -mindepth 1 -maxdepth 1 -type d ! -name definition | wc -l   # definitions
find . -name '0*.md' | wc -l                                        # instances
grep -rho '^### BA[0-9]*' . | wc -l                                 # findings
```

Live output:

```
13
26
53
```

## Master Doc Instances Table

| Type | ID | Title | Subject |
|------|----|-------|---------|
| `algorithm/` | 001 | [One `fetch_add`, Whatever the Count](../algorithm/001_one_fetch_add_whatever_the_count.md) | `claim`'s single instruction, and the measured cost curve across four batch sizes |
| `algorithm/` | 002 | [Check, Then Advance](../algorithm/002_check_then_advance.md) | `claim_gated`'s three steps, the test that pins their order, and a cast the lint gate cannot see |
| `api/` | 001 | [Twelve Items, Seven `must_use`](../api/001_twelve_items_seven_must_use.md) | The surface with its attributes, and the one call whose dropped result burns sequences |
| `api/` | 002 | [Two Claim Functions, One Caller](../api/002_two_claim_functions_one_caller.md) | One manifest edge, one import line, and a design decision that travelled without one |
| `data_structure/` | 001 | [Sixteen Bytes That Are Not a Buffer](../data_structure/001_sixteen_bytes_that_are_not_a_buffer.md) | The layout, the missing niche, and the storage dependency the crate declines |
| `data_structure/` | 002 | [The Struct `ring_claim` Wrote Again](../data_structure/002_the_struct_ring_claim_wrote_again.md) | The same structure defined twice, and which of the two the family reaches |
| `decisions/` | 001 | [Ordering Is the Caller's, Except Where It Isn't](../decisions/001_ordering_is_the_callers_except_where_it_is_not.md) | The `order` parameter's scope, the two hard-coded `Acquire` literals, and the missing `loom` seam |
| `decisions/` | 002 | [Two Errors, Not One](../decisions/002_two_errors_not_one.md) | `BatchTooLarge` against `Full`, the classification a caller branches on, and what `Full` does not carry |
| `integration/` | 001 | [Four Edges In, Two Written Down](../integration/001_four_edges_in_two_written_down.md) | The manifest against the module comment against the readme, and the citation path all 33 crates get wrong |
| `integration/` | 002 | [The Feature Is Planned, Its Problems Are Addressed](../integration/002_the_feature_is_planned_its_problems_are_addressed.md) | The single dependent, the design corpus above it, and the unmeasured headline claim |
| `invariant/` | 001 | [Disjointness Is Free](../invariant/001_disjointness_is_free.md) | Why no two claims overlap through either entry point, and why that makes the test unable to fail |
| `invariant/` | 002 | [Ascending, Not Contiguous](../invariant/002_ascending_not_contiguous.md) | Per-thread order, the gap distribution the `<=` tolerates, and the bound nothing asserts |
| `item/` | 001 | [The Method Whose Reason Was Declined](../item/001_the_method_whose_reason_was_declined.md) | `overlaps`: its stated purpose, the test that passed it over, and its load-bearing empty guard |
| `item/` | 002 | [One Past the End](../item/002_one_past_the_end.md) | `end()`: the crate's only addition, its four consumers, and the clause that holds only under one producer |
| `lifecycle/` | 001 | [No Lifecycle and No Rollback](../lifecycle/001_no_lifecycle_and_no_rollback.md) | The missing destructor, the four that exist elsewhere, and where the rule is actually written |
| `lifecycle/` | 002 | [The Empty Claim as a First-Class State](../lifecycle/002_the_empty_claim_as_a_first_class_state.md) | Zero at every entry point, the one guard that had to be hand-written, and what an empty flush costs |
| `non_functional_requirement/` | 001 | [What One Batch Actually Buys](../non_functional_requirement/001_what_one_batch_actually_buys.md) | The cost curve at one and eight threads, and the two batch sizes that buy nothing |
| `non_functional_requirement/` | 002 | [Sixteen Bytes and No Allocation](../non_functional_requirement/002_sixteen_bytes_and_no_allocation.md) | The heap the crate never touches, and the family's allocating version of its fold |
| `pattern/` | 001 | [The Range Object](../pattern/001_the_range_object.md) | Four types, two shapes, an identical eight-method surface, and one attribute that differs |
| `pattern/` | 002 | [Two Functions Where One Would Have Hidden It](../pattern/002_two_functions_where_one_would_have_hidden_it.md) | The gated/ungated split, its predicted callers, and the struct form one tier up |
| `pitfall/` | 001 | [The Window Between the Gate and the Advance](../pitfall/001_the_window_between_the_gate_and_the_advance.md) | The overrun rates under contention, and the CAS loop three tiers up that names the failure mode |
| `pitfall/` | 002 | [The Addition With No Panics Section](../pitfall/002_the_addition_with_no_panics_section.md) | Debug panic, release wrap, and the `ring_types` doc that claimed saturation until it was corrected |
| `type/` | 001 | [The Ring That Can Gate Against Itself](../type/001_the_ring_that_can_gate_against_itself.md) | Two unrelated cursor parameters, the aliased call, and how the rest of the family avoided it |
| `type/` | 002 | [The Iterator Nobody Can Name](../type/002_the_iterator_nobody_can_name.md) | What `impl Iterator` erases, what survives it, and what the one dependant paid |
| `workaround/` | 001 | [The Capture Bound Edition 2024 Made Necessary](../workaround/001_the_capture_bound_edition_2024_made_necessary.md) | `+ use< >` proven load-bearing at both sites, edition-specific, and removable by a by-value receiver |
| `workaround/` | 002 | [The `usize`/`u64` Seam](../workaround/002_the_usize_u64_seam.md) | Three casts, two lossless widenings, and one that casts `usize` to `usize` |

## Findings

Fifty-three, each verified by a command whose output is quoted in its instance.

| ID | Finding | Subject | Reachable | Where |
|----|---------|---------|-----------|-------|
| BA1 | The amortisation is `fetch_add` returning the old value: publishing the claim and advancing the cursor are one instruction, which is why nothing can be unclaimed | `ring_batch` | n/a — observation | [algorithm/001](../algorithm/001_one_fetch_add_whatever_the_count.md) |
| BA2 | A claim costs 6.4 ns at every size from 1 to 1024; this crate's own contract states "roughly what a single item costs" and omits the 115× and 1841× ratios a batch-size/tail-latency tradeoff needs | `docs/feature` | n/a — doc gap | [algorithm/001](../algorithm/001_one_fetch_add_whatever_the_count.md) |
| BA3 | The size check precedes the cursor reads, and the test pins that by asserting both cells were never touched — the crate's only body-ordering decision with a test rather than a comment behind it | `ring_batch` | n/a — observation | [algorithm/002](../algorithm/002_check_then_advance.md) |
| BA4 | `free_slots( .. ) as usize` casts `usize` to `usize`; `clippy::unnecessary_cast` warns on all three same-crate forms of it and is silent across a crate boundary, so `-D warnings` cannot see it | `ring_batch` | n/a — observation | [algorithm/002](../algorithm/002_check_then_advance.md) |
| BA5 | `#[ must_use ]` is on all seven pure `const` accessors, where dropping the result does nothing, and on none of the three free functions | `ring_batch` | n/a — observation | [api/001](../api/001_twelve_items_seven_must_use.md) |
| BA6 | `claim( &cursor, 8, order );` as a bare statement compiles clean under `-D warnings`, advances the cursor, and orphans eight sequences no consumer will ever be released from | `ring_batch` | **latent hazard** | [api/001](../api/001_twelve_items_seven_must_use.md) |
| BA7 | One manifest dependent reaching four of twelve items; `claim_gated` and `drain_order` — half of this crate's own stated deliverable — have no caller in the 33 crates | `ring_batch` | n/a — coverage | [api/002](../api/002_two_claim_functions_one_caller.md) |
| BA8 | The only cross-crate mention of `claim_gated` restates its ordering justification almost verbatim, between two crates with no dependency edge in either direction | `ring_cursor` | n/a — observation | [api/002](../api/002_two_claim_functions_one_caller.md) |
| BA9 | 16 bytes, 8-aligned, `Copy`, no niche — `Option< BatchClaim >` costs 24, because the empty claim the crate depends on uses the encoding a niche would need | `ring_batch` | n/a — observation | [data_structure/001](../data_structure/001_sixteen_bytes_that_are_not_a_buffer.md) |
| BA10 | The crate takes no storage dependency, says so in the module comment, and the manifest agrees; the cost is that a claim folded against a foreign capacity is undetectable | `ring_batch` | n/a — observation | [data_structure/001](../data_structure/001_sixteen_bytes_that_are_not_a_buffer.md) |
| BA11 | `Claim` and `BatchClaim` differ in one field name; their `sequences` bodies are character-identical and all eight shared operations return the same results | `ring_claim` | n/a — duplication | [data_structure/002](../data_structure/002_the_struct_ring_claim_wrote_again.md) |
| BA12 | The Tier 5 duplicate is the one with two dependents and both ring implementations behind it; the Tier 2 original reaches one leaf crate | `ring_claim` | n/a — observation | [data_structure/002](../data_structure/002_the_struct_ring_claim_wrote_again.md) |
| BA13 | Nine lines justify the crate's only two `Ordering` literals; the decision is about the parameter's *scope*, and the principle travelled to `ring_cursor` across a boundary with no dependency edge | `ring_batch` | n/a — observation | [decisions/001](../decisions/001_ordering_is_the_callers_except_where_it_is_not.md) |
| BA14 | The crate carries no `loom` in its manifest while five others do, so its ordering argument has no falsifier — a patched copy with both gating loads `Relaxed` passes all 21 integration tests and 10 doctests | `ring_batch` | n/a — coverage | [decisions/001](../decisions/001_ordering_is_the_callers_except_where_it_is_not.md) |
| BA15 | The error split's test asserts `is_configuration()` and `!is_transient()` rather than the variant, pinning the property the split exists for; it is the crate's only test of that shape | `ring_batch` | n/a — observation | [decisions/002](../decisions/002_two_errors_not_one.md) |
| BA16 | `Full` is a unit variant carrying no free count and no cursor position, so a caller cannot trim its request; `ring_claim` answered that with `claim_up_to` and `ring_batch` has no counterpart | `ring_claim` | n/a — observation | [decisions/002](../decisions/002_two_errors_not_one.md) |
| BA17 | Eleven of the 29 crates with a `Depends on` sentence understated it when filed; `ring_batch`, `ring_mpsc` and `ring_publish`'s module comments have since been corrected to name every dependency — `ring_batch`'s readme has not, and still omits two | `` | **wrong doc** | [integration/001](../integration/001_four_edges_in_two_written_down.md) |
| BA18 | All 33 source files cite a path ending in `.md` for their family's own design corpus, which does not exist; all 33 readmes cite the directory that does | `` | **wrong doc** | [integration/001](../integration/001_four_edges_in_two_written_down.md) |
| BA19 | The undeclared fourth dependency has one call site, inside `drain_order`, the function no crate in the family calls | `ring_batch` | n/a — observation | [integration/001](../integration/001_four_edges_in_two_written_down.md) |
| BA20 | All 16 features cited by the family read `present`, flipped as the contiguous 167-188 block rather than per feature — the field records that a contiguous range was declared done, not that any one feature under it was built | `docs/feature` | **misleading doc** | [integration/002](../integration/002_the_feature_is_planned_its_problems_are_addressed.md) |
| BA21 | This crate's own stated claim is a number, `ring_bench` exists to produce numbers, and no file under `ring_bench/` mentions `ring_batch` | `ring_bench` | n/a — coverage | [integration/002](../integration/002_the_feature_is_planned_its_problems_are_addressed.md) |
| BA22 | Disjointness comes from `fetch_add` alone and survives the gated path unchanged — 16000 sequences per run, zero claimed twice, through both entry points over 20 runs each | `ring_batch` | n/a — observation | [invariant/001](../invariant/001_disjointness_is_free.md) |
| BA23 | The disjointness test calls its property "the property a claim protocol must never violate"; it is the property this protocol cannot violate, so both of its witnesses are blind to the crate's actual failure | `ring_batch` | **misleading doc** | [invariant/001](../invariant/001_disjointness_is_free.md) |
| BA24 | The ordering assertion's `<=` is load-bearing: only 39,469 of 159,680 adjacent pairs were contiguous, and the widest gap between one thread's own claims was 680 sequences | `ring_batch` | n/a — observation | [invariant/002](../invariant/002_ascending_not_contiguous.md) |
| BA25 | The capacity bound — the only invariant that can actually break — is asserted by no test; both threaded tests call `claim`, and all 13 `claim_gated` references sit above the contention section | `ring_batch` | n/a — coverage | [invariant/002](../invariant/002_ascending_not_contiguous.md) |
| BA26 | `overlaps` is documented as existing so a whole-run test can assert disjointness; that test exists, names the same property, and states it uses a per-sequence `HashSet` instead because it is stronger | `ring_batch` | **misleading doc** | [item/001](../item/001_the_method_whose_reason_was_declined.md) |
| BA27 | Nine call sites, more than any other method, all inside `overlaps`' own two tests and none anywhere else in the family; the `!is_empty()` guard those tests cover is load-bearing where `contains`' absence of one is correct | `ring_batch` | n/a — observation | [item/001](../item/001_the_method_whose_reason_was_declined.md) |
| BA28 | `end()`'s "the value the cell now holds" held on all 60,000 claims at one and two threads and failed 2,840 times out of 560,000 at four and above, with the cell up to 596,104 sequences ahead | `ring_batch` | **misleading doc** | [item/002](../item/002_one_past_the_end.md) |
| BA29 | `end()`'s `+` is the only arithmetic operator in the crate body, and three of the eight methods plus the ordering assertion route through it | `ring_batch` | n/a — observation | [item/002](../item/002_one_past_the_end.md) |
| BA30 | A claim is irreversible the instant it is returned, and the crate never says so; the rule appears only in `ring_tls::flush_into`'s doc, as a justification for something else | `ring_batch` | n/a — doc gap | [lifecycle/001](../lifecycle/001_no_lifecycle_and_no_rollback.md) |
| BA31 | Four range types across four crates — two with a committing `Drop`, two without — and nothing records that a range with no ring reference cannot have one | `ring_mpsc` | n/a — duplication | [lifecycle/001](../lifecycle/001_no_lifecycle_and_no_rollback.md) |
| BA32 | Zero is legal at both entry points and correct across all eight methods; five behaviours fall out of the arithmetic and only `overlaps` needed a hand-written guard | `ring_batch` | n/a — observation | [lifecycle/002](../lifecycle/002_the_empty_claim_as_a_first_class_state.md) |
| BA33 | One thread flushing an empty buffer in a loop doubles a working producer's per-claim cost (22.24 ns against 11.36 ns); the doc warns of "one atomic" and does not say whose | `ring_tls` | **measured cost** | [lifecycle/002](../lifecycle/002_the_empty_claim_as_a_first_class_state.md) |
| BA34 | This crate's own stated claim holds and understates: a batch of sixty-four costs 11.49 ns against a single item's 5.93 ns on one thread, and at eight threads is cheaper than a single-item claim (63.59 ns against 80.92 ns) | `ring_batch` | n/a — observation | [non_functional_requirement/001](../non_functional_requirement/001_what_one_batch_actually_buys.md) |
| BA35 | A batch of one is exactly break-even at both thread counts (1.0×), so a caller that flushes per push pays the whole machinery for none of the amortisation; the crossover is unstated and unchecked | `ring_tls` | **measured cost** | [non_functional_requirement/001](../non_functional_requirement/001_what_one_batch_actually_buys.md) |
| BA36 | The crate allocates nothing and nothing holds it there — three of 33 crates are `no_std` and this is not one of them, the sole crate attribute is `deny( missing_docs )`, and one `collect()` would compile and pass | `ring_batch` | n/a — unenforced | [non_functional_requirement/002](../non_functional_requirement/002_sixteen_bytes_and_no_allocation.md) |
| BA37 | `ring_index::run` is the same fold returning a `Vec`, 9.6× slower at one item and 1.7× at sixty-four, with no caller outside its own tests; neither crate mentions the other's version | `ring_index` | n/a — duplication | [non_functional_requirement/002](../non_functional_requirement/002_sixteen_bytes_and_no_allocation.md) |
| BA38 | `BatchClaim` and `ring_claim::Claim` expose eight identically-named methods on two identically-shaped fields, three tiers apart, with no dependency edge and no mention of each other | `ring_claim` | n/a — duplication | [pattern/001](../pattern/001_the_range_object.md) |
| BA39 | `ring_claim::Claim` carries a type-level `#[ must_use ]` whose message states the irreversibility rule `ring_batch` never writes; the same attribute would also close the crate's one silent-drop gap | `ring_batch` | n/a — doc gap | [pattern/001](../pattern/001_the_range_object.md) |
| BA40 | The split's stated reason names the SPSC and MPSC paths; neither depends on this crate, and the one that does imports only the ungated half | `ring_batch` | n/a — observation | [pattern/002](../pattern/002_two_functions_where_one_would_have_hidden_it.md) |
| BA41 | The same gate is a free function here and a struct one tier up; the struct owns its producer cursor, which makes the same-cell-twice mistake unrepresentable | `ring_claim` | n/a — observation | [pattern/002](../pattern/002_two_functions_where_one_would_have_hidden_it.md) |
| BA42 | The gate and the advance are separate operations, so the cursor passes `consumer + capacity` in roughly one round in two hundred at 16 producers, by a whole batch each time; `claim`'s doc offers the gated form as the one that does not outrun the ring | `ring_batch` | **latent hazard** | [pitfall/001](../pitfall/001_the_window_between_the_gate_and_the_advance.md) |
| BA43 | `Claimer::claim` closes the identical window with a `compare_exchange` retry loop and a four-line comment naming the failure mode; neither crate references the other | `ring_claim` | n/a — duplication | [pitfall/001](../pitfall/001_the_window_between_the_gate_and_the_advance.md) |
| BA44 | `end()` panics in debug at `lib.rs:113:10` and wraps in release, after which `len()` says 1, `is_empty()` says false, and every method routing through `end()` behaves as though the claim were empty; no `# Panics` section exists in the crate | `ring_batch` | **latent hazard** | [pitfall/002](../pitfall/002_the_addition_with_no_panics_section.md) |
| BA45 | `Seq::next`'s doc said the addition "saturates in a release build … deliberately not wrapping"; `next` and `advanced_by` are plain `+`, which wraps, and `distance_to` in the same `impl` block uses a real `saturating_sub`. **Since corrected** in `ring_types` — both paragraphs rewritten, and `advanced_by` given the overflow note it never had | `ring_types` | **wrong doc** | [pitfall/002](../pitfall/002_the_addition_with_no_panics_section.md) |
| BA46 | `claim_gated( &cell, &cell, .. )` type-checks and grants every request — six of six on a ring of four — because `free_slots( at, at, cap )` is always the full capacity | `ring_batch` | **latent hazard** | [type/001](../type/001_the_ring_that_can_gate_against_itself.md) |
| BA47 | The only function in 33 crates that can be handed one ring end twice; every other holds both ends through a single `Ring< S >` or `GatingSet`, and `ring_debug`'s two concrete types make the aliased call impossible | `ring_batch` | n/a — observation | [type/001](../type/001_the_ring_that_can_gate_against_itself.md) |
| BA48 | `impl Iterator` costs exactly `DoubleEndedIterator`; `size_hint` survives exact, and `ExactSizeIterator::len` was never available because `Range< u64 >` does not implement it | `ring_batch` | n/a — observation | [type/002](../type/002_the_iterator_nobody_can_name.md) |
| BA49 | The one dependant stores a `BatchClaim` beside a redundant `next : u64` and hand-writes `Iterator`, because the opaque return cannot be a struct field; it carries the family's only `ExactSizeIterator` impl | `ring_tls` | n/a — duplication | [type/002](../type/002_the_iterator_nobody_can_name.md) |
| BA50 | `+ use< >` is edition-2024-specific and load-bearing at both sites — the same source and caller pass under 2021 — yet carries no comment at any of the family's three occurrences, and no caller needing it exists | `ring_batch` | n/a — doc gap | [workaround/001](../workaround/001_the_capture_bound_edition_2024_made_necessary.md) |
| BA51 | Both bounds exist because a sixteen-byte `Copy` type is passed by reference; taking it by value removes both, which is what `ring_claim::Claim::sequences( self )` already does | `ring_batch` | n/a — observation | [workaround/001](../workaround/001_the_capture_bound_edition_2024_made_necessary.md) |
| BA52 | The crate only ever widens `usize` → `u64`, which is why it has no fallible conversion and why `claim` can return a value rather than a `Result` | `ring_batch` | n/a — observation | [workaround/002](../workaround/002_the_usize_u64_seam.md) |
| BA53 | `free_slots( .. ) as usize` casts a `usize` to `usize`; it compiles away, clippy stays silent even with `unnecessary_cast` and `cast_lossless` forced on, and the parentheses make it read as a deliberate width fix | `ring_batch` | n/a — observation | [workaround/002](../workaround/002_the_usize_u64_seam.md) |

### Fifty-Three Findings About One Struct and Three Functions

The crate is 314 lines: one sixteen-byte struct with eight `const` methods, three
free functions, one `+`, one `fetch_add`, one comparison. It owns no storage, no
`Drop`, no `unsafe`, no allocation, and no state at all. The distribution:

| Subject of the finding | Count |
|------------------------|------:|
| `ring_batch` itself | 35 |
| `ring_claim`, three tiers up, with no edge in either direction | 6 |
| `ring_tls`, the one crate that depends on it | 3 |
| every `src/lib.rs` in the family at once | 2 |
| the design corpus — a single feature record 1, `docs/feature` as a collection 1 | 2 |
| `ring_types` 1, `ring_index` 1, `ring_mpsc` 1, `ring_cursor` 1, `ring_bench` 1 | 5 |

**Eighteen of fifty-three are not about this crate**, and six of those eighteen
are about one crate three tiers above it that this one has never heard of.

### The Shape Every Definition Found

Stated once because eight of the thirteen definitions arrive at it independently:
**the corrected version of nearly everything in this crate already exists in
`ring_claim`, and neither crate mentions the other.** Not a shared abstraction, not
a `use` — no dependency edge exists in either direction, and no prose in either
crate names the other.

| What `ring_batch` has | What `ring_claim` already had | Finding |
|-----------------------|-------------------------------|---------|
| `BatchClaim { start, count }` | `Claim { start, len }`, character-identical `sequences` body, all eight operations agreeing | BA11, BA38 |
| A `claim` whose dropped result silently burns sequences | A type-level `#[ must_use ]` whose message states the rule `ring_batch` never writes down | BA6, BA39 |
| A `Full` variant carrying no free count | `claim_up_to`, so a refused caller can trim rather than fail | BA16 |
| Two loose cursor parameters that can be the same cell | `Claimer`, which owns its producer cursor and makes the mistake unrepresentable | BA41, BA46, BA47 |
| A gate and an advance as separate operations, overrunning the ring | A `compare_exchange` retry loop, with a four-line comment naming that exact failure | BA42, BA43 |
| `sequences( &self )`, needing an edition-2024 capture bound | `sequences( self )`, needing none | BA50, BA51 |

The tier ordering is what makes this worth recording rather than merely noting.
`ring_batch` is Tier 2 and `ring_claim` is Tier 5, so the lower, more primitive
crate is the one missing the guards — and the Tier 5 duplicate is the one with two
dependents and both ring implementations behind it, while the Tier 2 original
reaches exactly one leaf crate (BA12). The primitive is not the thing being built
on; it is the thing that got written first and then re-derived, better, higher up.

The remaining gaps fall into four groups:

| Where the gap is | Findings |
|------------------|----------|
| a doc sentence the code contradicts, or one a reader will generalize wrongly | BA17, BA18, BA23, BA26, BA28, BA45 |
| a true fact with nothing stating it where a reader would need it | BA2, BA30, BA39, BA50 |
| one computation written twice, every copy correct | BA11, BA31, BA37, BA38, BA43, BA49 |
| a claim asserted in prose and by no test | BA7, BA14, BA21, BA25 |
| a real property with no lint, test, or check behind it | BA36 |

### Severity

| Tier | Meaning | Findings | Count |
|------|---------|----------|------:|
| **latent hazard** | well-typed code silently does the wrong thing | BA6, BA42, BA44, BA46 | 4 |
| **wrong doc** | a statement the code it describes contradicts | BA17, BA18, BA45 | 3 |
| **misleading doc** | a true statement a reader will generalize wrongly | BA23, BA26, BA28 | 3 |
| **measured cost** | a real, measured runtime cost nothing records | BA33, BA35 | 2 |
| n/a — duplication | one computation written twice, both correct | BA11, BA31, BA37, BA38, BA43, BA49 | 6 |
| n/a — doc gap | a true fact is unstated where it is needed | BA2, BA30, BA39, BA50 | 4 |
| n/a — coverage | the tests do not reach what they appear to | BA7, BA14, BA21, BA25 | 4 |
| n/a — drift | a document and its implementation have diverged | BA20 | 1 |
| n/a — unenforced | a real property with no lint, test, or check behind it | BA36 | 1 |
| n/a — observation | true, useful, and carrying no defect | the remaining 25 | 25 |

The first four tiers are the reachable ones — 12 of 53. Everything below them is
a record, not a problem.

**All four hazards compile clean under `-D warnings`, and three of the four are
one API shape away from impossible.** `claim( &cursor, 8, order );` as a bare
statement type-checks, advances the cursor, and orphans eight sequences (BA6) —
a type-level `#[ must_use ]`, which the twin already carries, makes it a warning.
`claim_gated( &cell, &cell, .. )` type-checks and grants six of six requests on a
ring of four (BA46) — one owned producer cursor, which the twin already has, makes
the call unwriteable. The gate and the advance run as separate operations, so the
cursor passes `consumer + capacity` in about one round in two hundred at sixteen
producers (BA42) — a `compare_exchange` retry loop, which the twin already runs,
closes the window. The fourth is arithmetic rather than shape: `end()`'s `+`
panics in debug and wraps in release, after which `len()` reports 1 while every
method routing through `end()` behaves as though the claim were empty, and when
BA44 was filed the crate had no `# Panics` section anywhere. `end()` now carries
one naming both halves; the arithmetic is unchanged and untested at the boundary.

The three **wrong doc** findings split two-and-one. Two are family-wide rather
than local — eleven of twenty-nine `Depends on` sentences understate their
manifests (BA17), and all thirty-three source files cite a spec path that is a
file where the real one is a directory (BA18), which the thirty-three readmes get
right. The third is one tier down and independent: `Seq::next` said its addition
"saturates in a release build … deliberately not wrapping" where the operator is a
plain `+`, in the same `impl` block as a `distance_to` that uses a real
`saturating_sub` (BA45). That sentence is what turned BA44 from an overflow into a
surprise, and it has since been rewritten to say the addition wraps — with the
584-year figure kept as the reason that is survivable rather than as a claim about
the arithmetic. `advanced_by`, the second body the sentence covered, had no
overflow note at all until one was written; it now records that the reachability
argument does not carry over, because `n` is the caller's.

The two **measured costs** are both `ring_tls`', both about the crossover this
crate never states. A batch of one is exactly break-even at one and eight threads,
so a caller flushing per push pays the whole machinery for none of the
amortisation (BA35), and `flush_into` checks neither end of the range. One thread
flushing an empty buffer in a loop doubles a *working* producer's per-claim cost —
22.24 ns against 11.36 ns — while the doc warns of "one atomic" without saying
whose (BA33).
