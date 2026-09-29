# non_functional_requirement

Two files, and between them they cover everything this crate is required to be
rather than to do. The first takes the properties that hold at any load — no
allocation, no blocking, no `unsafe` — and asks what actually enforces each. The
second takes the property that only exists under load, derives it from the
source, and finds that nothing measures it.

They reach the same conclusion by opposite routes. The first finds enforcement
concentrated on the least fragile property and absent from the two a plausible
change would break. The second finds a real, substantial benchmark harness in
the family that cannot reach this crate at all. In both cases the property is
genuinely held; in both cases what is missing is the record that anyone is
holding it.

That is the theme worth carrying out of this definition: **this crate's
non-functional properties are all true and none of them are claimed.** A future
change that broke any of the four would compile, pass 28 of 28 tests, pass all
six manual checks, and produce no signal anywhere.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Nothing Allocates, Nothing Waits, Nothing Is Unsafe](001_nothing_allocates_nothing_waits_nothing_is_unsafe.md) | CL35, CL36, CL55 — `no_std` in substance across seven crates and declared in three of 33, enforcement aimed at the one property nobody would break, and the title's first claim measured false at one allocation per `claim` until `b7e075ca` made it true and a test made it stay true |
| 002 | [What Contention Costs](002_what_contention_costs.md) | CL37, CL38 — the `k × C` cost model derived from source, a benchmark harness this crate cannot enter, and the one number it did measure |

### The Four Properties and What Holds Them

| Property | True today | Enforced by | Breaks silently |
|----------|:----------:|-------------|:---------------:|
| No `unsafe` | ✔ | `unsafe-code = "deny"`, plus a clippy lint | **no** |
| No allocation | ✘ — one per `claim`, measured | nothing | **it already did** |
| Never blocks | ✔ | nothing | **yes** |
| `no_std`-clean (7-crate chain) | ✔ | `ring_types` only — the other six undeclared | **yes** |
| Performance | — | unmeasured, unclaimed, unmeasurable in isolation | n/a |

One row of five had teeth, and it was the row protecting a crate whose entire
implementation is three comparisons and a compare-exchange. One of the four
without teeth was not merely unenforced but already false: `Claimer::claim`
allocated once per call, two crates down, and no test, lint, or check in the
family reported it ([CL55](001_nothing_allocates_nothing_waits_nothing_is_unsafe.md)).
That row now has teeth of its own — `tests/allocation_test.rs` measures the
property through this crate's API, so the whole dependency chain is inside the
check. Three of five remain held by nobody noticing.

### The Cheapest Available Fix

Both files independently arrived at the same one-line remedy, which is why it is
stated once here rather than twice. Half of it has since been overtaken:
allocation is now held by `tests/allocation_test.rs`, which measures the
property through the crate's API and therefore covers the whole dependency
chain, which a grep of one file never could. What follows still applies to
blocking and `no_std`-cleanliness. `tests/manual/readme.md § C2` already exists
and already greps this source:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -vE "^[[:space:]]*//" ring_claim/src/lib.rs | grep -E "fetch_add|compare_exchange|\.store\("
```

Live output:

```
      match self.cursor.compare_exchange( current, next, CLAIM_SUCCESS, GATING )
      match self.cursor.compare_exchange( current, next, CLAIM_SUCCESS, GATING )
```

Extending its expression to `\bloop\b|park|spin_loop|yield_now|\bstd::` would put
blocking and `no_std`-cleanliness under the same weight as the `fetch_add` ban,
in the same check, at the cost of one alternation. It would not enforce them the
way a lint does — a manual check is run by a person — but it would move them
from "nobody is looking" to "one of six checks looks."

A `Vec<|Box<|String|alloc::` alternation is deliberately not in that list any
more. A source grep for allocation is what was already being run when CL55's
allocation went unnoticed for the length of the corpus: the `Vec` was one crate
away, and no expression over this file's text was ever going to find it.

The `no_std` attribute itself turns out not to be a larger decision at all.
Measured rather than assumed: a scratch copy of this crate with `#![ no_std ]`
prepended to `src/lib.rs` and nothing else changed compiles, and all 27
integration tests and 17 doctests pass. The suite's `std::thread::scope` is not
an obstacle — `tests/` and doctests are separate crates that link `std` on their
own, so no `extern crate std;` and no `[dev-dependencies]` entry is needed. The
attribute costs one line and is enforced by the compiler forever, which makes
its absence from all 33 crates harder to read as a tradeoff than as an oversight.

