# Doc Definitions

Module Index for `ring_publish` — every doc definition this crate declares and every
instance under each, in one place.

## Master Doc Definitions Table

| Type | Purpose | Master File | Instances |
|------|---------|-------------|----------:|
| `algorithm/` | One exchange with two contracts, and the only unbounded loop in the family | [algorithm/readme.md](../algorithm/readme.md) | 2 |
| `api/` | Six methods, no caller, and a `Result` whose error is not an error | [api/readme.md](../api/readme.md) | 2 |
| `data_structure/` | Eight live bytes in sixty-four, and the four cursors around them | [data_structure/readme.md](../data_structure/readme.md) | 2 |
| `decisions/` | Two rulings, and the argument neither of them makes | [decisions/readme.md](../decisions/readme.md) | 2 |
| `definition/` | Module Index — every definition and every instance in this crate, in one place | this file | — |
| `integration/` | Seven crates name it, none depends on it, and two changed shape for it | [integration/readme.md](../integration/readme.md) | 2 |
| `invariant/` | One mutation site, and a boundary that is exclusive rather than inclusive | [invariant/readme.md](../invariant/readme.md) | 2 |
| `item/` | Six methods one at a time, and the two that are derivable from a sibling | [item/readme.md](../item/readme.md) | 2 |
| `lifecycle/` | Five states, four transitions, and the one that changes nothing | [lifecycle/readme.md](../lifecycle/readme.md) | 2 |
| `non_functional_requirement/` | What a publication costs, and the three terms the spin's cost is built from | [non_functional_requirement/readme.md](../non_functional_requirement/readme.md) | 2 |
| `pattern/` | A try-and-loop with a fixed target, and eleven named ordering constants | [pattern/readme.md](../pattern/readme.md) | 2 |
| `pitfall/` | Two ways to corrupt the ring with well-typed code | [pitfall/readme.md](../pitfall/readme.md) | 2 |
| `type/` | One cast, two derives, and an overflow sentence that is wrong twice | [type/readme.md](../type/readme.md) | 2 |
| `workaround/` | A `loom` seam with six users, and two documents that say three | [workaround/readme.md](../workaround/readme.md) | 2 |

