# Doc Definitions

Module Index for `ring_stats` — every doc definition this crate declares and every
instance under each, in one place.

## Master Doc Definitions Table

| Type | Purpose | Master File | Instances |
|------|---------|-------------|----------:|
| `algorithm/` | Eighteen atomic operations, ten of them a single instruction, and the compositions that are not | [algorithm/readme.md](../algorithm/readme.md) | 2 |
| `api/` | Sixteen methods, no exclusive borrow anywhere, and the snapshot the surface long refused | [api/readme.md](../api/readme.md) | 2 |
| `data_structure/` | Seven counters in 56 bytes, and three drop counters behind an enum, one of which is not a drop | [data_structure/readme.md](../data_structure/readme.md) | 2 |
| `decisions/` | One ordering applied without exception, and the one requirement clause with a design consequence | [decisions/readme.md](../decisions/readme.md) | 2 |
| `definition/` | Module Index — every definition and every instance in this crate, in one place | this file | — |
| `integration/` | Both halves of the edge census — a write path with two absent callers, a read path with a refused edge | [integration/readme.md](../integration/readme.md) | 2 |
| `invariant/` | The property that holds per counter, and the cross-counter relation nothing enforces | [invariant/readme.md](../invariant/readme.md) | 2 |
| `item/` | Declaration-level reference — the five recorders as one shape, and the two methods that are not readings | [item/readme.md](../item/readme.md) | 2 |
| `lifecycle/` | A counter set from zero to reset to zero, and a crate that ships while its feature stays planned | [lifecycle/readme.md](../lifecycle/readme.md) | 2 |
| `non_functional_requirement/` | The one property the crate claims, and the one it needs and does not state | [non_functional_requirement/readme.md](../non_functional_requirement/readme.md) | 2 |
| `pattern/` | The record-and-read mirror, and the derived reading assembled from separate loads | [pattern/readme.md](../pattern/readme.md) | 2 |
| `pitfall/` | The two readings that fail precisely under the traffic that prompts anyone to take them | [pitfall/readme.md](../pitfall/readme.md) | 2 |
| `type/` | What `AtomicU64` forecloses, and one width standing for two kinds of quantity | [type/readme.md](../type/readme.md) | 2 |
| `workaround/` | Two language constraints absorbed correctly, each missing the check that the absorption still holds | [workaround/readme.md](../workaround/readme.md) | 2 |

