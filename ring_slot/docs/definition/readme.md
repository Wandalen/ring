# Doc Definitions

Module Index for `ring_slot` — every doc definition this crate declares and every
instance under each, in one place.

## Master Doc Definitions Table

| Type | Purpose | Master File | Instances |
|------|---------|-------------|----------:|
| `algorithm/` | Two irreducible computations, one branch in the whole crate, and a fold whose name collides one tier up | [algorithm/readme.md](../algorithm/readme.md) | 2 |
| `api/` | Ten functions with two attribute sets that do not overlap, and a trait stopping before the payload | [api/readme.md](../api/readme.md) | 2 |
| `data_structure/` | A newtype that costs nothing and a length field that costs more than the payload it measures | [data_structure/readme.md](../data_structure/readme.md) | 2 |
| `decisions/` | Two rulings — one enforced harder than it was asked for, one argued without measurement | [decisions/readme.md](../decisions/readme.md) | 2 |
| `definition/` | Module Index — every definition and every instance in this crate, in one place | this file | — |
| `integration/` | Seven dependents, genericity holding across three layers, and the layer that drops it | [integration/readme.md](../integration/readme.md) | 2 |
| `invariant/` | Two properties — one exhausted over its whole domain, one unrepresentable to break | [invariant/readme.md](../invariant/readme.md) | 2 |
| `item/` | Ten functions one at a time, and the two absences that determine an API two crates up | [item/readme.md](../item/readme.md) | 2 |
| `lifecycle/` | A slot across one publish and across a ring's whole life, and what `clear` leaves behind | [lifecycle/readme.md](../lifecycle/readme.md) | 2 |
| `non_functional_requirement/` | Six absences holding a cost budget up, and a clone proportional to capacity rather than payload | [non_functional_requirement/readme.md](../non_functional_requirement/readme.md) | 2 |
| `pattern/` | One trait with two implementors, and a returned value four crates read four ways | [pattern/readme.md](../pattern/readme.md) | 2 |
| `pitfall/` | A test that passes for the wrong reason, and residue that outlives the publish that wrote it | [pitfall/readme.md](../pitfall/readme.md) | 2 |
| `type/` | A `Copy` withheld twice over, and a width that is part of the type rather than part of the value | [type/readme.md](../type/readme.md) | 2 |
| `workaround/` | Two external constraints absorbed correctly and recorded nowhere | [workaround/readme.md](../workaround/readme.md) | 2 |