13 definitions, 26 instances.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_publish/docs
find . -mindepth 1 -maxdepth 1 -type d ! -name definition | wc -l   # 13
find . -name '0*.md' | wc -l                                        # 26
```

## Master Doc Instances Table

| Type | ID | Title | Subject |
|------|----|-------|---------|
| `algorithm/` | 001 | [The Compare-Exchange That Refuses](../algorithm/001_the_compare_exchange_that_refuses.md) | The three positions a `start` can be in, and the family's three exchange sites |
| `algorithm/` | 002 | [A Loop With No Budget](../algorithm/002_a_loop_with_no_budget.md) | The family's only unbounded spin, and its four-step termination argument |
| `api/` | 001 | [Six Methods and No Caller](../api/001_six_methods_and_no_caller.md) | Seven public items, an internal-only call graph, and no caller outside the tests |
| `api/` | 002 | [A Result Whose Error Is Not an Error](../api/002_a_result_whose_error_is_not_an_error.md) | 39 fallible signatures family-wide, and the `RingError` variant that does not exist |
| `data_structure/` | 001 | [One Padded Cursor and Nothing Else](../data_structure/001_one_padded_cursor_and_nothing_else.md) | Four type layers, 56 bytes of padding, and the one `const fn` that is not the constructor |
| `data_structure/` | 002 | [The Four Cursors of the Handshake](../data_structure/002_the_four_cursors_of_the_handshake.md) | The topology, and a wiring bug that passes every other test in the file |
| `decisions/` | 001 | [Refused Rather Than Reordered](../decisions/001_refused_rather_than_reordered.md) | Three designs priced side by side, decided on testability and held by a grep |
| `decisions/` | 002 | [A Plain Spin Rather Than a `WaitKind`](../decisions/002_a_plain_spin_rather_than_a_wait_kind.md) | Three waiting primitives, and the four things a budget's exhaustion could mean |
| `integration/` | 001 | [Ten Crates Name It and None Depends On It](../integration/001_ten_crates_name_it_and_none_depends_on_it.md) | Every edge in and out, and the two-command check that keeps the manifest honest |
| `integration/` | 002 | [The Two Crates That Declined](../integration/002_the_two_crates_that_declined.md) | Two opposite objections, and an escalation condition that has since been met |
| `invariant/` | 001 | [The Frontier Moves Only By Compare-Exchange](../invariant/001_the_frontier_moves_only_by_compare_exchange.md) | One mutation site, and the three `SeqCell` methods that would break it |
| `invariant/` | 002 | [`is_published` Is Exclusive of the Frontier](../invariant/002_is_published_is_exclusive_of_the_frontier.md) | 320 assertions over a moving boundary, exclusive by force rather than by taste |
| `item/` | 001 | [The Three Readings of the Cursor](../item/001_the_three_readings_of_the_cursor.md) | `cursor`/`published`/`is_published`, and six writable accessors family-wide |
| `item/` | 002 | [The Two Publications](../item/002_the_two_publications.md) | Two contracts, four `# Panics` sections, and one branch repo-wide on the `Result` |
| `lifecycle/` | 001 | [A Slot From Claim to Visibility](../lifecycle/001_a_slot_from_claim_to_visibility.md) | Five states, and the `AtomicUsize` the loom model builds because nothing else sees one |
| `lifecycle/` | 002 | [The Four-Operation Handshake](../lifecycle/002_the_four_operation_handshake.md) | Four operations, three clauses, and the one checked exhaustively under concurrency |
| `non_functional_requirement/` | 001 | [What a Publication Costs](../non_functional_requirement/001_what_a_publication_costs.md) | Atomics per method, and five benchmark candidates that reach none of them |
| `non_functional_requirement/` | 002 | [What the Spin Costs](../non_functional_requirement/002_what_the_spin_costs.md) | One iteration exactly, and three terms this crate cannot see |
| `pattern/` | 001 | [Try-and-Loop Over Compare-Exchange](../pattern/001_try_and_loop_over_compare_exchange.md) | A fallible primitive with the family's only blocking twin, and the loop census |
| `pattern/` | 002 | [The Named Ordering Constant](../pattern/002_the_named_ordering_constant.md) | Eleven constants, seven names, four values, zero edges between the shared pairs |
| `pitfall/` | 001 | [Publishing a Range You Never Claimed](../pitfall/001_publishing_a_range_you_never_claimed.md) | Four break modes, and the type that would enforce three of them |
| `pitfall/` | 002 | [Conflating the Two Cursors](../pitfall/002_conflating_the_two_cursors.md) | The founding hazard, five cursor-holding structs, and three designs' answers |
| `type/` | 001 | [A `Seq`, a `usize`, and the One Cast](../type/001_a_seq_a_usize_and_the_one_cast.md) | The whole type surface, the family's 42 casts, and the one made here |
| `type/` | 002 | [Two Derives, and the Ones That Are Absent](../type/002_two_derives_and_the_ones_that_are_absent.md) | `Debug` on 88 of 89, and `Sync` obtained by composition rather than by `unsafe` |
| `workaround/` | 001 | [The `loom` Seam and Its Only User](../workaround/001_the_loom_seam_and_its_only_user.md) | A four-line atomic switch, six crates that touch it, and two documents saying three |
| `workaround/` | 002 | [Four Dev-Dependencies That Look Like a Cycle](../workaround/002_four_dev_dependencies_that_look_like_a_cycle.md) | Zero reverse edges in the workspace, both closures, three scaffolding corrections |

## Findings

Fifty-two, each verified by a command whose output is quoted in its instance.