**13 definitions, 26 instances, 52 findings.** Regenerate all three counts:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_stats/docs
find . -mindepth 1 -maxdepth 1 -type d ! -name definition | wc -l   # 13
find . -name '[0-9][0-9][0-9]_*.md' | wc -l                          # 26
command grep -rho '^### ST[0-9]*' . | wc -l                          # 52
```

## Master Doc Instances Table

| Type | ID | Title | Carries |
|------|----|-------|---------|
| `algorithm/` | 001 | [Eleven Operations, and the Three That Are Not One](../algorithm/001_eleven_operations_and_three_compositions.md) | Every atomic site in the crate, and the three methods that issue more than one |
| `algorithm/` | 002 | [`in_flight` Subtracts One Moment From Another](../algorithm/002_in_flight_subtracts_two_moments.md) | The direction the seam fails in, measured, and why that direction has no tell |
| `api/` | 001 | [Fourteen Methods, and No Way to Borrow the Type Exclusively](../api/001_fourteen_methods_and_no_exclusive_borrow.md) | Every signature, the eight `must_use` marks, and what `&self` on `reset` costs |
| `api/` | 002 | [Seven Readers, and No Way to Read the Set](../api/002_seven_readers_and_no_way_to_read_the_set.md) | The traits declined, the snapshot type that does not exist, and who assembles one anyway |
| `data_structure/` | 001 | [Seven Counters on One Cache Line](../data_structure/001_seven_counters_on_one_line.md) | 56 bytes, no `repr`, measured 2.6×–3.3×, and the unused padding crate next door |
| `data_structure/` | 002 | [Three Drop Counters Behind One Enum, One of Which Is Not a Drop](../data_structure/002_three_drop_counters_behind_one_enum.md) | The enum index, the break in field naming, and the total that erases it |
| `decisions/` | 001 | [`Relaxed` Everywhere, With a Reason That Covers One Load](../decisions/001_relaxed_with_a_reason_that_covers_one_load.md) | The ordering argument, its uniform application, and a measurement of the cost it names |
| `decisions/` | 002 | [Per-Policy Drops Delivered, and Five Methods Nobody Asked For](../decisions/002_per_policy_drop_counters_stated_and_delivered.md) | What this crate was originally asked to count, what the crate ships, and where the findings landed |
| `integration/` | 001 | [The Write Path, and Two Callers That Are Not There](../integration/001_the_write_path_and_two_callers_that_are_not_there.md) | Every production recorder call, `resolve`'s importers, and `reset`'s absent caller |
| `integration/` | 002 | [The Read Path, and the Edge That Was Assigned and Removed](../integration/002_the_read_path_and_the_removed_edge.md) | `ring_bench`'s post-hoc totals, `ring_factory`'s recorded refusal, and one dependency |
| `invariant/` | 001 | [Monotone Per Counter, and Only Per Counter](../invariant/001_monotone_per_counter_and_only_per_counter.md) | The property `Relaxed` supports, `reset` as its one exception, and the derived reading that lacks it |
| `invariant/` | 002 | [`claimed` Never Trails `published` — Assumed, Named, Enforced Nowhere](../invariant/002_claimed_never_trails_published.md) | The cross-counter contract, who owns it, and the three routes to a zero reading |
| `item/` | 001 | [Five Recorders, and the One Nothing Calls](../item/001_five_recorders_and_the_one_nothing_calls.md) | The write-side family, the explicit-amount contract, and `wait_nanos`' missing producer |
| `item/` | 002 | [The Two Methods That Are Not One Operation](../item/002_the_two_methods_that_are_not_one_operation.md) | `in_flight` among the counters, and `reset` against `new` |
| `lifecycle/` | 001 | [Zero, Counting, Reset, Zero — and the Moment in Between](../lifecycle/001_zero_to_reset_to_zero.md) | The runtime states, and the reset window measured from a second thread |
| `lifecycle/` | 002 | [Implemented, Tested, and Still Planned](../lifecycle/002_implemented_tested_and_still_planned.md) | This crate's own feature-record status, the field it sits on, and the one consumer awaiting these counters |
| `non_functional_requirement/` | 001 | [Cheap Enough to Leave On](../non_functional_requirement/001_cheap_enough_to_leave_on.md) | The stated requirement, measured per call and per system |
| `non_functional_requirement/` | 002 | [Counters That Must Not Distort What They Measure](../non_functional_requirement/002_counters_that_must_not_distort_what_they_measure.md) | The requirement the crate does not state, and what it offers a caller who needs it |
| `pattern/` | 001 | [The Record-and-Read Pair](../pattern/001_the_record_and_read_pair.md) | The mirror, its exactness, and the distinction it spent to stay uniform |
| `pattern/` | 002 | [The Derived Reading From Separate Loads](../pattern/002_the_derived_reading_from_separate_loads.md) | The fold, against the parameterised and snapshot forms the family already has |
| `pitfall/` | 001 | [The Leak `in_flight` Cannot See](../pitfall/001_the_leak_in_flight_cannot_see.md) | The detector that reports health, and why swapping the two loads makes it worse |
| `pitfall/` | 002 | [The Total That Counts a Refusal as a Loss](../pitfall/002_the_total_that_counts_a_refusal_as_a_loss.md) | The safest policy producing the largest loss figure, and the breakdown that tears |
| `type/` | 001 | [A Type That Cannot Be Copied, Compared or Cloned](../type/001_a_type_that_cannot_be_copied_compared_or_cloned.md) | The derive list, the traits the fields foreclose, and `Debug`'s seven loads |
| `type/` | 002 | [Seven Counters and One Width](../type/002_seven_counters_and_one_width.md) | One `u64` for two kinds of quantity, and a duration summed across threads |
| `workaround/` | 001 | [A Floor That Absorbs More Than It Was Built For](../workaround/001_a_floor_that_absorbs_more_than_it_was_built_for.md) | `saturating_sub`, the caller-bug rationale, and what it actually catches |
| `workaround/` | 002 | [Seven Counters, Enumerated Four Times by Hand](../workaround/002_seven_counters_enumerated_four_times_by_hand.md) | `reset`'s array, the three lists beside it, and the family's own check |

## Findings

Fifty-two, each verified by a command whose output is quoted in its instance.

| ID | Finding | Subject | Reachable | Where |
|----|---------|---------|-----------|-------|
| ST1 | Ten public methods compile to one atomic operation and a move — ten of fourteen when this was written, ten of sixteen since `snapshot` and `checked_in_flight` joined the methods that are not one operation — with no loop, no retry and no arithmetic on a value read back, and the two `match` arms select a field rather than decide anything, so the production path has no algorithm of its own and every question worth asking is about the compositions and the layout beneath them | `ring_stats` | n/a — observation | [algorithm/001](../algorithm/001_eleven_operations_and_three_compositions.md) |
| ST2 | `dropped_total`, `in_flight` and `reset` issue three, two and seven atomic operations, and each is documented as a single reading or action over the whole set — while the module comment's ordering rationale ("a stats read is a diagnostic, never a synchronisation point") is true of the ten single-operation methods and silent about these three; `snapshot`, added later, issues seven more and is the one composition whose own doc opens by naming the count and denying atomicity | `ring_stats` | n/a — doc gap | [algorithm/001](../algorithm/001_eleven_operations_and_three_compositions.md) |
| ST3 | `in_flight` loads `claimed` before `published` and both climb, so the result is understated by exactly the traffic passing through the window and can never be overstated — measured, a fixed leak of 8 read as fewer than 8 in 13,099 of 2,000,000 samples and as *zero* in 27, while `saturating_sub` floors the wrong answer at the value a healthy ring reports | `ring_stats` | **latent hazard** | [algorithm/002](../algorithm/002_in_flight_subtracts_two_moments.md) |
| ST4 | The one `in_flight` bug the family caught — 240 leaked slots reported on a 16-slot ring, written up in `ring_bench`'s test file — announced itself by exceeding a known bound, which the under-reporting direction cannot do; all 16 call sites are tests, none production, and the only assertion made under contention still reads after every producer has joined | `ring_stats` | **misleading doc** | [algorithm/002](../algorithm/002_in_flight_subtracts_two_moments.md) |
| ST5 | Every method but `new` takes `&self` and none takes `&mut self` — thirteen of fourteen when this was written, fifteen of sixteen now — which is correct for a set shared by every producer and consumer, but is also the reason `reset`, the one method whose contract covers all seven counters, has no choice but seven independent stores, and neither half of that trade is written on the method | `ring_stats` | n/a — doc gap | [api/001](../api/001_fourteen_methods_and_no_exclusive_borrow.md) |
| ST6 | Every value-returning method carries `#[ must_use ]` and all six `()`-returning ones correctly do not — eight against eight when this was written, ten against ten since, with both later additions marked on arrival — giving total coverage spent entirely on returns that can be re-obtained by calling again, the family's habit of applying the attribute by position rather than by consequence, at its most complete | `ring_stats` | n/a — observation | [api/001](../api/001_fourteen_methods_and_no_exclusive_borrow.md) |
| ST7 | `RingStats` is the live counter set, deriving `Debug` and `Default` and nothing else, so for a time no operation on the API returned more than one counter and even `{:?}` is seven reads at seven moments — the crate's three multi-counter methods were the three places it needed one anyway, and all three are findings; `RingStats::snapshot` is now a fourth, returning a `StatsCounts` whose own parts agree with each other though its loads are still seven | `ring_stats` | n/a — doc gap | [api/002](../api/002_seven_readers_and_no_way_to_read_the_set.md) |
| ST8 | `ring_atomic` ships `OpCounts` — a `Copy`, comparable four-field value returned by `counts()` — so the family already had the snapshot pattern, one crate over, with the tear confined to a single documentable function; `ring_stats` declined it without recording the choice, distributing the same tear across every caller, until `RingStats::snapshot` and `StatsCounts` brought the shape into this crate too | `ring_stats` | n/a — duplication | [api/002](../api/002_seven_readers_and_no_way_to_read_the_set.md) |
| ST9 | Seven `AtomicU64` with no `repr` pack into 56 bytes on one cache line while producers, consumers and the overflow path each write different fields and none reads a neighbour — measured as a median of nine paired ratios, the shipped layout costs 2.6×–3.3× the same counters given a line each, at 3, 6 and 12 threads, with all eighteen ratios above 2.1× | `ring_stats` | **measured cost** | [data_structure/001](../data_structure/001_seven_counters_on_one_line.md) |
| ST10 | The workspace has measured cache-line packing twice with opposite answers — 2× *faster* packed for `ring_atomic`'s `CountingSeq`, ~2.9× *slower* packed here — and neither struct records which regime it is in, so both obvious readings ("pad these" and "the family packs counters") are correct for one and a regression for the other | `ring_stats` | n/a — doc gap | [data_structure/001](../data_structure/001_seven_counters_on_one_line.md) |
| ST11 | `dropped( OverflowPolicy::Fail )` is a method whose name asserts a loss, reading a field named `failed` because it is not one, keyed by a variant documented as handing the item back to the caller — the storage and `ring_types`' own `reports_failure()`/`drops_silently()` predicates all record the distinction, and the only public way to ask for the refusal count is to ask how many were dropped | `ring_stats` | **misleading doc** | [data_structure/002](../data_structure/002_three_drop_counters_behind_one_enum.md) |
| ST12 | The module comment argues that drop causes must never be collapsed into one bucket, and `dropped_total` folds all three counters into exactly that — including refusals, which lose nothing — while `OverflowPolicy::drops_silently()` sits in this crate's only dependency as a ready one-line filter, and `refusals_are_counted_even_though_nothing_is_lost` pins the conflated total its own name denies | `ring_stats` | n/a — unenforced | [data_structure/002](../data_structure/002_three_drop_counters_behind_one_enum.md) |
| ST13 | Eighteen atomic sites, all `Relaxed`, no exceptions — eleven of them when this was written, and every site added since took the same ordering — against `ring_cursor` at fourteen `Acquire`/`Release` sites and no `Relaxed`, and `ring_atomic` mixing all four, making this the one crate in the group that made a single ordering choice, applied it everywhere, and wrote down why | `ring_stats` | n/a — observation | [decisions/001](../decisions/001_relaxed_with_a_reason_that_covers_one_load.md) |
| ST14 | The fence the ordering decision was made to avoid does not measure: `Relaxed` against `AcqRel` on the same counters gives medians within 10% of 1.0 at 1, 3, 6 and 12 threads with the sign flipping between runs — on AArch64 the difference is `ldadd` against `ldaddal`, and on x86-64 a `lock`-prefixed RMW is already a full barrier — while the layout those same operations run in costs 2.6×–3.3× | `ring_stats` | **measured cost** | [decisions/001](../decisions/001_relaxed_with_a_reason_that_covers_one_load.md) |
| ST15 | "And under which policy" is the only clause in this crate's own originating requirement that constrains storage rather than naming a number, was not present in the design record it descends from, and is implemented past the letter — three fields, exhaustive `match` in both directions, rationale restated in the implementer's own words | `ring_stats` | n/a — observation | [decisions/002](../decisions/002_per_policy_drop_counters_stated_and_delivered.md) |
| ST16 | Seven fields for four requested counters and five public methods on subjects this crate's own originating requirement never names — and every method-specific finding in this corpus lands on three of those five (`in_flight`, `reset`, `dropped_total`), while the four requested counters have produced none, each being one atomic operation that does what its name says | `ring_stats` | n/a — coverage | [decisions/002](../decisions/002_per_policy_drop_counters_stated_and_delivered.md) |
| ST17 | The workspace's only non-benchmark line that moves a counter is `record_drop` inside `ring_overflow::resolve`, which no `src/` anywhere imports — `ring_core` handles the full-ring case through `would_resolve`, the documented counter-free half, and does not declare `ring_stats` — so a record dropped in production moves nothing, and this crate's own promised diagnosis is unreachable from the write path with nothing recording that | `ring_stats` | **latent hazard** | [integration/001](../integration/001_the_write_path_and_two_callers_that_are_not_there.md) |
| ST18 | `RingStats::reset`'s doc states it is "Used by `ring_shutdown`'s reset", and `ring_shutdown` neither declares `ring_stats` in its four dependencies nor contains the string `stats` in any `.rs` file — the call could not compile, and `reset()` is invoked only from two tests in this crate and its own doctest | `ring_stats` | **wrong doc** | [integration/001](../integration/001_the_write_path_and_two_callers_that_are_not_there.md) |
| ST19 | `ring_bench::Outcome` is the family's only live consumer and its accessor's own doc says the counters are "written once from totals after the clock stopped" — one thread, one write per counter, `claimed` equal to `published`, one policy per run — so the sole worked example of `RingStats` in use exercises none of the concurrent behaviour that every finding in this corpus concerns | `ring_stats` | n/a — coverage | [integration/002](../integration/002_the_read_path_and_the_removed_edge.md) |
| ST20 | `ring_factory` refuses a `ring_stats` edge in five lines of manifest comment carrying the reasoning, the resolved decision reference and the reversal condition — while `ring_stats`' own manifest declares one dependency and says nothing about `ring_align`, whose `CacheAligned` addresses the 2.6×–3.3× its layout costs; the same absence, documented to opposite standards | `ring_stats` | n/a — observation | [integration/002](../integration/002_the_read_path_and_the_removed_edge.md) |
| ST21 | `each_counter_is_monotone_while_writers_run` was the only test in the file that read a counter while another thread wrote, and it samples `published` alone — one of seven, in a body whose name and doc both say "every counter" — pointing the crate's window into concurrent behaviour at the one read with no composition, no second counter, and no seam; two later tests read a derived value and a reset window under live writers, leaving this one still the narrowest of the three | `ring_stats` | n/a — coverage | [invariant/001](../invariant/001_monotone_per_counter_and_only_per_counter.md) |
| ST22 | Monotonicity is stated on the counters, where it holds, and justified by "the one a live progress display needs" — while `dropped_total` inherits it and `in_flight` cannot, being a gauge that moves in both directions and under-reports whenever it does, so the named use case reaches for the one reading the property excludes | `ring_stats` | **misleading doc** | [invariant/001](../invariant/001_monotone_per_counter_and_only_per_counter.md) |
| ST23 | `claimed >= published` is depended on by `in_flight`, named in a test doc as "a caller bug" when violated, and enforced by nothing — the crate holds zero `debug_assert` — while `saturating_sub` reports the violation as `0`, the value a healthy ring returns, where `checked_sub` would have reported it as `None` for one word of difference | `ring_stats` | **latent hazard** | [invariant/002](../invariant/002_claimed_never_trails_published.md) |
| ST24 | `in_flight() == 0` is reachable three ways — balanced ring, caller bug floored by saturation, and real leak masked by the load window — two of which were chosen deliberately, while the method's doc scopes its claim to nonzero readings taken at rest and says nothing about what a zero does or does not establish | `ring_stats` | **misleading doc** | [invariant/002](../invariant/002_claimed_never_trails_published.md) |
| ST25 | All five recorders take `&self` and an explicit `u64` amount and add it to one counter, with no implicit-one form anywhere — which is what makes `record_claim( 0 )` a meaningful no-op, makes `n` single calls provably equal one batched call, and lets a caller write a whole run's totals in four calls; shape, contract and tests agree exactly, the one place in the crate where they do | `ring_stats` | n/a — observation | [item/001](../item/001_five_recorders_and_the_one_nothing_calls.md) |
| ST26 | `record_wait` is invoked by nothing outside this crate — while `wait_nanos` is one of the four counters this crate was originally asked to provide, and `ring_wait`, the crate that spins, yields and sleeps 50µs at a time, measures none of it and declares `ring_types` and `ring_cursor` rather than `ring_stats` — so `wait_nanos()` returns zero in every buildable configuration and zero legitimately means nothing waited | `ring_stats` | **latent hazard** | [item/001](../item/001_five_recorders_and_the_one_nothing_calls.md) |
| ST27 | Six readers are past participles or totals answering how much has happened, each one monotone; `in_flight` is present tense, a gauge that rises and falls with no history — and the API presents both kinds with the same return type, the same `#[ must_use ]`, and docs of the same length, so nothing separates the six that accumulate from the one that observes | `ring_stats` | n/a — doc gap | [item/002](../item/002_the_two_methods_that_are_not_one_operation.md) |
| ST28 | `new` is a `const fn` building a value with no atomic operation in its body, and `reset` walks seven counters through a shared reference — the same all-zero postcondition reached atomically by one and observably torn by the other, eight lines apart in the same `impl` under one-line docs a clause apart, with nothing marking that a caller choosing between them is choosing a guarantee | `ring_stats` | n/a — doc gap | [item/002](../item/002_the_two_methods_that_are_not_one_operation.md) |
| ST29 | `reset` stores zero into seven counters in field order through a shared reference, and because a fill sets `claimed` before `wait_nanos` while a reset clears them in the same order, a concurrent read caught mid-reset returns `( 0, set )` — a combination no fill produces and no complete state can hold, measured at 101, 5, 69, 30, 16 and 16 catches per two million samples across six runs, against a doc that describes only the postcondition | `ring_stats` | **latent hazard** | [lifecycle/001](../lifecycle/001_zero_to_reset_to_zero.md) |
| ST30 | Both reset tests called `reset` and asserted afterwards on a single thread, spawning zero threads between them, which is precisely the condition under which the window cannot appear; `a_reader_beside_a_reset_sees_only_values_the_writer_wrote` now runs a reader through 20,000 fill-then-reset rounds and asserts on every read that `claimed` and `wait_nanos` each hold a value the writer actually wrote, leaving the two originals single-threaded and correct as the postcondition checks they are | `ring_stats` | n/a — coverage | [lifecycle/001](../lifecycle/001_zero_to_reset_to_zero.md) |
| ST31 | This crate's own feature record now reads `Status: present`, flipped as part of the contiguous 167-188 block rather than by any check of this crate — `present` is a minority of the records and falls in eight contiguous runs, two of them 22 features long, and the census caught a ninth feature joining one of those runs between two gate runs while nothing about this crate changed; so the field is a per-block marker that cannot carry a per-feature fact, and the crate's two half-deliveries (a named counter with no producer, a documented caller with no dependency edge) still have nowhere to surface | `ring_stats` | **misleading doc** | [lifecycle/002](../lifecycle/002_implemented_tested_and_still_planned.md) |
| ST32 | The one external record that reads these counters rather than produces them names `drops` alone out of seven, is itself `planned`, and quotes its source's own escape clause — included "when the cost of the counters is already being paid and dropped when it is not" — making the crate's sole documented consumer conditional on the one property of the crate that has never been measured | `ring_stats` | n/a — observation | [lifecycle/002](../lifecycle/002_implemented_tested_and_still_planned.md) |
| ST33 | `record_claim` measures six to ten nanoseconds over an identical loop touching no counter, so the crate's one stated non-functional requirement is met — while the crate itself has no `benches/` directory, no `Instant` anywhere in its suite, and no assertion bounding any cost, so the only property it claims is the only one it never checks | `ring_stats` | n/a — coverage | [non_functional_requirement/001](../non_functional_requirement/001_cheap_enough_to_leave_on.md) |
| ST34 | Per-call cost is flat from one thread to twelve, which fixes aggregate throughput at roughly 100M records per second regardless of core count — and neither escape works: splitting threads across `claimed` and `consumed` measures the same as loading one counter, because all seven fields share a cache line, so the two counters contend as one | `ring_stats` | **measured cost** | [non_functional_requirement/001](../non_functional_requirement/001_cheap_enough_to_leave_on.md) |
| ST35 | The crate repeats "cheap enough to leave on" twice without naming a single condition under which it does not apply, and the one consumer that hit such a condition — `ring_bench`, whose counted events are its timed events — had to write the qualification itself, in its own docs, where it does not generalise to the next consumer | `ring_stats` | n/a — doc gap | [non_functional_requirement/002](../non_functional_requirement/002_counters_that_must_not_distort_what_they_measure.md) |
| ST36 | A crate whose entire purpose is instrumentation ships no mechanism for controlling the instrument: no `[features]` section, no `cfg( feature = … )`, no no-op variant and no sharded recorder — so the only way to decline the cost is to stop calling, and the sharding that would have removed the contention entirely was rejected downstream partly because this crate does not provide it | `ring_stats` | n/a — doc gap | [non_functional_requirement/002](../non_functional_requirement/002_counters_that_must_not_distort_what_they_measure.md) |
| ST37 | Five recorders and five counter-naming readers correspond exactly, including `record_drop( policy, n )` against `dropped( policy )` with the same three-arm `match` written out byte-for-byte on both sides — so no counter can be written and not read or read and not written, and the two readers that name no counter stand out against the mirror rather than needing to be discovered | `ring_stats` | n/a — observation | [pattern/001](../pattern/001_the_record_and_read_pair.md) |
| ST38 | The mirror requires one verb per counter family, so all three `OverflowPolicy` variants are written by `record_drop` and read by `dropped` — including `Fail`, where the ring refuses the push and loses nothing, whose field is honestly named `failed` and whose accessors both say drop, which is what lets `dropped_total` report a thousand refusals as a thousand losses while `ring_types::reports_failure` sits one call away | `ring_stats` | **misleading doc** | [pattern/001](../pattern/001_the_record_and_read_pair.md) |
| ST39 | The workspace solves "derive a reading from several atomics" three ways — `ring_seqno::free_slots` takes the values as parameters and performs no load, `ring_atomic::counts` derives `total` from its own four locals so the whole always matches the parts returned with it, and `ring_stats` loads inside the accessor and returns the fold alone — and only the third can return a value no state held while withholding the components that would show it; `RingStats::snapshot` now offers the second shape beside it, leaving the crate with both and the accessors unchanged | `ring_stats` | n/a — observation | [pattern/002](../pattern/002_the_derived_reading_from_separate_loads.md) |
| ST40 | The five exact single-load readers are the ones nobody looks at, and the two derived ones are exactly what the crate puts forward — `in_flight` as the leak detector its module comment describes, `dropped_total` as the number a monitor prints — while the ordering rationale reasons about "a stats read" in the singular, which is the right unit for the five and the wrong one for the two | `ring_stats` | n/a — doc gap | [pattern/002](../pattern/002_the_derived_reading_from_separate_loads.md) |
| ST41 | `in_flight` loads `claimed` before `published` so its error is always downward, and `saturating_sub` floors that error at zero — the exact reading a healthy ring gives — so against a permanent eight-slot leak, two million readings understate it around one percent of the time and report no leak at all seven to sixteen times, with nothing in the value distinguishing the miss from a correct answer | `ring_stats` | **latent hazard** | [pitfall/001](../pitfall/001_the_leak_in_flight_cannot_see.md) |
| ST42 | Swapping the two loads removes every understatement and every zero reading against a real leak, and introduces overstatement on a ring with **no** leak — more than one outstanding slot reported around three percent of the time, as high as 832 where at most one ever is — so neither ordering of two independent loads is correct and the cheap repair makes the common case worse | `ring_stats` | **measured cost** | [pitfall/001](../pitfall/001_the_leak_in_flight_cannot_see.md) |
| ST43 | `dropped_total` is documented "Items lost across every policy" and folds `Fail`, where the ring refuses the push and hands the item back losing nothing, so a `Fail` ring and a `DropOldest` ring under identical pressure report identical numbers meaning opposite things — and the operator who chose the policy that loses nothing is the one whose loss figure climbs fastest | `ring_stats` | **misleading doc** | [pitfall/002](../pitfall/002_the_total_that_counts_a_refusal_as_a_loss.md) |
| ST44 | The per-policy breakdown a reader would check instead is three separate loads at three moments: driven from one writer keeping the counters within one of each other, three to five percent of two million breakdowns show a spread the ring never held, the widest running to 3,325 — so the total is semantically wrong and the decomposition that would correct it is numerically inconsistent, both failing precisely under the traffic that prompts the question | `ring_stats` | **latent hazard** | [pitfall/002](../pitfall/002_the_total_that_counts_a_refusal_as_a_loss.md) |
| ST45 | `RingStats` derives only `Debug` and `Default` and implements no trait by hand, because `AtomicU64` forecloses `Clone`, `Copy`, `PartialEq`, `Eq` and `Hash` — each would need a consistent read of seven counters — and where `ring_atomic::OpCounts` derives all six over plain `usize` fields, this crate shipped only the live-counter half; `StatsCounts` is now the value half, deriving those five plus `Hash` over nine plain `u64` fields | `ring_stats` | n/a — observation | [type/001](../type/001_a_type_that_cannot_be_copied_compared_or_cloned.md) |
| ST46 | `Debug` is derived and `AtomicU64`'s `Debug` is a relaxed load, so `{:?}` performs seven independent loads and prints them as one struct literal — the consistent snapshot every other route is closed against — and with all seven bumped in lockstep, over ninety-nine percent of two hundred thousand renderings show a spread the set never held, running to 19,831 on counters never more than one apart | `ring_stats` | **latent hazard** | [type/001](../type/001_a_type_that_cannot_be_copied_compared_or_cloned.md) |
| ST47 | All seven fields are `AtomicU64` and all seven readers return `u64`, so the six counters of items and the one counter of nanoseconds are indistinguishable to the compiler — `record_claim( n : u64 )` and `record_wait( nanos : u64 )` differ by an identifier — while `ring_types`, which this crate declares, ships `Seq`, `Capacity` and `SlotIndex` for exactly this purpose and `ring_stats` imports only `OverflowPolicy` | `ring_stats` | n/a — doc gap | [type/002](../type/002_seven_counters_and_one_width.md) |
| ST48 | `record_wait` takes `&self` and is additive like every other recorder, so `wait_nanos` is a sum over waiters rather than a span of wall-clock time and can exceed the process lifetime — against a four-word contract, "Nanoseconds spent waiting.", with no subject, no producer count to normalise by, and no caller anywhere in the workspace to have surfaced it | `ring_stats` | n/a — doc gap | [type/002](../type/002_seven_counters_and_one_width.md) |
| ST49 | `saturating_sub` is documented in the suite as protection against a caller bug, and on a ring where every producer claims before it publishes — no caller bug anywhere — it fires on one to two percent of two million readings at each of three producer counts, underflowing by as much as 36,141, because `claimed` is loaded before `published` and an ordinary correct producer can publish past the value already in hand | `ring_stats` | **measured cost** | [workaround/001](../workaround/001_a_floor_that_absorbs_more_than_it_was_built_for.md) |
| ST50 | The floor maps three distinct conditions onto `0` — a healthy ring, a reading taken across a seam, and a genuine `published > claimed` — where `checked_sub` returning `Option< u64 >` separates the third for one word at the definition, an alternative the suite's own rationale never weighed because it reasons only about the wrap that `saturating_sub` already beats; `StatsCounts::checked_in_flight` now offers that word on the snapshot, leaving `in_flight` itself still mapping all three onto zero | `ring_stats` | n/a — doc gap | [workaround/001](../workaround/001_a_floor_that_absorbs_more_than_it_was_built_for.md) |
| ST51 | `reset` walks an array of references because Rust cannot iterate fields, which is correct — and left the seven counters written out four times by hand, of which the compiler enforced two, so an eighth counter omitted from the array survived every reset silently, read plausibly, stayed monotone, and turned no test red; the array now lives in `RingStats::counters` behind a declared length of `RingStats::COUNTERS` with a `const` size assertion beside it, so the same omission is now two compile errors rather than a green suite | `ring_stats` | **latent hazard** | [workaround/002](../workaround/002_seven_counters_enumerated_four_times_by_hand.md) |
| ST52 | `ring_types` publishes its hand-maintained sets as `ALL` constants and asserts their length in both a doctest and a test, documenting the reasoning in its own corpus — and `ring_stats`, with four hand-maintained sets of seven, applied the convention to none; `RingStats::COUNTERS`, the size assertion beside it and the fixed-length array `counters` returns now convert a silently-missed counter into two compile errors rather than a test that stays green | `ring_stats` | n/a — coverage | [workaround/002](../workaround/002_seven_counters_enumerated_four_times_by_hand.md) |

