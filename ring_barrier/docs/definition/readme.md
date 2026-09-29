# Doc Definitions

Module Index for `ring_barrier` — every doc definition this crate declares and every
instance under each, in one place.

## Master Doc Definitions Table

| Type | Purpose | Master File | Instances |
|------|---------|-------------|----------:|
| `algorithm/` | The two computations, and the two endings one of them has | [algorithm/readme.md](../algorithm/readme.md) | 2 |
| `api/` | The nine methods, and what one borrowed slice commits them to | [api/readme.md](../api/readme.md) | 2 |
| `data_structure/` | Sixteen bytes pointing at other people's cursors | [data_structure/readme.md](../data_structure/readme.md) | 2 |
| `decisions/` | Choices with live alternatives, recorded with their arguments | [decisions/readme.md](../decisions/readme.md) | 2 |
| `definition/` | Module Index — every definition and every instance in this crate, in one place | this file | — |
| `integration/` | Three dependencies, one dependent, and the edge that is dev-only | [integration/readme.md](../integration/readme.md) | 2 |
| `invariant/` | Properties that must hold for every input, including two absences | [invariant/readme.md](../invariant/readme.md) | 2 |
| `item/` | Per-method contracts and coverage | [item/readme.md](../item/readme.md) | 2 |
| `lifecycle/` | A value with one state, and the drain loop that gives it a purpose | [lifecycle/readme.md](../lifecycle/readme.md) | 2 |
| `non_functional_requirement/` | What a frontier read costs, and what a non-blocking wait must not do | [non_functional_requirement/readme.md](../non_functional_requirement/readme.md) | 2 |
| `pattern/` | Shapes this crate participates in rather than invents | [pattern/readme.md](../pattern/readme.md) | 2 |
| `pitfall/` | Wrong uses that compile and look right | [pitfall/readme.md](../pitfall/readme.md) | 2 |
| `type/` | The width the crate never converts, and the traits it could not have | [type/readme.md](../type/readme.md) | 2 |
| `workaround/` | External constraints this crate absorbs, with costs and deletion conditions | [workaround/readme.md](../workaround/readme.md) | 2 |