| ID | Finding | Subject | Reachable | Where |
|----|---------|---------|-----------|-------|
| PB1 | Six crates have no dependents and five are tooling or explicitly optional; this is the only tiered write-path primitive among them, while its three Tier 5 siblings have two, two, and one | family | n/a — unadopted | [integration/001](../integration/001_ten_crates_name_it_and_none_depends_on_it.md) |
| PB2 | Ten crates name it in prose and none depends on it; two of the ten changed shape *for* it — `ring_barrier` opened a signature, `ring_atomic` built a `loom` seam — for a test that lives in a crate neither depends on | family | n/a — unadopted | [integration/001](../integration/001_ten_crates_name_it_and_none_depends_on_it.md) |
| PB3 | The four dev-dependencies are the reached-test, not the crate; the check that keeps the sections honest is deliberately two commands, because a single-command version reports four false positives on exactly the crate whose distinguishing feature is which section its edges are in | `ring_publish` | n/a — observation | [integration/001](../integration/001_ten_crates_name_it_and_none_depends_on_it.md) |
| PB4 | This crate's own module documentation named `ring_mpsc` before that crate existed, and [`decisions/001`](../decisions/001_refused_rather_than_reordered.md) quotes it back as the reason the mechanism question was never open: *"The dependency answers it against itself"* | `ring_publish` | n/a — observation | [integration/002](../integration/002_the_two_crates_that_declined.md) |
| PB5 | [`decisions/001`](../decisions/001_refused_rather_than_reordered.md) set an escalation condition naming `ring_core` as the next possible adopter; `ring_core` has since been implemented and does not adopt, and the decision that ruling said would become necessary is not written | `ring_core` | n/a — drift | [integration/002](../integration/002_the_two_crates_that_declined.md) |
| PB6 | The declining crate reproduced the constant it declined: same name, same value, near-identical justifying prose including one sentence verbatim — and *its* copy is asserted by a doctest where this crate's is covered only by two hand-run manual checks | `ring_mpsc` | n/a — coverage | [integration/002](../integration/002_the_two_crates_that_declined.md) |
| PB7 | Three genuine compare-exchange sites exist in the family; the two in `ring_claim` retry against a moved target and feed the failure value back as the next input, while this one retries against a fixed target and discards it — the difference between a contest and a turn-gate on the same primitive | family | n/a — observation | [algorithm/001](../algorithm/001_the_compare_exchange_that_refuses.md) |
| PB8 | The failure value is asserted against a literal five times and bound zero times; two documents describe it as *"what to try against next"*, borrowing `compare_exchange`'s idiom, but `start` is the caller's own claim and cannot be substituted | `ring_publish` | n/a — drift | [algorithm/001](../algorithm/001_the_compare_exchange_that_refuses.md) |
| PB9 | `publish` is the only bare unbounded loop in any `ring_*` library; it is correct because of *what* it waits on — a predecessor committed to a slot write it cannot abandon — not because of how long the wait is | family | n/a — observation | [algorithm/002](../algorithm/002_a_loop_with_no_budget.md) |
| PB10 | Every call site of every method is a test in this crate; only two items have a caller inside the library, and both are a method calling its own simpler sibling | `ring_publish` | n/a — unadopted | [api/001](../api/001_six_methods_and_no_caller.md) |
| PB11 | The one silently-droppable return value is derivable from its own arguments, so its missing `#[ must_use ]` is correct; the annotation pattern tracks the *error* payload, and a future `publish` that grew a failure mode would need one with nothing to notice its absence | `ring_publish` | n/a — observation | [api/001](../api/001_six_methods_and_no_caller.md) |
| PB12 | 21 of the family's 39 fallible signatures return `RingError` and one returns a `Seq`; this crate is the only one that names `RingError` without either declaring or importing it, in the sentence explaining why it does not | family | n/a — observation | [api/002](../api/002_a_result_whose_error_is_not_an_error.md) |
| PB13 | 64 bytes of struct for 8 bytes of state, and the padding is against what the *caller* puts next to it rather than anything inside `Publisher`; one publisher per ring makes the trade obviously right, which is why [`decisions/001`](../decisions/001_refused_rather_than_reordered.md) rejected the identical type for a per-slot stamp array | `ring_publish` | n/a — observation | [data_structure/001](../data_structure/001_one_padded_cursor_and_nothing_else.md) |
| PB14 | The crate's one `const fn` is an accessor, not the constructor; `new` could be `const` in an ordinary build but would have to carry `ring_cursor`'s `cfg(loom)` duplication down into this crate to stay so | `ring_publish` | n/a — observation | [data_structure/001](../data_structure/001_one_padded_cursor_and_nothing_else.md) |
| PB15 | A consumer holding a cursor the producer does not read passes every other test in the file and gates nothing; the guard is an address comparison because two cursors are value-equal until the first commit, which is exactly when a value assertion would run | `ring_publish` | n/a — coverage | [data_structure/002](../data_structure/002_the_four_cursors_of_the_handshake.md) |
| PB16 | `ring_claim` justifies the visibility of two public items by naming `ring_publish` as their consumer; this crate calls neither, and given the dependency direction never can | `ring_claim` | n/a — drift | [data_structure/002](../data_structure/002_the_four_cursors_of_the_handshake.md) |
| PB17 | The rejection is enforced by a grep rather than trusted to prose, and the comment-stripping filter is load-bearing: this crate's own module documentation argues about bitmaps and contiguity at length, so an unfiltered grep would report the rejected designs as implementations | `ring_publish` | n/a — observation | [decisions/001](../decisions/001_refused_rather_than_reordered.md) |
| PB18 | Three of 33 crates contain a waiting primitive; the other two take a strategy or a budget and this one takes neither, and `ring_poll` declines `ring_wait` on exactly opposite grounds — it must never block, this crate must always eventually succeed | family | n/a — observation | [decisions/002](../decisions/002_a_plain_spin_rather_than_a_wait_kind.md) |
| PB19 | The invariant is enforced by absence and checked by grep, because a `store`-based `publish` would pass every behavioural test in the suite under a single producer and most of them under several | `ring_publish` | n/a — observation | [invariant/001](../invariant/001_the_frontier_moves_only_by_compare_exchange.md) |
| PB20 | The boundary is asserted exhaustively twice, and both tests are kept because they fail differently: one names the boundary in its message, the other localises an arbitrary inconsistency with `"at frontier N, asking about M"` | `ring_publish` | n/a — observation | [invariant/002](../invariant/002_is_published_is_exclusive_of_the_frontier.md) |
| PB21 | Ten of thirteen `cursor()` call sites are one expression repeated to the character; the accessor has exactly one real use and the crate never abstracted it, because the helper that would collapse them can only live in the test file and the file's three helpers stop short of an eleventh | `ring_publish` | n/a — duplication | [item/001](../item/001_the_three_readings_of_the_cursor.md) |
| PB22 | Six cursor accessors across five crates all hand out the same mutable-through-shared handle, and there is no read-only cursor type anywhere in the family; the hole is not local to this crate, and nothing checks it in any crate | family | n/a — unenforced | [item/001](../item/001_the_three_readings_of_the_cursor.md) |
| PB23 | The family writes 49 `# Errors` sections and 4 `# Panics`; two say "Never", and the difference is that a bounded loop's "Never" is complete in one sentence while this one needs three, because the honest answer is *"no — it does something worse"* | family | n/a — observation | [item/002](../item/002_the_two_publications.md) |
| PB24 | One site repo-wide branches on `try_publish`'s `Result`, and it is `publish`, the method whose purpose is to make branching unnecessary; of eighteen sites, fifteen assert the value and two discard it | `ring_publish` | n/a — observation | [item/002](../item/002_the_two_publications.md) |
| PB25 | Of the four transitions, the one this crate exists to protect is the only one that changes no shared state; states 2 and 3 are genuinely indistinguishable within the tiered stack, and the two crates that *can* tell them apart per slot are the two that built stamps instead of using this crate | family | n/a — observation | [lifecycle/001](../lifecycle/001_a_slot_from_claim_to_visibility.md) |
| PB26 | So the loom model constructs its own observation instrument out of a bare `AtomicUsize`, with a `0xABC` sentinel chosen against two specific failures: `0` is indistinguishable from never-written, and `1` could collide with an initial value | `ring_publish` | n/a — observation | [lifecycle/001](../lifecycle/001_a_slot_from_claim_to_visibility.md) |
| PB27 | Three files family-wide mention a numbered clause, and this is the only crate that pins all three of its criterion's clauses to specific assertions by number, at the assertion site; nothing mechanical checks that a clause has an assertion | family | n/a — observation | [lifecycle/002](../lifecycle/002_the_four_operation_handshake.md) |
| PB28 | The criterion's *"asserted over every interleaving"* reads as covering all three clauses; the model checks clause 1 exhaustively, clause 3 at every point the drain could reach it, and clause 2 once per execution at a quiescent point | `ring_publish` | n/a — coverage | [lifecycle/002](../lifecycle/002_the_four_operation_handshake.md) |
| PB29 | The family built a benchmark crate with five candidates and nine path dependencies, and this crate is in none of them; there is no `benches/` directory anywhere in the 33 | `ring_bench` | n/a — coverage | [non_functional_requirement/001](../non_functional_requirement/001_what_a_publication_costs.md) |
| PB30 | 25 of 33 crates name no `std::` path and three declare `#![no_std]`; absence of the path is not compatibility, since `Vec`/`Box`/`String` arrive through the prelude unprefixed, and the property is settled by compiling for a bare-metal target rather than by grepping | family | n/a — observation | [non_functional_requirement/001](../non_functional_requirement/001_what_a_publication_costs.md) |
| PB31 | The iteration count is a function of three terms and not one is a parameter, a field, or an argument; the worst case is a peer's payload write rather than a queue depth, because *k* predecessors write concurrently | `ring_publish` | n/a — observation | [non_functional_requirement/002](../non_functional_requirement/002_what_the_spin_costs.md) |
| PB32 | Seven crates' tests look at a clock and this one names no clock type at all, which is correct: with no budget there is nothing a clock could assert, and any duration promised would be a claim about caller code | family | n/a — observation | [non_functional_requirement/002](../non_functional_requirement/002_what_the_spin_costs.md) |
| PB33 | Thirteen `try_*` methods exist across five crates and twelve are the only form their operation has; the three blocking-sounding `push` methods belong to types with no `try_` twin and none of them loops | family | n/a — observation | [pattern/001](../pattern/001_try_and_loop_over_compare_exchange.md) |
| PB34 | Eleven named ordering constants exist in seven crates; four names appear twice and no dependency edge connects either member of any pair, so the shared vocabulary is convention held in prose, not in code — nothing imports another crate's `PUBLISH`, asserts they are equal, or would notice if one changed | family | n/a — observation | [pattern/002](../pattern/002_the_named_ordering_constant.md) |
| PB35 | `ring_claim::Claim` is exactly the two values `publish` takes, carries a `must_use` naming this crate's failure from the other side, and is not on the signature; the dependency-cycle objection is false and this crate's own manifest says so | `ring_claim` | n/a — inconsistency | [pitfall/001](../pitfall/001_publishing_a_range_you_never_claimed.md) |
| PB36 | `Claim` is `Copy`, and all seven of its accessors take `self` by value, so this crate's own tests depend on the derive to compile; making it linear is a signature change to all seven, not just a dropped derive | `ring_claim` | n/a — observation | [pitfall/001](../pitfall/001_publishing_a_range_you_never_claimed.md) |
| PB37 | Five cursor fields across four structs store a `PaddedCursor`, the only aggregate among them aggregates *consumers*, and `CursorPair` pairs producer with consumer; no type in the family holds the claimed and published pair | family | n/a — observation | [pitfall/002](../pitfall/002_conflating_the_two_cursors.md) |
| PB38 | Three designs, three answers: two cursors and caller discipline here, `Claimer` plus per-slot stamps in `ring_mpsc`, and nothing at all in `ring_spsc` where one producer collapses the distinction | family | n/a — observation | [pitfall/002](../pitfall/002_conflating_the_two_cursors.md) |
| PB39 | The family casts 42 times across 14 crates; `ring_publish` casts once, and only in the widening direction, while eleven `u64 as usize` casts elsewhere would truncate on a 32-bit target | family | n/a — observation | [type/001](../type/001_a_seq_a_usize_and_the_one_cast.md) |
| PB40 | `advanced_by`, the method `try_publish` calls, documents no overflow behaviour, while its structurally identical sibling `next` documents it and gets it wrong twice: it names saturation, which Rust never does, and disclaims the wrapping that release builds actually perform | `ring_types` | **wrong doc** | [type/001](../type/001_a_seq_a_usize_and_the_one_cast.md) |
| PB41 | 88 of the family's 89 derive attributes include `Debug`, driven by a lint rather than a convention; the one exception hand-writes `Debug` instead; only three are exactly `Debug, Default`, and two of the three are this crate and the type it wraps | family | n/a — observation | [type/002](../type/002_two_derives_and_the_ones_that_are_absent.md) |
| PB42 | Every absent trait is absent because `AtomicU64` implements none of them, not because of a decision here; a *manual* `Clone` would compile and would reach [`pitfall/002`](../pitfall/002_conflating_the_two_cursors.md)'s corruption by a fresh route | `ring_publish` | n/a — unenforced | [type/002](../type/002_two_derives_and_the_ones_that_are_absent.md) |
| PB43 | Two documents name this crate as the seam's only user and both are stale by three crates: `Cargo.toml:226` lists three readers where six touch it, and `ring_atomic:59-60` names `handshake_test.rs` as *"what uses it"* where four loom models now exist | `ring_atomic` | **misleading doc** | [workaround/001](../workaround/001_the_loom_seam_and_its_only_user.md) |
| PB44 | `§ P6` checks two manifests and five declare `loom`, so the property *"loom never reaches a shipped build"* is established family-wide by nothing; widening the check would be the wrong fix | family | n/a — unenforced | [workaround/001](../workaround/001_the_loom_seam_and_its_only_user.md) |
| PB45 | Not one manifest in the workspace declares `ring_publish`; the only manifest that ever did records its removal and the reason, so the acyclicity claim is a present fact with zero candidates rather than a promise | family | n/a — observation | [workaround/002](../workaround/002_four_dev_dependencies_that_look_like_a_cycle.md) |
| PB46 | Three manifests were scaffolded before their implementations and corrected after; two of the three corrections remove `ring_seqno`, for the same reason each time | family | n/a — observation | [workaround/002](../workaround/002_four_dev_dependencies_that_look_like_a_cycle.md) |
| PB47 | The `advanced_by` call precedes the exchange with no branch between them, so a refused publication computes `start + len` and discards it — putting `advanced_by`'s undocumented overflow behaviour on the path that was going to return `Err` | `ring_publish` | n/a — observation | [algorithm/001](../algorithm/001_the_compare_exchange_that_refuses.md) |
| PB48 | Six public functions, five `&self` receivers and no `&mut self` at all, against twelve of thirty-three crates that carry one; the shape is what lets a publisher be shared, and `Sync`, the property it exists to support, is asserted nowhere | `ring_publish` | n/a — unenforced | [api/001](../api/001_six_methods_and_no_caller.md) |
| PB49 | The rejection defers per-slot availability to `ring_mpsc` by naming a bitmap; `ring_mpsc` says `bitmap` zero times and `stamp` fifty-two, and puts the mechanism in a heading — the second forecast in this crate overtaken by the crate it named | `ring_mpsc` | n/a — drift | [decisions/001](../decisions/001_refused_rather_than_reordered.md) |
| PB50 | `Acquire` appears twice in `src/lib.rs` and both are doc lines, so the crate that calls the Release/Acquire pairing *the entire happens-before edge* supplies one half and can only describe the other | `ring_publish` | n/a — observation | [invariant/001](../invariant/001_the_frontier_moves_only_by_compare_exchange.md) |
| PB51 | `is_published` — the question this crate is graded on — has thirteen call sites, all in a file gated `#![ cfg( not( loom ) ) ]`, so it is exercised where interleavings are sampled and never where they are enumerated | `ring_publish` | n/a — coverage | [invariant/002](../invariant/002_is_published_is_exclusive_of_the_frontier.md) |
| PB52 | All three genuine compare-exchange sites pair a crate-local success constant with `ring_cursor`'s imported `GATING`; no site in the family names both orderings from one place, and `GATING`'s single declaration is why the failure half cannot drift | family | n/a — observation | [pattern/002](../pattern/002_the_named_ordering_constant.md) |

