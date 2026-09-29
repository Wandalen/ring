# Doc Definitions

Module Index for `ring_store` — every doc definition this crate declares and every
instance under each, in one place.

## Master Doc Definitions Table

| Type | Purpose | Master File | Instances |
|------|---------|-------------|----------:|
| `algorithm/` | One mask where a modulo would be, and an allocation measured against the claim that names it | [algorithm/readme.md](../algorithm/readme.md) | 2 |
| `api/` | Twelve functions under an attribute rule nobody states, and six accessors of which consumers use two | [api/readme.md](../api/readme.md) | 2 |
| `data_structure/` | Three words whatever the slot costs, and the constructor bound that is the whole type's storability requirement | [data_structure/readme.md](../data_structure/readme.md) | 2 |
| `decisions/` | A panic argued from provenance the type cannot carry, and a word kept to be checked against itself | [decisions/readme.md](../decisions/readme.md) | 2 |
| `definition/` | Module Index — every definition and every instance in this crate, in one place | this file | — |
| `integration/` | Four dependents of two kinds, and every `unsafe` block in the family reaching into this storage | [integration/readme.md](../integration/readme.md) | 2 |
| `invariant/` | A relation unbreakable after one line, and a non-aliasing clause discharged one tier down | [invariant/readme.md](../invariant/readme.md) | 2 |
| `item/` | The two values this crate handles — a slot it never opens, an index it never validates | [item/readme.md](../item/readme.md) | 2 |
| `lifecycle/` | Allocate once, borrow forever, drop plainly — and a sweep whose only caller is a test | [lifecycle/readme.md](../lifecycle/readme.md) | 2 |
| `non_functional_requirement/` | The two properties that make this worth a crate, measured, and the tests that do not assert them | [non_functional_requirement/readme.md](../non_functional_requirement/readme.md) | 2 |
| `pattern/` | A fold delegated one tier down, and an API that is borrows all the way through | [pattern/readme.md](../pattern/readme.md) | 2 |
| `pitfall/` | Two questions sharing the name `is_empty`, and two accessors that compile into nothing | [pitfall/readme.md](../pitfall/readme.md) | 2 |
| `type/` | One parameter across three `impl` blocks, one derive, and the three types that cross the boundary | [type/readme.md](../type/readme.md) | 2 |
| `workaround/` | Three construction steps routing around two bounds, and a heap allocation bought deliberately | [workaround/readme.md](../workaround/readme.md) | 2 |