### What Is Correctly Absent

Recorded so the gaps above are not read as a list of everything that should be
added:

| Not present | Correctly so |
|-------------|--------------|
| a retry budget | its exhaustion has no correct handling; `ring_publish:42-53` makes the same argument for its own spin |
| a latency bound | contention is the caller's workload, not this crate's property |
| wait-freedom | the loop is lock-free; every failed attempt is another producer's success |
| a primitive-level benchmark | a `Claimer` writes nothing, so an isolated number would measure the harness |
| checked arithmetic on the cursor | the wrap point is ~585 years away ([`lifecycle/002`](../lifecycle/002_the_claimer_over_a_rings_life.md)) |

Five of these have a stated reason somewhere in the family. The gaps in the
previous section have none — that is the distinction this definition is drawing.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order

# the three negative properties, in one grep, with the family's comment filter
grep -vE "^[[:space:]]*//" ring_claim/src/lib.rs \
  | grep -E 'unsafe|Vec<|Box<|String|alloc::|\bloop\b|park|spin_loop|yield_now|\bstd::'

# std across the whole transitive chain, and no_std across the whole family
for c in ring_claim ring_types ring_cursor ring_gating ring_atomic ring_align ring_seqno; do
  printf '%-12s %s\n' "$c" \
    "$( grep -vE "^[[:space:]]*//" ring/$c/src/*.rs | grep -cE '\bstd::' )"
done
grep -rl 'no_std' ring_*/src/lib.rs | wc -l

# what enforces anything
sed -n '/\[workspace.lints/,/^\[[^w]/p' Cargo.toml

# benchmark coverage of a Tier 5 primitive
ls -d ring_*/benches 2>/dev/null | wc -l
grep -rl 'criterion\|\[\[bench\]\]' ring_*/Cargo.toml | wc -l
command grep -rc 'ring_claim' ring_bench/ | sort

# the per-attempt cost that scales with consumer count
command grep -m1 -A32 -F '  pub fn slowest( &self ) -> Option< Seq >' ring_gating/src/lib.rs
```

Live output:

```
ring_claim   0
ring_types   0
ring_cursor  0
ring_gating  0
ring_atomic  0
ring_align   0
ring_seqno     0
3
[workspace.lints.rust]
rust_2018_idioms = { level = "warn", priority = -1 }
future_incompatible = { level = "warn", priority = -1 }
missing_docs = "warn"
missing_debug_implementations = "warn"
unsafe-code = "deny"
# `loom` is set by RUSTFLAGS, not by any feature, so rustc has no other way to
# learn it is a real cfg. Declared once here rather than per crate: the lints
# table is inherited workspace-wide, and a crate cannot both inherit it and add
# its own. Only ring_atomic, ring_cursor and ring_publish read the cfg — see
# ring_atomic's module documentation on the seam.
unexpected_cfgs = { level = "warn", check-cfg = [ 'cfg(loom)' ] }