13 definitions, 26 instances.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_slot/docs
find . -mindepth 1 -maxdepth 1 -type d ! -name definition | wc -l   # 13
find . -name '0*.md' | wc -l                                        # 26
grep -rho '^### SL[0-9]*' . | wc -l                                 # 52
```

## Master Doc Instances Table

| Type | ID | Title | Subject |
|------|----|-------|---------|
| `algorithm/` | 001 | [Write Is a Bound Check and a Copy](../algorithm/001_write_is_a_bound_check_and_a_copy.md) | The ordering that makes failure total, and the absence of every control-flow construct |
| `algorithm/` | 002 | [Emptiness Three Ways](../algorithm/002_emptiness_three_ways.md) | Two irreducible computations, and a fold whose name collides in the crate above |
| `api/` | 001 | [Ten Functions, Six `const`, Seven `must_use`](../api/001_ten_functions_six_const_seven_must_use.md) | An annotation set inverted relative to consequence, and a justification pointing at a policy the family refuses |
| `api/` | 002 | [A Trait With Two Methods](../api/002_a_trait_with_two_methods.md) | A boundary drawn before the payload, and the `Default` every consumer adds back |
| `data_structure/` | 001 | [Sixteen Bytes to Carry Eight](../data_structure/001_sixteen_bytes_to_carry_eight.md) | A fixed word of overhead at every `N`, and why a narrower field was not chosen |
| `data_structure/` | 002 | [The Niche `Option` Finds and the Array Does Not](../data_structure/002_the_niche_option_finds_and_the_array_does_not.md) | A free newtype, and a wrapping cost that compounds with the length field |
| `decisions/` | 001 | [Two Shapes Rather Than One](../decisions/001_two_shapes_rather_than_one.md) | A trait where a convention would have sufficed, and a founding argument with no measurement |
| `decisions/` | 002 | [A Length Rather Than a Flag](../decisions/002_a_length_rather_than_a_flag.md) | `unsafe`-freedom bought with a bounded zeroing cost, and the distinction a length cannot make |
| `integration/` | 001 | [Seven Dependents and Four That Stay Generic](../integration/001_seven_dependents_and_four_that_stay_generic.md) | Genericity holding across three layers, and the layer that drops it |
| `integration/` | 002 | [Where the Second Shape Stops](../integration/002_where_the_second_shape_stops.md) | One library consumer for `BytesSlot`, and a promised bench that does not exist |
| `invariant/` | 001 | [A Read Returns What Was Written](../invariant/001_a_read_returns_what_was_written.md) | A domain exhausted rather than sampled, and the one fixture with a non-zero tail |
| `invariant/` | 002 | [The Two Emptiness Paths Agree](../invariant/002_the_two_emptiness_paths_agree.md) | A delegation invariant, and the shape that has no second path at all |
| `item/` | 001 | [The Four of a `TypedSlot`](../item/001_the_four_of_a_typed_slot.md) | Accessors consumed as function values, and a mutability split that determines an API elsewhere |
| `item/` | 002 | [The Six of a `BytesSlot`](../item/002_the_six_of_a_bytes_slot.md) | A receiver that is never read, and the absence of any incremental writer |
| `lifecycle/` | 001 | [A Slot Across One Publish](../lifecycle/001_a_slot_across_one_publish.md) | A mandatory third call that fails silently, and the two-step model only one shape can use |
| `lifecycle/` | 002 | [A Slot Across a Ring's Laps](../lifecycle/002_a_slot_across_a_rings_laps.md) | One `clear` that frees and one that forgets, and residue with no upper bound |
| `non_functional_requirement/` | 001 | [What a Slot Costs](../non_functional_requirement/001_what_a_slot_costs.md) | Six absences holding up the budget, and a `no_std` profile the crate does not declare while its dependency does |
| `non_functional_requirement/` | 002 | [Four Traits and What They Compare](../non_functional_requirement/002_four_traits_and_what_they_compare.md) | A clone proportional to capacity, and the trait that is required but not derived |
| `pattern/` | 001 | [One Trait, Two Shapes](../pattern/001_one_trait_two_shapes.md) | Object safety nobody claims, and the payload half living one crate away |
| `pattern/` | 002 | [The Displaced Value Returned](../pattern/002_the_displaced_value_returned.md) | A real reason to keep the value that nothing states, and four call sites handling one return four ways |
| `pitfall/` | 001 | [The Test That Names a Property the Type Lacks](../pitfall/001_the_test_that_names_a_property_the_type_lacks.md) | An assertion that passes for the wrong reason, and two relations the crate never reconciles |
| `pitfall/` | 002 | [Clear Forgets, It Does Not Erase](../pitfall/002_clear_forgets_it_does_not_erase.md) | One contract with two deliveries, and residue that outlives its publish |
| `type/` | 001 | [Two Shapes, One Trait, No `Copy`](../type/001_two_shapes_one_trait_no_copy.md) | A derive omitted for two different reasons, and two payload channels different in kind |
| `type/` | 002 | [The Const Parameter as Capacity](../type/002_the_const_parameter_as_capacity.md) | Width fixed at compile time, and an unbounded `N` whose far end is named but untested |
| `workaround/` | 001 | [Four Functions That Could Be `const`](../workaround/001_four_functions_that_could_be_const.md) | Two free to mark today, two blocked by an operator rather than by what they do |
| `workaround/` | 002 | [`BatchTooLarge` Borrowed for a Different Shape](../workaround/002_batchtoolarge_borrowed_for_a_different_shape.md) | Slot-denominated fields filled with bytes, and why borrowing is still right |

## Findings

Fifty-two, each verified by a command whose output is quoted in its instance.

| ID | Finding | Subject | Reachable | Where |
|----|---------|---------|-----------|-------|
| SL1 | Genericity over `Slot` survives three layers — storage, both rings, and `ring_event`'s recycle — with no coordination between them | family | n/a — observation | [integration/001](../integration/001_seven_dependents_and_four_that_stay_generic.md) |
| SL2 | `ring_core` hard-wires `TypedSlot< T >` at thirteen sites, so the traffic `BytesSlot` exists for has no route through the family's unified handle, and nothing states the limit | `ring_core` | n/a — doc gap | [integration/001](../integration/001_seven_dependents_and_four_that_stay_generic.md) |
| SL3 | `BytesSlot` has exactly one library consumer — `ring_event`'s two trait impls — against 46 mentions family-wide, 42 of them doctests and tests | family | n/a — observation | [integration/002](../integration/002_where_the_second_shape_stops.md) |
| SL4 | `ring_slot`'s own argument calls for the bench that justifies two shapes; `ring_bench` benches `TypedSlot` twice and never instantiates `BytesSlot` | `ring_bench` | **wrong doc** | [integration/002](../integration/002_where_the_second_shape_stops.md) |
| SL5 | The ruling is enforced by a trait rather than a convention, which is strictly stronger than the ruling required and is why four crates stayed generic across three layers | `ring_slot` | n/a — observation | [decisions/001](../decisions/001_two_shapes_rather_than_one.md) |
| SL6 | The founding copy-cost asymmetry is argued in prose, calls for its own measurement, and is measured nowhere | `ring_slot` | n/a — unenforced | [decisions/001](../decisions/001_two_shapes_rather_than_one.md) |
| SL7 | The length representation buys workspace-wide `unsafe`-freedom at an `N`-byte zeroing cost bounded by the ring's allocate-once behaviour | `ring_slot` | n/a — observation | [decisions/002](../decisions/002_a_length_rather_than_a_flag.md) |
| SL8 | A length cannot distinguish empty from zero-length; the complete explanation of that limitation lives in `ring_event::Peek`, two crates from the type it describes | `ring_event` | n/a — doc gap | [decisions/002](../decisions/002_a_length_rather_than_a_flag.md) |
| SL9 | The invariant is exhausted over every length `0..=CAP` on a fresh slot rather than sampled, with the reason stated in one line of doc comment | `ring_slot` | n/a — observation | [invariant/001](../invariant/001_a_read_returns_what_was_written.md) |
| SL10 | The only fixture with a non-zero tail asserts solely through `read()`; the module comment states the property of the slot, where it holds only of `read` | `ring_slot` | n/a — coverage | [invariant/001](../invariant/001_a_read_returns_what_was_written.md) |
| SL11 | The trait `is_empty` calls the inherent one, so disagreement is unrepresentable; the test that asserts it anyway guards the future edit, not the present code | `ring_slot` | n/a — observation | [invariant/002](../invariant/002_the_two_emptiness_paths_agree.md) |
| SL12 | `TypedSlot` has no inherent `is_empty`, so the concrete-type ergonomics `BytesSlot`'s doc argues for are absent on the other shape and nothing says so | `ring_slot` | n/a — doc gap | [invariant/002](../invariant/002_the_two_emptiness_paths_agree.md) |
| SL13 | The bound check precedes every mutation, so a refused write is total with no rollback; both directions are covered by separate tests | `ring_slot` | n/a — observation | [algorithm/001](../algorithm/001_write_is_a_bound_check_and_a_copy.md) |
| SL14 | No loop, no retry, no atomic, no branch outside `write` — complexity one everywhere but a single comparison | `ring_slot` | n/a — observation | [algorithm/001](../algorithm/001_write_is_a_bound_check_and_a_copy.md) |
| SL15 | The two real emptiness computations read different fields and cannot be merged, which is why `Slot::is_empty` has no default body | `ring_slot` | n/a — observation | [algorithm/002](../algorithm/002_emptiness_three_ways.md) |
| SL16 | `Buffer::all_empty` is the fold; `Buffer::is_empty` is a differently-named method that is always `false`, existing only to satisfy a clippy lint | `ring_store` | **misleading doc** | [algorithm/002](../algorithm/002_emptiness_three_ways.md) |
| SL17 | `#[ must_use ]` covers the five cheapest functions and omits `set` and `take`, the only two returning an owned value whose loss is silent | `ring_slot` | **latent hazard** | [api/001](../api/001_ten_functions_six_const_seven_must_use.md) |
| SL18 | The policy `set`'s justification serves is unreachable — `ring_core` refuses `DropOldest` at construction, and the one call site that binds the value reads it to assert the opposite of what the sentence claims | `ring_slot` | **wrong doc** | [api/001](../api/001_ten_functions_six_const_seven_must_use.md) |
| SL19 | Neither trait method moves a value, drawing the generic boundary exactly before the point where the two shapes' payload types diverge | `ring_slot` | n/a — observation | [api/002](../api/002_a_trait_with_two_methods.md) |
| SL20 | Three of four generic consumers add `Default` to `Slot` because the trait declares no constructor, and the added bound costs `const` construction | `ring_slot` | n/a — observation | [api/002](../api/002_a_trait_with_two_methods.md) |
| SL21 | The length field costs a fixed word plus alignment at every `N`, and all four widths the consuming crates use sit in the band where it dominates — 100% overhead at `N == 8` | `ring_slot` | **measured cost** | [data_structure/001](../data_structure/001_sixteen_bytes_to_carry_eight.md) |
| SL22 | A `u8` length fits every `N` in use; `usize` is still right for cast-freedom and `slice::len` parity, and the trade is recorded nowhere | `ring_slot` | n/a — observation | [data_structure/001](../data_structure/001_sixteen_bytes_to_carry_eight.md) |
| SL23 | `TypedSlot< T >` is byte-identical to `Option< T >` at every `T` measured, so the newtype is free and composes into further `Option`s for free | `ring_slot` | n/a — observation | [data_structure/002](../data_structure/002_the_niche_option_finds_and_the_array_does_not.md) |
| SL24 | `BytesSlot< N >` has no niche at any `N`, so every wrapping layer costs a full word — compounding with the length field rather than trading against it | `ring_slot` | **measured cost** | [data_structure/002](../data_structure/002_the_niche_option_finds_and_the_array_does_not.md) |
| SL25 | `get` and `take` are consumed as function values at fifty sites in three crates; the crate's own doctests demonstrate only the receiver form, and the point-free contract is undeclared | `ring_slot` | n/a — doc gap | [item/001](../item/001_the_four_of_a_typed_slot.md) |
| SL26 | The `&self`/`&mut self` split is the whole drain contract and the reason `Batch::get_mut` exists; it is explained twice in `ring_spsc` and not at the definition site | `ring_slot` | n/a — doc gap | [item/001](../item/001_the_four_of_a_typed_slot.md) |
| SL27 | `capacity` names a receiver it never reads, has no caller outside the crate's own suite, and cannot be reached from the type alone because no associated const exists | `ring_slot` | n/a — observation | [item/002](../item/002_the_six_of_a_bytes_slot.md) |
| SL28 | No mutable accessor and no incremental writer, so a payload is replaced whole — correct for a ring, and unstated, leaving a large-`N` caller to discover the staging copy | `ring_slot` | n/a — doc gap | [item/002](../item/002_the_six_of_a_bytes_slot.md) |
| SL29 | A `ring_event` drain never empties the slot, so `recycle` is mandatory and separate; omitting it is neither a compile error nor a runtime error and leaves a drained ring reading non-empty | `ring_event` | **latent hazard** | [lifecycle/001](../lifecycle/001_a_slot_across_one_publish.md) |
| SL30 | The family runs two publish models and the safer, self-clearing one works only for `TypedSlot`; `read`'s doc never notes that it borrows where `take` would empty | `ring_slot` | n/a — doc gap | [lifecycle/001](../lifecycle/001_a_slot_across_one_publish.md) |
| SL31 | The two `clear` bodies agree on everything the API can see and disagree underneath — one drops the payload, the other moves a counter | `ring_slot` | n/a — observation | [lifecycle/002](../lifecycle/002_a_slot_across_a_rings_laps.md) |
| SL32 | A cleared `BytesSlot` retains all `N` bytes until a longer write lands on that slot or the ring dies; no bulk reset is wired to any lifecycle event and no doc states the window | `ring_slot` | **latent hazard** | [lifecycle/002](../lifecycle/002_a_slot_across_a_rings_laps.md) |
| SL33 | Six mechanically-checkable absences hold the cost budget up; only `unsafe` is enforced by a lint, and no test asserts any of the other five | `ring_slot` | n/a — unenforced | [non_functional_requirement/001](../non_functional_requirement/001_what_a_slot_costs.md) |
| SL34 | The crate has the source profile of a `no_std` crate and does not declare it, while its one dependency `ring_types` does — so a future `std` import would compile and remove the property with no signal | `ring_slot` | n/a — unenforced | [non_functional_requirement/001](../non_functional_requirement/001_what_a_slot_costs.md) |
| SL35 | `Clone` on a `BytesSlot` copies `N`, not `len` — 4104 bytes for a one-byte payload — and is unconditional on that shape while conditional on `TypedSlot`'s `T` | `ring_slot` | **measured cost** | [non_functional_requirement/002](../non_functional_requirement/002_four_traits_and_what_they_compare.md) |
| SL36 | The three real consumer bounds are all `Slot + Default`, satisfied by a hand-written `impl< T >`; replacing it with `#[ derive( Default ) ]` compiles today and narrows the type silently | `ring_slot` | **latent hazard** | [non_functional_requirement/002](../non_functional_requirement/002_four_traits_and_what_they_compare.md) |
| SL37 | `Slot` is object-safe by construction and the family dispatches statically everywhere; a mixed-shape ring is expressible today and neither endorsed nor forbidden | `ring_slot` | n/a — observation | [pattern/001](../pattern/001_one_trait_two_shapes.md) |
| SL38 | The trait carries lifecycle and no payload, so the abstraction spans two crates — and nothing here points at `ring_event::Fill`/`Peek`, which hold the other half | `ring_slot` | n/a — doc gap | [pattern/001](../pattern/001_one_trait_two_shapes.md) |
| SL39 | The reason to keep the displaced value is real and unstated — it is the only evidence a caller can obtain that a slot it believed empty was not; the sentence that is there names a Tier 1 policy crate structurally beside this one, which holds no slot and evicts nothing | `ring_slot` | **wrong doc** | [pattern/002](../pattern/002_the_displaced_value_returned.md) |
| SL40 | The family's only claimed-slot protocol check sits behind `debug_assert!`, so the one place a silently-destroyed record could be caught is compiled out of release builds | `ring_core` | **latent hazard** | [pattern/002](../pattern/002_the_displaced_value_returned.md) |
| SL41 | `slots_compare_by_payload_not_by_tail` asserts a property `BytesSlot` does not have; it passes because both fixtures are written once from empty, so no tail can differ | `ring_slot` | **wrong doc** | [pitfall/001](../pitfall/001_the_test_that_names_a_property_the_type_lacks.md) |
| SL42 | Four accessors define the slot as its first `len` bytes and two derives define it as all `N`; nothing in the source names the split, and the one test positioned to catch it denies it | `ring_slot` | **latent hazard** | [pitfall/001](../pitfall/001_the_test_that_names_a_property_the_type_lacks.md) |
| SL43 | `Slot::clear` promises the payload is dropped; `BytesSlot` resets a length and leaves the bytes printable by `Debug` and significant to `PartialEq`, unremarked on either type | `ring_slot` | n/a — doc gap | [pitfall/002](../pitfall/002_clear_forgets_it_does_not_erase.md) |
| SL44 | A slot holds the longest payload ever written to its position, not the current one — unreachable through `read()`, reachable through `Debug`, and printed by any diagnostic dump or assertion failure | `ring_slot` | **latent hazard** | [pitfall/002](../pitfall/002_clear_forgets_it_does_not_erase.md) |
| SL45 | Thirty-seven types in the family derive `Copy` and neither slot does; `BytesSlot` could, and the reason it must not — an implicit `N`-byte memcpy per move — is recorded nowhere | `ring_slot` | n/a — doc gap | [type/001](../type/001_two_shapes_one_trait_no_copy.md) |
| SL46 | The absent `Copy` is what makes `ring_store`'s four reference-only accessors enforceable; nothing states that a `Copy` implementor of `Slot` would undo it | `ring_slot` | n/a — doc gap | [type/001](../type/001_two_shapes_one_trait_no_copy.md) |
| SL47 | Width is part of the type and slot count is part of the value, one tier apart under the same word, with no note on either — and no container can hold two widths | `ring_slot` | n/a — doc gap | [type/002](../type/002_the_const_parameter_as_capacity.md) |
| SL48 | `N` carries no bound and no `where` clause; the far end is instantiated once to assert `capacity()` and never exercised, and the guidance on where the design stops working lives in four other instances rather than at `N` | `ring_slot` | n/a — doc gap | [type/002](../type/002_the_const_parameter_as_capacity.md) |
| SL49 | `set` and `take` are non-`const` for no reason the code contains — both bodies const-evaluate verbatim on the workspace's toolchain | `ring_slot` | n/a — observation | [workaround/001](../workaround/001_four_functions_that_could_be_const.md) |
| SL50 | `read` and `write` are blocked by `Index`/`IndexMut` not being const-stable, not by what they do; nothing marks them as waiting, so the constraint will lift silently | `ring_slot` | n/a — doc gap | [workaround/001](../workaround/001_four_functions_that_could_be_const.md) |
| SL51 | `write` fills slot-denominated fields with byte counts, so a refused write renders `batch of 20 exceeds ring capacity 16` — a batch that does not exist and a ring capacity that is not involved | `ring_slot` | **misleading doc** | [workaround/002](../workaround/002_batchtoolarge_borrowed_for_a_different_shape.md) |
| SL52 | The borrow is the correct call at this tier and inherits the right classification; neither the construction site nor the variant's declaration records that its fields carry two different units | `ring_types` | n/a — doc gap | [workaround/002](../workaround/002_batchtoolarge_borrowed_for_a_different_shape.md) |

