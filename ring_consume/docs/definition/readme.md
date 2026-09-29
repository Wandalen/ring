# Doc Definitions

Module Index for `ring_consume` — every doc definition this crate declares and every
instance under each, in one place.

## Master Doc Definitions Table

| Type | Purpose | Master File | Instances |
|------|---------|-------------|----------:|
| `algorithm/` | Three steps with no branch, and a guard whose refusal costs what acceptance costs | [algorithm/readme.md](../algorithm/readme.md) | 2 |
| `api/` | Sixteen items, every `must_use` deliberately silent, and a contract covering safety not fitness | [api/readme.md](../api/readme.md) | 2 |
| `data_structure/` | Sixteen bytes and twenty-four, and the borrow that is the wiring rather than an optimisation | [data_structure/readme.md](../data_structure/readme.md) | 2 |
| `decisions/` | Two rulings that each move a guarantee into an unbacked premise | [decisions/readme.md](../decisions/readme.md) | 2 |
| `definition/` | Module Index — every definition and every instance in this crate, in one place | this file | — |
| `integration/` | Four edges in, none out, and eight `Barrier` methods nothing calls | [integration/readme.md](../integration/readme.md) | 2 |
| `invariant/` | Two properties held across three crates, and a guard on one of two doors | [invariant/readme.md](../invariant/readme.md) | 2 |
| `item/` | Fourteen methods, three shaped from outside the crate, and the widest hole documented as an accessor | [item/readme.md](../item/readme.md) | 2 |
| `lifecycle/` | Four states with one unobservable, and a consumer disposable enough that the suite depends on it | [lifecycle/readme.md](../lifecycle/readme.md) | 2 |
| `non_functional_requirement/` | Four stated properties, three measured false, one enforced and unbreakable | [non_functional_requirement/readme.md](../non_functional_requirement/readme.md) | 2 |
| `pattern/` | A range type invented three times, and a protocol whose halves invert exactly | [pattern/readme.md](../pattern/readme.md) | 2 |
| `pitfall/` | Two traps laid by reasonable expectations rather than by bad code | [pitfall/readme.md](../pitfall/readme.md) | 2 |
| `type/` | A maximal `const` surface, and a withheld `Copy` its own accessors bypass | [type/readme.md](../type/readme.md) | 2 |
| `workaround/` | Three inherited constraints, one of which cost an allocation per read until it was removed | [workaround/readme.md](../workaround/readme.md) | 2 |

