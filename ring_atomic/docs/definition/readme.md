# Doc Definitions

Module Index for `ring_atomic` — every doc definition this crate declares and every
instance under each, in one place.

## Master Doc Definitions Table

| Type | Purpose | Master File | Instances |
|------|---------|-------------|----------:|
| `algorithm/` | Both implementations of the four operations, and the read that is four reads | [algorithm/readme.md](../algorithm/readme.md) | 2 |
| `api/` | What the four signatures commit to, and the two attributes they decline to carry | [api/readme.md](../api/readme.md) | 2 |
| `data_structure/` | Two cells, one report, and what packing five contended counters costs | [data_structure/readme.md](../data_structure/readme.md) | 2 |
| `decisions/` | Choices with live alternatives, recorded with their arguments | [decisions/readme.md](../decisions/readme.md) | 2 |
| `definition/` | Module Index — every definition and every instance in this crate, in one place | this file | — |
| `integration/` | Two edges in of which one is dead, five out that the re-export turns into twelve | [integration/readme.md](../integration/readme.md) | 2 |
| `invariant/` | Properties relied on upstream, including the one this crate declines to enforce | [invariant/readme.md](../invariant/readme.md) | 2 |
| `item/` | Per-item contracts and coverage, constructor by constructor and method by method | [item/readme.md](../item/readme.md) | 2 |
| `lifecycle/` | A cell from zero to drop, and the crate from task file to 376 lines | [lifecycle/readme.md](../lifecycle/readme.md) | 2 |
| `non_functional_requirement/` | What the instrument costs, the budget nobody wrote, and three optimizations tested | [non_functional_requirement/readme.md](../non_functional_requirement/readme.md) | 2 |
| `pattern/` | The single creation site, and the substitute that is not a mock | [pattern/readme.md](../pattern/readme.md) | 2 |
| `pitfall/` | Two defensive constructs that turn an anomaly into a plausible number | [pitfall/readme.md](../pitfall/readme.md) | 2 |
| `type/` | What the trait requires against what its users need, and the report nobody keeps | [type/readme.md](../type/readme.md) | 2 |
| `workaround/` | External constraints this crate absorbs, with costs and deletion conditions | [workaround/readme.md](../workaround/readme.md) | 2 |