13 definitions, 26 instances.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_store/docs
find . -mindepth 1 -maxdepth 1 -type d ! -name definition | wc -l   # definitions
find . -name '0*.md' | wc -l                                        # instances
grep -rho '^### BF[0-9]*' . | wc -l                                 # findings
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
| `algorithm/` | 001 | [One Mask, No Modulo](../algorithm/001_one_mask_no_modulo.md) | One composition written two ways, and a Tier 1 crate selling three functions to a family that buys one |
| `algorithm/` | 002 | [One Allocation, N Defaults](../algorithm/002_one_allocation_n_defaults.md) | The allocation claim measured exact, and the per-slot cost the crate does not bound |
| `api/` | 001 | [Twelve Functions, Three `const`, Seven `must_use`](../api/001_twelve_functions_three_const_seven_must_use.md) | An exact `must_use` rule nobody states, and a `const` boundary drawn one crate away |
| `api/` | 002 | [Six Ways to Reach a Slot](../api/002_six_ways_to_reach_a_slot.md) | Three pairs of which consumers use one, and an exit that does not exist for a reason nothing records |
| `data_structure/` | 001 | [Three Words, Whatever the Slot Costs](../data_structure/001_three_words_whatever_the_slot_costs.md) | A handle whose size is independent of both parameters, and an absence pinned at one instantiation |
| `data_structure/` | 002 | [The `Default` Bound Is a Bound on the Type](../data_structure/002_the_default_bound_is_a_bound_on_the_type.md) | A storability bound that is `Default` rather than the conspicuous `Slot`, and a family-wide requirement written in one consumer's prose |
| `decisions/` | 001 | [Panic Rather Than `Option`](../decisions/001_panic_rather_than_option.md) | A decision argued from provenance, and a premise the type does not hold |
| `decisions/` | 002 | [A Length Kept to Be Checked Against Itself](../decisions/002_a_length_kept_to_be_checked_against_itself.md) | A redundant word that guards one line, and a function that is a compile-time constant |
| `integration/` | 001 | [Four Dependents, Two That Build On It](../integration/001_four_dependents_two_that_build_on_it.md) | Two kinds of edge under one name, a four-tier gap, and nine functions with no caller |
| `integration/` | 002 | [Every `unsafe` Block in the Family Reaches Into a Buffer](../integration/002_every_unsafe_block_in_the_family.md) | The opt-out set equals the consumer set, and where the soundness argument actually lives |
| `invariant/` | 001 | [Capacity Equals Length, Always](../invariant/001_capacity_equals_length_always.md) | An invariant nothing can break after `new`, and a contract clause naming an absent API |
| `invariant/` | 002 | [Two Distinct Indices Never Alias](../invariant/002_two_distinct_indices_never_alias.md) | A test that cannot fail, and the tier that actually earns the assurance |
| `item/` | 001 | [The Slot as the Buffer Sees It](../item/001_the_slot_as_the_buffer_sees_it.md) | Two method calls and no payload access, and a trait whose generic reach is three lines |
| `item/` | 002 | [The `SlotIndex` the Buffer Trusts](../item/002_the_slotindex_the_buffer_trusts.md) | An accessor used where a public field was offered, and a sequence that lives for one line |
| `lifecycle/` | 001 | [Allocate Once, Borrow Forever, Drop Plainly](../lifecycle/001_allocate_once_borrow_forever_drop_plainly.md) | No `Drop` impl and destructors anyway, and a slot nothing ever empties |
| `lifecycle/` | 002 | [The Sweep Nothing Calls](../lifecycle/002_the_sweep_nothing_calls.md) | A requester that cannot reach the method, and a guarantee half the shapes deliver |
| `non_functional_requirement/` | 001 | [Allocate Once, Then Never Again](../non_functional_requirement/001_allocate_once_then_never_again.md) | The allocation claim measured end to end, and the boundary it stops at |
| `non_functional_requirement/` | 002 | [Bounded Work per Operation](../non_functional_requirement/002_bounded_work_per_operation.md) | A clean cost split, and a predicate cheapest when the answer is no |
| `pattern/` | 001 | [Delegate the Fold, Own the Storage](../pattern/001_delegate_the_fold_own_the_storage.md) | One fold reaching both rings without either depending on it, and the route that opens the capacity back up |
| `pattern/` | 002 | [Every Write Is a Borrow](../pattern/002_every_write_is_a_borrow.md) | A lattice complete on borrows and empty by value, and a private field fixed by public types |
| `pitfall/` | 001 | [The Two Questions Named `is_empty`](../pitfall/001_the_two_questions_named_is_empty.md) | A predicate that is always false, and seventeen more like it |
| `pitfall/` | 002 | [The Exclusive Accessor You Can Throw Away](../pitfall/002_the_exclusive_accessor_you_can_throw_away.md) | Two accessors that compile into nothing, and the lint that is not on |
| `type/` | 001 | [One Parameter, Three `impl` Blocks, One Derive](../type/001_one_parameter_three_impl_blocks_one_derive.md) | A bound split two crates now depend on, and a derive that dumps residue |
| `type/` | 002 | [The Types That Cross the Boundary](../type/002_the_types_that_cross_the_boundary.md) | One validated type of three, and eight derives against one |
| `workaround/` | 001 | [Three Steps to Avoid Two Bounds](../workaround/001_three_steps_to_avoid_two_bounds.md) | What each construction step buys, and what a drifting argument would cost |
| `workaround/` | 002 | [Capacity Is a Value, Width Is a Type](../workaround/002_capacity_is_a_value_width_is_a_type.md) | The heap as the price of a configurable capacity, and the opposite choice next to it |

## Findings

Fifty-three, each verified by a command whose output is quoted in its instance.