13 definitions, 26 instances.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_consume/docs
find . -mindepth 1 -maxdepth 1 -type d ! -name definition | wc -l   # 13
find . -name '0*.md' | wc -l                                        # 26
grep -rho '^### CN[0-9]*' . | wc -l                                 # 55
```

Live output:

```
13
26
55
```

## Master Doc Instances Table

| Type | ID | Title | Subject |
|------|----|-------|---------|
| `algorithm/` | 001 | [Position, Frontier, Pending](../algorithm/001_position_frontier_pending.md) | Three steps, no branch, no loop, and a clamp that bounds length and never start |
| `algorithm/` | 002 | [The Two-Sided Guard](../algorithm/002_the_two_sided_guard.md) | Both failure conditions documented, an inclusive upper bound, and a refusal that costs what acceptance costs |
| `api/` | 001 | [Sixteen Public Items](../api/001_sixteen_public_items.md) | Three unannotated methods each correctly unannotated, and eleven bare `must_use` where the family has twelve messaged |
| `api/` | 002 | [The Two Commits](../api/002_the_two_commits.md) | A contract stating safety and never fitness, and a `Result` that cannot fire where it is meant to be used |
| `data_structure/` | 001 | [Sixteen and Twenty-Four](../data_structure/001_sixteen_and_twenty_four.md) | Eight measured types, the third 16-byte range in the family, and a consumer a fifth the size of a claimer |
| `data_structure/` | 002 | [Two Borrows and No Owned State](../data_structure/002_two_borrows_and_no_owned_state.md) | Why an owned cursor breaks the mechanism, and auto-derived `Send`/`Sync` named nowhere |
| `decisions/` | 001 | [Two Calls, Not One](../decisions/001_two_calls_not_one.md) | The guard design named and priced, and the window no signature could close |
| `decisions/` | 002 | [Plain Stores Rather Than Compare-Exchange](../decisions/002_plain_stores_rather_than_compare_exchange.md) | An exact atomic mirror of the write half, and a premise enforced by prose |
| `integration/` | 001 | [Four Edges In and None Out](../integration/001_four_edges_in_and_none_out.md) | The whole graph, two Tier 5 primitives with no library dependents, and a `ring_mpsc` doc past its decision |
| `integration/` | 002 | [Eight Methods and the One That Is Called](../integration/002_eight_methods_and_the_one_that_is_called.md) | `Barrier`'s nine, the one this crate uses, and the chain lengthened for a method nobody calls |
| `invariant/` | 001 | [Never Reads Past What Was Published](../invariant/001_never_reads_past_what_was_published.md) | An invariant spread over three crates and named together nowhere, and the one threaded test that asserts it |
| `invariant/` | 002 | [The Cursor Only Moves Forward](../invariant/002_the_cursor_only_moves_forward.md) | A guard on one of two doors, and a manual check naming a test that does not exist |
| `item/` | 001 | [The Six of a Run](../item/001_the_six_of_a_run.md) | A public constructor that makes the type guarantee nothing, and an `end` computed rather than stored |
| `item/` | 002 | [The Eight of a Consumer](../item/002_the_eight_of_a_consumer.md) | `cursor()` as the crate's widest hole, and `barrier()`'s shape forced by a test in a fourth crate |
| `lifecycle/` | 001 | [A Sequence From Published to Committed](../lifecycle/001_a_sequence_from_published_to_committed.md) | Three observable transitions and the load-bearing one that is not, and an irreversible act called irreversible nowhere |
| `lifecycle/` | 002 | [The Consumer Over a Ring's Life](../lifecycle/002_the_consumer_over_a_rings_life.md) | Disposability the 900-case sweep structurally requires, and a wrap point four crates assume away |
| `non_functional_requirement/` | 001 | [What the Read Path Costs](../non_functional_requirement/001_what_the_read_path_costs.md) | An allocation per read measured then removed, a chain carrying `std` and a blocking loop, and two enforced properties of four |
| `non_functional_requirement/` | 002 | [Eleven Constants and the One That Is Shared](../non_functional_requirement/002_eleven_constants_and_the_one_that_is_shared.md) | The ordering-constant census, and four names declared twice across crates with no edge between them |
| `pattern/` | 001 | [The Half-Open Range as a Value](../pattern/001_the_half_open_range_as_a_value.md) | Three independent 16-byte ranges agreeing on six decisions and diverging on the length type |
| `pattern/` | 002 | [Read Then Report](../pattern/002_read_then_report.md) | Neither report operation taking the value its ask produced, and an exact fallibility inversion |
| `pitfall/` | 001 | [`commit_available` Does Not Call `commit`](../pitfall/001_commit_available_does_not_call_commit.md) | A duplicated store correct for an arithmetic reason, and an idle poll that still writes |
| `pitfall/` | 002 | [The Empty Barrier Is the Case Nobody Measured](../pitfall/002_the_empty_barrier_is_the_case_nobody_measured.md) | The one configuration that reports clean, and why the cost survived in two crates |
| `type/` | 001 | [`Available`'s Const Surface](../type/001_availables_const_surface.md) | A maximal `const` surface with a provable exception, and the niche a public field forecloses |
| `type/` | 002 | [The Lifetime on `Consumer`](../type/002_the_lifetime_on_consumer.md) | One lifetime unifying two borrows for free, and a withheld `Copy` its own accessors bypass |
| `workaround/` | 001 | [Five Functions, None `const`](../workaround/001_five_functions_none_const.md) | Four const-able bodies left non-`const`, proven by compilation, and `.0` as a `Seq`'s only exit |
| `workaround/` | 002 | [A Vector That Changed a Slice's Type](../workaround/002_a_vector_to_change_a_slices_type.md) | An allocation that computed nothing, and which of the three named ways out actually shipped |

## Findings

Fifty-five, each verified by a command whose output is quoted in its instance.

| ID | Finding | Subject | Reachable | Where |
|----|---------|---------|-----------|-------|
| CN1 | The crate's entire arithmetic contribution is one call to `ring_seqno::pending`, from a crate it names exactly once; everything else is delegation to `ring_barrier` and `ring_cursor` | `ring_consume` | n/a — observation | [integration/001](../integration/001_four_edges_in_and_none_out.md) |
| CN2 | `ring_consume` and `ring_claim` are the two Tier 5 primitives with zero library dependents, so nothing outside their own suites and one handshake test in a third crate exercises them | family | n/a — coverage | [integration/001](../integration/001_four_edges_in_and_none_out.md) |
| CN3 | `ring_mpsc` documents having dropped both halves of the handshake, and the surrounding code has since moved past the decision the doc describes | `ring_mpsc` | n/a — drift | [integration/001](../integration/001_four_edges_in_and_none_out.md) |
| CN4 | Eight of `Barrier`'s nine public methods have no library caller — seven have no caller outside their own crate's tests either, `over` being reached from two sibling test files; `ring_consume`, the only library importer, calls `frontier()` and nothing else | `ring_barrier` | n/a — coverage | [integration/002](../integration/002_eight_methods_and_the_one_that_is_called.md) |
| CN5 | The chain is one crate longer than `ring_claim`'s and carries `std::` and a blocking loop, both entering through `ring_wait` solely because `Barrier::wait_for` exists — a method with no library caller anywhere | `ring_wait` | n/a — doc gap | [integration/002](../integration/002_eight_methods_and_the_one_that_is_called.md) |
| CN6 | The rejected single-call-returning-a-guard design is named and priced in the module doc, and the argument — only the caller knows when the read finished — is the crate's primary asset | `ring_consume` | n/a — observation | [decisions/001](../decisions/001_two_calls_not_one.md) |
| CN7 | The window between `available` and `commit` is explicit, entirely unguarded, and unguardable by any signature this crate could offer; committing before reading is well-typed and silent | `ring_consume` | **latent hazard** | [decisions/001](../decisions/001_two_calls_not_one.md) |
| CN8 | The two halves are exact atomic opposites — `ring_claim` 2 CAS / 0 store / 1 load, `ring_consume` 0 CAS / 2 store / 1 load — the family's cleanest structural fact, recorded in neither crate | family | n/a — observation | [decisions/002](../decisions/002_plain_stores_rather_than_compare_exchange.md) |
| CN9 | Single-consumer is a premise the plain stores depend on and prose is its only enforcement; `Consumer::new` is public and its arguments are a shared reference and a `Copy` value | `ring_consume` | **latent hazard** | [decisions/002](../decisions/002_plain_stores_rather_than_compare_exchange.md) |
| CN10 | I1 rests on one `map_or`, on `slowest` being a minimum, and on `distance_to` saturating — three facts in three crates, none of which names the other two | `ring_consume` | n/a — unenforced | [invariant/001](../invariant/001_never_reads_past_what_was_published.md) |
| CN11 | One `thread::scope` test asserts that a consumer never reads past the frontier, and it is the only thing that does | `ring_consume` | n/a — coverage | [invariant/001](../invariant/001_never_reads_past_what_was_published.md) |
| CN12 | `commit`'s lower bound is the only enforcement of monotonicity and it guards one of two doors; `cursor()` returns a `&PaddedCursor` whose `SeqCell::store` takes `&self`, so one line of safe code moves the cursor backwards | `ring_consume` | **latent hazard** | [invariant/002](../invariant/002_the_cursor_only_moves_forward.md) |
| CN13 | `tests/manual/readme.md § N6` names `commit_accepts_exactly_the_reachable_range`, which does not exist; the real test is `the_accepted_commits_are_exactly_the_available_range_inclusive_of_both_ends` at `consume_test.rs:261`, and the check's grep still passes, so it can report 6/6 forever | `ring_consume` | **wrong doc** | [invariant/002](../invariant/002_the_cursor_only_moves_forward.md) |
| CN14 | The whole computation is straight-line — no loop, no retry, no back-off — because a consumer has no competitor by construction; the absence of the loop is the single-consumer premise made visible in the control flow | `ring_consume` | n/a — observation | [algorithm/001](../algorithm/001_position_frontier_pending.md) |
| CN15 | `available_up_to` clamps the length and never the start, so it bounds how much is offered and not from where; the name suggests a bounded position | `ring_consume` | n/a — doc gap | [algorithm/001](../algorithm/001_position_frontier_pending.md) |
| CN16 | `commit`'s `# Errors` documents both failure conditions, which the write half's equivalent does not — the family's model for documenting an overloaded error variant | `ring_consume` | n/a — observation | [algorithm/002](../algorithm/002_the_two_sided_guard.md) |
| CN17 | A refused commit performs every load an accepted one does, because the guard needs `available()` to evaluate; the check is not a cheap early-out and nothing says so | `ring_consume` | n/a — doc gap | [algorithm/002](../algorithm/002_the_two_sided_guard.md) |
| CN18 | The three methods with no `must_use` are each covered by their return type — `Result`, the side effect, and `Iterator` — so the count invites a false finding and none is available | `ring_consume` | n/a — observation | [api/001](../api/001_sixteen_public_items.md) |
| CN19 | All eleven `must_use` here are unmessaged, correctly: an `Available` is a permission, not an obligation, where every one of the family's twelve messaged annotations names a cost of letting the value go unused | `ring_consume` | n/a — observation | [api/001](../api/001_sixteen_public_items.md) |
| CN20 | The two commits' contracts state when each is safe and never when each is right; `commit_available` after a partial read discards the remainder silently, which is the crate's most likely real-world defect | `ring_consume` | **latent hazard** | [api/002](../api/002_the_two_commits.md) |
| CN21 | `commit` cannot return `Err` for a `through` that came from `available()`, so its `Result` costs every correct call site an `unwrap` and benefits only incorrect ones; its doctest demonstrates the two refusals and never the idiom | `ring_consume` | **misleading doc** | [api/002](../api/002_the_two_commits.md) |
| CN22 | `Available` is the third independently-written 16-byte half-open range in the family, agreeing on layout, `Copy`, derived `end`, and an iterator, and diverging only on length type | family | n/a — duplication | [data_structure/001](../data_structure/001_sixteen_and_twenty_four.md) |
| CN23 | A `Consumer` is 24 bytes against a `Claimer`'s 128, and the entire difference is that the producer owns its cursor and the consumer borrows one | family | n/a — observation | [data_structure/001](../data_structure/001_sixteen_and_twenty_four.md) |
| CN24 | The borrow is the mechanism, not an optimisation: an owned cursor is a different address, so the producer would gate on a cursor nobody advances and the ring would fill and stop. The module doc argues it, uniquely in the family, on mechanism grounds | `ring_consume` | n/a — observation | [data_structure/002](../data_structure/002_two_borrows_and_no_owned_state.md) |
| CN25 | `Send` and `Sync` are auto-derived, load-bearing, relied on by the `thread::scope` tests that would not compile without them, and named nowhere in a crate whose purpose is cross-thread reading | `ring_consume` | n/a — unenforced | [data_structure/002](../data_structure/002_two_borrows_and_no_owned_state.md) |
| CN26 | `Available::new` is public, so the type guarantees nothing about provenance — a caller may construct any range and pass it anywhere; correct here, and the opposite of what a public constructor on a concurrency type usually implies | `ring_consume` | n/a — observation | [item/001](../item/001_the_six_of_a_run.md) |
| CN27 | `end` is computed by `advanced_by`, which is plain `+`, so the range type inherits `ring_types`' unchecked arithmetic; unreachable at ~585 years and worth naming as the specific inheritance | `ring_types` | n/a — observation | [item/001](../item/001_the_six_of_a_run.md) |
| CN28 | `cursor()` returns `&'a PaddedCursor` at full lifetime, and `SeqCell::store` takes `&self`, so the return value permits stores that bypass both commits and both halves of the guard; the doc comment describes it as an accessor | `ring_consume` | **latent hazard** | [item/002](../item/002_the_eight_of_a_consumer.md) |
| CN29 | `barrier()` returns by value because `Barrier` is `Copy`, a shape `ring_publish/tests/handshake_test.rs` forced and which `ring_barrier`'s module doc alone records; from here it looks like a micro-optimisation | `ring_barrier` | n/a — observation | [item/002](../item/002_the_eight_of_a_consumer.md) |
| CN30 | Three of a sequence's four transitions are observable and the load-bearing one — *read* — is not, because it happens in a buffer this crate never sees; every hazard in the crate is that gap, framed once in the module doc and referenced by neither commit function | `ring_consume` | n/a — doc gap | [lifecycle/001](../lifecycle/001_a_sequence_from_published_to_committed.md) |
| CN31 | Commit is irreversible — the producer may write the slot the instant the store retires — and the easier of the two calls is the one that discards more; nothing in either doc comment says the operation cannot be undone | `ring_consume` | n/a — doc gap | [lifecycle/001](../lifecycle/001_a_sequence_from_published_to_committed.md) |
| CN32 | A `Consumer` is disposable, and the 900-case sweep structurally relies on it — a fresh `Consumer` **and** a fresh `PaddedCursor` per case, because the state lives one level down; stated as a property nowhere | `ring_consume` | n/a — observation | [lifecycle/002](../lifecycle/002_the_consumer_over_a_rings_life.md) |
| CN33 | The wrap point is ~585 years away and at least four crates assume it away without argument; the one place its consequence is visible, `distance_to`'s saturation, describes itself as a signedness choice | family | n/a — observation | [lifecycle/002](../lifecycle/002_the_consumer_over_a_rings_life.md) |
| CN34 | Every read of the available range performed **one heap allocation**, measured at 1000/1000 in release with a counting `GlobalAlloc`, firing even when nothing remained to read; commit `b7e075ca` removed the site in `ring_cursor::slowest` and `tests/allocation_test.rs` now holds all six rows at zero | `ring_cursor` | **measured cost** | [non_functional_requirement/001](../non_functional_requirement/001_what_the_read_path_costs.md) |
| CN35 | The chain is neither `no_std`-clean nor non-blocking — `ring_wait` carries two `std::` paths and two blocking constructs — where `ring_claim`'s seven-crate chain is both; a reader who checks one Tier 5 primitive and generalises is wrong about the other | `ring_wait` | **wrong doc** | [non_functional_requirement/001](../non_functional_requirement/001_what_the_read_path_costs.md) |
| CN36 | One of four stated properties had enforcement and it was `unsafe`-freedom, guarded by two workspace lints in a crate that would never have used it, while the three a plausible change breaks were guarded by nothing and were all false; the allocation row has since gained both a guard and a true value, leaving two false and unguarded | `ring_consume` | n/a — unenforced | [non_functional_requirement/001](../non_functional_requirement/001_what_the_read_path_costs.md) |
| CN37 | Eleven ordering constants across seven crates carry seven distinct names, and exactly one — `ring_cursor::GATING`, with six importers — is actually shared; the rest are per-crate private redeclarations | family | n/a — observation | [non_functional_requirement/002](../non_functional_requirement/002_eleven_constants_and_the_one_that_is_shared.md) |
| CN38 | Four names — `COMMIT`, `PUBLISH`, `OBSERVE`, `OWN` — are each declared twice, in pairs of crates with no dependency edge between them, so the two declarations can diverge with nothing to detect it | family | n/a — duplication | [non_functional_requirement/002](../non_functional_requirement/002_eleven_constants_and_the_one_that_is_shared.md) |
| CN39 | Three crates converged on six decisions for the range value and diverged on the length type — `usize` twice, `u64` once — and the minority instance is the one matching `ring_seqno::pending`'s return type, so the newest got it right | family | n/a — duplication | [pattern/001](../pattern/001_the_half_open_range_as_a_value.md) |
| CN40 | The family's `must_use` rule — a range value whose drop strands a slot gets a messaged annotation — is followed in four places and stated in none; `ring_batch::BatchClaim` carries `Claim`'s exact drop consequence and has no annotation at all | `ring_batch` | n/a — coverage | [pattern/001](../pattern/001_the_half_open_range_as_a_value.md) |
| CN41 | Neither half's report operation takes the value its ask produced — `publish` takes `( Seq, usize )` and `commit` a bare `Seq` — so `Claim`'s severe `must_use` is the entire obligation mechanism rather than a supplement to a type-level one, and publishing or committing an unowned range compiles on both sides | family | n/a — doc gap | [pattern/002](../pattern/002_read_then_report.md) |
| CN42 | The two halves have exactly inverted fallibility in both phases — `claim` fallible / `publish` infallible against `available` infallible / `commit` fallible — every cell correct, and nothing recording that the pattern is a pattern | family | n/a — doc gap | [pattern/002](../pattern/002_read_then_report.md) |
| CN43 | `commit_available` performs its own `self.cursor.store` rather than delegating to `commit`; the guard it skips would never have fired, so the two agree today for an arithmetic reason, and anything added to `commit` silently does not apply to the other | `ring_consume` | **latent hazard** | [pitfall/001](../pitfall/001_commit_available_does_not_call_commit.md) |
| CN44 | An idle poll used to write: `commit_available` stored unconditionally, so a consumer with nothing to read issued a `Release` store to a line the producer is reading, on every iteration, for no state change — the store is now guarded by `end != run.start()` | `ring_consume` | **measured cost** | [pitfall/001](../pitfall/001_commit_available_does_not_call_commit.md) |
| CN45 | The permission/obligation asymmetry between the two range values is argued in one crate's module doc and merely asserted in three others | `ring_barrier` | n/a — observation | [pitfall/002](../pitfall/002_the_empty_barrier_is_the_case_nobody_measured.md) |
| CN46 | The empty-barrier configuration allocated nothing because `frontier()` returns `None` before reaching the `Vec`, so the cheapest measurement was the one that reported clean — which is how an allocation on every read survived unnoticed in two Tier 5 crates, and is why both empty-barrier rows are kept in the replacement test and labelled as unable to distinguish the two states | `ring_consume` | n/a — coverage | [pitfall/002](../pitfall/002_the_empty_barrier_is_the_case_nobody_measured.md) |
| CN47 | Every item the language permits to be `const` is `const` — 8 of 14, with the boundary falling exactly at the atomics — and the one non-atomic exception, `sequences`, is blocked by `Iterator::map`, proven by the E0015 compile error | `ring_consume` | n/a — observation | [type/001](../type/001_availables_const_surface.md) |
| CN48 | `Seq( pub u64 )` forecloses the niche, so `Option< Seq >` is 16 bytes where `Option< NonZeroU64 >` is 8, on a value `available()` reads every call; the same public field is the escape hatch `sequences()` requires, so the two consequences trade against each other and neither is documented | `ring_types` | n/a — observation | [type/001](../type/001_availables_const_surface.md) |
| CN49 | The single `'a` unifies two independent borrows by variance and costs a caller nothing — a cursor from an outer scope and a barrier from an inner one compose, measured — while the signature reads as a constraint; `ring_handle::Drain` is the family's one two-lifetime struct, so the alternative was reachable | `ring_consume` | n/a — observation | [type/002](../type/002_the_lifetime_on_consumer.md) |
| CN50 | Withholding `Copy` blocks only the accidental duplicate: three `Consumer`s over one cursor compile and run, and the third is built from the first's own `cursor()` and `barrier()`, reconstructing a type that deliberately does not implement `Clone` | `ring_consume` | n/a — doc gap | [type/002](../type/002_the_lifetime_on_consumer.md) |
| CN51 | Four of `ring_seqno`'s five public functions are const-able with their bodies unchanged, proven by evaluating them in `const` position; the fifth is blocked by `slice::iter`, and `ring_types` directly beneath is 12 of 12 `const` | `ring_seqno` | n/a — observation | [workaround/001](../workaround/001_five_functions_none_const.md) |
| CN52 | `.0` is the only exit from a `Seq` — no `From`, no `Deref`, no `Into` — and 15 source files take it 84 times; this crate's uses are forced by `Range`, and `ring_batch`'s eight comparison escapes in `contains` and `overlaps` were recorded here as gratuitous but are forced too: both are `pub const fn`, where a derived `PartialOrd` is `error[E0015]: cannot call non-const operator in constant functions` | `ring_types` | n/a — observation | [workaround/001](../workaround/001_five_functions_none_const.md) |
| CN53 | The allocation existed to change a slice's element type and computed nothing; the inline `.map( .. ).min()` returned identical answers including the empty case, with zero allocations against one, and the function it was adapting for was a single `.min()` — that inline line is now the shipped body | `ring_cursor` | **measured cost** | [workaround/002](../workaround/002_a_vector_to_change_a_slices_type.md) |
| CN54 | What the allocation bought was one call, not the dependency edge — the edge survived its removal on three other calls; of the three ways out, the one this section ranked last on a criterion that was not in play is the one that shipped, and no test measured any of it at the time | `ring_cursor` | **measured cost** | [workaround/002](../workaround/002_a_vector_to_change_a_slices_type.md) |
| CN55 | The family's one avoidable `.0` escape was already recorded as `ring_trace` TR49/TR50, which state that `TraceEntry::end` writes `Self( self.0 + n )` by hand and prescribe `advanced_by`; `end` in fact reads `saturating_add`, its own source comment gives the reason, and TR49's own quoted output prints that comment above the prose contradicting it — the recommendation was declined by the crate that wrote it, with no Disposition recording it | `ring_trace` | **wrong doc** | [workaround/001](../workaround/001_five_functions_none_const.md) |