[workspace.lints.clippy]
undocumented_unsafe_blocks = "deny"
0
0
ring_bench/Cargo.toml:0
ring_bench/docs/algorithm/001_one_workload_through_six_runners.md:0
ring_bench/docs/algorithm/002_the_eligibility_filter_runs_before_the_comparison.md:0
ring_bench/docs/algorithm/readme.md:0
ring_bench/docs/api/001_the_run_surface.md:0
ring_bench/docs/api/002_the_report_surface.md:0
ring_bench/docs/api/readme.md:0
ring_bench/docs/data_structure/001_the_workload_description.md:0
ring_bench/docs/data_structure/002_three_counts_that_are_not_interchangeable.md:0
ring_bench/docs/data_structure/readme.md:0
ring_bench/docs/decisions/001_five_candidates_for_four_named_paths.md:0
ring_bench/docs/decisions/002_no_test_asserts_an_ordering.md:0
ring_bench/docs/decisions/readme.md:0
ring_bench/docs/definition/readme.md:0
ring_bench/docs/integration/001_declared_edges_and_the_three_that_were_missing.md:0
ring_bench/docs/integration/002_the_only_consumer_of_two_contract_names.md:1
ring_bench/docs/integration/readme.md:0
ring_bench/docs/invariant/001_received_never_exceeds_reported_never_exceeds_offered.md:0
ring_bench/docs/invariant/002_the_counters_are_written_outside_the_clock.md:0
ring_bench/docs/invariant/readme.md:0
ring_bench/docs/item/001_seven_nouns_and_the_one_never_compared.md:0
ring_bench/docs/item/002_forty_verbs_twenty_five_of_them_const.md:0
ring_bench/docs/item/readme.md:0
ring_bench/docs/lifecycle/001_from_a_description_to_a_verdict.md:0
ring_bench/docs/lifecycle/002_one_candidate_through_one_run.md:0
ring_bench/docs/lifecycle/readme.md:0
ring_bench/docs/non_functional_requirement/001_the_comparison_is_reproducible_and_same_conditions.md:0
ring_bench/docs/non_functional_requirement/002_the_harness_is_not_in_the_measurement.md:0
ring_bench/docs/non_functional_requirement/readme.md:0
ring_bench/docs/pattern/001_the_measurement_is_a_value.md:0
ring_bench/docs/pattern/002_a_refusal_is_a_row.md:0
ring_bench/docs/pattern/readme.md:0
ring_bench/docs/pitfall/001_the_door_caps_what_the_structure_does_not.md:0
ring_bench/docs/pitfall/002_a_counter_inside_the_timed_region_measures_itself.md:0
ring_bench/docs/pitfall/003_ok_is_not_kept_and_the_verdict_inverts.md:0
ring_bench/docs/pitfall/readme.md:0
ring_bench/docs/readme.md:0
ring_bench/docs/type/001_candidate.md:0
ring_bench/docs/type/002_run_error.md:1
ring_bench/docs/type/readme.md:0
ring_bench/docs/workaround/001_the_clock_is_the_platforms_and_the_tie_break_is_the_list.md:0
ring_bench/docs/workaround/002_a_feature_that_cannot_be_negated_at_the_use_site.md:0
ring_bench/docs/workaround/readme.md:0
ring_bench/examples/comparison.rs:0
ring_bench/readme.md:0
ring_bench/src/lib.rs:0
ring_bench/task/readme.md:0
ring_bench/task/unverified/128_implement_ring_bench.md:0
ring_bench/tests/bench_test.rs:1
ring_bench/tests/manual/readme.md:0
  pub fn slowest( &self ) -> Option< Seq >
  {
    ring_cursor::slowest( &self.cursors )
  }

  /// How many slots a producer at `producer` may claim right now.
  ///
  /// Zero when the ring is full. A full capacity when the set is empty, since
  /// a ring nobody reads has no data anyone can lose.
  ///
  /// ```
  /// use core::sync::atomic::Ordering;
  /// use ring_cursor::SeqCell;
  /// use ring_gating::GatingSet;
  /// use ring_types::{ Capacity, Seq };
  ///
  /// let set = GatingSet::new( Capacity::new( 4 ).unwrap(), 1 );
  /// assert_eq!( set.headroom( Seq::ZERO ), 4 );
  /// assert_eq!( set.headroom( Seq( 3 ) ), 1 );
  /// assert_eq!( set.headroom( Seq( 4 ) ), 0, "a full lap ahead" );
  ///
  /// set.cursor( 0 ).unwrap().store( Seq( 2 ), Ordering::Release );
  /// assert_eq!( set.headroom( Seq( 4 ) ), 2, "the consumer released two slots" );
  /// ```
  #[ must_use ]
  pub fn headroom( &self, producer : Seq ) -> usize
  {
    self.slowest().map_or( self.capacity.get(), | slowest |
    {
      ring_seqno::free_slots( producer, slowest, self.capacity )
    } )
  }