13 definitions, 26 instances, 52 findings.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_atomic/docs
find . -mindepth 1 -maxdepth 1 -type d ! -name definition | wc -l   # 13
find . -name '0*.md' | wc -l                                        # 26
command grep -rho '^### AT[0-9]*' . | sort -u | wc -l                # 52
```

## Master Doc Instances Table

| Type | ID | Title | Subject |
|------|----|-------|---------|
| `algorithm/` | 001 | [One Intrinsic, or Two](../algorithm/001_one_intrinsic_or_two.md) | Both implementations of the four operations, and what the instrument costs |
| `algorithm/` | 002 | [`counts` Is Four Reads, Not One](../algorithm/002_counts_is_four_reads_not_one.md) | The snapshot that is not one, measured, and the test that cannot see it |
| `api/` | 001 | [The Return Value That Is a Claim](../api/001_the_return_value_that_is_a_claim.md) | The three returning methods, the five `must_use` spent elsewhere, and the proof the attribute works here |
| `api/` | 002 | [A Shared Cell That Is Not `Sync`](../api/002_a_shared_cell_that_is_not_sync.md) | The empty supertrait list, the three bare bounds it produces, and what each admits |
| `data_structure/` | 001 | [One Word, and Five on One Line](../data_structure/001_one_word_and_five.md) | Both cells' sizes and alignment, the missing `repr`, and packed-vs-padded measured at four thread counts |
| `data_structure/` | 002 | [`OpCounts`, and the `total` It Stores](../data_structure/002_opcounts_and_the_total_it_stores.md) | The five public fields, the redundant one, and what derived equality does with it |
| `decisions/` | 001 | [Orderings Named, Never Defaulted](../decisions/001_orderings_named_never_defaulted.md) | The claim's true scope, the nine literals outside it, and why no ordering would help |
| `decisions/` | 002 | [A Trait, Because Two Criteria Needed Two Implementations](../decisions/002_a_trait_because_the_criteria_needed_two.md) | The argument, both criteria as written, and the fourteen assertions built on them |
| `integration/` | 001 | [Two Declared, One Used](../integration/001_two_declared_one_used.md) | The unused `ring_seqno` edge, the three documents that record it, and where it came from |
| `integration/` | 002 | [Five Crates Downstream, Twelve Reached](../integration/002_five_crates_downstream.md) | The re-export that triples the trait's reach, its argument, and its own stale count |
| `invariant/` | 001 | [Monotonicity Is Relied On Here and Required Nowhere](../invariant/001_monotonicity_is_relied_on_and_not_required.md) | The three ways backwards, what the family says about it, and the crate built to detect it |
| `invariant/` | 002 | [Every Increment Survives, and Nothing Relates the Two Numbers](../invariant/002_every_increment_survives.md) | What the two concurrency tests actually rest on, and the ordering nothing checks |
| `item/` | 001 | [Six Constructors for Two Types](../item/001_six_constructors_for_two_types.md) | The `loom` split, the hand-written `Default`, and the `const` nothing uses |
| `item/` | 002 | [`counts`, the Method That Is Not a Snapshot](../item/002_counts_the_method_that_is_not_a_snapshot.md) | The two off-trait methods, their contracts, and the code they can never reach |
| `lifecycle/` | 001 | [Born at Zero, Climbing Until Dropped](../lifecycle/001_born_at_zero_climbing_until_dropped.md) | Every store in the family, the absent destructor, and slots recycling where sequences do not |
| `lifecycle/` | 002 | [From Task File to Crate, With the Gate Left Behind](../lifecycle/002_from_task_file_to_crate.md) | The readiness gate 33 tasks never crossed, and the two that tried |
| `non_functional_requirement/` | 001 | [What the Instrument Costs](../non_functional_requirement/001_what_the_instrument_costs.md) | The shim's overhead, the absent budget, and the three units the `ring_batch` criterion mixes |
| `non_functional_requirement/` | 002 | [Two Optimizations That Are Not, and One That Is](../non_functional_requirement/002_the_optimizations_that_are_not.md) | Weak CAS, padding, and packing the counters into one word |
| `pattern/` | 001 | [One Place Where an Atomic Is Created](../pattern/001_one_place_where_an_atomic_is_created.md) | The single-creation-site pattern, the ten raw atomics outside it, and what centralizing creation does not buy |
| `pattern/` | 002 | [The Counting Cell Is Not a Mock](../pattern/002_the_counting_cell_is_not_a_mock.md) | Delegation instead of reimplementation, the one test the family's counting assertions rest on, and the fidelity claim nobody bounded |
| `pitfall/` | 001 | [The Snapshot That Never Happened](../pitfall/001_the_snapshot_that_never_happened.md) | Why a torn `OpCounts` is undetectable from the returned value, and the four artifacts that say it is safe |
| `pitfall/` | 002 | [The Wrap That Reads as an Empty Ring](../pitfall/002_the_wrap_that_reads_as_an_empty_ring.md) | `fetch_add` at the top of `u64`, the guard that sits on another path, and why every gate then grants |
| `type/` | 001 | [What the Trait Promises a Caller](../type/001_what_the_trait_promises.md) | The absent supertrait, the three unmarked returns, and the one contract clause in the file |
| `type/` | 002 | [The Report That Is All Public](../type/002_the_report_that_is_all_public.md) | Six derives, five public fields, no inherent impl, and 16 downstream reads that never keep a value |
| `workaround/` | 001 | [The Loom Seam and the Manifest Above It](../workaround/001_the_loom_seam_and_the_manifest_above_it.md) | What the seam costs inside the crate, and the `check-cfg` entry it cannot contain |
| `workaround/` | 002 | [What the Seam Does Not Switch](../workaround/002_what_the_seam_does_not_switch.md) | The third imported name that crosses unswitched, and the suite that compiles but cannot run |

## Findings

Fifty-two, each verified by a command whose output is quoted in its instance.

| ID | Finding | Subject | Reachable | Where |
|----|---------|---------|-----------|-------|
| AT1 | Every `AtomicSeq` method is one intrinsic with a newtype around the result — no branch, no assertion, no ordering chosen — so the production path has no algorithm of its own to be wrong about | `ring_atomic` | n/a — observation | [algorithm/001](../algorithm/001_one_intrinsic_or_two.md) |
| AT2 | `CountingSeq` issues two hardware atomics per logical operation and turns a pure read into a read-modify-write — 1.90× on `fetch_add`, 3.42× on `load`, 1.82× at eight threads — while certifying a count of logical operations for two criteria phrased in fences | `ring_atomic` | **measured cost** | [algorithm/001](../algorithm/001_one_intrinsic_or_two.md) |
| AT3 | `counts` builds one struct from four separately-timed `Relaxed` loads, so it can report a state that never existed — 11,575 impossible snapshots in 2,000,000, the widest by 5,456 | `ring_atomic` | **latent hazard** | [algorithm/002](../algorithm/002_counts_is_four_reads_not_one.md) |
| AT4 | `reset_counts` has the same seam on the write side; both methods' doc comments read as descriptions of an instant, and the one concurrent test touches a single counter so neither seam is reachable from the suite | `ring_atomic` | n/a — doc gap | [algorithm/002](../algorithm/002_counts_is_four_reads_not_one.md) |
| AT5 | `fetch_add`'s own doc calls the return value "the first sequence the caller now owns", the advance cannot be undone, and dropping the value compiles clean — while all five `must_use` the crate spends sit on items where dropping costs nothing | `ring_atomic` | **latent hazard** | [api/001](../api/001_the_return_value_that_is_a_claim.md) |
| AT6 | `#[ must_use ]` on the trait declaration propagates to every implementor, proved against a patched copy; across all 33 crates the attribute appears 253 times and never on a trait method | `ring_atomic` | n/a — unenforced | [api/001](../api/001_the_return_value_that_is_a_claim.md) |
| AT7 | `SeqCell` declares no supertrait, so `&dyn SeqCell` is neither `Send` nor `Sync` and cannot be shared between the producer and consumer the trait exists to serve — the one object-safety test runs on a single thread | `ring_atomic` | **latent hazard** | [api/002](../api/002_a_shared_cell_that_is_not_sync.md) |
| AT8 | The three generic bounds in the family are bare `C : SeqCell`, which a `Cell< u64 >`-backed implementor satisfies — `ring_batch::claim` hands it a claim for eight sequences with no atomicity anywhere in the call | `ring_atomic` | n/a — unenforced | [api/002](../api/002_a_shared_cell_that_is_not_sync.md) |
| AT9 | `CountingSeq` packs five contended atomics into 40 bytes at alignment 8, with no `repr` anywhere in the crate, in a workspace whose `ring_align` exports `CacheAligned< T >` and whose `ring_cursor` uses it | `ring_atomic` | n/a — observation | [data_structure/001](../data_structure/001_one_word_and_five.md) |
| AT10 | Padding the four counters onto their own cache lines is 2× *slower* at 8 and 16 threads (0.50×, 0.49×) because every operation touches its counter and the cell together — the packing is correct, undocumented, and one plausible "fix" from a halving | `ring_atomic` | **measured cost** | [data_structure/001](../data_structure/001_one_word_and_five.md) |
| AT11 | `total` is documented as a definition ("Every operation above, summed") and enforced by nothing — the crate's own test hand-maintains it in a struct literal, and writing `total : 2` there would pass every assertion | `ring_atomic` | n/a — unenforced | [data_structure/002](../data_structure/002_opcounts_and_the_total_it_stores.md) |
| AT12 | Derived `PartialEq` compares the redundant field, so two reports with identical counters can be unequal — against a type whose own test comment says it is comparable "so a test can assert a whole shape at once" | `ring_atomic` | n/a — observation | [data_structure/002](../data_structure/002_opcounts_and_the_total_it_stores.md) |
| AT13 | "This crate never picks one on a caller's behalf" is true of the caller's cell and silent about the crate's own nine `Relaxed` literals — including the four at `:310-313` that make the torn snapshot possible, in a crate whose stated purpose is to make the family's ordering decisions readable in one place | `ring_atomic` | **misleading doc** | [decisions/001](../decisions/001_orderings_named_never_defaulted.md) |
| AT14 | `SeqCst` on every counter operation does not close the torn-snapshot window: ten runs, ten non-zero counts, no ordering effect, and its widest observed skew the largest of the twenty measured — the question is not an ordering question, which nothing states | `ring_atomic` | **measured cost** | [decisions/001](../decisions/001_orderings_named_never_defaulted.md) |
| AT15 | The trait-versus-struct decision is fully realized: both cited criteria exist as quoted, both name a test file that uses `CountingSeq`, and fourteen assertions across four files depend on it — the crate's only decision with a written argument and delivered evidence | `ring_atomic` | n/a — observation | [decisions/002](../decisions/002_a_trait_because_the_criteria_needed_two.md) |
| AT16 | The `ring_batch` criterion asks for "one fence, not 64" and is asserted as one *call*, which the shim's own bookkeeping turns into two hardware atomics — six of the fourteen assertions carry messages naming atomics or operations where they measure calls | `ring_atomic` | **misleading doc** | [decisions/002](../decisions/002_a_trait_because_the_criteria_needed_two.md) |
| AT17 | `ring_seqno` is declared, named in the module comment, the readme, and the task file, and imported nowhere — `cargo udeps --all-targets` confirms it, and it is one of only two unused dependencies in all 33 crates | `ring_atomic` | **wrong doc** | [integration/001](../integration/001_two_declared_one_used.md) |
| AT18 | The one real edge supplies a single newtype, and is indistinguishable from the dead one in the manifest, the readme, the module comment, and the task file — only the compiler can tell them apart | `ring_atomic` | n/a — observation | [integration/001](../integration/001_two_declared_one_used.md) |
| AT19 | Five crates declare `ring_atomic`; `SeqCell` is in scope in twelve via `ring_cursor`'s one-line re-export, by an exact rule with no exceptions — so a trait-level change is a twelve-crate change that every tool shows as five | `ring_atomic` | n/a — observation | [integration/002](../integration/002_five_crates_downstream.md) |
| AT20 | The re-export's own argument says it saves "six manifests" where the count is now eight, and `ring_mpsc` is the only crate in the family whose source acknowledges the trait has two import paths | `ring_cursor` | **wrong doc** | [integration/002](../integration/002_five_crates_downstream.md) |
| AT21 | All three mutating methods move a cell backwards with no refusal, `fetch_add( u64::MAX )` included despite being named and documented as an advance — while `ring_types` and `ring_consume` both name monotonicity as what the family's gates rely on, and the word appears nowhere in this crate | `ring_atomic` | **latent hazard** | [invariant/001](../invariant/001_monotonicity_is_relied_on_and_not_required.md) |
| AT22 | The family's answer is `ring_debug`'s `Violation::CursorWentBackwards` — runtime detection one tier up, demonstrated by a doctest that walks a producer 5 → 7 → 2 — and nothing in this crate records that the trade was made or that the detector exists | `ring_atomic` | n/a — doc gap | [invariant/001](../invariant/001_monotonicity_is_relied_on_and_not_required.md) |
| AT23 | Both concurrency tests, 60,000 operations between them, assert properties of `AtomicU64` reached through a one-line delegation — they cannot fail unless the standard library is wrong | `ring_atomic` | n/a — coverage | [invariant/002](../invariant/002_every_increment_survives.md) |
| AT24 | The shim increments its counter before delegating, so the cell can never lead at any instant — yet 7,176 of 1,000,000 paired reads observed exactly that, because a caller's `counts()` and `load()` are two moments, and the ordering decision behind it is written only in the body | `ring_atomic` | **latent hazard** | [invariant/002](../invariant/002_every_increment_survives.md) |
| AT25 | Both duplications among the six constructors carry a written reason, including the subtle one about not depending on `loom`'s own trait impls — making constructors the best-documented surface in a crate that documents no contract at all | `ring_atomic` | n/a — observation | [item/001](../item/001_six_constructors_for_two_types.md) |
| AT26 | Zero `const` or `static` items in all 33 crates hold a cell, so the capability the `cfg( loom )` split exists for is unused — while those four declarations are exactly where the crate's compile-time coupling to the root manifest lives | `ring_atomic` | n/a — observation | [item/001](../item/001_six_constructors_for_two_types.md) |
| AT27 | `counts`/`reset_counts` are off the trait, so only generic code can be measured — the family's three generic parameters sit in the two crates that count, while `ring_claim::Claimer`'s unbounded CAS retry loop, the one path whose count is not derivable by reading, owns its cursor concretely and admits no substitution | `ring_atomic` | n/a — doc gap | [item/002](../item/002_counts_the_method_that_is_not_a_snapshot.md) |
| AT28 | Both contracts describe an instant neither call delivers — `counts` is four reads, `reset_counts` four stores — and following `reset_counts`' documented recipe costs one of the counts being measured, an effect the crate's own test asserts deliberately and its doctest is ordered to dodge | `ring_atomic` | **misleading doc** | [item/002](../item/002_counts_the_method_that_is_not_a_snapshot.md) |
| AT29 | Five cursor writes across 33 crates and every one computes a forward value (`advanced_by`, `next()`, a clamp) — so the monotonicity the primitive refuses to enforce is supplied entirely by the shape of five expressions, and nothing checks that a sixth would match | `ring_atomic` | n/a — unenforced | [lifecycle/001](../lifecycle/001_born_at_zero_climbing_until_dropped.md) |
| AT30 | A cell has no destructor, no rewind, and no reuse: `ring_shutdown::reset` discards forward and leaves both cursors climbing, so slots recycle and sequences never do — the asymmetry the whole design rests on, stated nowhere in the crate that owns it, and unreadable through `ring_core`'s fourteen public methods | `ring_atomic` | n/a — doc gap | [lifecycle/001](../lifecycle/001_born_at_zero_climbing_until_dropped.md) |
| AT31 | The implementation task sits in `unverified/` with an unwritten Scope declaring the crate may not be claimed yet, against 376 lines and 17 tests — and all 33 tasks are in the same state, so every decision the family made survives only wherever its author happened to put it | `ring_atomic` | n/a — drift | [lifecycle/002](../lifecycle/002_from_task_file_to_crate.md) |
| AT32 | Of three task-file shapes, the two that worked out a Scope are the two most specifically wrong — `ring_tls`'s names `BumpLog` nineteen times as its first deliverable, for a crate that exports `TlsBuffer< T >` and contains no `BumpLog` at all | `ring_tls` | **wrong doc** | [lifecycle/002](../lifecycle/002_from_task_file_to_crate.md) |
| AT33 | A counted `load` costs about 2.7× the production one against `fetch_add`'s 1.9×, because acquire on aarch64 is a load instruction and counting replaces something nearly free with an RMW — so a read-weighted benchmark is distorted harder than a claim-weighted one, and the crate states no budget anywhere against which either could be judged | `ring_atomic` | **measured cost** | [non_functional_requirement/001](../non_functional_requirement/001_what_the_instrument_costs.md) |
| AT34 | The `ring_batch` criterion asks that a 64-slot claim "issues one fence, not 64", and the claim disassembles to nine instructions with one atomic and zero fence instructions on aarch64 — three units in one sentence (fences said, atomics executed, calls asserted), where "one atomic operation, not 64" would cost four words and remove the reconstruction | `bench_harness` | **misleading doc** | [non_functional_requirement/001](../non_functional_requirement/001_what_the_instrument_costs.md) |
| AT35 | `compare_exchange_weak` in the family's own retry-loop shape measures at 0.91×–1.15× with every spread straddling 1.00 and two runs disagreeing on the sign — no effect to find, against a fourth trait method every future implementor would owe | `ring_atomic` | n/a — observation | [non_functional_requirement/002](../non_functional_requirement/002_the_optimizations_that_are_not.md) |
| AT36 | Packing the four counters into one `AtomicU64` as 16-bit lanes measures ~2.5× faster at 8 and 16 threads *and* makes `counts()` a single load, taking impossible snapshots from 17 to 0 in 500,000 — blocked only by a 65,535 ceiling against a largest-known counted run of 40,000 | `ring_atomic` | n/a — observation | [non_functional_requirement/002](../non_functional_requirement/002_the_optimizations_that_are_not.md) |
| AT37 | "This crate is the one place in 33 crates where an atomic is *created*" is false — ten raw atomics live in `ring_stats`, `ring_shutdown` and `ring_bench` — and the unstated boundary hides a real question, since `ring_stats` holds seven concurrently-mutated `AtomicU64`s with zero `cfg( loom )` sites; inserting the word *sequence* makes the sentence true and the exception visible | `ring_atomic` | **wrong doc** | [pattern/001](../pattern/001_one_place_where_an_atomic_is_created.md) |
| AT38 | "Instrumenting it here instruments all of them" holds for the `loom` swap and not for the counting instrument, which reaches only code generic over `SeqCell`; "no other crate needs to know the seam exists" holds for sibling crates and not for the workspace manifest above them, which must carry the `check-cfg` entry | `ring_atomic` | **misleading doc** | [pattern/001](../pattern/001_one_place_where_an_atomic_is_created.md) |
| AT39 | Thirteen counting assertions across `ring_batch` and `ring_tls` are licensed by one twenty-line single-threaded parity test over five fixed values, which never exercises `fetch_add( 0 )`, the `u64::MAX` wrap, a `compare_exchange` whose `current` equals `new`, any other ordering, or any concurrency | `ring_atomic` | n/a — coverage | [pattern/002](../pattern/002_the_counting_cell_is_not_a_mock.md) |
| AT40 | The doc comment argues the substitution's fidelity at length — "behaves exactly like `AtomicSeq`", "the same values production would" — and bounds it nowhere, while the type costs ~1.9×/~2.7× per call and 40 bytes against 8, is exported with no feature gate, no `cfg( test )` and no `doc( hidden )`, and is kept out of production by convention across thirteen construction sites | `ring_atomic` | **misleading doc** | [pattern/002](../pattern/002_the_counting_cell_is_not_a_mock.md) |
| AT41 | `Seq::next` refuses to wrap and states the stakes correctly, but production advances through `AtomicSeq::fetch_add`, which wraps identically in debug and release — so the guard is absent from the only path that carries traffic, and the owning crate mentions overflow, wrap and saturation zero times in source or tests; release also disproves `next`'s own claim to saturate, yielding `0` | `ring_atomic` | **latent hazard** | [pitfall/002](../pitfall/002_the_wrap_that_reads_as_an_empty_ring.md) |
| AT42 | Past the wrap every gate in the family grants — `free_slots` reports 64 of 64, `may_claim` true, `headroom` 64, `check` `Ok(())` — because `distance_to`'s saturation fails open and runs before `free_slots`' saturation, which would have failed safe; backpressure does not degrade, it inverts, and no `RingError` is ever constructed | `ring_atomic` | **latent hazard** | [pitfall/002](../pitfall/002_the_wrap_that_reads_as_an_empty_ring.md) |
| AT43 | About 1 snapshot in 100 reports a state the cell was never in, off by up to 1,198 operations, and all of them pass `total == loads + stores + fetch_adds + compare_exchanges` — the only cross-field relation `OpCounts` exposes is derived from the torn reads, so the check a suspicious caller would reach for is the one that cannot detect the tear | `ring_atomic` | **latent hazard** | [pitfall/001](../pitfall/001_the_snapshot_that_never_happened.md) |
| AT44 | `counts()`' one-line contract reads as an instant, its doctest is single-threaded, `reset_counts` describes the quiescent pattern without requiring it, and `counts_are_exact_under_contention` — the only place "counts" and "contention" meet — reads after every thread has joined; sixteen downstream call sites obey a discipline nothing states | `ring_atomic` | **misleading doc** | [pitfall/001](../pitfall/001_the_snapshot_that_never_happened.md) |
| AT45 | `SeqCell` declares no supertrait, so a `Cell< u64 >`-backed implementation satisfies it and is accepted by `ring_batch::claim`, the family's multi-producer entry point; all three generic bounds in 33 crates are bare `C : SeqCell`, and both real implementations are `Sync` only by auto-derivation — `pub trait SeqCell : Sync` states the actual requirement at zero cost | `ring_atomic` | **latent hazard** | [type/001](../type/001_what_the_trait_promises.md) |
| AT46 | `fetch_add` returns "the first sequence the caller now owns" and discarding it compiles silently, permanently stranding that range; only `compare_exchange` warns, and only because std marks `Result` — meanwhile the crate spends all five of its `#[ must_use ]` on four constructors and `counts()`, and `ring_types` spends eleven more on pure functions over a `Copy` type | `ring_atomic` | **latent hazard** | [type/001](../type/001_what_the_trait_promises.md) |
| AT47 | `OpCounts` carries `PartialEq`/`Eq` so a caller can assert a whole shape at once, and downstream takes that option zero times across 16 field reads; all nine whole-value uses are in `ring_atomic`, the three comparisons all in its own test file hand-writing `total` — while the bundling is what forces four separate reads and what creates a field able to contradict its operands | `ring_atomic` | n/a — observation | [type/002](../type/002_the_report_that_is_all_public.md) |
| AT48 | The crate gives its two cells four `new` functions and two hand-written `Default` impls, and its one report no inherent impl at all — correct for a record, but `OpCounts` is a *reading*, and `Copy` with public fields leaves a value from a live cell indistinguishable from a hand-written literal and a snapshot taken under contention indistinguishable from one taken quiet | `ring_atomic` | n/a — observation | [type/002](../type/002_the_report_that_is_all_public.md) |
| AT49 | The crate absorbs loom's missing `const` constructors and uncertain trait impls at six `cfg` sites, two duplicated constructors and one hand-written `Default`, but cannot absorb the `check-cfg` entry — Cargo forbids a crate from both inheriting the workspace lints table and extending it — so lifted out of the workspace it fails under `-D warnings` with one error per `cfg` site, six in all | `ring_atomic` | n/a — observation | [workaround/001](../workaround/001_the_loom_seam_and_the_manifest_above_it.md) |
| AT50 | The `check-cfg` entry's comment says "Only ring_atomic, ring_cursor and ring_publish read the cfg" where six files across six crates do and five declare `loom` as a dependency — the word "Only" would lead a reader to think removing one crate's loom test frees the entry, and this is the fourth never-recomputed count in this crate's neighbourhood | root `Cargo.toml` | **wrong doc** | [workaround/001](../workaround/001_the_loom_seam_and_the_manifest_above_it.md) |
| AT51 | `AtomicU64` and `AtomicUsize` are switched by `cfg` while `Ordering` is imported unconditionally from `core` one line below, which is correct only because loom re-exports core's type rather than instrumenting its own — verified by building the loom branch clean, and recorded nowhere, so the asymmetry reads as an oversight rather than the deliberate choice it is | `ring_atomic` | n/a — doc gap | [workaround/002](../workaround/002_what_the_seam_does_not_switch.md) |
| AT52 | All 17 tests compile under `--cfg loom` and every one panics at the first atomic access, because none opens a `loom::model` and the family's fifteen live in four other crates — so `cargo build --tests` reports the loom configuration green, and a behavioural break in the seam's own branch would surface only three or more dependency edges downstream | `ring_atomic` | n/a — coverage | [workaround/002](../workaround/002_what_the_seam_does_not_switch.md) |