13 definitions, 26 instances.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_barrier/docs
find . -mindepth 1 -maxdepth 1 -type d ! -name definition | wc -l   # 13
find . -name '0*.md' | wc -l                                        # 26
```

## Master Doc Instances Table

| Type | ID | Title | Subject |
|------|----|-------|---------|
| `algorithm/` | 001 | [The Frontier in Two Delegations](../algorithm/001_the_frontier_in_two_delegations.md) | Both computations leave the crate; the `Vec` is at step 3 |
| `algorithm/` | 002 | [`wait_for` Asks Twice](../algorithm/002_wait_for_asks_twice.md) | Two phases, three endings, and the allocation ledger |
| `api/` | 001 | [Nine Methods Over One Borrowed Slice](../api/001_nine_methods_over_one_borrowed_slice.md) | The whole surface in three tiers, 4 `const fn` of 9, 8 `#[ must_use ]` |
| `api/` | 002 | [The Borrow Is the Whole Type](../api/002_the_borrow_is_the_whole_type.md) | What `&'a [ … ]` buys, and the reference that outlives its barrier |
| `data_structure/` | 001 | [One Field and a Sixteen-Byte View](../data_structure/001_one_field_and_a_sixteen_byte_view.md) | 16 bytes pointing at 64 per dependency, against `GatingSet`'s `Vec` |
| `data_structure/` | 002 | [The Slice's Three Provenances](../data_structure/002_the_slices_three_provenances.md) | Publisher cursor, gating set, bare array — the 63-site census |
| `decisions/` | 001 | [Zero for a Barrier Over Nothing](../decisions/001_zero_for_a_barrier_over_nothing.md) | The `map_or( 0 )` default, four alternatives, and the near-miss |
| `decisions/` | 002 | [A Slice Rather Than an Aggregate](../decisions/002_a_slice_rather_than_an_aggregate.md) | The four-cursor handshake that the aggregate signature forbade |
| `integration/` | 001 | [Three Dependencies and One Dependent](../integration/001_three_dependencies_and_one_dependent.md) | The manifest graph, and the consumer that re-derives `available` |
| `integration/` | 002 | [The Dependency That Is Not `ring_seqno`](../integration/002_the_dependency_that_is_not_ring_seq.md) | `ring_wait` where the sibling has `ring_seqno` — the clamp costs a crate |
| `invariant/` | 001 | [The Frontier Never Exceeds a Dependency](../invariant/001_the_frontier_never_exceeds_a_dependency.md) | The safety property, and what its 20,000-iteration test can and cannot catch |
| `invariant/` | 002 | [Capacity Never Enters the Arithmetic](../invariant/002_capacity_never_enters_the_arithmetic.md) | An absence, asserted as 4 against 1,000 over one cursor |
| `item/` | 001 | [The Three Barrier Readings](../item/001_the_three_barrier_readings.md) | `frontier`, `available`, `admits` — contracts, callers and coverage |
| `item/` | 002 | [The Five Accessors and the Wait](../item/002_the_five_accessors_and_the_wait.md) | `over`, `dependencies`, `len`, `is_empty`, `cursor`, and `wait_for` |
| `lifecycle/` | 001 | [A Barrier From `over` to the End of a Borrow](../lifecycle/001_a_barrier_from_over_to_the_end_of_a_borrow.md) | One state, no destructor, and the family's four that have one |
| `lifecycle/` | 002 | [A Consumer Draining Behind a Producer](../lifecycle/002_a_consumer_draining_behind_a_producer.md) | The drain loop step by step, and the four-operation handshake |
| `non_functional_requirement/` | 001 | [Every Frontier Read Allocates](../non_functional_requirement/001_every_frontier_read_allocates_nothing.md) | 8 bytes per dependency per read, times the wait budget, measured |
| `non_functional_requirement/` | 002 | [A Non-Blocking Wait Must Look Exactly Once](../non_functional_requirement/002_a_non_blocking_wait_must_look_exactly_once.md) | `WaitKind::None`'s contract, and the test whose failure mode is a hang |
| `pattern/` | 001 | [The Borrowed View and the Owned Set](../pattern/001_the_borrowed_view_and_the_owned_set.md) | The ownership rule, recovered from a test that would not compile |
| `pattern/` | 002 | [The Quantity, the Predicate, and the Wait](../pattern/002_the_quantity_the_predicate_and_the_wait.md) | The three-rung ladder, and the four third rungs nobody calls |
| `pitfall/` | 001 | [The Two Empty Answers Look Like a Bug](../pitfall/001_the_two_empty_answers_look_like_a_bug.md) | Why *empty* means three things here, and the one nothing guards |
| `pitfall/` | 002 | [Returning the Request Instead of the Frontier](../pitfall/002_returning_the_request_instead_of_the_frontier.md) | The `from + count` mutation, and the 1 call site in 6 that catches it |
| `type/` | 001 | [A `u64` Distance and a `usize` Headroom](../type/001_a_u64_distance_and_a_usize_headroom.md) | What each width measures, and the crate that never casts |
| `type/` | 002 | [The Traits Derived and the Traits Absent](../type/002_the_traits_derived_and_the_traits_absent.md) | Three derives, two auto traits, five impossible, one declined |
| `workaround/` | 001 | [The Check That Capacity Stays Out](../workaround/001_the_check_that_capacity_stays_out.md) | A grep guarding an absence, and why it now proves less |
| `workaround/` | 002 | [`ring_gating` as a Dev-Dependency](../workaround/002_ring_gating_as_a_dev_dependency.md) | The placement, and the half of its guard that goes silent |

## Findings

Twenty-one, each verified by a command whose output is quoted in its instance,
or by a probe program in a scratch crate.

| ID | Finding | Subject | Reachable | Where |
|----|---------|---------|-----------|-------|
| BR1 | `Barrier::over` has 63 call sites and **none** in any `src/` — 33 in this crate's tests, 20 in `ring_consume`'s, 10 in `ring_publish`'s | `ring_barrier` | n/a — coverage | [api/001](../api/001_nine_methods_over_one_borrowed_slice.md) |
| BR2 | `ring_consume::Consumer::available` re-derives `Barrier::available` term-for-term, and `ring_seqno::pending` is its only `ring_seqno` use — the whole `ring_consume → ring_seqno` edge exists to spell one method by hand | `ring_consume` | n/a — duplication | [integration/001](../integration/001_three_dependencies_and_one_dependent.md) |
| BR3 | `available` has exactly 1 library caller (`admits`), `admits` exactly 1 (`wait_for`), and `wait_for` none outside tests — the readings form a chain hanging from the test suite | `ring_barrier` | n/a — coverage | [item/001](../item/001_the_three_barrier_readings.md) |
| BR4 | `wait_for` has 6 call sites, all in `tests/barrier_test.rs`; zero anywhere else in the family | `ring_barrier` | n/a — coverage | [item/002](../item/002_the_five_accessors_and_the_wait.md) |
| BR5 | `wait_for` is the only `ring_wait::wait_until` caller in the family that discards the returned attempt count | `ring_barrier` | n/a — unenforced | [algorithm/002](../algorithm/002_wait_for_asks_twice.md) |
| BR6 | `Barrier::over( &[] ).admits( ZERO, 0 )` is `true` while `wait_for( ZERO, 0, … )` is `Err( Empty )` — the one input class where the two phases disagree; untested, undocumented, unguarded | `ring_barrier` | n/a — inconsistency | [pitfall/001](../pitfall/001_the_two_empty_answers_look_like_a_bug.md) |
| BR7 | `size_of::< Barrier >() == 16 == size_of::< &[ PaddedCursor ] >()`, `align_of == 8`, and `Send + Sync + Copy` all hold — probe-verified, stated nowhere in the source | `ring_barrier` | n/a — observation | [api/002](../api/002_the_borrow_is_the_whole_type.md) |
| BR8 | `Barrier::cursor` returns `Option< &'a PaddedCursor >`, so a handed-out reference outlives its barrier; the same shape against `GatingSet::cursor` is `error[E0597]` | `ring_barrier` | n/a — observation | [api/002](../api/002_the_borrow_is_the_whole_type.md) |
| BR9 | 4 of 9 methods are `const fn` here against 1 of 11 in `ring_gating`; the boundary is `<[T]>::get` not being stable-`const` | `ring_barrier` | n/a — observation | [api/001](../api/001_nine_methods_over_one_borrowed_slice.md) |
| BR10 | B1's grep now passes because the slice signature left capacity no way into the file, not because capacity was kept out — the manual plan says so itself | `ring_barrier` | n/a — coverage | [workaround/001](../workaround/001_the_check_that_capacity_stays_out.md) |
| BR11 | `available` names 4 methods family-wide with 3 return types (`u64`, `Available`, `usize` ×2); `admits` names 3; `frontier` names exactly 1 | family | n/a — inconsistency | [pattern/002](../pattern/002_the_quantity_the_predicate_and_the_wait.md) |
| BR12 | `wait_for` is tested with `None` ×3, `Spin` ×2, `Yield` ×1 and `Park` ×0 — `Park` being the only variant that sleeps | `ring_barrier` | n/a — coverage | [item/002](../item/002_the_five_accessors_and_the_wait.md) |
| BR13 | Measured: a stalled `wait_for( …, Spin, 10_000 )` performs 10,000 reads and **0 allocations**; a satisfied `wait_for( …, None, 1 )` performs 2 reads and **0 allocations** — the read counts still match `n` and `n + 1`, the allocations they used to cost are gone since `ring_cursor` commit `b7e075ca` | `ring_barrier` | **measured cost** | [non_functional_requirement/001](../non_functional_requirement/001_every_frontier_read_allocates_nothing.md) |
| BR14 | Simulated, re-promoting `ring_gating` makes B4's first command print `ring_gating` and its second go **silent** — an emptied `[dev-dependencies]` satisfies "expected: no output" by having no subject left | `ring_barrier` | n/a — coverage | [workaround/002](../workaround/002_ring_gating_as_a_dev_dependency.md) |
| BR15 | `a_barrier_never_reports_a_frontier_a_dependency_has_not_reached` asserts equality with `Seq::ZERO` 20,000 times and **can** fail — unlike `ring_gating`'s counterpart (G12), which is a tautology | `ring_barrier` | n/a — observation | [invariant/001](../invariant/001_the_frontier_never_exceeds_a_dependency.md) |
| BR16 | `.dependencies()` has exactly one call site in the entire family, in this crate's own tests | `ring_barrier` | n/a — coverage | [item/002](../item/002_the_five_accessors_and_the_wait.md) |
| BR17 | `ring_flush` names `FlushPolicy::OnBarrier` and `FlushCause::Barrier`, declares no edge to this crate, and its own module doc explains why an edge would not help | `ring_flush` | n/a — inconsistency | [integration/001](../integration/001_three_dependencies_and_one_dependent.md) |
| BR18 | `admits_and_available_never_disagree` asserts `admits( f, c ) == ( c <= available( f ) )` — literally `admits`'s body — across 20,736 cases; it cannot fail today, and exists to guard a future hoisted-frontier optimization | `ring_barrier` | n/a — coverage | [item/001](../item/001_the_three_barrier_readings.md) |
| BR19 | The 512-item drain loop treats a spent budget as *try again* with no bound, so a dead producer makes the test hang rather than fail | `ring_barrier` | n/a — coverage | [lifecycle/002](../lifecycle/002_a_consumer_draining_behind_a_producer.md) |
| BR20 | No third rung in the family has a caller in any `src/`: `ring_wait::for_space`, `ring_wait::for_data`, `GatingSet::check` and `Barrier::wait_for` have 24 callers between them, every one a test of the rung itself | family | n/a — coverage | [pattern/002](../pattern/002_the_quantity_the_predicate_and_the_wait.md) |
| BR21 | 13 source files carry 38 casts between `usize` and `u64`; `ring_barrier` carries none — and the family's six *bare* narrowings are all in the crates on the route that does not use a `Barrier` | family | n/a — observation | [type/001](../type/001_a_u64_distance_and_a_usize_headroom.md) |
| BR22 | A successful `wait_for` folds the whole dependency slice once per attempt and once more to build its return value, so a wait succeeding on attempt *n* performs *n + 1* folds — 8,200 atomic loads at the default budget with eight dependencies, documented nowhere | `ring_barrier` | n/a — observation | [algorithm/001](../algorithm/001_the_frontier_in_two_delegations.md) |
| BR23 | The rustdoc calls the returned sequence "the frontier at the moment the wait succeeded"; it is read four lines after that moment, so the value returned is never the one `admits` accepted and is only guaranteed to be at least as far | `ring_barrier` | **misleading doc** | [algorithm/001](../algorithm/001_the_frontier_in_two_delegations.md) |
| BR24 | `wait_for` produces `RingError::Empty` on two lines for conditions sharing no cause — a budget that ran out, which says wait longer, and a barrier over an empty slice, which will never admit anything and makes the retry the variant invites an infinite loop | `ring_barrier` | n/a — inconsistency | [algorithm/002](../algorithm/002_wait_for_asks_twice.md) |
| BR25 | Eight of nine methods carry an explicit `must_use` and the ninth is the only one returning a `Result`, which `std` already marks — the correct allocation, and the opposite of what `ring_wait` does under the same ruling | `ring_barrier` | n/a — observation | [api/001](../api/001_nine_methods_over_one_borrowed_slice.md) |
| BR26 | `cursor` and `dependencies` both return `'a` rather than a borrow of `&self`, so both outlive the barrier that produced them; nine methods take `&self` and none returns a reference tied to it | `ring_barrier` | n/a — observation | [api/002](../api/002_the_borrow_is_the_whole_type.md) |
| BR27 | `Barrier` derives `Debug` and so does `PaddedCursor`, so formatting a barrier performs one atomic load per dependency at whatever ordering `std` picked — a second door to shared memory in a crate that otherwise delegates every read so the ordering lives in one place | `ring_barrier` | n/a — observation | [data_structure/001](../data_structure/001_one_field_and_a_sixteen_byte_view.md) |
| BR28 | Ten of thirty-three crates carry a lifetime-parameterised struct and this is the smallest: one field, sixteen bytes, `Copy`; `GatingSet`, the other half of the same feature, has no lifetime and is not `Copy` because it owns its cursors | family | n/a — observation | [data_structure/001](../data_structure/001_one_field_and_a_sixteen_byte_view.md) |
| BR29 | `PaddedCursor::new`, `Barrier::over` and `ring_consume::Consumer::new` are all `const`, so a bounded consumer can be built entirely at compile time; no `const` or `static` of either type exists anywhere, and keeping the chain intact constrains three constructors across three crates | family | n/a — observation | [data_structure/002](../data_structure/002_the_slices_three_provenances.md) |
| BR30 | The empty barrier gets three different shapes of answer inside one crate — `None` from `frontier`, `0` from `available` indistinguishable from *caught up*, and `Err( Empty )` from `wait_for` indistinguishable from a spent budget | `ring_barrier` | n/a — inconsistency | [decisions/001](../decisions/001_zero_for_a_barrier_over_nothing.md) |
| BR31 | `ring_seqno::slowest` and `ring_cursor::slowest` are one concept at two levels — same name, same return, same empty-set convention, different element type — and nothing in the family distinguishes them but the argument | family | n/a — duplication | [decisions/002](../decisions/002_a_slice_rather_than_an_aggregate.md) |
| BR32 | `ring_gating`'s empty-set argument opens by citing `ring_seqno::slowest`; `GatingSet::slowest` calls `ring_cursor::slowest`. Both return `None` for an empty input, so the argument survives its own citation being wrong, which is why it has gone unnoticed | `ring_gating` | **wrong doc** | [decisions/002](../decisions/002_a_slice_rather_than_an_aggregate.md) |
| BR33 | Three manifests name this crate and only `ring_consume` links it — `ring_publish`'s entry is a dev-dependency, so the handshake test the module doc cites as the reason for the slice signature lives in a crate that, in a release build, does not depend on this one at all | `ring_barrier` | n/a — observation | [integration/001](../integration/001_three_dependencies_and_one_dependent.md) |
| BR34 | `ring_seqno` exports the family's five sequence-arithmetic helpers and every one takes a capacity, so the crate named for sequence arithmetic is unreachable from the crate whose only job is a sequence subtraction — for the exact reason that subtraction must stay capacity-free | family | n/a — observation | [integration/002](../integration/002_the_dependency_that_is_not_ring_seq.md) |
| BR35 | `Seq::distance_to` justifies its saturation by describing two kinds of caller, and every one of the family's eleven call sites is the second kind — not one compares the two sequences first, so the justification rests entirely on the first category being empty | family | n/a — unenforced | [invariant/001](../invariant/001_the_frontier_never_exceeds_a_dependency.md) |
| BR36 | A consumer past its frontier and one exactly at it both get `available == 0`; the doctest asserts the two side by side and labels the second a property rather than a state that must never occur, and the saturation erases the evidence before `available` can return | `ring_barrier` | n/a — observation | [invariant/001](../invariant/001_the_frontier_never_exceeds_a_dependency.md) |
| BR37 | The crate whose invariant is about two threads racing on shared cursors is compiled out of every loom run, contributing twenty-two ordinary tests and no interleaving model; its one real-race test asserts progress rather than ordering | `ring_barrier` | n/a — coverage | [invariant/002](../invariant/002_capacity_never_enters_the_arithmetic.md) |
| BR38 | `frontier`, `available` and `admits` are one fold plus two arithmetic steps, layered so the cheapest question costs the most expensive one — `admits( from, 0 )` is a constant `true` and still folds every cursor, which is why `ring_consume` re-derives rather than paying twice | `ring_barrier` | n/a — observation | [item/001](../item/001_the_three_barrier_readings.md) |
| BR39 | `dependencies()` appears twice in the whole family, both in this crate — one doctest, one test assertion, no consumer; the accessor consumers reach for is `cursor( index )`, and the collection form survives because it is what a container would expose | `ring_barrier` | n/a — coverage | [item/002](../item/002_the_five_accessors_and_the_wait.md) |
| BR40 | The type has no `Drop`, no close, no release — the borrow is the whole state and ending is the borrow checker's business, which is what makes `Copy` sound and lets `ring_consume::Consumer` store a `Barrier` by value; `GatingSet` owns its cursors and can do none of it | `ring_barrier` | n/a — observation | [lifecycle/001](../lifecycle/001_a_barrier_from_over_to_the_end_of_a_borrow.md) |
| BR41 | The wait path has one test with a real publisher on another thread, six against cursors written by hand, and zero production callers — `ring_consume` composes `available` itself and leaves waiting to its own caller | `ring_barrier` | n/a — coverage | [lifecycle/002](../lifecycle/002_a_consumer_draining_behind_a_producer.md) |
| BR42 | A stalled `wait_for` costs `spins × len()` atomic loads and the two factors arrive from different call sites, often different crates — 8,192 at the default budget with eight dependencies, and neither signature suggests they multiply | `ring_barrier` | n/a — observation | [non_functional_requirement/001](../non_functional_requirement/001_every_frontier_read_allocates_nothing.md) |
| BR43 | The one-look guarantee is asserted here and implemented in `ring_wait::pause`'s `None` arm two crates away, with neither test referencing the other; a `wait_for` that stopped forwarding `kind` would fail only this one, for a reason its name does not describe | family | n/a — duplication | [non_functional_requirement/002](../non_functional_requirement/002_a_non_blocking_wait_must_look_exactly_once.md) |
| BR44 | `Barrier` and `GatingSet` share no trait, no return type and no signature either could pass to the other — `available` returns `u64`, `headroom` returns `usize` — and the only common code is the fold that lives in neither of them | family | n/a — observation | [pattern/001](../pattern/001_the_borrowed_view_and_the_owned_set.md) |
| BR45 | Quantity, predicate, wait is written four times across the family with the steps named differently each time; `admits` is the one name two crates share and its two signatures take different parameters for different questions | family | n/a — inconsistency | [pattern/002](../pattern/002_the_quantity_the_predicate_and_the_wait.md) |
| BR46 | The assertion that makes the empty-set asymmetry deliberate rather than accidental is only writable because `ring_gating` is a dev-dependency; remove the entry and the library is unchanged while the one statement that the asymmetry is intentional stops being checkable | `ring_barrier` | n/a — unenforced | [pitfall/001](../pitfall/001_the_two_empty_answers_look_like_a_bug.md) |
| BR47 | One test stands between the crate and a change that would compile, pass everything else, and silently cap every batch at the size its consumer asked for — both candidates are a `Seq`, and the correct one is a single expression a refactor toward *return what was asked for* would replace without a second thought | `ring_barrier` | n/a — coverage | [pitfall/002](../pitfall/002_returning_the_request_instead_of_the_frontier.md) |
| BR48 | `available` returns `u64` and `len` returns `usize` and the crate casts nowhere, because the two counts never meet in an expression — a future method relating sequences to dependencies would introduce the family's thirty-ninth cast into the one crate whose type story is that it has none | `ring_barrier` | n/a — observation | [type/001](../type/001_a_u64_distance_and_a_usize_headroom.md) |
| BR49 | There is no `Default`, so the empty barrier — the input class the crate documents most — is written `Barrier::over( &[] )` four times instead; a `Default` would be sound but would advertise a `'static` borrow unlike every other way of building the type | `ring_barrier` | n/a — observation | [type/002](../type/002_the_traits_derived_and_the_traits_absent.md) |
| BR50 | All six mentions of capacity in the source are `//!` prose and none is code, so the guard's comment filter is the entire check — drop it and the guard fails on a correct crate, which is the right failure direction and the opposite of the two guards in this family that cannot fail at all | `ring_barrier` | n/a — observation | [workaround/001](../workaround/001_the_check_that_capacity_stays_out.md) |
| BR51 | The reason `ring_gating` is a dev-dependency is explained in three lines of manifest comment that no rustdoc renders, no test reads and no gate checks, while forty lines of module documentation discuss the two crates' relationship without mentioning it | `ring_barrier` | n/a — doc gap | [workaround/002](../workaround/002_ring_gating_as_a_dev_dependency.md) |
| BR52 | The drain loop is bounded by items received rather than attempts made, which is the only form needing no second arbitrary constant — it cannot be flaky and it can hang, and the failure mode lives entirely in the test runner's timeout | `ring_barrier` | n/a — observation | [lifecycle/002](../lifecycle/002_a_consumer_draining_behind_a_producer.md) |

### Twenty-One Findings and No Reachable Defect

Every entry is `n/a` in the Reachable column, and as with `ring_gating` that is
the headline rather than an accident of scoring. 57 lines of code containing no
loop, no `unsafe`, no destructor, no `Ordering`, no `&mut self` and no cast leave
very little that can be wrong. The one arithmetic operation is a
`saturating_sub` in another crate.

What the findings are about instead is **reach and cost**. The reach: a
constructor with 63 call sites and none in production (BR1), a method chain
whose top hangs entirely from tests (BR3, BR4), a whole API rung nothing in the
family climbs (BR20), an accessor with one caller (BR16). The cost: one atomic
load per dependency per read, multiplied by the wait budget to a measured 8,192
loads for one stalled wait at the default budget with eight dependencies
(BR42) — zero of them allocating (BR13) — on a path the benchmark crate cannot
see.

And three findings are about the *instruments*: a grep that now passes for a
weaker reason than it used to (BR10), a guard whose second half goes silent
under the exact edit it exists to catch (BR14), and an assertion that restates
the body of the method it tests (BR18). Set against `ring_gating`'s G12 — a
concurrency test that cannot fail — this crate's BR15 is the one place the
comparison runs the other way: its safety test asserts an exact equality and
would genuinely fail.

### Severity

| Tier | Findings | Why |
|------|----------|-----|
| Unreachable inconsistency | BR6 | Two of the crate's own methods disagree on one input class, with no test, no doc and no guard |
| Unmeasured hot path | BR13, BR42 | A measured 10,000-read worst case on the crate's only waiting path, zero of them allocating, benchmarked by nothing |
| Blunted instrument | BR10, BR14, BR18 | Three checks that pass for weaker reasons than their names suggest |
| Test that hangs rather than fails | BR19 | A dead producer stalls the suite instead of reporting |
| Duplication | BR2 | The one production reading of this crate is re-derived term-for-term by its only consumer |
| Generality with no caller | BR1, BR3, BR4, BR16, BR20 | Specified, correct, and exercised only by tests — BR20 across four crates at once |
| Coverage gap | BR12 | The one `WaitKind` that sleeps is the one never passed |
| Cross-crate naming | BR11, BR17 | Each side right on its own terms; together they block a shared vocabulary |
| Observation | BR5, BR7, BR8, BR9, BR15, BR21 | Correct as written; each records a property the source states nowhere |