### Fifty-Two Findings, All of Them About This Crate

Every subject above is `ring_stats`, which no other corpus in this family has
produced. The reason is structural rather than flattering. The crate declares one
dependency and two crates declare it, so there is one crate beneath it to audit
and two above — and neither of the two reaches a counter from a production path
(ST17, ST19). A corpus that cannot find fault upward or downward finds all of it
at home.

The sibling crates appear constantly and almost always as the comparison that
went the other way: `ring_atomic`'s `OpCounts` is the snapshot type this one
declines (ST8, ST45), `ring_atomic::counts` and `ring_seqno::free_slots` are the two
derived-reading shapes it did not pick (ST39), `ring_types::drops_silently` is the
one-line filter that would fix `dropped_total` (ST12), `ring_types`' `ALL` length
assertions are the check `reset`'s array does not have (ST52), `ring_align`'s
`CacheAligned` is the padding its layout costs 2.6×–3.3× for the want of (ST9,
ST20), and `ring_factory` is the crate that refused a `ring_stats` edge in writing
where this one refused `ring_align` in silence (ST20). The two exceptions cut the
other way: `ring_stats` is the only crate in its group to have made one ordering
decision, applied it without exception, and recorded why (ST13), and the one
clause of this crate's own originating requirement with a real design consequence is implemented past the
letter (ST15).