### Fifty-Two Findings and Two Wrong Sentences

Fifty entries are `n/a` in the Reachable column and two are not, and both
exceptions are prose rather than code: `ring_types` documents an overflow
behaviour Rust does not have and disclaims the one release builds perform (PB40),
and `ring_atomic` and the workspace manifest both still describe the `loom` seam
as having one user when six crates touch it (PB43). Neither is in this crate.

That the crate itself yields no reachable defect is not luck. It is 222 lines
with one struct, one field, six methods, one cast — widening — one `unsafe`-free
mutation site, and no `std::` path. Its entire behaviour is a compare-exchange
and a loop around it.

What the findings are about instead is **adoption, cost, and instruments**.

The adoption: seven crates name it in prose and none depends on it (PB2), six
crates in the family have no dependents and this is the only load-bearing one
among them (PB1), every call site of every method is one of its own tests (PB10),
and the two crates that would have been its consumers each built something else —
`ring_spsc` because one producer collapses the problem, `ring_mpsc` because
per-slot stamps answer it differently (PB38). `ring_core` was named as
the next possible adopter; `ring_core` exists now and does not adopt, and the
follow-up decision that question required is unwritten (PB5).

The cost: a benchmark crate with five candidates and nine path dependencies
reaches none of this code, and the family has no `benches/` directory at all
(PB29), so every figure in `non_functional_requirement/` is a count read off the
source. The one cost that is not a count — the spin's iteration total — depends on
three terms, none of which is a parameter, a field, or an argument (PB31), and the
crate names no clock type, which is correct because with no budget there is
nothing a clock could assert (PB32).