### Fifty-Two Findings About One Trait and Two Structs

The crate is 288 lines declaring `Slot`, which has two methods and no payload;
`TypedSlot< T >`, which is an `Option`; and `BytesSlot< N >`, which is an array
and a length. The distribution:

| Subject of the finding | Count |
|------------------------|------:|
| `ring_slot` itself | 43 |
| the family as a whole | 2 |
| another single crate — `ring_core` 2, `ring_event` 2, `ring_bench` 1, `ring_store` 1, `ring_types` 1 | 7 |

**Nine of fifty-two are not about this crate** — a much lower share than a Tier 5
primitive produces, and for the opposite reason. `ring_slot` sits at Tier 1 with
one dependency, so there is almost nothing beneath it to audit; what it finds
elsewhere it finds *above* itself, in the crates that consume it. The two
`ring_core` findings (SL2, SL40), both `ring_event` ones (SL8, SL29), and the
`ring_store` one (SL16) are all consequences of this crate's shape landing in a
consumer — which is why the same reading that documents a 288-line leaf turns up
a protocol check compiled out of release builds four tiers up.

### The Shape Every Definition Found

Stated once because most of the thirteen definitions arrive at it independently:
**the two shapes are not the same kind of thing, and almost every sentence in the
crate says "a slot".**