### The Shape Every Definition Found

Stated once because most of the thirteen definitions reach it independently:
**the two readings the crate puts forward are the two that stop being true under
the traffic that makes anyone take them.**

| Where the gap is | Findings |
|------------------|----------|
| a reading that fails precisely when it is consulted | ST3, ST4, ST22, ST24, ST41, ST42, ST43, ST44, ST46 |
| a real cost weighed carefully against the wrong alternative | ST9, ST14, ST23, ST34, ST50 |
| something built, named, and left with no route to a caller | ST17, ST18, ST19, ST26, ST32 |
| a property claimed once and checked by nothing | ST12, ST21, ST30, ST33, ST36, ST52 |

Three rows of that fourth line have since acquired the check they named — ST21 and
ST30 by the two contention tests added beside `snapshot` and `reset`, ST52 by
`RingStats::COUNTERS`. The findings stand as written; what changed is that the suite
now reaches where they said it did not.
| a distinction the storage keeps and the surface erases | ST11, ST27, ST38, ST47, ST48 |
| the family already solved it one crate over | ST8, ST10, ST20, ST39, ST52 |

Not one is a bug in the sense of a failing test. Every recorder does exactly what
its name says, every counter is exact under contention, and every test passes —
seventeen when this was written, twenty-one since. The cost is concentrated in the
methods that touch more than one counter — `in_flight`, `dropped_total` and `reset`,
three of fourteen then, joined since by `snapshot` — which are the ones the crate
advertises and the ones that produce every method-specific finding here (ST16).