| ID | Finding | Subject | Reachable | Where |
|----|---------|---------|-----------|-------|
| BF1 | Four manifests name the crate and two of them are dev-dependencies; the apparent dependent count is double the build-graph one, and nothing distinguishes the two kinds | `ring_store` | n/a — observation | [integration/001](../integration/001_four_dependents_two_that_build_on_it.md) |
| BF2 | Both real consumers sit at Tier 6 and the four intervening tiers never touch storage — so no protocol crate can break on a `Buffer` change, and none exercises it either | family | n/a — observation | [integration/001](../integration/001_four_dependents_two_that_build_on_it.md) |
| BF3 | Of twelve public functions the real consumers call three; the other nine are covered by this crate's own suite and by nothing downstream | `ring_store` | n/a — coverage | [integration/001](../integration/001_four_dependents_two_that_build_on_it.md) |
| BF4 | The two crates that opt out of the workspace `unsafe` denial are exactly this crate's two ordinary dependents — a ring cannot express its borrow pattern through `&mut Buffer` | family | n/a — observation | [integration/002](../integration/002_every_unsafe_block_in_the_family.md) |
| BF5 | All twelve `unsafe` blocks in the family reach a `Buffer` slot, and the soundness argument for them lives entirely on the consumer side — a change here would be assessed against a contract both callers have reinterpreted | `ring_store` | **latent hazard** | [integration/002](../integration/002_every_unsafe_block_in_the_family.md) |
| BF6 | The panic decision is argued from where a `SlotIndex` came from rather than from what the type permits, so it inherits whatever strength that provenance claim has | `ring_store` | n/a — observation | [decisions/001](../decisions/001_panic_rather_than_option.md) |
| BF7 | `# Panics` tells a caller an out-of-range index means two rings were mixed; the field is public, no validation exists, and this crate's own suite hand-builds twenty-five indices against three folded ones | `ring_store` | **misleading doc** | [decisions/001](../decisions/001_panic_rather_than_option.md) |
| BF8 | The `capacity` field is eight bytes per buffer that `slots.len()` already knows; it is kept so `new`'s allocation claim is checkable from outside, and it guards exactly that one line | `ring_store` | **measured cost** | [decisions/002](../decisions/002_a_length_kept_to_be_checked_against_itself.md) |
| BF9 | `is_empty` is false for all sixteen legal capacities and its documentation opens by saying so; it exists because `len` does, and a lint requires the pair | `ring_store` | n/a — observation | [decisions/002](../decisions/002_a_length_kept_to_be_checked_against_itself.md) |
| BF10 | `at` and `at_mut` spell one composition two ways; the two-line form looks borrow-checker-imposed and is not — two-phase borrows accept the one-liner | `ring_store` | n/a — observation | [algorithm/001](../algorithm/001_one_mask_no_modulo.md) |
| BF11 | A Tier 1 crate publishes three functions and the family imports one; `aliases` and `run` have no caller, and `run` would allocate per batch on a path premised on not allocating | `ring_index` | n/a — coverage | [algorithm/001](../algorithm/001_one_mask_no_modulo.md) |
| BF12 | Construction costs exactly one allocation at every capacity and nothing afterwards allocates — an exact match to the reached-test's claim, asserted by no test | `ring_store` | n/a — observation | [algorithm/002](../algorithm/002_one_allocation_n_defaults.md) |
| BF13 | `Default::default` runs once per slot, so a shape's `default()` sits on the construction path `capacity` times; the doc's "once" attaches to the allocation, and nothing tells a shape author which | `ring_store` | n/a — doc gap | [algorithm/002](../algorithm/002_one_allocation_n_defaults.md) |
| BF14 | `#[ must_use ]` marks every reader and no mutator across all twelve functions, and `iter`'s apparent exception is the rule holding through `slice::Iter`'s own attribute — a regularity a reader can rely on and that nothing states | `ring_store` | n/a — observation | [api/001](../api/001_twelve_functions_three_const_seven_must_use.md) |
| BF15 | `get`/`get_mut` compile as `const` unchanged and `at`/`at_mut` cannot only because `ring_index::of` is a plain `fn` though it too compiles as `const`; four different reasons for non-constness are indistinguishable from outside | `ring_index` | n/a — doc gap | [api/001](../api/001_twelve_functions_three_const_seven_must_use.md) |
| BF16 | Six accessors are three mutability pairs; consumers call one pair, so four have never met a caller and their ergonomics are untested by use | `ring_store` | n/a — coverage | [api/002](../api/002_six_ways_to_reach_a_slot.md) |
| BF17 | No by-value `IntoIterator` and no decomposing function, so slots leave only when the buffer drops — a closure that protects the raw pointers two consumers hold, recorded nowhere | `ring_store` | n/a — doc gap | [api/002](../api/002_six_ways_to_reach_a_slot.md) |
| BF18 | The handle is 24 bytes at every capacity and every slot size, so a buffer contributes a fixed footprint to each ring and the benched candidates differ only in protocol state | `ring_store` | **measured cost** | [data_structure/001](../data_structure/001_three_words_whatever_the_slot_costs.md) |
| BF19 | The `size_of` stand-in for "holds no cursor" is instantiated at one slot type, so a field whose size depended on `S` could pass; the constancy BF18 measures is asserted nowhere | `ring_store` | n/a — coverage | [data_structure/001](../data_structure/001_three_words_whatever_the_slot_costs.md) |
| BF20 | `new` is the only constructor, so its bound is the whole type's storability requirement — and that bound is `Default` alone, while the conspicuous trait `Slot` is needed only by `clear` and `all_empty`; a shape author meets the loud requirement before the real one | `ring_store` | n/a — observation | [data_structure/002](../data_structure/002_the_default_bound_is_a_bound_on_the_type.md) |
| BF21 | Every slot shape must implement `Default`, stated once in this crate's prose inside a sentence about `unsafe`; the crate defining `Slot` never mentions it, so a new shape fails to compile pointing at a file its author never opened | `ring_slot` | **latent hazard** | [data_structure/002](../data_structure/002_the_default_bound_is_a_bound_on_the_type.md) |
| BF22 | The capacity/length relation is established in one line and unbreakable afterwards, since nothing writes either value and no second constructor exists | `ring_store` | n/a — observation | [invariant/001](../invariant/001_capacity_equals_length_always.md) |
| BF23 | This crate's reached-test — the contract it is verified against — names an indexed `set` that does not exist; writing a payload is the slot's operation, expressed here as an exclusive borrow | `ring_store` | **wrong doc** | [invariant/001](../invariant/001_capacity_equals_length_always.md) |
| BF24 | `distinct_indices_have_distinct_addresses` asserts a property of slices, not of `Buffer`; the `len`/storage mismatch its comment names is unreachable given a two-line implementation | `ring_store` | n/a — coverage | [invariant/002](../invariant/002_two_distinct_indices_never_alias.md) |
| BF25 | The load-bearing form of non-aliasing belongs to `ring_index` and is tested there; nothing connects that clause to the tier that actually discharges it | `ring_store` | n/a — doc gap | [invariant/002](../invariant/002_two_distinct_indices_never_alias.md) |
| BF26 | The buffer makes exactly two `Slot` calls and never touches a payload, which is the structural reason one definition serves both slot shapes without a branch | `ring_store` | n/a — observation | [item/001](../item/001_the_slot_as_the_buffer_sees_it.md) |
| BF27 | The `Slot` trait's generic surface is three call sites family-wide; `Slot::is_empty` is reached generically only inside `all_empty`, which has no caller, so nothing exercises the generic contract | `ring_slot` | n/a — coverage | [item/001](../item/001_the_slot_as_the_buffer_sees_it.md) |
| BF28 | Both `SlotIndex` unwraps go through the accessor rather than the public tuple field, while the crate's own tests build twenty-five indices by tuple syntax — the discipline holds in the source and is dropped in the examples | `ring_store` | n/a — observation | [item/002](../item/002_the_slotindex_the_buffer_trusts.md) |
| BF29 | `at( seq )` reads as sequence-addressed but stores no sequence, so a returned `&S` carries no evidence of which lap it belongs to; the correspondence lives in `ring_mpsc`'s stamps array and is documented only there | `ring_store` | n/a — doc gap | [item/002](../item/002_the_slotindex_the_buffer_trusts.md) |
| BF30 | The buffer needs no `Drop` impl because `Box< [ S ] >` has one, making storage the family's single owner of every payload; the family's four `Drop` impls are all RAII publish guards one tier up | `ring_store` | n/a — observation | [lifecycle/001](../lifecycle/001_allocate_once_borrow_forever_drop_plainly.md) |
| BF31 | Nothing in the family calls `take`, so a delivered payload stays resident for up to a lap and is destroyed by the buffer's drop if the ring dies first; that residency is not liveness is written only in one `ring_spsc` line comment | `ring_store` | n/a — doc gap | [lifecycle/001](../lifecycle/001_allocate_once_borrow_forever_drop_plainly.md) |
| BF32 | `clear`'s doc attributes the operation to `ring_shutdown`, which has no dependency on this crate and never mentions a buffer; the sweep's only caller family-wide is this crate's own test | `ring_store` | **misleading doc** | [lifecycle/002](../lifecycle/002_the_sweep_nothing_calls.md) |
| BF33 | `clear` states the family's strongest payload-visibility guarantee — a recycled ring must not hand a consumer the previous world's payloads — at a tier that can only forward `Slot::clear`, which for `BytesSlot` leaves every byte resident while `all_empty` reports true | `ring_store` | **misleading doc** | [lifecycle/002](../lifecycle/002_the_sweep_nothing_calls.md) |
| BF34 | The family has one fold implementation and both rings reach it transitively through `Buffer::at` without depending on `ring_index`; the safety of that route rests on `at` closing over `self.capacity` | `ring_store` | n/a — observation | [pattern/001](../pattern/001_delegate_the_fold_own_the_storage.md) |
| BF35 | `drain_order` manufactures `SlotIndex` values against a capacity supplied as a parameter, making it the one place `get`'s mixed-capacity panic scenario becomes reachable; neither crate's docs connect the two | `ring_batch` | n/a — doc gap | [pattern/001](../pattern/001_delegate_the_fold_own_the_storage.md) |
| BF36 | Every operation after `new` is a borrow and no by-value exit exists, which is what keeps `Buffer` from ever naming a payload type — and is stated nowhere, so absence reads as oversight | `ring_store` | n/a — observation | [pattern/002](../pattern/002_every_write_is_a_borrow.md) |
| BF37 | `slots` is private but its representation is published by four signatures naming `core::slice::Iter`, so the storage layout cannot change without a breaking release and nothing near the field says so | `ring_store` | n/a — doc gap | [pattern/002](../pattern/002_every_write_is_a_borrow.md) |
| BF38 | `Buffer::is_empty()` returns `false` for a buffer holding nothing, so `if buffer.is_empty()` is a branch that can never be taken under a predicate that compiles, is `const` and is `#[ must_use ]`; the question a caller means is `all_empty` | `ring_store` | **latent hazard** | [pitfall/001](../pitfall/001_the_two_questions_named_is_empty.md) |
| BF39 | Seventeen `is_empty` methods family-wide, and where the two buffer types meet the collision is resolved by binding the `ring_store::Buffer` to a variable named `ring` — a convention with no comment and nothing enforcing it | family | n/a — observation | [pitfall/001](../pitfall/001_the_two_questions_named_is_empty.md) |
| BF40 | `get_mut` and `at_mut` carry no `#[ must_use ]` while `get` and `at` do, so calling either as a statement compiles clean under `-D warnings` and does nothing | `ring_store` | **latent hazard** | [pitfall/002](../pitfall/002_the_exclusive_accessor_you_can_throw_away.md) |
| BF41 | `unused_results` rejects both discards and is absent from a workspace lint table that is otherwise deliberate and commented, so neither closure — the attribute or the lint — was recorded as considered | `ring_store` | n/a — unenforced | [pitfall/002](../pitfall/002_the_exclusive_accessor_you_can_throw_away.md) |
| BF42 | One allocation per buffer, sized exactly `capacity * size_of::< S >()` at every capacity and both shapes, and zero across ten thousand operations — the crate's strongest property, asserted by no test | `ring_store` | n/a — coverage | [non_functional_requirement/001](../non_functional_requirement/001_allocate_once_then_never_again.md) |
| BF43 | Allocate-once covers the slot array and not the payloads, so only `BytesSlot` preserves it end to end; that this is the reason the second slot shape exists is stated in neither crate | `ring_store` | n/a — doc gap | [non_functional_requirement/001](../non_functional_requirement/001_allocate_once_then_never_again.md) |
| BF44 | The four addressers are constant-time and the two `capacity`-bounded sweeps have no library caller, so a ring's per-message path is constant-time because nothing on it reaches the linear operations | `ring_store` | n/a — observation | [non_functional_requirement/002](../non_functional_requirement/002_bounded_work_per_operation.md) |
| BF45 | `all_empty` short-circuits, so confirming emptiness always costs full capacity while finding an occupied slot can cost one call, and the `true` it returns is an unsynchronized snapshot — neither is documented | `ring_store` | n/a — doc gap | [non_functional_requirement/002](../non_functional_requirement/002_bounded_work_per_operation.md) |
| BF46 | The bound split was recorded as unused generality; `ring_spsc` and `ring_mpsc` now store `Buffer< UnsafeCell< S > >`, which only the two loosest blocks accept, so the relaxation is load-bearing — an unused relaxation and one whose absence is silently deforming a caller are indistinguishable from inside the defining crate | `ring_store` | n/a — observation | [type/001](../type/001_one_parameter_three_impl_blocks_one_derive.md) |
| BF47 | The lint-required `#[ derive( Debug ) ]` renders 3,153,718 characters for a 256×`BytesSlot< 4096 >` buffer and includes payload bytes that survived a `clear`; both rings avoid it manually, only one says why, and neither names the volume or the residue | `ring_store` | **latent hazard** | [type/001](../type/001_one_parameter_three_impl_blocks_one_derive.md) |
| BF48 | Of the three crossing types only `Capacity` is validated and only it has a private field, so `get`'s bounds check is all that stands between an arbitrary `SlotIndex` and a slice index | `ring_store` | n/a — observation | [type/002](../type/002_the_types_that_cross_the_boundary.md) |
| BF49 | `Buffer` derives one trait against `Capacity`'s eight; the absences of `Clone`, `PartialEq` and `Default` are each correct and each unstated, so a reader meets them as omissions rather than as decisions | `ring_store` | n/a — doc gap | [type/002](../type/002_the_types_that_cross_the_boundary.md) |
| BF50 | `new`'s three steps are the intersection of the workspace `unsafe` denial and a `Clone` bound the `Slot` trait does not carry — `vec![ S::default(); n ]` fails to compile under `S : Slot + Default` and would propagate `Clone` to the payload if the bound were added — and neither constraint is named at the call site | `ring_store` | n/a — doc gap | [workaround/001](../workaround/001_three_steps_to_avoid_two_bounds.md) |
| BF51 | The two `capacity.get()` arguments must stay equal or `into_boxed_slice` shrinks and reallocates, measured at 1 alloc + 1 realloc, breaking the allocate-once property silently with no test asserting it; `with_capacity` is measurably redundant today but is the only form the API guarantees | `ring_store` | n/a — coverage | [workaround/001](../workaround/001_three_steps_to_avoid_two_bounds.md) |
| BF52 | The heap allocation is the price of keeping capacity a runtime value, and `ring_config::new( slots : usize ) -> Result` is the concrete signature that makes a const-generic capacity impossible — a whole design was traded here and nothing in either crate records it | `ring_store` | n/a — doc gap | [workaround/002](../workaround/002_capacity_is_a_value_width_is_a_type.md) |
| BF53 | Slot width is a const generic and slot count is a runtime value, meeting on one line of this crate's own doc example, under a coherent rule — a size that changes the type is a type parameter, a size that changes only the count is a value — that appears in neither crate | `ring_store` | n/a — observation | [workaround/002](../workaround/002_capacity_is_a_value_width_is_a_type.md) |