### Fifty-Five Findings About Two Structs

The crate is 418 lines defining `Available`, which is two integers, and
`Consumer`, which is two borrows and no state. The distribution is where the
ratio comes from:

| Subject of the finding | Count |
|------------------------|------:|
| `ring_consume` itself | 30 |
| the family as a whole | 10 |
| another single crate — `ring_cursor` 3, `ring_barrier` 3, `ring_types` 3, `ring_wait` 2, `ring_batch` 1, `ring_mpsc` 1, `ring_seqno` 1, `ring_trace` 1 | 15 |

**Twenty-five of fifty-five are not about this crate.** A Tier 5 primitive sits
where four tiers of decisions arrive, so reading it carefully audits everything
beneath it — which is how an allocation per read in `ring_cursor` (CN34, CN53,
CN54), `std` and a blocking loop entering through `ring_wait` for a method
nothing calls (CN5, CN35), four const-able `ring_seqno` bodies (CN51), a
`must_use` gap in `ring_batch` only visible from a three-way comparison (CN40),
and a `ring_trace` finding whose prescription its own crate declined in source
without saying so (CN55) were all found by documenting a crate that contains
none of them. It also shows the cost of that reach: the `ring_cursor`
allocation was removed by its own crate without any of the three findings above
being told, and all three sat wrong until somebody re-read them.

