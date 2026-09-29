# Doc Definitions

Module Index for `ring_claim` — every doc definition this crate declares and every
instance under each, in one place.

## Master Doc Definitions Table

| Type | Purpose | Master File | Instances |
|------|---------|-------------|----------:|
| `algorithm/` | A gate that is the loop condition, and two loops that disagree at zero | [algorithm/readme.md](../algorithm/readme.md) | 2 |
| `api/` | Seventeen items, nothing discardable, and a constructor whose reason expired | [api/readme.md](../api/readme.md) | 2 |
| `data_structure/` | Sixteen bytes and a hundred and twenty-eight, and the borrow that costs both | [data_structure/readme.md](../data_structure/readme.md) | 2 |
| `decisions/` | Two rulings, and the rejected design shipped uncalled two tiers up | [decisions/readme.md](../decisions/readme.md) | 2 |
| `definition/` | Module Index — every definition and every instance in this crate, in one place | this file | — |
| `integration/` | Two dependents in different manifest sections, and three predicates never called | [integration/readme.md](../integration/readme.md) | 2 |
| `invariant/` | The one property the crate exists for, verified without the gate it depends on | [invariant/readme.md](../invariant/readme.md) | 2 |
| `item/` | Eight readings of a range and seven of a claimer, in three silent cost tiers | [item/readme.md](../item/readme.md) | 2 |
| `lifecycle/` | A range from grant to publication, and the claimer that never closes | [lifecycle/readme.md](../lifecycle/readme.md) | 2 |
| `non_functional_requirement/` | Three properties held, one enforced, and nothing timed | [non_functional_requirement/readme.md](../non_functional_requirement/readme.md) | 2 |
| `pattern/` | The CAS-retry split at opposite seams, and a range type invented three times | [pattern/readme.md](../pattern/readme.md) | 2 |
| `pitfall/` | Two ways to stall the ring with code that draws no warning | [pitfall/readme.md](../pitfall/readme.md) | 2 |
| `type/` | Three casts that all widen, and a lifetime relied on for more than it says | [type/readme.md](../type/readme.md) | 2 |
| `workaround/` | Two escapes from the language and one from a project rule | [workaround/readme.md](../workaround/readme.md) | 2 |