```

| | Value |
|--|------:|
| `unsafe` blocks in the crate | **0** |
| …lints denying them | 2 |
| Allocating types in the crate | **0** |
| …in the six transitive dependencies the Scope covers | 3, in 2 crates |
| …reached by a `claim` call | **0** — was 1, in `ring_cursor::slowest`, until `b7e075ca` |
| Heap allocations per `Claimer::claim`, measured, release | **0** — was **1**, same commit |
| …per `Claimer::claim` on an ungated set | 0 — and always was |
| …lints or checks guarding that | **1** — `tests/allocation_test.rs`, six call shapes and a control arm |
| Blocking constructs in the crate | **0** |
| …lints or checks guarding that | **0** |
| `std::` paths across the 7-crate chain | **0** |
| Crates of 33 declaring `#![ no_std ]` | **3** — `ring_types`, `ring_stats`, `ring_overflow` |
| Source changes to add it to `ring_claim` | **0** — measured |
| …tests still passing after adding it | **27 + 17** |
| `benches/` directories in the family | **0** |
| Bench-framework dependencies | **0** |
| `ring_bench` candidates | 6 |
| …that are a Tier 0–5 primitive | **0** |
| Cursor reads per claim attempt, `C` consumers | `C + 1` |
| Atomic RMWs per attempt | 1 |
| Cache lines a `Claimer` occupies | 2 |
| …written | **1** |
| Timed measurements of this crate, anywhere | **0** |

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CL35 | family | n/a — unenforced | `ring_claim` and all six of its transitive dependencies contain zero `std::` paths, every `use` in the chain resolving to `core::` or a sibling crate, while **three of the 33 family crates declare `#![ no_std ]`** and of this chain's seven only `ring_types` is among them; declaring it on the rest costs nothing, measured — 27 integration tests and 17 doctests pass unchanged |
| CL36 | `ring_claim` | n/a — unenforced | The only enforced property is `unsafe`, guarded by two lints, in a crate of three comparisons and a compare-exchange; the two a plausible change would actually break — allocation and non-blocking — are guarded by nothing, and `§ C2`'s existing grep could carry both for the cost of one alternation |
| CL55 | `ring_cursor` | **wrong doc** | `Claimer::claim` and `Claimer::headroom` each performed **one heap allocation per call** until commit `b7e075ca`, measured in release with a counting `GlobalAlloc`; the site was `ring_cursor::slowest`, collecting cursor positions into a `Vec` only to adapt `&[ PaddedCursor ]` to the `&[ Seq ]` its callee takes, where the allocation-free fold that has since replaced it would do. It read as clean because the evidence command grepped one file while the claim covers seven crates, and because an ungated set's zero-length `Vec` never reaches the allocator — the property is now held by `tests/allocation_test.rs` rather than by prose |
| CL37 | family | n/a — coverage | The family has no `benches/` directory, no `[[bench]]` section and no bench framework in any of 33 crates; `ring_bench` measures six complete write paths including a `MutexQueue` baseline, and `ring_claim` appears in none of them and cannot, since a `Candidate` is an end-to-end path and a `Claimer` writes nothing |
| CL38 | `ring_claim` | n/a — observation | The one measured number anywhere in this crate is `§ C1`'s 4/5→8/8 detection rate, which quantifies a *test's* sensitivity against a broken implementation, not the code's speed; it reads like a performance figure at a glance and nothing in the crate has ever been timed |

Supporting observations recorded alongside those findings, carrying no ID of their own:

| Note | Where |
|------|-------|
| Declaring it costs nothing, measured: `#![ no_std ]` prepended to `src/lib.rs` compiles with zero source changes and all 27 integration tests and 17 doctests pass, because `tests/` and doctests are separate crates that link `std` themselves | [001](001_nothing_allocates_nothing_waits_nothing_is_unsafe.md) |
| "Never blocks" and "always terminates" are different claims and the crate makes only the first: the retry loop is lock-free, not wait-free, and an individual caller has no bound | [001](001_nothing_allocates_nothing_waits_nothing_is_unsafe.md) |
| `CLAIM_SUCCESS` is argued against its weaker alternative in six lines naming a concrete reordering, matching `ring_publish`'s treatment of `PUBLISH` — a consistent family practice and the strongest documentation either crate has | [001](001_nothing_allocates_nothing_waits_nothing_is_unsafe.md) |
| The founding decision refused a mutex on composability grounds while the family's only mutex comparison measures whole-ring throughput — the comparison that exists and the decision that was made answer different questions, and nothing records that | [002](002_what_contention_costs.md) |
| Cost per claim is roughly `k × C` — `k` retries scaled by producers, `C` cursor reads per attempt scaled by consumers — and both factors rise together under load, which `headroom`'s documentation does not mention | [002](002_what_contention_costs.md) |