### Fifty-Three Findings About 261 Lines

The crate is a `Box< [ S ] >`, a `Capacity`, and twelve functions. It owns no
protocol, no atomics, no cursor and no policy; the fold it appears to perform it
delegates. The distribution:

| Subject of the finding | Count |
|------------------------|------:|
| `ring_store` itself | 45 |
| the family as a whole | 3 |
| another single crate — `ring_slot` 2, `ring_index` 2, `ring_batch` 1 | 5 |

**Eight of fifty-three are not about this crate**, and all eight are immediate
neighbours: the crate one tier below that defines the slot (BF21, BF27), the one
that owns the fold (BF11, BF15), and the one that manufactures indices against a
capacity of its own choosing (BF35). Nothing was found further away, which is
itself the finding — a storage tier has no reach. What it *has* is the property
BF5 records: every `unsafe` block in the family dereferences into this array, so
a crate that reaches nowhere is reached into by everything.

### The Shape Every Definition Found

Stated once because most of the thirteen definitions arrive at it independently:
**the code is 261 lines that do exactly what they say, wrapped in documentation
describing a larger crate.** Almost every doc sentence that reaches past the
boxed slice reaches into a neighbour, and about half of those are wrong about the
neighbour.

| Where the gap is | Findings |
|------------------|----------|
| a doc sentence reaching into a neighbour and getting it wrong | BF7, BF21, BF23, BF32, BF33, BF35 |
| a measured property with no test behind it | BF12, BF19, BF24, BF42, BF51 |
| a correct decision with nothing recording that a decision was made | BF17, BF36, BF37, BF49, BF50, BF52, BF53 |
| a function or a bound the family never reaches | BF3, BF11, BF16, BF20, BF27, BF44, BF46 |
| two different things sharing one name across a tier boundary | BF29, BF38, BF39, BF45 |
| a guarantee stated here and dischargeable only elsewhere | BF5, BF13, BF25, BF31, BF43 |