### Reading

The split is exactly even: 26 of the 52 name something a caller can reach today, and
26 record a property of the crate as it stands. That evenness is itself the finding.
`ring_atomic` is 376 lines with no branch on the production path, no allocation, no
`unsafe` and no destructor — every `AtomicSeq` method is one intrinsic with a newtype
around the result (AT1) — so almost nothing in it *can* be wrong in the ordinary
sense, and the reachable half is not about code being incorrect.

It is about what the code declines to say. Every one of the ten latent hazards lives
in a decision the crate made deliberately and wrote down nowhere: the trait that
declines `Sync` and is satisfied by a `Cell< u64 >` (AT7, AT45); the three returning
methods that decline `#[ must_use ]`, including the one whose return is a caller's
only record of an irreversible claim (AT5, AT46); the four separately-timed loads
behind a method whose contract reads as an instant (AT3, AT43); the advance that
wraps where its arithmetic sibling panics (AT41, AT42); the counter incremented before
the cell it describes (AT24); the backwards move nothing refuses (AT21). What the
crate *does* explain thoroughly — why a trait, why the `const` split, why `Default`
is hand-written (AT15, AT25) — is the part nobody could get wrong by accident.

### The Four Threads

**One — a defence converts an anomaly into a plausible number.** This is the pitfall
definition's whole subject and it appears twice with different machinery. Past the
`u64` wrap, `distance_to`'s saturation reports an overrun ring as empty and every gate
in the family then grants (AT42); under contention, `OpCounts::total` is derived from
the same four torn reads it would be used to check, so it certifies every impossible
snapshot (AT43). In both cases the value that reaches the caller is in range, is
well-typed, and is wrong.