### Severity

| Tier | Meaning | Findings | Count |
|------|---------|----------|------:|
| **latent hazard** | well-typed, correct-looking code reads a number that is wrong | ST3, ST17, ST23, ST26, ST29, ST41, ST44, ST46, ST51 | 9 |
| **misleading doc** | a true statement a caller can act on and be wrong | ST4, ST11, ST22, ST24, ST31, ST38, ST43 | 7 |
| **measured cost** | a real, measured runtime cost nothing records | ST9, ST14, ST34, ST42, ST49 | 5 |
| **wrong doc** | a stated fact is false, measured | ST18 | 1 |
| n/a — doc gap | a true fact is unstated where it is needed | ST2, ST5, ST7, ST10, ST27, ST28, ST35, ST36, ST40, ST47, ST48, ST50 | 12 |
| n/a — observation | true, useful, and carrying no defect | ST1, ST6, ST13, ST15, ST20, ST25, ST32, ST37, ST39, ST45 | 10 |
| n/a — coverage | the tests do not reach what they appear to | ST16, ST19, ST21, ST30, ST33, ST52 | 6 |
| n/a — unenforced | a real property with no lint, test, or check behind it | ST12 | 1 |
| n/a — duplication | a shape the workspace already has, rebuilt or declined without record | ST8 | 1 |