### The Shape Every Definition Found

Stated once because most of the thirteen definitions arrive at it independently:
**the crate does less than it appears to, and the difference lands on the
caller.**

| Where the gap is | Findings |
|------------------|----------|
| a guarantee is actually a premise | CN7, CN9, CN30, CN50 |
| a stated property is false, measured | CN13, CN34, CN35 |
| a real, exceptionless discipline is stated nowhere | CN10, CN25, CN33, CN40, CN42 |
| a contract covers safety and not fitness | CN20, CN21, CN31 |
| a property is held but nothing enforces it | CN10, CN25, CN36 |
| a cost survives because the cheap check passes | CN46, CN53 |

Not one is a bug. Every behaviour described is correct against what the code
intends. The cost is uniform: a reader who trusts the prose over the source is
misled, and in eight cases the source is in another crate.

### Severity

| Tier | Meaning | Findings | Count |
|------|---------|----------|------:|
| **latent hazard** | well-typed code reaches a wedged ring or silent data loss | CN7, CN9, CN12, CN20, CN28, CN43 | 6 |
| **wrong doc** | a stated fact is false, measured | CN13, CN35, CN55 | 3 |
| **misleading doc** | a true statement a caller can act on and be wrong | CN21 | 1 |
| **measured cost** | a real, measured runtime cost nothing records | CN34, CN44, CN53, CN54 | 4 |
| n/a — doc gap | a true fact is unstated where it is needed | CN5, CN15, CN17, CN30, CN31, CN41, CN42, CN50 | 8 |
| n/a — unenforced | a real property with no lint, test, or check behind it | CN10, CN25, CN36 | 3 |
| n/a — coverage | the tests do not reach what they appear to | CN2, CN4, CN11, CN40, CN46 | 5 |
| n/a — drift | a doc describing a state that has since changed | CN3 | 1 |
| n/a — duplication | one fact stated in two places that can diverge | CN22, CN38, CN39 | 3 |
| n/a — observation | true, useful, and carrying no defect | the remaining 21 | 21 |

The first four tiers are the reachable ones — 14 of 55. Everything below them is
a record, not a problem.

The six latent hazards share the property that makes them worth ranking first:
**none produces a diagnostic at the point of the mistake.** Committing before
reading compiles and runs (CN7); a second consumer compiles, runs, and silently
skips records (CN9, CN20); a store through `cursor()` is one line of safe code
(CN12, CN28); and the duplicated store is correct today, so no test can fail
until someone changes `commit` (CN43).

The four measured costs are separated from the hazards deliberately. None
threatens correctness — every one is a cost that is real, reproducible, and
recorded nowhere, and three of the four resolve with the same one-parameter
change in `ring_seqno`.