Not one is a bug. Every behaviour described is correct against what the code
intends, and several decisions are better than the documents that ordered them —
the reached-test asks for an indexed `set` that would have been the wrong API, and the
crate declines to provide it (BF23). The cost is uniform: the crate's own prose
is the least reliable thing about it, and a reader who trusts a doc sentence over
a signature will be wrong roughly one time in six.

### Severity

| Tier | Meaning | Findings | Count |
|------|---------|----------|------:|
| **latent hazard** | well-typed code silently does the wrong thing | BF5, BF21, BF38, BF40, BF47 | 5 |
| **wrong doc** | a stated fact is false, measured | BF23 | 1 |
| **misleading doc** | a true statement a caller can act on and be wrong | BF7, BF32, BF33 | 3 |
| **measured cost** | a real, measured runtime cost nothing records | BF8, BF18 | 2 |
| n/a — doc gap | a true fact is unstated where it is needed | BF13, BF15, BF17, BF25, BF29, BF31, BF35, BF37, BF43, BF45, BF49, BF50, BF52 | 13 |
| n/a — coverage | the tests do not reach what they appear to | BF3, BF11, BF16, BF19, BF24, BF27, BF42, BF51 | 8 |
| n/a — unenforced | a real property with no lint, test, or check behind it | BF41 | 1 |
| n/a — observation | true, useful, and carrying no defect | the remaining 20 | 20 |