The instruments: a grep that keeps three rejected designs out of the
implementation, whose comment-stripping filter is load-bearing because the module
doc argues for the rejected designs at length (PB17); an invariant enforced by the
*absence* of two method calls, because a `store`-based `publish` would pass the
whole behavioural suite (PB19); a loom model that builds an `AtomicUsize` with a
`0xABC` sentinel because nothing in the tiered stack can distinguish written from
unwritten per slot (PB26); and a manual check on two manifests standing in for a
property five crates could break (PB44).

### Severity

| Tier | Findings | Why |
|------|----------|-----|
| Wrong documentation | PB40, PB43 | Two sentences in shipped source that are false as written — one about overflow, one about who uses the `loom` seam |
| Latent hazard | PB22, PB35, PB36, PB42 | Four ways well-typed code reaches corruption: a writable cursor handed to a reader, a range never claimed, a `Copy` claim reused, a manual `Clone` |
| Drift | PB5, PB8, PB16 | Three statements true when written and overtaken since — an escalation condition met, an idiom borrowed from a different operation, a seam justified by a caller that cannot exist |
| Coverage gap | PB6, PB15, PB28 | A constant covered by hand-run checks where its copy is doctested, a wiring bug that passes every other test in its file, and a liveness clause the criterion's wording does not distinguish |
| Unmeasured | PB29, PB31, PB32 | Every figure in this corpus is a count and none is a measurement, for a cost whose three terms the crate cannot see |
| Unadopted | PB1, PB2, PB10 | The crate is complete, tested, and named by seven neighbours, and nothing links against it |
| Blunted instrument | PB44 | A two-manifest check standing in for a property five crates can break |
| Duplication | PB21 | Ten call sites that are one expression to the character, with nowhere to put the helper |
| Observation | PB3, PB4, PB7, PB9, PB11, PB12, PB13, PB14, PB17, PB18, PB19, PB20, PB23, PB24, PB25, PB26, PB27, PB30, PB33, PB34, PB37, PB38, PB39, PB41, PB45, PB46 | Correct as written; each records a property the source states nowhere |