**Two — guard ordering decides the outcome, not guard strength.** Three defences sit
on the wrap chain: a `debug_assert`-grade panic in `Seq::next`, a fail-open saturation
in `distance_to`, and a fail-safe saturation in `free_slots`. The chain passes all
three, because the panic guards the arithmetic path while production advances through
the atomic one, and the fail-open clamp runs first (AT41, AT42).

**Three — requirements satisfied by accident of composition.** `SeqCell` declares no
supertrait; all three generic bounds in 33 crates are bare `C : SeqCell`; both real
implementations are `Sync` only because the atomics inside them are (AT8, AT45). The
requirement is real, is met everywhere, and is stated nowhere — so the gap has never
been reached and nothing would notice a third implementation that closed it.

**Four — counts written once and never recomputed.** Four instances in this crate's
neighbourhood: "the one place in 33 crates where an atomic is created" against ten raw
atomics elsewhere (AT37), `ring_cursor`'s "six manifests" against eight (AT20), the
module comment naming `handshake_test.rs` as the loom seam's only user against four
test files, and the root manifest's "Only ring_atomic, ring_cursor and ring_publish"
against six crates (AT50). Each was true when written; none has a check that would
notice the drift, and every one is a single `grep` away.

### Severity

| Tier | Findings | Why |
|------|----------|-----|
| **latent hazard** | AT3, AT5, AT7, AT21, AT24, AT41, AT42, AT43, AT45, AT46 | Reachable from a caller writing ordinary, compiling code — a stranded claim, a torn report, an inverted gate, a backwards cursor |
| **wrong doc** | AT17, AT20, AT32, AT37, AT50 | A statement in source, manifest or task file that is false as written and would mislead a reader acting on it |
| **misleading doc** | AT13, AT16, AT28, AT34, AT38, AT40, AT44 | True on its own terms and read as broader than it is — a scope, a unit, or an instant the call does not deliver |
| **measured cost** | AT2, AT10, AT14, AT33 | A price paid on a real path, measured here, against no budget anywhere in the crate |
| n/a — doc gap | AT4, AT22, AT27, AT30, AT51 | A deliberate design choice with no record, so it reads as an oversight |
| n/a — unenforced | AT6, AT8, AT11, AT29 | A property the family relies on that no type, test or lint requires |
| n/a — coverage | AT23, AT39, AT52 | A test or configuration that passes without exercising what it appears to |
| n/a — drift | AT31 | Process state that no longer matches the artifact it governs |
| n/a — observation | AT1, AT9, AT12, AT15, AT18, AT19, AT25, AT26, AT35, AT36, AT47, AT48, AT49 | Correct as written; recorded because it is load-bearing for something else here |