| Where the gap is | Findings |
|------------------|----------|
| the two shapes differ and the prose does not distinguish them | SL8, SL12, SL30, SL31, SL35, SL43 |
| a right decision with no record that a trade was made | SL22, SL45, SL46, SL47, SL50, SL52 |
| a property held by one accessor and claimed for the type | SL10, SL41, SL42, SL44 |
| a doc pointing somewhere that cannot do what it says | SL16, SL18, SL39, SL51 |
| a contract stated at the consumer rather than the definition | SL8, SL25, SL26, SL28, SL38 |
| an argument or property with nothing enforcing it | SL4, SL6, SL33, SL34, SL36 |

Not one is a bug. Every behaviour described is correct against what the code
intends, and several of the decisions are better than the documents that ordered
them — the trait in SL5 is stronger enforcement than the ruling asked for. The
cost is uniform: a reader who takes "a slot" to mean both shapes is wrong about
one of them, and the source will not say which.

### Severity

| Tier | Meaning | Findings | Count |
|------|---------|----------|------:|
| **latent hazard** | well-typed code reaches silent data loss or leaked residue | SL17, SL29, SL32, SL36, SL40, SL42, SL44 | 7 |
| **wrong doc** | a stated fact is false, measured | SL4, SL18, SL39, SL41 | 4 |
| **misleading doc** | a true statement a caller can act on and be wrong | SL16, SL51 | 2 |
| **measured cost** | a real, measured runtime cost nothing records | SL21, SL24, SL35 | 3 |
| n/a — doc gap | a true fact is unstated where it is needed | SL2, SL8, SL12, SL25, SL26, SL28, SL30, SL38, SL43, SL45, SL46, SL47, SL48, SL50, SL52 | 15 |
| n/a — unenforced | a real property with no lint, test, or check behind it | SL6, SL33, SL34 | 3 |
| n/a — coverage | the tests do not reach what they appear to | SL10 | 1 |
| n/a — observation | true, useful, and carrying no defect | the remaining 17 | 17 |