The first four tiers are the reachable ones — 22 of 52. That was the highest
share in the family when it was written and is no longer: `ring_shutdown` and
`ring_bench` both reach 29 of 52 since being brought to standard. What holds is
the reason the share is high here at all — a crate with no callers has nothing to
blame for its own numbers, so every defect it carries is its own.

The nine latent hazards share one property, and it is what puts them above the
`misleading doc` tier rather than beside it: **each returns a plausible number
rather than an error.** `in_flight` reports zero against a real leak (ST3, ST41)
and zero against the caller bug it was told to catch (ST23); `record_wait`'s
counter reports zero because nothing writes it (ST26); a read caught mid-reset
returns a state the ring never held (ST29); `{:?}` prints seven values that were
never simultaneously true (ST46); the per-policy breakdown that would correct
`dropped_total` is itself inconsistent three to five percent of the time (ST44);
and an eighth counter left out of `reset`'s array would stay plausible and
monotone forever (ST51). None of them has a value a caller could reject.

All nine now carry a `**Disposition:**` line, and all nine read `applied` — each
naming what changed and quoting a line of the crate's own regenerated census as
evidence. Six landed in the source: `in_flight`'s doc gained the direction of its
error (ST3, ST41), `record_wait`'s and `wait_nanos`' gained the missing edge (ST26),
`reset`'s gained the window and its measured rate (ST29), `RingStats`'s gained the
tearing `{:?}` (ST46), the module comment gained the write-path census (ST17), and
`RingStats::COUNTERS` turned the hand-written array into a length the compiler checks
(ST51). Two added surface: `StatsCounts::checked_in_flight` separates the caller bug
from a balanced ring (ST23), and `RingStats::snapshot` returns a value whose own total
and breakdown agree (ST44).

What none of them changed is the reading a caller gets by default. `in_flight` still
floors at zero, `dropped_total` still folds a refusal in with two losses, and the
default route through the API still returns a plausible number rather than an error —
the new readings sit beside the old ones rather than replacing them. In eight of the
nine the fix was the sentence, not the value, which is the honest measure of what a
documentation pass can close.

The five measured costs are separated deliberately. None threatens correctness,
and two of them are the same cache line charged twice — the layout multiplier
(ST9) and the throughput ceiling it fixes (ST34) — against a fence the crate was
designed to avoid that does not measure at all (ST14).