The first four tiers are the reachable ones — 11 of 53. Everything below them is
a record, not a problem.

The **latent hazard** legend is broader here than the one `ring_slot` uses. There
the five hazards all ended in lost or leaked payload bytes; two of these five end
in nothing happening at all. `Buffer::is_empty()` is a `const`, `#[ must_use ]`
predicate that returns `false` for an empty buffer, so `if buffer.is_empty()`
guards a branch that can never be taken (BF38); `get_mut( i );` as a statement
compiles clean under `-D warnings` and does no work (BF40). Neither loses data.
Both are well-typed code doing the wrong thing quietly, which is the property
worth ranking first.

What all five share is that **none produces a diagnostic at the point of the
mistake.** A new slot shape that forgets `Default` fails to compile pointing at
`ring_store`, a crate its author never opened (BF21). A `#[ derive( Debug ) ]`
on any future type holding a `Buffer` satisfies the lint that demanded it and
prints three million characters including bytes a `clear` was believed to have
removed (BF47). And the soundness of all twelve `unsafe` blocks in the family
rests on a contract this crate states nowhere, so the one change that could
break them is the one this crate would judge safe on its own terms (BF5).

The two measured costs are separated from the hazards deliberately. Neither
threatens correctness: eight bytes of redundant `capacity` per buffer (BF8), and
a 24-byte handle that is the same size at every capacity and every slot type
(BF18) — which is the good news, stated as a cost only because nothing records
that it was ever in question.