13 definitions, 26 instances.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_claim/docs
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
| `algorithm/` | 001 | [The Gate Inside the Retry](../algorithm/001_the_gate_inside_the_retry.md) | Three placements for the gate, two of them races, and an unbounded loop that terminates |
| `algorithm/` | 002 | [Two Loops That Disagree at Zero](../algorithm/002_two_loops_that_disagree_at_zero.md) | Adapt-or-fail on retry, one `Err` for two conditions, and the divergence in neither contract |
| `api/` | 001 | [Seventeen Items and Nothing That Drops Silently](../api/001_seventeen_items_and_nothing_that_drops_silently.md) | Every public item, the four covered by return type alone, and a justification that expired |
| `api/` | 002 | [The Two Constructors of a Range](../api/002_the_two_constructors_of_a_range.md) | One that establishes exclusivity and one that asserts it, and the contract stated three times |
| `data_structure/` | 001 | [Sixteen Bytes and One Hundred Twenty-Eight](../data_structure/001_sixteen_bytes_and_one_hundred_twenty_eight.md) | Both layouts measured, 56 bytes of padding, and the cache-line test that names the wrong thing |
| `data_structure/` | 002 | [The Borrow That Is Half the Type](../data_structure/002_the_borrow_that_is_half_the_type.md) | Why the gate cannot be owned, the type it forced into `ring_mpsc`, and the 23 lifetime-carriers |
| `decisions/` | 001 | [Compare-Exchange Rather Than `fetch_add`](../decisions/001_compare_exchange_rather_than_fetch_add.md) | What the rejected design actually breaks, and where it ships uncalled |
| `decisions/` | 002 | [`must_use` Without `Drop`](../decisions/002_must_use_without_drop.md) | The family's twelve messaged annotations, ten of them backed by nothing, and the four `Drop` guards none of which is here |
| `integration/` | 001 | [Two Dependents That Split One Feature](../integration/001_two_dependents_that_split_one_feature.md) | Every edge in and out, one of them test-only, and the arithmetic a dependency does on its behalf |
| `integration/` | 002 | [Four Predicates and the One That Is Called](../integration/002_four_predicates_and_the_one_that_is_called.md) | Three uncalled predicates for three different reasons, one of them a race in this shape |
| `invariant/` | 001 | [No Two Producers Hold One Sequence](../invariant/001_no_two_producers_hold_one_sequence.md) | The founding property, and five concurrent tests four of which disable the gate |
| `invariant/` | 002 | [A Refused Claim Moves Nothing](../invariant/002_a_refused_claim_moves_nothing.md) | Two calls that leave the cursor unmoved and agree on nothing else |
| `item/` | 001 | [The Eight Readings of a Range](../item/001_the_eight_readings_of_a_range.md) | The `const` boundary and why it falls there, and two guards a doctest cannot see |
| `item/` | 002 | [The Seven of the Claimer](../item/002_the_seven_of_the_claimer.md) | Three undocumented cost tiers, and an accessor documented for a purpose it forbids |
| `lifecycle/` | 001 | [A Range from Grant to Publication](../lifecycle/001_a_range_from_grant_to_publication.md) | Where a claim goes after this crate stops watching, and the lint that keeps a spin terminating |
| `lifecycle/` | 002 | [The Claimer Over a Ring's Life](../lifecycle/002_the_claimer_over_a_rings_life.md) | Construction to ~585 years, a safe-code break in one line, and an overflow sentence that is wrong |
| `non_functional_requirement/` | 001 | [Nothing Allocates, Nothing Waits, Nothing Is Unsafe](../non_functional_requirement/001_nothing_allocates_nothing_waits_nothing_is_unsafe.md) | Three properties, one enforced, one measured false at an allocation per `claim`, and `no_std` free to declare across the 30 crates that still don't |
| `non_functional_requirement/` | 002 | [What Contention Costs](../non_functional_requirement/002_what_contention_costs.md) | A `k × C` model from source, a harness this crate cannot enter, and nothing ever timed |
| `pattern/` | 001 | [Retrying Against a Moving Target](../pattern/001_retrying_against_a_moving_target.md) | Two crates applying CAS-retry at opposite seams, and what an internal loop cannot report |
| `pattern/` | 002 | [The Half-Open Range as a Value](../pattern/002_the_half_open_range_as_a_value.md) | Three independent 16-byte implementations agreeing on derives and nothing else |
| `pitfall/` | 001 | [Dropping a Claim](../pitfall/001_dropping_a_claim.md) | Four routes to a stranded claim, all silent, surfacing in a thread that did nothing wrong |
| `pitfall/` | 002 | [Claiming Zero](../pitfall/002_claiming_zero.md) | Two opposite zero behaviours, both deliberate, both documented only in the test file |
| `type/` | 001 | [A Seq, a usize, and Three Casts](../type/001_a_seq_a_usize_and_three_casts.md) | Every cast widens, the narrowing lives two crates away, and no slot index is ever built |
| `type/` | 002 | [The Lifetime on the Claimer](../type/002_the_lifetime_on_the_claimer.md) | Two accessors with opposite reach, and the `Sync` the whole design rests on |
| `workaround/` | 001 | [Two Const Functions the Compiler Refuses](../workaround/001_two_const_functions_the_compiler_refuses.md) | E0015 from a derive two crates away, and two escapes no crate took together |
| `workaround/` | 002 | [The Constructor That Exists for the Test Directory](../workaround/002_the_constructor_that_exists_for_the_test_directory.md) | What a forgeable `Claim` cannot promise, and the convention that keeps it forgeable |

## Findings

Fifty-five, each verified by a command whose output is quoted in its instance.

| ID | Finding | Subject | Reachable | Where |
|----|---------|---------|-----------|-------|
| CL1 | The only Tier 5 primitive whose two dependents sit in different manifest sections; the two halves of the claim/publish handshake are joined by an edge that exists only under `cargo test` and have never been linked in a release build | `ring_claim` | n/a — observation | [integration/001](../integration/001_two_dependents_that_split_one_feature.md) |
| CL2 | Both halves of the handshake were scaffolded into `ring_mpsc` and one survived: a per-slot stamp replaces a published cursor, but nothing replaces range exclusivity, which can only be established at the cursor | `ring_mpsc` | n/a — drift | [integration/001](../integration/001_two_dependents_that_split_one_feature.md) |
| CL3 | `ring_seqno` was removed from this crate and from `ring_publish` by the same check, because `GatingSet::headroom` reaches Tier 1 arithmetic on this crate's behalf | `ring_claim` | n/a — observation | [integration/001](../integration/001_two_dependents_that_split_one_feature.md) |
| CL4 | `ring_gating`'s principal caller uses one of its four public predicates; the one that returns a number is used, the three that return a verdict are not | `ring_gating` | n/a — observation | [integration/002](../integration/002_four_predicates_and_the_one_that_is_called.md) |
| CL5 | `GatingSet::check` is `claim`'s two guards in the same order with the same payloads, and calling it would move the gate outside the retry — the exact `fetch_add` design the module doc rejects | `ring_gating` | **latent hazard** | [integration/002](../integration/002_four_predicates_and_the_one_that_is_called.md) |
| CL6 | Three uncalled predicates for three different reasons: `admits` is sugar over the called one, `check`'s shape is wrong here specifically, `limit` answers a diagnostic question and its only outside caller is a test in this crate | `ring_gating` | n/a — observation | [integration/002](../integration/002_four_predicates_and_the_one_that_is_called.md) |
| CL7 | The gate is the loop condition, so it is re-read against the value the failed exchange returned; of the three places it could go, the tempting one is the shape of `GatingSet::check` and is a race | `ring_claim` | n/a — observation | [algorithm/001](../algorithm/001_the_gate_inside_the_retry.md) |
| CL8 | Both loop exits report `Err( RingError::Full )`, for a genuinely full ring and for a producer that lost every exchange; the collapse is operationally correct and diagnostically lossy, which is what `GatingSet::limit` exists to recover | `ring_claim` | n/a — diagnostics | [algorithm/001](../algorithm/001_the_gate_inside_the_retry.md) |
| CL9 | `claim( 0 )` returns `Ok` on a full ring and `claim_up_to( 0 )` returns `Err( Full )`; both are tested, both are defensible, and neither divergence appears in either `# Errors` section | `ring_claim` | **misleading doc** | [algorithm/002](../algorithm/002_two_loops_that_disagree_at_zero.md) |
| CL10 | `claim_up_to` recomputes its grant width on every retry and `claim` does not, so a loser adapts to a smaller ring rather than failing; that also makes its retry arm strictly harder to reach, which is how the coverage gate found it untested | `ring_claim` | n/a — coverage | [algorithm/002](../algorithm/002_two_loops_that_disagree_at_zero.md) |
| CL11 | No item's result can be discarded silently; the four without an explicit `#[ must_use ]` are each covered by their return type, and nothing checks that a future method keeps the property | `ring_claim` | n/a — unenforced | [api/001](../api/001_seventeen_items_and_nothing_that_drops_silently.md) |
| CL12 | `Claim::new` is public "because `ring_publish` … need[s] to construct one directly", and all 27 of its call sites are inside this crate; `ring_publish`'s reached-test imports `Claimer` and calls `Claimer::new` ten times | `ring_claim` | n/a — drift | [api/001](../api/001_seventeen_items_and_nothing_that_drops_silently.md) |
| CL13 | Two variants and the distinction is a retry instruction; the same contract is stated three times, one of them in `GatingSet::check`'s `# Errors` section in almost identical words, for a function this crate cannot call | `ring_claim` | n/a — duplication | [api/002](../api/002_the_two_constructors_of_a_range.md) |
| CL14 | Thirteen of seventeen public items perform no atomic operation, and the type that names the crate performs none at all; that split is why six single-threaded tests cover the range arithmetic and five `thread::scope` blocks cover two methods | `ring_claim` | n/a — observation | [api/002](../api/002_the_two_constructors_of_a_range.md) |
| CL15 | `Claimer` is 128 bytes holding 72 bytes of fields; adding one 8-byte reference to a 64-byte cache-line-aligned struct costs 64 bytes, not 8, and 56 of them are dead | `ring_claim` | n/a — observation | [data_structure/001](../data_structure/001_sixteen_bytes_and_one_hundred_twenty_eight.md) |
| CL16 | A `Claimer` spans two cache lines and only the first is ever written; the second holds a reference assigned once, so it stays shared-clean in every core's cache, and the padding is what guarantees nothing else lands there | `ring_claim` | n/a — observation | [data_structure/001](../data_structure/001_sixteen_bytes_and_one_hundred_twenty_eight.md) |
| CL17 | The borrow forced `ring_mpsc::Ends` into existence; of the family's three `Ends` types it is the only one whose doc states a cause, and that cause is this crate's second field, named in three separate doc comments | `ring_mpsc` | n/a — observation | [data_structure/002](../data_structure/002_the_borrow_that_is_half_the_type.md) |
| CL18 | Four of the family's 23 lifetime-carrying public types have no type parameter, and they are exactly the four operating on sequences rather than payloads — the Tier 5 boundary drawn in the type system | family | n/a — observation | [data_structure/002](../data_structure/002_the_borrow_that_is_half_the_type.md) |
| CL19 | The rejected `fetch_add` design does *not* hand out overlapping ranges between producers; the add is atomic, so every producer gets a distinct start, and the only symptom is a grant that runs past the gate into slots a consumer has not released | `ring_claim` | n/a — doc gap | [decisions/001](../decisions/001_compare_exchange_rather_than_fetch_add.md) |
| CL20 | `ring_batch::claim_gated` is the rejected design line for line — guard, gate, `fetch_add` — public and uncalled, and its documentation carried no producer-count restriction until a `# One producer only` section was added; the argument for why the shape is wrong still lives two tiers up, so that section has to carry the pointer back | `ring_batch` | **latent hazard** | [decisions/001](../decisions/001_compare_exchange_rather_than_fetch_add.md) |
| CL21 | Ten of the family's twelve messaged `must_use` annotations have no runtime mechanism behind them; `Claim`'s and `ring_atomic`'s name the same permanent consequence — a range stranded, every consumer stalled — and neither can carry a destructor; `let _ = …` silences the lint and nothing else happens, ever | `ring_claim` | n/a — unenforced | [decisions/002](../decisions/002_must_use_without_drop.md) |
| CL22 | All four `Drop` impls in the family are in `ring_mpsc` and `ring_spsc`, on borrow-carrying guard types, and none is in a Tier 5 primitive; `ring_spsc`'s `Producer::claim` argues for the guard shape by naming "a bare `claim`/`publish` pair" — which is this crate | family | n/a — observation | [decisions/002](../decisions/002_must_use_without_drop.md) |
| CL23 | Four of the five concurrent tests build their `GatingSet` with **zero consumers**, which `ring_gating::headroom` answers with the full capacity unconditionally, so the gate never refuses; the crate's two invariants are verified under configurations that are mirror opposites on every axis a concurrency bug lives on | `ring_claim` | n/a — coverage | [invariant/001](../invariant/001_no_two_producers_hold_one_sequence.md) |
| CL24 | The invariant with the most test code behind it cannot detect the crate's founding defect: `fetch_add` claiming produces no duplicate grants, so 4 of 5 concurrent tests pass against it unchanged | `ring_claim` | n/a — coverage | [invariant/001](../invariant/001_no_two_producers_hold_one_sequence.md) |
| CL25 | The two calls that leave the cursor unmoved differ in every other respect: `claim( 1 )` on a full ring returns `Err` and issues no atomic at all, while `claim( 0 )` returns `Ok` and issues a `compare_exchange` — the cheap path is the failing one, which is the opposite of what a reader predicts | `ring_claim` | n/a — observation | [invariant/002](../invariant/002_a_refused_claim_moves_nothing.md) |
| CL26 | The contention test reads the consumer limit *after* each grant, and monotonicity is what makes that both sound and sensitive; reading it before would fail correct implementations whenever a consumer advanced in the window | `ring_claim` | n/a — observation | [invariant/002](../invariant/002_a_refused_claim_moves_nothing.md) |
| CL27 | `Claim`'s `const` boundary fell where it did because `contains` and `overlaps` compared `Seq` values through `PartialOrd`, which `const fn` cannot call; `ring_batch::BatchClaim` implements the identical predicates against `.0` and is `const` on both, reaching 7 of 8 where this crate reached 5 of 8 — this crate has since taken the same `.0` escape, so both now reach 7 of 8 | `ring_claim` | **measured cost** | [item/001](../item/001_the_eight_readings_of_a_range.md) |
| CL28 | `overlaps`'s two `is_empty()` guards are load-bearing, and its doctest picks the one empty case the arithmetic already handles: deleting both guards leaves all three doctest assertions passing while diverging from the first-principles definition on **52 of 900** pairs in the exhaustive test | `ring_claim` | n/a — coverage | [item/001](../item/001_the_eight_readings_of_a_range.md) |
| CL29 | The seven sort into three undocumented cost tiers (three free, two reading, two writing), and `headroom` is the one that misleads: its own doc calls it "a hint only", and a caller who reads it and then claims that many has rebuilt the rejected check-then-act shape outside the crate | `ring_claim` | **misleading doc** | [item/002](../item/002_the_seven_of_the_claimer.md) |
| CL30 | `cursor()` is documented "for `ring_publish` to read and for a gating set … to be built against"; `ring_publish` owns its own `PaddedCursor` and its module doc opens by forbidding that conflation, the `ring_mpsc` dependency runs the other way, and the single real caller uses `addr()` for a cache-line check | `ring_claim` | n/a — drift | [item/002](../item/002_the_seven_of_the_claimer.md) |
| CL31 | `ring_publish::publish` is a budget-free, strategy-free `loop` whose termination argument is that the predecessor "is committed to" publishing; nothing enforces that, so the family's one deliberately unbounded spin is kept terminating by a `#[ must_use ]` lint two tiers down — and its `# Panics` section documented only the caller's own deadlock until the predecessor case was added beside it | `ring_publish` | **latent hazard** | [lifecycle/001](../lifecycle/001_a_range_from_grant_to_publication.md) |
| CL32 | Because `try_publish` advances only when the published cursor is *exactly* at the claim's start, a dropped claim of width one stops the ring as completely as one of width a thousand: the loss is the ordering token, not the slots, and every later claim keeps succeeding for a while | `ring_claim` | **latent hazard** | [lifecycle/001](../lifecycle/001_a_range_from_grant_to_publication.md) |
| CL33 | `Claimer::cursor()` returns `&PaddedCursor`, and `SeqCell` is a public trait whose four `&self` methods include `store` and `fetch_add`; the crate's founding invariant is therefore breakable from safe code in one line, and `tests/manual/readme.md § C2` cannot see it because it greps this crate's own source | `ring_claim` | **latent hazard** | [lifecycle/002](../lifecycle/002_the_claimer_over_a_rings_life.md) |
| CL34 | `Seq::next`'s doc states release-mode `u64` addition "saturates … deliberately not wrapping"; it wraps, measured, and the error runs in the unsafe direction — saturation would preserve monotonicity and stop the ring loudly, wrapping inverts every gate comparison | `ring_types` | **wrong doc** | [lifecycle/002](../lifecycle/002_the_claimer_over_a_rings_life.md) |
| CL35 | `ring_claim` and all six of its transitive dependencies contain zero `std::` paths, every `use` in the chain resolving to `core::` or a sibling crate, while **three of the 33 family crates declare `#![ no_std ]`** and of this chain's seven only `ring_types` is among them; declaring it on the rest costs nothing, measured — 27 integration tests and 17 doctests pass unchanged | family | n/a — unenforced | [non_functional_requirement/001](../non_functional_requirement/001_nothing_allocates_nothing_waits_nothing_is_unsafe.md) |
| CL36 | The only enforced property is `unsafe`, guarded by two lints, in a crate of three comparisons and a compare-exchange; the two a plausible change would actually break — allocation and non-blocking — are guarded by nothing, and `§ C2`'s existing grep could carry both for the cost of one alternation | `ring_claim` | n/a — unenforced | [non_functional_requirement/001](../non_functional_requirement/001_nothing_allocates_nothing_waits_nothing_is_unsafe.md) |
| CL37 | The family has no `benches/` directory, no `[[bench]]` section and no bench framework in any of 33 crates; `ring_bench` measures six complete write paths including a `MutexQueue` baseline, and `ring_claim` appears in none of them and cannot, since a `Candidate` is an end-to-end path and a `Claimer` writes nothing | family | n/a — coverage | [non_functional_requirement/002](../non_functional_requirement/002_what_contention_costs.md) |
| CL38 | The one measured number anywhere in this crate is `§ C1`'s 4/5→8/8 detection rate, which quantifies a *test's* sensitivity against a broken implementation, not the code's speed; it reads like a performance figure at a glance and nothing in the crate has ever been timed | `ring_claim` | n/a — observation | [non_functional_requirement/002](../non_functional_requirement/002_what_contention_costs.md) |
| CL39 | Only two crates apply the CAS-retry pattern rather than provide it, and they split it at opposite seams: `ring_claim` hides the loop and exposes no single-attempt variant, `ring_publish` exposes `try_publish` and returns the observed value as `Err( Seq )` | family | n/a — observation | [pattern/001](../pattern/001_retrying_against_a_moving_target.md) |
| CL40 | Because the loop is internal, a first-evaluation refusal and a 40-attempt loss both return `Err( RingError::Full )`, so a caller cannot distinguish back-pressure from contention though the two call for opposite responses | `ring_claim` | n/a — diagnostics | [pattern/001](../pattern/001_retrying_against_a_moving_target.md) |
| CL41 | Three crates implement the plain-value half-open range independently (`Claim`, `BatchClaim`, `Available`), all 16 bytes, agreeing on derives and on nothing else: two receivers, two width types, three method counts, one type-level annotation between them, and no file mentioning that the other two exist | family | n/a — duplication | [pattern/002](../pattern/002_the_half_open_range_as_a_value.md) |
| CL42 | Annotation tracks the cost of dropping across five of the seven range types, and `ring_mpsc:927-931` spends five lines justifying an *absence*, proving the rule is deliberate; the two gaps are `ring_mpsc::Batch` and `ring_batch::BatchClaim`, each a twin of an annotated type whose warning sentence applies verbatim | family | n/a — coverage | [pattern/002](../pattern/002_the_half_open_range_as_a_value.md) |
| CL43 | All four routes to a stranded claim (explicit discard, `?` early return, unwind, loop skip) compile with **zero warnings**; `#[ must_use ]` fires only on an unused value, and binding the claim satisfies it immediately, before any of the four routes begin | `ring_claim` | **latent hazard** | [pitfall/001](../pitfall/001_dropping_a_claim.md) |
| CL44 | The producer that drops its claim never fails; `claim` keeps returning `Ok`, `claimed()` keeps advancing and `headroom()` keeps reporting a healthy number, while the published frontier is pinned forever at the stranded sequence, so the thread that hangs is a later one that did nothing wrong | `ring_claim` | **latent hazard** | [pitfall/001](../pitfall/001_dropping_a_claim.md) |
| CL45 | `claim( 0 )` succeeds on a completely full ring and is the only call that does; `claim_up_to( 0 )` fails on a completely empty one and is the only call that does; both behaviours are deliberate and correct against their contracts, and both are explained only in `claim_test.rs` | `ring_claim` | n/a — doc gap | [pitfall/002](../pitfall/002_claiming_zero.md) |
| CL46 | `claim_up_to`'s `# Errors` states `Full` means *"not even one slot is free"*, which is false for `max = 0` on an empty ring; the natural drain loop that trusts it backs off forever waiting for a consumer to free space in a ring that is already empty | `ring_claim` | **wrong doc** | [pitfall/002](../pitfall/002_claiming_zero.md) |
| CL47 | All three casts in the crate are `usize` → `u64` widenings serving `Seq::advanced_by`; the reciprocal narrowing at `ring_seqno:98` used to depend on an invariant `ring_claim` maintains and neither crate states, but a refactor moved the narrow to the far side of a `saturating_sub` against a widened `capacity`, so the cast is now lossless by local construction alone | family | n/a — observation | [type/001](../type/001_a_seq_a_usize_and_three_casts.md) |
| CL48 | The crate names 2 of `ring_types`' 6 public types and contains zero occurrences of `SlotIndex`, `%`, `mask`, or `Capacity`: a `Claim` is a range of sequences, never of slots, which is why `overlaps` compares claims from different rings and why the 900-pair test needs no ring at all | `ring_claim` | n/a — observation | [type/001](../type/001_a_seq_a_usize_and_three_casts.md) |
| CL49 | `consumers()` returns `&'a GatingSet` and outlives its `Claimer`; `cursor()` returns an elided borrow that cannot; both doc comments are one symmetric sentence and neither mentions it, and the accessor documented for cross-crate use is the one whose lifetime cannot leave the local scope | `ring_claim` | n/a — doc gap | [type/002](../type/002_the_lifetime_on_the_claimer.md) |
| CL50 | `Claimer : Sync` is what the entire multi-producer design rests on, holds by auto-derivation, and appears nowhere: zero occurrences of `Send`, `Sync`, or `thread` in a crate whose module doc opens on contention, while four sibling crates write explicit `T : Send` bounds | `ring_claim` | n/a — unenforced | [type/002](../type/002_the_lifetime_on_the_claimer.md) |
| CL51 | `contains` and `overlaps` were `const` in `ring_batch` and not in `ring_claim`, with line-for-line identical bodies differing only by `.0`; `E0015` blocks the `Seq` operators because the `PartialOrd` impl is derived and non-`const`, and the diagnostic blames `ring_types/src/id.rs:24`, two crates from the code that cannot compile | `ring_claim` | n/a — observation | [workaround/001](../workaround/001_two_const_functions_the_compiler_refuses.md) |
| CL52 | `+ use< >` appears on exactly the 2 of 7 iterator-returning methods that take `&self` and yield owned values, with zero exceptions — the family's only other occurrence is on a free function, outside a census anchored on `self`; omitting it is **not a local error** — the library compiles and the failure is an `E0597` in a downstream crate, noting a signature the caller does not own | family | **latent hazard** | [workaround/001](../workaround/001_two_const_functions_the_compiler_refuses.md) |
| CL53 | A public constructor makes `Claim` forgeable, so `ring_publish`'s PB35 proposal would enforce **2 of 4** preconditions rather than 4 — worth making, but it hands the crate above a type that looks like proof of a grant and is not, and PB35 argues for the change without pricing it | `ring_publish` | n/a — observation | [workaround/002](../workaround/002_the_constructor_that_exists_for_the_test_directory.md) |
| CL54 | 33 of 33 crates keep tests in `tests/` with zero `#[ cfg( test ) ]` modules in any `src/`, so all 32 family constructors are unconditionally public, and none of `doc( hidden )`, `pub( crate )`, or a test-only feature is used anywhere to narrow one | family | n/a — observation | [workaround/002](../workaround/002_the_constructor_that_exists_for_the_test_directory.md) |
| CL55 | `Claimer::claim` and `Claimer::headroom` each performed **one heap allocation per call** until commit `b7e075ca`, measured in release with a counting `GlobalAlloc`; the site was `ring_cursor::slowest`, collecting cursor positions into a `Vec` only to adapt `&[ PaddedCursor ]` to the `&[ Seq ]` its callee takes, where the allocation-free fold that has since replaced it would do. It read as clean because the evidence command grepped one file while the claim covers seven crates, and because an ungated set's zero-length `Vec` never reaches the allocator — the property is now held by `tests/allocation_test.rs` rather than by prose | `ring_cursor` | **wrong doc** | [non_functional_requirement/001](../non_functional_requirement/001_nothing_allocates_nothing_waits_nothing_is_unsafe.md) |

### Fifty-Five Findings About Two Structs

The crate is 461 lines defining two types, one of which is two integers. That it
supports fifty-five verified findings is the result worth stating first, and the
reason is visible in the distribution:

| Subject of the finding | Count |
|------------------------|------:|
| `ring_claim` itself | 35 |
| the family as a whole | 10 |
| another single crate — `ring_gating` 3, `ring_mpsc` 2, `ring_publish` 2, `ring_batch` 1, `ring_cursor` 1, `ring_types` 1 | 10 |

**Twenty of fifty-five are not about this crate.** A Tier 5 primitive sits where
four tiers of decisions arrive and one tier of consumers depends, so reading it
carefully is the cheapest available audit of everything it touches — which is how
a wrong sentence in `ring_types` (CL34), a rejected design shipped uncalled in
`ring_batch` (CL20), an unbounded spin in `ring_publish` (CL31), and a heap
allocation on every `claim` inside `ring_cursor` (CL55) were all
found by documenting a crate that contains none of them. CL55 has since been
fixed at its source and pinned by a test here.

### The Shape Every Definition Found

Stated once because eleven of the thirteen definitions arrive at it
independently: **the code is right and the documentation is not.**

| Where the gap is | Findings |
|------------------|----------|
| a doc states something false | CL34, CL46 |
| a doc names a caller, purpose, or beneficiary that does not exist | CL12, CL30 |
| a real, exceptionless discipline is stated nowhere | CL35, CL47, CL50, CL54 |
| a deliberate behaviour is explained only in the test file | CL9, CL45 |
| a property is held but nothing enforces it | CL11, CL21, CL36, CL50 |

Not one of these is a bug. Every behaviour they describe is correct against what
the code intends. The cost is uniformly the same: a reader who trusts the prose
over the source is misled, and in four cases the source is two crates away.

### Severity

| Tier | Meaning | Findings | Count |
|------|---------|----------|------:|
| **latent hazard** | well-typed code reaches a wedged ring or a broken build | CL5, CL20, CL31, CL32, CL33, CL43, CL44, CL52 | 8 |
| **wrong doc** | a stated fact is false, measured | CL34, CL46, CL55 | 3 |
| **misleading doc** | a true statement a caller can act on and be wrong | CL9, CL29 | 2 |
| n/a — doc gap | a true fact is unstated where it is needed | CL19, CL45, CL49 | 3 |
| n/a — unenforced | a real property with no lint, test, or check behind it | CL11, CL21, CL35, CL36, CL50 | 5 |
| n/a — coverage | the tests do not reach what they appear to | CL10, CL23, CL24, CL28, CL37, CL42 | 6 |
| n/a — drift | a doc or plan describing a state that has since changed | CL2, CL12, CL30 | 3 |
| n/a — duplication | one fact stated in two places that can diverge | CL13, CL41 | 2 |
| n/a — diagnostics | correct behaviour that discards information a caller needs | CL8, CL40 | 2 |
| n/a — observation | true, useful, and carrying no defect | the remaining 21 | 21 |

The first three tiers are the reachable ones — 13 of 55. Everything below them is
a record, not a problem.

The eight latent hazards share one property that makes them worth ranking above
the rest: **none produces a diagnostic at the point of the mistake.** Four
compile with zero warnings (CL43), one fails only in a downstream crate (CL52),
one surfaces in a different thread at a later call (CL44), one requires reading a
crate two tiers up to know it is wrong (CL20), and one is a single line of safe
code (CL33).