The first four tiers are the reachable ones — 16 of 52. Everything below them is
a record, not a problem.

The seven latent hazards share the property that makes them worth ranking first:
**none produces a diagnostic at the point of the mistake.** Discarding what `set`
returns compiles clean (SL17); omitting `recycle` is neither a compile error nor
a runtime one (SL29); swapping a hand-written `Default` for a derive compiles and
narrows the type (SL36); and the one check that would catch a destroyed record is
removed by the release profile (SL40).

Three of the seven — SL32, SL42, SL44 — were the same fact seen from three
definitions: a `BytesSlot` keeps the longest payload ever written to it, and the
crate held two incompatible answers to what the slot's value *is*. The accessors
said the first `len` bytes; `Debug` and `PartialEq` said all `N`. Nothing in the
source named the disagreement, and `slots_compare_by_payload_not_by_tail` — the
one test positioned to find it — asserted the opposite and passed. Their
dispositions closed the split from both ends: `Debug`, `PartialEq` and `Eq` are
written by hand over `read()` so all six observations agree, `Slot::clear`'s
contract now states how long the residue lasts and that no public API can reach
it, and the test builds the fixture it was named for. The retention itself is
unchanged and deliberately so — SL32's own pricing declines a per-lap memset —
so what the three findings recorded is now true of the memory and false of the
API, which is the distinction none of the three could previously state.

The three measured costs are separated from the hazards deliberately. None
threatens correctness; all three are `BytesSlot` paying for a fixed width, and
they compound rather than trade — the length field (SL21), the absent niche
(SL24), and a `Clone` proportional to `N` (SL35) all charge for the same array.
